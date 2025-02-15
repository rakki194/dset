use super::super::*;
use anyhow::Result;
use serde_json::json;
use tempfile::tempdir;
use tokio::fs;

#[tokio::test]
async fn test_process_caption_file_json() -> Result<()> {
    let temp_dir = tempdir()?;
    let test_file = temp_dir.path().join("test.txt");
    
    // Create a test JSON caption file
    let caption_json = json!({
        "caption": "A test caption"
    });
    fs::write(&test_file, caption_json.to_string()).await?;
    
    caption::process_file(&test_file).await?;
    
    Ok(())
}

#[tokio::test]
async fn test_process_caption_file_text() -> Result<()> {
    let temp_dir = tempdir()?;
    let test_file = temp_dir.path().join("test.txt");
    
    // Create a test plain text caption file
    let caption_text = "This is a plain text caption";
    fs::write(&test_file, caption_text).await?;
    
    caption::process_file(&test_file).await?;
    
    Ok(())
}

#[test]
fn test_json_to_text_string() -> Result<()> {
    let json = json!("Test caption");
    let result = caption::json_to_text(&json)?;
    assert_eq!(result, "Test caption");
    Ok(())
}

#[test]
fn test_json_to_text_object() -> Result<()> {
    let json = json!({
        "caption": "Test caption"
    });
    let result = caption::json_to_text(&json)?;
    assert_eq!(result, "Test caption");
    Ok(())
}

#[test]
fn test_json_to_text_invalid() {
    let json = json!({ "not_caption": "Test" });
    assert!(caption::json_to_text(&json).is_err());
    
    let json = json!(42);
    assert!(caption::json_to_text(&json).is_err());
}

#[tokio::test]
async fn test_process_json_to_caption() -> Result<()> {
    let temp_dir = tempdir()?;
    let input_file = temp_dir.path().join("test.json");
    let output_file = input_file.with_extension("txt");
    
    // Create test JSON file with tag probabilities
    let json = json!({
        "tag1": 0.9,
        "tag2": 0.8,
        "tag3": 0.1  // Should be excluded due to low probability
    });
    fs::write(&input_file, json.to_string()).await?;
    
    process_json_to_caption(&input_file).await?;
    
    // Verify output
    assert!(output_file.exists());
    let content = fs::read_to_string(&output_file).await?;
    assert!(content.contains("tag1"));
    assert!(content.contains("tag2"));
    assert!(!content.contains("tag3"));  // Should be excluded
    
    Ok(())
} 