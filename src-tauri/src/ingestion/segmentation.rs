// Chapter segmentation for Chinese novels

use regex::Regex;

/// A detected chapter segment
#[derive(Debug, Clone)]
pub struct ChapterSegment {
    pub index: u32,
    pub title: Option<String>,
    pub content: String,
    pub char_count: u32,
    pub start_line: usize,
    /// Parent title for hierarchical structure (e.g., volume name)
    pub parent_title: Option<String>,
}

/// Common chapter title patterns for Chinese novels
const CHAPTER_PATTERNS: &[&str] = &[
    // 第X章 标题
    r"^第[零一二三四五六七八九十百千万\d]+[章节回卷集部篇][\s　]*[^\n]*",
    // Chapter X / Chapter X: Title
    r"^[Cc]hapter\s+\d+[:\s]*[^\n]*",
    // 楔子、序章、序、引子、尾声
    r"^(楔子|序章|序言|序|引子|尾声|终章|番外|后记)[\s　]*[^\n]*",
    // 卷X / 第X卷
    r"^卷[零一二三四五六七八九十\d]+[\s　]*[^\n]*",
    // 数字. 标题 or 数字、标题
    r"^\d{1,4}[\.、．][\s　]*[^\n]+",
    // (数字) or 【数字】
    r"^[\(（\[【]\d{1,4}[\)）\]】][\s　]*[^\n]*",
];

/// Segment text into chapters using default patterns
pub fn segment_chapters(content: &str) -> Vec<ChapterSegment> {
    let combined_pattern = CHAPTER_PATTERNS.join("|");
    segment_chapters_with_pattern(content, &combined_pattern)
}

/// Segment text into chapters using custom pattern
pub fn segment_chapters_with_pattern(content: &str, pattern: &str) -> Vec<ChapterSegment> {
    let re = match Regex::new(pattern) {
        Ok(r) => r,
        Err(_) => return fallback_segmentation(content),
    };

    let lines: Vec<&str> = content.lines().collect();
    let mut chapters: Vec<ChapterSegment> = Vec::new();
    let mut current_start: Option<(usize, String)> = None;

    for (line_num, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Skip empty lines for pattern matching
        if trimmed.is_empty() {
            continue;
        }

        // Check if this line is a chapter title
        if let Some(mat) = re.find(trimmed) {
            // Save previous chapter if exists
            if let Some((start_line, title)) = current_start.take() {
                let chapter_content = collect_chapter_content(&lines, start_line, line_num);
                let char_count = count_content_chars(&chapter_content);

                if char_count > 0 {
                    chapters.push(ChapterSegment {
                        index: chapters.len() as u32 + 1,
                        title: Some(title),
                        content: chapter_content,
                        char_count,
                        start_line,
                        parent_title: None,
                    });
                }
            }

            // Start new chapter
            let title = mat.as_str().trim().to_string();
            current_start = Some((line_num, title));
        }
    }

    // Handle last chapter
    if let Some((start_line, title)) = current_start {
        let chapter_content = collect_chapter_content(&lines, start_line, lines.len());
        let char_count = count_content_chars(&chapter_content);

        if char_count > 0 {
            chapters.push(ChapterSegment {
                index: chapters.len() as u32 + 1,
                title: Some(title),
                content: chapter_content,
                char_count,
                start_line,
                parent_title: None,
            });
        }
    }

    // If no chapters detected, check for preface content
    if chapters.is_empty() {
        return fallback_segmentation(content);
    }

    // Handle content before first chapter (preface/intro)
    if !chapters.is_empty() && chapters[0].start_line > 0 {
        let preface_content = collect_chapter_content(&lines, 0, chapters[0].start_line);
        let char_count = count_content_chars(&preface_content);

        if char_count > 100 {
            // Only add if substantial
            chapters.insert(
                0,
                ChapterSegment {
                    index: 0,
                    title: Some("前言".to_string()),
                    content: preface_content,
                    char_count,
                    start_line: 0,
                    parent_title: None,
                },
            );

            // Reindex chapters
            for (i, chapter) in chapters.iter_mut().enumerate() {
                chapter.index = i as u32 + 1;
            }
        }
    }

    chapters
}

/// Fallback segmentation when no chapter markers found
fn fallback_segmentation(content: &str) -> Vec<ChapterSegment> {
    // Try to split by significant blank lines or treat as single chapter
    let paragraphs: Vec<&str> = content
        .split("\n\n")
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .collect();

    if paragraphs.is_empty() {
        return vec![];
    }

    // If content is small, treat as single chapter
    let total_chars = count_content_chars(content);
    if total_chars < 10000 {
        return vec![ChapterSegment {
            index: 1,
            title: None,
            content: content.to_string(),
            char_count: total_chars,
            start_line: 0,
            parent_title: None,
        }];
    }

    // Split into roughly equal sized chapters (around 5000 chars each)
    const TARGET_CHAPTER_SIZE: u32 = 5000;
    let mut chapters = Vec::new();
    let mut current_content = String::new();
    let mut current_chars: u32 = 0;
    let mut chapter_index: u32 = 1;

    for para in paragraphs {
        let para_chars = count_content_chars(para);
        current_content.push_str(para);
        current_content.push_str("\n\n");
        current_chars += para_chars;

        if current_chars >= TARGET_CHAPTER_SIZE {
            chapters.push(ChapterSegment {
                index: chapter_index,
                title: Some(format!("第{}部分", chapter_index)),
                content: current_content.trim().to_string(),
                char_count: current_chars,
                start_line: 0, // Approximate
                parent_title: None,
            });
            chapter_index += 1;
            current_content = String::new();
            current_chars = 0;
        }
    }

    // Handle remaining content
    if current_chars > 0 {
        chapters.push(ChapterSegment {
            index: chapter_index,
            title: Some(format!("第{}部分", chapter_index)),
            content: current_content.trim().to_string(),
            char_count: current_chars,
            start_line: 0,
            parent_title: None,
        });
    }

    chapters
}

/// Collect content from start_line to end_line
fn collect_chapter_content(lines: &[&str], start_line: usize, end_line: usize) -> String {
    lines[start_line..end_line]
        .iter()
        .copied()
        .collect::<Vec<_>>()
        .join("\n")
}

/// Count meaningful characters in content (excluding whitespace)
fn count_content_chars(content: &str) -> u32 {
    content
        .chars()
        .filter(|c| !c.is_whitespace())
        .count() as u32
}

/// Detect if content is likely a chapter title (short, matches pattern)
pub fn is_likely_chapter_title(line: &str) -> bool {
    let trimmed = line.trim();

    // Too long to be a title
    if trimmed.chars().count() > 50 {
        return false;
    }

    // Check against patterns
    for pattern in CHAPTER_PATTERNS {
        if let Ok(re) = Regex::new(pattern) {
            if re.is_match(trimmed) {
                return true;
            }
        }
    }

    false
}

/// Extract chapter number from title if possible
pub fn extract_chapter_number(title: &str) -> Option<u32> {
    // 第X章 pattern
    let re = Regex::new(r"第([零一二三四五六七八九十百千万\d]+)[章节回卷集部篇]").ok()?;
    if let Some(caps) = re.captures(title) {
        let num_str = caps.get(1)?.as_str();
        return parse_chinese_number(num_str);
    }

    // Chapter X pattern
    let re = Regex::new(r"[Cc]hapter\s+(\d+)").ok()?;
    if let Some(caps) = re.captures(title) {
        return caps.get(1)?.as_str().parse().ok();
    }

    // Pure number pattern
    let re = Regex::new(r"^(\d+)[\.、．\s]").ok()?;
    if let Some(caps) = re.captures(title) {
        return caps.get(1)?.as_str().parse().ok();
    }

    None
}

/// Parse Chinese numerals to integer
fn parse_chinese_number(s: &str) -> Option<u32> {
    // If it's already a digit string
    if let Ok(n) = s.parse::<u32>() {
        return Some(n);
    }

    // Map Chinese digits
    let digit_map: std::collections::HashMap<char, u32> = [
        ('零', 0),
        ('一', 1),
        ('二', 2),
        ('三', 3),
        ('四', 4),
        ('五', 5),
        ('六', 6),
        ('七', 7),
        ('八', 8),
        ('九', 9),
        ('十', 10),
        ('百', 100),
        ('千', 1000),
        ('万', 10000),
    ]
    .into_iter()
    .collect();

    let chars: Vec<char> = s.chars().collect();
    if chars.is_empty() {
        return None;
    }

    // Simple parsing for common cases
    let mut result: u32 = 0;
    let mut current: u32 = 0;

    for ch in chars {
        if let Some(&val) = digit_map.get(&ch) {
            if val >= 10 {
                // Multiplier
                if current == 0 {
                    current = 1;
                }
                if val == 10000 {
                    result = (result + current) * val;
                    current = 0;
                } else {
                    result += current * val;
                    current = 0;
                }
            } else {
                // Digit
                current = val;
            }
        }
    }

    result += current;

    if result > 0 {
        Some(result)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_segment_chapters_basic() {
        let content = r#"
第一章 开始

这是第一章的内容。

第二章 发展

这是第二章的内容。
"#;

        let chapters = segment_chapters(content);
        assert_eq!(chapters.len(), 2);
        assert_eq!(chapters[0].title, Some("第一章 开始".to_string()));
        assert_eq!(chapters[1].title, Some("第二章 发展".to_string()));
    }

    #[test]
    fn test_segment_with_preface() {
        let content = r#"
这是一本小说的简介，内容比较长，超过一百个字符。
这里有很多内容作为前言部分，需要被单独识别出来。
还有更多的介绍文字，确保字符数超过阈值。

第一章 正文开始

这是正文内容。
"#;

        let chapters = segment_chapters(content);
        // Should detect preface if long enough
        assert!(!chapters.is_empty());
    }

    #[test]
    fn test_is_likely_chapter_title() {
        assert!(is_likely_chapter_title("第一章 开始"));
        assert!(is_likely_chapter_title("Chapter 1: Beginning"));
        assert!(is_likely_chapter_title("楔子"));
        assert!(is_likely_chapter_title("1. 第一节"));
        assert!(!is_likely_chapter_title("这是一段普通的文字，不是章节标题"));
    }

    #[test]
    fn test_extract_chapter_number() {
        assert_eq!(extract_chapter_number("第一章 开始"), Some(1));
        assert_eq!(extract_chapter_number("第十二章 发展"), Some(12));
        assert_eq!(extract_chapter_number("第一百章 高潮"), Some(100));
        assert_eq!(extract_chapter_number("Chapter 5"), Some(5));
        assert_eq!(extract_chapter_number("3. 第三节"), Some(3));
    }

    #[test]
    fn test_parse_chinese_number() {
        assert_eq!(parse_chinese_number("一"), Some(1));
        assert_eq!(parse_chinese_number("十"), Some(10));
        assert_eq!(parse_chinese_number("十一"), Some(11));
        assert_eq!(parse_chinese_number("二十"), Some(20));
        assert_eq!(parse_chinese_number("二十三"), Some(23));
        assert_eq!(parse_chinese_number("一百"), Some(100));
        assert_eq!(parse_chinese_number("一百二十三"), Some(123));
        assert_eq!(parse_chinese_number("123"), Some(123));
    }

    #[test]
    fn test_fallback_segmentation() {
        let content = "这是一段没有章节标记的短文。\n\n另一段内容。";
        let chapters = segment_chapters(content);
        assert_eq!(chapters.len(), 1);
    }
}
