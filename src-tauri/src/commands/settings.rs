use crate::core::agent::{AgentConfig, TaskBindings};
use crate::core::prompt_cards::PromptCard;
use crate::core::provider::ProviderConfig;
use crate::storage::config::{ConfigStore, EmbeddingProviderType, EmbeddingProviderSettings};
use crate::storage::keychain::{mask_api_key, KeychainService};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct ProviderWithStatus {
    #[serde(flatten)]
    pub config: ProviderConfig,
    pub has_api_key: bool,
    pub masked_api_key: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SaveProviderRequest {
    #[serde(flatten)]
    pub config: ProviderConfig,
    pub api_key: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TestConnectionResult {
    pub success: bool,
    pub message: String,
    pub latency_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<u32>,
}

#[tauri::command]
pub fn get_providers() -> Result<Vec<ProviderWithStatus>, String> {
    let config_store = ConfigStore::new().map_err(|e| e.to_string())?;
    let keychain = KeychainService::new();

    let providers = config_store.load_providers().map_err(|e| e.to_string())?;

    let providers_with_status: Vec<ProviderWithStatus> = providers
        .into_iter()
        .map(|config| {
            let has_api_key = keychain.has_key(&config.api_key_ref);
            let masked_api_key = if has_api_key {
                keychain.get_key(&config.api_key_ref).ok().map(|k| mask_api_key(&k))
            } else {
                None
            };

            ProviderWithStatus {
                config,
                has_api_key,
                masked_api_key,
            }
        })
        .collect();

    Ok(providers_with_status)
}

#[tauri::command]
pub fn save_provider(request: SaveProviderRequest) -> Result<(), String> {
    // Validate provider configuration
    if request.config.base_url.is_empty() || request.config.base_url == "unknown" {
        return Err("Provider base_url cannot be empty or 'unknown'. Please provide a valid URL.".to_string());
    }

    if request.config.default_model.is_empty() {
        return Err("Provider default_model cannot be empty. Please select a model.".to_string());
    }

    let config_store = ConfigStore::new().map_err(|e| e.to_string())?;
    let keychain = KeychainService::new();

    if let Some(api_key) = &request.api_key {
        if !api_key.is_empty() {
            keychain
                .store_key(&request.config.api_key_ref, api_key)
                .map_err(|e| e.to_string())?;
        }
    }

    config_store
        .upsert_provider(request.config)
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn delete_provider(id: String) -> Result<bool, String> {
    let config_store = ConfigStore::new().map_err(|e| e.to_string())?;
    let keychain = KeychainService::new();

    if let Ok(Some(provider)) = config_store.get_provider(&id) {
        let _ = keychain.delete_key(&provider.api_key_ref);
    }

    config_store.delete_provider(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn test_provider_connection(id: String) -> Result<TestConnectionResult, String> {
    let config_store = ConfigStore::new().map_err(|e| e.to_string())?;
    let keychain = KeychainService::new();

    let provider = config_store
        .get_provider(&id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Provider not found: {}", id))?;

    let api_key = if provider.api_key_ref.is_empty() {
        String::new()
    } else {
        keychain
            .get_key(&provider.api_key_ref)
            .map_err(|e| format!("API key not found: {}", e))?
    };

    let start = std::time::Instant::now();

    let url = format!(
        "{}/v1/models",
        provider.base_url.trim_end_matches('/')
    );

    let client = reqwest::Client::new();
    let mut request_builder = client
        .get(&url)
        .timeout(std::time::Duration::from_millis(provider.timeout_ms));

    if !api_key.is_empty() {
        request_builder = request_builder.header("Authorization", format!("Bearer {}", api_key));
    }

    match request_builder.send().await {
        Ok(response) => {
            let latency_ms = start.elapsed().as_millis() as u64;
            if response.status().is_success() {
                Ok(TestConnectionResult {
                    success: true,
                    message: "连接成功".to_string(),
                    latency_ms: Some(latency_ms),
                    dimensions: None,
                })
            } else {
                Ok(TestConnectionResult {
                    success: false,
                    message: format!("HTTP {}: {}", response.status(), response.status().canonical_reason().unwrap_or("未知错误")),
                    latency_ms: Some(latency_ms),
                    dimensions: None,
                })
            }
        }
        Err(e) => Ok(TestConnectionResult {
            success: false,
            message: format!("连接失败: {}", e),
            latency_ms: None,
            dimensions: None,
        }),
    }
}

#[tauri::command]
pub fn get_agents() -> Result<Vec<AgentConfig>, String> {
    let config_store = ConfigStore::new().map_err(|e| e.to_string())?;
    config_store.load_agents().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_agent(agent: AgentConfig) -> Result<(), String> {
    let config_store = ConfigStore::new().map_err(|e| e.to_string())?;
    config_store.upsert_agent(agent).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_agent(id: String) -> Result<bool, String> {
    let config_store = ConfigStore::new().map_err(|e| e.to_string())?;
    config_store.delete_agent(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_task_bindings() -> Result<TaskBindings, String> {
    let config_store = ConfigStore::new().map_err(|e| e.to_string())?;
    config_store.load_task_bindings().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_task_bindings(bindings: TaskBindings) -> Result<(), String> {
    let config_store = ConfigStore::new().map_err(|e| e.to_string())?;
    config_store
        .save_task_bindings(&bindings)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_library_path() -> Result<String, String> {
    let config_store = ConfigStore::new().map_err(|e| e.to_string())?;
    let path = config_store.get_library_path().map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn set_library_path(path: String) -> Result<(), String> {
    // Validate that the path exists or can be created
    let path_buf = std::path::PathBuf::from(&path);
    if !path_buf.exists() {
        std::fs::create_dir_all(&path_buf).map_err(|e| format!("Failed to create directory: {}", e))?;
    }

    let config_store = ConfigStore::new().map_err(|e| e.to_string())?;
    config_store.set_library_path(&path).map_err(|e| e.to_string())
}

#[derive(Debug, Serialize)]
pub struct FetchModelsResult {
    pub success: bool,
    pub models: Vec<String>,
    pub message: String,
}

#[tauri::command]
pub async fn fetch_provider_models(
    base_url: String,
    api_format: String,
    api_key: Option<String>,
    api_key_ref: Option<String>,
) -> Result<FetchModelsResult, String> {
    let keychain = KeychainService::new();

    // Get API key: prefer provided key, then try keychain
    let effective_api_key = if let Some(key) = &api_key {
        if !key.is_empty() {
            Some(key.clone())
        } else {
            None
        }
    } else {
        None
    }.or_else(|| {
        api_key_ref.as_ref().and_then(|key_ref| {
            if !key_ref.is_empty() {
                keychain.get_key(key_ref).ok()
            } else {
                None
            }
        })
    });

    let url = format!(
        "{}/v1/models",
        base_url.trim_end_matches('/')
    );

    let client = reqwest::Client::new();
    let mut request_builder = client
        .get(&url)
        .timeout(std::time::Duration::from_secs(30));

    // Add authorization header based on API format
    if let Some(key) = &effective_api_key {
        match api_format.as_str() {
            "anthropic" => {
                request_builder = request_builder.header("x-api-key", key);
                request_builder = request_builder.header("anthropic-version", "2023-06-01");
            }
            _ => {
                request_builder = request_builder.header("Authorization", format!("Bearer {}", key));
            }
        }
    }

    match request_builder.send().await {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<serde_json::Value>().await {
                    Ok(json) => {
                        let models: Vec<String> = json
                            .get("data")
                            .and_then(|d| d.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|m| m.get("id").and_then(|id| id.as_str()).map(String::from))
                                    .collect()
                            })
                            .unwrap_or_default();

                        Ok(FetchModelsResult {
                            success: true,
                            models,
                            message: "获取模型列表成功".to_string(),
                        })
                    }
                    Err(e) => Ok(FetchModelsResult {
                        success: false,
                        models: vec![],
                        message: format!("解析响应失败: {}", e),
                    }),
                }
            } else {
                Ok(FetchModelsResult {
                    success: false,
                    models: vec![],
                    message: format!(
                        "HTTP {}: {}",
                        response.status(),
                        response.status().canonical_reason().unwrap_or("未知错误")
                    ),
                })
            }
        }
        Err(e) => Ok(FetchModelsResult {
            success: false,
            models: vec![],
            message: format!("连接失败: {}", e),
        }),
    }
}

#[tauri::command]
pub fn get_logging_enabled() -> Result<bool, String> {
    let config_store = ConfigStore::new().map_err(|e| e.to_string())?;
    config_store.get_logging_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_logging_enabled(enabled: bool) -> Result<(), String> {
    tracing::info!("set_logging_enabled called with enabled={}", enabled);

    let config_store = ConfigStore::new().map_err(|e| e.to_string())?;
    config_store.set_logging_enabled(enabled).map_err(|e| e.to_string())?;

    // Reinitialize logging based on the new setting
    crate::reinit_logging(enabled);

    tracing::info!("Logging setting updated, is_api_logging_enabled={}", crate::is_api_logging_enabled());

    Ok(())
}

#[tauri::command]
pub fn get_auto_accept_threshold() -> Result<String, String> {
    let config_store = ConfigStore::new().map_err(|e| e.to_string())?;
    config_store.get_auto_accept_threshold().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_auto_accept_threshold(threshold: String) -> Result<(), String> {
    // Validate threshold value
    let valid_thresholds = ["off", "high", "medium", "low"];
    if !valid_thresholds.contains(&threshold.as_str()) {
        return Err(format!("Invalid threshold value: {}. Valid values: off, high, medium, low", threshold));
    }

    let config_store = ConfigStore::new().map_err(|e| e.to_string())?;
    config_store.set_auto_accept_threshold(&threshold).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_enabled_agents() -> Result<Vec<String>, String> {
    let config_store = ConfigStore::new().map_err(|e| e.to_string())?;
    config_store.get_enabled_agents().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_enabled_agents(agents: Vec<String>) -> Result<(), String> {
    // ConfigStore handles filtering of invalid agent types
    let config_store = ConfigStore::new().map_err(|e| e.to_string())?;
    config_store.set_enabled_agents(&agents).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_request_retry_count() -> Result<u32, String> {
    let config_store = ConfigStore::new().map_err(|e| e.to_string())?;
    config_store.get_request_retry_count().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_request_retry_count(count: u32) -> Result<(), String> {
    // Validate retry count (0-10)
    if count > 10 {
        return Err("重试次数不能超过 10 次".to_string());
    }

    let config_store = ConfigStore::new().map_err(|e| e.to_string())?;
    config_store.set_request_retry_count(count).map_err(|e| e.to_string())
}

// ============================================================================
// Embedding Configuration Commands
// ============================================================================

#[derive(Debug, Serialize)]
pub struct EmbeddingConfigResponse {
    pub active_provider: String,
    pub gemini: EmbeddingProviderConfigResponse,
    pub openai: EmbeddingProviderConfigResponse,
}

#[derive(Debug, Serialize)]
pub struct EmbeddingProviderConfigResponse {
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<u32>,
    pub has_api_key: bool,
    pub masked_api_key: Option<String>,
    pub base_url: Option<String>,
    pub proxy_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SaveEmbeddingConfigRequest {
    pub provider: String,
    pub model: String,
    pub dimensions: Option<u32>,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub proxy_url: Option<String>,
}

fn build_provider_config_response(
    settings: &EmbeddingProviderSettings,
    keychain: &KeychainService,
) -> EmbeddingProviderConfigResponse {
    let has_api_key = !settings.api_key_ref.is_empty() && keychain.has_key(&settings.api_key_ref);
    let masked_api_key = if has_api_key {
        keychain.get_key(&settings.api_key_ref).ok().map(|k| mask_api_key(&k))
    } else {
        None
    };

    EmbeddingProviderConfigResponse {
        model: settings.model.clone(),
        dimensions: settings.dimensions,
        has_api_key,
        masked_api_key,
        base_url: settings.base_url.clone(),
        proxy_url: settings.proxy_url.clone(),
    }
}

#[tauri::command]
pub fn get_embedding_config() -> Result<EmbeddingConfigResponse, String> {
    let config_store = ConfigStore::new().map_err(|e| e.to_string())?;
    let keychain = KeychainService::new();

    let config = config_store.get_embedding_config().map_err(|e| e.to_string())?;

    let active_provider = match config.active_provider {
        EmbeddingProviderType::Disabled => "disabled",
        EmbeddingProviderType::OpenAI => "openai",
        EmbeddingProviderType::Gemini => "gemini",
    };

    Ok(EmbeddingConfigResponse {
        active_provider: active_provider.to_string(),
        gemini: build_provider_config_response(&config.gemini, &keychain),
        openai: build_provider_config_response(&config.openai, &keychain),
    })
}

#[tauri::command]
pub fn save_embedding_config(request: SaveEmbeddingConfigRequest) -> Result<(), String> {
    let config_store = ConfigStore::new().map_err(|e| e.to_string())?;
    let keychain = KeychainService::new();

    let provider_type = match request.provider.as_str() {
        "disabled" => EmbeddingProviderType::Disabled,
        "openai" => EmbeddingProviderType::OpenAI,
        "gemini" => EmbeddingProviderType::Gemini,
        _ => return Err(format!("Invalid provider: {}", request.provider)),
    };

    let mut config = config_store.get_embedding_config().map_err(|e| e.to_string())?;
    config.active_provider = provider_type;

    // Update the specific provider settings
    let api_key_ref = format!("embedding_{}", request.provider);

    // Store API key if provided
    if let Some(api_key) = &request.api_key {
        if !api_key.is_empty() {
            keychain
                .store_key(&api_key_ref, api_key)
                .map_err(|e| e.to_string())?;
        }
    }

    // Update provider-specific settings
    match request.provider.as_str() {
        "gemini" => {
            config.gemini.model = request.model;
            config.gemini.dimensions = request.dimensions;
            config.gemini.proxy_url = request.proxy_url;
            // Update api_key_ref if a new key was provided
            if request.api_key.as_ref().map(|k| !k.is_empty()).unwrap_or(false) {
                config.gemini.api_key_ref = api_key_ref;
            }
        }
        "openai" => {
            config.openai.model = request.model;
            config.openai.dimensions = request.dimensions;
            config.openai.base_url = request.base_url;
            // Update api_key_ref if a new key was provided
            if request.api_key.as_ref().map(|k| !k.is_empty()).unwrap_or(false) {
                config.openai.api_key_ref = api_key_ref;
            }
        }
        _ => {} // disabled, nothing to update
    }

    config_store.save_embedding_config(&config).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn test_embedding_connection() -> Result<TestConnectionResult, String> {
    let config_store = ConfigStore::new().map_err(|e| e.to_string())?;
    let keychain = KeychainService::new();

    let config = config_store.get_embedding_config().map_err(|e| e.to_string())?;

    if config.active_provider == EmbeddingProviderType::Disabled {
        return Ok(TestConnectionResult {
            success: false,
            message: "Embedding 已禁用".to_string(),
            latency_ms: None,
            dimensions: None,
        });
    }

    let settings = match config.active_provider {
        EmbeddingProviderType::OpenAI => &config.openai,
        EmbeddingProviderType::Gemini => &config.gemini,
        EmbeddingProviderType::Disabled => unreachable!(),
    };

    let api_key = if settings.api_key_ref.is_empty() {
        String::new()
    } else {
        keychain.get_key(&settings.api_key_ref).unwrap_or_default()
    };

    let start = std::time::Instant::now();

    // Build embedding config for sidecar
    let embedding_config = match config.active_provider {
        EmbeddingProviderType::OpenAI => serde_json::json!({
            "provider": "openai",
            "api_key": api_key,
            "model": settings.model,
            "dimensions": settings.dimensions,
            "base_url": settings.base_url.clone().unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
        }),
        EmbeddingProviderType::Gemini => serde_json::json!({
            "provider": "gemini",
            "api_key": api_key,
            "model": settings.model,
            "dimensions": settings.dimensions,
            "proxy_url": settings.proxy_url,
        }),
        EmbeddingProviderType::Disabled => unreachable!(),
    };

    // Test with a simple embedding request
    let sidecar = crate::sidecar::get_sidecar();
    if !sidecar.is_running() {
        if let Err(e) = sidecar.start() {
            return Ok(TestConnectionResult {
                success: false,
                message: format!("无法启动 Python 引擎: {}", e),
                latency_ms: None,
                dimensions: None,
            });
        }
    }

    let params = serde_json::json!({
        "text": "测试文本",
        "config": embedding_config,
    });

    match sidecar.call("generate_embedding", params).await {
        Ok(result) => {
            let latency_ms = start.elapsed().as_millis() as u64;
            if result.get("embedding").is_some() {
                let dims = result.get("dimensions").and_then(|d| d.as_u64()).unwrap_or(0);
                Ok(TestConnectionResult {
                    success: true,
                    message: format!("连接成功，向量维度: {}", dims),
                    latency_ms: Some(latency_ms),
                    dimensions: Some(dims as u32),
                })
            } else {
                Ok(TestConnectionResult {
                    success: false,
                    message: "响应格式错误".to_string(),
                    latency_ms: Some(latency_ms),
                    dimensions: None,
                })
            }
        }
        Err(e) => Ok(TestConnectionResult {
            success: false,
            message: format!("连接失败: {}", e),
            latency_ms: None,
            dimensions: None,
        }),
    }
}

#[derive(Debug, Serialize)]
pub struct FetchEmbeddingModelsResult {
    pub success: bool,
    pub models: Vec<EmbeddingModelInfo>,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct EmbeddingModelInfo {
    pub id: String,
    pub name: String,
}

#[tauri::command]
pub async fn fetch_embedding_models(
    provider: String,
    api_key: Option<String>,
    api_key_ref: Option<String>,
    base_url: Option<String>,
    proxy_url: Option<String>,
) -> Result<FetchEmbeddingModelsResult, String> {
    let keychain = KeychainService::new();

    // Get effective API key
    let effective_api_key = api_key
        .filter(|k| !k.is_empty())
        .or_else(|| {
            api_key_ref.as_ref().and_then(|key_ref| {
                if !key_ref.is_empty() {
                    keychain.get_key(key_ref).ok()
                } else {
                    None
                }
            })
        });

    // Validate required fields
    match provider.as_str() {
        "gemini" => {
            if effective_api_key.is_none() {
                return Ok(FetchEmbeddingModelsResult {
                    success: false,
                    models: vec![],
                    message: "请先配置 API Key".to_string(),
                });
            }
        }
        "openai" => {
            if effective_api_key.is_none() {
                return Ok(FetchEmbeddingModelsResult {
                    success: false,
                    models: vec![],
                    message: "请先配置 API Key".to_string(),
                });
            }
        }
        _ => {
            return Ok(FetchEmbeddingModelsResult {
                success: false,
                models: vec![],
                message: format!("不支持的 provider: {}", provider),
            });
        }
    }

    // Call Python sidecar
    let sidecar = crate::sidecar::get_sidecar();
    if !sidecar.is_running() {
        if let Err(e) = sidecar.start() {
            return Ok(FetchEmbeddingModelsResult {
                success: false,
                models: vec![],
                message: format!("无法启动 Python 引擎: {}", e),
            });
        }
    }

    let params = serde_json::json!({
        "provider": provider,
        "api_key": effective_api_key.unwrap_or_default(),
        "base_url": base_url.unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
        "proxy_url": proxy_url,
    });

    match sidecar.call("fetch_embedding_models", params).await {
        Ok(result) => {
            let success = result.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
            let message = result.get("message").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let models: Vec<EmbeddingModelInfo> = result
                .get("models")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|m| {
                            Some(EmbeddingModelInfo {
                                id: m.get("id")?.as_str()?.to_string(),
                                name: m.get("name")?.as_str()?.to_string(),
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();

            Ok(FetchEmbeddingModelsResult {
                success,
                models,
                message,
            })
        }
        Err(e) => Ok(FetchEmbeddingModelsResult {
            success: false,
            models: vec![],
            message: format!("调用失败: {}", e),
        }),
    }
}

/// Check if embedding is properly configured
#[derive(Debug, Serialize)]
pub struct EmbeddingConfigStatus {
    /// Whether embedding is configured (not disabled and has required settings)
    pub is_configured: bool,
    /// Current provider name
    pub provider: String,
    /// What's missing for the configuration to be complete
    pub missing: Vec<String>,
    /// Human-readable status message
    pub message: String,
}

#[tauri::command]
pub fn check_embedding_configured() -> Result<EmbeddingConfigStatus, String> {
    let config_store = ConfigStore::new().map_err(|e| e.to_string())?;
    let keychain = KeychainService::new();

    let config = config_store.get_embedding_config().map_err(|e| e.to_string())?;

    let provider_str = match config.active_provider {
        EmbeddingProviderType::Disabled => "disabled",
        EmbeddingProviderType::OpenAI => "openai",
        EmbeddingProviderType::Gemini => "gemini",
    };

    // Check if disabled
    if config.active_provider == EmbeddingProviderType::Disabled {
        return Ok(EmbeddingConfigStatus {
            is_configured: false,
            provider: provider_str.to_string(),
            missing: vec!["Embedding 服务".to_string()],
            message: "请先在设置中配置 Embedding 服务".to_string(),
        });
    }

    let mut missing = Vec::new();

    let settings = match config.active_provider {
        EmbeddingProviderType::OpenAI => &config.openai,
        EmbeddingProviderType::Gemini => &config.gemini,
        EmbeddingProviderType::Disabled => unreachable!(),
    };

    // Check common requirements
    let has_api_key = !settings.api_key_ref.is_empty() && keychain.has_key(&settings.api_key_ref);
    if !has_api_key {
        missing.push("API Key".to_string());
    }
    if settings.model.is_empty() {
        missing.push("模型".to_string());
    }

    // Check provider-specific requirements
    match config.active_provider {
        EmbeddingProviderType::OpenAI => {
            if settings.base_url.is_none() || settings.base_url.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
                missing.push("Base URL".to_string());
            }
        }
        EmbeddingProviderType::Gemini => {
            // proxy_url is optional for Gemini
        }
        EmbeddingProviderType::Disabled => unreachable!(),
    }

    let is_configured = missing.is_empty();
    let message = if is_configured {
        format!("Embedding 已配置 ({})", provider_str)
    } else {
        format!("请先配置: {}", missing.join("、"))
    };

    Ok(EmbeddingConfigStatus {
        is_configured,
        provider: provider_str.to_string(),
        missing,
        message,
    })
}

// ============================================================================
// Prompt Cards Commands
// ============================================================================

#[tauri::command]
pub fn get_prompt_cards() -> Result<Vec<PromptCard>, String> {
    let config_store = ConfigStore::new().map_err(|e| e.to_string())?;
    config_store.load_prompt_cards().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_prompt_cards(cards: Vec<PromptCard>) -> Result<(), String> {
    let config_store = ConfigStore::new().map_err(|e| e.to_string())?;
    config_store.save_prompt_cards(&cards).map_err(|e| e.to_string())
}
