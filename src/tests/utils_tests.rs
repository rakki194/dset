use super::super::*;
use anyhow::Result;
use serde_json::json;
use tempfile::tempdir;
use tokio::fs;

#[test]
fn test_split_content() {
    let content = "tag1, tag2, tag3., This is a sentence.";
    let (tags, sentences) = split_content(content);
    
    assert_eq!(tags.len(), 3);
    assert!(tags.contains(&"tag1"));
    assert!(tags.contains(&"tag2"));
    assert!(tags.contains(&"tag3"));
    assert_eq!(sentences, "This is a sentence.");
    
    // Test with no sentences
    let content = "tag1, tag2, tag3";
    let (tags, sentences) = split_content(content);
    assert_eq!(tags.len(), 3);
    assert_eq!(sentences, "");
}

#[tokio::test]
async fn test_format_json_file() -> Result<()> {
    let temp_dir = tempdir()?;
    let test_file = temp_dir.path().join("test.json");
    
    // Create unformatted JSON
    let json = json!({
        "key1": "value1",
        "key2": {"nested": "value2"}
    });
    fs::write(&test_file, json.to_string()).await?;
    
    format_json_file(test_file.clone()).await?;
    
    // Verify the file is properly formatted
    let content = fs::read_to_string(&test_file).await?;
    assert!(content.contains("\n"));  // Should have line breaks
    assert!(content.contains("  "));  // Should have indentation
    
    Ok(())
}

#[tokio::test]
async fn test_process_json_file() -> Result<()> {
    let temp_dir = tempdir()?;
    let test_file = temp_dir.path().join("test.json");
    
    // Create test JSON
    let json = json!({
        "key": "value"
    });
    fs::write(&test_file, json.to_string()).await?;
    
    // Test processor that verifies JSON content
    let processor = |json: &Value| {
        let expected = "value";
        let actual = json["key"].as_str().unwrap();
        assert_eq!(actual, expected);
        async move { Ok(()) }
    };
    
    process_json_file(&test_file, processor).await?;
    
    Ok(())
}

#[tokio::test]
async fn test_process_json_file_invalid() -> Result<()> {
    let temp_dir = tempdir()?;
    let test_file = temp_dir.path().join("test.json");
    
    // Create invalid JSON
    fs::write(&test_file, "invalid json").await?;
    
    let processor = |_: &Value| async { Ok(()) };
    let result = process_json_file(&test_file, processor).await;
    assert!(result.is_err());
    
    Ok(())
} 