//! API Logger Module
//!
//! Handles logging of API requests and responses to a dedicated log file.

use chrono::Local;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

/// Get the API log file path
fn get_api_log_path() -> Option<PathBuf> {
    crate::storage::paths::get_logs_dir().ok().map(|dir| dir.join("api.log"))
}

/// Sanitize params by masking sensitive fields like API keys
fn sanitize_params(params: &serde_json::Value) -> serde_json::Value {
    match params {
        serde_json::Value::Object(map) => {
            let mut new_map = serde_json::Map::new();
            for (key, value) in map {
                if key == "api_key" || key == "apiKey" || key.ends_with("_key") {
                    // Mask the key
                    if let Some(s) = value.as_str() {
                        if s.len() > 8 {
                            let masked = format!("{}...{}", &s[..4], &s[s.len()-4..]);
                            new_map.insert(key.clone(), serde_json::Value::String(masked));
                        } else {
                            new_map.insert(key.clone(), serde_json::Value::String("****".to_string()));
                        }
                    } else {
                        new_map.insert(key.clone(), serde_json::Value::String("****".to_string()));
                    }
                } else if key == "provider_config" || key == "config" || key == "agent_config" {
                    // Recursively sanitize provider_config, embedding config, and agent_config
                    new_map.insert(key.clone(), sanitize_params(value));
                } else if key == "content" {
                    // Truncate long content (using char boundaries for UTF-8 safety)
                    if let Some(s) = value.as_str() {
                        let char_count: usize = s.chars().count();
                        if char_count > 100 {
                            // Take first 100 characters safely
                            let truncated_str: String = s.chars().take(100).collect();
                            let truncated = format!("{}... ({} chars)", truncated_str, char_count);
                            new_map.insert(key.clone(), serde_json::Value::String(truncated));
                        } else {
                            new_map.insert(key.clone(), value.clone());
                        }
                    } else {
                        new_map.insert(key.clone(), value.clone());
                    }
                } else if key == "texts" {
                    // Truncate texts array for embedding requests
                    if let Some(arr) = value.as_array() {
                        let truncated_arr: Vec<serde_json::Value> = arr.iter().enumerate().map(|(i, v)| {
                            if let Some(s) = v.as_str() {
                                let char_count: usize = s.chars().count();
                                if char_count > 50 {
                                    let truncated_str: String = s.chars().take(50).collect();
                                    serde_json::Value::String(format!("[{}] {}... ({} chars)", i, truncated_str, char_count))
                                } else {
                                    serde_json::Value::String(format!("[{}] {}", i, s))
                                }
                            } else {
                                v.clone()
                            }
                        }).take(5).collect(); // Only show first 5 texts

                        let total_count = arr.len();
                        let mut result_arr = truncated_arr;
                        if total_count > 5 {
                            result_arr.push(serde_json::Value::String(format!("... and {} more texts", total_count - 5)));
                        }
                        new_map.insert(key.clone(), serde_json::Value::Array(result_arr));
                    } else {
                        new_map.insert(key.clone(), value.clone());
                    }
                } else if key == "known_characters" || key == "known_events" || key == "known_settings" {
                    // Truncate known_* arrays to show count only
                    if let Some(arr) = value.as_array() {
                        let count = arr.len();
                        new_map.insert(key.clone(), serde_json::Value::String(format!("[{} items]", count)));
                    } else {
                        new_map.insert(key.clone(), value.clone());
                    }
                } else {
                    new_map.insert(key.clone(), value.clone());
                }
            }
            serde_json::Value::Object(new_map)
        }
        other => other.clone(),
    }
}

/// Log an API request with LLM info
pub fn log_api_request(method: &str, params: &serde_json::Value) {
    let logging_enabled = crate::is_api_logging_enabled();

    if !logging_enabled {
        return;
    }

    let Some(log_path) = get_api_log_path() else {
        tracing::warn!("Failed to get API log path");
        return;
    };

    // Ensure the parent directory exists
    if let Some(parent) = log_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");

    // Check if this is an embedding request (has "config" with "provider" field)
    let is_embedding_request = params
        .get("config")
        .and_then(|c| c.get("provider"))
        .is_some();

    // Get embedding purpose if present
    let embedding_purpose = params
        .get("purpose")
        .and_then(|v| v.as_str())
        .unwrap_or("unspecified");

    let (api_base, model, api_format) = if is_embedding_request {
        // Embedding request format
        let config = params.get("config");
        let provider = config
            .and_then(|c| c.get("provider"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let model = config
            .and_then(|c| c.get("model"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let base_url = config
            .and_then(|c| c.get("base_url"))
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| {
                // For Gemini, use the Gemini API endpoint
                if provider == "gemini" {
                    "generativelanguage.googleapis.com"
                } else {
                    "embedding-api"
                }
            });
        (base_url, model, provider)
    } else if let Some(agent_config) = params.get("agent_config") {
        // analyze_single_agent format: agent_config (singular)
        let provider_config = agent_config.get("provider_config");
        (
            provider_config
                .and_then(|c| c.get("api_base"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown"),
            agent_config
                .get("model")
                .and_then(|v| v.as_str())
                .or_else(|| provider_config.and_then(|c| c.get("model")).and_then(|v| v.as_str()))
                .unwrap_or("unknown"),
            provider_config
                .and_then(|c| c.get("api_format"))
                .and_then(|v| v.as_str())
                .unwrap_or("openai"),
        )
    } else if let Some(agent_configs) = params.get("agent_configs") {
        // analyze_chapter format: agent_configs (plural)
        // Try to get from first available agent config
        let first_config = agent_configs
            .as_object()
            .and_then(|obj| obj.values().next());

        if let Some(cfg) = first_config {
            let provider_config = cfg.get("provider_config");
            (
                provider_config
                    .and_then(|c| c.get("api_base"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown"),
                cfg.get("model")
                    .and_then(|v| v.as_str())
                    .or_else(|| provider_config.and_then(|c| c.get("model")).and_then(|v| v.as_str()))
                    .unwrap_or("unknown"),
                provider_config
                    .and_then(|c| c.get("api_format"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("openai"),
            )
        } else {
            ("unknown", "unknown", "openai")
        }
    } else {
        // Legacy format: provider_config at top level
        let provider_config = params.get("provider_config");
        (
            provider_config
                .and_then(|c| c.get("api_base"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown"),
            provider_config
                .and_then(|c| c.get("model"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown"),
            provider_config
                .and_then(|c| c.get("api_format"))
                .and_then(|v| v.as_str())
                .unwrap_or("openai"),
        )
    };

    // Determine the endpoint based on api_format
    let full_url = if is_embedding_request {
        format!("{}/embedding", api_base)
    } else {
        let endpoint = match api_format {
            "openai_responses" => "/v1/responses",
            "anthropic" => "/v1/messages",
            "ollama" => "/api/chat",
            _ => "/v1/chat/completions", // default openai format
        };
        format!("{}{}", api_base.trim_end_matches('/'), endpoint)
    };

    // Sanitize and format params (single line, no pretty print)
    let sanitized_params = sanitize_params(params);
    let params_str = serde_json::to_string(&sanitized_params)
        .unwrap_or_else(|_| "{}".to_string());

    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        let _ = writeln!(file, "[{}] [INFO] [llm] Request to {}", timestamp, full_url);
        let _ = writeln!(file, "[{}] [INFO] [llm] Model: {}", timestamp, model);
        let _ = writeln!(file, "[{}] [INFO] [llm] API Format: {}", timestamp, api_format);
        if is_embedding_request {
            let _ = writeln!(file, "[{}] [INFO] [llm] Purpose: {}", timestamp, embedding_purpose);
        }
        let _ = writeln!(file, "[{}] [INFO] [llm] Method: {}", timestamp, method);
        let _ = writeln!(file, "[{}] [INFO] [llm] Params: {}", timestamp, params_str);
        // Flush immediately to ensure logs are written before the request completes
        if let Err(e) = file.flush() {
            tracing::error!("Failed to flush API log: {}", e);
        }
    } else {
        tracing::error!("Failed to open API log file: {:?}", log_path);
    }
}

/// Log an API response
pub fn log_api_response(_method: &str, result: &Result<serde_json::Value, String>, duration_ms: u64) {
    if !crate::is_api_logging_enabled() {
        return;
    }

    let Some(log_path) = get_api_log_path() else {
        return;
    };

    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");

    let (status, details) = match result {
        Ok(value) => {
            // Extract some useful info from the response
            let techniques_count = value
                .get("techniques")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            let characters_count = value
                .get("characters")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            let settings_count = value
                .get("settings")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            let events_count = value
                .get("events")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0);

            (
                "SUCCESS",
                format!(
                    "techniques={}, characters={}, settings={}, events={}",
                    techniques_count, characters_count, settings_count, events_count
                ),
            )
        }
        Err(e) => ("ERROR", e.clone()),
    };

    let log_entry = format!(
        "[{}] [INFO] [llm] Response: {} - {} ({}ms)",
        timestamp, status, details, duration_ms
    );

    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        let _ = writeln!(file, "{}", log_entry);
        let _ = writeln!(file); // Empty line for readability
        let _ = file.flush();
    }
}
