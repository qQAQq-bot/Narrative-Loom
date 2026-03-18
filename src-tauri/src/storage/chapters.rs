// Chapter text utility functions

/// Extract paragraphs from chapter content
pub fn extract_paragraphs(content: &str) -> Vec<&str> {
    content
        .split("\n\n")
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .collect()
}

/// Count Chinese characters (excluding punctuation and whitespace)
pub fn count_chinese_chars(text: &str) -> usize {
    text.chars()
        .filter(|c| {
            // CJK Unified Ideographs
            ('\u{4E00}'..='\u{9FFF}').contains(c) ||
            // CJK Unified Ideographs Extension A
            ('\u{3400}'..='\u{4DBF}').contains(c) ||
            // CJK Unified Ideographs Extension B
            ('\u{20000}'..='\u{2A6DF}').contains(c)
        })
        .count()
}

/// Estimate reading time in minutes (based on ~400 Chinese chars/min)
pub fn estimate_reading_time_minutes(char_count: usize) -> u32 {
    const CHARS_PER_MINUTE: usize = 400;
    ((char_count + CHARS_PER_MINUTE - 1) / CHARS_PER_MINUTE) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_paragraphs() {
        let content = "第一段\n\n第二段\n\n\n第三段";
        let paragraphs = extract_paragraphs(content);
        assert_eq!(paragraphs.len(), 3);
        assert_eq!(paragraphs[0], "第一段");
        assert_eq!(paragraphs[1], "第二段");
        assert_eq!(paragraphs[2], "第三段");
    }

    #[test]
    fn test_count_chinese_chars() {
        let text = "这是中文，有标点！ABC 123";
        let count = count_chinese_chars(text);
        assert_eq!(count, 4); // 这是中文
    }

    #[test]
    fn test_estimate_reading_time() {
        assert_eq!(estimate_reading_time_minutes(400), 1);
        assert_eq!(estimate_reading_time_minutes(800), 2);
        assert_eq!(estimate_reading_time_minutes(401), 2);
        assert_eq!(estimate_reading_time_minutes(0), 0);
    }
}
