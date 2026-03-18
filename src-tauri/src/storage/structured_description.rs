// Structured description types for characters and settings
// Enables accumulated descriptions across chapters with dimension-based storage

use serde::{Deserialize, Serialize};

/// Structured description for characters
/// Each dimension accumulates details from multiple chapters
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CharacterStructuredDescription {
    /// Physical appearance (height, build, distinctive features, clothing style)
    #[serde(default)]
    pub appearance: Vec<DescriptionEntry>,

    /// Personality traits and behaviors
    #[serde(default)]
    pub personality: Vec<DescriptionEntry>,

    /// Background, history, origin story
    #[serde(default)]
    pub background: Vec<DescriptionEntry>,

    /// Skills, abilities, powers
    #[serde(default)]
    pub abilities: Vec<DescriptionEntry>,

    /// Goals, motivations, desires
    #[serde(default)]
    pub goals: Vec<DescriptionEntry>,

    /// Current status, condition, situation
    #[serde(default)]
    pub status: Vec<DescriptionEntry>,

    /// Relationships mentioned (separate from the relationships_json field)
    #[serde(default)]
    pub relationship_notes: Vec<DescriptionEntry>,
}

/// Structured description for settings
/// Each dimension accumulates details from multiple chapters
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SettingStructuredDescription {
    /// Physical characteristics (size, appearance, structure)
    #[serde(default)]
    pub physical: Vec<DescriptionEntry>,

    /// Atmosphere, mood, ambiance
    #[serde(default)]
    pub atmosphere: Vec<DescriptionEntry>,

    /// History, origin, backstory
    #[serde(default)]
    pub history: Vec<DescriptionEntry>,

    /// Function, purpose, usage
    #[serde(default)]
    pub function: Vec<DescriptionEntry>,

    /// Rules, laws, customs
    #[serde(default)]
    pub rules: Vec<DescriptionEntry>,

    /// Inhabitants, residents, population
    #[serde(default)]
    pub inhabitants: Vec<DescriptionEntry>,

    /// Current status, condition
    #[serde(default)]
    pub status: Vec<DescriptionEntry>,
}

/// A single description entry with source chapter info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DescriptionEntry {
    /// The description text
    pub text: String,

    /// Chapter ID where this was discovered
    pub chapter_id: String,

    /// Chapter index for ordering/display
    pub chapter_index: u32,

    /// Evidence quote from the text
    #[serde(default)]
    pub evidence: Option<String>,
}

impl CharacterStructuredDescription {
    /// Create a new empty structured description
    pub fn new() -> Self {
        Self::default()
    }

    /// Merge another structured description into this one
    /// Avoids duplicates by checking text similarity
    pub fn merge(&mut self, other: &CharacterStructuredDescription) {
        merge_entries(&mut self.appearance, &other.appearance);
        merge_entries(&mut self.personality, &other.personality);
        merge_entries(&mut self.background, &other.background);
        merge_entries(&mut self.abilities, &other.abilities);
        merge_entries(&mut self.goals, &other.goals);
        merge_entries(&mut self.status, &other.status);
        merge_entries(&mut self.relationship_notes, &other.relationship_notes);
    }

    /// Add an appearance entry
    pub fn add_appearance(&mut self, text: String, chapter_id: String, chapter_index: u32, evidence: Option<String>) {
        self.appearance.push(DescriptionEntry { text, chapter_id, chapter_index, evidence });
    }

    /// Add a personality entry
    pub fn add_personality(&mut self, text: String, chapter_id: String, chapter_index: u32, evidence: Option<String>) {
        self.personality.push(DescriptionEntry { text, chapter_id, chapter_index, evidence });
    }

    /// Add a background entry
    pub fn add_background(&mut self, text: String, chapter_id: String, chapter_index: u32, evidence: Option<String>) {
        self.background.push(DescriptionEntry { text, chapter_id, chapter_index, evidence });
    }

    /// Add an abilities entry
    pub fn add_abilities(&mut self, text: String, chapter_id: String, chapter_index: u32, evidence: Option<String>) {
        self.abilities.push(DescriptionEntry { text, chapter_id, chapter_index, evidence });
    }

    /// Add a goals entry
    pub fn add_goals(&mut self, text: String, chapter_id: String, chapter_index: u32, evidence: Option<String>) {
        self.goals.push(DescriptionEntry { text, chapter_id, chapter_index, evidence });
    }

    /// Add a status entry
    pub fn add_status(&mut self, text: String, chapter_id: String, chapter_index: u32, evidence: Option<String>) {
        self.status.push(DescriptionEntry { text, chapter_id, chapter_index, evidence });
    }

    /// Generate a plain text summary from all dimensions
    pub fn to_plain_text(&self) -> String {
        let mut parts = Vec::new();

        if !self.appearance.is_empty() {
            let text = self.appearance.iter().map(|e| e.text.as_str()).collect::<Vec<_>>().join("；");
            parts.push(format!("【外貌】{}", text));
        }
        if !self.personality.is_empty() {
            let text = self.personality.iter().map(|e| e.text.as_str()).collect::<Vec<_>>().join("；");
            parts.push(format!("【性格】{}", text));
        }
        if !self.background.is_empty() {
            let text = self.background.iter().map(|e| e.text.as_str()).collect::<Vec<_>>().join("；");
            parts.push(format!("【背景】{}", text));
        }
        if !self.abilities.is_empty() {
            let text = self.abilities.iter().map(|e| e.text.as_str()).collect::<Vec<_>>().join("；");
            parts.push(format!("【能力】{}", text));
        }
        if !self.goals.is_empty() {
            let text = self.goals.iter().map(|e| e.text.as_str()).collect::<Vec<_>>().join("；");
            parts.push(format!("【目标】{}", text));
        }
        if !self.status.is_empty() {
            let text = self.status.iter().map(|e| e.text.as_str()).collect::<Vec<_>>().join("；");
            parts.push(format!("【状态】{}", text));
        }

        parts.join("\n")
    }
}

impl SettingStructuredDescription {
    /// Create a new empty structured description
    pub fn new() -> Self {
        Self::default()
    }

    /// Merge another structured description into this one
    pub fn merge(&mut self, other: &SettingStructuredDescription) {
        merge_entries(&mut self.physical, &other.physical);
        merge_entries(&mut self.atmosphere, &other.atmosphere);
        merge_entries(&mut self.history, &other.history);
        merge_entries(&mut self.function, &other.function);
        merge_entries(&mut self.rules, &other.rules);
        merge_entries(&mut self.inhabitants, &other.inhabitants);
        merge_entries(&mut self.status, &other.status);
    }

    /// Add a physical entry
    pub fn add_physical(&mut self, text: String, chapter_id: String, chapter_index: u32, evidence: Option<String>) {
        self.physical.push(DescriptionEntry { text, chapter_id, chapter_index, evidence });
    }

    /// Add an atmosphere entry
    pub fn add_atmosphere(&mut self, text: String, chapter_id: String, chapter_index: u32, evidence: Option<String>) {
        self.atmosphere.push(DescriptionEntry { text, chapter_id, chapter_index, evidence });
    }

    /// Add a history entry
    pub fn add_history(&mut self, text: String, chapter_id: String, chapter_index: u32, evidence: Option<String>) {
        self.history.push(DescriptionEntry { text, chapter_id, chapter_index, evidence });
    }

    /// Add a function entry
    pub fn add_function(&mut self, text: String, chapter_id: String, chapter_index: u32, evidence: Option<String>) {
        self.function.push(DescriptionEntry { text, chapter_id, chapter_index, evidence });
    }

    /// Add a rules entry
    pub fn add_rules(&mut self, text: String, chapter_id: String, chapter_index: u32, evidence: Option<String>) {
        self.rules.push(DescriptionEntry { text, chapter_id, chapter_index, evidence });
    }

    /// Add an inhabitants entry
    pub fn add_inhabitants(&mut self, text: String, chapter_id: String, chapter_index: u32, evidence: Option<String>) {
        self.inhabitants.push(DescriptionEntry { text, chapter_id, chapter_index, evidence });
    }

    /// Add a status entry
    pub fn add_status(&mut self, text: String, chapter_id: String, chapter_index: u32, evidence: Option<String>) {
        self.status.push(DescriptionEntry { text, chapter_id, chapter_index, evidence });
    }

    /// Generate a plain text summary from all dimensions
    pub fn to_plain_text(&self) -> String {
        let mut parts = Vec::new();

        if !self.physical.is_empty() {
            let text = self.physical.iter().map(|e| e.text.as_str()).collect::<Vec<_>>().join("；");
            parts.push(format!("【物理特征】{}", text));
        }
        if !self.atmosphere.is_empty() {
            let text = self.atmosphere.iter().map(|e| e.text.as_str()).collect::<Vec<_>>().join("；");
            parts.push(format!("【氛围】{}", text));
        }
        if !self.history.is_empty() {
            let text = self.history.iter().map(|e| e.text.as_str()).collect::<Vec<_>>().join("；");
            parts.push(format!("【历史】{}", text));
        }
        if !self.function.is_empty() {
            let text = self.function.iter().map(|e| e.text.as_str()).collect::<Vec<_>>().join("；");
            parts.push(format!("【功能】{}", text));
        }
        if !self.rules.is_empty() {
            let text = self.rules.iter().map(|e| e.text.as_str()).collect::<Vec<_>>().join("；");
            parts.push(format!("【规则】{}", text));
        }
        if !self.inhabitants.is_empty() {
            let text = self.inhabitants.iter().map(|e| e.text.as_str()).collect::<Vec<_>>().join("；");
            parts.push(format!("【居民】{}", text));
        }
        if !self.status.is_empty() {
            let text = self.status.iter().map(|e| e.text.as_str()).collect::<Vec<_>>().join("；");
            parts.push(format!("【状态】{}", text));
        }

        parts.join("\n")
    }
}

/// Helper function to merge entries avoiding duplicates
fn merge_entries(target: &mut Vec<DescriptionEntry>, source: &[DescriptionEntry]) {
    for entry in source {
        // Check if a similar entry already exists
        let exists = target.iter().any(|e| {
            e.text == entry.text ||
            (e.chapter_id == entry.chapter_id && texts_are_similar(&e.text, &entry.text))
        });

        if !exists {
            target.push(entry.clone());
        }
    }
}

/// Check if two text entries are similar enough to be considered duplicates
fn texts_are_similar(text1: &str, text2: &str) -> bool {
    // Simple containment check
    let t1 = text1.trim().to_lowercase();
    let t2 = text2.trim().to_lowercase();

    if t1.len() < 5 || t2.len() < 5 {
        return t1 == t2;
    }

    t1.contains(&t2) || t2.contains(&t1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_character_description_merge() {
        let mut desc1 = CharacterStructuredDescription::new();
        desc1.add_appearance("高大魁梧".to_string(), "ch1".to_string(), 1, None);
        desc1.add_personality("沉默寡言".to_string(), "ch1".to_string(), 1, None);

        let mut desc2 = CharacterStructuredDescription::new();
        desc2.add_appearance("一头黑发".to_string(), "ch2".to_string(), 2, None);
        desc2.add_appearance("高大魁梧".to_string(), "ch2".to_string(), 2, None); // Duplicate
        desc2.add_personality("性格刚烈".to_string(), "ch2".to_string(), 2, None);

        desc1.merge(&desc2);

        assert_eq!(desc1.appearance.len(), 2); // Should not include duplicate
        assert_eq!(desc1.personality.len(), 2);
    }

    #[test]
    fn test_to_plain_text() {
        let mut desc = CharacterStructuredDescription::new();
        desc.add_appearance("高大魁梧".to_string(), "ch1".to_string(), 1, None);
        desc.add_personality("沉默寡言".to_string(), "ch1".to_string(), 1, None);

        let text = desc.to_plain_text();
        assert!(text.contains("【外貌】"));
        assert!(text.contains("高大魁梧"));
        assert!(text.contains("【性格】"));
    }
}
