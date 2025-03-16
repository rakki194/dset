#![warn(clippy::all, clippy::pedantic)]

//! Functionality for concatenating multiple files with different extensions.
//!
//! This module provides utilities to concatenate the contents of files with different
//! extensions (e.g., .caption, .wd, .tags) into a single output file (e.g., .txt).
//! It supports both predefined presets and custom extension combinations.
//!
//! # Example
//! ```no_run
//! use std::path::Path;
//! use dset::concat::{ConcatConfig, FileExtensionPreset, concat_files};
//!
//! async fn example() -> anyhow::Result<()> {
//!     // Use a preset configuration
//!     let config = ConcatConfig::from_preset(FileExtensionPreset::CaptionWdTags);
//!     concat_files(Path::new("./images"), &config, false).await?;
//!     Ok(())
//! }
//! ```

use std::collections::{HashSet, HashMap};
use std::path::Path;
use anyhow::{Context, Result};
use log::{debug, info, warn};
use tokio::fs;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use md5;

/// Predefined presets for file extension combinations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileExtensionPreset {
    /// Concatenates .caption, .wd, .tags files into .txt
    CaptionWdTags,
    /// Concatenates .florence, .wd, .tags files
    FlorenceWdTags,
}

impl fmt::Display for FileExtensionPreset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CaptionWdTags => write!(f, "caption+wd+tags"),
            Self::FlorenceWdTags => write!(f, "florence+wd+tags"),
        }
    }
}

/// Configuration for file concatenation
///
/// This configuration controls how files are concatenated, with the following behavior:
/// - Base extensions define which files to look for (e.g., jpg, png)
/// - Extensions to concatenate define which related files to process (e.g., caption, wd, tags)
/// - Caption files (with extension "caption" or "florence") are treated specially:
///   - Their content is appended after the concatenated tags
///   - They aren't included in tag deduplication
/// - When remove_duplicates is true, tags from non-caption files are deduplicated
///
/// # Example
/// ```no_run
/// use dset::concat::ConcatConfig;
///
/// let config = ConcatConfig {
///     base_extensions: vec!["jpg".into()],
///     extensions_to_concat: vec!["caption".into(), "wd".into(), "tags".into()],
///     output_extension: "txt".into(),
///     remove_duplicates: true,
///     tag_separator: ", ".into(),
///     deduplicate_files: false,
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConcatConfig {
    /// Base file extensions to find (without the dot)
    pub base_extensions: Vec<String>,
    /// Extensions to concatenate (without the dot)
    pub extensions_to_concat: Vec<String>,
    /// Output file extension (without the dot)
    pub output_extension: String,
    /// Set to true to remove duplicate tags
    pub remove_duplicates: bool,
    /// Tag separator to use when concatenating
    pub tag_separator: String,
    /// Set to true to deduplicate files with identical content
    pub deduplicate_files: bool,
}

impl ConcatConfig {
    /// Creates a new configuration with the specified parameters
    #[must_use]
    pub fn new(
        base_extensions: Vec<String>,
        extensions_to_concat: Vec<String>,
        output_extension: String,
        remove_duplicates: bool,
        tag_separator: String,
    ) -> Self {
        Self {
            base_extensions,
            extensions_to_concat,
            output_extension,
            remove_duplicates,
            tag_separator,
            deduplicate_files: false,
        }
    }

    /// Creates a configuration with deduplication enabled
    #[must_use]
    pub fn with_deduplication(mut self, deduplicate: bool) -> Self {
        self.deduplicate_files = deduplicate;
        self
    }

    /// Creates a configuration from a predefined preset
    #[must_use]
    pub fn from_preset(preset: FileExtensionPreset) -> Self {
        match preset {
            FileExtensionPreset::CaptionWdTags => Self {
                base_extensions: vec!["png".into(), "jpg".into(), "jpeg".into(), "webp".into(), 
                                    "gif".into(), "tiff".into(), "bmp".into(), "jxl".into(), "avif".into()],
                extensions_to_concat: vec!["caption".into(), "wd".into(), "tags".into()],
                output_extension: "txt".into(),
                remove_duplicates: true,
                tag_separator: ", ".into(),
                deduplicate_files: false,
            },
            FileExtensionPreset::FlorenceWdTags => Self {
                base_extensions: vec!["png".into(), "jpg".into(), "jpeg".into(), "webp".into(), 
                                    "gif".into(), "tiff".into(), "bmp".into(), "jxl".into(), "avif".into()],
                extensions_to_concat: vec!["florence".into(), "wd".into(), "tags".into()],
                output_extension: "txt".into(),
                remove_duplicates: true,
                tag_separator: ", ".into(),
                deduplicate_files: false,
            },
        }
    }
}

/// Reads the content of a file as a string
async fn read_file_content(path: &Path) -> Result<String> {
    let content = fs::read_to_string(path)
        .await
        .with_context(|| format!("Failed to read file: {}", path.display()))?;
    Ok(content.trim().to_string())
}

/// Concatenates tags from multiple files, with special handling for caption files.
///
/// This function processes tag files and caption files differently:
/// 1. Identifies which file is the caption file based on extension (.caption or .florence)
/// 2. Extracts and optionally deduplicates tags from all non-caption files
/// 3. Appends the caption content after the deduplicated tags
///
/// The resulting format is: "tag1, tag2, tag3, caption_content"
///
/// # Arguments
/// * `contents` - Contents of each file to concatenate
/// * `config` - Configuration for concatenation
/// * `file_paths` - Paths of files being concatenated (used to identify the caption file)
///
/// # Returns
/// A string containing the concatenated content
fn concat_tags(contents: &[String], config: &ConcatConfig, file_paths: &[std::path::PathBuf]) -> String {
    if contents.is_empty() {
        return String::new();
    }
    
    // Find which file is the caption file based on extension
    let caption_ext = if config.extensions_to_concat.contains(&"caption".to_string()) {
        "caption"
    } else if config.extensions_to_concat.contains(&"florence".to_string()) {
        "florence"
    } else {
        // If there's no caption or florence extension, use the last one
        config.extensions_to_concat.last().unwrap()
    };
    
    // Find the index of the caption file 
    let mut caption_index = None;
    for (i, path) in file_paths.iter().enumerate() {
        if let Some(ext) = path.extension() {
            if ext == caption_ext {
                caption_index = Some(i);
                break;
            }
        }
    }
    
    // Default to the last file if we couldn't determine which is the caption
    let caption_index = caption_index.unwrap_or(contents.len() - 1);
    let caption_content = &contents[caption_index];
    
    // Process all tag file contents (all except the caption file)
    let mut unique_tags = HashSet::new();
    let mut all_tags = Vec::new();
    
    for (i, content) in contents.iter().enumerate() {
        // Skip the caption file
        if i == caption_index {
            continue;
        }
        
        // Split by comma and trim each tag
        let tags = content.split(',')
            .map(|tag| tag.trim())
            .filter(|&tag| !tag.is_empty());
            
        for tag in tags {
            if config.remove_duplicates {
                unique_tags.insert(tag.to_string());
            } else {
                all_tags.push(tag.to_string());
            }
        }
    }
    
    // Format the tag portion
    let tags_portion = if config.remove_duplicates {
        let mut sorted_tags: Vec<_> = unique_tags.into_iter().collect();
        sorted_tags.sort();
        sorted_tags.join(&config.tag_separator)
    } else {
        all_tags.join(&config.tag_separator)
    };
    
    // Combine tags with caption
    if tags_portion.is_empty() {
        caption_content.clone()
    } else if caption_content.is_empty() {
        tags_portion
    } else {
        format!("{}{}{}", tags_portion, config.tag_separator, caption_content)
    }
}

/// Processes a single image file, looking for corresponding files to concatenate
pub async fn process_image_file(
    image_path: &Path, 
    config: &ConcatConfig, 
    dry_run: bool
) -> Result<bool> {
    // Get the stem of the image file (filename without extension)
    let stem = image_path.file_stem()
        .with_context(|| format!("Failed to get file stem from: {}", image_path.display()))?
        .to_string_lossy();
    
    let parent = image_path.parent()
        .with_context(|| format!("Failed to get parent directory of: {}", image_path.display()))?;
    
    // Check if all required files exist
    let mut missing_files = Vec::new();
    let mut file_paths = Vec::new();
    
    for ext in &config.extensions_to_concat {
        let ext_file = parent.join(format!("{}.{}", stem, ext));
        if !ext_file.exists() {
            missing_files.push(ext_file.to_string_lossy().to_string());
        } else {
            file_paths.push(ext_file);
        }
    }
    
    if !missing_files.is_empty() {
        warn!("Skipping {}: Missing files: {}", image_path.display(), missing_files.join(", "));
        return Ok(false);
    }
    
    // Read content from all files
    let mut contents = Vec::new();
    for path in &file_paths {
        let content = read_file_content(path).await?;
        contents.push(content);
    }
    
    // Concatenate contents
    let concatenated = concat_tags(&contents, config, &file_paths);
    
    // Create the output file path
    let output_path = parent.join(format!("{}.{}", stem, config.output_extension));
    
    if dry_run {
        info!("Would write to {}: {}", output_path.display(), concatenated);
    } else {
        fs::write(&output_path, &concatenated)
            .await
            .with_context(|| format!("Failed to write to: {}", output_path.display()))?;
        debug!("Wrote {}", output_path.display());
    }
    
    Ok(true)
}

/// Walks through a directory and concatenates files according to the configuration
pub async fn concat_files(
    directory: &Path, 
    config: &ConcatConfig,
    dry_run: bool
) -> Result<usize> {
    let directory = directory.to_path_buf();
    let config_clone = config.clone();
    
    info!("Searching for files in: {}", directory.display());
    info!("Using extensions: {}", config.extensions_to_concat.join(", "));
    info!("Output extension: {}", config.output_extension);
    if config.deduplicate_files {
        info!("File deduplication enabled - will check for identical file contents");
    }
    
    let processed_count = Arc::new(AtomicUsize::new(0));
    let skipped_duplicates = Arc::new(AtomicUsize::new(0));
    let mut base_extensions = HashSet::new();
    for ext in &config.base_extensions {
        base_extensions.insert(ext.clone());
        debug!("Added base extension: {}", ext);
    }
    
    // Track file content hashes for deduplication
    let content_hashes: Arc<tokio::sync::Mutex<HashMap<String, String>>> = 
        Arc::new(tokio::sync::Mutex::new(HashMap::new()));
    
    let processed_count_clone = processed_count.clone();
    let skipped_duplicates_clone = skipped_duplicates.clone();
    let content_hashes_clone = content_hashes.clone();
    
    xio::walk_directory(&directory, "*", move |path| {
        let path = path.to_path_buf();
        let base_exts = base_extensions.clone();
        let config = config_clone.clone();
        let dry_run = dry_run;
        let count = processed_count_clone.clone();
        let skipped = skipped_duplicates_clone.clone();
        let hashes = content_hashes_clone.clone();
        
        async move {
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                debug!("Checking file: {} with extension: {}", path.display(), ext_str);
                debug!("Base extensions: {:?}", base_exts);
                if base_exts.contains(&ext_str) {
                    debug!("Found base extension match: {}", path.display());
                    // Check for duplicate content if enabled
                    if config.deduplicate_files {
                        debug!("Checking for duplicate content: {}", path.display());
                        let is_duplicate = check_duplicate_content(&path, &config, hashes.clone()).await;
                        if is_duplicate {
                            debug!("Skipping duplicate file: {}", path.display());
                            skipped.fetch_add(1, Ordering::Relaxed);
                            return Ok(());
                        } else {
                            debug!("File is not a duplicate, proceeding: {}", path.display());
                        }
                    }
                    
                    // Process the image file
                    debug!("Processing file: {}", path.display());
                    match process_image_file(&path, &config, dry_run).await {
                        Ok(true) => {
                            debug!("Successfully processed: {}", path.display());
                            count.fetch_add(1, Ordering::Relaxed);
                        },
                        Ok(false) => {
                            debug!("Skipped due to missing files: {}", path.display());
                        },
                        Err(err) => warn!("Error processing {}: {}", path.display(), err),
                    }
                } else {
                    debug!("Skipping non-base extension: {}", path.display());
                }
            }
            Ok(())
        }
    }).await?;
    
    let final_count = processed_count.load(Ordering::Relaxed);
    let final_skipped = skipped_duplicates.load(Ordering::Relaxed);
    
    if dry_run {
        info!("Dry run completed. Would have processed {} files.", final_count);
    } else {
        info!("Concatenation completed. Processed {} files.", final_count);
    }
    
    if config.deduplicate_files {
        info!("Skipped {} duplicate files.", final_skipped);
    }
    
    Ok(final_count)
}

/// Checks if a file has duplicate content compared to already processed files
async fn check_duplicate_content(
    path: &Path,
    config: &ConcatConfig,
    hashes: Arc<tokio::sync::Mutex<HashMap<String, String>>>,
) -> bool {
    // Get the stem of the image file (filename without extension)
    let stem = match path.file_stem() {
        Some(s) => s.to_string_lossy(),
        None => {
            debug!("Could not get file stem for: {}", path.display());
            return false;
        },
    };
    
    let parent = match path.parent() {
        Some(p) => p,
        None => {
            debug!("Could not get parent directory for: {}", path.display());
            return false;
        },
    };
    
    debug!("Checking duplicate content for file: {} with stem: {}", path.display(), stem);
    
    // Check if all required files exist
    let mut file_paths = Vec::new();
    for ext in &config.extensions_to_concat {
        let ext_file = parent.join(format!("{}.{}", stem, ext));
        if !ext_file.exists() {
            debug!("Missing required file: {}", ext_file.display());
            return false; // Missing file, can't deduplicate
        }
        debug!("Found required file: {}", ext_file.display());
        file_paths.push(ext_file);
    }
    
    // Generate a content hash from all files
    let mut combined_content = String::new();
    for path in &file_paths {
        match fs::read_to_string(path).await {
            Ok(content) => {
                debug!("Read content from: {}", path.display());
                combined_content.push_str(&content);
            },
            Err(err) => {
                debug!("Failed to read content from {}: {}", path.display(), err);
                return false; // Can't read content, can't deduplicate
            },
        }
    }
    
    // Create a simple hash of the content
    let content_hash = format!("{:x}", md5::compute(combined_content.as_bytes()));
    debug!("Generated hash for {}: {}", path.display(), content_hash);
    
    // Check if this hash already exists
    let mut hashes_map = hashes.lock().await;
    if let Some(existing_file) = hashes_map.get(&content_hash) {
        debug!("Found duplicate content: {} matches {}", path.display(), existing_file);
        true
    } else {
        // No duplicate found, store this hash
        debug!("No duplicate found for {}, storing hash", path.display());
        hashes_map.insert(content_hash, path.to_string_lossy().to_string());
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs::File;
    use tokio::io::AsyncWriteExt;
    
    #[tokio::test]
    async fn test_concat_tags_with_duplicates() -> Result<()> {
        let config = ConcatConfig {
            base_extensions: vec!["jpg".into()],
            extensions_to_concat: vec!["wd".into(), "tags".into(), "caption".into()],
            output_extension: "txt".into(),
            remove_duplicates: true, 
            tag_separator: ", ".into(),
            deduplicate_files: false,
        };
        
        let contents = vec![
            "tag1, tag2, tag3".to_string(),    // wd
            "tag2, tag4, tag5".to_string(),    // tags
            "a photo of a person".to_string(), // caption
        ];
        
        // Create file paths to match the content
        let file_paths = vec![
            std::path::PathBuf::from("test.wd"),
            std::path::PathBuf::from("test.tags"),
            std::path::PathBuf::from("test.caption"),
        ];
        
        let result = concat_tags(&contents, &config, &file_paths);
        
        // Tags should be deduplicated and caption should be appended
        assert_eq!(result, "tag1, tag2, tag3, tag4, tag5, a photo of a person");
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_concat_tags_without_duplicates() -> Result<()> {
        let config = ConcatConfig {
            base_extensions: vec!["jpg".into()],
            extensions_to_concat: vec!["wd".into(), "tags".into(), "caption".into()],
            output_extension: "txt".into(),
            remove_duplicates: false,
            tag_separator: ", ".into(),
            deduplicate_files: false,
        };
        
        let contents = vec![
            "tag1, tag2, tag3".to_string(),    // wd
            "tag2, tag4, tag5".to_string(),    // tags
            "a photo of a person".to_string(), // caption
        ];
        
        // Create file paths to match the content
        let file_paths = vec![
            std::path::PathBuf::from("test.wd"),
            std::path::PathBuf::from("test.tags"),
            std::path::PathBuf::from("test.caption"),
        ];
        
        let result = concat_tags(&contents, &config, &file_paths);
        
        // Should preserve order and duplicates for tags, then append caption
        assert_eq!(result, "tag1, tag2, tag3, tag2, tag4, tag5, a photo of a person");
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_process_image_file() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let temp_path = temp_dir.path();
        
        // Create test files
        let image_path = temp_path.join("test.jpg");
        let caption_path = temp_path.join("test.caption");
        let wd_path = temp_path.join("test.wd");
        let tags_path = temp_path.join("test.tags");
        
        File::create(&image_path).await?.sync_all().await?;
        let mut caption_file = File::create(&caption_path).await?;
        caption_file.write_all(b"caption1, caption2").await?;
        caption_file.sync_all().await?;
        
        let mut wd_file = File::create(&wd_path).await?;
        wd_file.write_all(b"wd1, wd2").await?;
        wd_file.sync_all().await?;
        
        let mut tags_file = File::create(&tags_path).await?;
        tags_file.write_all(b"tag1, tag2").await?;
        tags_file.sync_all().await?;
        
        let config = ConcatConfig {
            base_extensions: vec!["jpg".into()],
            extensions_to_concat: vec!["caption".into(), "wd".into(), "tags".into()],
            output_extension: "txt".into(),
            remove_duplicates: true,
            tag_separator: ", ".into(),
            deduplicate_files: false,
        };
        
        // Process the image in dry-run mode
        let processed_dry = process_image_file(&image_path, &config, true).await?;
        assert!(processed_dry);
        assert!(!temp_path.join("test.txt").exists());
        
        // Process the image in real mode
        let processed = process_image_file(&image_path, &config, false).await?;
        assert!(processed);
        
        // Check that the output file was created with the expected content
        // Caption file content should come last, after the deduplicated tags
        let output_content = fs::read_to_string(temp_path.join("test.txt")).await?;
        assert_eq!(output_content, "tag1, tag2, wd1, wd2, caption1, caption2");
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_file_deduplication() -> Result<()> {
        // Initialize the logger for debugging
        let _ = env_logger::builder()
            .filter_level(log::LevelFilter::Debug)
            .is_test(true)
            .try_init();
        
        info!("Starting file deduplication test");
        
        // Create a temporary directory for the test
        let temp_dir = tempfile::tempdir()?;
        let temp_path = temp_dir.path();
        
        // Create test files with different names but identical content for some
        let image1_path = temp_path.join("image1.jpg");
        let image2_path = temp_path.join("image2.jpg");
        let image3_path = temp_path.join("image3.jpg");
        
        // Create caption files for the three images
        let caption1_path = temp_path.join("image1.caption");
        let caption2_path = temp_path.join("image2.caption");
        let caption3_path = temp_path.join("image3.caption");
        
        // Create WebUI description files
        let wd1_path = temp_path.join("image1.wd");
        let wd2_path = temp_path.join("image2.wd");
        let wd3_path = temp_path.join("image3.wd");
        
        // Create tags files
        let tags1_path = temp_path.join("image1.tags");
        let tags2_path = temp_path.join("image2.tags");
        let tags3_path = temp_path.join("image3.tags");
        
        // Write test content - image files just need to exist for the test
        info!("Creating test files in {}", temp_path.display());
        let mut image1_file = File::create(&image1_path).await?;
        image1_file.write_all(b"test image 1").await?;
        image1_file.sync_all().await?;
        
        let mut image2_file = File::create(&image2_path).await?;
        image2_file.write_all(b"test image 2").await?;
        image2_file.sync_all().await?;
        
        let mut image3_file = File::create(&image3_path).await?;
        image3_file.write_all(b"test image 3").await?;
        image3_file.sync_all().await?;
        
        // Create identical caption content for images 1 and 2, different for 3
        let mut caption1_file = File::create(&caption1_path).await?;
        caption1_file.write_all(b"a photo of a person").await?;
        caption1_file.sync_all().await?;
        
        let mut caption2_file = File::create(&caption2_path).await?;
        caption2_file.write_all(b"a photo of a person").await?;
        caption2_file.sync_all().await?;
        
        let mut caption3_file = File::create(&caption3_path).await?;
        caption3_file.write_all(b"person, portrait, indoor").await?;
        caption3_file.sync_all().await?;
        
        // Create identical WebUI description content for images 1 and 2, different for 3
        let mut wd1_file = File::create(&wd1_path).await?;
        wd1_file.write_all(b"masterpiece, digital art").await?;
        wd1_file.sync_all().await?;
        
        let mut wd2_file = File::create(&wd2_path).await?;
        wd2_file.write_all(b"masterpiece, digital art").await?;
        wd2_file.sync_all().await?;
        
        let mut wd3_file = File::create(&wd3_path).await?;
        wd3_file.write_all(b"highly detailed, 4k").await?;
        wd3_file.sync_all().await?;
        
        // Create the tags files with identical content for 1 and 2, different for 3
        let tags_content = "tag1, tag2, tag3";
        let mut tags1_file = File::create(&tags1_path).await?;
        tags1_file.write_all(tags_content.as_bytes()).await?;
        tags1_file.sync_all().await?;
        
        let mut tags2_file = File::create(&tags2_path).await?;
        tags2_file.write_all(tags_content.as_bytes()).await?;
        tags2_file.sync_all().await?;
        
        let mut tags3_file = File::create(&tags3_path).await?;
        tags3_file.write_all(b"tag4, tag5, tag6").await?;
        tags3_file.sync_all().await?;
        
        // Create a configuration with deduplication enabled
        let config = ConcatConfig {
            base_extensions: vec!["jpg".into()],
            extensions_to_concat: vec!["caption".into(), "wd".into(), "tags".into()],
            output_extension: "txt".into(),
            remove_duplicates: true,
            tag_separator: ", ".into(),
            deduplicate_files: true, // Enable deduplication
        };
        
        // Debug paths to make sure they're correct
        info!("Test files created at:");
        info!("Image 1: {}", image1_path.display());
        info!("Caption 1: {}", caption1_path.display());
        info!("WD 1: {}", wd1_path.display());
        info!("Tags 1: {}", tags1_path.display());
        
        // Instead of using concat_files which relies on directory walking,
        // directly use process_image_file and check_duplicate_content
        
        // Set up the deduplication hash table
        let content_hashes: Arc<tokio::sync::Mutex<HashMap<String, String>>> = 
            Arc::new(tokio::sync::Mutex::new(HashMap::new()));
        
        // Process the first image - should succeed
        info!("Processing first image: {}", image1_path.display());
        let is_duplicate1 = check_duplicate_content(&image1_path, &config, content_hashes.clone()).await;
        assert!(!is_duplicate1, "First image should not be detected as duplicate");
        
        let processed1 = process_image_file(&image1_path, &config, false).await?;
        assert!(processed1, "First image should be processed successfully");
        
        // Process the second image - should be detected as duplicate
        info!("Processing second image: {}", image2_path.display());
        let is_duplicate2 = check_duplicate_content(&image2_path, &config, content_hashes.clone()).await;
        assert!(is_duplicate2, "Second image should be detected as duplicate");
        
        // Process the third image - should not be a duplicate
        info!("Processing third image: {}", image3_path.display());
        let is_duplicate3 = check_duplicate_content(&image3_path, &config, content_hashes.clone()).await;
        assert!(!is_duplicate3, "Third image should not be detected as duplicate");
        
        let processed3 = process_image_file(&image3_path, &config, false).await?;
        assert!(processed3, "Third image should be processed successfully");
        
        // Check that output files were created correctly
        assert!(temp_path.join("image1.txt").exists(), "image1.txt should exist");
        assert!(!temp_path.join("image2.txt").exists(), "image2.txt should not exist (duplicate)");
        assert!(temp_path.join("image3.txt").exists(), "image3.txt should exist");
        
        // Read the content of the output files to verify
        let output1_content = fs::read_to_string(temp_path.join("image1.txt")).await?;
        let output3_content = fs::read_to_string(temp_path.join("image3.txt")).await?;
        
        // Print actual content for debugging
        info!("Output 1 content: '{}'", output1_content);
        info!("Output 3 content: '{}'", output3_content);
        
        // Check first file contains the deduplicated tags
        assert!(output1_content.contains("tag1, tag2, tag3"), 
                "Output for image1 should contain deduplicated tags");
        assert!(output1_content.contains("digital art, masterpiece"), 
                "Output for image1 should contain wd content (in alphabetical order)");
        assert!(output1_content.contains("a photo of a person"), 
                "Output for image1 should contain caption content");
        
        // Check third file contains its unique content
        assert!(output3_content.contains("tag4, tag5, tag6"), 
                "Output for image3 should contain its unique tags content");
        assert!(output3_content.contains("4k, highly detailed"), 
                "Output for image3 should contain its unique wd content (in alphabetical order)");
        assert!(output3_content.contains("person, portrait, indoor"), 
                "Output for image3 should contain its unique caption content");
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_concat_tags_caption_handling() -> Result<()> {
        let config = ConcatConfig {
            base_extensions: vec!["jpg".into()],
            extensions_to_concat: vec!["wd".into(), "tags".into(), "caption".into()],
            output_extension: "txt".into(),
            remove_duplicates: true, 
            tag_separator: ", ".into(),
            deduplicate_files: false,
        };
        
        // Test with tag that also appears in caption - should not deduplicate across
        let contents = vec![
            "person, photo".to_string(),            // wd
            "person, indoor, white background".to_string(), // tags
            "a photo of a person".to_string(),       // caption
        ];
        
        let file_paths = vec![
            std::path::PathBuf::from("test.wd"),
            std::path::PathBuf::from("test.tags"),
            std::path::PathBuf::from("test.caption"),
        ];
        
        let result = concat_tags(&contents, &config, &file_paths);
        
        // Tags should be deduplicated among themselves, not with caption content
        assert_eq!(result, "indoor, person, photo, white background, a photo of a person");
        
        // Test with caption as first extension
        let config = ConcatConfig {
            base_extensions: vec!["jpg".into()],
            extensions_to_concat: vec!["caption".into(), "wd".into(), "tags".into()],
            output_extension: "txt".into(),
            remove_duplicates: true, 
            tag_separator: ", ".into(),
            deduplicate_files: false,
        };
        
        let contents = vec![
            "a photo of a person".to_string(),       // caption
            "person, photo".to_string(),            // wd
            "person, indoor, white background".to_string(), // tags
        ];
        
        let file_paths = vec![
            std::path::PathBuf::from("test.caption"),
            std::path::PathBuf::from("test.wd"),
            std::path::PathBuf::from("test.tags"),
        ];
        
        let result = concat_tags(&contents, &config, &file_paths);
        
        // Caption should still be appended after deduplicated tags
        assert_eq!(result, "indoor, person, photo, white background, a photo of a person");
        
        Ok(())
    }
} 