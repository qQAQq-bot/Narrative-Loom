// File parsing and encoding detection

use std::fs::File;
use std::io::{self, Read};
use std::path::Path;


/// Detect file encoding by reading BOM and analyzing content
pub fn detect_encoding(path: &Path) -> Result<String, io::Error> {
    let mut file = File::open(path)?;
    let mut buffer = [0u8; 4096];
    let bytes_read = file.read(&mut buffer)?;
    let sample = &buffer[..bytes_read];

    // Check BOM (Byte Order Mark)
    if sample.len() >= 3 && sample[0] == 0xEF && sample[1] == 0xBB && sample[2] == 0xBF {
        return Ok("utf-8".to_string());
    }
    if sample.len() >= 2 && sample[0] == 0xFF && sample[1] == 0xFE {
        return Ok("utf-16le".to_string());
    }
    if sample.len() >= 2 && sample[0] == 0xFE && sample[1] == 0xFF {
        return Ok("utf-16be".to_string());
    }

    // Try to decode as UTF-8
    if is_valid_utf8(sample) {
        return Ok("utf-8".to_string());
    }

    // Heuristic detection for Chinese encodings
    if looks_like_gbk(sample) {
        return Ok("gbk".to_string());
    }

    // Default to GB18030 (superset of GBK)
    Ok("gb18030".to_string())
}

/// Check if bytes are valid UTF-8
fn is_valid_utf8(bytes: &[u8]) -> bool {
    std::str::from_utf8(bytes).is_ok()
}

/// Heuristic check for GBK encoding
/// GBK uses 2 bytes for Chinese characters: first byte 0x81-0xFE, second byte 0x40-0xFE
fn looks_like_gbk(bytes: &[u8]) -> bool {
    let mut i = 0;
    let mut gbk_chars = 0;
    let mut total_chars = 0;

    while i < bytes.len() {
        let b = bytes[i];

        if b <= 0x7F {
            // ASCII
            i += 1;
            total_chars += 1;
        } else if b >= 0x81 && b <= 0xFE && i + 1 < bytes.len() {
            let b2 = bytes[i + 1];
            if (b2 >= 0x40 && b2 <= 0x7E) || (b2 >= 0x80 && b2 <= 0xFE) {
                // Valid GBK sequence
                gbk_chars += 1;
                i += 2;
                total_chars += 1;
            } else {
                i += 1;
            }
        } else {
            i += 1;
        }

        // Sample enough characters
        if total_chars > 100 {
            break;
        }
    }

    // If we found GBK sequences and no invalid sequences, likely GBK
    gbk_chars > 10
}

/// Read file content with specified encoding
pub fn read_file_with_encoding(path: &Path, encoding: &str) -> Result<String, io::Error> {
    let mut file = File::open(path)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;

    // Skip BOM if present
    let bytes = skip_bom(&bytes, encoding);

    decode_bytes(bytes, encoding)
}

/// Skip BOM bytes based on encoding
fn skip_bom<'a>(bytes: &'a [u8], encoding: &str) -> &'a [u8] {
    match encoding.to_lowercase().as_str() {
        "utf-8" => {
            if bytes.len() >= 3 && bytes[0] == 0xEF && bytes[1] == 0xBB && bytes[2] == 0xBF {
                return &bytes[3..];
            }
        }
        "utf-16le" => {
            if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xFE {
                return &bytes[2..];
            }
        }
        "utf-16be" => {
            if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
                return &bytes[2..];
            }
        }
        _ => {}
    }
    bytes
}

/// Decode bytes to string using specified encoding
fn decode_bytes(bytes: &[u8], encoding: &str) -> Result<String, io::Error> {
    match encoding.to_lowercase().as_str() {
        "utf-8" => {
            String::from_utf8(bytes.to_vec()).map_err(|e| {
                io::Error::new(io::ErrorKind::InvalidData, format!("UTF-8 decode error: {}", e))
            })
        }
        "utf-16le" => decode_utf16le(bytes),
        "utf-16be" => decode_utf16be(bytes),
        "gbk" | "gb2312" => decode_gbk(bytes),
        "gb18030" => decode_gb18030(bytes),
        "big5" => decode_big5(bytes),
        _ => {
            // Try UTF-8 as fallback
            String::from_utf8(bytes.to_vec()).map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, format!("Unknown encoding: {}", encoding))
            })
        }
    }
}

fn decode_utf16le(bytes: &[u8]) -> Result<String, io::Error> {
    if bytes.len() % 2 != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid UTF-16LE: odd number of bytes",
        ));
    }

    let u16_values: Vec<u16> = bytes
        .chunks(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();

    String::from_utf16(&u16_values).map_err(|e| {
        io::Error::new(io::ErrorKind::InvalidData, format!("UTF-16LE decode error: {}", e))
    })
}

fn decode_utf16be(bytes: &[u8]) -> Result<String, io::Error> {
    if bytes.len() % 2 != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid UTF-16BE: odd number of bytes",
        ));
    }

    let u16_values: Vec<u16> = bytes
        .chunks(2)
        .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
        .collect();

    String::from_utf16(&u16_values).map_err(|e| {
        io::Error::new(io::ErrorKind::InvalidData, format!("UTF-16BE decode error: {}", e))
    })
}

/// Decode GBK/GB2312 encoded bytes
/// This is a simplified implementation - for production, use encoding_rs crate
fn decode_gbk(bytes: &[u8]) -> Result<String, io::Error> {
    // For now, we'll use a simple approach that handles common cases
    // In production, you should use the `encoding_rs` crate
    decode_with_fallback(bytes, "GBK")
}

fn decode_gb18030(bytes: &[u8]) -> Result<String, io::Error> {
    decode_with_fallback(bytes, "GB18030")
}

fn decode_big5(bytes: &[u8]) -> Result<String, io::Error> {
    decode_with_fallback(bytes, "BIG5")
}

/// Fallback decoder that uses lossy conversion
fn decode_with_fallback(bytes: &[u8], _encoding_name: &str) -> Result<String, io::Error> {
    // Try encoding_rs if available (would need to add as dependency)
    // For now, provide a basic implementation that handles ASCII + marks unknown as replacement char

    let mut result = String::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        let b = bytes[i];

        if b <= 0x7F {
            // ASCII
            result.push(b as char);
            i += 1;
        } else if b >= 0x81 && b <= 0xFE && i + 1 < bytes.len() {
            // Double-byte character
            let b2 = bytes[i + 1];

            // Try to map common GBK characters
            // This is a simplified version - real implementation needs full mapping table
            if let Some(ch) = gbk_to_unicode(b, b2) {
                result.push(ch);
            } else {
                result.push('\u{FFFD}'); // Replacement character
            }
            i += 2;
        } else {
            result.push('\u{FFFD}');
            i += 1;
        }
    }

    Ok(result)
}

/// Map GBK bytes to Unicode (simplified - only common punctuation)
fn gbk_to_unicode(high: u8, low: u8) -> Option<char> {
    // Common Chinese punctuation
    match (high, low) {
        (0xA1, 0xA1) => Some('\u{3000}'), // Ideographic space
        (0xA1, 0xA2) => Some('\u{3001}'), // 、
        (0xA1, 0xA3) => Some('\u{3002}'), // 。
        (0xA1, 0xA4) => Some('\u{00B7}'), // ·
        (0xA1, 0xB6) => Some('\u{300A}'), // 《
        (0xA1, 0xB7) => Some('\u{300B}'), // 》
        (0xA1, 0xB8) => Some('\u{2018}'), // '
        (0xA1, 0xB9) => Some('\u{2019}'), // '
        (0xA1, 0xBA) => Some('\u{201C}'), // "
        (0xA1, 0xBB) => Some('\u{201D}'), // "
        (0xA3, 0xA1) => Some('！'),
        (0xA3, 0xAC) => Some('，'),
        (0xA3, 0xBA) => Some('：'),
        (0xA3, 0xBB) => Some('；'),
        (0xA3, 0xBF) => Some('？'),
        _ => {
            // For CJK characters in GBK zone, use Unicode mapping
            // GBK range: 0xB0A1-0xF7FE for Level 1, 0x8140-0xA0FE for extended
            // This would require a full lookup table

            // For now, return a placeholder that indicates we have a character
            // Real implementation should use encoding_rs crate
            if high >= 0xB0 && high <= 0xF7 && low >= 0xA1 && low <= 0xFE {
                // Level 1 Chinese characters - return placeholder
                // In production, use proper mapping
                Some('\u{FFFD}')
            } else {
                None
            }
        }
    }
}

/// Normalize line endings to LF
pub fn normalize_line_endings(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// Clean up text: remove excessive whitespace, normalize
pub fn clean_text(text: &str) -> String {
    let normalized = normalize_line_endings(text);

    // Remove excessive blank lines (more than 2)
    let mut result = String::with_capacity(normalized.len());
    let mut blank_count = 0;

    for line in normalized.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            blank_count += 1;
            if blank_count <= 2 {
                result.push('\n');
            }
        } else {
            blank_count = 0;
            result.push_str(trimmed);
            result.push('\n');
        }
    }

    result.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_utf8() {
        assert!(is_valid_utf8(b"Hello World"));
        assert!(is_valid_utf8("你好世界".as_bytes()));
        assert!(!is_valid_utf8(&[0xFF, 0xFE, 0x00]));
    }

    #[test]
    fn test_normalize_line_endings() {
        assert_eq!(normalize_line_endings("a\r\nb\rc\n"), "a\nb\nc\n");
    }

    #[test]
    fn test_clean_text() {
        let input = "Line 1\n\n\n\n\nLine 2\n\nLine 3";
        let output = clean_text(input);
        assert!(output.matches('\n').count() <= 4);
    }

    #[test]
    fn test_decode_utf16le() {
        let bytes: Vec<u8> = vec![0x48, 0x00, 0x69, 0x00]; // "Hi" in UTF-16LE
        let result = decode_utf16le(&bytes).unwrap();
        assert_eq!(result, "Hi");
    }
}
