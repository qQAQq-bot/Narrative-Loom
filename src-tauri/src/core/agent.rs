use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub id: String,
    pub name: String,
    pub kind: AgentKind,
    pub enabled: bool,
    pub provider_id: String,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: Option<u32>,
    pub system_prompt: Option<String>,
    pub output_mode: OutputMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentKind {
    BuiltIn(BuiltInAgent),
    Custom,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuiltInAgent {
    TechniqueAnalysis,
    CharacterExtraction,
    SettingExtraction,
    EventExtraction,
    StyleAnalysis,
    TimelineExtraction, // Deprecated, kept for backwards compatibility
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputMode {
    Text,
    JsonObject,
    JsonSchema { schema: serde_json::Value },
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    TechniqueAnalysis,
    CharacterExtraction,
    SettingExtraction,
    EventExtraction,
    StyleAnalysis,
    TimelineExtraction, // Deprecated, kept for backwards compatibility
}

/// Default per-chapter analysis types when caller doesn't specify.
///
/// Note: This matches the Python sidecar defaults and the UI's default enabled
/// agents (excluding style, which is opt-in).
pub const DEFAULT_CHAPTER_ANALYSIS_TYPES: [&str; 4] = ["technique", "character", "setting", "event"];

impl TaskType {
    /// Convert a Python-side analysis type string (e.g. `"technique"`) into a `TaskType`.
    ///
    /// We intentionally do **not** map `"timeline"` here. Timeline extraction was deprecated and
    /// removed from the main pipeline; time-related fields now live on `events`.
    pub fn from_analysis_type(s: &str) -> Option<Self> {
        match s {
            "technique" => Some(Self::TechniqueAnalysis),
            "character" => Some(Self::CharacterExtraction),
            "setting" => Some(Self::SettingExtraction),
            "event" => Some(Self::EventExtraction),
            "style" => Some(Self::StyleAnalysis),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskBindings {
    pub bindings: HashMap<TaskType, String>,
}
