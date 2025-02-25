# dset

A Rust library for processing and managing dataset-related files, with a focus on machine learning datasets, captions, and safetensors files. Built on top of xio for efficient file operations.

## Features

### 🔧 SafeTensors Processing

- Extract and decode embedded metadata from SafeTensors files
- Automatic JSON decoding of nested metadata fields
- Support for special metadata fields
- Memory-mapped file handling for efficient processing
- Pretty-printed JSON output

### 📝 Caption File Handling

- Multi-format support:
  - Plain text captions
  - JSON captions
  - e621 JSON format support
  - Automatic format detection
- Caption file validation:
  - Check for existence and content
  - Handle empty and whitespace-only files
- Tag extraction and probability filtering
- Special character escaping (e.g., parentheses)
- Conversion between formats
- Batch processing capabilities
- e621 tag processing with artist formatting and filtering
- Text processing utilities:
  - String replacement with formatting options
  - Special character normalization (smart quotes → standard quotes)
  - Whitespace and newline normalization

### 🗃️ File Operations

- File management:
  - Rename files (remove image extensions)
  - Check file existence
  - Content validation
- Batch processing capabilities
- Efficient async I/O operations
- Format conversions

### 🔢 JSON Processing

- Format validation and pretty printing
- Deep JSON string decoding
- Nested JSON structure handling
- Automatic type conversion
- Support for `None` values
- Probability-based tag filtering
- e621 JSON post data extraction

### 🎯 Content Processing

- Smart content splitting into tags and sentences
- Tag probability threshold filtering (default: 0.2)
- Special character escaping in tags
- Sorting tags by probability
- Batch file processing

### ⚡ Performance Features

- Asynchronous operations using Tokio
- Memory-mapped file handling
- Parallel processing capabilities
- Efficient string and JSON parsing
- Optimized file I/O

### 🛡️ Error Handling

- Comprehensive error context with anyhow
- Detailed error messages
- Safe error recovery
- Proper resource cleanup

## Installation

```bash
cargo add dset
```

## Usage Examples

### SafeTensors Metadata Extraction

```rust
use dset::{Path, process_safetensors_file};
use anyhow::Result;

async fn extract_metadata(path: &str) -> Result<()> {
    // Extracts metadata and saves it as a JSON file
    process_safetensors_file(Path::new(path)).await?;
    
    // The output will be saved as "{path}.json"
    Ok(())
}
```

### Caption File Processing

```rust
use dset::{
    Path,
    process_caption_file,
    process_json_to_caption,
    caption::caption_file_exists_and_not_empty
};
use anyhow::Result;

async fn handle_captions() -> Result<()> {
    let path = Path::new("image1.txt");
    
    // Check if caption file exists and has content
    if caption_file_exists_and_not_empty(&path).await {
        // Process the caption file (auto-detects format)
        process_caption_file(&path).await?;
    }
    
    // Convert JSON caption to text format
    process_json_to_caption(Path::new("image2.json")).await?;
    
    Ok(())
}
```

### File Operations

```rust
use dset::{Path, rename_file_without_image_extension};
use std::io;

async fn handle_files() -> io::Result<()> {
    // Remove intermediate image extensions from files
    let path = Path::new("image.jpg.toml");
    rename_file_without_image_extension(&path).await?;  // Will rename to "image.toml"
    
    // Won't modify files that are actually images
    let img = Path::new("photo.jpg");
    rename_file_without_image_extension(&img).await?;  // Will remain "photo.jpg"
    
    Ok(())
}
```

### JSON Processing and Formatting

```rust
use dset::{Path, format_json_file, process_json_file};
use serde_json::Value;
use anyhow::Result;

async fn handle_json() -> Result<()> {
    // Format a JSON file
    format_json_file(Path::new("data.json").to_path_buf()).await?;
    
    // Process JSON with custom handler
    process_json_file(Path::new("data.json"), |json: &Value| async {
        println!("Processing: {}", json);
        Ok(())
    }).await?;
    
    Ok(())
}
```

### Content Splitting

```rust
use dset::split_content;

fn process_tags_and_text() {
    let content = "tag1, tag2, tag3., This is the main text.";
    let (tags, sentences) = split_content(content);
    
    println!("Tags: {:?}", tags);  // ["tag1", "tag2", "tag3"]
    println!("Text: {}", sentences);  // "This is the main text."
}
```

### E621 JSON Processing

```rust
use dset::{Path, process_e621_json_file};
use anyhow::Result;

async fn process_e621() -> Result<()> {
    // Process an e621 JSON file and create a caption file
    process_e621_json_file(Path::new("e621_post.json")).await?;
    
    // This will create a caption file with the same name but .txt extension
    // with properly formatted tags from the e621 post data
    Ok(())
}
```

### Text Processing

```rust
use dset::caption::{format_text_content, replace_string, replace_special_chars};
use std::path::{Path, PathBuf};
use anyhow::Result;

async fn example() -> Result<()> {
    // Format text by normalizing whitespace
    let formatted = format_text_content("  Multiple    spaces   \n\n  and newlines  ")?;
    assert_eq!(formatted, "Multiple spaces and newlines");
    
    // Replace text in a file
    replace_string(Path::new("caption.txt"), "old text", "new text").await?;
    
    // Replace special characters in a file (smart quotes, etc.)
    replace_special_chars(PathBuf::from("document.txt")).await?;
    
    Ok(())
}
```

## Error Handling

The library uses `anyhow` for comprehensive error handling:

```rust
use dset::Path;
use anyhow::{Context, Result};

async fn example() -> Result<()> {
    process_safetensors_file(Path::new("model.safetensors"))
        .await
        .context("Failed to process safetensors file")?;
    Ok(())
}
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. When contributing:

1. Ensure all tests pass
2. Add tests for new features
3. Update documentation
4. Follow the existing code style
5. Add error handling where appropriate

## License

This project is licensed under the MIT License.
