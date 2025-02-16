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

        // Convert the raw metadata to a JSON value
        let metadata_json: Value = serde_json::to_value(&metadata)
            .context("Failed to convert metadata to JSON value")?;
        
        // Use the new helper function to extract and recursively decode JSON fields
        let processed_metadata = crate::metadata::extract_training_metadata(&metadata_json);

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