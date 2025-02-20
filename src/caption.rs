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
/// * `json` - A reference to a serde_json Value containing the caption data
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
}
