use crate::core::agent::{AgentConfig, AgentKind, BuiltInAgent, OutputMode, TaskBindings, TaskType};
use crate::core::prompt_cards::PromptCard;
use crate::core::provider::{ApiFormat, ProviderConfig};
use crate::storage::paths;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to find config directory")]
    NoConfigDir,

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON parse error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Path error: {0}")]
    PathError(#[from] paths::PathError),
}

pub struct ConfigStore {
    config_dir: PathBuf,
}

impl ConfigStore {
    pub fn new() -> Result<Self, ConfigError> {
        let config_dir = paths::get_config_dir()?;
        fs::create_dir_all(&config_dir)?;

        Ok(Self { config_dir })
    }

    fn providers_path(&self) -> PathBuf {
        self.config_dir.join("providers.json")
    }

    fn agents_path(&self) -> PathBuf {
        self.config_dir.join("agents.json")
    }

    fn bindings_path(&self) -> PathBuf {
        self.config_dir.join("task_bindings.json")
    }

    fn settings_path(&self) -> PathBuf {
        self.config_dir.join("settings.json")
    }

    fn embedding_path(&self) -> PathBuf {
        self.config_dir.join("embedding.json")
    }

    fn prompt_cards_path(&self) -> PathBuf {
        self.config_dir.join("prompt_cards.json")
    }

    /// Get the library path. Returns the custom path if set, otherwise the default.
    pub fn get_library_path(&self) -> Result<PathBuf, ConfigError> {
        let settings_path = self.settings_path();
        if settings_path.exists() {
            let content = fs::read_to_string(&settings_path)?;
            if let Ok(settings) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(path) = settings.get("library_path").and_then(|v| v.as_str()) {
                    if !path.is_empty() {
                        return Ok(PathBuf::from(path));
                    }
                }
            }
        }
        // Return default path using the new paths module
        Ok(paths::get_library_dir()?)
    }

    /// Set the library path.
    pub fn set_library_path(&self, path: &str) -> Result<(), ConfigError> {
        let settings_path = self.settings_path();
        let mut settings: serde_json::Value = if settings_path.exists() {
            let content = fs::read_to_string(&settings_path)?;
            serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}))
        } else {
            serde_json::json!({})
        };

        settings["library_path"] = serde_json::Value::String(path.to_string());
        let content = serde_json::to_string_pretty(&settings)?;
        fs::write(settings_path, content)?;
        Ok(())
    }

    /// Get whether logging is enabled. Returns false by default.
    pub fn get_logging_enabled(&self) -> Result<bool, ConfigError> {
        let settings_path = self.settings_path();
        if settings_path.exists() {
            let content = fs::read_to_string(&settings_path)?;
            if let Ok(settings) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(enabled) = settings.get("logging_enabled").and_then(|v| v.as_bool()) {
                    return Ok(enabled);
                }
            }
        }
        Ok(false)
    }

    /// Set whether logging is enabled.
    pub fn set_logging_enabled(&self, enabled: bool) -> Result<(), ConfigError> {
        let settings_path = self.settings_path();
        let mut settings: serde_json::Value = if settings_path.exists() {
            let content = fs::read_to_string(&settings_path)?;
            serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}))
        } else {
            serde_json::json!({})
        };

        settings["logging_enabled"] = serde_json::Value::Bool(enabled);
        let content = serde_json::to_string_pretty(&settings)?;
        fs::write(settings_path, content)?;
        Ok(())
    }

    /// Get the request retry count. Returns 3 by default.
    pub fn get_request_retry_count(&self) -> Result<u32, ConfigError> {
        let settings_path = self.settings_path();
        if settings_path.exists() {
            let content = fs::read_to_string(&settings_path)?;
            if let Ok(settings) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(count) = settings.get("request_retry_count").and_then(|v| v.as_u64()) {
                    return Ok(count as u32);
                }
            }
        }
        Ok(3) // 默认 3 次重试
    }

    /// Set the request retry count.
    pub fn set_request_retry_count(&self, count: u32) -> Result<(), ConfigError> {
        let settings_path = self.settings_path();
        let mut settings: serde_json::Value = if settings_path.exists() {
            let content = fs::read_to_string(&settings_path)?;
            serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}))
        } else {
            serde_json::json!({})
        };

        settings["request_retry_count"] = serde_json::Value::Number(count.into());
        let content = serde_json::to_string_pretty(&settings)?;
        fs::write(settings_path, content)?;
        Ok(())
    }

    pub fn load_providers(&self) -> Result<Vec<ProviderConfig>, ConfigError> {
        let path = self.providers_path();
        if !path.exists() {
            return Ok(default_providers());
        }

        let content = fs::read_to_string(&path)?;
        let providers: Vec<ProviderConfig> = serde_json::from_str(&content)?;
        Ok(providers)
    }

    pub fn save_providers(&self, providers: &[ProviderConfig]) -> Result<(), ConfigError> {
        let content = serde_json::to_string_pretty(providers)?;
        fs::write(self.providers_path(), content)?;
        Ok(())
    }

    pub fn load_agents(&self) -> Result<Vec<AgentConfig>, ConfigError> {
        let path = self.agents_path();
        if !path.exists() {
            return Ok(default_agents());
        }

        let content = fs::read_to_string(&path)?;
        let mut agents: Vec<AgentConfig> = serde_json::from_str(&content)?;

        // Auto-add missing built-in agents
        let defaults = default_agents();
        let mut added_any = false;

        for default_agent in defaults {
            // Only handle built-in agents
            if let AgentKind::BuiltIn(built_in_type) = &default_agent.kind {
                // Check if this built-in type already exists
                let exists = agents.iter().any(|a| {
                    if let AgentKind::BuiltIn(existing_type) = &a.kind {
                        existing_type == built_in_type
                    } else {
                        false
                    }
                });

                if !exists {
                    agents.push(default_agent);
                    added_any = true;
                }
            }
        }

        // Save if we added any missing agents
        if added_any {
            let _ = self.save_agents(&agents);
        }

        Ok(agents)
    }

    pub fn save_agents(&self, agents: &[AgentConfig]) -> Result<(), ConfigError> {
        let content = serde_json::to_string_pretty(agents)?;
        fs::write(self.agents_path(), content)?;
        Ok(())
    }

    pub fn load_task_bindings(&self) -> Result<TaskBindings, ConfigError> {
        let path = self.bindings_path();
        if !path.exists() {
            return Ok(default_task_bindings());
        }

        let content = fs::read_to_string(&path)?;
        let mut bindings: TaskBindings = serde_json::from_str(&content)?;

        // Auto-add missing default bindings
        let defaults = default_task_bindings();
        let mut added_any = false;
        for (task_type, agent_id) in &defaults.bindings {
            if !bindings.bindings.contains_key(task_type) {
                bindings.bindings.insert(*task_type, agent_id.clone());
                added_any = true;
            }
        }

        if added_any {
            let _ = self.save_task_bindings(&bindings);
        }

        Ok(bindings)
    }

    pub fn save_task_bindings(&self, bindings: &TaskBindings) -> Result<(), ConfigError> {
        let content = serde_json::to_string_pretty(bindings)?;
        fs::write(self.bindings_path(), content)?;
        Ok(())
    }

    pub fn get_provider(&self, id: &str) -> Result<Option<ProviderConfig>, ConfigError> {
        let providers = self.load_providers()?;
        Ok(providers.into_iter().find(|p| p.id == id))
    }

    pub fn upsert_provider(&self, provider: ProviderConfig) -> Result<(), ConfigError> {
        let mut providers = self.load_providers()?;
        if let Some(pos) = providers.iter().position(|p| p.id == provider.id) {
            providers[pos] = provider;
        } else {
            providers.push(provider);
        }
        self.save_providers(&providers)
    }

    pub fn delete_provider(&self, id: &str) -> Result<bool, ConfigError> {
        let mut providers = self.load_providers()?;
        let original_len = providers.len();
        providers.retain(|p| p.id != id);
        if providers.len() < original_len {
            self.save_providers(&providers)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn get_agent(&self, id: &str) -> Result<Option<AgentConfig>, ConfigError> {
        let agents = self.load_agents()?;
        Ok(agents.into_iter().find(|a| a.id == id))
    }

    pub fn upsert_agent(&self, agent: AgentConfig) -> Result<(), ConfigError> {
        let mut agents = self.load_agents()?;
        if let Some(pos) = agents.iter().position(|a| a.id == agent.id) {
            agents[pos] = agent;
        } else {
            agents.push(agent);
        }
        self.save_agents(&agents)
    }

    pub fn delete_agent(&self, id: &str) -> Result<bool, ConfigError> {
        let mut agents = self.load_agents()?;
        let original_len = agents.len();
        agents.retain(|a| a.id != id);
        if agents.len() < original_len {
            self.save_agents(&agents)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Get auto-accept threshold. Returns "off" by default.
    /// Valid values: "off", "high", "medium", "low"
    pub fn get_auto_accept_threshold(&self) -> Result<String, ConfigError> {
        let settings_path = self.settings_path();
        if settings_path.exists() {
            let content = fs::read_to_string(&settings_path)?;
            if let Ok(settings) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(threshold) = settings.get("auto_accept_threshold").and_then(|v| v.as_str()) {
                    return Ok(threshold.to_string());
                }
            }
        }
        Ok("off".to_string())
    }

    /// Set auto-accept threshold.
    /// Valid values: "off", "high", "medium", "low"
    pub fn set_auto_accept_threshold(&self, threshold: &str) -> Result<(), ConfigError> {
        let settings_path = self.settings_path();
        let mut settings: serde_json::Value = if settings_path.exists() {
            let content = fs::read_to_string(&settings_path)?;
            serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}))
        } else {
            serde_json::json!({})
        };

        settings["auto_accept_threshold"] = serde_json::Value::String(threshold.to_string());
        let content = serde_json::to_string_pretty(&settings)?;
        fs::write(settings_path, content)?;
        Ok(())
    }

    /// Valid agent types for analysis.
    const VALID_AGENT_TYPES: [&'static str; 5] =
        ["technique", "character", "setting", "event", "style"];

    /// Get enabled agent types for analysis.
    /// Returns all valid agent types by default.
    /// Filters out any invalid/deprecated agent types from stored settings.
    pub fn get_enabled_agents(&self) -> Result<Vec<String>, ConfigError> {
        let settings_path = self.settings_path();
        if settings_path.exists() {
            let content = fs::read_to_string(&settings_path)?;
            if let Ok(settings) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(agents) = settings.get("enabled_agents").and_then(|v| v.as_array()) {
                    // Filter to only valid agent types
                    let valid_agents: Vec<String> = agents
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .filter(|s| Self::VALID_AGENT_TYPES.contains(&s.as_str()))
                        .collect();

                    // If we have at least one valid agent, return them
                    if !valid_agents.is_empty() {
                        return Ok(valid_agents);
                    }
                }
            }
        }
        // Default: all implemented agents enabled
        Ok(Self::VALID_AGENT_TYPES
            .iter()
            .map(|s| s.to_string())
            .collect())
    }

    /// Set enabled agent types for analysis.
    /// Silently filters out any invalid agent types.
    pub fn set_enabled_agents(&self, agents: &[String]) -> Result<(), ConfigError> {
        let settings_path = self.settings_path();
        let mut settings: serde_json::Value = if settings_path.exists() {
            let content = fs::read_to_string(&settings_path)?;
            serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}))
        } else {
            serde_json::json!({})
        };

        // Filter to only valid agent types
        let valid_agents: Vec<serde_json::Value> = agents
            .iter()
            .filter(|s| Self::VALID_AGENT_TYPES.contains(&s.as_str()))
            .map(|s| serde_json::Value::String(s.clone()))
            .collect();

        settings["enabled_agents"] = serde_json::Value::Array(valid_agents);
        let content = serde_json::to_string_pretty(&settings)?;
        fs::write(settings_path, content)?;
        Ok(())
    }

    /// Get Python executable path from settings.
    /// Returns None if not set (will fall back to environment variable or default).
    pub fn get_python_exe(&self) -> Result<Option<String>, ConfigError> {
        let settings_path = self.settings_path();
        if settings_path.exists() {
            let content = fs::read_to_string(&settings_path)?;
            if let Ok(settings) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(path) = settings.get("python_exe").and_then(|v| v.as_str()) {
                    if !path.is_empty() {
                        return Ok(Some(path.to_string()));
                    }
                }
            }
        }
        Ok(None)
    }

    /// Set Python executable path.
    pub fn set_python_exe(&self, path: &str) -> Result<(), ConfigError> {
        let settings_path = self.settings_path();
        let mut settings: serde_json::Value = if settings_path.exists() {
            let content = fs::read_to_string(&settings_path)?;
            serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}))
        } else {
            serde_json::json!({})
        };

        settings["python_exe"] = serde_json::Value::String(path.to_string());
        let content = serde_json::to_string_pretty(&settings)?;
        fs::write(settings_path, content)?;
        Ok(())
    }

    /// Get embedding configuration.
    /// Returns default config if not set.
    /// Handles migration from old format automatically.
    pub fn get_embedding_config(&self) -> Result<EmbeddingConfig, ConfigError> {
        let path = self.embedding_path();
        if !path.exists() {
            return Ok(EmbeddingConfig::default());
        }

        let content = fs::read_to_string(&path)?;

        // Try to parse new format first
        if let Ok(config) = serde_json::from_str::<EmbeddingConfig>(&content) {
            return Ok(config);
        }

        // Try to parse old format and migrate
        if let Ok(old_config) = serde_json::from_str::<serde_json::Value>(&content) {
            let provider_str = old_config.get("provider")
                .and_then(|v| v.as_str())
                .unwrap_or("disabled");

            let active_provider = match provider_str {
                "openai" => EmbeddingProviderType::OpenAI,
                "gemini" => EmbeddingProviderType::Gemini,
                _ => EmbeddingProviderType::Disabled,
            };

            // Migrate old settings to the appropriate provider
            let model = old_config.get("model")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let api_key_ref = old_config.get("api_key_ref")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let base_url = old_config.get("base_url")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let proxy_url = old_config.get("proxy_url")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let settings = EmbeddingProviderSettings {
                model,
                dimensions: None,
                api_key_ref,
                base_url: if active_provider == EmbeddingProviderType::OpenAI { base_url } else { None },
                proxy_url: if active_provider == EmbeddingProviderType::Gemini { proxy_url } else { None },
            };

            let mut config = EmbeddingConfig::default();
            config.active_provider = active_provider.clone();

            // Put settings in the right provider slot
            match active_provider {
                EmbeddingProviderType::OpenAI => config.openai = settings,
                EmbeddingProviderType::Gemini => config.gemini = settings,
                EmbeddingProviderType::Disabled => {}
            }

            // Save migrated config
            let _ = self.save_embedding_config(&config);

            return Ok(config);
        }

        // Fallback to default
        Ok(EmbeddingConfig::default())
    }

    /// Save embedding configuration.
    pub fn save_embedding_config(&self, config: &EmbeddingConfig) -> Result<(), ConfigError> {
        let content = serde_json::to_string_pretty(config)?;
        fs::write(self.embedding_path(), content)?;
        Ok(())
    }

    /// Load prompt cards from config.
    /// Returns empty vector if file doesn't exist.
    pub fn load_prompt_cards(&self) -> Result<Vec<PromptCard>, ConfigError> {
        let path = self.prompt_cards_path();
        if !path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&path)?;
        let cards: Vec<PromptCard> = serde_json::from_str(&content)?;
        Ok(cards)
    }

    /// Save prompt cards to config.
    pub fn save_prompt_cards(&self, cards: &[PromptCard]) -> Result<(), ConfigError> {
        let content = serde_json::to_string_pretty(cards)?;
        fs::write(self.prompt_cards_path(), content)?;
        Ok(())
    }
}

/// Embedding provider type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum EmbeddingProviderType {
    Disabled,
    OpenAI,
    Gemini,
}

impl Default for EmbeddingProviderType {
    fn default() -> Self {
        Self::Disabled
    }
}

/// Provider-specific embedding settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmbeddingProviderSettings {
    pub model: String,
    /// Optional output embedding dimensions override.
    ///
    /// If set, the embedding provider will be requested (when supported) to return
    /// vectors with this dimensionality. Changing this typically requires rebuilding
    /// existing vector databases.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<u32>,
    pub api_key_ref: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy_url: Option<String>,
}

/// Embedding configuration with per-provider settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    pub active_provider: EmbeddingProviderType,
    #[serde(default)]
    pub gemini: EmbeddingProviderSettings,
    #[serde(default)]
    pub openai: EmbeddingProviderSettings,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            active_provider: EmbeddingProviderType::Disabled,
            gemini: EmbeddingProviderSettings::default(),
            openai: EmbeddingProviderSettings::default(),
        }
    }
}

impl Default for ConfigStore {
    fn default() -> Self {
        Self::new().expect("Failed to initialize config store")
    }
}

pub fn default_providers() -> Vec<ProviderConfig> {
    vec![
        ProviderConfig {
            id: "openai".to_string(),
            name: "OpenAI".to_string(),
            enabled: true,
            base_url: "https://api.openai.com".to_string(),
            api_format: ApiFormat::OpenAI,
            path_override: None,
            api_key_ref: "openai".to_string(),
            default_model: "gpt-4o".to_string(),
            available_models: vec![
                "gpt-4o".to_string(),
                "gpt-4o-mini".to_string(),
                "gpt-4-turbo".to_string(),
                "gpt-3.5-turbo".to_string(),
            ],
            headers: HashMap::new(),
            timeout_ms: 60000,
            max_retries: 3,
        },
        ProviderConfig {
            id: "anthropic".to_string(),
            name: "Anthropic (Claude)".to_string(),
            enabled: false,
            base_url: "https://api.anthropic.com".to_string(),
            api_format: ApiFormat::Anthropic,
            path_override: None,
            api_key_ref: "anthropic".to_string(),
            default_model: "claude-sonnet-4-20250514".to_string(),
            available_models: vec![
                "claude-sonnet-4-20250514".to_string(),
                "claude-opus-4-20250514".to_string(),
                "claude-3-5-sonnet-20241022".to_string(),
                "claude-3-5-haiku-20241022".to_string(),
            ],
            headers: HashMap::new(),
            timeout_ms: 120000,
            max_retries: 3,
        },
        ProviderConfig {
            id: "deepseek".to_string(),
            name: "DeepSeek".to_string(),
            enabled: false,
            base_url: "https://api.deepseek.com".to_string(),
            api_format: ApiFormat::OpenAI,
            path_override: None,
            api_key_ref: "deepseek".to_string(),
            default_model: "deepseek-chat".to_string(),
            available_models: vec![
                "deepseek-chat".to_string(),
                "deepseek-coder".to_string(),
            ],
            headers: HashMap::new(),
            timeout_ms: 60000,
            max_retries: 3,
        },
        ProviderConfig {
            id: "ollama".to_string(),
            name: "Ollama (Local)".to_string(),
            enabled: false,
            base_url: "http://localhost:11434".to_string(),
            api_format: ApiFormat::Ollama,
            path_override: None,
            api_key_ref: "".to_string(),
            default_model: "qwen2.5:32b".to_string(),
            available_models: vec![
                "qwen2.5:32b".to_string(),
                "qwen2.5:14b".to_string(),
                "llama3.1:70b".to_string(),
                "mistral:7b".to_string(),
            ],
            headers: HashMap::new(),
            timeout_ms: 300000,
            max_retries: 2,
        },
    ]
}

pub fn default_agents() -> Vec<AgentConfig> {
    vec![
        AgentConfig {
            id: "technique-analysis".to_string(),
            name: "Technique Analysis Agent".to_string(),
            kind: AgentKind::BuiltIn(BuiltInAgent::TechniqueAnalysis),
            enabled: true,
            provider_id: "openai".to_string(),
            model: "gpt-4o".to_string(),
            temperature: 0.7,
            max_tokens: Some(4096),
            system_prompt: None,
            output_mode: OutputMode::JsonObject,
        },
        AgentConfig {
            id: "character-extraction".to_string(),
            name: "Character Extraction Agent".to_string(),
            kind: AgentKind::BuiltIn(BuiltInAgent::CharacterExtraction),
            enabled: true,
            provider_id: "openai".to_string(),
            model: "gpt-4o".to_string(),
            temperature: 0.3,
            max_tokens: Some(4096),
            system_prompt: None,
            output_mode: OutputMode::JsonObject,
        },
        AgentConfig {
            id: "setting-extraction".to_string(),
            name: "Setting Extraction Agent".to_string(),
            kind: AgentKind::BuiltIn(BuiltInAgent::SettingExtraction),
            enabled: true,
            provider_id: "openai".to_string(),
            model: "gpt-4o".to_string(),
            temperature: 0.3,
            max_tokens: Some(4096),
            system_prompt: None,
            output_mode: OutputMode::JsonObject,
        },
        AgentConfig {
            id: "event-extraction".to_string(),
            name: "Event Extraction Agent".to_string(),
            kind: AgentKind::BuiltIn(BuiltInAgent::EventExtraction),
            enabled: true,
            provider_id: "openai".to_string(),
            model: "gpt-4o".to_string(),
            temperature: 0.3,
            max_tokens: Some(4096),
            system_prompt: None,
            output_mode: OutputMode::JsonObject,
        },
        AgentConfig {
            id: "style-analysis".to_string(),
            name: "Style Analysis Agent".to_string(),
            kind: AgentKind::BuiltIn(BuiltInAgent::StyleAnalysis),
            enabled: true,
            provider_id: "openai".to_string(),
            model: "gpt-4o".to_string(),
            temperature: 0.3,
            max_tokens: Some(4096),
            system_prompt: None,
            output_mode: OutputMode::JsonObject,
        },
    ]
}

pub fn default_task_bindings() -> TaskBindings {
    let mut bindings = HashMap::new();
    bindings.insert(TaskType::TechniqueAnalysis, "technique-analysis".to_string());
    bindings.insert(TaskType::CharacterExtraction, "character-extraction".to_string());
    bindings.insert(TaskType::SettingExtraction, "setting-extraction".to_string());
    bindings.insert(TaskType::EventExtraction, "event-extraction".to_string());
    bindings.insert(TaskType::StyleAnalysis, "style-analysis".to_string());
    TaskBindings { bindings }
}
