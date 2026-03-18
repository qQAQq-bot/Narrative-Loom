use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechniqueCard {
    pub id: String,
    pub chapter_id: String,
    pub technique_type: TechniqueType,
    pub title: String,
    pub description: String,
    pub mechanism: String,
    pub evidence: Vec<super::evidence::Evidence>,
    pub tags: Vec<String>,
    pub collected: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TechniqueType {
    Structure,
    Scene,
    Character,
    Dialogue,
    Description,
    Pacing,
    Suspense,
    Foreshadowing,
    Theme,
    Voice,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeCard {
    pub id: String,
    pub chapter_id: String,
    pub knowledge_type: KnowledgeType,
    pub title: String,
    pub content: serde_json::Value,
    pub evidence: Vec<super::evidence::Evidence>,
    pub confidence: Confidence,
    pub status: CardStatus,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeType {
    Character,
    Setting,
    Event,
    Timeline,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardStatus {
    Pending,
    Accepted,
    Rejected,
    Merged,
}

impl CardStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
            Self::Merged => "merged",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "accepted" => Some(Self::Accepted),
            "rejected" => Some(Self::Rejected),
            "merged" => Some(Self::Merged),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    High,
    Medium,
    Low,
}
