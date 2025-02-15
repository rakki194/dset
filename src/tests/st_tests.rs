use super::super::*;
use anyhow::Result;
use tempfile::tempdir;
use tokio::fs;
use safetensors::{tensor::TensorView, Dtype, serialize};
use std::collections::HashMap;

fn create_test_safetensors(path: &std::path::Path) -> Result<()> {
    // Create a simple tensor with metadata
    let mut tensors = HashMap::new();
    let data: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0];
    // Convert f32 data to bytes first
    let data_bytes: Vec<u8> = data.iter()
        .flat_map(|x| x.to_le_bytes())
        .collect();
    let view = TensorView::new(
        Dtype::F32,
        vec![2, 2],
        &data_bytes,
    )?;
    tensors.insert("test_tensor".to_string(), view);

    // Add metadata
    let mut metadata = HashMap::new();
    metadata.insert("model_name".to_string(), "test_model".to_string());
    metadata.insert("version".to_string(), "1.0".to_string());
    let metadata = Some(metadata);

    // Create the header and write to file
    let serialized = serialize(&tensors, &metadata)?;
    std::fs::write(path, serialized)?;
    
    Ok(())
}

#[tokio::test]
async fn test_process_file() -> Result<()> {
    let temp_dir = tempdir()?;
    let test_file = temp_dir.path().join("test.safetensors");
    let expected_json_file = test_file.with_extension("metadata.json");
    
    create_test_safetensors(&test_file)?;
    
    st::process_file(&test_file).await?;
    
    // Verify the JSON file was created
    assert!(expected_json_file.exists());
    let json_content = fs::read_to_string(&expected_json_file).await?;
    let json: serde_json::Value = serde_json::from_str(&json_content)?;
    
    assert!(json.is_object());
    if let serde_json::Value::Object(obj) = json {
        assert_eq!(obj.get("model_name").unwrap().as_str().unwrap(), "test_model");
        assert_eq!(obj.get("version").unwrap().as_str().unwrap(), "1.0");
    }
    
    Ok(())
}

#[tokio::test]
async fn test_process_file_invalid() -> Result<()> {
    let temp_dir = tempdir()?;
    let test_file = temp_dir.path().join("invalid.safetensors");
    
    // Create an invalid safetensors file
    fs::write(&test_file, "invalid data").await?;
    
    let result = st::process_file(&test_file).await;
    assert!(result.is_err());
    
    Ok(())
} 