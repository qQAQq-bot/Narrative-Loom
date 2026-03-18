// EPUB file parsing module (P1-022)

use epub::doc::{EpubDoc, NavPoint};
use regex::Regex;
use std::collections::HashSet;
use std::io;
use std::path::Path;

use super::segmentation::ChapterSegment;

/// EPUB metadata extracted from the file
#[derive(Debug, Clone)]
pub struct EpubMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub language: Option<String>,
    pub description: Option<String>,
    pub cover_data: Option<Vec<u8>>,
    pub cover_mime_type: Option<String>,
}

/// Parse an EPUB file and extract chapters
pub fn parse_epub(path: &Path) -> Result<(EpubMetadata, Vec<ChapterSegment>), io::Error> {
    let mut doc = EpubDoc::new(path).map_err(|e| {
        io::Error::new(io::ErrorKind::InvalidData, format!("Failed to open EPUB: {}", e))
    })?;

    // Extract metadata
    let metadata = extract_metadata(&mut doc);

    // Extract chapters
    let chapters = extract_chapters(&mut doc)?;

    Ok((metadata, chapters))
}

/// Extract metadata from EPUB document
fn extract_metadata(doc: &mut EpubDoc<std::io::BufReader<std::fs::File>>) -> EpubMetadata {
    // mdata returns Option<&MetadataItem> or Option<Vec<String>> depending on version
    // We need to convert to Option<String>
    let title = get_metadata_string(doc, "title");
    let author = get_metadata_string(doc, "creator");
    let language = get_metadata_string(doc, "language");
    let description = get_metadata_string(doc, "description");

    EpubMetadata {
        title,
        author,
        language,
        description,
        cover_data: None,
        cover_mime_type: None,
    }
}

/// Helper to get metadata as String
fn get_metadata_string(doc: &EpubDoc<std::io::BufReader<std::fs::File>>, property: &str) -> Option<String> {
    // In epub crate 2.x, metadata is Vec<MetadataItem>
    // Each MetadataItem has `property` and `value` fields
    // Use mdata() method which returns the first matching item
    doc.mdata(property).map(|item| item.value.clone())
}

/// Extract chapters from EPUB document using TOC structure for hierarchy
#[allow(deprecated)]
fn extract_chapters(
    doc: &mut EpubDoc<std::io::BufReader<std::fs::File>>,
) -> Result<Vec<ChapterSegment>, io::Error> {
    // First try to extract chapters using the TOC (table of contents) for proper hierarchy
    // The TOC contains the hierarchical structure (volumes -> chapters)
    if !doc.toc.is_empty() {
        log::debug!("Using TOC for chapter extraction, {} entries", doc.toc.len());
        let chapters = extract_chapters_from_toc(doc)?;
        if !chapters.is_empty() {
            return Ok(chapters);
        }
    }

    // Fallback to spine-based extraction if TOC is empty or didn't work
    log::debug!("Falling back to spine-based chapter extraction");
    extract_chapters_from_spine(doc)
}

/// Extract chapters from TOC structure, preserving volume/chapter hierarchy
fn extract_chapters_from_toc(
    doc: &mut EpubDoc<std::io::BufReader<std::fs::File>>,
) -> Result<Vec<ChapterSegment>, io::Error> {
    let mut chapters = Vec::new();
    let mut index = 0u32;
    let mut processed_paths: HashSet<String> = HashSet::new();

    // Clone TOC and spine to avoid borrow issues
    let toc = doc.toc.clone();
    let spine = doc.spine.clone();

    // Build a map of spine order for finding intermediate pages
    let spine_order: Vec<String> = spine.iter().map(|s| s.idref.clone()).collect();

    log::info!("Extracting chapters from TOC with {} top-level entries", toc.len());

    for nav_point in &toc {
        // Process this nav point and its children
        process_nav_point(
            doc,
            nav_point,
            None, // No parent for top-level items
            &mut chapters,
            &mut index,
            &mut processed_paths,
            &spine_order,
        )?;
    }

    log::info!("Extracted {} chapters from TOC", chapters.len());

    Ok(chapters)
}

/// Recursively process a NavPoint and its children
fn process_nav_point(
    doc: &mut EpubDoc<std::io::BufReader<std::fs::File>>,
    nav_point: &NavPoint,
    parent_title: Option<&str>,
    chapters: &mut Vec<ChapterSegment>,
    index: &mut u32,
    processed_paths: &mut HashSet<String>,
    spine_order: &[String],
) -> Result<(), io::Error> {
    let content_path = nav_point.content.to_string_lossy().to_string();
    let label = nav_point.label.trim();

    log::debug!(
        "Processing NavPoint: label='{}', path='{}', children={}, parent={:?}",
        label,
        content_path,
        nav_point.children.len(),
        parent_title
    );

    // Check if this nav point has children (it's a volume/section header)
    if !nav_point.children.is_empty() {
        // This is a volume/section with sub-chapters
        // Use the label as parent_title for children
        let volume_title = label.to_string();

        // Some EPUBs have content for the volume header itself
        // We need to collect content from the volume's page AND any intermediate pages
        // that come before the first child chapter
        if !content_path.is_empty() && !processed_paths.contains(&content_path) {
            // Find the first child's path to know where to stop collecting intermediate pages
            let first_child_path = nav_point.children.first()
                .map(|c| c.content.to_string_lossy().to_string());

            // Collect content from volume page and any intermediate pages
            let (combined_content, combined_char_count) = collect_content_until_next_toc_entry(
                doc,
                &content_path,
                first_child_path.as_deref(),
                processed_paths,
                spine_order,
            )?;

            if combined_char_count > 20 {
                log::debug!(
                    "Adding volume chapter: '{}' with {} chars (combined from multiple pages)",
                    volume_title,
                    combined_char_count
                );
                chapters.push(ChapterSegment {
                    index: *index,
                    title: Some(volume_title.clone()),
                    content: combined_content,
                    char_count: combined_char_count,
                    start_line: 0,
                    parent_title: None, // Volume header has no parent
                });
                *index += 1;
            } else {
                log::debug!(
                    "Skipping volume '{}': too short ({} chars)",
                    volume_title,
                    combined_char_count
                );
            }
        }

        // Process children with this nav point's label as parent
        for child in &nav_point.children {
            process_nav_point(doc, child, Some(&volume_title), chapters, index, processed_paths, spine_order)?;
        }
    } else {
        // This is a leaf node (actual chapter)
        // Skip if we've already processed this path
        if processed_paths.contains(&content_path) {
            log::debug!("Skipping '{}': path already processed", label);
            return Ok(());
        }

        if let Some(chapter) = extract_chapter_content(doc, &content_path, Some(label), parent_title)? {
            // Lower threshold: include chapters with at least 10 characters
            // This ensures short sections like "赞誉", "序", "引言" are not skipped
            // The TOC explicitly lists them, so they should be included
            if chapter.char_count >= 10 {
                log::debug!(
                    "Adding chapter: '{}' with {} chars, parent={:?}",
                    label,
                    chapter.char_count,
                    parent_title
                );
                processed_paths.insert(content_path);
                chapters.push(ChapterSegment {
                    index: *index,
                    title: Some(label.to_string()),
                    content: chapter.content,
                    char_count: chapter.char_count,
                    start_line: 0,
                    parent_title: parent_title.map(|s| s.to_string()),
                });
                *index += 1;
            } else {
                log::debug!(
                    "Skipping chapter '{}': too short ({} chars)",
                    label,
                    chapter.char_count
                );
            }
        } else {
            log::warn!(
                "No content found for chapter '{}' at path '{}'",
                label,
                content_path
            );
        }
    }

    Ok(())
}

/// Collect content from a starting page until the next TOC entry
/// This handles cases where volume content spans multiple pages not all listed in TOC
fn collect_content_until_next_toc_entry(
    doc: &mut EpubDoc<std::io::BufReader<std::fs::File>>,
    start_path: &str,
    next_toc_path: Option<&str>,
    processed_paths: &mut HashSet<String>,
    spine_order: &[String],
) -> Result<(String, u32), io::Error> {
    let mut combined_content = String::new();
    let mut total_char_count = 0u32;

    // Get the starting path without fragment
    let start_path_clean = start_path.split('#').next().unwrap_or(start_path);
    let next_path_clean = next_toc_path.map(|p| p.split('#').next().unwrap_or(p));

    // Find the resource ID for the starting path
    let start_resource_id = find_resource_id_by_path(doc, start_path_clean);

    if start_resource_id.is_none() {
        log::debug!("Could not find resource for path: {}", start_path);
        return Ok((combined_content, total_char_count));
    }

    let start_id = start_resource_id.unwrap();

    // Find position in spine
    let start_pos = spine_order.iter().position(|id| id == &start_id);
    if start_pos.is_none() {
        // Not in spine, just extract this one page
        if let Some(chapter) = extract_chapter_content(doc, start_path, None, None)? {
            processed_paths.insert(start_path.to_string());
            return Ok((chapter.content, chapter.char_count));
        }
        return Ok((combined_content, total_char_count));
    }

    let start_idx = start_pos.unwrap();

    // Find the stop position (next TOC entry's spine position)
    let stop_idx = if let Some(next_path) = next_path_clean {
        if let Some(next_id) = find_resource_id_by_path(doc, next_path) {
            spine_order.iter().position(|id| id == &next_id).unwrap_or(spine_order.len())
        } else {
            spine_order.len()
        }
    } else {
        spine_order.len()
    };

    log::debug!(
        "Collecting content from spine index {} to {} (exclusive)",
        start_idx, stop_idx
    );

    // Collect content from start_idx to stop_idx (exclusive)
    for idx in start_idx..stop_idx {
        let resource_id = &spine_order[idx];

        // Get the resource path for tracking
        if let Some(resource) = doc.resources.get(resource_id) {
            let resource_path = resource.path.to_string_lossy().to_string();

            // Skip if already processed
            if processed_paths.contains(&resource_path) {
                continue;
            }

            // Get the content
            if let Some((content, _mime)) = doc.get_resource_str(resource_id) {
                let text = html_to_text(&content);
                let trimmed = text.trim();

                if !trimmed.is_empty() {
                    if !combined_content.is_empty() {
                        combined_content.push_str("\n\n");
                    }
                    combined_content.push_str(trimmed);
                    total_char_count += trimmed.chars().count() as u32;

                    log::debug!(
                        "  Added content from spine[{}] = '{}', {} chars",
                        idx, resource_id, trimmed.chars().count()
                    );
                }

                processed_paths.insert(resource_path);
            }
        }
    }

    Ok((combined_content, total_char_count))
}

/// Helper struct for chapter content extraction
struct ChapterContent {
    content: String,
    char_count: u32,
}

/// Extract content for a specific path from the EPUB
fn extract_chapter_content(
    doc: &mut EpubDoc<std::io::BufReader<std::fs::File>>,
    content_path: &str,
    _toc_label: Option<&str>,
    _parent_title: Option<&str>,
) -> Result<Option<ChapterContent>, io::Error> {
    // Remove fragment identifier if present (e.g., "chapter1.xhtml#section1" -> "chapter1.xhtml")
    let path_without_fragment = content_path.split('#').next().unwrap_or(content_path);

    // Try to find the resource by path
    // First, try direct path match
    let resource_id = find_resource_id_by_path(doc, path_without_fragment);

    if let Some(id) = resource_id {
        if let Some((content, _mime)) = doc.get_resource_str(&id) {
            let text = html_to_text(&content);
            let trimmed = text.trim();

            if trimmed.is_empty() {
                log::debug!("Content for path '{}' is empty after HTML stripping", content_path);
                return Ok(None);
            }

            // Use TOC label as title, content as-is
            let char_count = trimmed.chars().count() as u32;

            return Ok(Some(ChapterContent {
                content: trimmed.to_string(),
                char_count,
            }));
        } else {
            log::debug!("Failed to get resource content for id '{}'", id);
        }
    } else {
        log::debug!("No resource found for path '{}'", content_path);
    }

    Ok(None)
}

/// Find resource ID by path (handles various path formats)
fn find_resource_id_by_path(
    doc: &EpubDoc<std::io::BufReader<std::fs::File>>,
    path: &str,
) -> Option<String> {
    let normalized_path = normalize_path(path);

    for (id, resource) in &doc.resources {
        let resource_path = resource.path.to_string_lossy().to_lowercase();
        let resource_normalized = normalize_path(&resource_path);

        // Try exact match
        if resource_normalized == normalized_path {
            return Some(id.clone());
        }

        // Try matching just the filename
        let path_filename = normalized_path.rsplit('/').next().unwrap_or(&normalized_path);
        let resource_filename = resource_normalized.rsplit('/').next().unwrap_or(&resource_normalized);

        if path_filename == resource_filename {
            return Some(id.clone());
        }

        // Try if resource path ends with our path
        if resource_normalized.ends_with(&normalized_path) {
            return Some(id.clone());
        }
    }

    None
}

/// Extract chapters from spine (fallback method)
#[allow(deprecated)]
fn extract_chapters_from_spine(
    doc: &mut EpubDoc<std::io::BufReader<std::fs::File>>,
) -> Result<Vec<ChapterSegment>, io::Error> {
    let mut chapters = Vec::new();
    let mut index = 0u32;

    // Get the number of chapters in spine
    let num_chapters = doc.get_num_pages();

    for chapter_num in 0..num_chapters {
        if doc.set_current_page(chapter_num) {
            if let Some((content, _mime)) = doc.get_current_str() {
                let text = html_to_text(&content);
                let trimmed = text.trim();

                // Skip empty chapters or very short content (likely navigation)
                // Lower threshold to 10 chars to include short sections
                if trimmed.is_empty() || trimmed.chars().count() < 10 {
                    continue;
                }

                // Try to extract chapter title from content
                let title = extract_chapter_title(&content, &text);

                // Get content without the title if it was extracted
                let content_text = if let Some(ref t) = title {
                    remove_title_from_content(&text, t)
                } else {
                    text.clone()
                };

                let char_count = content_text.chars().count() as u32;

                chapters.push(ChapterSegment {
                    index,
                    title,
                    content: content_text,
                    char_count,
                    start_line: 0,
                    parent_title: None, // Spine-based extraction doesn't have hierarchy
                });

                index += 1;
            }
        }
    }

    // If no chapters found, try alternative approach
    if chapters.is_empty() {
        chapters = extract_chapters_alternative(doc)?;
    }

    Ok(chapters)
}

/// Alternative chapter extraction by iterating through spine
fn extract_chapters_alternative(
    doc: &mut EpubDoc<std::io::BufReader<std::fs::File>>,
) -> Result<Vec<ChapterSegment>, io::Error> {
    let mut chapters = Vec::new();
    let mut index = 0u32;

    // Clone spine to avoid borrow issues
    let spine = doc.spine.clone();

    for spine_item in spine {
        // Get the resource ID (idref in spine)
        let resource_id = &spine_item.idref;

        if let Some((content, _mime)) = doc.get_resource_str(resource_id) {
            let text = html_to_text(&content);
            let trimmed = text.trim();

            // Skip empty or very short content
            // Lower threshold to 10 chars to include short sections
            if trimmed.is_empty() || trimmed.chars().count() < 10 {
                continue;
            }

            let title = extract_chapter_title(&content, &text);
            let content_text = if let Some(ref t) = title {
                remove_title_from_content(&text, t)
            } else {
                text.clone()
            };

            let char_count = content_text.chars().count() as u32;

            chapters.push(ChapterSegment {
                index,
                title,
                content: content_text,
                char_count,
                start_line: 0,
                parent_title: None,
            });

            index += 1;
        }
    }

    Ok(chapters)
}

/// Convert HTML content to plain text
fn html_to_text(html: &str) -> String {
    // Use a more robust approach: process character by character to handle complex nested tags
    let mut result = String::new();
    let mut chars = html.chars().peekable();
    let mut in_tag = false;
    let mut tag_content = String::new();
    let mut skip_stack: Vec<String> = Vec::new(); // Stack of tags to skip (handles nesting)

    while let Some(c) = chars.next() {
        // If we're skipping content within certain tags
        if !skip_stack.is_empty() {
            if c == '<' {
                // Collect the tag
                let mut tag_buf = String::from("<");
                while let Some(&nc) = chars.peek() {
                    tag_buf.push(nc);
                    chars.next();
                    if nc == '>' {
                        break;
                    }
                }

                let tag_lower = tag_buf.to_lowercase();
                let current_skip_tag = skip_stack.last().unwrap().clone();

                // Check if this is a closing tag for our current skip tag
                let closing_pattern = format!("</{}", current_skip_tag);
                if tag_lower.starts_with(&closing_pattern) {
                    skip_stack.pop();
                }
                // Check if this is a nested opening tag of the same type (e.g., nested svg)
                else {
                    let opening_pattern = format!("<{}", current_skip_tag);
                    if tag_lower.starts_with(&opening_pattern) && !tag_lower.contains("/>") && !tag_lower.ends_with("/>") {
                        skip_stack.push(current_skip_tag);
                    }
                }
            }
            continue;
        }

        if c == '<' {
            in_tag = true;
            tag_content.clear();
            tag_content.push(c);
        } else if c == '>' && in_tag {
            tag_content.push(c);
            in_tag = false;

            // Process the tag
            let tag_lower = tag_content.to_lowercase();

            // Check for tags whose content should be skipped entirely
            // Self-closing tags (ending with />) should not trigger skipping
            let is_self_closing = tag_lower.ends_with("/>");

            if !is_self_closing {
                if tag_lower.starts_with("<script") {
                    skip_stack.push("script".to_string());
                } else if tag_lower.starts_with("<style") {
                    skip_stack.push("style".to_string());
                } else if tag_lower.starts_with("<svg") {
                    skip_stack.push("svg".to_string());
                } else if tag_lower.starts_with("<title") {
                    skip_stack.push("title".to_string());
                } else if tag_lower.starts_with("<head") {
                    skip_stack.push("head".to_string());
                }
            }

            // Block elements that should add newlines
            if tag_lower.starts_with("<p")
                || tag_lower.starts_with("</p")
                || tag_lower.starts_with("<div")
                || tag_lower.starts_with("</div")
                || tag_lower.starts_with("<br")
                || tag_lower.starts_with("<h1")
                || tag_lower.starts_with("</h1")
                || tag_lower.starts_with("<h2")
                || tag_lower.starts_with("</h2")
                || tag_lower.starts_with("<h3")
                || tag_lower.starts_with("</h3")
                || tag_lower.starts_with("<h4")
                || tag_lower.starts_with("</h4")
                || tag_lower.starts_with("<h5")
                || tag_lower.starts_with("</h5")
                || tag_lower.starts_with("<h6")
                || tag_lower.starts_with("</h6")
                || tag_lower.starts_with("<li")
                || tag_lower.starts_with("</li")
                || tag_lower.starts_with("<tr")
                || tag_lower.starts_with("</tr")
                || tag_lower.starts_with("<section")
                || tag_lower.starts_with("</section")
                || tag_lower.starts_with("<article")
                || tag_lower.starts_with("</article")
                || tag_lower.starts_with("<figure")
                || tag_lower.starts_with("</figure")
            {
                result.push('\n');
            }
            // For other tags (including img, image, etc.), just skip them
            // The content after them will still be captured

            tag_content.clear();
        } else if in_tag {
            tag_content.push(c);
        } else {
            // Regular text content
            result.push(c);
        }
    }

    // Decode HTML entities
    let mut text = decode_html_entities(&result);

    // Normalize whitespace
    let multi_space_re = Regex::new(r"[ \t]+").unwrap();
    text = multi_space_re.replace_all(&text, " ").to_string();

    // Normalize newlines
    let multi_newline_re = Regex::new(r"\n{3,}").unwrap();
    text = multi_newline_re.replace_all(&text, "\n\n").to_string();

    // Trim each line
    text.lines()
        .map(|line| line.trim())
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

/// Decode common HTML entities
fn decode_html_entities(text: &str) -> String {
    text.replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#39;", "'")
        .replace("&mdash;", "\u{2014}")
        .replace("&ndash;", "\u{2013}")
        .replace("&hellip;", "\u{2026}")
        .replace("&lsquo;", "\u{2018}")
        .replace("&rsquo;", "\u{2019}")
        .replace("&ldquo;", "\u{201C}")
        .replace("&rdquo;", "\u{201D}")
        .replace("&#8212;", "\u{2014}")
        .replace("&#8211;", "\u{2013}")
        .replace("&#8230;", "\u{2026}")
}

/// Try to extract chapter title from HTML content
fn extract_chapter_title(html: &str, plain_text: &str) -> Option<String> {
    // First, try to find a chapter-style title pattern in the plain text
    // This preserves chapter numbers like "第一章 标题" or "Chapter 1: Title"
    let chapter_pattern = regex::Regex::new(
        r"(?m)^[\s\u{3000}]*(第[零一二三四五六七八九十百千万\d]+[章节回卷集部篇][\s\u{3000}]*[^\n]*|[Cc]hapter\s+\d+[:\s]*[^\n]*|楔子|序章|序言|序|引子|尾声|终章|番外|后记)"
    ).ok();

    if let Some(re) = chapter_pattern {
        if let Some(mat) = re.find(plain_text) {
            let title = mat.as_str().trim();
            if !title.is_empty() && title.chars().count() < 100 {
                return Some(title.to_string());
            }
        }
    }

    // Fallback: Try to find title in heading tags
    let h1_re = Regex::new(r"(?is)<h1[^>]*>(.*?)</h1>").unwrap();
    if let Some(caps) = h1_re.captures(html) {
        let title = html_to_text(&caps[1]);
        if !title.is_empty() && title.chars().count() < 100 {
            return Some(title.trim().to_string());
        }
    }

    let h2_re = Regex::new(r"(?is)<h2[^>]*>(.*?)</h2>").unwrap();
    if let Some(caps) = h2_re.captures(html) {
        let title = html_to_text(&caps[1]);
        if !title.is_empty() && title.chars().count() < 100 {
            return Some(title.trim().to_string());
        }
    }

    // Try to find title with class="chapter-title" or similar
    let class_title_re = Regex::new(r#"(?is)<[^>]+class="[^"]*(?:chapter|title)[^"]*"[^>]*>(.*?)</[^>]+>"#).unwrap();
    if let Some(caps) = class_title_re.captures(html) {
        let title = html_to_text(&caps[1]);
        if !title.is_empty() && title.chars().count() < 100 {
            return Some(title.trim().to_string());
        }
    }

    None
}

/// Remove the title from the beginning of content
fn remove_title_from_content(text: &str, title: &str) -> String {
    let trimmed = text.trim();
    if trimmed.starts_with(title) {
        trimmed[title.len()..].trim_start().to_string()
    } else {
        trimmed.to_string()
    }
}

/// Extract cover image from EPUB
#[allow(deprecated)]
pub fn extract_cover(path: &Path) -> Result<Option<(Vec<u8>, String)>, io::Error> {
    let mut doc = EpubDoc::new(path).map_err(|e| {
        io::Error::new(io::ErrorKind::InvalidData, format!("Failed to open EPUB: {}", e))
    })?;

    // Log available resources for debugging
    log::debug!("EPUB resources count: {}", doc.resources.len());
    for (id, resource) in &doc.resources {
        log::debug!("  Resource: {} -> {:?} ({})", id, resource.path, resource.mime);
    }

    // Try to get cover image using the built-in method
    // get_cover() returns Option<(Vec<u8>, String)> where String is mime type
    if let Some((cover_data, mime_type)) = doc.get_cover() {
        log::info!("Found cover using get_cover() method, size: {} bytes, mime: {}", cover_data.len(), mime_type);
        return Ok(Some((cover_data, mime_type)));
    }
    log::debug!("get_cover() returned None");

    // Try to find cover by resource ID (from metadata)
    if let Some(cover_id) = doc.get_cover_id() {
        log::debug!("Found cover_id from metadata: {}", cover_id);
        if let Some((cover_data, mime_type)) = doc.get_resource(&cover_id) {
            log::info!("Found cover using get_cover_id(): {}, size: {} bytes", cover_id, cover_data.len());
            return Ok(Some((cover_data, mime_type)));
        } else {
            log::warn!("cover_id '{}' exists in metadata but resource not found", cover_id);
        }
    } else {
        log::debug!("No cover_id in metadata");
    }

    // Try to find cover in resources by name pattern
    // resources is HashMap<String, ResourceItem> where ResourceItem has path, mime, properties
    let resource_ids: Vec<String> = doc.resources.keys().cloned().collect();

    // Strategy 1: Match by resource ID containing "cover"
    for resource_id in &resource_ids {
        if let Some(resource) = doc.resources.get(resource_id) {
            let id_lower = resource_id.to_lowercase();

            if (id_lower.contains("cover") || id_lower.contains("frontcover"))
                && (resource.mime.contains("image") || resource.mime.contains("jpeg") || resource.mime.contains("png"))
            {
                if let Some((data, mime_type)) = doc.get_resource(resource_id) {
                    log::info!("Found cover by resource ID pattern: {}, size: {} bytes", resource_id, data.len());
                    return Ok(Some((data, mime_type)));
                }
            }
        }
    }

    // Strategy 2: Check properties for cover-image (EPUB3)
    for resource_id in &resource_ids {
        if let Some(resource) = doc.resources.get(resource_id) {
            if let Some(ref props) = resource.properties {
                if props.contains("cover-image") {
                    if let Some((data, mime_type)) = doc.get_resource(resource_id) {
                        log::info!("Found cover by properties (cover-image): {}, size: {} bytes", resource_id, data.len());
                        return Ok(Some((data, mime_type)));
                    }
                }
            }
        }
    }

    // Strategy 3: Try to extract image from first page (some EPUBs have cover as first page content)
    if doc.set_current_page(0) {
        if let Some((content, _)) = doc.get_current_str() {
            log::debug!("Attempting to extract cover from first page HTML, content length: {}", content.len());

            // Extract all image sources from HTML using multiple patterns
            let img_sources = extract_image_sources_from_html(&content);
            log::debug!("Found {} image sources in first page", img_sources.len());

            for img_src in &img_sources {
                log::debug!("Trying to match image source: {}", img_src);

                // Normalize the source path for matching
                let img_src_normalized = normalize_path(img_src);

                for resource_id in &resource_ids {
                    if let Some(resource) = doc.resources.get(resource_id) {
                        if !resource.mime.contains("image") {
                            continue;
                        }

                        let resource_path_str = resource.path.to_string_lossy().to_lowercase();
                        let resource_path_normalized = normalize_path(&resource_path_str);

                        // Try multiple matching strategies
                        let matched =
                            // Exact path match
                            resource_path_normalized == img_src_normalized ||
                            // Resource path ends with image source
                            resource_path_normalized.ends_with(&img_src_normalized) ||
                            // Image source ends with resource filename
                            img_src_normalized.ends_with(&extract_filename(&resource_path_normalized)) ||
                            // Resource filename matches image source filename
                            extract_filename(&resource_path_normalized) == extract_filename(&img_src_normalized) ||
                            // Partial path match (for relative paths)
                            resource_path_str.contains(&img_src.to_lowercase()) ||
                            img_src.to_lowercase().contains(&extract_filename(&resource_path_str));

                        if matched {
                            if let Some((data, mime_type)) = doc.get_resource(resource_id) {
                                log::info!("Found cover from first page: {} -> {}, size: {} bytes", img_src, resource_id, data.len());
                                return Ok(Some((data, mime_type)));
                            }
                        }
                    }
                }
            }
        }
    }

    // Strategy 4: Find any image resource that looks like a cover by path
    for resource_id in &resource_ids {
        if let Some(resource) = doc.resources.get(resource_id) {
            let path_lower = resource.path.to_string_lossy().to_lowercase();
            if resource.mime.contains("image")
                && (path_lower.contains("cover") || path_lower.contains("title") || path_lower.contains("front"))
            {
                if let Some((data, mime_type)) = doc.get_resource(resource_id) {
                    log::info!("Found cover by path pattern: {}, size: {} bytes", path_lower, data.len());
                    return Ok(Some((data, mime_type)));
                }
            }
        }
    }

    // Strategy 5: If first page has exactly one image, use it as cover (common pattern)
    if doc.set_current_page(0) {
        if let Some((content, _)) = doc.get_current_str() {
            let img_sources = extract_image_sources_from_html(&content);
            if img_sources.len() == 1 {
                let img_src = &img_sources[0];
                let img_src_normalized = normalize_path(img_src);

                for resource_id in &resource_ids {
                    if let Some(resource) = doc.resources.get(resource_id) {
                        if !resource.mime.contains("image") {
                            continue;
                        }
                        let resource_path_normalized = normalize_path(&resource.path.to_string_lossy().to_lowercase());
                        if resource_path_normalized.ends_with(&extract_filename(&img_src_normalized)) ||
                           extract_filename(&resource_path_normalized) == extract_filename(&img_src_normalized) {
                            if let Some((data, mime_type)) = doc.get_resource(resource_id) {
                                log::info!("Found cover as single image on first page: {}, size: {} bytes", resource_id, data.len());
                                return Ok(Some((data, mime_type)));
                            }
                        }
                    }
                }
            }
        }
    }

    // Strategy 6: Find the largest image as fallback (often the cover is the largest)
    let mut largest_image: Option<(String, usize)> = None;
    for resource_id in &resource_ids {
        if let Some(resource) = doc.resources.get(resource_id) {
            if resource.mime.contains("image") {
                if let Some((data, _)) = doc.get_resource(resource_id) {
                    let size = data.len();
                    // Only consider images larger than 10KB (to skip icons/thumbnails)
                    if size > 10 * 1024 {
                        if largest_image.is_none() || size > largest_image.as_ref().unwrap().1 {
                            largest_image = Some((resource_id.clone(), size));
                        }
                    }
                }
            }
        }
    }

    if let Some((resource_id, size)) = largest_image {
        if let Some((data, mime_type)) = doc.get_resource(&resource_id) {
            log::info!("Found cover as largest image: {}, size: {} bytes", resource_id, size);
            return Ok(Some((data, mime_type)));
        }
    }

    log::warn!("No cover image found in EPUB after trying all strategies");
    Ok(None)
}

/// Extract all image sources from HTML content
fn extract_image_sources_from_html(html: &str) -> Vec<String> {
    let mut sources = Vec::new();

    // Pattern 1: <img src="...">
    let img_src_re = Regex::new(r#"(?i)<img[^>]+src\s*=\s*["']([^"']+)["']"#).unwrap();
    for caps in img_src_re.captures_iter(html) {
        sources.push(caps[1].to_string());
    }

    // Pattern 2: <img ... src=...> (without quotes, less common)
    let img_src_noquote_re = Regex::new(r#"(?i)<img[^>]+src\s*=\s*([^\s>"']+)"#).unwrap();
    for caps in img_src_noquote_re.captures_iter(html) {
        let src = caps[1].to_string();
        if !sources.contains(&src) {
            sources.push(src);
        }
    }

    // Pattern 3: xlink:href="..." (for SVG images)
    let xlink_re = Regex::new(r#"(?i)xlink:href\s*=\s*["']([^"']+)["']"#).unwrap();
    for caps in xlink_re.captures_iter(html) {
        let src = caps[1].to_string();
        if !sources.contains(&src) {
            sources.push(src);
        }
    }

    // Pattern 4: href="..." in <image> tags (SVG)
    let image_href_re = Regex::new(r#"(?i)<image[^>]+href\s*=\s*["']([^"']+)["']"#).unwrap();
    for caps in image_href_re.captures_iter(html) {
        let src = caps[1].to_string();
        if !sources.contains(&src) {
            sources.push(src);
        }
    }

    // Pattern 5: background-image: url(...)
    let bg_image_re = Regex::new(r#"(?i)background-image\s*:\s*url\s*\(\s*["']?([^"')]+)["']?\s*\)"#).unwrap();
    for caps in bg_image_re.captures_iter(html) {
        let src = caps[1].to_string();
        if !sources.contains(&src) {
            sources.push(src);
        }
    }

    sources
}

/// Normalize path by removing leading ../ and ./ and converting to lowercase
fn normalize_path(path: &str) -> String {
    let mut result = path.to_lowercase();

    // Remove URL encoding if present
    if result.contains('%') {
        if let Ok(decoded) = urlencoding::decode(&result) {
            result = decoded.to_string();
        }
    }

    // Remove leading path traversals
    while result.starts_with("../") {
        result = result[3..].to_string();
    }
    while result.starts_with("./") {
        result = result[2..].to_string();
    }

    // Remove leading slash
    if result.starts_with('/') {
        result = result[1..].to_string();
    }

    result
}

/// Extract filename from path
fn extract_filename(path: &str) -> String {
    path.rsplit('/').next().unwrap_or(path).to_string()
}

/// Detect image MIME type from magic bytes
#[allow(dead_code)]
fn detect_image_mime(data: &[u8]) -> String {
    if data.len() >= 8 {
        // PNG signature
        if data[0..8] == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A] {
            return "image/png".to_string();
        }
        // JPEG signature
        if data[0..2] == [0xFF, 0xD8] {
            return "image/jpeg".to_string();
        }
        // GIF signature
        if data[0..6] == [0x47, 0x49, 0x46, 0x38, 0x39, 0x61]
            || data[0..6] == [0x47, 0x49, 0x46, 0x38, 0x37, 0x61]
        {
            return "image/gif".to_string();
        }
        // WebP signature
        if data.len() >= 12 && data[0..4] == [0x52, 0x49, 0x46, 0x46] && data[8..12] == [0x57, 0x45, 0x42, 0x50] {
            return "image/webp".to_string();
        }
    }
    "application/octet-stream".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_html_entities() {
        let text = "&lt;hello&gt; &amp; &quot;world&quot;";
        let decoded = decode_html_entities(text);
        assert_eq!(decoded, "<hello> & \"world\"");
    }

    #[test]
    fn test_detect_image_mime() {
        let png_header = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        assert_eq!(detect_image_mime(&png_header), "image/png");

        let jpeg_header = [0xFF, 0xD8, 0xFF, 0xE0];
        assert_eq!(detect_image_mime(&jpeg_header), "image/jpeg");
    }
}
