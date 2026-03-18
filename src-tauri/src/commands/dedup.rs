// Card intelligent deduplication module
// Handles three types of name deduplication:
// 1. Combo names (names containing "&", "和", "与", "、")
// 2. Containment (one name is a substring of another)
// 3. Similarity (Jaro-Winkler similarity > 0.8)

use serde::{Deserialize, Serialize};
use strsim::jaro_winkler;

/// Result of deduplication check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DedupResult {
    /// Whether the name should be skipped
    pub should_skip: bool,
    /// Reason for skipping (if applicable)
    pub reason: Option<String>,
    /// ID of the existing entity to merge with (if applicable)
    pub merge_with: Option<String>,
    /// Name of the existing entity (for logging)
    pub merge_with_name: Option<String>,
}

impl DedupResult {
    pub fn accept() -> Self {
        Self {
            should_skip: false,
            reason: None,
            merge_with: None,
            merge_with_name: None,
        }
    }

    pub fn skip(reason: &str) -> Self {
        Self {
            should_skip: true,
            reason: Some(reason.to_string()),
            merge_with: None,
            merge_with_name: None,
        }
    }

    pub fn merge(id: &str, name: &str, reason: &str) -> Self {
        Self {
            should_skip: false,
            reason: Some(reason.to_string()),
            merge_with: Some(id.to_string()),
            merge_with_name: Some(name.to_string()),
        }
    }
}

/// Known entity info for deduplication matching
#[derive(Debug, Clone)]
pub struct KnownEntity {
    pub id: String,
    pub name: String,
    pub aliases: Vec<String>,
}

/// Check if a name is a combo name (multiple entities combined)
/// Examples: "张三 & 李四", "王五和赵六", "A与B", "甲、乙"
pub fn is_combo_name(name: &str) -> bool {
    let combo_patterns = ["&", " & ", "和", "与", "、", " and ", " AND "];

    for pattern in &combo_patterns {
        if name.contains(pattern) {
            // Make sure it's not part of a proper name
            // e.g., "Johnson & Johnson" should be treated as a single entity in some cases
            // but for Chinese names like "张三和李四", it's clearly a combo
            let parts: Vec<&str> = name.split(pattern).collect();
            if parts.len() >= 2 && parts.iter().all(|p| !p.trim().is_empty()) {
                return true;
            }
        }
    }

    false
}

/// Check if new_name is contained in or contains existing_name
/// Returns true if there's a containment relationship (suggesting they're the same entity)
fn is_name_contained(new_name: &str, existing_name: &str) -> bool {
    let new_normalized = normalize_name(new_name);
    let existing_normalized = normalize_name(existing_name);

    // Skip if either name is too short
    if new_normalized.len() < 2 || existing_normalized.len() < 2 {
        return false;
    }

    // Check containment: "费舍尔" is contained in "费舍尔·贝纳维德斯"
    if existing_normalized.contains(&new_normalized) || new_normalized.contains(&existing_normalized) {
        // Additional check: the contained name should be at least 2 characters
        let shorter = if new_normalized.len() < existing_normalized.len() {
            &new_normalized
        } else {
            &existing_normalized
        };

        // The contained name should be meaningful (at least 2 Chinese chars or 3 letters)
        if shorter.chars().count() >= 2 {
            return true;
        }
    }

    false
}

/// Calculate name similarity using Jaro-Winkler algorithm
/// Returns true if similarity is above threshold (0.8)
fn is_name_similar(new_name: &str, existing_name: &str, threshold: f64) -> bool {
    let new_normalized = normalize_name(new_name);
    let existing_normalized = normalize_name(existing_name);

    // Skip if either name is too short
    if new_normalized.len() < 2 || existing_normalized.len() < 2 {
        return false;
    }

    let similarity = jaro_winkler(&new_normalized, &existing_normalized);
    similarity >= threshold
}

/// Normalize a name for comparison
/// Removes common suffixes, list markers, normalizes whitespace, converts to lowercase
fn normalize_name(name: &str) -> String {
    let mut normalized = name.to_lowercase();

    // Remove common list markers like "~1", "~2", "#1", etc.
    let list_marker_patterns = [
        regex::Regex::new(r"[~#]\d+$").unwrap(),
        regex::Regex::new(r"\s*\(\d+\)$").unwrap(),
        regex::Regex::new(r"\s*（\d+）$").unwrap(),
    ];
    for pattern in &list_marker_patterns {
        normalized = pattern.replace(&normalized, "").to_string();
    }

    // Remove common suffixes for settings
    let suffixes_to_remove = ["种族", "族", "人种", "城市", "城"];
    for suffix in &suffixes_to_remove {
        if normalized.ends_with(suffix) && normalized.len() > suffix.len() + 1 {
            normalized = normalized[..normalized.len() - suffix.len()].to_string();
        }
    }

    // Normalize whitespace and punctuation
    normalized = normalized
        .replace("·", "")
        .replace("-", "")
        .replace("_", "")
        .replace(" ", "")
        .trim()
        .to_string();

    normalized
}

/// Check a new entity name against existing entities for deduplication
/// Returns DedupResult indicating whether to skip, accept, or merge
pub fn check_entity_dedup(
    new_name: &str,
    existing_entities: &[KnownEntity],
    similarity_threshold: f64,
) -> DedupResult {
    // 1. Check if it's a combo name
    if is_combo_name(new_name) {
        return DedupResult::skip(&format!(
            "Combo name detected: '{}' contains multiple entities",
            new_name
        ));
    }

    let normalized_new = normalize_name(new_name);

    // 2. Check against each existing entity
    for entity in existing_entities {
        // Check exact match (case-insensitive)
        if normalize_name(&entity.name) == normalized_new {
            return DedupResult::merge(
                &entity.id,
                &entity.name,
                &format!("Exact match: '{}' matches '{}'", new_name, entity.name),
            );
        }

        // Check aliases for exact match
        for alias in &entity.aliases {
            if normalize_name(alias) == normalized_new {
                return DedupResult::merge(
                    &entity.id,
                    &entity.name,
                    &format!("Alias match: '{}' matches alias '{}' of '{}'", new_name, alias, entity.name),
                );
            }
        }

        // Check containment (e.g., "费舍尔" in "费舍尔·贝纳维德斯")
        if is_name_contained(new_name, &entity.name) {
            return DedupResult::merge(
                &entity.id,
                &entity.name,
                &format!("Containment: '{}' and '{}' are likely the same", new_name, entity.name),
            );
        }

        // Check similarity
        if is_name_similar(new_name, &entity.name, similarity_threshold) {
            let similarity = jaro_winkler(&normalize_name(new_name), &normalize_name(&entity.name));
            return DedupResult::merge(
                &entity.id,
                &entity.name,
                &format!("Similar names: '{}' ~ '{}' (similarity: {:.2})", new_name, entity.name, similarity),
            );
        }
    }

    // No deduplication needed
    DedupResult::accept()
}

/// Filter a list of entity names, removing duplicates and combo names
/// Returns only the unique, valid names with their dedup info
pub fn filter_entity_names(
    new_names: &[String],
    existing_entities: &[KnownEntity],
    similarity_threshold: f64,
) -> Vec<(String, DedupResult)> {
    let mut results = Vec::new();
    let mut seen_names: Vec<String> = Vec::new();

    for name in new_names {
        // Check against existing entities
        let mut result = check_entity_dedup(name, existing_entities, similarity_threshold);

        // Also check against already-seen names in this batch
        if !result.should_skip && result.merge_with.is_none() {
            for seen in &seen_names {
                if is_name_contained(name, seen) || is_name_similar(name, seen, similarity_threshold) {
                    result = DedupResult::skip(&format!(
                        "Duplicate in batch: '{}' is similar to '{}'",
                        name, seen
                    ));
                    break;
                }
            }
        }

        if !result.should_skip {
            seen_names.push(name.clone());
        }

        results.push((name.clone(), result));
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_combo_name() {
        assert!(is_combo_name("张三 & 李四"));
        assert!(is_combo_name("可希尔 & 法希尔"));
        assert!(is_combo_name("王五和赵六"));
        assert!(is_combo_name("甲与乙"));
        assert!(is_combo_name("A、B、C"));

        assert!(!is_combo_name("张三"));
        assert!(!is_combo_name("费舍尔·贝纳维德斯"));
        assert!(!is_combo_name("Johnson"));
    }

    #[test]
    fn test_is_name_contained() {
        assert!(is_name_contained("费舍尔", "费舍尔·贝纳维德斯"));
        assert!(is_name_contained("费舍尔·贝纳维德斯", "费舍尔"));

        assert!(!is_name_contained("张", "张三")); // Too short
        assert!(!is_name_contained("王五", "赵六"));
    }

    #[test]
    fn test_is_name_similar() {
        assert!(is_name_similar("龙人种", "龙人种族", 0.8));
        assert!(is_name_similar("测试", "测试一下", 0.7));

        assert!(!is_name_similar("张三", "李四", 0.8));
    }

    #[test]
    fn test_check_entity_dedup() {
        let existing = vec![
            KnownEntity {
                id: "1".to_string(),
                name: "费舍尔·贝纳维德斯".to_string(),
                aliases: vec!["费舍尔".to_string()],
            },
            KnownEntity {
                id: "2".to_string(),
                name: "龙人种族".to_string(),
                aliases: vec![],
            },
        ];

        // Combo name should be skipped
        let result = check_entity_dedup("可希尔 & 法希尔", &existing, 0.8);
        assert!(result.should_skip);

        // Alias match should merge
        let result = check_entity_dedup("费舍尔", &existing, 0.8);
        assert!(!result.should_skip);
        assert_eq!(result.merge_with, Some("1".to_string()));

        // Similar name should merge
        let result = check_entity_dedup("龙人种", &existing, 0.8);
        assert!(!result.should_skip);
        assert_eq!(result.merge_with, Some("2".to_string()));

        // New name should be accepted
        let result = check_entity_dedup("新角色", &existing, 0.8);
        assert!(!result.should_skip);
        assert!(result.merge_with.is_none());
    }
}
