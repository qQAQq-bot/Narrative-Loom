use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub base_url: String,
    pub api_format: ApiFormat,
    pub path_override: Option<String>,
    pub api_key_ref: String,
    pub default_model: String,
    pub available_models: Vec<String>,
    pub headers: HashMap<String, String>,
    pub timeout_ms: u64,
    pub max_retries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ApiFormat {
    /// OpenAI-compatible chat completions API (also used by DeepSeek, etc.)
    OpenAI,
    /// Anthropic Claude Messages API
    Anthropic,
    /// Local Ollama API
    Ollama,
    /// Legacy: maps to OpenAI for backwards compatibility
    ChatCompletions,
    /// Legacy: maps to OpenAI for backwards compatibility
    Responses,
}

impl ApiFormat {
    pub fn default_path(&self) -> &str {
        match self {
            ApiFormat::OpenAI | ApiFormat::ChatCompletions => "/v1/chat/completions",
            ApiFormat::Responses => "/v1/responses",
            ApiFormat::Anthropic => "/v1/messages",
            ApiFormat::Ollama => "/api/chat",
        }
    }

    /// Get the format string expected by Python providers
    pub fn to_python_format(&self) -> &str {
        match self {
            ApiFormat::OpenAI | ApiFormat::ChatCompletions => "openai",
            ApiFormat::Responses => "openai_responses",
            ApiFormat::Anthropic => "anthropic",
            ApiFormat::Ollama => "ollama",
        }
    }
}

impl ProviderConfig {
    pub fn build_url(&self) -> String {
        let path = self
            .path_override
            .as_deref()
            .unwrap_or_else(|| self.api_format.default_path());
        format!("{}{}", self.base_url.trim_end_matches('/'), path)
    }
}
