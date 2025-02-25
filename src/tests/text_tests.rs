#![warn(clippy::all, clippy::pedantic)]

use crate::caption::{format_text_content, replace_special_chars, replace_string};
use tempfile::TempDir;
use tokio::fs;

#[test]
fn test_format_text_content() -> anyhow::Result<()> {
    // Test with multiple spaces
    let text = "This   has  too   many    spaces";
    let formatted = format_text_content(text)?;
    assert_eq!(formatted, "This has too many spaces");
    
    // Test with newlines
    let text = "Line 1\nLine 2\n\nLine 3";
    let formatted = format_text_content(text)?;
    assert_eq!(formatted, "Line 1 Line 2 Line 3");
    
    // Test with tabs
    let text = "Text\twith\ttabs";
    let formatted = format_text_content(text)?;
    assert_eq!(formatted, "Text with tabs");
    
    // Test with leading/trailing whitespace
    let text = "  \t  Text with spaces around   \n  ";
    let formatted = format_text_content(text)?;
    assert_eq!(formatted, "Text with spaces around");
    
    // Test with a mix of whitespace characters
    let text = "  Mixed \t spacing  \n and \r newlines  ";
    let formatted = format_text_content(text)?;
    assert_eq!(formatted, "Mixed spacing and newlines");
    
    // Test with empty string
    let text = "";
    let formatted = format_text_content(text)?;
    assert_eq!(formatted, "");
    
    // Test with only whitespace
    let text = "   \t \n   ";
    let formatted = format_text_content(text)?;
    assert_eq!(formatted, "");
    
    Ok(())
}

#[tokio::test]
async fn test_replace_string() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test_replace.txt");
    
    // Create test file
    let original_content = "This is a test string. This string should be replaced.";
    fs::write(&file_path, original_content).await?;
    
    // Test basic string replacement
    replace_string(&file_path, "test", "sample").await?;
    let content = fs::read_to_string(&file_path).await?;
    assert_eq!(content, "This is a sample string. This string should be replaced.");
    
    // Test replacing multiple occurrences
    replace_string(&file_path, "string", "text").await?;
    let content = fs::read_to_string(&file_path).await?;
    assert_eq!(content, "This is a sample text. This text should be replaced.");
    
    // Test replacing with empty string - should also format content
    replace_string(&file_path, "This is a ", "").await?;
    let content = fs::read_to_string(&file_path).await?;
    assert_eq!(content, "sample text. This text should be replaced.");
    
    // Test when search string is not found - file should remain unchanged
    let before = fs::read_to_string(&file_path).await?;
    replace_string(&file_path, "nonexistent", "replacement").await?;
    let after = fs::read_to_string(&file_path).await?;
    assert_eq!(before, after);
    
    // Test with empty search string - file should remain unchanged
    let before = fs::read_to_string(&file_path).await?;
    replace_string(&file_path, "", "replacement").await?;
    let after = fs::read_to_string(&file_path).await?;
    assert_eq!(before, after);
    
    Ok(())
}

#[tokio::test]
async fn test_replace_special_chars() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test_special_chars.txt");
    
    // Create test file with special characters
    let special_chars_content = "Text with 'smart quotes' and \"double quotes\".";
    fs::write(&file_path, special_chars_content).await?;
    
    // Process the file
    replace_special_chars(file_path.clone()).await?;
    
    // Check the content
    let content = fs::read_to_string(&file_path).await?;
    
    // Verify smart quotes are replaced with straight quotes
    assert_eq!(content, "Text with 'smart quotes' and \"double quotes\".");
    
    // Test when file has no special characters
    let no_special_chars = "Text with regular 'quotes' and \"quotes\".";
    fs::write(&file_path, no_special_chars).await?;
    
    // Get the last modified time
    let metadata_before = fs::metadata(&file_path).await?;
    let modified_before = metadata_before.modified()?;
    
    // Process the file - should not modify it since no changes needed
    replace_special_chars(file_path.clone()).await?;
    
    // Get the last modified time after processing
    let metadata_after = fs::metadata(&file_path).await?;
    let modified_after = metadata_after.modified()?;
    
    // Content should be unchanged
    let content = fs::read_to_string(&file_path).await?;
    assert_eq!(content, "Text with regular 'quotes' and \"quotes\".");
    
    // Modified time should be the same (or very close) if file wasn't changed
    // Note: Some filesystems have different timestamp precision
    let diff = modified_after.duration_since(modified_before).unwrap_or_default();
    assert!(diff.as_secs() < 1, "File should not have been modified");
    
    Ok(())
}

#[tokio::test]
async fn test_replace_string_formatting() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test_formatting.txt");
    
    // Create test file with excess whitespace
    let content = "  This   has  too   many    spaces  \n\n  and newlines  ";
    fs::write(&file_path, content).await?;
    
    // Replace with empty string should trigger formatting
    replace_string(&file_path, "too   many", "").await?;
    
    // Check the content
    let result = fs::read_to_string(&file_path).await?;
    assert_eq!(result, "This has spaces and newlines");
    
    Ok(())
}

#[tokio::test]
async fn test_replace_special_chars_complex() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("complex_special_chars.txt");
    
    // Create test file with a mix of special characters and regular text
    let content = "Here's a mix of 'smart quotes', \"double quotes\" and regular quotes: 'normal', \"normal\".";
    fs::write(&file_path, content).await?;
    
    // Process the file
    replace_special_chars(file_path.clone()).await?;
    
    // Check the content - only smart quotes should be replaced, preserving the rest of the text exactly
    let result = fs::read_to_string(&file_path).await?;
    assert_eq!(result, "Here's a mix of 'smart quotes', \"double quotes\" and regular quotes: 'normal', \"normal\".");
    
    Ok(())
}

#[tokio::test]
async fn test_integration_text_processing() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("integration_test.txt");
    
    // Create a file with various text issues to fix
    let content = "  This   text has 'smart quotes' and   excess  \n\n whitespace.";
    fs::write(&file_path, content).await?;
    
    // First replace a string
    replace_string(&file_path, "text", "document").await?;
    
    // Then replace special characters
    replace_special_chars(file_path.clone()).await?;
    
    // Finally, remove some text and trigger formatting
    replace_string(&file_path, "This   document has ", "").await?;
    
    // Check the final result
    let result = fs::read_to_string(&file_path).await?;
    assert_eq!(result, "'smart quotes' and excess whitespace.");
    
    Ok(())
} 