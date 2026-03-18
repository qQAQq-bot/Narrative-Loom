use serde::{Deserialize, Serialize};

/// Position of a prompt card relative to the agent's system prompt
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptCardPosition {
    /// Applied before the agent's system prompt
    Prefix,
    /// Applied after the agent's system prompt
    Suffix,
}

/// A global system prompt card that can be applied to all agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptCard {
    /// Unique identifier for the card
    pub id: String,
    /// Short display name for the card
    pub title: String,
    /// The prompt text content
    pub content: String,
    /// Whether this card is currently enabled
    pub enabled: bool,
    /// Position relative to agent prompt (prefix or suffix)
    pub position: PromptCardPosition,
    /// Order within the same position (lower = earlier)
    pub order: i32,
    /// Last update timestamp (ISO8601)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

impl PromptCard {
    /// Create a new prompt card with default values
    pub fn new(id: String, title: String, content: String, position: PromptCardPosition) -> Self {
        Self {
            id,
            title,
            content,
            enabled: true,
            position,
            order: 0,
            updated_at: None,
        }
    }
}
