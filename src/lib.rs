#![warn(clippy::all, clippy::pedantic)]

//! A library for processing and managing dataset-related files and metadata.
//!
//! This library provides functionality for:
//! - Processing safetensors files and extracting metadata
//! - Handling caption files
//! - Processing and formatting JSON files
//! - Converting between different file formats (JSON to caption)
//! - Managing reasoning datasets for AI training
//!
//! The library is organized into several modules:
//! - `caption`: Handles caption file processing
//! - `metadata`: Manages metadata extraction and processing
//! - `st`: SafeTensors-related functionality
//! - `reasoning`: Manages AI reasoning datasets for training

pub mod caption;
pub mod metadata;
pub mod reasoning;
pub mod st;

use log::info;
pub use xio;

// Re-export commonly used types
use anyhow::{Context, Result};
use serde_json::Value;
use std::{
    io,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::fs;

// Include test modules
#[cfg(test)]
mod tests {
    pub mod e621_tests;
    pub mod text_tests;
}

/// Extracts and parses JSON metadata from a safetensors file.
///
/// This function reads a safetensors file, extracts its metadata, and converts it into
/// a JSON value. The metadata is processed through the `metadata::extract_training_metadata`
/// function to decode any nested JSON fields.
///
/// # Arguments
/// * `path` - Path to the safetensors file
///
/// # Returns
/// * `Result<Value>` - The parsed JSON metadata if successful
///
/// # Errors
/// Returns an error if:
/// * The file cannot be opened
/// * Memory mapping fails
/// * Metadata cannot be read from the safetensors file
/// * Metadata cannot be converted to JSON
fn get_json_metadata(path: &Path) -> Result<Value> {
    use ::safetensors::SafeTensors;
    use memmap2::MmapOptions;
    use std::fs::File;

    let file = File::open(path).context("Failed to open file")?;
    let mmap = unsafe {
        MmapOptions::new()
            .map(&file)
            .context("Failed to mmap file")?
    };
    let (_header_size, metadata) =
        SafeTensors::read_metadata(&mmap).context("Failed to read metadata")?;

    // Convert the raw metadata into a JSON value
    let metadata_json: Value =
        serde_json::to_value(&metadata).context("Failed to convert metadata to JSON value")?;

    // Use the new helper function to extract and recursively decode JSON fields
    let training_metadata = crate::metadata::extract_training_metadata(&metadata_json);

    Ok(training_metadata)
}

/// Processes a safetensors file by extracting its metadata and saving it as a JSON file.
///
/// This function:
/// 1. Extracts metadata from the safetensors file
/// 2. Pretty-prints the JSON metadata
/// 3. Saves the metadata to a new file with the same name but .json extension
///
/// # Arguments
/// * `path` - Path to the safetensors file to process
///
/// # Returns
/// * `Result<()>` - Success or failure of the operation
///
/// # Errors
/// Returns an error if:
/// * Metadata extraction fails
/// * JSON formatting fails
/// * Writing the output file fails
pub async fn process_safetensors_file(path: &Path) -> Result<()> {
    let json = get_json_metadata(path)?;
    let pretty_json = serde_json::to_string_pretty(&json)?;
    info!("{pretty_json}");
    fs::write(path.with_extension("json"), pretty_json).await?;
    Ok(())
}

/// Processes a caption file using the functionality in the caption module.
///
/// This is a wrapper function that delegates the actual processing to the
/// caption module's implementation.
///
/// # Arguments
/// * `path` - Path to the caption file to process
///
/// # Returns
/// * `Result<()>` - Success or failure of the operation
///
/// # Errors
/// Returns an error if the caption processing fails
pub async fn process_caption_file(path: &Path) -> Result<()> {
    caption::process_file(path).await
}

/// Processes a JSON file using a provided async processor function.
///
/// This function reads a JSON file, parses it, and applies a custom processor
/// function to the parsed data. The processor function can perform any desired
/// transformations or operations on the JSON data.
///
/// # Type Parameters
/// * `F` - The processor function type
/// * `Fut` - The future type returned by the processor function
///
/// # Arguments
/// * `file_path` - Path to the JSON file to process
/// * `processor` - Async function that processes the parsed JSON data
///
/// # Returns
/// * `io::Result<()>` - Success or failure of the operation
///
/// # Errors
/// Returns an error if:
/// * The file cannot be read
/// * The content cannot be parsed as JSON
/// * The processor function returns an error
#[must_use = "Processes a JSON file and requires handling of the result to ensure proper file processing"]
pub async fn process_json_file<F, Fut>(file_path: &Path, processor: F) -> io::Result<()>
where
    F: FnOnce(Value) -> Fut + Send,
    Fut: std::future::Future<Output = io::Result<()>> + Send,
{
    let content = fs::read_to_string(file_path).await?;
    let data: Value = serde_json::from_str(&content)?;
    processor(data).await
}

/// Formats a JSON file by pretty-printing its contents.
///
/// This function reads a JSON file, parses it, and writes it back with proper
/// formatting and indentation. The original file is overwritten with the
/// formatted version.
///
/// # Arguments
/// * `path` - Path to the JSON file to format
///
/// # Returns
/// * `Result<()>` - Success or failure of the operation
///
/// # Errors
/// Returns an error if:
/// * The file cannot be read
/// * The content cannot be parsed as JSON
/// * The formatted JSON cannot be written back to the file
#[must_use = "Formats a JSON file and requires handling of the result to ensure the file is properly formatted"]
pub async fn format_json_file(path: PathBuf) -> Result<()> {
    info!("Processing file: {}", path.display());

    let file_content = fs::read_to_string(path.clone())
        .await
        .context("Failed to read file content")?;
    let json: Value = serde_json::from_str(&file_content).context("Failed to parse JSON")?;
    let pretty_json = serde_json::to_string_pretty(&json).context("Failed to format JSON")?;
    fs::write(path.clone(), pretty_json)
        .await
        .context("Failed to write formatted JSON")?;

    info!("Formatted {} successfully.", path.display());
    Ok(())
}

/// Splits a content string into tags and sentences.
///
/// This function takes a string in the format "tag1, tag2, tag3., Sentence text"
/// and splits it into a vector of tags and the remaining sentence text.
///
/// # Arguments
/// * `content` - The string to split, expected to be in the format "tags., sentence"
///
/// # Returns
/// * `(Vec<String>, String)` - A tuple containing:
///   * A vector of tag strings
///   * The remaining sentence text
///
/// # Examples
/// ```
/// use dset::split_content;
///
/// let content = "tag1, tag2, tag3., This is a sentence.";
/// let (tags, sentence) = split_content(content);
/// assert_eq!(tags, vec!["tag1", "tag2", "tag3"]);
/// assert_eq!(sentence, "This is a sentence.");
/// ```
#[must_use = "Splits content into tags and sentences and the result should be checked"]
pub fn split_content(content: &str) -> (Vec<String>, String) {
    let split: Vec<_> = content.split("., ").collect();
    let tags: Vec<_> = split[0]
        .split(',')
        .map(str::trim)
        .map(String::from)
        .collect();
    let sentences = (*split.get(1).unwrap_or(&"")).to_string();
    (tags, sentences.trim().to_string())
}

/// Converts a JSON file containing tag probabilities into a caption file.
///
/// This function reads a JSON file containing tag-probability pairs, filters
/// tags based on a probability threshold (0.2), and writes the selected tags
/// to a new .txt file. Tags are sorted by probability in descending order.
///
/// # Arguments
/// * `input_path` - Path to the input JSON file
///
/// # Returns
/// * `io::Result<()>` - Success or failure of the operation
///
/// # Errors
/// Returns an error if:
/// * The input file cannot be read
/// * The content cannot be parsed as JSON
/// * The output file cannot be written
///
/// # Format
/// Input JSON should be in the format:
/// ```json
/// {
///     "tag1": 0.9,
///     "tag2": 0.5,
///     "tag3": 0.1
/// }
/// ```
#[must_use = "Processes a JSON file to create a caption file and requires handling of the result to ensure proper conversion"]
pub async fn process_json_to_caption(input_path: &Path) -> io::Result<()> {
    // Early return if not a JSON file
    if input_path.extension().and_then(|s| s.to_str()) != Some("json") {
        return Ok(());
    }

    let content = fs::read_to_string(input_path).await?;
    let json: Value = serde_json::from_str(&content)?;
    info!("Processing JSON: {}", json);

    let mut tags = Vec::new();
    if let Value::Object(map) = json {
        for (tag, prob) in map {
            if let Value::Number(prob) = prob {
                if let Some(prob) = prob.as_f64() {
                    if prob >= 0.2 {
                        tags.push((tag, prob));
                    }
                }
            }
        }
    }

    tags.sort_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    let tags: Vec<_> = tags
        .into_iter()
        .map(|(tag, _)| {
            // Escape special characters with backslashes
            tag.replace('(', "\\(").replace(')', "\\)")
        })
        .collect();

    let output = tags.join(", ");
    fs::write(input_path.with_extension("txt"), output).await?;
    Ok(())
}

/// Renames a file to remove any image extensions that appear between the base filename and the actual extension.
///
/// For example:
/// - `image.jpg.toml` -> `image.toml`
/// - `image.jpeg.json` -> `image.json`
/// - `image.png` -> `image.png` (unchanged)
/// - `image.png.jpg` -> `image.jpg`
///
/// This is useful for cleaning up file names in datasets where image extensions might have been
/// accidentally preserved when converting files to other formats.
///
/// # Arguments
/// * `path` - Path to the file to rename
///
/// # Returns
/// * `io::Result<()>` - Success or failure of the operation
///
/// # Errors
/// Returns an error if:
/// * The file cannot be renamed
/// * The file system operation fails
/// * The file name is invalid UTF-8
///
/// # Panics
/// This function will panic if:
/// * The file name has multiple extensions but `parts.last()` fails to get the last extension
///   (this should never happen as we check `parts.len() >= 3` before accessing)
///
/// # Example
/// ```no_run
/// use std::path::Path;
/// use dset::rename_file_without_image_extension;
///
/// async fn example() -> std::io::Result<()> {
///     let path = Path::new("image.jpg.toml");
///     rename_file_without_image_extension(&path).await?;  // Will rename to "image.toml"
///     Ok(())
/// }
/// ```
#[must_use = "Renames a file and requires handling of the result to ensure the file is properly renamed"]
pub async fn rename_file_without_image_extension(path: &Path) -> io::Result<()> {
    // Get the file stem and extension
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid file name"))?;

    // Split the filename into parts
    let parts: Vec<&str> = file_name.split('.').collect();

    // Only proceed if we have at least 3 parts (name.img_ext.real_ext)
    if parts.len() >= 3 {
        // Check if any middle extension is an image extension
        let mut has_image_ext = false;
        for ext in &parts[1..parts.len() - 1] {
            if matches!(ext.to_lowercase().as_str(), "jpg" | "jpeg" | "png") {
                has_image_ext = true;
                break;
            }
        }

        if has_image_ext {
            // Reconstruct the filename without image extensions
            let mut new_name = String::from(parts[0]);
            // SAFETY: We checked parts.len() >= 3 above, so last() will never be None
            let last_ext = parts.last().unwrap();
            new_name.push('.');
            new_name.push_str(last_ext);

            // Create the new path in the same directory
            let parent = path.parent().unwrap_or_else(|| Path::new(""));
            let new_path = parent.join(new_name);

            fs::rename(path, &new_path).await?;
            info!("Renamed {} to {}", path.display(), new_path.display());
        }
    }
    Ok(())
}

/// Process an e621 JSON file and generate caption files.
///
/// # Arguments
///
/// * `file_path` - Path to the JSON file to process
/// * `config` - Optional configuration for processing. If None, uses default settings.
///
/// # Errors
///
/// This function will return an error if:
/// * The file cannot be read
/// * The file contains invalid JSON
/// * There are issues writing the caption files
/// * The JSON structure doesn't match the expected e621 format
///
/// # Returns
///
/// Returns `Ok(())` on success, or an error if any step fails.
pub async fn process_e621_json_file(
    file_path: &Path,
    config: Option<caption::E621Config>,
) -> Result<()> {
    let content = fs::read_to_string(file_path).await?;
    let data_owned: Value = serde_json::from_str(&content)?;
    let file_path = Arc::new(file_path.to_path_buf());
    caption::process_e621_json_data(&data_owned, &file_path, config).await
}

pub use caption::{
    caption_file_exists_and_not_empty, format_text_content, json_to_text, process_file,
    replace_special_chars, replace_string,
};
