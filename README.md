# dset

A Rust library for processing and managing dataset-related files, with a focus on machine learning datasets, captions, and safetensors files. Built on top of xio for efficient file operations.

## Features

- 🔧 SafeTensors file processing and metadata extraction
- 📝 Caption file handling and conversion
- 🔄 JSON processing utilities
- 🎯 Smart content splitting and tag processing
- ⚡ Asynchronous operations using Tokio
- 🛡️ Robust error handling with anyhow
- 🖼️ Image-related utilities (via imx integration)

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
dset = "0.1.0"
```

## Usage Examples

### Processing SafeTensors Files

Extract metadata from SafeTensors files:

```rust
use dset::{Path, process_safetensors_file};
use anyhow::Result;

async fn extract_model_metadata(path: &str) -> Result<()> {
    process_safetensors_file(Path::new(path)).await
}
```

### Working with Caption Files

Process and convert caption files:

```rust
use dset::{Path, process_caption_file};
use anyhow::Result;

async fn handle_caption(path: &str) -> Result<()> {
    process_caption_file(Path::new(path)).await
}
```

### JSON Processing

Convert JSON files to caption format:

```rust
use dset::{Path, process_json_to_caption};
use std::io;

async fn convert_json_caption(path: &str) -> io::Result<()> {
    process_json_to_caption(Path::new(path)).await
}
```

### Content Splitting

Split content into tags and sentences:

```rust
use dset::split_content;

fn process_content(content: &str) {
    let (tags, sentences) = split_content(content);
    println!("Tags: {:?}", tags);
    println!("Sentences: {}", sentences);
}
```

## Advanced Features

### SafeTensors Metadata Processing

- Extracts embedded metadata from SafeTensors files
- Automatically decodes JSON-encoded strings in metadata
- Handles special fields like:
  - `ss_bucket_info`
  - `ss_tag_frequency`
  - `ss_dataset_dirs`
  - `ss_network_args`
  - `resize_params`

### Caption Processing

- Supports both JSON and plain text caption formats
- Automatic format detection and parsing
- Tag probability filtering
- Special character escaping

### Image Utilities (via imx)

- Image file detection
- Caption file validation
- Letterbox removal
- File extension handling

## Error Handling

All operations return `Result` types with detailed error information:
- `anyhow::Result` for rich error context
- `io::Result` for basic file operations
- Comprehensive error messages with context

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License. 