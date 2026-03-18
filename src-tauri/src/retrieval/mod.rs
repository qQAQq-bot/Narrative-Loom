pub mod context_builder;
pub mod recall_fusion;
pub mod vector_search;

pub use context_builder::{AnalysisContext, ContextBuilder, ContextError, format_context_for_prompt};
pub use context_builder::{CharacterSummary, EntityRecallTuning, EventSummary, PassageSummary, SettingSummary};

pub use vector_search::{
    VectorSearcher, VectorSearchError, SearchResult, SearchResultMetadata,
    EntitySearchResult, EntityInfo, AllEntities, EntityMention,
    extract_entity_mentions,
};

pub use recall_fusion::{FusedChunk, RecallMode, RecallWeights, ScoredChunk};
