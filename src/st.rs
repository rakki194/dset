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

        // Extract just the __metadata__ contents if it exists
        let metadata_json: Value = serde_json::to_value(&metadata)
            .context("Failed to convert metadata to JSON value")?;
        
        let mut training_metadata = if let Value::Object(obj) = metadata_json {
            if let Some(Value::Object(meta)) = obj.get("__metadata__") {
                Value::Object(meta.clone())
            } else {
                Value::Object(serde_json::Map::new())
            }
        } else {
            Value::Object(serde_json::Map::new())
        };

        // Decode JSON-encoded strings in the metadata
        if let Value::Object(ref mut meta_obj) = training_metadata {
            // Try to decode any string field that looks like JSON
            let fields_to_decode: Vec<String> = meta_obj.iter()
                .filter_map(|(key, value)| {
                    if let Value::String(s) = value {
                        let trimmed = s.trim();
                        if (trimmed.starts_with('{') && trimmed.ends_with('}')) || 
                           (trimmed.starts_with('[') && trimmed.ends_with(']')) {
                            return Some(key.clone());
                        }
                    }
                    None
                })
                .collect();
            
            for field in fields_to_decode {
                if let Some(Value::String(json_str)) = meta_obj.get(&field) {
                    if let Ok(decoded) = serde_json::from_str(json_str) {
                        meta_obj.insert(field.clone(), decoded);
                    }
                }
            }
        }

        // Write metadata to JSON file
        let json_path = path.with_extension("metadata.json");
        std::fs::write(&json_path, 
            serde_json::to_string_pretty(&training_metadata)
                .context("Failed to serialize metadata to JSON")?
        ).with_context(|| format!("Failed to write metadata to {}", json_path.display()))?;

        if training_metadata.as_object().is_some_and(serde_json::Map::is_empty) {
            log::info!("No training metadata found in {}", path.display());
        } else {
            log::info!("Wrote metadata to {}", json_path.display());
        }
        Ok(())
    }).await?
} 