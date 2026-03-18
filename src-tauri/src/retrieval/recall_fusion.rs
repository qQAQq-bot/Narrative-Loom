use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecallMode {
    Normal,
    FallbackEmbedding,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RecallWeights {
    pub vector: f32,
    pub keyword: f32,
    pub history: f32,
}

impl RecallWeights {
    pub fn for_mode(mode: RecallMode) -> Self {
        match mode {
            RecallMode::Normal => Self {
                vector: 0.55,
                keyword: 0.30,
                history: 0.15,
            },
            RecallMode::FallbackEmbedding => Self {
                vector: 0.25,
                keyword: 0.60,
                history: 0.15,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredChunk {
    pub chunk_id: String,
    pub score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FusedChunk {
    pub chunk_id: String,
    pub score: f32,
    pub vector_rank: Option<usize>,
    pub keyword_rank: Option<usize>,
    pub history_rank: Option<usize>,
}

#[derive(Debug, Default)]
struct FusionAccumulator {
    score: f32,
    vector_rank: Option<usize>,
    keyword_rank: Option<usize>,
    history_rank: Option<usize>,
}

pub fn fuse_chunk_scores(
    vector: Vec<ScoredChunk>,
    keyword: Vec<ScoredChunk>,
    history: Vec<ScoredChunk>,
) -> Vec<FusedChunk> {
    fuse_chunk_scores_with_mode(vector, keyword, history, RecallMode::Normal)
}

pub fn fuse_chunk_scores_with_mode(
    vector: Vec<ScoredChunk>,
    keyword: Vec<ScoredChunk>,
    history: Vec<ScoredChunk>,
    mode: RecallMode,
) -> Vec<FusedChunk> {
    const RRF_K: f32 = 60.0;
    const RAW_SCORE_SCALE: f32 = 0.05;
    const MULTI_CHANNEL_BOOST: f32 = 0.02;

    let weights = RecallWeights::for_mode(mode);
    let mut merged: HashMap<String, FusionAccumulator> = HashMap::new();

    for (rank, chunk) in vector.into_iter().enumerate() {
        let entry = merged.entry(chunk.chunk_id).or_default();
        let rrf_score = 1.0 / (RRF_K + rank as f32 + 1.0);
        entry.score += weights.vector * rrf_score + weights.vector * chunk.score * RAW_SCORE_SCALE;
        entry.vector_rank = Some(rank + 1);
    }

    for (rank, chunk) in keyword.into_iter().enumerate() {
        let entry = merged.entry(chunk.chunk_id).or_default();
        let rrf_score = 1.0 / (RRF_K + rank as f32 + 1.0);
        entry.score += weights.keyword * rrf_score + weights.keyword * chunk.score * RAW_SCORE_SCALE;
        entry.keyword_rank = Some(rank + 1);
    }

    for (rank, chunk) in history.into_iter().enumerate() {
        let entry = merged.entry(chunk.chunk_id).or_default();
        let rrf_score = 1.0 / (RRF_K + rank as f32 + 1.0);
        entry.score += weights.history * rrf_score + weights.history * chunk.score * RAW_SCORE_SCALE;
        entry.history_rank = Some(rank + 1);
    }

    let mut fused: Vec<FusedChunk> = merged
        .into_iter()
        .map(|(chunk_id, acc)| {
            let channel_hits = [acc.vector_rank, acc.keyword_rank, acc.history_rank]
                .iter()
                .filter(|rank| rank.is_some())
                .count();
            let boost = if channel_hits > 1 {
                (channel_hits - 1) as f32 * MULTI_CHANNEL_BOOST
            } else {
                0.0
            };

            FusedChunk {
                chunk_id,
                score: acc.score + boost,
                vector_rank: acc.vector_rank,
                keyword_rank: acc.keyword_rank,
                history_rank: acc.history_rank,
            }
        })
        .collect();

    fused.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.chunk_id.cmp(&b.chunk_id))
    });

    fused
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuse_chunk_scores_rrf_and_boosts() {
        let vector = vec![
            ScoredChunk {
                chunk_id: "c1".to_string(),
                score: 0.95,
            },
            ScoredChunk {
                chunk_id: "c2".to_string(),
                score: 0.80,
            },
        ];
        let keyword = vec![
            ScoredChunk {
                chunk_id: "c1".to_string(),
                score: 0.88,
            },
            ScoredChunk {
                chunk_id: "c3".to_string(),
                score: 0.60,
            },
        ];
        let history = vec![
            ScoredChunk {
                chunk_id: "c2".to_string(),
                score: 0.90,
            },
            ScoredChunk {
                chunk_id: "c1".to_string(),
                score: 0.70,
            },
        ];

        let fused = fuse_chunk_scores(vector, keyword, history);
        assert_eq!(fused[0].chunk_id, "c1");
        assert!(fused[0].score > fused[1].score);
    }

    #[test]
    fn test_fallback_mode_increases_keyword_weight() {
        let weights = RecallWeights::for_mode(RecallMode::FallbackEmbedding);
        assert!(weights.keyword > weights.vector);
    }
}
