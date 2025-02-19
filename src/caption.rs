use serde_json::Value;
use std::path::Path;
use tokio::task;

/// Process a caption file
/// 
/// # Errors
/// Returns an error if:
/// - Failed to read the file
/// - Failed to parse JSON (if file is in JSON format)
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
        println!("Plain text caption for {}: {}", path.display(), content.trim());
        
        Ok(())
    }).await?
}

/// Convert JSON caption to plain text
/// 
/// # Errors
/// Returns an error if:
/// - JSON is not a string or object with a "caption" field
/// - The "caption" field is not a string
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
    use tempfile::TempDir;
    use std::fs;
    use serde_json::json;

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