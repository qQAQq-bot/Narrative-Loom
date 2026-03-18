# Narrative Loom - Implementation Checklist

> 基于 PRD v3.1 的详细实施任务清单
> 
> **核心定位**：学习写作 + 知识积累 + RAG 增强分析
> **技术栈**：Tauri 2.0 + Vue 3 + TailwindCSS + Rust + Python Sidecar（向量检索当前使用 SQLite+BLOB + Rust 余弦计算；sqlite-vec 集成暂缓）

---

## 总览

| Phase | Duration | Focus | Tasks |
|-------|----------|-------|-------|
| Phase 0 | 2-3 days | 项目初始化 + Provider/Agent 配置 | 29 |
| **Phase 0.5** | 3-4 days | **RAG 基础设施** | **15** |
| Phase 1 | 1 week | 书库与导入 | 25 |
| **Phase 1.5** | 2 days | **导入时 Embedding 生成** | **6** |
| Phase 2 | 1.5 weeks | 章节工作台 + Agent 系统 | 45 |
| **Phase 2.5** | 3 days | **分析时上下文检索** | **8** |
| Phase 3 | 1.5 weeks | 收件箱与故事圣经 | 30 |
| **Phase 3.5** | 2 days | **故事圣经 Embedding 同步** | **5** |
| Phase 4 | 1 week | 完善与优化 + Settings UI | 35 |
| **Phase 4.5** | 2 days | **高级检索功能** | **6** |
| **Total** | **~7.5 weeks** | | **~204 tasks** |

---

## Phase 0: Project Setup (2 days)

### 0.1 Tauri + Vue 初始化

- [x] **P0-001**: 使用 `pnpm create tauri-app` 初始化项目（选择 Vue + TypeScript）
- [x] **P0-002**: 升级到 Tauri 2.0（如需要）
- [x] **P0-003**: 配置 TypeScript strict mode
- [x] **P0-004**: 配置 ESLint + Prettier
- [x] **P0-005**: 安装 Vue 依赖 (vue-router@4, pinia@2)

### 0.2 TailwindCSS 配置

- [x] **P0-010**: 安装 TailwindCSS + PostCSS + Autoprefixer
- [x] **P0-011**: 创建 tailwind.config.js（主题色、字体）
- [x] **P0-012**: 创建 postcss.config.js
- [x] **P0-013**: 配置 src/style.css（Tailwind imports）
- [x] **P0-014**: 安装 @tailwindcss/typography + @tailwindcss/forms
- [ ] **P0-015**: 安装中文字体（Noto Serif SC）

### 0.3 Rust 骨架

- [x] **P0-020**: 配置 src-tauri/Cargo.toml 依赖
- [x] **P0-021**: 创建 Rust 模块目录结构
- [x] **P0-022**: 配置 tracing 日志
- [x] **P0-023**: 添加 sqlx + rusqlite 依赖

### 0.4 Python Sidecar 骨架

- [x] **P0-030**: 创建 python/ 目录结构
- [x] **P0-031**: 创建 pyproject.toml
- [x] **P0-032**: 创建 JSON-RPC 服务器骨架 (server.py)
- [x] **P0-033**: 创建 __main__.py 入口

### 0.5 开发环境

- [ ] **P0-040**: 创建 scripts/dev-setup.sh
- [ ] **P0-041**: 创建 .env.example
- [x] **P0-042**: 配置 .gitignore
- [x] **P0-043**: 验证 `pnpm tauri dev` 可运行

### 0.6 Provider 与 Agent 配置 (Rust)

- [x] **P0-050**: 实现 `core/provider.rs` (ProviderConfig, ApiFormat)
- [x] **P0-051**: 实现 `core/agent.rs` (AgentConfig, AgentKind, TaskType, TaskBindings)
- [x] **P0-052**: 实现 `storage/keychain.rs` (OS Keychain 集成 - API Key 安全存储)
- [x] **P0-053**: 实现 `storage/config.rs` (Provider/Agent 配置持久化)
- [x] **P0-054**: 创建默认 Provider 配置 (OpenAI, DeepSeek, Ollama)
- [x] **P0-055**: 创建默认 Agent 配置 (5 个内置 Agent)
- [x] **P0-056**: 实现 `commands/settings.rs::get_providers()`
- [x] **P0-057**: 实现 `commands/settings.rs::save_provider()`
- [x] **P0-058**: 实现 `commands/settings.rs::delete_provider()`
- [x] **P0-059**: 实现 `commands/settings.rs::test_provider_connection()`
- [x] **P0-060**: 实现 `commands/settings.rs::get_agents()`
- [x] **P0-061**: 实现 `commands/settings.rs::save_agent()`
- [x] **P0-062**: 实现 `commands/settings.rs::get_task_bindings()`
- [x] **P0-063**: 实现 `commands/settings.rs::save_task_bindings()`

---

## Phase 0.5: RAG 基础设施 (3-4 days)

> **目标**：搭建 RAG 所需的向量存储和 Embedding 生成基础设施

### 0.5.1 Rust 核心类型

- [ ] **P0.5-001**: (Deferred) 添加 `sqlite-vec` 依赖到 Cargo.toml（当前未启用 sqlite-vec）
- [x] **P0.5-002**: 创建 `core/embedding.rs` (Chunk, ChunkId, ChunkType, ChunkMetadata)
- [x] **P0.5-003**: 创建 `core/embedding.rs` (VectorEntry, EmbeddingConfig, EmbeddingProvider)

### 0.5.2 向量存储层 (Rust)

- [x] **P0.5-004**: 创建 `vectors.db` schema（实现已内联在 `src-tauri/src/storage/vectors.rs`）
- [x] **P0.5-005**: 实现 `storage/vectors.rs::init_vector_db()` (初始化 vectors.db)
- [x] **P0.5-006**: 实现 `storage/vectors.rs::insert_chunks()` (批量插入文本块)
- [x] **P0.5-007**: 实现 `storage/vectors.rs::insert_vectors()` (批量插入向量)
- [x] **P0.5-008**: 实现 `storage/vectors.rs::search_similar()` (向量相似度搜索)
- [x] **P0.5-009**: 实现 `storage/vectors.rs::update_chunk()` (更新单个块)
- [x] **P0.5-010**: 实现 `storage/vectors.rs::delete_chunks_by_chapter()` (按章节删除)

### 0.5.3 Python Embedding 模块

- [x] **P0.5-011**: 创建 `python/loom/embedding/__init__.py`
- [x] **P0.5-012**: 创建 `python/loom/embedding/base.py` (EmbeddingProvider 基类)
- [x] **P0.5-013**: 创建 `python/loom/embedding/local.py` (bge-m3 via sentence-transformers)
- [x] **P0.5-014**: 创建 `python/loom/embedding/openai.py` (text-embedding-3-small)
- [x] **P0.5-015**: 创建 `python/loom/embedding/chunker.py` (中文段落分块逻辑)

### 0.5.4 Python JSON-RPC 扩展

- [x] **P0.5-016**: 更新 `server.py` 添加 `chunk_text()` RPC 方法
- [x] **P0.5-017**: 更新 `server.py` 添加 `generate_embedding()` RPC 方法
- [x] **P0.5-018**: 更新 `server.py` 添加 `generate_embeddings()` RPC 方法 (批量)

### 0.5.5 Rust Commands

- [x] **P0.5-019**: 创建 `commands/embedding.rs`
- [x] **P0.5-020**: 实现 `commands/embedding.rs::generate_chapter_embeddings()`
- [x] **P0.5-021**: 实现 `commands/embedding.rs::rebuild_book_embeddings()`

---

## Phase 1: 书库与导入 (1 week)

### 1.1 核心类型定义 (Rust)

- [x] **P1-001**: 实现 `core/ids.rs` (BookId, ChapterId, CardId, EntityId)
- [x] **P1-002**: 实现 `core/book.rs` (Book, Chapter, BookStatus)
- [x] **P1-003**: 实现 `core/card.rs` (TechniqueCard, KnowledgeCard)
- [x] **P1-004**: 实现 `core/bible.rs` (Character, Setting, Event, TimelineEvent)
- [x] **P1-005**: 实现 `core/evidence.rs` (Evidence)
- [x] **P1-006**: 实现 `core/error.rs` (AppError)

### 1.2 存储层 (Rust)

- [x] **P1-010**: 创建 `storage/schema.sql`（书籍元数据、章节、卡片、圣经表）
- [x] **P1-011**: 实现 `storage/library.rs`（书库索引管理）
- [x] **P1-012**: 实现 `storage/book_db.rs`（单书 SQLite 连接）
- [x] **P1-013**: 实现 `storage/chapters.rs`（章节文件读写）
- [x] **P1-014**: 实现 `storage/migrations.rs`

### 1.3 导入功能 (Rust)

- [x] **P1-020**: 实现 `ingestion/mod.rs`（导入入口）
- [x] **P1-021**: 实现 `ingestion/parser.rs`（TXT 解析 + 编码检测）
- [x] **P1-022**: 实现 `ingestion/epub_parser.rs`（EPUB 解析）
- [x] **P1-023**: 实现 `ingestion/segmentation.rs`（章节分割）
- [x] **P1-024**: 实现封面提取（集成在 EPUB 解析中）
- [x] **P1-025**: 实现编码检测（UTF-8, GBK, GB18030, UTF-16）

### 1.4 书库 Commands (Rust)

- [x] **P1-030**: 实现 `commands/library.rs::list_books()`
- [x] **P1-031**: 实现 `commands/library.rs::import_book()` + `preview_book_import()`
- [x] **P1-032**: 实现 `commands/library.rs::delete_book()`
- [x] **P1-033**: 实现 `commands/library.rs::get_book()` + `get_book_chapters()` + `get_chapter_content()`
- [x] **P1-034**: 注册所有 commands 到 Tauri (lib.rs 已更新)

### 1.5 书库 UI (Vue + TailwindCSS)

- [x] **P1-040**: 创建基础 UI 组件 (Button, Modal, Badge)
- [x] **P1-041**: 创建 `components/library/BookShelf.vue`（书架网格）
- [x] **P1-042**: 创建 `components/library/BookCard.vue`（书籍卡片）
- [x] **P1-043**: 创建 `components/library/ImportModal.vue`（导入对话框）
- [x] **P1-044**: BookCover 功能已集成到 BookCard.vue
- [x] **P1-045**: 更新 `views/LibraryView.vue`（书库页面）
- [x] **P1-046**: 创建 `stores/library.ts`（书库状态）
- [x] **P1-047**: 创建 `composables/useTauri.ts`（IPC 封装）
- [x] **P1-048**: Vue Router 已配置（/ → LibraryView）

---

## Phase 1.5: 导入时 Embedding 生成 (2 days)

> **目标**：书籍导入时自动生成章节 Embedding，存入 vectors.db

### 1.5.1 导入流程集成

- [x] **P1.5-001**: 修改 `ingestion/mod.rs` 添加 Embedding 生成步骤
- [x] **P1.5-002**: 实现 `ingestion/embedding_task.rs` (Embedding 任务处理)
- [x] **P1.5-003**: 实现 chunk_text_for_embedding 分块逻辑
- [x] **P1.5-004**: 实现 Embedding 生成错误处理 (warn 并继续)

### 1.5.2 测试与验证

- [ ] **P1.5-005**: 测试：导入 10 万字小说，验证 Embedding 生成
- [ ] **P1.5-006**: 测试：验证 vectors.db 数据完整性

---

## Phase 2: 章节工作台 (1.5 weeks)

### 2.1 章节读取 (Rust)

- [x] **P2-001**: 实现 `commands/chapter.rs::get_chapters()`
- [x] **P2-002**: 实现 `commands/chapter.rs::get_chapter()` + `get_chapter_by_index()`
- [x] **P2-003**: 实现 `get_adjacent_chapters()` 获取前后章节

### 2.2 Python Sidecar 通信 (Rust)

- [x] **P2-010**: 实现 `sidecar/manager.rs`（进程启动/停止/重启）
- [x] **P2-011**: 实现 `sidecar/protocol.rs`（JSON-RPC 协议）
- [x] **P2-012**: RpcRequest/RpcResponse/RpcError 类型定义
- [x] **P2-013**: 实现健康检查 `health_check()`
- [x] **P2-014**: 实现自动重启机制

### 2.3 Python LLM 集成

- [x] **P2-020**: 实现 `providers/base.py`（LLM 基类）
- [x] **P2-021**: 实现 `providers/openai.py`
- [x] **P2-022**: 实现重试和错误处理
- [x] **P2-023**: 实现结构化输出解析 `complete_json()`

### 2.4 Python 分析逻辑

- [x] **P2-030**: 实现 `schemas/card.py`（Pydantic 模型）
- [x] **P2-031**: 技法分析逻辑在 `agents/technique.py` 中
- [x] **P2-032**: 技法分析 Prompt 模板 (TECHNIQUE_SYSTEM_PROMPT)
- [x] **P2-033**: 人物提取在 `agents/character.py` 中
- [x] **P2-034**: 设定提取在 `agents/setting.py` 中
- [x] **P2-035**: 事件提取在 `agents/event.py` 中
- [x] **P2-036**: 知识提取 Prompt 模板 (各 Agent 内置)

### 2.5 Python Agent 系统

- [x] **P2-037**: 实现 `agents/base.py` (BaseAgent 基类)
- [x] **P2-038**: 实现 `agents/technique.py` (TechniqueAgent)
- [x] **P2-039**: 实现 `agents/character.py` (CharacterAgent)
- [x] **P2-040**: 实现 `agents/setting.py` (SettingAgent)
- [x] **P2-041**: 实现 `agents/event.py` (EventAgent)
- [x] **P2-042**: （deprecated）TimelineAgent 已移除；时间信息并入 EventAgent 输出字段，并在蓝图时间线视图展示
- [x] **P2-043**: Agent 工厂 `create_agent()` 在 `agents/__init__.py`
- [x] **P2-044**: ChatCompletions API 调用器 (在 OpenAIProvider)
- [x] **P2-045**: Responses API 调用器
- [x] **P2-046**: 结构化输出解析 (JSON Schema mode)
- [x] **P2-047**: Agent 错误处理和重试逻辑 (在 OpenAIProvider)

### 2.6 分析 Commands (Rust)

- [x] **P2-048**: 实现 `commands/analysis.rs::analyze_chapter()`
- [x] **P2-049**: 实现分析进度事件发送
- [x] **P2-050**: 实现卡片读取逻辑
- [x] **P2-051**: 实现 `commands/analysis.rs::get_technique_cards()`
- [x] **P2-052**: 实现 `commands/analysis.rs::get_knowledge_cards()`

### 2.7 书籍导航 UI

- [x] **P2-053**: 创建 `views/BookView.vue`（书籍容器 + 侧边栏）
- [x] **P2-054**: 创建 `components/common/Sidebar.vue`（导航栏）
- [x] **P2-055**: 创建 `components/reader/ChapterNav.vue`（章节列表）
- [x] **P2-056**: 创建 `stores/book.ts`（当前书籍状态）
- [x] **P2-057**: 配置路由 /book/:id

### 2.8 章节阅读器 UI

- [x] **P2-058**: 创建 `views/ChapterView.vue`（章节工作台）
- [x] **P2-059**: 创建 `components/reader/ChapterReader.vue`（原文阅读区）
- [x] **P2-060**: 创建 `components/reader/TextHighlight.vue`（文本高亮）
- [x] **P2-061**: 创建 `components/reader/EvidenceLink.vue`（证据链接）

### 2.9 卡片展示 UI

- [x] **P2-062**: 创建 `components/cards/TechniqueCard.vue`（技法卡片）
- [x] **P2-063**: 创建 `components/cards/KnowledgeCard.vue`（知识卡片）
- [x] **P2-064**: 实现卡片栏（技法/知识 Tab 切换）
- [x] **P2-065**: 实现"收藏到技法库"功能
- [x] **P2-066**: 实现"送入收件箱/直接入库"功能
- [x] **P2-067**: 创建 `stores/analysis.ts`（分析状态）

---

## Phase 2.5: 分析时上下文检索 (3 days)

> **目标**：分析章节时通过 RAG 检索相关历史上下文，增强 LLM 分析质量

### 2.5.1 上下文构建器 (Rust)

- [x] **P2.5-001**: 创建 `retrieval/context_builder.rs`
- [x] **P2.5-002**: 实现 `build_analysis_context()` (构建分析上下文)
- [x] **P2.5-003**: 实现 `extract_entity_mentions()` (从章节提取实体提及)
- [x] **P2.5-004**: 实现多路检索逻辑 (人物/设定/事件/相似段落)

### 2.5.2 向量检索 (Rust)

- [x] **P2.5-005**: 创建 `retrieval/vector_search.rs`
- [x] **P2.5-006**: 实现 `search_by_text()` (文本查询)
- [x] **P2.5-007**: 实现 `search_by_entities()` (实体历史检索)

### 2.5.3 Python 分析集成

- [x] **P2.5-008**: 修改 Python `analysis/*` 模块接收增强上下文
- [x] **P2.5-009**: 修改 Prompt 模板支持检索上下文 (已知人物/设定/事件)
- [x] **P2.5-010**: 实现上下文 Token 控制 (≤8K tokens)

### 2.5.4 测试与验证

- [ ] **P2.5-011**: 测试：分析第 100 章时能检索到前文人物信息
- [ ] **P2.5-012**: 测试：验证跨章节实体识别准确性

---

## Phase 3: 收件箱与故事圣经 (1.5 weeks)

### 3.1 收件箱 Commands (Rust)

- [x] **P3-001**: 实现 `commands/inbox.rs::get_inbox()`
- [x] **P3-002**: 实现 `commands/inbox.rs::accept_card()`
- [x] **P3-003**: 实现 `commands/inbox.rs::reject_card()`
- [x] **P3-004**: 实现 `commands/inbox.rs::merge_card()`
- [x] **P3-005**: 实现批量操作

### 3.2 收件箱 UI

- [x] **P3-010**: 创建 `views/InboxView.vue`（收件箱页面）
- [x] **P3-011**: 创建 `components/inbox/InboxList.vue`（待审核列表）- 集成到 InboxView
- [x] **P3-012**: 创建 `components/inbox/ReviewCard.vue`（审核卡片）- 集成到 InboxView
- [x] **P3-013**: 创建 `components/inbox/BatchActions.vue`（批量操作）- 集成到 InboxView
- [x] **P3-014**: 实现筛选功能（人物/设定/事件/全部）
- [x] **P3-015**: 实现编辑后接受功能
- [x] **P3-016**: 实现合并到现有条目功能

### 3.3 故事圣经 Commands (Rust)

- [x] **P3-020**: 实现 `commands/bible.rs::get_characters()`
- [x] **P3-021**: 实现 `commands/bible.rs::get_settings()`
- [x] **P3-022**: 实现 `commands/bible.rs::get_events()`
- [ ] **P3-023**: （deprecated）独立 `get_timeline()` 已移除；时间信息统一存储在 `events` 的 time_* 字段，并在蓝图视图中展示
- [x] **P3-024**: 实现 `commands/bible.rs::update_entity()` - update_character()
- [x] **P3-025**: 实现 `commands/bible.rs::delete_entity()`

### 3.4 故事圣经 UI

- [x] **P3-030**: 创建 `views/BibleView.vue`（故事圣经页面）
- [x] **P3-031**: 创建 `components/bible/CharacterList.vue`（人物列表）- 集成到 BibleView
- [x] **P3-032**: 创建 `components/cards/CharacterCard.vue`（人物详情卡）- 集成到 BibleView
- [x] **P3-033**: 创建 `components/bible/SettingList.vue`（设定列表）- 集成到 BibleView
- [x] **P3-034**: 创建 `components/cards/SettingCard.vue`（设定详情卡）- 集成到 BibleView
- [x] **P3-035**: 创建 `components/bible/EventList.vue`（事件列表）- 集成到 BibleView
- [x] **P3-036**: 创建 `components/cards/EventCard.vue`（事件详情卡）- 集成到 BibleView
- [ ] **P3-037**: （deprecated）独立 Timeline 视图不再维护；使用 `components/blueprint/BlueprintTimeline.vue` 作为时间线视图
- [x] **P3-038**: 创建 `components/bible/BibleEditor.vue`（条目编辑器）
- [x] **P3-039**: 实现 Tab 切换（人物/设定/事件/蓝图）

### 3.5 证据查看

- [x] **P3-040**: 实现点击证据跳转到原文
- [x] **P3-041**: 实现原文高亮显示证据范围
- [x] **P3-042**: 创建证据弹窗组件

---

## Phase 3.5: 故事圣经 Embedding 同步 (2 days)

> **目标**：故事圣经条目变更时自动更新对应的 Embedding

### 3.5.1 实体 Embedding 管理

- [x] **P3.5-001**: 实现人物条目变更时更新 Embedding (chunk_type=entity)
- [x] **P3.5-002**: 实现设定条目变更时更新 Embedding
- [x] **P3.5-003**: 实现事件条目变更时更新 Embedding

### 3.5.2 增量更新

- [x] **P3.5-004**: 实现增量更新逻辑 (只更新变更的 chunk)
- [x] **P3.5-005**: 实现 `rebuild_entity_embeddings()` 全量重建

---

## Phase 4: 完善与优化 (1 week)

### 4.1 技法库

- [x] **P4-001**: 创建 `views/TechniqueView.vue`（技法库页面）
- [x] **P4-002**: 实现收藏的技法浏览
- [x] **P4-003**: 实现按类型筛选
- [x] **P4-004**: 实现 `commands/analysis.rs::collect_technique()`

### 4.2 搜索功能

- [x] **P4-010**: 实现 `commands/search.rs::search()`
- [x] **P4-011**: 实现全文搜索（章节内容）
- [x] **P4-012**: 实现实体搜索（人物/设定/事件）
- [x] **P4-013**: 创建 `views/SearchView.vue`（搜索结果页）
- [x] **P4-014**: 创建 `components/common/SearchBar.vue`

### 4.3 导出功能

- [x] **P4-020**: 实现 `commands/export.rs::export_bible()`
- [x] **P4-021**: 实现 Markdown 导出
- [x] **P4-022**: 实现 JSON 导出
- [x] **P4-023**: 创建导出对话框

### 4.4 设置页面

- [x] **P4-030**: 创建 `views/SettingsView.vue`
- [x] **P4-031**: 实现 LLM 配置（API Key, Model）
- [x] **P4-032**: 实现主题切换（浅色/深色）
- [x] **P4-033**: 实现书库路径配置

### 4.5 Provider 设置 UI

- [x] **P4-034**: 创建 `components/settings/ProviderList.vue` (Provider 列表)
- [x] **P4-035**: 创建 `components/settings/ProviderCard.vue` (Provider 卡片)
- [x] **P4-036**: 创建 `components/settings/ProviderEditModal.vue` (Provider 编辑对话框)
- [x] **P4-037**: 实现 Provider 添加/编辑/删除
- [x] **P4-038**: 实现 Provider 连接测试
- [x] **P4-039**: 实现 API Key 安全输入和存储
- [x] **P4-040**: 创建 `composables/useProviders.ts`

### 4.6 Agent 设置 UI

- [x] **P4-041**: 创建 `components/settings/AgentList.vue` (Agent 列表)
- [x] **P4-042**: 创建 `components/settings/AgentCard.vue` (Agent 卡片)
- [x] **P4-043**: 创建 `components/settings/AgentEditModal.vue` (Agent 编辑对话框)
- [x] **P4-044**: 创建 `components/settings/TaskBindings.vue` (任务绑定表格)
- [x] **P4-045**: 实现 Agent 添加/编辑/删除
- [x] **P4-046**: 实现任务 → Agent 绑定配置
- [x] **P4-047**: 实现默认提示词查看 (AgentCard 显示"使用内置提示词"或显示自定义提示词)
- [x] **P4-048**: Agent 操作已在 settings store 中实现 (无需单独 composable)

### 4.7 其他 LLM 提供者

- [x] **P4-049**: 实现 `providers/anthropic.py`
- [x] **P4-050**: 实现 `providers/ollama.py`
- [x] **P4-051**: 实现 provider 切换逻辑 (create_provider 工厂函数)

### 4.8 性能优化

- [x] **P4-052**: 实现虚拟滚动（长列表）
- [x] **P4-053**: 优化章节加载速度
- [x] **P4-054**: 优化 SQLite 查询（索引）
- [x] **P4-055**: 内存使用优化

### 4.9 错误处理与 UX

- [x] **P4-056**: 完善错误消息展示
- [x] **P4-057**: 添加 loading 状态
- [x] **P4-058**: 添加空状态提示
- [x] **P4-059**: 添加操作确认对话框

### 4.10 打包分发

- [ ] **P4-060**: 配置 PyInstaller 打包 Python
- [ ] **P4-061**: 配置 Tauri bundler
- [ ] **P4-062**: 创建 Windows 安装程序
- [ ] **P4-063**: 创建 macOS 安装程序
- [ ] **P4-064**: 测试安装包

---

## Phase 4.5: 高级检索功能 (2 days)

> **目标**：实现混合搜索、语义搜索等高级 RAG 功能

### 4.5.1 混合搜索

- [x] **P4.5-001**: 实现 `retrieval/vector_search.rs::hybrid_search()` (向量 + 关键词)
- [x] **P4.5-002**: 实现 RRF (Reciprocal Rank Fusion) 结果合并

### 4.5.2 语义搜索 UI

- [x] **P4.5-003**: 实现 `commands/search.rs::semantic_search()` (语义搜索 Command)
- [x] **P4.5-004**: 更新 SearchView 支持语义搜索模式切换
- [x] **P4.5-005**: 创建语义搜索结果展示组件

### 4.5.3 高级功能

- [x] **P4.5-006**: 实现伏笔检测功能 (相似描述匹配)

---

## 依赖关系图

```
Phase 0 (Setup)
    │
    ├── P0-001..043 (Tauri/Vue/Rust/Python 骨架)
    │
    └── P0-050..063 (Provider/Agent 配置)
                │
                ▼
Phase 0.5 (RAG 基础设施) ◄── 新增
    │
    ├── P0.5-001..010 (Rust 向量存储)
    │
    └── P0.5-011..021 (Python Embedding)
                │
                ▼
Phase 1 (书库与导入)
    │
    ├── P1-001..006 (Core Types) ──┐
    │                               │
    ├── P1-010..014 (Storage) ─────┤
    │                               │
    ├── P1-020..025 (Import) ──────┼──► Phase 1.5 (导入时 Embedding) ◄── 新增
    │                               │              │
    └── P1-040..048 (Library UI) ──┘              │
                                                   ▼
                                          Phase 2 (章节工作台)
                                                   │
                                                   ├── P2-010..014 (Sidecar)
                                                   │
                                                   ├── P2-020..047 (Python Analysis + Agent)
                                                   │
                                                   └── P2-053..067 (Chapter UI)
                                                            │
                                                            ▼
                                                   Phase 2.5 (上下文检索) ◄── 新增
                                                            │
                                                            ▼
                                                   Phase 3 (收件箱与圣经)
                                                            │
                                                            ▼
                                                   Phase 3.5 (圣经 Embedding 同步) ◄── 新增
                                                            │
                                                            ▼
                                                   Phase 4 (完善与优化)
                                                            │
                                                            ├── P4-034..048 (Settings UI)
                                                            │
                                                            └── P4-060..064 (打包分发)
                                                                     │
                                                                     ▼
                                                            Phase 4.5 (高级检索) ◄── 新增
```

---

## 关键路径

```
P0-001 (Init Tauri)
    │
    ├──► P0-050..063 (Provider/Agent 配置)
    │
    ▼
P0.5-001..021 (RAG 基础设施) ◄── 新增关键路径
    │
    ▼
P1-001..006 (Core Types)
    │
    ▼
P1-010..014 (Storage)
    │
    ├──► P1-020..025 (Import) ──► P1.5-001..006 (导入 Embedding) ──► P1-040..048 (Library UI)
    │
    └──► P2-010..014 (Sidecar) ──► P2-020..047 (Python + Agents) ──► P2-048..052 (Analysis)
                                                                          │
                                                                          ▼
                                                                  P2.5-001..012 (上下文检索) ◄── 新增
                                                                          │
                                                                          ▼
                                                                  P2-053..067 (Chapter UI)
                                                                          │
                                                                          ▼
                                                                  P3-001..042 (Inbox + Bible)
                                                                          │
                                                                          ▼
                                                                  P3.5-001..005 (圣经 Embedding) ◄── 新增
                                                                          │
                                                                          ▼
                                                                  P4-001..064 (Polish + Settings)
                                                                          │
                                                                          ▼
                                                                  P4.5-001..006 (高级检索) ◄── 新增
```

---

## 快速开始命令

```bash
# 1. 安装依赖
npm install

# 2. 开发模式
npm run tauri -- dev

# 3. 构建生产版本
npm run tauri -- build

# 4. Python sidecar 开发
cd python && pip install -e .
python -m loom
```

---

## 验收标准

### Phase 0 验收
- [x] Tauri + Vue + TailwindCSS 项目可运行
- [x] Provider 配置可以保存和加载
- [x] Agent 配置可以保存和加载
- [x] API Key 可以安全存储到系统密钥链

### Phase 0.5 验收 (RAG 基础设施)
- [ ] sqlite-vec 扩展正常加载（deferred；当前实现使用 BLOB embeddings + Rust cosine）
- [x] vectors.db 可以创建和初始化
- [x] Python 可以生成 Embedding (本地 bge-m3 或 OpenAI API)
- [x] 向量相似度搜索返回正确结果

### Phase 1 验收
- [x] 可以在书库看到空状态提示
- [x] 可以导入 txt 文件并显示在书架
- [x] 可以看到书籍封面、标题、章节数
- [x] 可以删除书籍

### Phase 1.5 验收 (导入 Embedding)
- [x] 导入书籍时自动生成 Embedding
- [ ] 导入进度显示 Embedding 生成状态 (可选)
- [x] vectors.db 包含所有章节的文本块

### Phase 2 验收
- [x] 可以进入书籍，看到章节列表
- [x] 可以阅读章节内容
- [x] 可以触发分析，看到加载状态
- [x] 分析使用配置的 Agent 和 Provider
- [x] 分析完成后显示技法卡和知识卡
- [x] 卡片显示证据引用

### Phase 2.5 验收 (上下文检索)
- [x] 分析章节时自动检索相关历史上下文
- [x] LLM 分析 Prompt 包含检索到的人物/设定/事件信息
- [ ] 分析第 100 章时能正确识别前文出现的人物

### Phase 3 验收
- [x] 知识卡可以送入收件箱
- [x] 收件箱可以审核、接受、驳回
- [x] 接受后条目出现在故事圣经
- [ ] 可以编辑故事圣经条目
- [ ] 可以点击证据跳转到原文

### Phase 3.5 验收 (圣经 Embedding 同步)
- [ ] 编辑人物条目后 Embedding 自动更新
- [ ] 编辑设定条目后 Embedding 自动更新
- [ ] 后续分析能检索到更新后的实体描述

### Phase 4 验收
- [ ] 搜索功能正常
- [ ] 可以导出故事圣经为 Markdown
- [ ] 可以在设置中配置 Provider
- [ ] 可以在设置中配置 Agent 和任务绑定
- [ ] 可以切换 LLM 提供者
- [ ] 安装包可以正常安装和运行

### Phase 4.5 验收 (高级检索)
- [x] 语义搜索返回语义相关结果（不仅是关键词匹配）
- [x] 混合搜索结合向量和关键词结果
- [x] 伏笔检测能找到相似描述的段落对

---

*Last Updated: 2026-01-19*
