use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub id: String,
    pub name: String,
    pub aliases: Vec<String>,
    pub description: String,
    pub traits: Vec<String>,
    pub role: CharacterRole,
    pub first_appearance: String,
    pub relationships: Vec<Relationship>,
    pub evidence: Vec<super::evidence::Evidence>,
    pub notes: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CharacterRole {
    Protagonist,
    Antagonist,
    Supporting,
    Minor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub target_id: String,
    pub relation_type: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Setting {
    pub id: String,
    pub setting_type: SettingType,
    pub name: String,
    pub description: String,
    pub properties: HashMap<String, String>,
    pub evidence: Vec<super::evidence::Evidence>,
    pub notes: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SettingType {
    Location,
    Organization,
    PowerSystem,
    Item,
    Rule,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub title: String,
    pub description: String,
    pub chapter_id: String,
    pub characters_involved: Vec<String>,
    pub importance: Importance,
    pub evidence: Vec<super::evidence::Evidence>,
    pub notes: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Importance {
    Critical,
    Major,
    Minor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEvent {
    pub id: String,
    pub event_id: Option<String>,
    pub title: String,
    pub chapter_id: String,
    pub order: u32,
    pub time_marker: Option<String>,
    pub is_uncertain: bool,
}
