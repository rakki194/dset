# dset

A Rust library for processing and managing dataset-related files, particularly for machine learning datasets, captions, and safetensors files.

## Overview

- **SafeTensors Processing**
  - Extract metadata from safetensors files
  - Decode nested JSON structures
  - Memory-mapped file handling for efficiency
  - Pretty-printed JSON output

- **Caption File Handling**
  - Support for multiple formats (JSON, text)
  - Validate caption content
  - Extract and process tags
  - Batch processing capabilities

- **File Operations**
  - Rename files with standardized patterns
  - Check file existence and content
  - Validate file types and contents

- **JSON Processing**
  - Format validation and formatting
  - Deep JSON decoding
  - Handle nested structures

- **Content Processing**
  - Smart content splitting (tags vs. text)
  - Tag filtering and formatting
  - Batch processing with async support

- **Performance Features**
  - Async/await support for I/O operations
  - Memory-mapped file handling
  - Optimized parsing techniques

- **Error Handling**
  - Comprehensive error context
  - Recovery mechanisms
  - Detailed logging

## E621 Caption Processing

The library excels at processing e621 JSON post data into standardized caption files, ideal for creating training datasets. The configuration is highly customizable using `E621Config`:

```rust
use dset::caption::{E621Config, process_e621_json_file};
use std::path::Path;
use std::collections::HashMap;
use anyhow::Result;

async fn process_with_custom_config() -> Result<()> {
    // Create custom rating conversions
    let mut custom_ratings = HashMap::new();
    custom_ratings.insert("s".to_string(), "safe".to_string());
    custom_ratings.insert("q".to_string(), "maybe".to_string());
    custom_ratings.insert("e".to_string(), "nsfw".to_string());

    let config = E621Config::new()
        .with_filter_tags(false)  // Disable tag filtering
        .with_rating_conversions(Some(custom_ratings))  // Custom rating names
        .with_format(Some("Rating: {rating}\nArtists: {artists}\nTags: {general}".to_string()));  // Custom format

    process_e621_json_file(Path::new("e621_post.json"), Some(config)).await
}
```

### Available Options

- **Tag Filtering** (`filter_tags: bool`, default: `true`)
  - When enabled, filters out noise tags
  - Can be disabled to include all tags

- **Rating Conversions** (`rating_conversions: Option<HashMap<String, String>>`)
  - Default conversions:
    - "s" → "safe"
    - "q" → "questionable"
    - "e" → "explicit"
  - Can be customized or disabled (set to `None` to use raw ratings)

- **Artist Formatting**
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

The tag filtering system uses regular expressions for pattern matching and will:

- Skip invalid patterns gracefully (returning false)
- Handle regex compilation errors by panicking (this should never happen with the built-in patterns)
- Provide clear error context for debugging

To disable filtering, pass `Some(false)` as the `filter_tags` parameter.

### Caption File Generation

- Creates `.txt` files from e621 JSON posts
- Filename derived from post's image MD5
- Format: `[rating], [artist tags], [character tags], [other tags]`
- Skips generation if no valid tags remain after filtering (when filtering is enabled)

### Example Usage

```rust
use dset::caption::{E621Config, process_e621_json_file};
use std::path::Path;
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
use dset::caption::{E621Config, process_e621_json_file};
use std::path::Path;
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

## 🤖 AI Reasoning Dataset

The library provides comprehensive support for managing AI reasoning datasets, particularly useful for training language models in structured reasoning tasks. This functionality helps maintain consistent formatting and organization of reasoning data.

### Data Structure

The reasoning dataset format consists of three main components:

1. **Messages** - Individual conversation messages:

    ```rust
    Message {
        content: String,  // The message content
        role: String,     // The role (e.g., "user", "reasoning", "assistant")
    }
    ```

2. **Reasoning Entries** - Complete reasoning interactions:

    ```rust
    ReasoningEntry {
        user: String,         // The user's question/request
        reasoning: String,    // Detailed step-by-step reasoning
        assistant: String,    // Final summarized response
        template: String,     // Structured template combining all roles
        conversations: Vec<Message>,  // Complete conversation history
    }
    ```

3. **Dataset Collection** - Collection of reasoning entries:

    ```rust
    ReasoningDataset {
        entries: Vec<ReasoningEntry>
    }
    ```

### Features

- **Structured Data Management**
  - Organize reasoning data in a consistent format
  - Maintain conversation history with role attribution
  - Track detailed reasoning steps separately from final responses

- **Template Generation**
  - Automatic creation of structured templates
  - Consistent formatting with `<|im_start|>` and `<|im_end|>` tokens
  - Clear separation of user input, reasoning, and responses

- **File Operations**
  - Asynchronous JSON file loading and saving
  - Pretty-printed output for readability
  - Error handling with detailed context

- **Dataset Manipulation**
  - Add new entries to existing datasets
  - Query dataset size and emptiness
  - Efficient memory management

### Reasoning Example Usage

1. **Creating and Managing Datasets**

    ```rust
    use dset::reasoning::{ReasoningDataset, ReasoningEntry, Message};
    use anyhow::Result;

    async fn manage_dataset() -> Result<()> {
        // Create a new dataset
        let mut dataset = ReasoningDataset::new();

        // Create an entry
        let entry = ReasoningEntry {
            user: "What motivates Luna?".to_string(),
            reasoning: "Luna's motivations can be analyzed based on several factors:\n1. Desire for acceptance\n2. Self-expression needs\n3. Personal growth aspirations".to_string(),
            assistant: "Luna is motivated by acceptance, self-expression, and personal growth.".to_string(),
            template: ReasoningDataset::create_template(
                "What motivates Luna?",
                "Luna's motivations can be analyzed...",
                "Luna is motivated by acceptance, self-expression, and personal growth."
            ),
            conversations: vec![
                Message {
                    content: "What motivates Luna?".to_string(),
                    role: "user".to_string(),
                },
                Message {
                    content: "Luna's motivations can be analyzed...".to_string(),
                    role: "reasoning".to_string(),
                },
                Message {
                    content: "Luna is motivated by acceptance, self-expression, and personal growth.".to_string(),
                    role: "assistant".to_string(),
                },
            ],
        };

        // Add entry to dataset
        dataset.add_entry(entry);

        // Save dataset to file
        dataset.save("reasoning_data.json").await?;

        // Load dataset from file
        let loaded_dataset = ReasoningDataset::load("reasoning_data.json").await?;
        assert_eq!(loaded_dataset.len(), 1);

        Ok(())
    }
    ```

2. **Working with Templates**

    ```rust
    use dset::reasoning::ReasoningDataset;

    // Create a template string
    let template = ReasoningDataset::create_template(
        "What is the best approach?",
        "Let's analyze this step by step...",
        "Based on the analysis, the best approach is..."
    );

    // Template output format:
    // <|im_start|>user
    // What is the best approach?
    // <|im_end|>
    // <|im_start|>reasoning
    // Let's analyze this step by step...
    // <|im_end|>
    // <|im_start|>assistant
    // Based on the analysis, the best approach is...
    // <|im_end|>
    ```

#### JSON Output Format

The dataset is saved in a structured JSON format:

```json
{
  "entries": [
    {
      "user": "What motivates Luna?",
      "reasoning": "Luna's motivations can be analyzed...",
      "assistant": "Luna is motivated by acceptance, self-expression, and personal growth.",
      "template": "<|im_start|>user\n...<|im_end|>...",
      "conversations": [
        {
          "content": "What motivates Luna?",
          "role": "user"
        },
        {
          "content": "Luna's motivations can be analyzed...",
          "role": "reasoning"
        },
        {
          "content": "Luna is motivated by acceptance, self-expression, and personal growth.",
          "role": "assistant"
        }
      ]
    }
  ]
}
```

#### Best Practices

1. **Structured Reasoning**
   - Keep reasoning steps clear and organized
   - Use numbered lists or bullet points for complex analyses
   - Maintain consistent formatting across entries

2. **Role Attribution**
   - Use clear and consistent role names
   - Standard roles: "user", "reasoning", "assistant"
   - Consider adding custom roles for specific use cases

3. **Template Management**
   - Use the provided template generation function
   - Maintain consistent token usage (`<|im_start|>` and `<|im_end|>`)
   - Include all relevant conversation components

4. **Error Handling**
   - Use the Result type for error propagation
   - Handle file operations with proper error context
   - Validate data before saving

5. **Async Operations**
   - Use async/await for file operations
   - Consider batch processing for large datasets
   - Implement proper error handling for async operations

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

## Core Functions Reference

### SafeTensors Functions

#### `process_safetensors_file(path: &Path) -> Result<()>`

Processes a safetensors file by extracting its metadata and saving it as a JSON file.

- **Parameters:**
  - `path`: Path to the safetensors file
- **Returns:** Result indicating success or failure
- **Error Handling:** Provides detailed context for failures including file opening issues, memory mapping errors, and metadata extraction failures
- **Performance:** Uses memory mapping for efficient file access without loading the entire file into memory
- **Example:**

  ```rust
  process_safetensors_file(Path::new("model.safetensors")).await?;
  // Creates model.safetensors.metadata.json
  ```

#### `get_json_metadata(path: &Path) -> Result<Value>`

Extracts and parses JSON metadata from a safetensors file.

- **Parameters:**
  - `path`: Path to the safetensors file
- **Returns:** The extracted metadata as a serde_json Value
- **Error Handling:** Provides context for file opening, memory mapping, and JSON parsing errors
- **Performance:** Uses memory mapping for efficient handling of large files
- **Example:**

  ```rust
  let metadata = get_json_metadata(Path::new("model.safetensors")).await?;
  println!("Model metadata: {}", metadata);
  ```

#### `decode_json_strings(value: Value) -> Value`

Recursively decodes JSON-encoded strings within a serde_json::Value.

- **Parameters:**
  - `value`: JSON value potentially containing encoded strings
- **Returns:** Decoded JSON value with nested structures properly parsed
- **Behavior:**
  - Converts string "None" to JSON null
  - Tries to parse strings starting with '{' or '[' as JSON objects or arrays
  - Recursively processes all nested values
- **Example:**

  ```rust
  let raw_json = json!({"config": "{\"param\": 123}"});
  let decoded = decode_json_strings(raw_json);
  // Results in: {"config": {"param": 123}}
  ```

#### `extract_training_metadata(raw_metadata: &Value) -> Value`

Extracts and processes training metadata from raw safetensors metadata.

- **Parameters:**
  - `raw_metadata`: Raw metadata from a safetensors file
- **Returns:** Processed metadata with decoded JSON strings
- **Behavior:**
  - Looks for a `__metadata__` field first
  - Falls back to decoding the entire metadata object if not found
  - Returns an empty object if no valid metadata is found
- **Example:**

  ```rust
  let raw_meta = get_json_metadata(Path::new("model.safetensors")).await?;
  let training_meta = extract_training_metadata(&raw_meta);
  ```

### Caption Processing Functions

#### `process_file(path: &Path) -> Result<()>`

Processes a caption file in either JSON or plain text format.

- **Parameters:**
  - `path`: Path to the caption file
- **Returns:** Result indicating success or failure
- **Behavior:**
  - Auto-detects file format based on extension and content
  - For JSON files, extracts caption text and formats it
  - For text files, formats the text content
- **Error Handling:** Provides context for file I/O errors and JSON parsing failures
- **Example:**

  ```rust
  process_file(Path::new("caption.json")).await?;
  ```

#### `json_to_text(json: &Value) -> Result<String>`

Extracts caption text from a JSON value.

- **Parameters:**
  - `json`: JSON value containing caption data
- **Returns:** Extracted caption text
- **Behavior:**
  - If the JSON is a string, returns the string
  - If the JSON is an object with a "caption" field, returns that field
  - Fails for other JSON formats
- **Error Handling:** Returns error for unsupported JSON structures
- **Example:**

  ```rust
  let json = serde_json::from_str("{\"caption\": \"A beautiful landscape\"}")?;
  let text = json_to_text(&json)?;
  // text = "A beautiful landscape"
  ```

#### `caption_file_exists_and_not_empty(path: &Path) -> bool`

Checks if a caption file exists and has content.

- **Parameters:**
  - `path`: Path to the caption file
- **Returns:** Boolean indicating if the file exists and is not empty
- **Performance:** Uses efficient file operations to avoid unnecessary reads
- **Example:**

  ```rust
  if caption_file_exists_and_not_empty(Path::new("caption.txt")).await {
      println!("Caption file is valid");
  }
  ```

#### `process_e621_json_file(file_path: &Path, config: Option<E621Config>) -> Result<()>`

Processes an e621 JSON file and creates a caption file.

- **Parameters:**
  - `file_path`: Path to the e621 JSON file
  - `config`: Optional configuration for customizing processing
- **Returns:** Result indicating success or failure
- **Behavior:**
  - Reads and parses the e621 JSON file
  - Extracts tags according to configuration
  - Creates a caption file with the processed tags
- **Error Handling:** Provides context for file I/O errors and JSON parsing failures
- **Example:**

  ```rust
  let config = E621Config::new().with_filter_tags(true);
  process_e621_json_file(Path::new("post.json"), Some(config)).await?;
  ```

#### `process_e621_json_data(data: &Value, file_path: &Arc<PathBuf>, config: Option<E621Config>) -> Result<()>`

Processes e621 JSON data and creates a caption file.

- **Parameters:**
  - `data`: e621 JSON data as a serde_json Value
  - `file_path`: Path to the JSON file (used for output path calculation)
  - `config`: Optional configuration for customizing processing
- **Returns:** Result indicating success or failure
- **Behavior:**
  - Extracts rating and tags from the e621 JSON data
  - Formats tags according to configuration
  - Creates a caption file with the processed tags
- **Example:**

  ```rust
  let json_data = serde_json::from_str(json_str)?;
  let path = Arc::new(PathBuf::from("post.json"));
  process_e621_json_data(&json_data, &path, None).await?;
  ```

#### `format_text_content(content: &str) -> Result<String>`

Formats text content by normalizing whitespace.

- **Parameters:**
  - `content`: Text content to format
- **Returns:** Formatted text
- **Behavior:**
  - Trims leading and trailing whitespace
  - Replaces multiple spaces with a single space
  - Replaces newlines with spaces
- **Example:**

  ```rust
  let formatted = format_text_content("  Multiple    spaces   \n\n  and newlines  ")?;
  // formatted = "Multiple spaces and newlines"
  ```

#### `replace_string(path: &Path, search: &str, replace: &str) -> Result<()>`

Replaces occurrences of a string in a file.

- **Parameters:**
  - `path`: Path to the file
  - `search`: String to search for
  - `replace`: String to replace with
- **Returns:** Result indicating success or failure
- **Performance:** Reads the entire file into memory, so be cautious with very large files
- **Error Handling:** Provides context for file I/O errors
- **Example:**

  ```rust
  replace_string(Path::new("caption.txt"), "old text", "new text").await?;
  ```

#### `replace_special_chars(path: PathBuf) -> Result<()>`

Replaces special characters in a file with standard ASCII equivalents.

- **Parameters:**
  - `path`: Path to the file
- **Returns:** Result indicating success or failure
- **Behavior:**
  - Replaces smart quotes with standard quotes
  - Replaces other special characters with standard equivalents
- **Error Handling:** Provides context for file I/O errors
- **Example:**

  ```rust
  replace_special_chars(PathBuf::from("document.txt")).await?;
  ```

### Tag Processing Functions

#### `should_ignore_e621_tag(tag: &str) -> bool`

Determines if an e621 tag should be ignored.

- **Parameters:**
  - `tag`: Tag to check
- **Returns:** Boolean indicating if the tag should be ignored
- **Behavior:**
  - Checks against predefined patterns (years, aspect ratios, etc.)
  - Returns true for tags that should be filtered out
- **Performance:** Uses precompiled regex patterns for efficiency
- **Example:**

  ```rust
  if !should_ignore_e621_tag("2023") {
      tags.push("2023");
  }
  ```

#### `process_e621_tags(tags_dict: &Value, config: Option<&E621Config>) -> Vec<String>`

Processes e621 tags from a JSON dictionary.

- **Parameters:**
  - `tags_dict`: JSON dictionary containing e621 tags
  - `config`: Optional configuration for customizing processing
- **Returns:** Vector of processed tags
- **Behavior:**
  - Extracts tags from different categories (artist, character, etc.)
  - Formats tags according to configuration
  - Filters tags if enabled in configuration
- **Example:**

  ```rust
  let tags = process_e621_tags(&tags_json, Some(&config));
  ```

### Reasoning Dataset Functions

#### `ReasoningDataset::new() -> Self`

Creates a new empty reasoning dataset.

- **Returns:** Empty ReasoningDataset
- **Example:**

  ```rust
  let dataset = ReasoningDataset::new();
  ```

#### `ReasoningDataset::load<P: AsRef<Path>>(path: P) -> Result<Self>`

Loads a reasoning dataset from a JSON file.

- **Parameters:**
  - `path`: Path to the JSON file
- **Returns:** Loaded ReasoningDataset
- **Error Handling:** Provides context for file I/O errors and JSON parsing failures
- **Example:**

  ```rust
  let dataset = ReasoningDataset::load("dataset.json").await?;
  ```

#### `ReasoningDataset::save<P: AsRef<Path>>(&self, path: P) -> Result<()>`

Saves the reasoning dataset to a JSON file.

- **Parameters:**
  - `path`: Path to save the JSON file
- **Returns:** Result indicating success or failure
- **Behavior:** Creates a pretty-printed JSON file
- **Error Handling:** Provides context for file I/O errors and JSON serialization failures
- **Example:**

  ```rust
  dataset.save("dataset.json").await?;
  ```

#### `ReasoningDataset::add_entry(&mut self, entry: ReasoningEntry)`

Adds a new entry to the dataset.

- **Parameters:**
  - `entry`: ReasoningEntry to add
- **Example:**

  ```rust
  dataset.add_entry(entry);
  ```

#### `ReasoningDataset::len(&self) -> usize`

Returns the number of entries in the dataset.

- **Returns:** Number of entries
- **Example:**

  ```rust
  let count = dataset.len();
  ```

#### `ReasoningDataset::is_empty(&self) -> bool`

Returns true if the dataset is empty.

- **Returns:** Boolean indicating if the dataset is empty
- **Example:**

  ```rust
  if dataset.is_empty() {
      println!("Dataset is empty");
  }
  ```

#### `ReasoningDataset::create_template(user: &str, reasoning: &str, assistant: &str) -> String`

Creates a template string from user, reasoning, and assistant content.

- **Parameters:**
  - `user`: User's question or request
  - `reasoning`: Detailed reasoning steps
  - `assistant`: Assistant's response
- **Returns:** Formatted template string
- **Behavior:** Creates a template with `<|im_start|>` and `<|im_end|>` tokens
- **Example:**

  ```rust
  let template = ReasoningDataset::create_template(
      "What is X?",
      "X can be determined by...",
      "X is Y"
  );
  ```

### Utility Functions

#### `split_content(content: &str) -> (Vec<String>, String)`

Splits content into tags and sentences.

- **Parameters:**
  - `content`: Text content to split
- **Returns:** Tuple of (tags vector, sentences string)
- **Behavior:**
  - Identifies the tag portion (comma-separated items before the first sentence)
  - Extracts the sentence portion (text after the tags)
- **Example:**

  ```rust
  let (tags, text) = split_content("tag1, tag2, tag3., This is the main text.");
  // tags = ["tag1", "tag2", "tag3"]
  // text = "This is the main text."
  ```

## Usage Examples

### SafeTensors Metadata Extraction

```rust
use dset::{process_safetensors_file, get_json_metadata};
use std::path::Path;
use anyhow::Result;

async fn extract_metadata(path: &str) -> Result<()> {
    // Extracts metadata and saves it as a JSON file
    process_safetensors_file(Path::new(path)).await?;
    
    // The output will be saved as "{path}.json"
    
    // Alternatively, get the metadata directly
    let metadata = get_json_metadata(Path::new(path)).await?;
    println!("Model metadata: {}", metadata);
    
    Ok(())
}
```

### Caption File Processing

```rust
use dset::{
    caption::process_file,
    caption::process_json_to_caption,
    caption::caption_file_exists_and_not_empty
};
use std::path::Path;
use anyhow::Result;

async fn handle_captions() -> Result<()> {
    let path = Path::new("image1.txt");
    
    // Check if caption file exists and has content
    if caption_file_exists_and_not_empty(&path).await {
        // Process the caption file (auto-detects format)
        process_file(&path).await?;
    }
    
    // Convert JSON caption to text format
    process_json_to_caption(Path::new("image2.json")).await?;
    
    Ok(())
}
```

### File Operations

```rust
use dset::rename_file_without_image_extension;
use std::path::Path;
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

The library provides two main types of JSON processing capabilities besides the e621 caption processing:

#### 1. Tag Probability JSON Processing

Converts JSON files containing tag-probability pairs into caption files. Tags with probabilities above 0.2 are included in the output.

```json
{
    "tag1": 0.9,
    "tag2": 0.5,
    "tag3": 0.1
}
```

The above JSON would be converted to a caption file containing:

```plaintext
tag1, tag2
```

Note that:

- Tags are sorted by probability in descending order
- Only tags with probability >= 0.2 are included
- Special characters in tags are escaped (e.g., parentheses)
- The output is saved as a .txt file with the same base name

Example usage:

```rust
use dset::process_json_to_caption;
use std::path::Path;
use anyhow::Result;

async fn process_tags() -> Result<()> {
    // Process a JSON file containing tag probabilities
    // Input: tags.json
    // {
    //     "person": 0.98,
    //     "smiling": 0.85,
    //     "outdoor": 0.45,
    //     "blurry": 0.15
    // }
    // 
    // Output: tags.txt
    // person, smiling, outdoor
    process_json_to_caption(Path::new("tags.json")).await?;
    
    Ok(())
}
```

Both functions handle errors gracefully and provide async processing capabilities.

#### 2. General JSON Processing

The library provides two functions for general JSON handling:

1. `format_json_file`: Pretty prints any JSON file with proper indentation
2. `process_json_file`: Allows custom processing of JSON data with an async handler

Example usage:

```rust
use dset::{format_json_file, process_json_file};
use std::path::{Path, PathBuf};
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

Both functions handle errors gracefully and provide async processing capabilities.

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
use dset::process_safetensors_file;
use std::path::Path;
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
