use anyhow::Context;
use safetensors::SafeTensors;
use std::{fs::File, path::Path};
use memmap2::Mmap;
use tokio::task;
use serde_json::Value;

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

        // Extract just the __metadata__ contents if it exists
        let metadata_json: Value = serde_json::to_value(&metadata)
            .context("Failed to convert metadata to JSON value")?;
        
        let training_metadata = if let Value::Object(obj) = metadata_json {
            if let Some(Value::String(meta_str)) = obj.get("__metadata__") {
                // Parse the metadata string into a JSON object
                match serde_json::from_str(meta_str) {
                    Ok(parsed) => parsed,
                    Err(_) => {
                        // If parsing fails, keep the original string
                        let mut map = serde_json::Map::new();
                        map.insert("invalid_json".to_string(), Value::String(meta_str.to_string()));
                        Value::Object(map)
                    }
                }
            } else {
                // No __metadata__ field, use the entire metadata object
                Value::Object(obj)
            }
        } else {
            Value::Object(serde_json::Map::new())
        };

        // Process any JSON-encoded strings in the metadata
        let mut processed_metadata = if let Value::Object(obj) = training_metadata {
            let mut new_obj = serde_json::Map::new();
            for (key, value) in obj {
                if let Value::String(s) = &value {
                    let trimmed = s.trim();
                    if trimmed == "None" {
                        new_obj.insert(key, Value::Null);
                    } else if (trimmed.starts_with('{') && trimmed.ends_with('}')) || 
                              (trimmed.starts_with('[') && trimmed.ends_with(']')) {
                        if let Ok(decoded) = serde_json::from_str(trimmed) {
                            new_obj.insert(key, decoded);
                        } else {
                            new_obj.insert(key, value);
                        }
                    } else {
                        new_obj.insert(key, value);
                    }
                } else {
                    new_obj.insert(key, value);
                }
            }
            Value::Object(new_obj)
        } else {
            training_metadata
        };

        // Write metadata to JSON file
        let json_path = path.with_extension("metadata.json");
        std::fs::write(&json_path, 
            serde_json::to_string_pretty(&processed_metadata)
                .context("Failed to serialize metadata to JSON")?
        ).with_context(|| format!("Failed to write metadata to {}", json_path.display()))?;

        if processed_metadata.as_object().map_or(true, |obj| obj.is_empty()) {
            log::info!("No training metadata found in {}", path.display());
        } else {
            log::info!("Wrote metadata to {}", json_path.display());
        }
        Ok(())
    }).await?
} 