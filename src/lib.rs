#![warn(clippy::all, clippy::pedantic)]

pub mod st;
pub mod caption;

#[cfg(test)]
mod tests {
    mod st_tests;
    mod caption_tests;
    mod utils_tests;
}

pub use xio;
use log::info;

// Re-export image-related functionality
pub use imx::{
    is_image_file,
    caption_file_exists_and_not_empty,
    rename_file_without_image_extension,
    remove_letterbox,
};

// Re-export commonly used types
pub use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use serde_json::Value;
use tokio::{
    fs::{self, File, write},
    io::{self, AsyncWriteExt},
};

/// Get JSON metadata from a safetensors file
/// 
/// # Errors
/// Returns an error if:
/// - Failed to read the file
/// - Failed to parse the metadata
async fn get_json_metadata(path: &Path) -> Result<Value> {
    use memmap2::MmapOptions;
    use ::safetensors::SafeTensors;
    use std::fs::File;

    let file = File::open(path).context("Failed to open file")?;
    let mmap = unsafe { MmapOptions::new().map(&file).context("Failed to mmap file")? };
    let (_header_size, metadata) = SafeTensors::read_metadata(&mmap).context("Failed to read metadata")?;
    
    // Extract just the __metadata__ contents if it exists
    let metadata_json: Value = serde_json::to_value(&metadata).context("Failed to convert metadata to JSON value")?;
    
    let training_metadata = if let Value::Object(obj) = metadata_json {
        if let Some(Value::Object(meta)) = obj.get("__metadata__") {
            Value::Object(meta.clone())
        } else {
            Value::Object(serde_json::Map::new())
        }
    } else {
        Value::Object(serde_json::Map::new())
    };

    Ok(training_metadata)
}

/// Process a safetensors file and extract its metadata
/// 
/// # Errors
/// Returns an error if:
/// - Failed to process the safetensors file
pub async fn process_safetensors_file(path: &Path) -> Result<()> {
    let json = get_json_metadata(path).await?;
    let pretty_json = serde_json::to_string_pretty(&json)?;
    info!("{pretty_json}");
    write(path.with_extension("json"), pretty_json).await?;
    Ok(())
}

/// Process a caption file
/// 
/// # Errors
/// Returns an error if:
/// - Failed to process the caption file
pub async fn process_caption_file(path: &Path) -> Result<()> {
    caption::process_file(path).await
}

/// Processes a JSON file with a given processor function.
///
/// # Errors
///
/// Returns an `io::Error` if the file cannot be opened, read, or if the JSON cannot be parsed.
#[must_use = "Processes a JSON file and requires handling of the result to ensure proper file processing"]
pub async fn process_json_file<F, Fut>(file_path: &Path, processor: F) -> io::Result<()>
where
    F: FnOnce(&Value) -> Fut,
    Fut: std::future::Future<Output = io::Result<()>>,
{
    let content = fs::read_to_string(file_path).await?;
    let data: Value = serde_json::from_str(&content)?;
    processor(&data).await
}

/// Formats a JSON file to have pretty-printed JSON.
///
/// # Errors
///
/// Returns an `io::Error` if the file cannot be read, parsed as JSON, or written back.
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

/// Splits content into tags and sentences.
#[must_use = "Splits content into tags and sentences and the result should be checked"]
pub fn split_content(content: &str) -> (Vec<&str>, &str) {
    let split: Vec<_> = content.split("., ").collect();
    let tags: Vec<_> = split[0].split(',')
        .map(str::trim)
        .collect();
    let sentences = split.get(1).unwrap_or(&"");
    (tags, sentences.trim())
}

/// Processes a JSON file and converts it to a caption file.
///
/// # Errors
///
/// Returns an `io::Error` if the file cannot be read, parsed, or written.
#[must_use = "Processes a JSON file to create a caption file and requires handling of the result to ensure proper conversion"]
pub async fn process_json_to_caption(input_path: &Path) -> io::Result<()> {
    if input_path.extension().and_then(|s| s.to_str()) == Some("json") {
        let content = fs::read_to_string(input_path).await?;
        let json: Value = serde_json::from_str(&content)?;

        if let Value::Object(map) = json {
            let mut tags: Vec<(String, f64)> = map
                .iter()
                .filter_map(|(key, value)| {
                    if let Value::Number(num) = value {
                        let probability = num.as_f64().unwrap_or(0.0);
                        if probability > 0.2 {
                            Some((key.replace('(', "\\(").replace(')', "\\)"), probability))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect();

            tags.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

            let output_path = input_path.with_extension("txt");
            let mut output_file = File::create(output_path).await?;
            output_file
                .write_all(
                    tags.iter()
                        .map(|(tag, _)| tag.clone())
                        .collect::<Vec<String>>()
                        .join(", ")
                        .as_bytes(),
                )
                .await?;
        }
    }
    Ok(())
} 