#![warn(clippy::all, clippy::pedantic)]

//! Caption processing module for handling both JSON and plain text caption files.
//! 
//! This module provides functionality to process caption files in different formats:
//! - JSON files containing caption data either as direct strings or objects with a "caption" field
//! - Plain text files containing raw caption text
//! 
//! # Example
//! ```no_run
//! use std::path::Path;
//! use dset::caption::process_file;
//! 
//! async fn example() -> anyhow::Result<()> {
//!     let path = Path::new("captions/example.json");
//!     process_file(&path).await?;
//!     Ok(())
//! }
//! ```
//! 
//! The module handles file reading asynchronously and provides error handling for various
//! failure scenarios including file I/O errors and JSON parsing failures.

use serde_json::Value;
use std::path::Path;
use tokio::task;
use regex::Regex;
use std::path::PathBuf;
use std::sync::Arc;

/// Processes a caption file by reading its contents and interpreting them as either JSON or plain text.
///
/// This function attempts to read the file contents and first tries to parse them as JSON.
/// If JSON parsing succeeds, it processes the content as a JSON caption. If parsing fails,
/// it falls back to treating the content as plain text.
///
/// # Arguments
/// * `path` - A reference to the Path of the caption file to process
///
/// # Errors
/// Returns an error if:
/// * The file cannot be read from the filesystem
/// * The file contents cannot be decoded as UTF-8 text
/// * The spawned blocking task fails to complete
///
/// # Example
/// ```no_run
/// use std::path::Path;
/// use dset::caption::process_file;
/// 
/// async fn example() -> anyhow::Result<()> {
///     let path = Path::new("caption.txt");
///     process_file(&path).await?;
///     Ok(())
/// }
/// ```
pub async fn process_file(path: &Path) -> anyhow::Result<()> {
    log::info!("Processing caption file: {}", path.display());

    // Spawn blocking file operations in a separate thread
    let path = path.to_path_buf();
    task::spawn_blocking(move || -> anyhow::Result<()> {
        let content = xio::fs::read_to_string(&path)?;

        // Try to parse as JSON first
        if let Ok(json) = serde_json::from_str::<Value>(&content) {
            println!("JSON caption for {}: {:#?}", path.display(), json);
            return Ok(());
        }

        // If not JSON, treat as plain text
        println!(
            "Plain text caption for {}: {}",
            path.display(),
            content.trim()
        );

        Ok(())
    })
    .await?
}

/// Converts a JSON value into plain text by extracting the caption content.
///
/// This function handles two types of JSON inputs:
/// 1. Direct string values - returns the string directly
/// 2. Objects with a "caption" field - extracts and returns the "caption" field value
///
/// # Arguments
/// * `json` - A reference to a `serde_json` Value containing the caption data
///
/// # Returns
/// * `Ok(String)` - The extracted caption text
/// * `Err` - If the JSON format is not supported or missing required fields
///
/// # Errors
/// Returns an error if:
/// * The JSON value is neither a string nor an object
/// * The JSON object doesn't contain a "caption" field
/// * The "caption" field is not a string value
///
/// # Example
/// ```
/// use serde_json::json;
/// use dset::caption::json_to_text;
/// 
/// # fn main() -> anyhow::Result<()> {
/// let json = json!({"caption": "Hello world"});
/// let text = json_to_text(&json)?;
/// assert_eq!(text, "Hello world");
/// # Ok(())
/// # }
/// ```
pub fn json_to_text(json: &Value) -> anyhow::Result<String> {
    match json {
        Value::String(s) => Ok(s.clone()),
        Value::Object(obj) => {
            if let Some(Value::String(caption)) = obj.get("caption") {
                Ok(caption.clone())
            } else {
                Err(anyhow::anyhow!("No caption field found in JSON object"))
            }
        }
        _ => Err(anyhow::anyhow!("Unsupported JSON format")),
    }
}

/// Checks if a caption file exists and contains non-whitespace content.
///
/// # Arguments
/// * `path` - A reference to the Path of the caption file to check
///
/// # Returns
/// * `true` if the file exists and contains non-whitespace content
/// * `false` if the file doesn't exist, can't be read, or is empty/whitespace-only
///
/// # Example
/// ```no_run
/// use std::path::Path;
/// use dset::caption::caption_file_exists_and_not_empty;
/// 
/// async fn example() -> bool {
///     let path = Path::new("caption.txt");
///     caption_file_exists_and_not_empty(&path).await
/// }
/// ```
pub async fn caption_file_exists_and_not_empty(path: &Path) -> bool {
    if path.exists() {
        match tokio::fs::read_to_string(path).await {
            Ok(content) => !content.trim().is_empty(),
            Err(_) => false,
        }
    } else {
        false
    }
}

/// Patterns of tags to be ignored during e621 tag processing.
pub const IGNORED_E621_TAGS: [&str; 3] = [
    r"\bconditional_dnp\b",
    r"^\d{4}$",   // Years
    r"^\d+:\d+$", // Aspect ratio
];

/// Checks if a tag should be ignored based on predefined patterns.
///
/// # Arguments
///
/// * `tag` - A string slice representing the tag to be checked.
///
/// # Returns
///
/// * `bool` - `true` if the tag matches any pattern in `IGNORED_E621_TAGS`, otherwise `false`.
///
/// # Panics
///
/// This function will panic if any of the predefined patterns in `IGNORED_E621_TAGS`
/// cannot be compiled into a valid regular expression.
#[must_use] pub fn should_ignore_e621_tag(tag: &str) -> bool {
    IGNORED_E621_TAGS.iter().any(|&ignored_tag_pattern| {
        let pattern = Regex::new(ignored_tag_pattern).unwrap();
        pattern.is_match(tag)
    })
}

/// Processes and formats e621 tags from the JSON data.
///
/// # Arguments
///
/// * `tags_dict` - A reference to a JSON Value containing the tags.
///
/// # Returns
///
/// * `Vec<String>` - A vector of strings containing processed and formatted tags.
#[must_use] pub fn process_e621_tags(tags_dict: &Value) -> Vec<String> {
    let mut processed_tags = Vec::new();

    if let Value::Object(tags) = tags_dict {
        // Process artist tags first
        if let Some(Value::Array(artist_tags)) = tags.get("artist") {
            let artist_tags: Vec<String> = artist_tags
                .iter()
                .filter_map(|tag| tag.as_str())
                .filter(|&tag| !should_ignore_e621_tag(tag))
                .map(|tag| format!("by {}", tag.replace('_', " ").replace(" (artist)", "")))
                .collect();
            processed_tags.extend(artist_tags);
        }
        
        // Process character tags
        if let Some(Value::Array(char_tags)) = tags.get("character") {
            let char_tags: Vec<String> = char_tags
                .iter()
                .filter_map(|tag| tag.as_str())
                .filter(|&tag| !should_ignore_e621_tag(tag))
                .map(|tag| tag.replace('_', " "))
                .collect();
            processed_tags.extend(char_tags);
        }
        
        // Process species tags
        if let Some(Value::Array(species_tags)) = tags.get("species") {
            let species_tags: Vec<String> = species_tags
                .iter()
                .filter_map(|tag| tag.as_str())
                .filter(|&tag| !should_ignore_e621_tag(tag))
                .map(|tag| tag.replace('_', " "))
                .collect();
            processed_tags.extend(species_tags);
        }
        
        // Process copyright tags
        if let Some(Value::Array(copyright_tags)) = tags.get("copyright") {
            let copyright_tags: Vec<String> = copyright_tags
                .iter()
                .filter_map(|tag| tag.as_str())
                .filter(|&tag| !should_ignore_e621_tag(tag))
                .map(|tag| tag.replace('_', " "))
                .collect();
            processed_tags.extend(copyright_tags);
        }
        
        // Process general tags last
        if let Some(Value::Array(general_tags)) = tags.get("general") {
            let general_tags: Vec<String> = general_tags
                .iter()
                .filter_map(|tag| tag.as_str())
                .filter(|&tag| {
                    tag.to_lowercase() != "artist" && 
                    !should_ignore_e621_tag(tag)
                })
                .map(|tag| tag.replace('_', " "))
                .collect();
            processed_tags.extend(general_tags);
        }
        
        // Process meta tags - usually just include a few selected ones
        if let Some(Value::Array(meta_tags)) = tags.get("meta") {
            let meta_tags: Vec<String> = meta_tags
                .iter()
                .filter_map(|tag| tag.as_str())
                .filter(|&tag| !should_ignore_e621_tag(tag))
                .map(|tag| tag.replace('_', " "))
                .collect();
            processed_tags.extend(meta_tags);
        }
    }

    processed_tags
}

/// Processes JSON data from e621 and creates a caption file.
///
/// # Arguments
///
/// * `data` - A reference to the JSON Value containing e621 post data
/// * `file_path` - A reference to an Arc<PathBuf> representing the target file path
///
/// # Returns
///
/// * `anyhow::Result<()>` - Success or failure of the operation
///
/// # Panics
///
/// This function will panic if:
/// * The URL in the file data cannot be parsed into a valid file stem
/// * The file stem cannot be converted to a string
///
/// # Errors
///
/// Returns an error if:
/// * The caption file cannot be written to disk
/// * The JSON data structure doesn't match the expected format
///
/// # Example
/// ```no_run
/// use std::path::PathBuf;
/// use std::sync::Arc;
/// use serde_json::json;
/// use dset::caption::process_e621_json_data;
///
/// async fn example() -> anyhow::Result<()> {
///     let data = json!({
///         "post": {
///             "file": {
///                 "url": "https://example.com/image.jpg"
///             },
///             "rating": "s",
///             "tags": {}
///         }
///     });
///     let path = Arc::new(PathBuf::from("output.txt"));
///     process_e621_json_data(&data, &path).await?;
///     Ok(())
/// }
/// ```
pub async fn process_e621_json_data(data: &Value, file_path: &Arc<PathBuf>) -> anyhow::Result<()> {
    if let Some(post) = data.get("post") {
        if let Some(file_data) = post.get("file") {
            if let Some(url) = file_data.get("url").and_then(|u| u.as_str()) {
                use std::path::Path;
                use crate::xio::write_to_file;

                let filename = Path::new(url).file_stem().unwrap().to_str().unwrap();
                let caption_path = file_path.with_file_name(format!("{filename}.txt"));

                let rating = post.get("rating").and_then(|r| r.as_str()).unwrap_or("q");
                let rating_str = match rating {
                    "s" => "safe, ",
                    "e" => "nsfw, ",
                    _ => "questionable, ",
                };

                let mut caption_content = String::from(rating_str);

                if let Some(tags_data) = post.get("tags") {
                    let processed_tags = process_e621_tags(tags_data);

                    if !processed_tags.is_empty() {
                        caption_content.push_str(&processed_tags.join(", "));
                        write_to_file(&caption_path, &caption_content).await?;
                    }
                }
            }
        }
    }
    Ok(())
}

/// Formats text content by removing excessive whitespace and newlines.
///
/// This function cleans up text content by:
/// - Trimming leading/trailing whitespace
/// - Replacing multiple consecutive spaces with a single space
/// - Normalizing line endings
///
/// # Arguments
/// * `content` - A string slice containing the text to format
///
/// # Returns
/// * `anyhow::Result<String>` - The formatted text content
///
/// # Errors
///
/// This function currently does not return any errors, but returns Result
/// for consistency with other functions in the module and to allow for
/// future error handling.
///
/// # Example
/// ```
/// use dset::caption::format_text_content;
///
/// # fn main() -> anyhow::Result<()> {
/// let text = "  This   has  extra   spaces \n\n and newlines  ";
/// let formatted = format_text_content(text)?;
/// assert_eq!(formatted, "This has extra spaces and newlines");
/// # Ok(())
/// # }
/// ```
pub fn format_text_content(content: &str) -> anyhow::Result<String> {
    // Trim and normalize content
    let content = content.trim();
    
    // Replace multiple spaces with a single space
    let content = content
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    
    Ok(content)
}

/// Replaces all instances of a search string with a replacement string in a file.
///
/// This function reads a file, replaces all occurrences of a search string with 
/// a replacement string, and writes the result back to the file if changes were made.
///
/// # Arguments
/// * `path` - A reference to the Path of the file to process
/// * `search` - The string to search for
/// * `replace` - The string to replace with
///
/// # Returns
/// * `anyhow::Result<()>` - Success or failure of the operation
///
/// # Errors
///
/// Returns an error if:
/// * The file cannot be read from the filesystem
/// * The file contents cannot be decoded as UTF-8 text
/// * The modified content cannot be written back to the file
/// * The text content formatting fails when the replacement string is empty
///
/// # Example
/// ```no_run
/// use std::path::Path;
/// use dset::caption::replace_string;
///
/// async fn example() -> anyhow::Result<()> {
///     let path = Path::new("caption.txt");
///     replace_string(path, "old text", "new text").await?;
///     Ok(())
/// }
/// ```
pub async fn replace_string(path: &Path, search: &str, replace: &str) -> anyhow::Result<()> {
    // Skip if search string is empty
    if search.is_empty() {
        return Ok(());
    }

    // Read the file content
    let content = tokio::fs::read_to_string(path).await?;
    
    // Replace the search string with the replacement string
    let mut new_content = content.replace(search, replace);

    // If the replacement string is empty, format the text content
    if replace.is_empty() {
        new_content = format_text_content(&new_content)?;
    }

    // Only write back if there were changes
    if content != new_content {
        tokio::fs::write(path, new_content).await?;
    }

    Ok(())
}

/// Replaces special characters with their keyboard-friendly versions in a file.
///
/// This function reads a file, replaces special characters (like smart quotes) with
/// standard ASCII equivalents, and writes the result back to the file if changes were made.
///
/// # Arguments
/// * `path` - A `PathBuf` to the file to process
///
/// # Returns
/// * `anyhow::Result<()>` - Success or failure of the operation
///
/// # Errors
///
/// Returns an error if:
/// * The file cannot be read from the filesystem
/// * The file contents cannot be decoded as UTF-8 text
/// * The modified content cannot be written back to the file
///
/// # Example
/// ```no_run
/// use std::path::PathBuf;
/// use dset::caption::replace_special_chars;
///
/// async fn example() -> anyhow::Result<()> {
///     let path = PathBuf::from("caption.txt");
///     replace_special_chars(path).await?;
///     Ok(())
/// }
/// ```
pub async fn replace_special_chars(path: PathBuf) -> anyhow::Result<()> {
    // Read the file content
    let content = tokio::fs::read_to_string(&path).await?;
    
    // Replace special characters with their keyboard-friendly versions
    let new_content = content
        .replace(['"', '"'], "\"");

    // Only write back if there were changes
    if content != new_content {
        tokio::fs::write(&path, new_content).await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_process_file_plain_text() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "tag1, tag2, tag3., This is a test caption.")?;

        process_file(&file_path).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_process_file_json() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("test.json");
        let json = json!({
            "caption": "A test caption",
            "tags": ["tag1", "tag2"]
        });
        fs::write(&file_path, serde_json::to_string_pretty(&json)?)?;

        process_file(&file_path).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_process_file_invalid_json() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("invalid.json");
        fs::write(&file_path, "{ invalid json }")?;

        // Should handle invalid JSON gracefully by treating it as plain text
        process_file(&file_path).await?;
        Ok(())
    }

    #[test]
    fn test_json_to_text_string() -> anyhow::Result<()> {
        let json = json!("Test caption");
        let text = json_to_text(&json)?;
        assert_eq!(text, "Test caption");
        Ok(())
    }

    #[test]
    fn test_json_to_text_object() -> anyhow::Result<()> {
        let json = json!({
            "caption": "Test caption",
            "other_field": "ignored"
        });
        let text = json_to_text(&json)?;
        assert_eq!(text, "Test caption");
        Ok(())
    }

    #[test]
    fn test_json_to_text_invalid_object() {
        let json = json!({
            "not_caption": "Test caption"
        });
        assert!(json_to_text(&json).is_err());
    }

    #[test]
    fn test_json_to_text_unsupported_format() {
        let json = json!(42);
        assert!(json_to_text(&json).is_err());
    }

    #[tokio::test]
    async fn test_caption_file_exists_and_not_empty() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        
        // Test non-existent file
        let non_existent = temp_dir.path().join("non_existent.txt");
        assert!(!caption_file_exists_and_not_empty(&non_existent).await);
        
        // Test empty file
        let empty_file = temp_dir.path().join("empty.txt");
        fs::write(&empty_file, "")?;
        assert!(!caption_file_exists_and_not_empty(&empty_file).await);
        
        // Test whitespace-only file
        let whitespace_file = temp_dir.path().join("whitespace.txt");
        fs::write(&whitespace_file, "   \n  \t  ")?;
        assert!(!caption_file_exists_and_not_empty(&whitespace_file).await);
        
        // Test file with content
        let content_file = temp_dir.path().join("content.txt");
        fs::write(&content_file, "This is a caption")?;
        assert!(caption_file_exists_and_not_empty(&content_file).await);
        
        Ok(())
    }
}
