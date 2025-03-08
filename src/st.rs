#![warn(clippy::all, clippy::pedantic)]

use anyhow::Context;
use memmap2::Mmap;
use safetensors::SafeTensors;
use serde_json::Value;
use std::{fs::File, path::Path};
use tokio::task;

/// Process a safetensors file and extract its embedded metadata to a JSON file
///
/// # Errors
/// Returns an error if:
/// - Failed to open the file
/// - Failed to memory map the file
/// - Failed to read the safetensors header
/// - Failed to write the metadata JSON file
pub async fn process_file(path: &Path) -> anyhow::Result<()> {
    log::info!("Processing file: {}", path.display());

    // Spawn blocking file operations in a separate thread
    let path = path.to_path_buf();
    task::spawn_blocking(move || -> anyhow::Result<()> {
        let file = File::open(&path)
            .with_context(|| format!("Failed to open file: {}", path.display()))?;

        // Safety: The file is opened read-only and won't be modified while mapped
        let mmap = unsafe { Mmap::map(&file) }
            .with_context(|| format!("Failed to memory map file: {}", path.display()))?;

        let (_header_size, metadata) = SafeTensors::read_metadata(&mmap)
            .with_context(|| format!("Failed to read metadata from file: {}", path.display()))?;

        // Debug print the raw metadata
        log::info!("Raw metadata: {:?}", metadata);

        // Convert the raw metadata to a JSON value
        let metadata_json: Value =
            serde_json::to_value(&metadata).context("Failed to convert metadata to JSON value")?;

        // Extract metadata from the __metadata__ field if it exists
        let metadata_to_process = if let Some(meta) = metadata_json.get("__metadata__") {
            if let Some(meta_str) = meta.get("metadata") {
                if let Some(s) = meta_str.as_str() {
                    serde_json::from_str(s).unwrap_or(Value::Object(serde_json::Map::new()))
                } else {
                    Value::Object(serde_json::Map::new())
                }
            } else {
                Value::Object(serde_json::Map::new())
            }
        } else {
            Value::Object(serde_json::Map::new())
        };

        // Process the metadata
        let processed_metadata = crate::metadata::extract_training_metadata(&metadata_to_process);

        // Write metadata to JSON file
        let json_path = path.with_extension("metadata.json");
        std::fs::write(
            &json_path,
            serde_json::to_string_pretty(&processed_metadata)
                .context("Failed to serialize metadata to JSON")?,
        )
        .with_context(|| format!("Failed to write metadata to {}", json_path.display()))?;

        if processed_metadata
            .as_object()
            .is_none_or(serde_json::Map::is_empty)
        {
            log::info!("No training metadata found in {}", path.display());
        } else {
            log::info!("Wrote metadata to {}", json_path.display());
        }
        Ok(())
    })
    .await?
}

/// Inspects the state dictionary of a targeted safensor file.
///
/// This function reads the state dictionary from the specified safensor file
/// and returns it as a JSON value.
///
/// # Arguments
///
/// * `path` - The path to the safensor file to inspect.
///
/// # Returns
///
/// Returns a `Result<Value>` containing the state dictionary as a JSON value
/// or an error if the operation fails.
///
/// # Errors
///
/// Returns an error if:
/// - The file cannot be read
/// - The content cannot be parsed as JSON
pub fn inspect_state_dict(path: &Path) -> anyhow::Result<Value> {
    // Read the content of the safensor file as binary
    let file = File::open(path).context("Failed to open safensor file")?;
    let mmap = unsafe { Mmap::map(&file) }.context("Failed to memory map safensor file")?;

    // Read the state dictionary from the memory-mapped file
    let (_header_size, metadata) =
        SafeTensors::read_metadata(&mmap).context("Failed to read metadata from safensor file")?;

    // Convert the raw metadata to a JSON value
    let state_dict: Value = serde_json::to_value(&metadata)
        .context("Failed to convert state dictionary to JSON value")?;

    Ok(state_dict)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn create_test_safetensor(dir: &TempDir, metadata: &str) -> anyhow::Result<PathBuf> {
        let file_path = dir.path().join("test.safetensors");

        // Create a minimal safetensors file with metadata
        let mut file = fs::File::create(&file_path)?;

        // Verify the metadata is valid JSON
        serde_json::from_str::<serde_json::Value>(metadata)?;

        // Create a valid safetensors header with metadata and tensor info
        let header = serde_json::json!({
            "__metadata__": {
                "metadata": metadata
            },
            "test_tensor": {
                "dtype": "F32",
                "shape": [1],
                "data_offsets": [0, 4]  // 4 bytes for one f32
            }
        });

        // Convert header to compact string (no pretty print)
        let header_str = serde_json::to_string(&header)?;
        let header_bytes = header_str.as_bytes();

        // Write header size as 64-bit little endian
        let header_size = (header_bytes.len() as u64).to_le_bytes();
        file.write_all(&header_size)?;

        // Write the header itself
        file.write_all(header_bytes)?;

        // Write tensor data (4 bytes for a single f32)
        file.write_all(&0f32.to_le_bytes())?;

        Ok(file_path)
    }

    #[tokio::test]
    async fn test_process_file_with_metadata() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        let metadata = r#"{
            "ss_bucket_info": {
                "buckets": {
                    "0": {
                        "resolution": [1280, 800],
                        "count": 78
                    }
                },
                "mean_img_ar_error": 0.0
            }
        }"#;

        let file_path = create_test_safetensor(&temp_dir, metadata)?;
        process_file(&file_path).await?;

        // Verify the metadata JSON file was created
        let json_path = file_path.with_extension("metadata.json");
        assert!(json_path.exists());

        // Verify the content
        let content = fs::read_to_string(json_path)?;
        let json: Value = serde_json::from_str(&content)?;
        assert!(json.get("ss_bucket_info").is_some());

        Ok(())
    }

    #[tokio::test]
    async fn test_process_file_empty_metadata() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = create_test_safetensor(&temp_dir, "{}")?;

        process_file(&file_path).await?;

        // Verify the metadata JSON file was created
        let json_path = file_path.with_extension("metadata.json");
        assert!(json_path.exists());

        // Verify the content is an empty object
        let content = fs::read_to_string(json_path)?;
        let json: Value = serde_json::from_str(&content)?;
        assert!(json.as_object().unwrap().is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn test_process_file_invalid_path() {
        let result = process_file(Path::new("nonexistent.safetensors")).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_process_file_complex_metadata() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        let metadata = r#"{
            "ss_network_args": {
                "network_alpha": 128,
                "network_dim": 64,
                "network_module": "networks.lora"
            },
            "ss_tag_frequency": {
                "tag1": 0.8,
                "tag2": 0.5
            },
            "ss_dataset_dirs": [
                "/path/to/dataset1",
                "/path/to/dataset2"
            ]
        }"#;

        let file_path = create_test_safetensor(&temp_dir, metadata)?;
        process_file(&file_path).await?;

        // Verify the metadata JSON file was created and contains expected fields
        let json_path = file_path.with_extension("metadata.json");
        let content = fs::read_to_string(json_path)?;
        let json: Value = serde_json::from_str(&content)?;

        assert!(json.get("ss_network_args").is_some());
        assert!(json.get("ss_tag_frequency").is_some());
        assert!(json.get("ss_dataset_dirs").is_some());

        Ok(())
    }

    #[tokio::test]
    async fn test_inspect_state_dict() -> anyhow::Result<()> {
        // Create a temporary directory for the test
        let temp_dir = TempDir::new()?;
        
        // Create a test safetensors file with complex metadata
        let metadata = r#"{
            "ss_network_args": {
                "network_alpha": 128,
                "network_dim": 64,
                "network_module": "networks.lora"
            },
            "ss_tag_frequency": {
                "tag1": 0.8,
                "tag2": 0.5
            }
        }"#;
        
        // Add tensor definitions to validate shape extraction
        let file_path = temp_dir.path().join("test_model.safetensors");
        let mut file = fs::File::create(&file_path)?;
        
        // Create header with multiple tensors of different shapes
        let header = serde_json::json!({
            "__metadata__": {
                "metadata": metadata
            },
            "lora_up.weight": {
                "dtype": "F32",
                "shape": [768, 64],
                "data_offsets": [0, 196608]  // 768*64*4 = 196608 bytes
            },
            "lora_down.weight": {
                "dtype": "F16",
                "shape": [64, 768],
                "data_offsets": [196608, 294912]  // 64*768*2 = 98304 bytes
            },
            "conv.bias": {
                "dtype": "F32",
                "shape": [32],
                "data_offsets": [294912, 295040]  // 32*4 = 128 bytes
            }
        });
        
        // Write header
        let header_str = serde_json::to_string(&header)?;
        let header_bytes = header_str.as_bytes();
        let header_size = (header_bytes.len() as u64).to_le_bytes();
        file.write_all(&header_size)?;
        file.write_all(header_bytes)?;
        
        // Write some dummy tensor data (all zeros for simplicity)
        // We need at least 295040 bytes of tensor data based on the offsets
        file.write_all(&vec![0u8; 295040])?;
        
        // Test the inspect_state_dict function
        let state_dict = inspect_state_dict(&file_path)?;
        
        // Verify the results
        assert!(state_dict.is_object());
        let obj = state_dict.as_object().unwrap();
        
        // Check that we have all expected tensors
        assert!(obj.contains_key("__metadata__"));
        assert!(obj.contains_key("lora_up.weight"));
        assert!(obj.contains_key("lora_down.weight"));
        assert!(obj.contains_key("conv.bias"));
        
        // Check tensor shapes
        let up_weight = obj.get("lora_up.weight").unwrap();
        assert_eq!(up_weight.get("dtype").unwrap().as_str().unwrap(), "F32");
        assert_eq!(
            up_weight.get("shape").unwrap().as_array().unwrap(),
            &[serde_json::json!(768), serde_json::json!(64)]
        );
        
        let down_weight = obj.get("lora_down.weight").unwrap();
        assert_eq!(down_weight.get("dtype").unwrap().as_str().unwrap(), "F16");
        assert_eq!(
            down_weight.get("shape").unwrap().as_array().unwrap(),
            &[serde_json::json!(64), serde_json::json!(768)]
        );
        
        // Check metadata extraction
        let metadata_field = obj.get("__metadata__").unwrap();
        let metadata_content = metadata_field.get("metadata").unwrap().as_str().unwrap();
        assert!(metadata_content.contains("network_alpha"));
        
        Ok(())
    }
}
