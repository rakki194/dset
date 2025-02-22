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
  - Automatic format detection
- Tag extraction and probability filtering
- Special character escaping (e.g., parentheses)
- Conversion between formats
- Batch processing capabilities

### 🔄 JSON Processing

- Format validation and pretty printing
- Deep JSON string decoding
- Nested JSON structure handling
- Automatic type conversion
- Support for `None` values
- Probability-based tag filtering

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

Add this to your `Cargo.toml`:

```toml
[dependencies]
dset = "0.1.5"
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
use dset::{Path, process_caption_file, process_json_to_caption};
use anyhow::Result;

async fn handle_captions() -> Result<()> {
    // Process a caption file (auto-detects format)
    process_caption_file(Path::new("image1.txt")).await?;
    
    // Convert JSON caption to text format
    process_json_to_caption(Path::new("image2.json")).await?;
    
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

## Module Structure

### `st` Module

Handles SafeTensors file processing:

- Memory-mapped file reading
- Metadata extraction
- JSON conversion
- Async file operations

### `caption` Module

Manages caption file operations:

- Format detection
- JSON/text conversion
- Tag processing
- Batch operations

### `metadata` Module

Provides metadata processing utilities:

- JSON string decoding
- Nested structure handling
- Training metadata extraction
- Special field processing

## Advanced Features

### SafeTensors Metadata Processing

- Automatic detection and parsing of nested JSON strings
- Support for complex metadata structures
- Efficient memory mapping for large files
- Pretty-printed JSON output

### Caption Processing

- Probability-based tag filtering (>0.2 by default)
- Special character escaping in tags
- Automatic format detection
- Efficient batch processing

### JSON Processing

- Deep JSON string decoding
- Support for complex nested structures
- Efficient memory usage
- Pretty printing capabilities

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
