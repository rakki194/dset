#![warn(clippy::all, clippy::pedantic)]

use crate::caption::{process_e621_json_data, process_e621_tags, should_ignore_e621_tag};
use crate::process_e621_json_file;
use serde_json::json;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::fs;

#[test]
fn test_should_ignore_e621_tag() {
    // Test tags that should be ignored
    assert!(should_ignore_e621_tag("conditional_dnp"));
    assert!(should_ignore_e621_tag("2023"));  // Year
    assert!(should_ignore_e621_tag("16:9"));  // Aspect ratio
    
    // Test tags that should not be ignored
    assert!(!should_ignore_e621_tag("character"));
    assert!(!should_ignore_e621_tag("artist_name"));
    assert!(!should_ignore_e621_tag("red_background"));
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
    
    let processed_tags = process_e621_tags(&tags_json);
    
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
    
    // Process the mock data
    process_e621_json_data(&json_data, &file_path_arc).await?;
    
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
    
    // Create a mock e621 JSON file
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
    process_e621_json_file(&file_path).await?;
    
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
    let result = process_e621_json_file(&file_path).await;
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
    
    // This should not create a file since all tags are ignored
    let result = process_e621_json_file(&file_path).await;
    assert!(result.is_ok());
    
    // No caption file should be created since all tags are ignored
    let caption_path = temp_dir.path().join("empty_tags.txt");
    assert!(!caption_path.exists());
    
    Ok(())
}

#[tokio::test]
async fn test_process_real_e621_json_example() -> anyhow::Result<()> {
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
    process_e621_json_file(&file_path).await?;
    
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