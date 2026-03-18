use crate::core::embedding::ChunkType;
use crate::core::ids::{BookId, ChapterId};
use crate::retrieval::recall_fusion::{fuse_chunk_scores, ScoredChunk};
use crate::storage::book_db::{BookDb, Character, Setting, Event};
use crate::storage::vectors::VectorDb;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ContextError {
    #[error("Vector DB error: {0}")]
    VectorDbError(#[from] crate::storage::vectors::VectorDbError),

    #[error("Book DB error: {0}")]
    BookDbError(#[from] crate::storage::book_db::BookDbError),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Embedding error: {0}")]
    EmbeddingError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisContext {
    pub known_characters: Vec<CharacterSummary>,
    pub known_settings: Vec<SettingSummary>,
    pub known_events: Vec<EventSummary>,
    pub similar_passages: Vec<PassageSummary>,
    pub previous_summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterSummary {
    pub name: String,
    pub aliases: Vec<String>,
    pub description: Option<String>,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingSummary {
    pub name: String,
    pub description: Option<String>,
    pub location_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSummary {
    pub title: String,
    pub description: Option<String>,
    pub chapter_index: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassageSummary {
    pub content: String,
    pub chapter_id: String,
    pub score: f32,
}

#[derive(Debug, Clone)]
pub struct EntityRecallTuning {
    pub candidate_chunks_per_query: usize,
    pub max_passages: usize,
    pub min_character_coverage: usize,
    pub min_setting_coverage: usize,
    pub min_event_coverage: usize,
}

#[derive(Debug, Clone, Default)]
struct RankedEntities {
    characters: Vec<CharacterSummary>,
    settings: Vec<SettingSummary>,
    events: Vec<EventSummary>,
}

#[derive(Debug, Clone)]
struct CandidateChunk {
    content: String,
    chapter_id: String,
    entities_mentioned: Vec<String>,
}

impl Default for EntityRecallTuning {
    fn default() -> Self {
        Self {
            candidate_chunks_per_query: 60,
            max_passages: 12,
            min_character_coverage: 8,
            min_setting_coverage: 6,
            min_event_coverage: 6,
        }
    }
}

fn apply_entity_coverage_quota(
    mut ranked: RankedEntities,
    tuning: &EntityRecallTuning,
) -> RankedEntities {
    let target_characters = tuning.min_character_coverage.min(ranked.characters.len());
    let target_settings = tuning.min_setting_coverage.min(ranked.settings.len());
    let target_events = tuning.min_event_coverage.min(ranked.events.len());

    ranked.characters.truncate(target_characters);
    ranked.settings.truncate(target_settings);
    ranked.events.truncate(target_events);
    ranked
}

impl From<Character> for CharacterSummary {
    fn from(c: Character) -> Self {
        Self {
            name: c.name,
            aliases: c.aliases,
            description: c.description,
            role: c.role,
        }
    }
}

impl From<Setting> for SettingSummary {
    fn from(s: Setting) -> Self {
        Self {
            name: s.name,
            description: s.description,
            location_type: Some(s.setting_type),
        }
    }
}

impl From<Event> for EventSummary {
    fn from(e: Event) -> Self {
        Self {
            title: e.title,
            description: e.description,
            chapter_index: None,
        }
    }
}

pub struct ContextBuilder {
    book_db: BookDb,
    vector_db: Option<VectorDb>,
}

impl ContextBuilder {
    pub fn new<P: AsRef<Path>>(book_db_path: P, book_id: BookId) -> Result<Self, ContextError> {
        let book_db = BookDb::open(book_db_path.as_ref(), book_id)?;
        Ok(Self {
            book_db,
            vector_db: None,
        })
    }

    pub fn with_vector_db<P: AsRef<Path>>(mut self, path: P, dimensions: u32) -> Result<Self, ContextError> {
        self.vector_db = Some(VectorDb::open(path, dimensions)?);
        Ok(self)
    }

    pub fn build_analysis_context(
        &self,
        chapter_id: &ChapterId,
        query_embedding: Option<&[f32]>,
        max_characters: usize,
        max_settings: usize,
        max_events: usize,
        max_passages: usize,
    ) -> Result<AnalysisContext, ContextError> {
        let known_characters = self.get_known_characters(max_characters)?;
        let known_settings = self.get_known_settings(max_settings)?;
        let known_events = self.get_known_events(max_events)?;

        let similar_passages = if let (Some(vdb), Some(embedding)) = (&self.vector_db, query_embedding) {
            self.search_similar_passages(vdb, embedding, chapter_id, max_passages)?
        } else {
            Vec::new()
        };

        Ok(AnalysisContext {
            known_characters,
            known_settings,
            known_events,
            similar_passages,
            previous_summary: None,
        })
    }

    fn get_known_characters(&self, limit: usize) -> Result<Vec<CharacterSummary>, ContextError> {
        let characters = self.book_db.list_characters()?;
        Ok(characters
            .into_iter()
            .take(limit)
            .map(CharacterSummary::from)
            .collect())
    }

    fn get_known_settings(&self, limit: usize) -> Result<Vec<SettingSummary>, ContextError> {
        let settings = self.book_db.list_settings()?;
        Ok(settings
            .into_iter()
            .take(limit)
            .map(SettingSummary::from)
            .collect())
    }

    fn get_known_events(&self, limit: usize) -> Result<Vec<EventSummary>, ContextError> {
        let events = self.book_db.list_events()?;
        Ok(events
            .into_iter()
            .take(limit)
            .map(EventSummary::from)
            .collect())
    }

    fn search_similar_passages(
        &self,
        vector_db: &VectorDb,
        query_embedding: &[f32],
        exclude_chapter_id: &ChapterId,
        limit: usize,
    ) -> Result<Vec<PassageSummary>, ContextError> {
        let results = vector_db.search_similar(
            query_embedding,
            limit,
            Some(exclude_chapter_id.as_str()),
            Some(ChunkType::Paragraph),
        )?;

        Ok(results
            .into_iter()
            .map(|sc| PassageSummary {
                content: sc.chunk.content,
                chapter_id: sc.chunk.chapter_id,
                score: sc.score,
            })
            .collect())
    }

    /// Build analysis context with smart entity retrieval using vector search
    ///
    /// Instead of simply taking the first N entities, this method:
    /// 1. Tries to reuse existing embeddings from vectors.db for the current chapter
    /// 2. Falls back to generating new embeddings if none exist
    /// 3. Searches for similar paragraphs and extracts entity IDs from metadata
    /// 4. Returns entities that are most relevant to the current chapter
    /// 5. Falls back to simple truncation if vector search is unavailable
    pub fn build_smart_context(
        &self,
        chapter_id: &ChapterId,
        chapter_content: &str,
        max_characters: usize,
        max_settings: usize,
        max_events: usize,
        max_passages: usize,
    ) -> Result<AnalysisContext, ContextError> {
        // Try to use vector search for smart entity retrieval
        if let Some(ref vdb) = self.vector_db {
            // First, try to reuse existing embeddings from vectors.db
            // This avoids redundant API calls when embeddings were already generated
            let embeddings = match vdb.get_embeddings_by_chapter(chapter_id.as_str()) {
                Ok(embs) if !embs.is_empty() => {
                    tracing::debug!(
                        "Reusing {} existing embeddings for chapter {}",
                        embs.len(),
                        chapter_id.as_str()
                    );
                    embs
                }
                _ => {
                    // No existing embeddings, need to generate new ones
                    tracing::info!(
                        "No existing embeddings for chapter {}, generating via API",
                        chapter_id.as_str()
                    );

                    let query_chunks = split_content_for_query(chapter_content);

                    if query_chunks.is_empty() {
                        return self.build_analysis_context(
                            chapter_id,
                            None,
                            max_characters,
                            max_settings,
                            max_events,
                            max_passages,
                        );
                    }

                    match crate::sidecar::generate_embeddings_sync_with_purpose(&query_chunks, crate::sidecar::EmbeddingPurpose::RagQuery) {
                        Ok(embs) => embs,
                        Err(e) => {
                            tracing::warn!("Failed to generate embeddings for smart context: {}", e);
                            return self.build_analysis_context(
                                chapter_id,
                                None,
                                max_characters,
                                max_settings,
                                max_events,
                                max_passages,
                            );
                        }
                    }
                }
            };

            // Build context with embeddings (either reused or newly generated)
            return self.build_context_with_multi_query(
                vdb,
                &embeddings,
                chapter_id,
                chapter_content,
                max_characters,
                max_settings,
                max_events,
                max_passages,
            );
        }

        // Fallback to simple truncation
        self.build_analysis_context(
            chapter_id,
            None,
            max_characters,
            max_settings,
            max_events,
            max_passages,
        )
    }

    /// Build context using multiple query embeddings (full chapter coverage)
    ///
    /// Uses Paragraph chunks to find semantically related content, then extracts
    /// entity IDs from the metadata.entities_mentioned field of those chunks.
    fn build_context_with_multi_query(
        &self,
        vector_db: &VectorDb,
        query_embeddings: &[Vec<f32>],
        chapter_id: &ChapterId,
        chapter_content: &str,
        max_characters: usize,
        max_settings: usize,
        max_events: usize,
        max_passages: usize,
    ) -> Result<AnalysisContext, ContextError> {
        let tuning = EntityRecallTuning::default();
        let mut relevant_entity_ids: HashSet<String> = HashSet::new();
        let mut candidate_chunks: HashMap<String, CandidateChunk> = HashMap::new();
        let mut vector_candidates: Vec<ScoredChunk> = Vec::new();
        let mut keyword_candidates: Vec<ScoredChunk> = Vec::new();
        let mut history_candidates: Vec<ScoredChunk> = Vec::new();

        let search_limit = tuning.candidate_chunks_per_query.max(max_passages * 3);

        for embedding in query_embeddings {
            let similar_chunks = vector_db.search_similar(
                embedding,
                search_limit,
                Some(chapter_id.as_str()),
                Some(ChunkType::Paragraph),
            )?;

            for (rank, sc) in similar_chunks.into_iter().enumerate() {
                let chunk_id = sc.chunk.id.to_string();
                vector_candidates.push(ScoredChunk {
                    chunk_id: chunk_id.clone(),
                    score: sc.score,
                });

                if rank < search_limit / 2 {
                    history_candidates.push(ScoredChunk {
                        chunk_id: chunk_id.clone(),
                        score: sc.score * 0.8,
                    });
                }

                candidate_chunks
                    .entry(chunk_id)
                    .or_insert_with(|| CandidateChunk {
                        content: sc.chunk.content,
                        chapter_id: sc.chunk.chapter_id,
                        entities_mentioned: sc.chunk.metadata.entities_mentioned,
                    });
            }
        }

        let keyword_query = build_keyword_query(chapter_content);
        if !keyword_query.is_empty() {
            let keyword_hits = vector_db.search_by_keyword(
                &keyword_query,
                search_limit,
                Some(ChunkType::Paragraph),
                Some(chapter_id.as_str()),
            )?;

            for hit in keyword_hits {
                let chunk_id = hit.chunk.id.to_string();
                keyword_candidates.push(ScoredChunk {
                    chunk_id: chunk_id.clone(),
                    score: hit.score,
                });

                candidate_chunks.entry(chunk_id).or_insert_with(|| CandidateChunk {
                    content: hit.chunk.content,
                    chapter_id: hit.chunk.chapter_id,
                    entities_mentioned: hit.chunk.metadata.entities_mentioned,
                });
            }
        }

        let fused_chunks = fuse_chunk_scores(vector_candidates, keyword_candidates, history_candidates);
        let mut all_passages: Vec<PassageSummary> = Vec::new();

        for fused in fused_chunks.into_iter().take(tuning.candidate_chunks_per_query) {
            if let Some(candidate) = candidate_chunks.get(&fused.chunk_id) {
                for entity_id in &candidate.entities_mentioned {
                    relevant_entity_ids.insert(entity_id.clone());
                }

                all_passages.push(PassageSummary {
                    content: candidate.content.clone(),
                    chapter_id: candidate.chapter_id.clone(),
                    score: fused.score,
                });
            }
        }

        tracing::debug!(
            "Fused recall selected {} passages and {} entity IDs for chapter {}",
            all_passages.len(),
            relevant_entity_ids.len(),
            chapter_id.as_str()
        );

        // Get all entities from database
        let all_characters = self.book_db.list_characters()?;
        let all_settings = self.book_db.list_settings()?;
        let all_events = self.book_db.list_events()?;

        // Filter entities by relevance (those mentioned in similar paragraphs)
        let mut known_characters: Vec<CharacterSummary> = all_characters
            .iter()
            .filter(|c| relevant_entity_ids.contains(&c.id.to_string()))
            .take(max_characters)
            .map(|c| CharacterSummary::from(c.clone()))
            .collect();

        let mut known_settings: Vec<SettingSummary> = all_settings
            .iter()
            .filter(|s| relevant_entity_ids.contains(&s.id.to_string()))
            .take(max_settings)
            .map(|s| SettingSummary::from(s.clone()))
            .collect();

        let mut known_events: Vec<EventSummary> = all_events
            .iter()
            .filter(|e| relevant_entity_ids.contains(&e.id.to_string()))
            .take(max_events)
            .map(|e| EventSummary::from(e.clone()))
            .collect();

        // Supplement with recent entities if RAG didn't find enough
        // This ensures we always have some context even for new books
        if known_characters.len() < max_characters {
            let existing_ids: HashSet<_> = known_characters.iter().map(|c| c.name.clone()).collect();
            for c in all_characters.iter().rev().take(max_characters * 2) {
                if !existing_ids.contains(&c.name) && known_characters.len() < max_characters {
                    known_characters.push(CharacterSummary::from(c.clone()));
                }
            }
        }

        if known_settings.len() < max_settings {
            let existing_ids: HashSet<_> = known_settings.iter().map(|s| s.name.clone()).collect();
            for s in all_settings.iter().rev().take(max_settings * 2) {
                if !existing_ids.contains(&s.name) && known_settings.len() < max_settings {
                    known_settings.push(SettingSummary::from(s.clone()));
                }
            }
        }

        if known_events.len() < max_events {
            let existing_ids: HashSet<_> = known_events.iter().map(|e| e.title.clone()).collect();
            for e in all_events.iter().rev().take(max_events * 2) {
                if !existing_ids.contains(&e.title) && known_events.len() < max_events {
                    known_events.push(EventSummary::from(e.clone()));
                }
            }
        }

        let ranked = apply_entity_coverage_quota(
            RankedEntities {
                characters: known_characters,
                settings: known_settings,
                events: known_events,
            },
            &tuning,
        );
        let known_characters = ranked.characters;
        let known_settings = ranked.settings;
        let known_events = ranked.events;

        // Sort passages by score and take top N
        all_passages.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        all_passages.truncate(max_passages.min(tuning.max_passages));

        // Count how many entities came from RAG vs fallback
        let rag_chars: Vec<&CharacterSummary> = known_characters.iter()
            .filter(|c| all_characters.iter().any(|ac| ac.name == c.name && relevant_entity_ids.contains(&ac.id.to_string())))
            .collect();
        let rag_settings: Vec<&SettingSummary> = known_settings.iter()
            .filter(|s| all_settings.iter().any(|as_| as_.name == s.name && relevant_entity_ids.contains(&as_.id.to_string())))
            .collect();
        let rag_events: Vec<&EventSummary> = known_events.iter()
            .filter(|e| all_events.iter().any(|ae| ae.title == e.title && relevant_entity_ids.contains(&ae.id.to_string())))
            .collect();

        // Log RAG summary at debug level (verbose, repeats per agent)
        // Use tracing::debug to reduce noise - only visible with RUST_LOG=debug
        if !rag_chars.is_empty() || !rag_settings.is_empty() || !rag_events.is_empty() {
            tracing::debug!(
                "[RAG] Context for {}: {} chars ({} RAG), {} settings ({} RAG), {} events ({} RAG)",
                chapter_id.as_str(),
                known_characters.len(),
                rag_chars.len(),
                known_settings.len(),
                rag_settings.len(),
                known_events.len(),
                rag_events.len()
            );
        }

        Ok(AnalysisContext {
            known_characters,
            known_settings,
            known_events,
            similar_passages: all_passages,
            previous_summary: None,
        })
    }
}

pub fn format_context_for_prompt(context: &AnalysisContext, max_tokens: usize) -> String {
    let mut parts = Vec::new();
    let mut current_len = 0;
    let avg_chars_per_token = 2;
    let max_chars = max_tokens * avg_chars_per_token;

    if !context.known_characters.is_empty() {
        let mut char_section = String::from("【已知人物】\n");
        for c in &context.known_characters {
            let line = format!(
                "- {}{}：{}\n",
                c.name,
                if c.aliases.is_empty() {
                    String::new()
                } else {
                    format!("（{}）", c.aliases.join("、"))
                },
                c.description.as_deref().unwrap_or(&c.role)
            );
            if current_len + line.len() > max_chars {
                break;
            }
            char_section.push_str(&line);
            current_len += line.len();
        }
        parts.push(char_section);
    }

    if !context.known_settings.is_empty() {
        let mut setting_section = String::from("【已知设定】\n");
        for s in &context.known_settings {
            let line = format!(
                "- {}：{}\n",
                s.name,
                s.description.as_deref().unwrap_or("无描述")
            );
            if current_len + line.len() > max_chars {
                break;
            }
            setting_section.push_str(&line);
            current_len += line.len();
        }
        parts.push(setting_section);
    }

    if !context.known_events.is_empty() {
        let mut event_section = String::from("【已知事件】\n");
        for e in &context.known_events {
            let line = format!(
                "- {}：{}\n",
                e.title,
                e.description.as_deref().unwrap_or("无描述")
            );
            if current_len + line.len() > max_chars {
                break;
            }
            event_section.push_str(&line);
            current_len += line.len();
        }
        parts.push(event_section);
    }

    if !context.similar_passages.is_empty() && current_len < max_chars {
        let mut passage_section = String::from("【相关段落】\n");
        for p in &context.similar_passages {
            let line = format!("\"{}...\"\n", truncate_str(&p.content, 200));
            if current_len + line.len() > max_chars {
                break;
            }
            passage_section.push_str(&line);
            current_len += line.len();
        }
        parts.push(passage_section);
    }

    parts.join("\n")
}

fn truncate_str(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        s.chars().take(max_chars).collect()
    }
}

fn build_keyword_query(content: &str) -> String {
    content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(3)
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(240)
        .collect()
}

/// Split chapter content into chunks for query embedding generation
/// Each chunk is ~1500 characters to stay within embedding token limits
const QUERY_CHUNK_SIZE: usize = 1500;

fn split_content_for_query(content: &str) -> Vec<String> {
    let total_chars = content.chars().count();

    if total_chars == 0 {
        return Vec::new();
    }

    // If content is small enough, return as single chunk
    if total_chars <= QUERY_CHUNK_SIZE {
        return vec![content.to_string()];
    }

    let mut chunks = Vec::new();
    let chars: Vec<char> = content.chars().collect();

    // Split into chunks of QUERY_CHUNK_SIZE
    let mut start = 0;
    while start < total_chars {
        let end = (start + QUERY_CHUNK_SIZE).min(total_chars);

        // Try to find a good break point (paragraph or sentence boundary)
        let mut break_point = end;
        if end < total_chars {
            // Look backwards for a paragraph break
            for i in (start..end).rev() {
                if i + 1 < total_chars && chars[i] == '\n' && chars[i + 1] == '\n' {
                    break_point = i + 2;
                    break;
                }
            }

            // If no paragraph break found, look for sentence end
            if break_point == end {
                for i in (start..end).rev() {
                    if chars[i] == '。' || chars[i] == '！' || chars[i] == '？'
                        || chars[i] == '.' || chars[i] == '!' || chars[i] == '?'
                    {
                        break_point = i + 1;
                        break;
                    }
                }
            }
        }

        let chunk: String = chars[start..break_point].iter().collect();
        if !chunk.trim().is_empty() {
            chunks.push(chunk);
        }

        start = break_point;
    }

    tracing::debug!(
        "Split {} chars into {} query chunks",
        total_chars,
        chunks.len()
    );

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recall_tuning_defaults() {
        let tuning = EntityRecallTuning::default();
        assert_eq!(tuning.candidate_chunks_per_query, 60);
        assert_eq!(tuning.max_passages, 12);
        assert_eq!(tuning.min_character_coverage, 8);
        assert_eq!(tuning.min_setting_coverage, 6);
        assert_eq!(tuning.min_event_coverage, 6);
    }

    #[test]
    fn test_coverage_rerank_keeps_minimum_per_type() {
        let ranked = RankedEntities {
            characters: (0..10)
                .map(|i| CharacterSummary {
                    name: format!("角色{i}"),
                    aliases: vec![],
                    description: None,
                    role: "support".to_string(),
                })
                .collect(),
            settings: (0..8)
                .map(|i| SettingSummary {
                    name: format!("地点{i}"),
                    description: None,
                    location_type: Some("location".to_string()),
                })
                .collect(),
            events: (0..7)
                .map(|i| EventSummary {
                    title: format!("事件{i}"),
                    description: None,
                    chapter_index: Some(i),
                })
                .collect(),
        };

        let result = apply_entity_coverage_quota(ranked, &EntityRecallTuning::default());
        assert!(result.characters.len() >= 8);
        assert!(result.settings.len() >= 6);
        assert!(result.events.len() >= 6);
    }

    #[test]
    fn test_prompt_budget_prioritizes_entity_summaries_before_passages() {
        let ctx = AnalysisContext {
            known_characters: (0..120)
                .map(|i| CharacterSummary {
                    name: format!("角色{i}"),
                    aliases: vec![],
                    description: Some("非常详细的角色背景描述".repeat(4)),
                    role: "support".to_string(),
                })
                .collect(),
            known_settings: vec![SettingSummary {
                name: "北境要塞".to_string(),
                description: Some("风雪终年不断".to_string()),
                location_type: Some("location".to_string()),
            }],
            known_events: vec![EventSummary {
                title: "夜袭前哨战".to_string(),
                description: Some("双方在黎明前交火".to_string()),
                chapter_index: Some(100),
            }],
            similar_passages: vec![PassageSummary {
                content: "这是一段很长的证据文本。".repeat(120),
                chapter_id: "chapter_099".to_string(),
                score: 0.9,
            }],
            previous_summary: None,
        };

        let prompt = format_context_for_prompt(&ctx, 800);
        assert!(prompt.contains("【已知人物】"));
        assert!(prompt.contains("【已知设定】"));
        assert!(prompt.contains("【已知事件】"));
    }
}
