pub mod commands;
pub mod core;
pub mod ingestion;
pub mod retrieval;
pub mod sidecar;
pub mod storage;

use commands::*;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Global flag to control whether API logging is enabled
static API_LOGGING_ENABLED: AtomicBool = AtomicBool::new(false);

/// Check if API logging is enabled
pub fn is_api_logging_enabled() -> bool {
    API_LOGGING_ENABLED.load(Ordering::Relaxed)
}

/// Reinitialize logging settings (called when user changes the setting)
pub fn reinit_logging(enabled: bool) {
    API_LOGGING_ENABLED.store(enabled, Ordering::Relaxed);
    if enabled {
        tracing::info!("API logging enabled");
    } else {
        tracing::info!("API logging disabled");
    }
}

fn init_logging() {
    // Load the logging setting from config
    let logging_enabled = match storage::config::ConfigStore::new() {
        Ok(config_store) => config_store.get_logging_enabled().unwrap_or(false),
        Err(_) => false,
    };

    // Store the initial setting
    API_LOGGING_ENABLED.store(logging_enabled, Ordering::Relaxed);

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(
            "narrative_loom_lib=info,\
             narrative_loom_lib::ingestion::epub_parser=warn,\
             narrative_loom_lib::sidecar=warn,\
             narrative_loom_lib::storage::vectors=warn,\
             tauri=info"
        ));

    // Try to set up file logging if enabled
    let file_layer = if logging_enabled {
        match storage::paths::get_logs_dir() {
            Ok(logs_dir) => {
                let file_appender = tracing_appender::rolling::daily(&logs_dir, "narrative-loom.log");
                Some(
                    tracing_subscriber::fmt::layer()
                        .with_writer(file_appender)
                        .with_ansi(false), // Disable ANSI colors for file output
                )
            }
            Err(e) => {
                eprintln!("Warning: Could not set up file logging: {}", e);
                None
            }
        }
    } else {
        None
    };

    // Console layer for development
    let console_layer = tracing_subscriber::fmt::layer();

    // Build subscriber based on whether file logging is available
    if let Some(file_layer) = file_layer {
        tracing_subscriber::registry()
            .with(filter)
            .with(console_layer)
            .with(file_layer)
            .init();
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(console_layer)
            .init();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_logging();
    
    tracing::info!("Starting Narrative Loom");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            library::list_books,
            library::get_book,
            library::preview_book_import,
            library::import_book,
            library::delete_book,
            library::get_book_chapters,
            library::get_chapter_content,
            library::get_library_stats,
            library::update_book_metadata,
            library::get_book_cover,
            chapter::get_chapters,
            chapter::get_chapter,
            chapter::get_chapter_by_index,
            chapter::get_adjacent_chapters,
            settings::get_providers,
            settings::save_provider,
            settings::delete_provider,
            settings::test_provider_connection,
            settings::get_agents,
            settings::save_agent,
            settings::delete_agent,
            settings::get_task_bindings,
            settings::save_task_bindings,
            settings::get_library_path,
            settings::set_library_path,
            settings::fetch_provider_models,
            settings::get_logging_enabled,
            settings::set_logging_enabled,
            settings::get_auto_accept_threshold,
            settings::set_auto_accept_threshold,
            settings::get_enabled_agents,
            settings::set_enabled_agents,
            settings::get_request_retry_count,
            settings::set_request_retry_count,
            settings::get_embedding_config,
            settings::save_embedding_config,
            settings::test_embedding_connection,
            settings::fetch_embedding_models,
            settings::check_embedding_configured,
            settings::get_prompt_cards,
            settings::save_prompt_cards,
            embedding::generate_chapter_embeddings,
            embedding::search_similar_chunks,
            embedding::check_vector_db_compatibility,
            embedding::rebuild_book_embeddings,
            embedding::get_embedding_stats,
            embedding::update_character_embedding,
            embedding::update_setting_embedding,
            embedding::update_event_embedding,
            embedding::delete_entity_embedding,
            embedding::rebuild_entity_embeddings,
            embedding::get_entity_embedding_stats,
            embedding::get_chapter_entity_mentions,
            embedding::get_entity_cooccurrence,
            analysis::get_technique_cards,
            analysis::get_knowledge_cards,
            analysis::collect_technique,
            analysis::uncollect_technique,
            analysis::get_collected_techniques,
            analysis::update_knowledge_card_status,
            analysis::get_pending_knowledge_cards,
            analysis::analyze_chapter,
            analysis::cancel_analysis,
            analysis::analyze_single_agent,
            analysis::mark_chapter_analyzed,
            analysis::batch_analyze_chapters,
            analysis::delete_technique_card,
            analysis::clear_chapter_technique_cards,
            analysis::delete_knowledge_card,
            analysis::clear_chapter_knowledge_cards,
            analysis::clear_chapter_all_cards,
            analysis::clear_all_technique_cards,
            analysis::clear_all_knowledge_cards,
            analysis::get_style_profile,
            analysis::clear_style_profile,
            analysis::get_style_observation,
            inbox::get_inbox,
            inbox::get_inbox_stats,
            inbox::accept_card,
            inbox::reject_card,
            inbox::batch_accept_cards,
            inbox::batch_reject_cards,
            inbox::accept_card_with_edits,
            inbox::merge_card,
            inbox::get_merge_candidates,
            bible::get_characters,
            bible::get_character,
            bible::update_character,
            bible::get_settings,
            bible::update_setting,
            bible::get_events,
            bible::update_event,
            bible::get_bible_stats,
            bible::delete_character,
            bible::delete_setting,
            bible::delete_event,
            bible::clear_all_characters,
            bible::clear_all_settings,
            bible::clear_all_events,
            bible::get_story_blueprint,
            bible::merge_duplicate_characters,
            bible::merge_duplicate_settings,
            bible::auto_update_character_roles,
            bible::update_character_role,
            search::search,
            search::search_chapters_content,
            search::search_entities,
            search::vector_search,
            search::search_entity_mentions,
            search::get_entity_history,
            search::extract_mentions_from_text,
            search::get_all_entities,
            search::semantic_search,
            export::export_bible,
            export::get_export_preview,
            export::export_style_prompt,
            export::get_style_prompt_preview,
            export::export_style_profile_json,
            export::get_collected_technique_count,
            export::get_technique_prompt_preview,
            export::export_technique_prompt,
            export::get_style_profile_prompt_preview,
            export::export_style_profile_prompt,
            technique_library::get_technique_types,
            technique_library::get_technique_library,
            technique_library::get_technique_type_with_examples,
            technique_library::update_technique_type,
            technique_library::delete_technique_type,
            technique_library::toggle_example_featured,
            technique_library::delete_technique_example,
            technique_library::get_technique_library_stats,
            technique_library::clear_technique_library,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
