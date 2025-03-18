#![warn(clippy::all, clippy::pedantic)]

use crate::caption::{
    E621Config, process_e621_json_data, process_e621_tags, should_ignore_e621_tag,
};
use crate::process_e621_json_file;
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::fs;

#[test]
fn test_should_ignore_e621_tag() {
    // Test tags that should be ignored
    assert!(should_ignore_e621_tag("conditional_dnp"));
    assert!(should_ignore_e621_tag("2023")); // Year
    assert!(should_ignore_e621_tag("16:9")); // Aspect ratio

    // Test tags that should not be ignored
    assert!(!should_ignore_e621_tag("character"));
    assert!(!should_ignore_e621_tag("artist_name"));
    assert!(!should_ignore_e621_tag("red_background"));
}

#[test]
fn test_artist_formatting() {
    // Create a mock JSON with artist tags
    let tags_json = json!({
        "artist": ["artist1", "artist2 (artist)", "artist_with_underscores"]
    });

    // Test default formatting (with "by " prefix)
    let processed_tags = process_e621_tags(&tags_json, None);
    assert!(processed_tags.contains(&"by artist1".to_string()));
    assert!(processed_tags.contains(&"by artist2".to_string()));
    assert!(processed_tags.contains(&"by artist with underscores".to_string()));

    // Test with custom prefix
    let config = E621Config::new().with_artist_prefix(Some("drawn by ".to_string()));
    let processed_tags = process_e621_tags(&tags_json, Some(&config));
    assert!(processed_tags.contains(&"drawn by artist1".to_string()));
    assert!(processed_tags.contains(&"drawn by artist2".to_string()));

    // Test with custom suffix
    let config = E621Config::new()
        .with_artist_prefix(None)
        .with_artist_suffix(Some(" (Artist)".to_string()));
    let processed_tags = process_e621_tags(&tags_json, Some(&config));
    assert!(processed_tags.contains(&"artist1 (Artist)".to_string()));
    assert!(processed_tags.contains(&"artist2 (Artist)".to_string()));

    // Test with both prefix and suffix
    let config = E621Config::new()
        .with_artist_prefix(Some("art by ".to_string()))
        .with_artist_suffix(Some(" (verified)".to_string()));
    let processed_tags = process_e621_tags(&tags_json, Some(&config));
    assert!(processed_tags.contains(&"art by artist1 (verified)".to_string()));
    assert!(processed_tags.contains(&"art by artist2 (verified)".to_string()));

    // Test with no prefix or suffix
    let config = E621Config::new()
        .with_artist_prefix(None)
        .with_artist_suffix(None);
    let processed_tags = process_e621_tags(&tags_json, Some(&config));
    assert!(processed_tags.contains(&"artist1".to_string()));
    assert!(processed_tags.contains(&"artist2".to_string()));
}

#[test]
fn test_process_e621_tags() {
    // Create a mock JSON with different tag categories
    let tags_json = json!({
        "artist": ["artist1", "artist2 (artist)", "artist_with_underscores"],
        "character": ["character1", "character_name"],
        "species": ["wolf", "canine"],
        "general": ["red_background", "2023", "conditional_dnp", "16:9"]
    });

    // Test with default config (filtering enabled, underscore replacement enabled)
    let processed_tags = process_e621_tags(&tags_json, None);

    // Check artist formatting
    assert!(processed_tags.contains(&"by artist1".to_string()));
    assert!(processed_tags.contains(&"by artist2".to_string())); // (artist) should be removed
    assert!(processed_tags.contains(&"by artist with underscores".to_string())); // underscores replaced

    // Check character tags
    assert!(processed_tags.contains(&"character1".to_string()));
    assert!(processed_tags.contains(&"character name".to_string())); // underscores replaced

    // Check species tags
    assert!(processed_tags.contains(&"wolf".to_string()));
    assert!(processed_tags.contains(&"canine".to_string()));

    // Check general tags - excluded patterns should be filtered out
    assert!(processed_tags.contains(&"red background".to_string())); // underscores replaced
    assert!(!processed_tags.contains(&"2023".to_string())); // year should be ignored
    assert!(!processed_tags.contains(&"conditional_dnp".to_string())); // conditional_dnp should be ignored
    assert!(!processed_tags.contains(&"16:9".to_string())); // aspect ratio should be ignored

    // Test with filtering disabled but underscore replacement still enabled
    let config = E621Config::new()
        .with_filter_tags(false)
        .with_replace_underscores(true);
    let processed_tags = process_e621_tags(&tags_json, Some(&config));

    // All tags should be included when filtering is disabled
    assert!(processed_tags.contains(&"2023".to_string())); // year should be included
    assert!(processed_tags.contains(&"conditional dnp".to_string())); // conditional_dnp should be included with spaces
    assert!(processed_tags.contains(&"16:9".to_string())); // aspect ratio should be included
    assert!(processed_tags.contains(&"red background".to_string())); // underscores still replaced

    // Test with both filtering and underscore replacement disabled
    let config = E621Config::new()
        .with_filter_tags(false)
        .with_replace_underscores(false);
    let processed_tags = process_e621_tags(&tags_json, Some(&config));

    // Tags should preserve underscores and include filtered tags
    assert!(processed_tags.contains(&"red_background".to_string())); // underscores preserved
    assert!(processed_tags.contains(&"2023".to_string())); // year included
    assert!(processed_tags.contains(&"conditional_dnp".to_string())); // conditional_dnp included
    assert!(processed_tags.contains(&"16:9".to_string())); // aspect ratio included
    assert!(processed_tags.contains(&"character_name".to_string())); // underscores preserved
}

#[tokio::test]
async fn test_process_e621_json_data() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test_e621_post.json");

    // Create mock e621 JSON data with various tags
    let json_data = json!({
        "post": {
            "file": {
                "url": "https://e621.net/posts/12345/example_image.jpg"
            },
            "rating": "s", // safe rating
            "tags": {
                "artist": ["artist1", "artist2 (artist)"],
                "character": ["character1", "character_name"],
                "species": ["wolf", "canine"],
                "general": ["red_background", "2023", "conditional_dnp"]
            }
        }
    });

    let file_path_arc = Arc::new(file_path.clone());

    // Process the mock data with default config
    process_e621_json_data(&json_data, &file_path_arc, None).await?;

    // Check that the caption file was created
    let caption_path = temp_dir.path().join("example_image.txt");
    assert!(caption_path.exists());

    // Check the content of the caption file
    let content = fs::read_to_string(&caption_path).await?;

    // Verify rating is included
    assert!(content.starts_with("safe, "));

    // Verify artist tags are formatted correctly
    assert!(content.contains("by artist1"));
    assert!(content.contains("by artist2"));

    // Verify character tags are included and formatted correctly
    assert!(content.contains("character1"));
    assert!(content.contains("character name")); // underscores replaced

    // Verify species tags are included
    assert!(content.contains("wolf"));
    assert!(content.contains("canine"));

    // Verify ignored tags are not included
    assert!(!content.contains("2023"));
    assert!(!content.contains("conditional_dnp"));

    Ok(())
}

#[tokio::test]
async fn test_process_e621_json_file() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test_e621_post.json");

    // Create mock e621 JSON data with various tags
    let json_data = json!({
        "post": {
            "file": {
                "url": "https://e621.net/posts/12345/test_image.jpg"
            },
            "rating": "q", // questionable rating
            "tags": {
                "artist": ["artist1"],
                "character": ["character1"],
                "species": ["wolf"],
                "general": ["red_background"]
            }
        }
    });

    fs::write(&file_path, serde_json::to_string_pretty(&json_data)?).await?;

    // Process the file
    process_e621_json_file(&file_path, None).await?;

    // Check that the caption file was created
    let caption_path = temp_dir.path().join("test_image.txt");
    assert!(caption_path.exists());

    // Check the content of the caption file
    let content = fs::read_to_string(&caption_path).await?;

    // Verify rating is included
    assert!(content.starts_with("questionable, "));

    // Verify tags are included
    assert!(content.contains("by artist1"));
    assert!(content.contains("character1"));
    assert!(content.contains("wolf"));
    assert!(content.contains("red background"));

    Ok(())
}

#[tokio::test]
async fn test_process_e621_json_file_missing_data() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("invalid_e621.json");

    // Create JSON without required fields
    let invalid_json = json!({
        "post": {
            // Missing file.url
            "rating": "s",
            "tags": {
                "artist": ["artist1"]
            }
        }
    });

    fs::write(&file_path, serde_json::to_string_pretty(&invalid_json)?).await?;

    // This should not create a caption file but also not throw an error
    let result = process_e621_json_file(&file_path, None).await;
    assert!(result.is_ok());

    Ok(())
}

#[tokio::test]
async fn test_process_e621_json_file_empty_tags() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("empty_tags.json");

    // Create JSON with empty tags
    let empty_tags_json = json!({
        "post": {
            "file": {
                "url": "https://e621.net/posts/12345/empty_tags.jpg"
            },
            "rating": "e", // explicit rating
            "tags": {
                // All empty or only ignored tags
                "artist": [],
                "character": [],
                "species": [],
                "general": ["2023", "conditional_dnp", "16:9"]
            }
        }
    });

    fs::write(&file_path, serde_json::to_string_pretty(&empty_tags_json)?).await?;

    // Process with default config (filtering enabled)
    let result = process_e621_json_file(&file_path, None).await;
    assert!(result.is_ok());

    // No caption file should be created since all tags are ignored
    let caption_path = temp_dir.path().join("empty_tags.txt");
    assert!(!caption_path.exists());

    // Test with filtering disabled
    let config = E621Config::new().with_filter_tags(false);
    let result = process_e621_json_file(&file_path, Some(config)).await;
    assert!(result.is_ok());

    // Caption file should exist now since filtering is disabled
    assert!(caption_path.exists());

    Ok(())
}

#[tokio::test]
async fn test_process_real_e621_example() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("falco_example.json");

    // Create JSON using the real e621 example
    let real_json = json!({
        "post": {
            "id": 5396672,
            "created_at": "2025-02-25T15:48:29.290+01:00",
            "updated_at": "2025-02-25T15:48:29.290+01:00",
            "file": {
                "width": 1301,
                "height": 1314,
                "ext": "jpg",
                "size": 272721,
                "md5": "d20854c9096914d82ac3acf050f5d373",
                "url": "https://static1.e621.net/data/d2/08/d20854c9096914d82ac3acf050f5d373.jpg"
            },
            "preview": {
                "width": 148,
                "height": 150,
                "url": "https://static1.e621.net/data/preview/d2/08/d20854c9096914d82ac3acf050f5d373.jpg"
            },
            "sample": {
                "has": true,
                "height": 858,
                "width": 850,
                "url": "https://static1.e621.net/data/sample/d2/08/d20854c9096914d82ac3acf050f5d373.jpg",
                "alternates": {}
            },
            "score": {
                "up": 0,
                "down": 0,
                "total": 0
            },
            "tags": {
                "general": [
                    "anthro",
                    "armor",
                    "blue_body",
                    "blue_eyes",
                    "blue_feathers",
                    "clothed",
                    "clothing",
                    "feathers",
                    "headgear",
                    "helmet",
                    "lying",
                    "male",
                    "on_front",
                    "reflector_(object)",
                    "scouter",
                    "smile",
                    "solo",
                    "tail"
                ],
                "artist": ["ulala_ko"],
                "contributor": [],
                "copyright": ["nintendo", "star_fox"],
                "character": ["falco_lombardi"],
                "species": ["avian"],
                "invalid": [],
                "meta": [
                    "hi_res",
                    "painting_(artwork)",
                    "traditional_media_(artwork)",
                    "watercolor_(artwork)"
                ],
                "lore": []
            },
            "locked_tags": [],
            "change_seq": 64555418,
            "flags": {
                "pending": true,
                "flagged": false,
                "note_locked": false,
                "status_locked": false,
                "rating_locked": false,
                "deleted": false
            },
            "rating": "s",
            "fav_count": 0,
            "sources": [
                "https://x.com/Ulala_ko/status/1892467502606618839",
                "https://pbs.twimg.com/media/Gj6zVDAa4AAhzg7?format=jpg&name=orig"
            ],
            "pools": [],
            "relationships": {
                "parent_id": null,
                "has_children": false,
                "has_active_children": false,
                "children": []
            },
            "approver_id": null,
            "uploader_id": 55942,
            "description": "",
            "comment_count": 0,
            "is_favorited": false,
            "has_notes": false,
            "duration": null
        }
    });

    fs::write(&file_path, serde_json::to_string_pretty(&real_json)?).await?;

    // Process the file
    process_e621_json_file(&file_path, None).await?;

    // Check that the caption file was created with correct filename
    // The MD5 hash in the URL becomes the filename
    let caption_path = temp_dir.path().join("d20854c9096914d82ac3acf050f5d373.txt");
    assert!(caption_path.exists());

    // Check the content of the caption file
    let content = fs::read_to_string(&caption_path).await?;

    // Verify rating is included (should be "safe")
    assert!(content.starts_with("safe, "));

    // Verify important tags are included
    // Artist
    assert!(content.contains("by ulala ko"));

    // Character
    assert!(content.contains("falco lombardi"));

    // Species
    assert!(content.contains("avian"));

    // Copyright
    assert!(content.contains("nintendo"));
    assert!(content.contains("star fox"));

    // Selected meta tags
    assert!(content.contains("painting (artwork)"));
    assert!(content.contains("traditional media (artwork)"));
    assert!(content.contains("watercolor (artwork)"));

    // General tags (check a few)
    assert!(content.contains("anthro"));
    assert!(content.contains("blue eyes"));
    assert!(content.contains("armor"));

    Ok(())
}

#[tokio::test]
async fn test_process_real_e621_test_files() -> anyhow::Result<()> {
    // Process each of the real test files
    for i in 1..=3 {
        let file_path = PathBuf::from("../test_data").join(format!("e621-{}.json", i));

        // Process the file and ensure it succeeds
        let result = process_e621_json_file(&file_path, None).await;
        assert!(result.is_ok(), "Failed to process e621-{}.json", i);

        // Read the file content to verify specific aspects
        let content = fs::read_to_string(&file_path).await?;
        let json_data: serde_json::Value = serde_json::from_str(&content)?;

        if let Some(post) = json_data.get("post") {
            // Verify the post has required fields
            assert!(
                post.get("file").is_some(),
                "e621-{}.json missing file field",
                i
            );
            assert!(
                post.get("tags").is_some(),
                "e621-{}.json missing tags field",
                i
            );
            assert!(
                post.get("rating").is_some(),
                "e621-{}.json missing rating field",
                i
            );

            // Get the filename from the URL
            if let Some(url) = post["file"]["url"].as_str() {
                let filename = url.split('/').last().unwrap_or_default();
                let caption_path = PathBuf::from("../test_data").join(format!(
                    "{}.txt",
                    filename.split('.').next().unwrap_or_default()
                ));

                // Verify the caption file was created
                assert!(
                    caption_path.exists(),
                    "Caption file not created for e621-{}.json",
                    i
                );

                // Read and verify the caption content
                let caption_content = fs::read_to_string(&caption_path).await?;

                // Verify rating is included
                let rating = post["rating"].as_str().unwrap_or_default();
                let rating_text = match rating {
                    "s" => "safe",
                    "q" => "questionable",
                    "e" => "explicit",
                    _ => rating,
                };
                assert!(
                    caption_content.starts_with(&format!("{}, ", rating_text)),
                    "Caption doesn't start with correct rating for e621-{}.json",
                    i
                );

                // Verify artist tags are included
                if let Some(artists) = post["tags"]["artist"].as_array() {
                    for artist in artists {
                        if let Some(artist_name) = artist.as_str() {
                            let formatted_artist = format!(
                                "by {}",
                                artist_name.replace('_', " ").replace(" (artist)", "")
                            );
                            assert!(
                                caption_content.contains(&formatted_artist),
                                "Artist '{}' not found in caption for e621-{}.json",
                                artist_name,
                                i
                            );
                        }
                    }
                }

                // Clean up the generated caption file
                let _ = fs::remove_file(&caption_path).await;
            }
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_e621_config() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test.json");

    // Create test JSON data
    let json_data = json!({
        "post": {
            "file": {
                "url": "https://example.com/123abc.jpg"
            },
            "rating": "s",
            "tags": {
                "artist": ["artist1", "artist2 (artist)"],
                "character": ["character1"],
                "general": ["tag1", "2023", "16:9"]
            }
        }
    });

    fs::write(&file_path, json_data.to_string()).await?;

    // Test default config
    process_e621_json_file(&file_path, None).await?;
    let caption_path = temp_dir.path().join("123abc.txt");
    let content = fs::read_to_string(&caption_path).await?;
    assert!(content.starts_with("safe")); // Default rating conversion
    assert!(content.contains("by artist1")); // Default artist formatting
    assert!(!content.contains("2023")); // Default filtering enabled

    // Test custom rating conversions
    let mut custom_ratings = std::collections::HashMap::new();
    custom_ratings.insert("s".to_string(), "sfw".to_string());

    let config = E621Config::new().with_rating_conversions(Some(custom_ratings));

    process_e621_json_file(&file_path, Some(config)).await?;
    let content = fs::read_to_string(&caption_path).await?;
    assert!(content.starts_with("sfw")); // Custom rating conversion

    // Test custom format
    let config =
        E621Config::new().with_format(Some("Rating: {rating}\nArtists: {artists}".to_string()));

    process_e621_json_file(&file_path, Some(config)).await?;
    let content = fs::read_to_string(&caption_path).await?;
    assert!(content.starts_with("Rating: safe\n")); // Custom format
    assert!(content.contains("Artists: by artist1")); // Custom format with artists

    // Test raw ratings (no conversion)
    let config = E621Config::new().with_rating_conversions(None);

    process_e621_json_file(&file_path, Some(config)).await?;
    let content = fs::read_to_string(&caption_path).await?;
    assert!(content.starts_with("s")); // Raw rating

    // Test disabled filtering
    let config = E621Config::new().with_filter_tags(false);

    process_e621_json_file(&file_path, Some(config)).await?;
    let content = fs::read_to_string(&caption_path).await?;
    assert!(content.contains("2023")); // Year tag included when filtering disabled
    assert!(content.contains("16:9")); // Aspect ratio included when filtering disabled

    Ok(())
}
