// Export commands for exporting story bible and other data

use crate::core::ids::BookId;
use crate::storage::book_db::BookDb;
use crate::storage::library::Library;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportOptions {
    pub format: ExportFormat,
    pub include_characters: bool,
    pub include_settings: bool,
    pub include_events: bool,
    pub include_techniques: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Markdown,
    Json,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            format: ExportFormat::Markdown,
            include_characters: true,
            include_settings: true,
            include_events: true,
            include_techniques: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportResult {
    pub success: bool,
    pub file_path: Option<String>,
    pub content: Option<String>,
    pub error: Option<String>,
}

/// Options for style prompt export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleExportOptions {
    pub format: ExportFormat,
    pub include_narrative: bool,
    pub include_dialogue: bool,
    pub include_description: bool,
    pub include_pacing: bool,
    pub include_tension: bool,
    pub include_atmosphere: bool,
    pub example_count: usize,
    pub prompt_template: StylePromptTemplate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StylePromptTemplate {
    Generic,      // Generic system prompt
    OpenAI,       // OpenAI API format
    Claude,       // Claude API format
    LocalLlm,     // For local LLMs
}

impl Default for StyleExportOptions {
    fn default() -> Self {
        Self {
            format: ExportFormat::Markdown,
            include_narrative: true,
            include_dialogue: true,
            include_description: true,
            include_pacing: true,
            include_tension: true,
            include_atmosphere: true,
            example_count: 3,
            prompt_template: StylePromptTemplate::Generic,
        }
    }
}

/// Export story bible to a file
#[tauri::command]
pub async fn export_bible(
    book_id: String,
    options: ExportOptions,
    output_path: Option<String>,
) -> Result<ExportResult, String> {
    let library = Library::open().map_err(|e| e.to_string())?;
    let bid = BookId::from_string(book_id.clone());
    let book_dir = library.book_dir(&bid);
    let db_path = book_dir.join("book.db");

    if !db_path.exists() {
        return Err("Book database not found".to_string());
    }

    let db = BookDb::open(&db_path, bid.clone())
        .map_err(|e| format!("Failed to open book database: {}", e))?;

    // Get book metadata
    let book_meta = library
        .get_book(&bid)
        .map_err(|e| e.to_string())?
        .ok_or("Book not found")?;

    let content = match options.format {
        ExportFormat::Markdown => generate_markdown_export(&db, &book_meta.title, &options)?,
        ExportFormat::Json => generate_json_export(&db, &book_meta.title, &options)?,
    };

    // If output path is provided, write to file
    if let Some(path) = output_path {
        let output_path = PathBuf::from(&path);
        std::fs::write(&output_path, &content).map_err(|e| e.to_string())?;

        Ok(ExportResult {
            success: true,
            file_path: Some(path),
            content: None,
            error: None,
        })
    } else {
        // Return content directly
        Ok(ExportResult {
            success: true,
            file_path: None,
            content: Some(content),
            error: None,
        })
    }
}

/// Get export preview (returns content without saving)
#[tauri::command]
pub async fn get_export_preview(
    book_id: String,
    options: ExportOptions,
) -> Result<String, String> {
    let library = Library::open().map_err(|e| e.to_string())?;
    let bid = BookId::from_string(book_id);
    let book_dir = library.book_dir(&bid);
    let db_path = book_dir.join("book.db");

    if !db_path.exists() {
        return Err("Book database not found".to_string());
    }

    let db = BookDb::open(&db_path, bid.clone())
        .map_err(|e| format!("Failed to open book database: {}", e))?;

    let book_meta = library
        .get_book(&bid)
        .map_err(|e| e.to_string())?
        .ok_or("Book not found")?;

    match options.format {
        ExportFormat::Markdown => generate_markdown_export(&db, &book_meta.title, &options),
        ExportFormat::Json => generate_json_export(&db, &book_meta.title, &options),
    }
}

fn generate_markdown_export(
    db: &BookDb,
    book_title: &str,
    options: &ExportOptions,
) -> Result<String, String> {
    let mut md = String::new();

    // Title
    md.push_str(&format!("# {} - 故事圣经\n\n", book_title));
    md.push_str(&format!(
        "> 导出时间: {}\n\n",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
    ));
    md.push_str("---\n\n");

    // Characters
    if options.include_characters {
        let characters = db.list_characters().map_err(|e| e.to_string())?;
        if !characters.is_empty() {
            md.push_str("## 人物\n\n");
            for character in characters {
                md.push_str(&format!("### {}\n\n", character.name));

                if !character.aliases.is_empty() {
                    md.push_str(&format!("**别名**: {}\n\n", character.aliases.join(", ")));
                }

                md.push_str(&format!("**角色**: {}\n\n", character.role));

                if let Some(desc) = &character.description {
                    md.push_str(&format!("{}\n\n", desc));
                }

                if !character.traits.is_empty() {
                    md.push_str("**特征**:\n");
                    for trait_item in &character.traits {
                        md.push_str(&format!("- {}\n", trait_item));
                    }
                    md.push_str("\n");
                }

                // Relationships
                if let Some(rels) = character.relationships.as_object() {
                    if !rels.is_empty() {
                        md.push_str("**关系**:\n");
                        for (name, relation) in rels {
                            md.push_str(&format!(
                                "- {} - {}\n",
                                name,
                                relation.as_str().unwrap_or("")
                            ));
                        }
                        md.push_str("\n");
                    }
                }

                if let Some(notes) = &character.notes {
                    md.push_str(&format!("**备注**: {}\n\n", notes));
                }

                md.push_str("---\n\n");
            }
        }
    }

    // Settings
    if options.include_settings {
        let settings = db.list_settings().map_err(|e| e.to_string())?;
        if !settings.is_empty() {
            md.push_str("## 设定\n\n");
            for setting in settings {
                md.push_str(&format!("### {} ({})\n\n", setting.name, setting.setting_type));

                if let Some(desc) = &setting.description {
                    md.push_str(&format!("{}\n\n", desc));
                }

                // Properties
                if let Some(props) = setting.properties.as_object() {
                    if !props.is_empty() {
                        md.push_str("**属性**:\n");
                        for (key, value) in props {
                            md.push_str(&format!("- {}: {}\n", key, value));
                        }
                        md.push_str("\n");
                    }
                }

                if let Some(notes) = &setting.notes {
                    md.push_str(&format!("**备注**: {}\n\n", notes));
                }

                md.push_str("---\n\n");
            }
        }
    }

    // Events
    if options.include_events {
        let events = db.list_events().map_err(|e| e.to_string())?;
        if !events.is_empty() {
            md.push_str("## 事件\n\n");
            for event in events {
                md.push_str(&format!("### {}\n\n", event.title));

                md.push_str(&format!("**重要性**: {}\n\n", event.importance));

                if let Some(desc) = &event.description {
                    md.push_str(&format!("{}\n\n", desc));
                }

                if !event.characters_involved.is_empty() {
                    md.push_str(&format!(
                        "**相关人物**: {}\n\n",
                        event
                            .characters_involved
                            .iter()
                            .map(|c| c.to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }

                if let Some(notes) = &event.notes {
                    md.push_str(&format!("**备注**: {}\n\n", notes));
                }

                md.push_str("---\n\n");
            }
        }
    }

    // Techniques
    if options.include_techniques {
        let techniques = db.list_collected_technique_cards().map_err(|e| e.to_string())?;
        if !techniques.is_empty() {
            md.push_str("## 收藏的技法\n\n");

            // Group by type
            let mut grouped: std::collections::HashMap<String, Vec<_>> = std::collections::HashMap::new();
            for tech in techniques {
                grouped
                    .entry(tech.technique_type.clone())
                    .or_default()
                    .push(tech);
            }

            for (tech_type, techs) in grouped {
                md.push_str(&format!("### {}\n\n", get_technique_type_label(&tech_type)));
                for tech in techs {
                    md.push_str(&format!("#### {}\n\n", tech.title));
                    md.push_str(&format!("{}\n\n", tech.description));

                    if !tech.mechanism.is_empty() {
                        md.push_str(&format!("**实现机制**: {}\n\n", tech.mechanism));
                    }

                    if !tech.evidence.is_empty() {
                        md.push_str("**原文证据**:\n");
                        for evidence in &tech.evidence {
                            md.push_str(&format!("> {}\n\n", evidence));
                        }
                    }

                    if !tech.tags.is_empty() {
                        md.push_str(&format!("**标签**: {}\n\n", tech.tags.join(", ")));
                    }
                }
            }
        }
    }

    Ok(md)
}

fn generate_json_export(
    db: &BookDb,
    book_title: &str,
    options: &ExportOptions,
) -> Result<String, String> {
    let mut export_data = serde_json::json!({
        "title": book_title,
        "exported_at": chrono::Local::now().to_rfc3339(),
        "version": "1.0"
    });

    if options.include_characters {
        let characters = db.list_characters().map_err(|e| e.to_string())?;
        export_data["characters"] = serde_json::to_value(&characters).map_err(|e| e.to_string())?;
    }

    if options.include_settings {
        let settings = db.list_settings().map_err(|e| e.to_string())?;
        export_data["settings"] = serde_json::to_value(&settings).map_err(|e| e.to_string())?;
    }

    if options.include_events {
        let events = db.list_events().map_err(|e| e.to_string())?;
        export_data["events"] = serde_json::to_value(&events).map_err(|e| e.to_string())?;
    }

    if options.include_techniques {
        let techniques = db.list_collected_technique_cards().map_err(|e| e.to_string())?;
        export_data["techniques"] = serde_json::to_value(&techniques).map_err(|e| e.to_string())?;
    }

    serde_json::to_string_pretty(&export_data).map_err(|e| e.to_string())
}

fn get_technique_type_label(tech_type: &str) -> &str {
    match tech_type {
        "narrative" => "叙事技法",
        "dialogue" => "对话技法",
        "description" => "描写技法",
        "structure" => "结构技法",
        "pacing" => "节奏技法",
        "tension" => "张力技法",
        "foreshadowing" => "伏笔技法",
        "character" => "人物刻画",
        "atmosphere" => "氛围营造",
        "scene" => "场景技法",
        "suspense" => "悬念技法",
        "theme" => "主题技法",
        "voice" => "叙述声音",
        "other" => "其他技法",
        _ => tech_type,
    }
}

/// Export author style prompt
#[tauri::command]
pub async fn export_style_prompt(
    book_id: String,
    options: StyleExportOptions,
    output_path: Option<String>,
) -> Result<ExportResult, String> {
    let library = Library::open().map_err(|e| e.to_string())?;
    let bid = BookId::from_string(book_id.clone());
    let book_dir = library.book_dir(&bid);
    let db_path = book_dir.join("book.db");

    if !db_path.exists() {
        return Err("Book database not found".to_string());
    }

    let db = BookDb::open(&db_path, bid.clone())
        .map_err(|e| format!("Failed to open book database: {}", e))?;

    let book_meta = library
        .get_book(&bid)
        .map_err(|e| e.to_string())?
        .ok_or("Book not found")?;

    let author = book_meta.author.as_deref().unwrap_or("未知作者");

    let content = match options.format {
        ExportFormat::Markdown => {
            generate_style_markdown(&db, &book_meta.title, author, &options)?
        }
        ExportFormat::Json => {
            generate_style_json(&db, &book_meta.title, author, &options)?
        }
    };

    if let Some(path) = output_path {
        let output_path = PathBuf::from(&path);
        std::fs::write(&output_path, &content).map_err(|e| e.to_string())?;

        Ok(ExportResult {
            success: true,
            file_path: Some(path),
            content: None,
            error: None,
        })
    } else {
        Ok(ExportResult {
            success: true,
            file_path: None,
            content: Some(content),
            error: None,
        })
    }
}

/// Get style prompt preview
#[tauri::command]
pub async fn get_style_prompt_preview(
    book_id: String,
    options: StyleExportOptions,
) -> Result<String, String> {
    let library = Library::open().map_err(|e| e.to_string())?;
    let bid = BookId::from_string(book_id);
    let book_dir = library.book_dir(&bid);
    let db_path = book_dir.join("book.db");

    if !db_path.exists() {
        return Err("Book database not found".to_string());
    }

    let db = BookDb::open(&db_path, bid.clone())
        .map_err(|e| format!("Failed to open book database: {}", e))?;

    let book_meta = library
        .get_book(&bid)
        .map_err(|e| e.to_string())?
        .ok_or("Book not found")?;

    let author = book_meta.author.as_deref().unwrap_or("未知作者");

    match options.format {
        ExportFormat::Markdown => {
            generate_style_markdown(&db, &book_meta.title, author, &options)
        }
        ExportFormat::Json => {
            generate_style_json(&db, &book_meta.title, author, &options)
        }
    }
}

/// Collected style information
#[derive(Debug, Clone, Serialize)]
struct StyleInfo {
    category: String,
    techniques: Vec<TechniqueStyle>,
}

#[derive(Debug, Clone, Serialize)]
struct TechniqueStyle {
    title: String,
    description: String,
    mechanism: String,
    examples: Vec<String>,
}

fn collect_style_info(db: &BookDb, options: &StyleExportOptions) -> Result<Vec<StyleInfo>, String> {
    let techniques = db.list_collected_technique_cards().map_err(|e| e.to_string())?;

    // Group techniques by category
    let mut grouped: HashMap<String, Vec<TechniqueStyle>> = HashMap::new();

    // Filter categories based on options
    let included_categories: Vec<&str> = vec![
        if options.include_narrative { Some("narrative") } else { None },
        if options.include_dialogue { Some("dialogue") } else { None },
        if options.include_description { Some("description") } else { None },
        if options.include_pacing { Some("pacing") } else { None },
        if options.include_tension { Some("tension") } else { None },
        if options.include_atmosphere { Some("atmosphere") } else { None },
        // Always include some general categories
        Some("character"),
        Some("scene"),
        Some("voice"),
    ]
    .into_iter()
    .flatten()
    .collect();

    for tech in techniques {
        if !included_categories.contains(&tech.technique_type.as_str()) {
            continue;
        }

        let examples: Vec<String> = tech
            .evidence
            .into_iter()
            .take(options.example_count)
            .collect();

        let style = TechniqueStyle {
            title: tech.title,
            description: tech.description,
            mechanism: tech.mechanism,
            examples,
        };

        grouped
            .entry(tech.technique_type.clone())
            .or_default()
            .push(style);
    }

    Ok(grouped
        .into_iter()
        .map(|(category, techniques)| StyleInfo {
            category,
            techniques,
        })
        .collect())
}

fn generate_style_markdown(
    db: &BookDb,
    book_title: &str,
    author: &str,
    options: &StyleExportOptions,
) -> Result<String, String> {
    // Try to use style profile first (new approach)
    if let Ok(Some(profile)) = db.get_style_profile() {
        return generate_style_markdown_from_profile(&profile.profile_json, book_title, author, options);
    }

    // Fallback to technique cards (legacy approach)
    let styles = collect_style_info(db, options)?;
    let mut md = String::new();

    // Generate based on template
    match options.prompt_template {
        StylePromptTemplate::Generic => {
            md.push_str(&format!(
                "# 作者风格模仿 Prompt\n\n来源: 《{}》 - {}\n\n",
                book_title, author
            ));
            md.push_str("---\n\n");
            md.push_str("## System Prompt\n\n");
            md.push_str("```\n");
            md.push_str(&generate_system_prompt(&styles, book_title, author));
            md.push_str("\n```\n\n");
        }
        StylePromptTemplate::OpenAI => {
            md.push_str(&format!(
                "# OpenAI API 风格配置\n\n来源: 《{}》 - {}\n\n",
                book_title, author
            ));
            md.push_str("---\n\n");
            md.push_str("## System Message\n\n");
            md.push_str("将以下内容作为 `system` 角色的 message：\n\n");
            md.push_str("```\n");
            md.push_str(&generate_system_prompt(&styles, book_title, author));
            md.push_str("\n```\n\n");
            md.push_str("## API 调用示例\n\n");
            md.push_str("```python\n");
            md.push_str("from openai import OpenAI\n\n");
            md.push_str("client = OpenAI()\n\n");
            md.push_str("response = client.chat.completions.create(\n");
            md.push_str("    model=\"gpt-4\",\n");
            md.push_str("    messages=[\n");
            md.push_str("        {\"role\": \"system\", \"content\": SYSTEM_PROMPT},\n");
            md.push_str("        {\"role\": \"user\", \"content\": \"请以这种风格续写...\"}\n");
            md.push_str("    ]\n");
            md.push_str(")\n");
            md.push_str("```\n\n");
        }
        StylePromptTemplate::Claude => {
            md.push_str(&format!(
                "# Claude API 风格配置\n\n来源: 《{}》 - {}\n\n",
                book_title, author
            ));
            md.push_str("---\n\n");
            md.push_str("## System Prompt\n\n");
            md.push_str("将以下内容作为 `system` 参数：\n\n");
            md.push_str("```\n");
            md.push_str(&generate_system_prompt(&styles, book_title, author));
            md.push_str("\n```\n\n");
            md.push_str("## API 调用示例\n\n");
            md.push_str("```python\n");
            md.push_str("import anthropic\n\n");
            md.push_str("client = anthropic.Anthropic()\n\n");
            md.push_str("message = client.messages.create(\n");
            md.push_str("    model=\"claude-3-5-sonnet-20241022\",\n");
            md.push_str("    max_tokens=4096,\n");
            md.push_str("    system=SYSTEM_PROMPT,\n");
            md.push_str("    messages=[\n");
            md.push_str("        {\"role\": \"user\", \"content\": \"请以这种风格续写...\"}\n");
            md.push_str("    ]\n");
            md.push_str(")\n");
            md.push_str("```\n\n");
        }
        StylePromptTemplate::LocalLlm => {
            md.push_str(&format!(
                "# 本地 LLM 风格配置\n\n来源: 《{}》 - {}\n\n",
                book_title, author
            ));
            md.push_str("---\n\n");
            md.push_str("## System Prompt\n\n");
            md.push_str("适用于 llama.cpp, Ollama, vLLM 等本地推理框架：\n\n");
            md.push_str("```\n");
            md.push_str(&generate_system_prompt(&styles, book_title, author));
            md.push_str("\n```\n\n");
            md.push_str("## Ollama 示例\n\n");
            md.push_str("```bash\n");
            md.push_str("ollama run llama3 --system \"$(cat system_prompt.txt)\"\n");
            md.push_str("```\n\n");
        }
    }

    // Add detailed style breakdown
    md.push_str("---\n\n");
    md.push_str("## 风格分析详情\n\n");

    for style in &styles {
        md.push_str(&format!("### {}\n\n", get_technique_type_label(&style.category)));

        for tech in &style.techniques {
            md.push_str(&format!("#### {}\n\n", tech.title));
            md.push_str(&format!("{}\n\n", tech.description));

            if !tech.mechanism.is_empty() {
                md.push_str(&format!("**实现方式**: {}\n\n", tech.mechanism));
            }

            if !tech.examples.is_empty() {
                md.push_str("**示例段落**:\n\n");
                for example in &tech.examples {
                    md.push_str(&format!("> {}\n\n", example));
                }
            }
        }
    }

    Ok(md)
}

fn generate_style_json(
    db: &BookDb,
    book_title: &str,
    author: &str,
    options: &StyleExportOptions,
) -> Result<String, String> {
    // Try to use style profile first (new approach)
    if let Ok(Some(profile)) = db.get_style_profile() {
        return generate_style_json_from_profile(&profile.profile_json, book_title, author, options);
    }

    // Fallback to technique cards (legacy approach)
    let styles = collect_style_info(db, options)?;
    let system_prompt = generate_system_prompt(&styles, book_title, author);

    let export_data = serde_json::json!({
        "meta": {
            "source_book": book_title,
            "author": author,
            "exported_at": chrono::Local::now().to_rfc3339(),
            "template": format!("{:?}", options.prompt_template),
        },
        "system_prompt": system_prompt,
        "style_breakdown": styles,
        "api_configs": {
            "openai": {
                "model": "gpt-4",
                "messages": [
                    {
                        "role": "system",
                        "content": system_prompt
                    }
                ]
            },
            "anthropic": {
                "model": "claude-3-5-sonnet-20241022",
                "system": system_prompt
            },
            "ollama": {
                "model": "llama3",
                "system": system_prompt
            }
        }
    });

    serde_json::to_string_pretty(&export_data).map_err(|e| e.to_string())
}

/// Generate markdown export for technique cards only
fn generate_technique_markdown(
    styles: &[StyleInfo],
    book_title: &str,
    author: &str,
    options: &StyleExportOptions,
) -> Result<String, String> {
    let mut md = String::new();
    let system_prompt = generate_system_prompt(styles, book_title, author);

    // Generate based on template
    match options.prompt_template {
        StylePromptTemplate::Generic => {
            md.push_str(&format!(
                "# 技法风格模仿 Prompt\n\n来源: 《{}》 - {}\n\n",
                book_title, author
            ));
            md.push_str("---\n\n");
            md.push_str("## System Prompt\n\n");
            md.push_str("```\n");
            md.push_str(&system_prompt);
            md.push_str("\n```\n\n");
        }
        StylePromptTemplate::OpenAI => {
            md.push_str(&format!(
                "# OpenAI API 技法配置\n\n来源: 《{}》 - {}\n\n",
                book_title, author
            ));
            md.push_str("---\n\n");
            md.push_str("## System Message\n\n");
            md.push_str("将以下内容作为 `system` 角色的 message：\n\n");
            md.push_str("```\n");
            md.push_str(&system_prompt);
            md.push_str("\n```\n\n");
            md.push_str("## API 调用示例\n\n");
            md.push_str("```python\n");
            md.push_str("from openai import OpenAI\n\n");
            md.push_str("client = OpenAI()\n\n");
            md.push_str("response = client.chat.completions.create(\n");
            md.push_str("    model=\"gpt-4\",\n");
            md.push_str("    messages=[\n");
            md.push_str("        {\"role\": \"system\", \"content\": SYSTEM_PROMPT},\n");
            md.push_str("        {\"role\": \"user\", \"content\": \"请以这种风格续写...\"}\n");
            md.push_str("    ]\n");
            md.push_str(")\n");
            md.push_str("```\n\n");
        }
        StylePromptTemplate::Claude => {
            md.push_str(&format!(
                "# Claude API 技法配置\n\n来源: 《{}》 - {}\n\n",
                book_title, author
            ));
            md.push_str("---\n\n");
            md.push_str("## System Prompt\n\n");
            md.push_str("将以下内容作为 `system` 参数：\n\n");
            md.push_str("```\n");
            md.push_str(&system_prompt);
            md.push_str("\n```\n\n");
            md.push_str("## API 调用示例\n\n");
            md.push_str("```python\n");
            md.push_str("import anthropic\n\n");
            md.push_str("client = anthropic.Anthropic()\n\n");
            md.push_str("message = client.messages.create(\n");
            md.push_str("    model=\"claude-3-5-sonnet-20241022\",\n");
            md.push_str("    max_tokens=4096,\n");
            md.push_str("    system=SYSTEM_PROMPT,\n");
            md.push_str("    messages=[\n");
            md.push_str("        {\"role\": \"user\", \"content\": \"请以这种风格续写...\"}\n");
            md.push_str("    ]\n");
            md.push_str(")\n");
            md.push_str("```\n\n");
        }
        StylePromptTemplate::LocalLlm => {
            md.push_str(&format!(
                "# 本地 LLM 技法配置\n\n来源: 《{}》 - {}\n\n",
                book_title, author
            ));
            md.push_str("---\n\n");
            md.push_str("## System Prompt\n\n");
            md.push_str("适用于 llama.cpp, Ollama, vLLM 等本地推理框架：\n\n");
            md.push_str("```\n");
            md.push_str(&system_prompt);
            md.push_str("\n```\n\n");
            md.push_str("## Ollama 示例\n\n");
            md.push_str("```bash\n");
            md.push_str("ollama run llama3 --system \"$(cat system_prompt.txt)\"\n");
            md.push_str("```\n\n");
        }
    }

    // Add detailed technique breakdown
    md.push_str("---\n\n");
    md.push_str("## 技法分析详情\n\n");

    for style in styles {
        md.push_str(&format!("### {}\n\n", get_technique_type_label(&style.category)));

        for tech in &style.techniques {
            md.push_str(&format!("#### {}\n\n", tech.title));
            md.push_str(&format!("{}\n\n", tech.description));

            if !tech.mechanism.is_empty() {
                md.push_str(&format!("**实现方式**: {}\n\n", tech.mechanism));
            }

            if !tech.examples.is_empty() {
                md.push_str("**示例段落**:\n\n");
                for example in &tech.examples {
                    md.push_str(&format!("> {}\n\n", example));
                }
            }
        }
    }

    Ok(md)
}

/// Generate JSON export for technique cards only
fn generate_technique_json(
    styles: &[StyleInfo],
    book_title: &str,
    author: &str,
    options: &StyleExportOptions,
) -> Result<String, String> {
    let system_prompt = generate_system_prompt(styles, book_title, author);

    let export_data = serde_json::json!({
        "meta": {
            "source_book": book_title,
            "author": author,
            "exported_at": chrono::Local::now().to_rfc3339(),
            "template": format!("{:?}", options.prompt_template),
            "source": "technique_cards",
        },
        "system_prompt": system_prompt,
        "technique_breakdown": styles,
        "api_configs": {
            "openai": {
                "model": "gpt-4",
                "messages": [
                    {
                        "role": "system",
                        "content": system_prompt
                    }
                ]
            },
            "anthropic": {
                "model": "claude-3-5-sonnet-20241022",
                "system": system_prompt
            },
            "ollama": {
                "model": "llama3",
                "system": system_prompt
            }
        }
    });

    serde_json::to_string_pretty(&export_data).map_err(|e| e.to_string())
}

/// Generate markdown export from style profile
fn generate_style_profile_markdown(
    profile: &serde_json::Value,
    book_title: &str,
    author: &str,
    options: &StyleProfileExportOptions,
) -> Result<String, String> {
    let mut md = String::new();
    let system_prompt = generate_system_prompt_from_profile(profile, book_title, author);

    // Generate based on template
    match options.prompt_template {
        StylePromptTemplate::Generic => {
            md.push_str(&format!(
                "# 风格档案 Prompt\n\n来源: 《{}》 - {}\n\n",
                book_title, author
            ));
            md.push_str("---\n\n");
            md.push_str("## System Prompt\n\n");
            md.push_str("```\n");
            md.push_str(&system_prompt);
            md.push_str("\n```\n\n");
        }
        StylePromptTemplate::OpenAI => {
            md.push_str(&format!(
                "# OpenAI API 风格配置\n\n来源: 《{}》 - {}\n\n",
                book_title, author
            ));
            md.push_str("---\n\n");
            md.push_str("## System Message\n\n");
            md.push_str("将以下内容作为 `system` 角色的 message：\n\n");
            md.push_str("```\n");
            md.push_str(&system_prompt);
            md.push_str("\n```\n\n");
            md.push_str("## API 调用示例\n\n");
            md.push_str("```python\n");
            md.push_str("from openai import OpenAI\n\n");
            md.push_str("client = OpenAI()\n\n");
            md.push_str("response = client.chat.completions.create(\n");
            md.push_str("    model=\"gpt-4\",\n");
            md.push_str("    messages=[\n");
            md.push_str("        {\"role\": \"system\", \"content\": SYSTEM_PROMPT},\n");
            md.push_str("        {\"role\": \"user\", \"content\": \"请以这种风格续写...\"}\n");
            md.push_str("    ]\n");
            md.push_str(")\n");
            md.push_str("```\n\n");
        }
        StylePromptTemplate::Claude => {
            md.push_str(&format!(
                "# Claude API 风格配置\n\n来源: 《{}》 - {}\n\n",
                book_title, author
            ));
            md.push_str("---\n\n");
            md.push_str("## System Prompt\n\n");
            md.push_str("将以下内容作为 `system` 参数：\n\n");
            md.push_str("```\n");
            md.push_str(&system_prompt);
            md.push_str("\n```\n\n");
            md.push_str("## API 调用示例\n\n");
            md.push_str("```python\n");
            md.push_str("import anthropic\n\n");
            md.push_str("client = anthropic.Anthropic()\n\n");
            md.push_str("message = client.messages.create(\n");
            md.push_str("    model=\"claude-3-5-sonnet-20241022\",\n");
            md.push_str("    max_tokens=4096,\n");
            md.push_str("    system=SYSTEM_PROMPT,\n");
            md.push_str("    messages=[\n");
            md.push_str("        {\"role\": \"user\", \"content\": \"请以这种风格续写...\"}\n");
            md.push_str("    ]\n");
            md.push_str(")\n");
            md.push_str("```\n\n");
        }
        StylePromptTemplate::LocalLlm => {
            md.push_str(&format!(
                "# 本地 LLM 风格配置\n\n来源: 《{}》 - {}\n\n",
                book_title, author
            ));
            md.push_str("---\n\n");
            md.push_str("## System Prompt\n\n");
            md.push_str("适用于 llama.cpp, Ollama, vLLM 等本地推理框架：\n\n");
            md.push_str("```\n");
            md.push_str(&system_prompt);
            md.push_str("\n```\n\n");
            md.push_str("## Ollama 示例\n\n");
            md.push_str("```bash\n");
            md.push_str("ollama run llama3 --system \"$(cat system_prompt.txt)\"\n");
            md.push_str("```\n\n");
        }
    }

    // Add detailed style profile breakdown
    md.push_str("---\n\n");
    md.push_str("## 风格分析详情\n\n");
    md.push_str(&format!("```json\n{}\n```\n", serde_json::to_string_pretty(profile).unwrap_or_default()));

    Ok(md)
}

/// Generate JSON export from style profile
fn generate_style_profile_json(
    profile: &serde_json::Value,
    book_title: &str,
    author: &str,
    options: &StyleProfileExportOptions,
) -> Result<String, String> {
    let system_prompt = generate_system_prompt_from_profile(profile, book_title, author);

    let export_data = serde_json::json!({
        "meta": {
            "source_book": book_title,
            "author": author,
            "exported_at": chrono::Local::now().to_rfc3339(),
            "template": format!("{:?}", options.prompt_template),
            "source": "style_profile",
        },
        "system_prompt": system_prompt,
        "style_profile": profile,
        "api_configs": {
            "openai": {
                "model": "gpt-4",
                "messages": [
                    {
                        "role": "system",
                        "content": system_prompt
                    }
                ]
            },
            "anthropic": {
                "model": "claude-3-5-sonnet-20241022",
                "system": system_prompt
            },
            "ollama": {
                "model": "llama3",
                "system": system_prompt
            }
        }
    });

    serde_json::to_string_pretty(&export_data).map_err(|e| e.to_string())
}

/// Generate markdown export from style profile (new approach) - LEGACY, kept for compatibility
fn generate_style_markdown_from_profile(
    profile: &serde_json::Value,
    book_title: &str,
    author: &str,
    options: &StyleExportOptions,
) -> Result<String, String> {
    let mut md = String::new();
    let system_prompt = generate_system_prompt_from_profile(profile, book_title, author);

    // Generate based on template
    match options.prompt_template {
        StylePromptTemplate::Generic => {
            md.push_str(&format!(
                "# 作者风格模仿 Prompt\n\n来源: 《{}》 - {}\n\n",
                book_title, author
            ));
            md.push_str("---\n\n");
            md.push_str("## System Prompt\n\n");
            md.push_str("```\n");
            md.push_str(&system_prompt);
            md.push_str("\n```\n\n");
        }
        StylePromptTemplate::OpenAI => {
            md.push_str(&format!(
                "# OpenAI API 风格配置\n\n来源: 《{}》 - {}\n\n",
                book_title, author
            ));
            md.push_str("---\n\n");
            md.push_str("## System Message\n\n");
            md.push_str("将以下内容作为 `system` 角色的 message：\n\n");
            md.push_str("```\n");
            md.push_str(&system_prompt);
            md.push_str("\n```\n\n");
        }
        StylePromptTemplate::Claude => {
            md.push_str(&format!(
                "# Claude API 风格配置\n\n来源: 《{}》 - {}\n\n",
                book_title, author
            ));
            md.push_str("---\n\n");
            md.push_str("## System Prompt\n\n");
            md.push_str("将以下内容作为 `system` 参数：\n\n");
            md.push_str("```\n");
            md.push_str(&system_prompt);
            md.push_str("\n```\n\n");
        }
        StylePromptTemplate::LocalLlm => {
            md.push_str(&format!(
                "# 本地 LLM 风格配置\n\n来源: 《{}》 - {}\n\n",
                book_title, author
            ));
            md.push_str("---\n\n");
            md.push_str("## System Prompt\n\n");
            md.push_str("适用于 llama.cpp, Ollama, vLLM 等本地推理框架：\n\n");
            md.push_str("```\n");
            md.push_str(&system_prompt);
            md.push_str("\n```\n\n");
        }
    }

    // Add detailed style breakdown from profile
    md.push_str("---\n\n");
    md.push_str("## 风格分析详情\n\n");
    md.push_str(&format!("```json\n{}\n```\n", serde_json::to_string_pretty(profile).unwrap_or_default()));

    Ok(md)
}

/// Generate JSON export from style profile (new approach)
fn generate_style_json_from_profile(
    profile: &serde_json::Value,
    book_title: &str,
    author: &str,
    options: &StyleExportOptions,
) -> Result<String, String> {
    let system_prompt = generate_system_prompt_from_profile(profile, book_title, author);

    let export_data = serde_json::json!({
        "meta": {
            "source_book": book_title,
            "author": author,
            "exported_at": chrono::Local::now().to_rfc3339(),
            "template": format!("{:?}", options.prompt_template),
            "source": "style_profile",
        },
        "system_prompt": system_prompt,
        "style_profile": profile,
        "api_configs": {
            "openai": {
                "model": "gpt-4",
                "messages": [
                    {
                        "role": "system",
                        "content": system_prompt
                    }
                ]
            },
            "anthropic": {
                "model": "claude-3-5-sonnet-20241022",
                "system": system_prompt
            },
            "ollama": {
                "model": "llama3",
                "system": system_prompt
            }
        }
    });

    serde_json::to_string_pretty(&export_data).map_err(|e| e.to_string())
}

fn generate_system_prompt(styles: &[StyleInfo], book_title: &str, author: &str) -> String {
    let mut prompt = String::new();

    prompt.push_str(&format!(
        "你是一位精通模仿{}写作风格的作家助手。\n\n",
        author
    ));

    prompt.push_str(&format!(
        "以下是从《{}》中提取的写作技法和风格特点，请在创作时严格遵循：\n\n",
        book_title
    ));

    for style in styles {
        let category_label = get_technique_type_label(&style.category);
        prompt.push_str(&format!("【{}】\n", category_label));

        for tech in &style.techniques {
            prompt.push_str(&format!("• {}: {}\n", tech.title, tech.description));

            if !tech.mechanism.is_empty() {
                prompt.push_str(&format!("  实现方式: {}\n", tech.mechanism));
            }
        }

        prompt.push('\n');
    }

    prompt.push_str("创作要求：\n");
    prompt.push_str("1. 保持原作的叙述节奏和语言风格\n");
    prompt.push_str("2. 运用上述技法自然地融入创作中\n");
    prompt.push_str("3. 保持人物对话的特色和氛围营造手法\n");
    prompt.push_str("4. 在描写和叙事中体现原作的美学追求\n");

    prompt
}

/// Generate system prompt from style profile (new approach)
fn generate_system_prompt_from_profile(profile: &serde_json::Value, book_title: &str, author: &str) -> String {
    let mut prompt = String::new();

    prompt.push_str(&format!(
        "你是一位精通模仿{}写作风格的作家助手。\n\n",
        author
    ));

    prompt.push_str(&format!(
        "以下是从《{}》中系统分析得出的写作风格特征，请在创作时严格遵循：\n\n",
        book_title
    ));

    // Vocabulary
    if let Some(vocab) = profile.get("vocabulary") {
        prompt.push_str("【词汇特征】\n");
        if let Some(level) = vocab.get("formality_level").and_then(|v| v.as_str()) {
            let level_label = match level {
                "formal" => "正式",
                "semi_formal" => "半正式",
                "colloquial" => "口语化",
                "mixed" => "混合",
                _ => level,
            };
            prompt.push_str(&format!("• 正式程度: {}\n", level_label));
        }
        if let Some(tendencies) = vocab.get("vocabulary_tendencies").and_then(|v| v.as_str()) {
            prompt.push_str(&format!("• 词汇倾向: {}\n", tendencies));
        }
        if let Some(patterns) = vocab.get("word_choice_patterns").and_then(|v| v.as_str()) {
            prompt.push_str(&format!("• 选词模式: {}\n", patterns));
        }
        prompt.push('\n');
    }

    // Sentence Structure
    if let Some(sentence) = profile.get("sentence_structure") {
        prompt.push_str("【句式结构】\n");
        if let Some(length) = sentence.get("typical_length").and_then(|v| v.as_str()) {
            let length_label = match length {
                "short" => "短句为主",
                "medium" => "中等长度",
                "long" => "长句为主",
                "varied" => "长短变化",
                _ => length,
            };
            prompt.push_str(&format!("• 句子长度: {}\n", length_label));
        }
        if let Some(rhythm) = sentence.get("rhythm_patterns").and_then(|v| v.as_str()) {
            prompt.push_str(&format!("• 节奏模式: {}\n", rhythm));
        }
        if let Some(structure) = sentence.get("paragraph_structure").and_then(|v| v.as_str()) {
            prompt.push_str(&format!("• 段落结构: {}\n", structure));
        }
        prompt.push('\n');
    }

    // Narrative Voice
    if let Some(narrative) = profile.get("narrative_voice") {
        prompt.push_str("【叙事声音】\n");
        if let Some(perspective) = narrative.get("perspective").and_then(|v| v.as_str()) {
            let persp_label = match perspective {
                "first_person" => "第一人称",
                "third_limited" => "第三人称有限",
                "third_omniscient" => "第三人称全知",
                "second_person" => "第二人称",
                "multiple" => "多视角",
                _ => perspective,
            };
            prompt.push_str(&format!("• 叙述视角: {}\n", persp_label));
        }
        if let Some(chars) = narrative.get("narrator_characteristics").and_then(|v| v.as_str()) {
            prompt.push_str(&format!("• 叙述者特征: {}\n", chars));
        }
        prompt.push('\n');
    }

    // Dialogue Style
    if let Some(dialogue) = profile.get("dialogue_style") {
        prompt.push_str("【对话风格】\n");
        if let Some(proportion) = dialogue.get("dialogue_proportion").and_then(|v| v.as_str()) {
            let prop_label = match proportion {
                "low" => "较少",
                "moderate" => "适度",
                "high" => "较多",
                _ => proportion,
            };
            prompt.push_str(&format!("• 对话比例: {}\n", prop_label));
        }
        if let Some(tags) = dialogue.get("dialogue_tags").and_then(|v| v.as_str()) {
            prompt.push_str(&format!("• 对话标签: {}\n", tags));
        }
        if let Some(diff) = dialogue.get("character_voice_differentiation").and_then(|v| v.as_str()) {
            prompt.push_str(&format!("• 角色语言区分: {}\n", diff));
        }
        prompt.push('\n');
    }

    // Description Style
    if let Some(desc) = profile.get("description_style") {
        prompt.push_str("【描写风格】\n");
        if let Some(detail) = desc.get("detail_level").and_then(|v| v.as_str()) {
            let detail_label = match detail {
                "minimal" => "极简",
                "selective" => "精选",
                "detailed" => "详细",
                "exhaustive" => "详尽",
                _ => detail,
            };
            prompt.push_str(&format!("• 细节程度: {}\n", detail_label));
        }
        if let Some(metaphor) = desc.get("metaphor_usage").and_then(|v| v.as_str()) {
            prompt.push_str(&format!("• 比喻使用: {}\n", metaphor));
        }
        if let Some(imagery) = desc.get("imagery_patterns").and_then(|v| v.as_str()) {
            prompt.push_str(&format!("• 意象模式: {}\n", imagery));
        }
        prompt.push('\n');
    }

    // Pacing
    if let Some(pacing) = profile.get("pacing") {
        prompt.push_str("【节奏控制】\n");
        if let Some(tempo) = pacing.get("overall_tempo").and_then(|v| v.as_str()) {
            let tempo_label = match tempo {
                "slow" => "舒缓",
                "moderate" => "中等",
                "fast" => "紧凑",
                "variable" => "变化",
                _ => tempo,
            };
            prompt.push_str(&format!("• 整体节奏: {}\n", tempo_label));
        }
        if let Some(transitions) = pacing.get("scene_transitions").and_then(|v| v.as_str()) {
            prompt.push_str(&format!("• 场景转换: {}\n", transitions));
        }
        if let Some(tension) = pacing.get("tension_building").and_then(|v| v.as_str()) {
            prompt.push_str(&format!("• 张力营造: {}\n", tension));
        }
        prompt.push('\n');
    }

    // Emotional Tone
    if let Some(emotion) = profile.get("emotional_tone") {
        prompt.push_str("【情感基调】\n");
        if let Some(mood) = emotion.get("dominant_mood").and_then(|v| v.as_str()) {
            prompt.push_str(&format!("• 主导情绪: {}\n", mood));
        }
        if let Some(techniques) = emotion.get("atmosphere_techniques").and_then(|v| v.as_str()) {
            prompt.push_str(&format!("• 氛围技法: {}\n", techniques));
        }
        prompt.push('\n');
    }

    // Key Observations
    if let Some(observations) = profile.get("key_observations").and_then(|v| v.as_array()) {
        if !observations.is_empty() {
            prompt.push_str("【关键观察】\n");
            for obs in observations.iter().take(5) {
                if let Some(s) = obs.as_str() {
                    prompt.push_str(&format!("• {}\n", s));
                }
            }
            prompt.push('\n');
        }
    }

    prompt.push_str("创作要求：\n");
    prompt.push_str("1. 保持原作的叙述节奏和语言风格\n");
    prompt.push_str("2. 自然运用上述风格特征\n");
    prompt.push_str("3. 保持人物对话的特色和氛围营造手法\n");
    prompt.push_str("4. 在描写和叙事中体现原作的美学追求\n");

    prompt
}

/// Options for style profile export (simpler, no technique category filters)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleProfileExportOptions {
    pub format: ExportFormat,
    pub example_count: usize,
    pub prompt_template: StylePromptTemplate,
}

impl Default for StyleProfileExportOptions {
    fn default() -> Self {
        Self {
            format: ExportFormat::Markdown,
            example_count: 3,
            prompt_template: StylePromptTemplate::Generic,
        }
    }
}

/// Get count of collected technique cards
#[tauri::command]
pub async fn get_collected_technique_count(book_id: String) -> Result<usize, String> {
    let library = Library::open().map_err(|e| e.to_string())?;
    let bid = BookId::from_string(book_id);
    let book_dir = library.book_dir(&bid);
    let db_path = book_dir.join("book.db");

    if !db_path.exists() {
        return Err("Book database not found".to_string());
    }

    let db = BookDb::open(&db_path, bid)
        .map_err(|e| format!("Failed to open book database: {}", e))?;

    let techniques = db.list_collected_technique_cards().map_err(|e| e.to_string())?;
    Ok(techniques.len())
}

/// Export technique cards as prompt (Tab 1: 技法Prompt)
#[tauri::command]
pub async fn export_technique_prompt(
    book_id: String,
    options: StyleExportOptions,
    output_path: Option<String>,
) -> Result<ExportResult, String> {
    let library = Library::open().map_err(|e| e.to_string())?;
    let bid = BookId::from_string(book_id.clone());
    let book_dir = library.book_dir(&bid);
    let db_path = book_dir.join("book.db");

    if !db_path.exists() {
        return Err("Book database not found".to_string());
    }

    let db = BookDb::open(&db_path, bid.clone())
        .map_err(|e| format!("Failed to open book database: {}", e))?;

    let book_meta = library
        .get_book(&bid)
        .map_err(|e| e.to_string())?
        .ok_or("Book not found")?;

    let author = book_meta.author.as_deref().unwrap_or("未知作者");

    // Only use technique cards (no style profile fallback)
    let styles = collect_style_info(&db, &options)?;
    let content = match options.format {
        ExportFormat::Markdown => {
            generate_technique_markdown(&styles, &book_meta.title, author, &options)?
        }
        ExportFormat::Json => {
            generate_technique_json(&styles, &book_meta.title, author, &options)?
        }
    };

    if let Some(path) = output_path {
        let output_path = PathBuf::from(&path);
        std::fs::write(&output_path, &content).map_err(|e| e.to_string())?;

        Ok(ExportResult {
            success: true,
            file_path: Some(path),
            content: None,
            error: None,
        })
    } else {
        Ok(ExportResult {
            success: true,
            file_path: None,
            content: Some(content),
            error: None,
        })
    }
}

/// Get technique prompt preview (Tab 1: 技法Prompt)
#[tauri::command]
pub async fn get_technique_prompt_preview(
    book_id: String,
    options: StyleExportOptions,
) -> Result<String, String> {
    let library = Library::open().map_err(|e| e.to_string())?;
    let bid = BookId::from_string(book_id);
    let book_dir = library.book_dir(&bid);
    let db_path = book_dir.join("book.db");

    if !db_path.exists() {
        return Err("Book database not found".to_string());
    }

    let db = BookDb::open(&db_path, bid.clone())
        .map_err(|e| format!("Failed to open book database: {}", e))?;

    let book_meta = library
        .get_book(&bid)
        .map_err(|e| e.to_string())?
        .ok_or("Book not found")?;

    let author = book_meta.author.as_deref().unwrap_or("未知作者");

    // Only use technique cards (no style profile fallback)
    let styles = collect_style_info(&db, &options)?;
    match options.format {
        ExportFormat::Markdown => {
            generate_technique_markdown(&styles, &book_meta.title, author, &options)
        }
        ExportFormat::Json => {
            generate_technique_json(&styles, &book_meta.title, author, &options)
        }
    }
}

/// Export style profile as prompt (Tab 2: 风格档案)
#[tauri::command]
pub async fn export_style_profile_prompt(
    book_id: String,
    options: StyleProfileExportOptions,
    output_path: Option<String>,
) -> Result<ExportResult, String> {
    let library = Library::open().map_err(|e| e.to_string())?;
    let bid = BookId::from_string(book_id.clone());
    let book_dir = library.book_dir(&bid);
    let db_path = book_dir.join("book.db");

    if !db_path.exists() {
        return Err("Book database not found".to_string());
    }

    let db = BookDb::open(&db_path, bid.clone())
        .map_err(|e| format!("Failed to open book database: {}", e))?;

    let book_meta = library
        .get_book(&bid)
        .map_err(|e| e.to_string())?
        .ok_or("Book not found")?;

    let author = book_meta.author.as_deref().unwrap_or("未知作者");

    // Only use style profile (no technique cards fallback)
    let profile = db.get_style_profile().map_err(|e| e.to_string())?;

    let content = if let Some(p) = profile {
        match options.format {
            ExportFormat::Markdown => {
                generate_style_profile_markdown(&p.profile_json, &book_meta.title, author, &options)?
            }
            ExportFormat::Json => {
                generate_style_profile_json(&p.profile_json, &book_meta.title, author, &options)?
            }
        }
    } else {
        return Err("No style profile found. Please run style analysis first.".to_string());
    };

    if let Some(path) = output_path {
        let output_path = PathBuf::from(&path);
        std::fs::write(&output_path, &content).map_err(|e| e.to_string())?;

        Ok(ExportResult {
            success: true,
            file_path: Some(path),
            content: None,
            error: None,
        })
    } else {
        Ok(ExportResult {
            success: true,
            file_path: None,
            content: Some(content),
            error: None,
        })
    }
}

/// Get style profile prompt preview (Tab 2: 风格档案)
#[tauri::command]
pub async fn get_style_profile_prompt_preview(
    book_id: String,
    options: StyleProfileExportOptions,
) -> Result<String, String> {
    let library = Library::open().map_err(|e| e.to_string())?;
    let bid = BookId::from_string(book_id);
    let book_dir = library.book_dir(&bid);
    let db_path = book_dir.join("book.db");

    if !db_path.exists() {
        return Err("Book database not found".to_string());
    }

    let db = BookDb::open(&db_path, bid.clone())
        .map_err(|e| format!("Failed to open book database: {}", e))?;

    let book_meta = library
        .get_book(&bid)
        .map_err(|e| e.to_string())?
        .ok_or("Book not found")?;

    let author = book_meta.author.as_deref().unwrap_or("未知作者");

    // Only use style profile (no technique cards fallback)
    let profile = db.get_style_profile().map_err(|e| e.to_string())?;

    if let Some(p) = profile {
        match options.format {
            ExportFormat::Markdown => {
                generate_style_profile_markdown(&p.profile_json, &book_meta.title, author, &options)
            }
            ExportFormat::Json => {
                generate_style_profile_json(&p.profile_json, &book_meta.title, author, &options)
            }
        }
    } else {
        Err("No style profile found. Please run style analysis first.".to_string())
    }
}

/// Export style profile as JSON file (raw profile data)
#[tauri::command]
pub async fn export_style_profile_json(
    book_id: String,
    output_path: Option<String>,
) -> Result<ExportResult, String> {
    let library = Library::open().map_err(|e| e.to_string())?;
    let bid = BookId::from_string(book_id.clone());
    let book_dir = library.book_dir(&bid);
    let db_path = book_dir.join("book.db");

    if !db_path.exists() {
        return Err("Book database not found".to_string());
    }

    let db = BookDb::open(&db_path, bid.clone())
        .map_err(|e| format!("Failed to open book database: {}", e))?;

    let book_meta = library
        .get_book(&bid)
        .map_err(|e| e.to_string())?
        .ok_or("Book not found")?;

    let profile = db.get_style_profile().map_err(|e| e.to_string())?;

    let export_data = if let Some(p) = profile {
        serde_json::json!({
            "version": "1.0",
            "source": {
                "book_title": book_meta.title,
                "author": book_meta.author,
                "analyzed_chapters": p.analyzed_chapters,
            },
            "exported_at": chrono::Local::now().to_rfc3339(),
            "profile": p.profile_json,
        })
    } else {
        return Err("No style profile found. Please run style analysis first.".to_string());
    };

    let content = serde_json::to_string_pretty(&export_data).map_err(|e| e.to_string())?;

    if let Some(path) = output_path {
        let output_path = PathBuf::from(&path);
        std::fs::write(&output_path, &content).map_err(|e| e.to_string())?;

        Ok(ExportResult {
            success: true,
            file_path: Some(path),
            content: None,
            error: None,
        })
    } else {
        Ok(ExportResult {
            success: true,
            file_path: None,
            content: Some(content),
            error: None,
        })
    }
}
