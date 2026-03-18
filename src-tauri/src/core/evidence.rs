use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub chapter_id: String,
    pub paragraph_index: u32,
    pub start_char: u32,
    pub end_char: u32,
    pub excerpt: String,
    pub hash: u64,
}
