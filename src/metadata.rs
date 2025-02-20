#![warn(clippy::all, clippy::pedantic)]

use serde_json::Value;

/// Recursively decodes JSON-encoded strings within a `serde_json::Value`.
/// If a string equals `None`, it is converted to JSON null. If a string starts with `{` or `[` and ends with `}` or `]`,
/// it attempts to parse it as JSON and then recursively decodes its contents.
pub fn decode_json_strings(value: Value) -> Value {
    match value {
        Value::String(s) => {
            let trimmed = s.trim();
            if trimmed == "None" {
                Value::Null
            } else if (trimmed.starts_with('{') && trimmed.ends_with('}')) ||
                      (trimmed.starts_with('[') && trimmed.ends_with(']')) {
                match serde_json::from_str::<Value>(trimmed) {
                    Ok(parsed) => decode_json_strings(parsed),
                    Err(_) => Value::String(s),
                }
            } else {
                Value::String(s)
            }
        },
        Value::Object(map) => {
            let new_map = map.into_iter()
                .map(|(k,v)| (k, decode_json_strings(v)))
                .collect();
            Value::Object(new_map)
        },
        Value::Array(arr) => {
            Value::Array(arr.into_iter().map(decode_json_strings).collect())
        },
        other => other,
    }
}

/// Extracts the training metadata from the raw metadata.
/// If the raw metadata contains a `__metadata__` field, it decodes that field.
/// Otherwise, it decodes the entire metadata.
#[must_use]
pub fn extract_training_metadata(raw_metadata: &Value) -> Value {
    if let Value::Object(map) = raw_metadata {
        if let Some(meta) = map.get("__metadata__") {
            match meta {
                Value::String(s) => {
                    if let Ok(parsed) = serde_json::from_str::<Value>(s) {
                        decode_json_strings(parsed)
                    } else {
                        let mut new_map = serde_json::Map::new();
                        new_map.insert("invalid_json".to_string(), Value::String(s.clone()));
                        Value::Object(new_map)
                    }
                },
                other => decode_json_strings(other.clone()),
            }
        } else {
            // If no `__metadata__` field exists, decode the entire metadata
            decode_json_strings(raw_metadata.clone())
        }
    } else {
        Value::Object(serde_json::Map::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_decode_json_strings_none() {
        let value = Value::String("None".to_string());
        let decoded = decode_json_strings(value);
        assert_eq!(decoded, Value::Null);
    }

    #[test]
    fn test_decode_json_strings_object() {
        let input = json!({
            "resize_params": "{\"recipe_str\": \"fro_ckpt=1,thr=-3.55\", \"weights\": {\"spn_lora\": 0.0, \"spn_ckpt\": 0.0, \"subspace\": 0.0, \"fro_lora\": 0.0, \"fro_ckpt\": 1.0, \"params\": 0.0}, \"target_size\": null, \"threshold\": -3.55, \"rescale\": 1.0}"
        });
        let decoded = decode_json_strings(input);
        let expected = json!({
            "resize_params": {
                "recipe_str": "fro_ckpt=1,thr=-3.55",
                "weights": {
                    "spn_lora": 0.0,
                    "spn_ckpt": 0.0,
                    "subspace": 0.0,
                    "fro_lora": 0.0,
                    "fro_ckpt": 1.0,
                    "params": 0.0
                },
                "target_size": null,
                "threshold": -3.55,
                "rescale": 1.0
            }
        });
        assert_eq!(decoded, expected);
    }

    #[test]
    fn test_extract_training_metadata() {
        let raw = json!({
            "__metadata__": "{\"ss_bucket_info\": \"{\\\"buckets\\\": {\\\"0\\\": {\\\"resolution\\\": [1280, 800], \\\"count\\\": 78}}, \\\"mean_img_ar_error\\\": 0.0}\"}"
        });
        let extracted = extract_training_metadata(&raw);
        let expected = json!({
            "ss_bucket_info": {
                "buckets": {
                    "0": {
                        "resolution": [1280, 800],
                        "count": 78
                    }
                },
                "mean_img_ar_error": 0.0
            }
        });
        assert_eq!(extracted, expected);
    }
} 