use serde::{Deserialize, Serialize};
use anyhow::Result;
use std::path::Path;
use tokio::fs;

/// Represents a single message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// The content of the message
    pub content: String,
    /// The role of the speaker (e.g., "user", "reasoning", "assistant")
    pub role: String,
}

/// Represents a complete reasoning dataset entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningEntry {
    /// The user's question or request
    pub user: String,
    /// Detailed step-by-step reasoning addressing the user's request
    pub reasoning: String,
    /// Assistant's summarized or direct response
    pub assistant: String,
    /// A structured template combining the roles of 'user', 'reasoning', and 'assistant'
    pub template: String,
    /// List of messages exchanged in the conversation
    pub conversations: Vec<Message>,
}

/// Represents a collection of reasoning dataset entries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningDataset {
    /// Vector of reasoning entries
    pub entries: Vec<ReasoningEntry>,
}

impl ReasoningDataset {
    /// Creates a new empty ReasoningDataset
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    /// Loads a reasoning dataset from a JSON file
    pub async fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path).await?;
        let dataset: ReasoningDataset = serde_json::from_str(&content)?;
        Ok(dataset)
    }

    /// Saves the reasoning dataset to a JSON file
    pub async fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content).await?;
        Ok(())
    }

    /// Adds a new entry to the dataset
    pub fn add_entry(&mut self, entry: ReasoningEntry) {
        self.entries.push(entry);
    }

    /// Returns the number of entries in the dataset
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns true if the dataset is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Creates a template string from user, reasoning, and assistant content
    pub fn create_template(user: &str, reasoning: &str, assistant: &str) -> String {
        format!(
            "<|im_start|>user\n{}<|im_end|>\n<|im_start|>reasoning\n{}<|im_end|>\n<|im_start|>assistant\n{}<|im_end|>",
            user, reasoning, assistant
        )
    }
}

impl Default for ReasoningDataset {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_dataset_operations() -> Result<()> {
        // Create a new dataset
        let mut dataset = ReasoningDataset::new();
        
        // Create a test entry
        let entry = ReasoningEntry {
            user: "What motivates Luna?".to_string(),
            reasoning: "Luna's motivations can be analyzed...".to_string(),
            assistant: "Luna is motivated by acceptance and self-expression.".to_string(),
            template: ReasoningDataset::create_template(
                "What motivates Luna?",
                "Luna's motivations can be analyzed...",
                "Luna is motivated by acceptance and self-expression.",
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
                    content: "Luna is motivated by acceptance and self-expression.".to_string(),
                    role: "assistant".to_string(),
                },
            ],
        };

        // Add entry to dataset
        dataset.add_entry(entry);
        assert_eq!(dataset.len(), 1);

        // Test save and load
        let temp_file = NamedTempFile::new()?;
        dataset.save(temp_file.path()).await?;
        let loaded_dataset = ReasoningDataset::load(temp_file.path()).await?;
        assert_eq!(loaded_dataset.len(), 1);

        Ok(())
    }
} 