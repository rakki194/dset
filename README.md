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
- e621 tag processing with:
  - Artist name formatting with prefix/suffix options
  - Tag filtering for years, aspect ratios, etc.
  - Optional underscore replacement (spaces vs underscores)
  - Customizable rating conversions
  - Custom caption format templates
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

## E621 Caption Processing

The library provides comprehensive support for processing e621 JSON post data into standardized caption files. This functionality is particularly useful for creating training datasets from e621 posts.

### Configuration

The processing can be customized using `E621Config`:

```rust
use dset::{E621Config, Path, process_e621_json_file};
use std::collections::HashMap;
use anyhow::Result;

async fn process_with_config() -> Result<()> {
    // Create a custom configuration
    let mut custom_ratings = HashMap::new();
    custom_ratings.insert("s".to_string(), "sfw".to_string());
    custom_ratings.insert("q".to_string(), "maybe".to_string());
    custom_ratings.insert("e".to_string(), "nsfw".to_string());

    let config = E621Config::new()
        .with_filter_tags(false)  // Disable tag filtering
        .with_rating_conversions(Some(custom_ratings))  // Custom rating names
        .with_format(Some("Rating: {rating}\nArtists: {artists}\nTags: {general}".to_string()));  // Custom format

    process_e621_json_file(Path::new("e621_post.json"), Some(config)).await
}
```

#### Available Options

- **Tag Filtering** (`filter_tags: bool`, default: `true`)
  - When enabled, filters out noise tags
  - Can be disabled to include all tags

- **Rating Conversions** (`rating_conversions: Option<HashMap<String, String>>`)
  - Default conversions:
    - "s" → "safe"
    - "q" → "questionable"
    - "e" → "explicit"
  - Can be customized or disabled (set to `None` to use raw ratings)

- **Artist Formatting** (new in 0.1.8)
  - `artist_prefix: Option<String>` (default: `Some("by ")`)
  - `artist_suffix: Option<String>` (default: `None`)
  - Customize how artist names are formatted
  - Set both to `None` for raw artist names
  - Examples:
    - Default: "by artist_name" → "by artist name"
    - Custom prefix: "drawn by artist_name" → "drawn by artist name"
    - Custom suffix: "artist_name (Artist)" → "artist name (Artist)"
    - Both: "art by artist_name (verified)" → "art by artist name (verified)"
    - None: "artist_name" → "artist name"

- **Format String** (`format: Option<String>`)
  - Default: `"{rating}, {artists}, {characters}, {species}, {copyright}, {general}, {meta}"`
  - Available placeholders:
    - `{rating}` - The rating (after conversion)
    - `{artists}` - Artist tags (with configured formatting)
    - `{characters}` - Character tags
    - `{species}` - Species tags
    - `{copyright}` - Copyright tags
    - `{general}` - General tags
    - `{meta}` - Meta tags
  - Each tag group is internally joined with ", "

### Tag Processing

- **Artist Tags**
  - Configurable prefix (default: "by ")
  - Optional suffix
  - Underscores replaced with spaces
  - "(artist)" suffix removed from source
  - Examples:
    - Default: "artist_name (artist)" → "by artist name"
    - Custom: "artist_name" → "drawn by artist name (verified)"
    - Raw: "artist_name" → "artist name"

- **Character Tags**
  - Underscores replaced with spaces
  - Original character names preserved
  - Example: "character_name" → "character name"

- **Species Tags**
  - Included as-is with spaces
  - Useful for dataset filtering

- **Copyright Tags**
  - Source material references preserved
  - Underscores replaced with spaces

- **General Tags**
  - Common descriptive tags
  - Underscores replaced with spaces
  - Filtered to remove noise

- **Meta Tags**
  - Selected important meta information
  - Art medium and style information preserved

### Tag Filtering

Tag filtering is enabled by default but can be disabled. When enabled, it automatically filters out:

- Year tags (e.g., "2023")
- Aspect ratio tags (e.g., "16:9")
- Conditional DNP tags
- Empty or whitespace-only tags

To disable filtering, pass `Some(false)` as the `filter_tags` parameter.

### Caption File Generation

- Creates `.txt` files from e621 JSON posts
- Filename derived from post's image MD5
- Format: `[rating], [artist tags], [character tags], [other tags]`
- Skips generation if no valid tags remain after filtering (when filtering is enabled)

### Example Usage

```rust
use dset::{E621Config, Path, process_e621_json_file};
use anyhow::Result;

async fn process_e621() -> Result<()> {
    // Process with default settings
    process_e621_json_file(Path::new("e621_post.json"), None).await?;
    
    // Process with custom format
    let config = E621Config::new()
        .with_format(Some("{rating}\nBy: {artists}\nTags: {general}".to_string()));
    process_e621_json_file(Path::new("e621_post.json"), Some(config)).await?;
    
    // Process with raw ratings (no conversion)
    let config = E621Config::new()
        .with_rating_conversions(None);
    process_e621_json_file(Path::new("e621_post.json"), Some(config)).await?;
    
    Ok(())
}
```

### Example Outputs

With default settings:

```plaintext
safe, by artist name, character name, species, tag1, tag2
```

With custom format:

```plaintext
Rating: safe
Artists: by artist name
Tags: tag1, tag2
```

With raw ratings:

```plaintext
s, by artist name, character name, species, tag1, tag2
```

### Batch Processing Example

```rust
use dset::{E621Config, Path, process_e621_json_file};
use anyhow::Result;
use tokio::fs;

async fn batch_process_e621() -> Result<()> {
    // Optional: customize processing for all files
    let config = E621Config::new()
        .with_filter_tags(false)
        .with_format(Some("{rating}\n{artists}\n{general}".to_string()));
    
    let entries = fs::read_dir("e621_posts").await?;
    
    for entry in entries {
        if let Ok(entry) = entry {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "json") {
                process_e621_json_file(&path, Some(config.clone())).await?;
            }
        }
    }
    Ok(())
}
```

## Installation

```bash
cargo add dset
```

## Logging Configuration

The library uses the `log` crate for logging. To enable logging in your application:

1. Add a logging implementation like `env_logger` to your project:

    ```bash
    cargo add env_logger
    ```

2. Initialize the logger in your application:

    ```rust
    use env_logger;

    fn main() {
        env_logger::init();
        // Your code here...
    }
    ```

3. Set the log level using the `RUST_LOG` environment variable:

    ```bash
    export RUST_LOG=info    # Show info and error messages
    export RUST_LOG=debug   # Show debug, info, and error messages
    export RUST_LOG=trace   # Show all log messages
    ```

The library uses different log levels:

- `error`: For unrecoverable errors
- `warn`: For recoverable errors or unexpected conditions
- `info`: For important operations and successful processing
- `debug`: For detailed processing information
- `trace`: For very detailed debugging information

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
use log::info;

fn process_tags_and_text() {
    let content = "tag1, tag2, tag3., This is the main text.";
    let (tags, sentences) = split_content(content);
    
    info!("Tags: {:?}", tags);  // ["tag1", "tag2", "tag3"]
    info!("Text: {}", sentences);  // "This is the main text."
}
```

### Text Processing

```rust
use dset::caption::{format_text_content, replace_string, replace_special_chars};
use std::path::{Path, PathBuf};
use anyhow::Result;
use log::info;

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

### Error Handling

The library uses `anyhow` for comprehensive error handling:

```rust
use dset::Path;
use anyhow::{Context, Result};
use log::info;

async fn example() -> Result<()> {
    process_safetensors_file(Path::new("model.safetensors"))
        .await
        .context("Failed to process safetensors file")?;
    info!("Successfully processed safetensors file");
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
