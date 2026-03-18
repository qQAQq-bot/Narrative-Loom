use narrative_loom_lib::core::book::{Book, BookStatus};
use narrative_loom_lib::core::embedding::{Chunk, VectorEntry, DEFAULT_EMBEDDING_DIMENSIONS};
use narrative_loom_lib::core::ids::{BookId, ChapterId, EntityId};
use narrative_loom_lib::retrieval::ContextBuilder;
use narrative_loom_lib::storage::book_db::{BookDb, Chapter, Character, Event, Setting};
use narrative_loom_lib::storage::vectors::VectorDb;
use tempfile::tempdir;

fn insert_fixture_entities(book_db: &BookDb, chapter_id: &ChapterId) {
    for idx in 0..8 {
        let character = Character {
            id: EntityId::from_string(format!("char_{idx}")),
            name: format!("角色{idx}"),
            aliases: vec![format!("角色别名{idx}")],
            description: Some("测试角色".to_string()),
            description_structured: None,
            traits: vec![],
            role: "supporting".to_string(),
            first_appearance_chapter_id: Some(chapter_id.clone()),
            relationships: serde_json::json!({}),
            evidence: vec![],
            notes: None,
            updated_at: "2026-02-16T00:00:00Z".to_string(),
        };
        book_db.insert_character(&character).unwrap();
    }

    for idx in 0..6 {
        let setting = Setting {
            id: EntityId::from_string(format!("setting_{idx}")),
            setting_type: "location".to_string(),
            name: format!("地点{idx}"),
            description: Some("测试地点".to_string()),
            description_structured: None,
            properties: serde_json::json!({}),
            evidence: vec![],
            notes: None,
            updated_at: "2026-02-16T00:00:00Z".to_string(),
        };
        book_db.insert_setting(&setting).unwrap();
    }

    for idx in 0..6 {
        let event = Event {
            id: EntityId::from_string(format!("event_{idx}")),
            title: format!("事件{idx}"),
            description: Some("测试事件".to_string()),
            chapter_id: Some(chapter_id.clone()),
            characters_involved: vec![],
            importance: "medium".to_string(),
            evidence: vec![],
            notes: None,
            updated_at: "2026-02-16T00:00:00Z".to_string(),
            time_marker: None,
            order_in_chapter: idx,
            is_flashback: false,
            relative_time: None,
        };
        book_db.insert_event(&event).unwrap();
    }
}

fn create_context_for_fixture() -> narrative_loom_lib::retrieval::AnalysisContext {
    let temp_dir = tempdir().unwrap();
    let book_db_path = temp_dir.path().join("book.db");
    let vectors_db_path = temp_dir.path().join("vectors.db");

    let book_id = BookId::from_string("book_fixture".to_string());
    let chapter_id = ChapterId::from_string("chapter_100".to_string());

    let book_db = BookDb::open(&book_db_path, book_id.clone()).unwrap();
    let book = Book {
        id: book_id.clone(),
        title: "Fixture Book".to_string(),
        author: None,
        cover_path: None,
        total_chapters: 1,
        analyzed_chapters: 0,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        status: BookStatus::Ready,
    };
    book_db.insert_book(&book).unwrap();

    let chapter = Chapter {
        id: chapter_id.clone(),
        book_id: book_id.clone(),
        index_num: 100,
        title: Some("Chapter 100".to_string()),
        parent_title: None,
        char_count: 5000,
        analyzed: false,
        technique_count: 0,
        knowledge_count: 0,
    };
    book_db.insert_chapter(&chapter).unwrap();
    book_db
        .insert_chapter_content(&chapter_id, "角色0 和 地点0 触发了 事件0。")
        .unwrap();

    insert_fixture_entities(&book_db, &chapter_id);

    let vector_db = VectorDb::open(&vectors_db_path, DEFAULT_EMBEDDING_DIMENSIONS).unwrap();
    vector_db
        .set_embedding_signature("fixture", "fixture-model", DEFAULT_EMBEDDING_DIMENSIONS)
        .unwrap();

    let mut entities = Vec::new();
    entities.extend((0..8).map(|idx| format!("char_{idx}")));
    entities.extend((0..6).map(|idx| format!("setting_{idx}")));
    entities.extend((0..6).map(|idx| format!("event_{idx}")));

    let chunk = Chunk::new_paragraph(
        "chapter_099".to_string(),
        0,
        "历史章节包含角色、地点和事件信息。".to_string(),
        0,
        40,
    )
    .with_entities(entities);

    let chunk_id = chunk.id.clone();
    vector_db.insert_chunk(&chunk).unwrap();
    vector_db
        .insert_vector(&VectorEntry::new(
            chunk_id,
            vec![0.001; DEFAULT_EMBEDDING_DIMENSIONS as usize],
        ))
        .unwrap();

    let current_chunk = Chunk::new_paragraph(
        "chapter_100".to_string(),
        0,
        "当前章节用于复用 embedding。".to_string(),
        0,
        20,
    );
    let current_chunk_id = current_chunk.id.clone();
    vector_db.insert_chunk(&current_chunk).unwrap();
    vector_db
        .insert_vector(&VectorEntry::new(
            current_chunk_id,
            vec![0.001; DEFAULT_EMBEDDING_DIMENSIONS as usize],
        ))
        .unwrap();

    let builder = ContextBuilder::new(&book_db_path, book_id)
        .unwrap()
        .with_vector_db(&vectors_db_path, DEFAULT_EMBEDDING_DIMENSIONS)
        .unwrap();

    builder
        .build_smart_context(
            &chapter_id,
            "角色0 在 地点0 再次经历 事件0 的余波。",
            20,
            10,
            15,
            12,
        )
        .unwrap()
}

#[test]
fn test_build_smart_context_returns_balanced_entities() {
    let ctx = create_context_for_fixture();
    assert!(ctx.known_characters.len() >= 8);
    assert!(ctx.known_settings.len() >= 6);
    assert!(ctx.known_events.len() >= 6);
}
