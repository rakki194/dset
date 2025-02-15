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