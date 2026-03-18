# Narrative Loom - Product Requirements Document

> 把一本书拆成可学习的写作技法与可维护的故事圣经

**Version**: 3.1.0  
**Created**: 2026-01-14  
**Updated**: 2026-01-15  
**Status**: Approved

---

## Table of Contents

1. [Overview](#1-overview)
2. [Goals & Non-Goals](#2-goals--non-goals)
3. [System Architecture](#3-system-architecture)
4. [Project Structure](#4-project-structure)
5. [Core Data Models](#5-core-data-models)
6. [User Interface Design](#6-user-interface-design)
7. [Core Workflows](#7-core-workflows)
8. [Module Specifications](#8-module-specifications)
9. [IPC Protocol](#9-ipc-protocol)
10. [Configuration](#10-configuration)
11. [Implementation Phases](#11-implementation-phases)
12. [RAG Architecture](#12-rag-architecture-retrieval-augmented-generation)
13. [Technical Considerations](#13-technical-considerations)
14. [Success Metrics](#14-success-metrics)

---

## 1. Overview

### 1.1 Problem Statement

写作学习者面临两个核心挑战：
1. **技法学习**：读了很多书，却难以系统性地理解作者的写作技法
2. **知识积累**：长篇小说的世界观、人物、情节错综复杂，难以整理和维护

### 1.2 Solution

**Narrative Loom** 是一个 AI 驱动的小说解析与学习系统：

> **「读一本书，学会它的写法，并把它的世界整理成你的故事圣经。」**

核心价值：
- **边读边学**：AI 标注写作技法，解释为什么有效
- **边读边积累**：自动提取人物、设定、事件，构建可维护的知识库
- **证据驱动**：每个结论都链接到原文，可验证、可修正

### 1.3 Core Positioning

```
┌─────────────────────────────────────────────────────────────┐
│                    Narrative Loom                            │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌─────────────────────────────────────────────────────────┐│
│  │              PRIMARY 核心功能                            ││
│  │                                                         ││
│  │   📚 学习写作                 📖 知识积累                ││
│  │   • 技法拆解                  • 故事脉络                 ││
│  │   • 为什么有效                • 人物档案                 ││
│  │   • 示例标注                  • 设定集                   ││
│  │   • 机制解释                  • 时间线                   ││
│  │                                                         ││
│  │              同一段文本，两种价值                        ││
│  └─────────────────────────────────────────────────────────┘│
│                                                              │
│  ┌─────────────────────────────────────────────────────────┐│
│  │              EXTENSION 扩展功能                          ││
│  │                                                         ││
│  │   ✨ 风格 Agent / Prompt 生成                           ││
│  │   ✨ 续写 / 扩写 / 世界观扩展                           ││
│  │                                                         ││
│  └─────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
```

### 1.4 Tech Stack

| Component | Technology | Rationale |
|-----------|------------|-----------|
| **Desktop Shell** | Tauri 2.0 | 轻量级、原生体验、单一安装包 |
| **Frontend** | Vue 3 + TypeScript | 用户偏好、响应式、生态成熟 |
| **Styling** | TailwindCSS | 快速开发、一致性、响应式设计 |
| **Core Engine** | Rust | 内存效率、流式处理、状态管理 |
| **AI/LLM Layer** | Python Sidecar | LLM 生态最成熟、快速迭代 Prompt |
| **Storage** | SQLite | 嵌入式、可靠、单文件 |

### 1.5 Key Features

| Feature | Description |
|---------|-------------|
| 📚 书库管理 | 每本书一个独立项目，可视化书架界面 |
| 📖 章节工作台 | 原文阅读 + AI 标注 + 卡片生成 |
| 📝 技法卡片 | 拆解写作技法，解释机制，提供示例 |
| 📋 知识卡片 | 提取人物/设定/事件/时间线 |
| 📥 收件箱 | 审核队列，确认后入库 |
| 📜 故事圣经 | 可编辑维护的知识库 |
| 🔍 证据溯源 | 每个结论链接到原文 |
| 🔗 全局搜索 | 跨章节、跨条目检索 |

---

## 2. Goals & Non-Goals

### 2.1 Goals

- [x] 支持 200 万字以上的长篇小说
- [x] 章节级增量分析，支持中断恢复
- [x] 提取写作技法并解释机制
- [x] 构建可维护的故事圣经（人物/设定/事件/时间线）
- [x] 确保所有结论可溯源到原文
- [x] 提供美观的书库和阅读界面
- [x] 内存占用 < 200MB
- [x] 支持开源/商业分发

### 2.2 Non-Goals (V1)

- 实时多用户协作
- 云端同步
- 移动端
- 完整的文字处理器功能
- 自动续写/生成（扩展功能，非核心）

---

## 3. System Architecture

### 3.1 High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            Narrative Loom Desktop                            │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │                         Tauri App Container                             │ │
│  ├────────────────────────────────────────────────────────────────────────┤ │
│  │                                                                         │ │
│  │  ┌─────────────────────┐                                               │ │
│  │  │   Vue 3 Frontend    │  TailwindCSS 样式                             │ │
│  │  │   + TailwindCSS     │  书库 / 章节工作台 / 故事圣经 / 收件箱         │ │
│  │  └──────────┬──────────┘                                               │ │
│  │             │ Tauri IPC                                                │ │
│  │             ▼                                                          │ │
│  │  ┌─────────────────────┐                                               │ │
│  │  │     Rust Core       │  书籍管理 / 存储 / 检索 / 任务队列             │ │
│  │  │   (src-tauri)       │  证据索引 / Delta 归约                        │ │
│  │  └──────────┬──────────┘                                               │ │
│  │             │ JSON-RPC over stdio                                      │ │
│  │             ▼                                                          │ │
│  │  ┌─────────────────────┐                                               │ │
│  │  │   Python Sidecar    │  LLM 调用 / 技法分析 / 知识提取               │ │
│  │  │   (Bundled)         │  Prompt 编排                                  │ │
│  │  └─────────────────────┘                                               │ │
│  │                                                                         │ │
│  └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │                          Local Storage                                   ││
│  │                                                                          ││
│  │   📁 books/                                                              ││
│  │      ├── {book-id}/                                                     ││
│  │      │   ├── book.db          # SQLite (元数据、卡片、圣经)              ││
│  │      │   ├── chapters/        # 章节原文分块                             ││
│  │      │   └── cover.jpg        # 封面图片                                ││
│  │      └── ...                                                            ││
│  │                                                                          ││
│  └─────────────────────────────────────────────────────────────────────────┘│
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 3.2 Core Data Flow

```
导入书籍
    │
    ▼
┌─────────────┐
│   书库      │ ─── 生成一本书（独立项目）
└─────────────┘
    │
    ▼ 进入书籍
┌─────────────┐
│ 章节工作台  │ ─── 阅读 + AI 分析
└─────────────┘
    │
    ├──► 技法卡 (TechniqueCard)
    │    • 技法类型
    │    • 为什么有效
    │    • 证据引用
    │
    └──► 知识卡 (KnowledgeCard)
         • 人物/设定/事件/时间线
         • 证据引用
         • 置信度
              │
              ▼
┌─────────────────────────┐
│  收件箱 (Inbox)         │ ─── 审核：接受/合并/驳回/修改
└─────────────────────────┘
              │
              ▼
┌─────────────────────────┐
│  故事圣经 (Story Bible) │ ─── 可维护的知识库
│  • 人物档案             │
│  • 设定集               │
│  • 事件列表             │
│  • 时间线               │
└─────────────────────────┘
```

### 3.3 Design Principles

| Principle | Implementation |
|-----------|----------------|
| **一书一库** | 每本书独立 SQLite + 文件夹 |
| **证据驱动** | 所有卡片必须有证据引用 |
| **候选→确认** | AI 输出先进收件箱，确认后入圣经 |
| **可编辑** | 用户可修正所有结论 |
| **低内存** | 流式读取，按需加载 |

---

## 4. Project Structure

### 4.1 Directory Layout

```
narrative-loom/
├── Cargo.toml                    # Workspace root
├── package.json                  # Frontend dependencies
├── vite.config.ts
├── tailwind.config.js            # TailwindCSS 配置
├── postcss.config.js
├── tsconfig.json
├── README.md
│
├── docs/
│   ├── PRD.md                    # This document
│   ├── TASKS.md                  # Implementation checklist
│   └── ...
│
├── src/                          # Vue Frontend
│   ├── App.vue
│   ├── main.ts
│   ├── style.css                 # Tailwind imports
│   │
│   ├── assets/
│   │   ├── fonts/
│   │   └── images/
│   │
│   ├── components/
│   │   ├── ui/                   # 基础 UI 组件
│   │   │   ├── Button.vue
│   │   │   ├── Card.vue
│   │   │   ├── Modal.vue
│   │   │   ├── Badge.vue
│   │   │   ├── Input.vue
│   │   │   ├── Tabs.vue
│   │   │   └── ...
│   │   │
│   │   ├── library/              # 书库组件
│   │   │   ├── BookShelf.vue     # 书架网格
│   │   │   ├── BookCard.vue      # 书籍卡片
│   │   │   ├── ImportModal.vue   # 导入对话框
│   │   │   └── BookCover.vue     # 封面展示
│   │   │
│   │   ├── reader/               # 阅读器组件
│   │   │   ├── ChapterReader.vue # 章节阅读器
│   │   │   ├── TextHighlight.vue # 文本高亮
│   │   │   ├── EvidenceLink.vue  # 证据链接
│   │   │   └── ChapterNav.vue    # 章节导航
│   │   │
│   │   ├── cards/                # 卡片组件
│   │   │   ├── TechniqueCard.vue # 技法卡片
│   │   │   ├── KnowledgeCard.vue # 知识卡片
│   │   │   ├── CharacterCard.vue # 人物卡片
│   │   │   ├── SettingCard.vue   # 设定卡片
│   │   │   └── EventCard.vue     # 事件卡片
│   │   │
│   │   ├── bible/                # 故事圣经组件
│   │   │   ├── CharacterList.vue
│   │   │   ├── SettingList.vue
│   │   │   ├── EventList.vue
│   │   │   ├── Timeline.vue
│   │   │   └── BibleEditor.vue   # 条目编辑器
│   │   │
│   │   ├── inbox/                # 收件箱组件
│   │   │   ├── InboxList.vue
│   │   │   ├── ReviewCard.vue
│   │   │   └── BatchActions.vue
│   │   │
│   │   └── common/               # 通用组件
│   │       ├── Sidebar.vue
│   │       ├── Header.vue
│   │       ├── SearchBar.vue
│   │       ├── ProgressBar.vue
│   │       └── EmptyState.vue
│   │
│   ├── views/                    # 页面视图
│   │   ├── LibraryView.vue       # 书库页面
│   │   ├── BookView.vue          # 书籍主页面（容器）
│   │   ├── ChapterView.vue       # 章节工作台
│   │   ├── BibleView.vue         # 故事圣经
│   │   ├── InboxView.vue         # 收件箱
│   │   ├── TechniqueView.vue     # 技法库
│   │   ├── SearchView.vue        # 搜索结果
│   │   └── SettingsView.vue      # 设置
│   │
│   ├── composables/              # Vue Composables
│   │   ├── useTauri.ts           # Tauri IPC
│   │   ├── useBook.ts            # 书籍操作
│   │   ├── useAnalysis.ts        # 分析操作
│   │   ├── useBible.ts           # 故事圣经操作
│   │   └── useSearch.ts          # 搜索
│   │
│   ├── stores/                   # Pinia Stores
│   │   ├── library.ts            # 书库状态
│   │   ├── book.ts               # 当前书籍状态
│   │   ├── analysis.ts           # 分析状态
│   │   └── ui.ts                 # UI 状态
│   │
│   ├── router/
│   │   └── index.ts
│   │
│   ├── types/                    # TypeScript 类型
│   │   ├── book.ts
│   │   ├── card.ts
│   │   ├── bible.ts
│   │   └── api.ts
│   │
│   └── utils/
│       ├── format.ts
│       └── helpers.ts
│
├── src-tauri/                    # Rust Core
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── build.rs
│   ├── icons/
│   └── src/
│       ├── main.rs
│       ├── lib.rs
│       │
│       ├── commands/             # Tauri Commands
│       │   ├── mod.rs
│       │   ├── library.rs        # 书库操作
│       │   ├── book.rs           # 书籍操作
│       │   ├── chapter.rs        # 章节操作
│       │   ├── analysis.rs       # 分析操作
│       │   ├── bible.rs          # 故事圣经操作
│       │   ├── inbox.rs          # 收件箱操作
│       │   ├── search.rs         # 搜索
│       │   └── export.rs         # 导出
│       │
│       ├── core/                 # 核心类型
│       │   ├── mod.rs
│       │   ├── ids.rs
│       │   ├── book.rs           # Book, Chapter
│       │   ├── card.rs           # TechniqueCard, KnowledgeCard
│       │   ├── bible.rs          # Character, Setting, Event, Timeline
│       │   ├── evidence.rs       # Evidence, Span
│       │   └── error.rs
│       │
│       ├── storage/              # 存储层
│       │   ├── mod.rs
│       │   ├── library.rs        # 书库管理
│       │   ├── book_db.rs        # 单书 SQLite
│       │   ├── chapters.rs       # 章节文件
│       │   ├── migrations.rs
│       │   └── schema.sql
│       │
│       ├── ingestion/            # 导入处理
│       │   ├── mod.rs
│       │   ├── import.rs         # 文件导入
│       │   ├── parser.rs         # 格式解析
│       │   ├── segmentation.rs   # 章节分割
│       │   └── cover.rs          # 封面提取/生成
│       │
│       ├── retrieval/            # 检索
│       │   ├── mod.rs
│       │   └── search.rs
│       │
│       ├── sidecar/              # Python 管理
│       │   ├── mod.rs
│       │   ├── manager.rs
│       │   ├── protocol.rs
│       │   └── messages.rs
│       │
│       └── tasks/                # 后台任务
│           ├── mod.rs
│           ├── queue.rs
│           └── scheduler.rs
│
├── python/                       # Python Sidecar
│   ├── pyproject.toml
│   ├── requirements.txt
│   └── loom/
│       ├── __init__.py
│       ├── __main__.py
│       ├── server.py             # JSON-RPC 服务
│       │
│       ├── providers/            # LLM 提供者
│       │   ├── __init__.py
│       │   ├── base.py
│       │   ├── openai.py
│       │   ├── anthropic.py
│       │   └── ollama.py
│       │
│       ├── analysis/             # 分析逻辑
│       │   ├── __init__.py
│       │   ├── technique.py      # 技法分析
│       │   ├── character.py      # 人物提取
│       │   ├── setting.py        # 设定提取
│       │   ├── event.py          # 事件提取
│       │   └── timeline.py       # 时间线提取
│       │
│       ├── prompts/              # Prompt 模板
│       │   ├── __init__.py
│       │   ├── technique.py
│       │   ├── extraction.py
│       │   └── templates/
│       │
│       └── schemas/              # Pydantic 模型
│           ├── __init__.py
│           ├── card.py
│           ├── bible.py
│           └── request.py
│
└── fixtures/                     # 测试数据
    └── sample-novel/
```

### 4.2 Frontend Dependencies

```json
{
  "dependencies": {
    "vue": "^3.4",
    "vue-router": "^4.2",
    "pinia": "^2.1",
    "@tauri-apps/api": "^2.0"
  },
  "devDependencies": {
    "typescript": "^5.3",
    "vite": "^5.0",
    "@vitejs/plugin-vue": "^5.0",
    "tailwindcss": "^3.4",
    "postcss": "^8.4",
    "autoprefixer": "^10.4",
    "@tailwindcss/typography": "^0.5",
    "@tailwindcss/forms": "^0.5"
  }
}
```

---

## 5. Core Data Models

### 5.1 Book (书籍)

```rust
/// 书籍
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Book {
    pub id: BookId,
    pub title: String,
    pub author: Option<String>,
    pub cover_path: Option<String>,
    pub total_chapters: u32,
    pub analyzed_chapters: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub status: BookStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BookStatus {
    Importing,
    Ready,
    Analyzing,
    Completed,
    Error(String),
}

/// 章节
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chapter {
    pub id: ChapterId,
    pub book_id: BookId,
    pub index: u32,
    pub title: Option<String>,
    pub char_count: u32,
    pub analyzed: bool,
    pub technique_count: u32,
    pub knowledge_count: u32,
}
```

### 5.2 Cards (卡片)

```rust
/// 技法卡片
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechniqueCard {
    pub id: CardId,
    pub chapter_id: ChapterId,
    pub technique_type: TechniqueType,
    pub title: String,
    pub description: String,           // 这是什么技法
    pub mechanism: String,              // 为什么有效
    pub evidence: Vec<Evidence>,        // 原文证据
    pub tags: Vec<String>,
    pub collected: bool,                // 是否收藏到技法库
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TechniqueType {
    Structure,      // 结构
    Scene,          // 场景
    Character,      // 人物塑造
    Dialogue,       // 对话
    Description,    // 描写
    Pacing,         // 节奏
    Suspense,       // 悬念
    Foreshadowing,  // 伏笔
    Theme,          // 主题
    Voice,          // 声音/风格
    Other(String),
}

/// 知识卡片（候选）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeCard {
    pub id: CardId,
    pub chapter_id: ChapterId,
    pub knowledge_type: KnowledgeType,
    pub title: String,
    pub content: serde_json::Value,     // 类型特定内容
    pub evidence: Vec<Evidence>,
    pub confidence: Confidence,
    pub status: CardStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KnowledgeType {
    Character,
    Setting,
    Event,
    Timeline,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CardStatus {
    Pending,        // 待审核（在收件箱）
    Accepted,       // 已接受（已入库）
    Rejected,       // 已驳回
    Merged,         // 已合并到其他条目
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Confidence {
    High,
    Medium,
    Low,
}
```

### 5.3 Evidence (证据)

```rust
/// 证据引用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub chapter_id: ChapterId,
    pub paragraph_index: u32,
    pub start_char: u32,
    pub end_char: u32,
    pub excerpt: String,            // 摘录（最多 200 字符）
    pub hash: u64,                  // 内容哈希
}
```

### 5.4 Story Bible (故事圣经)

```rust
/// 人物
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub id: EntityId,
    pub name: String,
    pub aliases: Vec<String>,
    pub description: String,
    pub traits: Vec<String>,
    pub role: CharacterRole,
    pub first_appearance: ChapterId,
    pub relationships: Vec<Relationship>,
    pub evidence: Vec<Evidence>,
    pub notes: String,              // 用户笔记
    pub updated_at: DateTime<Utc>,
}

/// 设定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Setting {
    pub id: EntityId,
    pub setting_type: SettingType,
    pub name: String,
    pub description: String,
    pub properties: HashMap<String, String>,
    pub evidence: Vec<Evidence>,
    pub notes: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SettingType {
    Location,       // 地点
    Organization,   // 组织/势力
    PowerSystem,    // 力量体系
    Item,           // 重要物品
    Rule,           // 世界规则
    Other(String),
}

/// 事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: EntityId,
    pub title: String,
    pub description: String,
    pub chapter_id: ChapterId,
    pub characters_involved: Vec<EntityId>,
    pub importance: Importance,
    pub evidence: Vec<Evidence>,
    pub notes: String,
    pub updated_at: DateTime<Utc>,
}

/// 时间线事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEvent {
    pub id: EntityId,
    pub event_id: Option<EntityId>,     // 关联的事件
    pub title: String,
    pub chapter_id: ChapterId,
    pub order: u32,                     // 相对顺序
    pub time_marker: Option<String>,    // 时间标记（如"三年后"）
    pub is_uncertain: bool,             // 时间是否不确定
}
```

---

## 6. User Interface Design

### 6.1 Design System (TailwindCSS)

#### Color Palette

```javascript
// tailwind.config.js
module.exports = {
  theme: {
    extend: {
      colors: {
        // Primary - 书香主题
        primary: {
          50: '#faf5f0',
          100: '#f0e6d8',
          200: '#e0ccb0',
          300: '#c9a87a',
          400: '#b38a50',
          500: '#9a7040',
          600: '#7d5a35',
          700: '#5f442a',
          800: '#423020',
          900: '#2a1f15',
        },
        // Accent - 知识高亮
        accent: {
          technique: '#3b82f6',   // 技法 - 蓝色
          character: '#10b981',   // 人物 - 绿色
          setting: '#8b5cf6',     // 设定 - 紫色
          event: '#f59e0b',       // 事件 - 橙色
          timeline: '#ec4899',    // 时间线 - 粉色
        },
      },
    },
  },
}
```

#### Typography

```css
/* 阅读器字体 */
.reader-content {
  @apply font-serif text-lg leading-relaxed text-gray-800;
}

/* 卡片标题 */
.card-title {
  @apply font-semibold text-gray-900;
}

/* 证据摘录 */
.evidence-excerpt {
  @apply font-serif italic text-gray-600 border-l-2 border-primary-300 pl-3;
}
```

### 6.2 Page Layouts

#### 书库页面 (LibraryView)

```
┌─────────────────────────────────────────────────────────────────┐
│  Narrative Loom                              [设置] [─] [□] [×] │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  我的书库                                        [+ 导入书籍]   │
│  ─────────                                                       │
│                                                                  │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐            │
│  │  📚     │  │  📚     │  │  📚     │  │  ┌───┐  │            │
│  │ [封面]  │  │ [封面]  │  │ [封面]  │  │  │ + │  │            │
│  │         │  │         │  │         │  │  └───┘  │            │
│  ├─────────┤  ├─────────┤  ├─────────┤  │         │            │
│  │斗破苍穹 │  │凡人修仙 │  │诡秘之主 │  │ 添加    │            │
│  │天蚕土豆 │  │忘语     │  │爱潜水.. │  │ 书籍    │            │
│  │████░░░░ │  │██████░░ │  │████████ │  │         │            │
│  │42/100章 │  │85/120章 │  │已完成   │  │         │            │
│  └─────────┘  └─────────┘  └─────────┘  └─────────┘            │
│                                                                  │
│  ┌─────────┐  ┌─────────┐                                       │
│  │  📚     │  │  📚     │                                       │
│  │ [封面]  │  │ [封面]  │                                       │
│  │         │  │         │                                       │
│  ├─────────┤  ├─────────┤                                       │
│  │遮天     │  │完美世界 │                                       │
│  │辰东     │  │辰东     │                                       │
│  │░░░░░░░░ │  │░░░░░░░░ │                                       │
│  │未开始   │  │未开始   │                                       │
│  └─────────┘  └─────────┘                                       │
│                                                                  │
├─────────────────────────────────────────────────────────────────┤
│  共 6 本书  |  3 本分析中  |  1 本已完成                        │
└─────────────────────────────────────────────────────────────────┘
```

#### 书籍主页面 (BookView)

```
┌─────────────────────────────────────────────────────────────────┐
│  ← 返回书库    斗破苍穹                         [导出] [设置]  │
├───────────┬─────────────────────────────────────────────────────┤
│           │                                                      │
│  导航栏   │                    主内容区                          │
│  ───────  │                    ─────────                         │
│           │                                                      │
│  📖 阅读  │    根据选中的导航项显示不同内容：                    │
│           │                                                      │
│  📥 收件箱│    • 阅读 → ChapterView (章节工作台)                │
│     (12)  │    • 收件箱 → InboxView (待审核列表)                │
│           │    • 故事圣经 → BibleView (知识库)                  │
│  📜 故事  │    • 技法库 → TechniqueView (收藏的技法)            │
│     圣经  │    • 搜索 → SearchView (搜索结果)                   │
│           │                                                      │
│  📝 技法库│                                                      │
│           │                                                      │
│  🔍 搜索  │                                                      │
│           │                                                      │
│  ───────  │                                                      │
│           │                                                      │
│  章节列表 │                                                      │
│  ───────  │                                                      │
│  ▼ 卷一   │                                                      │
│    第1章  │                                                      │
│    第2章 ✓│                                                      │
│    第3章 ✓│                                                      │
│    ...    │                                                      │
│  ▶ 卷二   │                                                      │
│           │                                                      │
├───────────┴─────────────────────────────────────────────────────┤
│  分析进度: 42/100 章  |  人物: 28  |  设定: 45  |  事件: 156    │
└─────────────────────────────────────────────────────────────────┘
```

#### 章节工作台 (ChapterView)

```
┌─────────────────────────────────────────────────────────────────┐
│  第42章 美杜莎女王                    [◀ 上一章] [分析] [下一章 ▶]│
├────────────────────────────────┬────────────────────────────────┤
│                                │                                │
│  原文阅读区                    │  卡片栏                        │
│  ──────────                    │  ──────                        │
│                                │                                │
│  萧炎看着眼前的女子，心中      │  [技法] [知识]                 │
│  一阵震惊。                    │  ─────────────                  │
│                                │                                │
│  [高亮:人物出场]               │  ┌─ 技法卡 ─────────────────┐ │
│  她的美貌足以让任何男子        │  │ 🎭 人物出场技法           │ │
│  为之倾倒，但那双金色的        │  │                           │ │
│  蛇瞳却透露着致命的危险...     │  │ 先写主角反应，再写外貌    │ │
│                                │  │ 特征，最后暗示威胁。      │ │
│  [高亮:对话张力]               │  │                           │ │
│  "你就是那个吞噬了异火的       │  │ 💡 为什么有效:            │ │
│  小子？"她的声音冰冷，         │  │ 通过主角视角建立代入感... │ │
│  却带着一丝好奇。              │  │                           │ │
│                                │  │ 📎 证据: 第1-3段          │ │
│  萧炎强压心中的震惊，          │  │ [收藏到技法库]            │ │
│  努力让自己的声音保持          │  └───────────────────────────┘ │
│  平稳："美杜莎女王，           │                                │
│  久仰大名。"                   │  ┌─ 知识卡 ─────────────────┐ │
│                                │  │ 👤 人物: 美杜莎女王       │ │
│                                │  │                           │ │
│                                │  │ • 蛇族女王                │ │
│                                │  │ • 金色蛇瞳                │ │
│                                │  │ • 实力强大                │ │
│                                │  │                           │ │
│                                │  │ 📎 证据: 第1-5段          │ │
│                                │  │ 置信度: 高                │ │
│                                │  │                           │ │
│                                │  │ [送入收件箱] [直接入库]   │ │
│                                │  └───────────────────────────┘ │
│                                │                                │
├────────────────────────────────┴────────────────────────────────┤
│  本章: 技法 3 | 人物 2 | 设定 1 | 事件 4      [全部送入收件箱]  │
└─────────────────────────────────────────────────────────────────┘
```

#### 故事圣经 (BibleView)

```
┌─────────────────────────────────────────────────────────────────┐
│  故事圣经                                        [导出] [搜索]  │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  [人物] [设定] [事件] [时间线]                                  │
│  ─────────────────────────────                                   │
│                                                                  │
│  人物列表 (28)                              人物详情             │
│  ────────────                               ──────────           │
│                                                                  │
│  🔍 搜索人物...                             萧炎                 │
│                                             ────                 │
│  ┌─────────────────────────┐               别名: 小炎子、炎哥    │
│  │ 主角                    │               角色: 主角            │
│  │ ├─ 萧炎 ◄──────────────┼───────────►   首次出场: 第1章       │
│  │ └─ 药老                 │                                     │
│  │                         │               描述:                 │
│  │ 重要配角                │               萧家少年，曾是天才    │
│  │ ├─ 萧薰儿               │               后陨落，后觉醒...     │
│  │ ├─ 美杜莎女王           │                                     │
│  │ ├─ 云韵                 │               特征:                 │
│  │ └─ 海波东               │               • 坚韧不拔            │
│  │                         │               • 重情重义            │
│  │ 反派                    │               • 天赋异禀            │
│  │ ├─ 云山                 │                                     │
│  │ ├─ 魂殿殿主             │               关系:                 │
│  │ └─ ...                  │               • 萧薰儿 (青梅竹马)   │
│  │                         │               • 药老 (师父)         │
│  └─────────────────────────┘               • 美杜莎 (契约关系)   │
│                                                                  │
│                                             证据来源:            │
│                                             • 第1章 第3段        │
│                                             • 第5章 第12段       │
│                                             • ...                │
│                                                                  │
│                                             [编辑] [查看证据]    │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

#### 收件箱 (InboxView)

```
┌─────────────────────────────────────────────────────────────────┐
│  收件箱                                          [全部接受]     │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  待审核: 12 条                    [人物] [设定] [事件] [全部]   │
│  ────────────                                                    │
│                                                                  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │ 👤 人物: 云韵                              第38章 | 置信:高│  │
│  │                                                            │  │
│  │ 云岚宗少宗主，斗皇强者，与萧炎有婚约纠葛...               │  │
│  │                                                            │  │
│  │ 📎 "云韵轻轻落下，白衣飘飘，宛如仙子下凡..."             │  │
│  │                                                            │  │
│  │ [接受入库] [编辑后接受] [合并到现有] [驳回]               │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │ ⚙️ 设定: 斗气等级体系                      第2章 | 置信:高 │  │
│  │                                                            │  │
│  │ 斗者→斗师→大斗师→斗灵→斗王→斗皇→斗宗→斗尊→斗圣→斗帝    │  │
│  │                                                            │  │
│  │ 📎 "在这片大陆上，斗气等级分为..."                        │  │
│  │                                                            │  │
│  │ [接受入库] [编辑后接受] [合并到现有] [驳回]               │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │ 📅 事件: 萧炎与美杜莎初遇                  第42章 | 置信:高│  │
│  │                                                            │  │
│  │ 萧炎在沙漠中遭遇美杜莎女王，双方初次交锋...               │  │
│  │                                                            │  │
│  │ [接受入库] [编辑后接受] [合并到现有] [驳回]               │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 7. Core Workflows

### 7.1 导入书籍

```
用户点击 [+ 导入书籍]
         │
         ▼
┌─────────────────────┐
│  选择文件           │
│  支持: .txt / .epub │
└─────────────────────┘
         │
         ▼
┌─────────────────────┐
│  填写书籍信息       │
│  • 标题 (自动识别)  │
│  • 作者 (可选)      │
│  • 封面 (可选上传)  │
└─────────────────────┘
         │
         ▼
┌─────────────────────┐
│  Rust: 解析文件     │
│  • 编码检测         │
│  • 章节分割         │
│  • 创建书籍目录     │
│  • 存储章节文件     │
└─────────────────────┘
         │
         ▼
书籍出现在书库中
```

### 7.2 分析章节

```
用户进入章节工作台，点击 [分析]
         │
         ▼
┌─────────────────────┐
│  Rust: 读取章节内容 │
│  构建分析上下文     │
└─────────────────────┘
         │
         ▼
┌─────────────────────┐
│  Python: LLM 分析   │
│  • 技法识别         │
│  • 人物提取         │
│  • 设定提取         │
│  • 事件提取         │
└─────────────────────┘
         │
         ▼
┌─────────────────────┐
│  Rust: 保存卡片     │
│  • 技法卡 → 直接保存│
│  • 知识卡 → 进收件箱│
└─────────────────────┘
         │
         ▼
UI 更新显示卡片
```

### 7.3 审核入库

```
用户在收件箱查看卡片
         │
         ├─► [接受入库] ──► 创建/更新故事圣经条目
         │
         ├─► [编辑后接受] ──► 打开编辑器 ──► 保存后入库
         │
         ├─► [合并到现有] ──► 选择目标条目 ──► 合并信息
         │
         └─► [驳回] ──► 标记为已驳回，不入库
```

---

## 8. Module Specifications

### 8.1 Vue Frontend

| Module | Responsibility |
|--------|----------------|
| `views/LibraryView` | 书库页面，书架展示 |
| `views/BookView` | 书籍容器，侧边导航 |
| `views/ChapterView` | 章节工作台，核心阅读+分析 |
| `views/BibleView` | 故事圣经，知识库浏览编辑 |
| `views/InboxView` | 收件箱，审核队列 |
| `views/TechniqueView` | 技法库，收藏的技法 |
| `stores/library` | 书库状态 |
| `stores/book` | 当前书籍状态 |
| `composables/useTauri` | Tauri IPC 封装 |

### 8.2 Rust Core

| Module | Responsibility |
|--------|----------------|
| `commands/library` | 书库 CRUD |
| `commands/book` | 书籍操作 |
| `commands/chapter` | 章节读取 |
| `commands/analysis` | 触发分析 |
| `commands/bible` | 故事圣经 CRUD |
| `commands/inbox` | 收件箱操作 |
| `storage/library` | 书库索引 |
| `storage/book_db` | 单书 SQLite |
| `ingestion/import` | 文件导入 |
| `sidecar/manager` | Python 进程管理 |

### 8.3 Python Sidecar

| Module | Responsibility |
|--------|----------------|
| `server.py` | JSON-RPC 服务 |
| `providers/*` | LLM 提供者 |
| `analysis/technique` | 技法分析 |
| `analysis/character` | 人物提取 |
| `analysis/setting` | 设定提取 |
| `analysis/event` | 事件提取 |
| `prompts/*` | Prompt 模板 |

---

## 9. IPC Protocol

### 9.1 Tauri Commands

```typescript
// 书库
invoke('list_books') -> Book[]
invoke('import_book', { path, title?, author? }) -> Book
invoke('delete_book', { bookId }) -> void
invoke('get_book', { bookId }) -> Book

// 章节
invoke('get_chapters', { bookId }) -> Chapter[]
invoke('get_chapter_content', { chapterId }) -> string

// 分析
invoke('analyze_chapter', { chapterId }) -> AnalysisResult
invoke('get_analysis_status', { bookId }) -> AnalysisStatus

// 卡片
invoke('get_technique_cards', { chapterId }) -> TechniqueCard[]
invoke('get_knowledge_cards', { chapterId }) -> KnowledgeCard[]
invoke('collect_technique', { cardId }) -> void

// 收件箱
invoke('get_inbox', { bookId }) -> KnowledgeCard[]
invoke('accept_card', { cardId }) -> Entity
invoke('reject_card', { cardId }) -> void
invoke('merge_card', { cardId, targetEntityId }) -> Entity

// 故事圣经
invoke('get_characters', { bookId }) -> Character[]
invoke('get_settings', { bookId }) -> Setting[]
invoke('get_events', { bookId }) -> Event[]  // Event 包含 time_marker/order_in_chapter/is_flashback/relative_time
invoke('update_entity', { entityId, data }) -> Entity

// 搜索
invoke('search', { bookId, query }) -> SearchResult[]
```

### 9.2 Python JSON-RPC

```python
# 请求
{
  "method": "analyze_chapter",
  "params": {
    "chapter_content": "...",
    "context": {
      "book_title": "斗破苍穹",
      "known_characters": [...],
      "known_settings": [...],
      "recent_events": [...]
    }
  }
}

# 响应
{
  "techniques": [...],
  "characters": [...],
  "settings": [...],
  "events": [...]
}
```

---

## 10. Configuration

### 10.1 App Config

```toml
# ~/.config/narrative-loom/config.toml

[general]
language = "zh-CN"
theme = "light"  # light | dark | system
books_dir = "~/Documents/NarrativeLoom/books"

[llm]
provider = "openai"  # openai | anthropic | ollama

[llm.openai]
api_key = "${OPENAI_API_KEY}"
model = "gpt-4o"

[llm.anthropic]
api_key = "${ANTHROPIC_API_KEY}"
model = "claude-sonnet-4-20250514"

[llm.ollama]
endpoint = "http://localhost:11434"
model = "qwen2.5:32b"

[analysis]
auto_analyze = false
batch_size = 1
```

### 10.2 TailwindCSS Config

```javascript
// tailwind.config.js
module.exports = {
  content: ['./src/**/*.{vue,js,ts}'],
  theme: {
    extend: {
      colors: {
        primary: { /* 书香主题色 */ },
        accent: {
          technique: '#3b82f6',
          character: '#10b981',
          setting: '#8b5cf6',
          event: '#f59e0b',
          timeline: '#ec4899',
        },
      },
      fontFamily: {
        serif: ['Noto Serif SC', 'serif'],
        sans: ['Inter', 'sans-serif'],
      },
    },
  },
  plugins: [
    require('@tailwindcss/typography'),
    require('@tailwindcss/forms'),
  ],
}
```

### 10.3 Provider System (LLM API 配置)

Provider 系统支持用户配置多个 LLM API 提供者，支持 OpenAI 兼容接口。

#### 10.3.1 数据模型

```rust
/// LLM 提供者配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub id: String,                           // 唯一标识 (如 "openai", "deepseek", "custom-1")
    pub name: String,                         // 显示名称 (如 "OpenAI", "DeepSeek")
    pub enabled: bool,                        // 是否启用
    pub base_url: String,                     // API 基础 URL (如 "https://api.openai.com")
    pub api_format: ApiFormat,                // API 格式
    pub path_override: Option<String>,        // 路径覆盖 (如 "/v1/chat/completions")
    pub api_key_ref: String,                  // API Key 引用名 (存储在 OS Keychain)
    pub default_model: String,                // 默认模型
    pub available_models: Vec<String>,        // 可用模型列表
    pub headers: HashMap<String, String>,     // 自定义请求头
    pub timeout_ms: u64,                      // 超时时间 (默认 60000)
    pub max_retries: u32,                     // 最大重试次数 (默认 3)
}

/// API 格式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApiFormat {
    /// OpenAI Chat Completions API (/v1/chat/completions)
    ChatCompletions,
    /// OpenAI Responses API (/v1/responses)
    Responses,
}

impl ApiFormat {
    pub fn default_path(&self) -> &str {
        match self {
            ApiFormat::ChatCompletions => "/v1/chat/completions",
            ApiFormat::Responses => "/v1/responses",
        }
    }
}
```

#### 10.3.2 内置提供者

| Provider | Base URL | API Format | 默认模型 |
|----------|----------|------------|----------|
| OpenAI | `https://api.openai.com` | ChatCompletions | gpt-4o |
| Anthropic (via proxy) | `https://api.anthropic.com` | ChatCompletions | claude-sonnet-4-20250514 |
| DeepSeek | `https://api.deepseek.com` | ChatCompletions | deepseek-chat |
| Ollama | `http://localhost:11434` | ChatCompletions | qwen2.5:32b |

#### 10.3.3 API Key 安全存储

```rust
/// API Key 不存储在配置文件中，使用 OS Keychain
/// 
/// Windows: Credential Manager
/// macOS: Keychain
/// Linux: Secret Service (libsecret)

pub trait KeychainService {
    fn store_key(&self, provider_id: &str, api_key: &str) -> Result<()>;
    fn get_key(&self, provider_id: &str) -> Result<String>;
    fn delete_key(&self, provider_id: &str) -> Result<()>;
}
```

#### 10.3.4 请求构建

```rust
impl ProviderConfig {
    /// 构建完整的 API URL
    pub fn build_url(&self) -> String {
        let path = self.path_override
            .as_deref()
            .unwrap_or_else(|| self.api_format.default_path());
        format!("{}{}", self.base_url.trim_end_matches('/'), path)
    }
}
```

### 10.4 Agent System (分析 Agent 配置)

Agent 系统允许用户配置不同的分析任务使用不同的模型和参数。

#### 10.4.1 数据模型

```rust
/// 分析 Agent 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub id: String,                           // 唯一标识
    pub name: String,                         // 显示名称
    pub kind: AgentKind,                      // Agent 类型
    pub enabled: bool,                        // 是否启用
    pub provider_id: String,                  // 使用的 Provider ID
    pub model: String,                        // 使用的模型
    pub temperature: f32,                     // 温度 (0.0-2.0)
    pub max_tokens: Option<u32>,              // 最大输出 token 数
    pub system_prompt: Option<String>,        // 自定义系统提示词 (覆盖默认)
    pub output_mode: OutputMode,              // 输出模式
}

/// Agent 类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentKind {
    /// 内置 Agent (不可删除，可配置)
    BuiltIn(BuiltInAgent),
    /// 自定义 Agent
    Custom,
}

/// 内置 Agent 类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BuiltInAgent {
    TechniqueAnalysis,      // 技法分析
    CharacterExtraction,    // 人物提取
    SettingExtraction,      // 设定提取
    EventExtraction,        // 事件提取
    TimelineExtraction,     // 时间线提取
}

/// 输出模式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputMode {
    /// 纯文本输出
    Text,
    /// JSON 对象 (使用 response_format: { type: "json_object" })
    JsonObject,
    /// JSON Schema (使用 response_format: { type: "json_schema", ... })
    JsonSchema { schema: serde_json::Value },
}

/// 分析任务类型
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum TaskType {
    TechniqueAnalysis,
    CharacterExtraction,
    SettingExtraction,
    EventExtraction,
    TimelineExtraction,
}

/// 任务 → Agent 绑定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskBindings {
    pub bindings: HashMap<TaskType, String>,  // TaskType → Agent ID
}
```

#### 10.4.2 默认 Agent 配置

| Agent | 默认 Provider | 默认模型 | Temperature | 输出模式 |
|-------|---------------|----------|-------------|----------|
| 技法分析 | openai | gpt-4o | 0.7 | JsonSchema |
| 人物提取 | openai | gpt-4o | 0.3 | JsonSchema |
| 设定提取 | openai | gpt-4o | 0.3 | JsonSchema |
| 事件提取 | openai | gpt-4o | 0.3 | JsonSchema |
| 时间线提取 | openai | gpt-4o | 0.3 | JsonSchema |

#### 10.4.3 Agent 调用流程

```
用户触发分析
      │
      ▼
┌─────────────────────┐
│  获取 TaskBindings  │ ─── 根据任务类型获取对应 Agent ID
└─────────────────────┘
      │
      ▼
┌─────────────────────┐
│  加载 AgentConfig   │ ─── 获取 Agent 配置
└─────────────────────┘
      │
      ▼
┌─────────────────────┐
│  获取 Provider      │ ─── 根据 agent.provider_id 获取 Provider
└─────────────────────┘
      │
      ▼
┌─────────────────────┐
│  构建请求           │ ─── 组装 system_prompt, model, temperature
└─────────────────────┘
      │
      ▼
┌─────────────────────┐
│  发送到 LLM API     │ ─── 使用 Provider 配置的 URL 和 API Key
└─────────────────────┘
      │
      ▼
┌─────────────────────┐
│  解析响应           │ ─── 根据 OutputMode 解析
└─────────────────────┘
```

#### 10.4.4 Python Sidecar Agent 实现

```python
# python/loom/agents/base.py
from pydantic import BaseModel
from typing import Optional
from enum import Enum

class OutputMode(str, Enum):
    TEXT = "text"
    JSON_OBJECT = "json_object"
    JSON_SCHEMA = "json_schema"

class AgentRequest(BaseModel):
    """从 Rust 传入的 Agent 请求"""
    agent_id: str
    provider_config: dict          # Provider 配置 (含 URL, model 等)
    agent_config: dict             # Agent 配置 (含 temperature, system_prompt 等)
    chapter_content: str           # 章节内容
    context: dict                  # 分析上下文

class BaseAgent:
    """Agent 基类"""
    
    def __init__(self, provider_config: dict, agent_config: dict):
        self.provider = provider_config
        self.config = agent_config
    
    async def execute(self, content: str, context: dict) -> dict:
        """执行分析任务"""
        raise NotImplementedError
    
    def build_system_prompt(self, context: dict) -> str:
        """构建系统提示词"""
        if custom := self.config.get("system_prompt"):
            return custom
        return self.default_system_prompt(context)
    
    def default_system_prompt(self, context: dict) -> str:
        """默认系统提示词 (子类覆盖)"""
        raise NotImplementedError
```

```python
# python/loom/agents/technique.py
class TechniqueAgent(BaseAgent):
    """技法分析 Agent"""
    
    def default_system_prompt(self, context: dict) -> str:
        return """你是一位专业的写作技法分析师。
        
分析以下小说章节，识别作者使用的写作技法。

对于每个技法，请提供：
1. 技法类型 (结构/场景/人物塑造/对话/描写/节奏/悬念/伏笔/主题/声音)
2. 技法名称
3. 技法描述 (这是什么技法)
4. 为什么有效 (分析其作用机制)
5. 证据 (原文引用，标注段落位置)

输出 JSON 格式。"""
```

### 10.5 Settings UI 设计

#### 10.5.1 设置页面结构

```
设置 (SettingsView)
├── 通用设置
│   ├── 语言
│   ├── 主题
│   └── 书库路径
│
├── Provider 设置 (新增)
│   ├── Provider 列表
│   ├── 添加 Provider
│   └── 编辑 Provider
│
├── Agent 设置 (新增)
│   ├── 任务绑定
│   └── Agent 配置
│
└── 关于
```

#### 10.5.2 Provider 设置 UI

```
┌─────────────────────────────────────────────────────────────────┐
│  设置 > Provider 配置                                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  LLM 提供者                                      [+ 添加]       │
│  ───────────                                                     │
│                                                                  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │ ✓ OpenAI                                          [编辑]  │  │
│  │   https://api.openai.com • ChatCompletions • gpt-4o       │  │
│  │   API Key: ●●●●●●●●●●●●sk-xxxx                            │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │ ○ DeepSeek                                        [编辑]  │  │
│  │   https://api.deepseek.com • ChatCompletions              │  │
│  │   API Key: 未配置                                         │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │ ○ Ollama (本地)                                   [编辑]  │  │
│  │   http://localhost:11434 • ChatCompletions                │  │
│  │   无需 API Key                                            │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

#### 10.5.3 Provider 编辑对话框

```
┌─────────────────────────────────────────────────────────────────┐
│  编辑 Provider: OpenAI                                    [×]   │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  名称                                                            │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │ OpenAI                                                    │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                  │
│  Base URL                                                        │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │ https://api.openai.com                                    │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                  │
│  API 格式                                                        │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │ ▼ Chat Completions (/v1/chat/completions)                 │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                  │
│  API Key                                                         │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │ sk-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx                       │  │
│  └───────────────────────────────────────────────────────────┘  │
│  ⓘ API Key 将安全存储在系统密钥链中                            │
│                                                                  │
│  默认模型                                                        │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │ ▼ gpt-4o                                                  │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                  │
│  [测试连接]                                                      │
│                                                                  │
│  ─────────────────────────────────────────────────────────────  │
│  高级设置                                              [展开 ▼] │
│                                                                  │
│                                     [取消]        [保存]        │
└─────────────────────────────────────────────────────────────────┘
```

#### 10.5.4 Agent 设置 UI

```
┌─────────────────────────────────────────────────────────────────┐
│  设置 > Agent 配置                                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  任务 Agent 绑定                                                │
│  ─────────────                                                   │
│                                                                  │
│  ┌─────────────────┬────────────────────┬─────────────────────┐ │
│  │ 任务            │ Agent              │ 模型                │ │
│  ├─────────────────┼────────────────────┼─────────────────────┤ │
│  │ 📝 技法分析     │ ▼ 技法分析 Agent   │ gpt-4o             │ │
│  │ 👤 人物提取     │ ▼ 人物提取 Agent   │ gpt-4o             │ │
│  │ ⚙️ 设定提取     │ ▼ 设定提取 Agent   │ gpt-4o             │ │
│  │ 📅 事件提取     │ ▼ 事件提取 Agent   │ gpt-4o             │ │
│  │ 📊 时间线提取   │ ▼ 时间线提取 Agent │ gpt-4o             │ │
│  └─────────────────┴────────────────────┴─────────────────────┘ │
│                                                                  │
│  Agent 配置                                      [+ 新建 Agent] │
│  ──────────                                                      │
│                                                                  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │ 📝 技法分析 Agent                                 [编辑]  │  │
│  │   Provider: OpenAI • Model: gpt-4o • Temp: 0.7            │  │
│  │   ⚡ 内置 Agent                                           │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │ 👤 人物提取 Agent                                 [编辑]  │  │
│  │   Provider: OpenAI • Model: gpt-4o • Temp: 0.3            │  │
│  │   ⚡ 内置 Agent                                           │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                  │
│  ... (更多 Agent)                                               │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

#### 10.5.5 Agent 编辑对话框

```
┌─────────────────────────────────────────────────────────────────┐
│  编辑 Agent: 技法分析 Agent                               [×]   │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  名称                                                            │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │ 技法分析 Agent                                            │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                  │
│  Provider                                                        │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │ ▼ OpenAI                                                  │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                  │
│  模型                                                            │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │ ▼ gpt-4o                                                  │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                  │
│  Temperature                                                     │
│  ┌────────────────────────────────────────────────────┐  0.7    │
│  │ ████████████████████████████░░░░░░░░░░░░░░░░░░░░░░ │        │
│  └────────────────────────────────────────────────────┘         │
│  ⓘ 较高的温度产生更有创意的分析                                │
│                                                                  │
│  输出模式                                                        │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │ ▼ JSON Schema (结构化输出)                                │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                  │
│  自定义系统提示词 (可选)                                        │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                                                           │  │
│  │ (留空使用默认提示词)                                      │  │
│  │                                                           │  │
│  └───────────────────────────────────────────────────────────┘  │
│  [查看默认提示词]                                               │
│                                                                  │
│                                     [取消]        [保存]        │
└─────────────────────────────────────────────────────────────────┘
```

---

## 11. Implementation Phases

### Phase 0: Project Setup (2 days)

- [ ] 初始化 Tauri + Vue + TailwindCSS 项目
- [ ] 配置 TypeScript, ESLint, Prettier
- [ ] 创建 Rust 模块结构
- [ ] 创建 Python sidecar 骨架
- [ ] 配置开发脚本

### Phase 1: 书库与导入 (1 week)

- [ ] 实现书库 UI (BookShelf, BookCard)
- [ ] 实现书籍导入流程
- [ ] 实现章节分割
- [ ] 实现书籍存储结构
- [ ] 实现封面生成/提取

### Phase 2: 章节工作台 (1.5 weeks)

- [ ] 实现章节阅读器
- [ ] 实现 Python LLM 调用
- [ ] 实现技法分析
- [ ] 实现知识提取
- [ ] 实现卡片展示
- [ ] 实现证据高亮

### Phase 3: 收件箱与故事圣经 (1.5 weeks)

- [ ] 实现收件箱 UI
- [ ] 实现审核操作
- [ ] 实现故事圣经四个视图
- [ ] 实现条目编辑
- [ ] 实现跨条目链接

### Phase 4: 完善与优化 (1 week)

- [ ] 实现全局搜索
- [ ] 实现技法库
- [ ] 实现导出功能
- [ ] 性能优化
- [ ] 错误处理完善
- [ ] 打包分发

---

## 12. RAG Architecture (Retrieval-Augmented Generation)

### 12.1 Why RAG?

对于几百万字的长篇小说（如 200 万字 = ~1000 章），传统的固定上下文方案存在以下问题：

| 挑战 | 说明 |
|------|------|
| **跨章节实体识别** | 第 800 章提到"那个老者"，需要知道是谁 |
| **关系演变追踪** | 萧炎和美杜莎的关系从敌对→契约→... |
| **伏笔回收检测** | 第 50 章埋的伏笔，第 800 章回收 |
| **时间线构建** | "三年后"相对于什么时间点？ |
| **Token 爆炸** | 故事圣经随分析进度线性增长，可能超出上下文限制 |

**RAG 方案**：按需检索语义相关的历史片段，动态构建最优上下文。

### 12.2 RAG Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                      Narrative Loom + RAG Architecture                       │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │                         Tauri App Container                             │ │
│  ├────────────────────────────────────────────────────────────────────────┤ │
│  │                                                                         │ │
│  │  ┌─────────────────────┐                                               │ │
│  │  │   Vue 3 Frontend    │                                               │ │
│  │  └──────────┬──────────┘                                               │ │
│  │             │ Tauri IPC                                                │ │
│  │             ▼                                                          │ │
│  │  ┌─────────────────────┐     ┌─────────────────────┐                  │ │
│  │  │     Rust Core       │────▶│   Retrieval Layer   │                  │ │
│  │  │   (src-tauri)       │     │   (RAG)             │                  │ │
│  │  └──────────┬──────────┘     └──────────┬──────────┘                  │ │
│  │             │                           │                              │ │
│  │             │ JSON-RPC                  │ Vector Query                 │ │
│  │             ▼                           ▼                              │ │
│  │  ┌─────────────────────┐     ┌─────────────────────┐                  │ │
│  │  │   Python Sidecar    │     │   Vector Store      │                  │ │
│  │  │   + Embedding Gen   │────▶│   (sqlite-vec)      │                  │ │
│  │  └─────────────────────┘     └─────────────────────┘                  │ │
│  │                                                                         │ │
│  └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │                          Storage (Per Book)                              ││
│  │   📁 books/{book-id}/                                                   ││
│  │      ├── book.db              # SQLite (metadata, cards, bible)         ││
│  │      ├── vectors.db           # sqlite-vec (embeddings)                 ││
│  │      ├── chapters/            # Raw chapter files                       ││
│  │      └── cover.jpg                                                      ││
│  └─────────────────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────────────────┘
```

### 12.3 Technology Selection

| 组件 | 选择 | 理由 |
|------|------|------|
| **Vector Store** | `sqlite-vec` | 单文件、与现有 SQLite 架构一致、Rust 原生支持 |
| **Embedding Model** | `bge-m3` (本地) / `text-embedding-3-small` (API) | 中文支持优秀、可离线 |
| **Embedding 生成** | Python Sidecar | LLM 生态成熟、模型加载方便 |
| **Chunk 策略** | 段落级 (200-500 字) | 小说自然段落是良好语义单元 |

### 12.4 Data Models

#### 12.4.1 Chunk (文本块)

```rust
/// 文本块
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub id: ChunkId,
    pub chapter_id: ChapterId,
    pub chunk_index: u32,           // 块在章节中的顺序
    pub chunk_type: ChunkType,
    pub content: String,            // 原文内容
    pub char_start: u32,            // 在章节中的起始位置
    pub char_end: u32,
    pub metadata: ChunkMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChunkType {
    Paragraph,          // 段落原文
    EntityDescription,  // 实体描述（从故事圣经提取）
    EventSummary,       // 事件摘要
    RelationshipNote,   // 关系描述
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMetadata {
    pub entities_mentioned: Vec<EntityId>,  // 提到的实体
    pub timestamp_marker: Option<String>,   // 时间标记 "三年后"
}

/// 向量索引条目
#[derive(Debug, Clone)]
pub struct VectorEntry {
    pub chunk_id: ChunkId,
    pub embedding: Vec<f32>,        // 1024 维 (bge-m3)
}
```

#### 12.4.2 Vector Database Schema

```sql
-- vectors.db (每本书一个独立文件)

-- 文本块表
CREATE TABLE chunks (
    id TEXT PRIMARY KEY,
    chapter_id TEXT NOT NULL,
    chunk_index INTEGER NOT NULL,
    chunk_type TEXT NOT NULL,       -- 'paragraph', 'entity', 'event', 'relationship'
    content TEXT NOT NULL,
    char_start INTEGER NOT NULL,
    char_end INTEGER NOT NULL,
    metadata_json TEXT,             -- JSON: entities_mentioned, timestamp_marker
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    
    FOREIGN KEY (chapter_id) REFERENCES chapters(id)
);

CREATE INDEX idx_chunks_chapter ON chunks(chapter_id);
CREATE INDEX idx_chunks_type ON chunks(chunk_type);

-- 向量表 (sqlite-vec 扩展)
CREATE VIRTUAL TABLE vec_chunks USING vec0(
    chunk_id TEXT PRIMARY KEY,
    embedding FLOAT[1024]           -- bge-m3 维度
);
```

### 12.5 Data Flow

#### 12.5.1 Import-time Embedding Generation

```
用户导入书籍
     │
     ▼
┌─────────────────────┐
│  Rust: 解析章节     │
│  章节分割 → 保存    │
└─────────────────────┘
     │
     ▼ (后台任务)
┌─────────────────────┐
│  Python: 分块       │
│  段落级分块         │
│  提取时间标记       │
└─────────────────────┘
     │
     ▼
┌─────────────────────┐
│  Python: Embedding  │
│  bge-m3 / OpenAI    │
│  批量生成向量       │
└─────────────────────┘
     │
     ▼
┌─────────────────────┐
│  Rust: 存储         │
│  chunks + vectors   │
│  → vectors.db       │
└─────────────────────┘
```

#### 12.5.2 Analysis-time Context Retrieval

```
用户点击 [分析第 N 章]
     │
     ▼
┌─────────────────────────────────────────────────────┐
│  Rust: 构建分析上下文                                │
│                                                      │
│  1. 读取当前章节内容                                 │
│  2. 提取章节中的实体 mention                         │
│  3. 向量检索：                                       │
│     ├── 当前章节 embedding → 相似历史段落            │
│     ├── 实体名称 → 该实体的历史描述                  │
│     └── 时间标记 → 相关时间点事件                    │
│  4. 组装上下文 (≤8K tokens)                         │
└─────────────────────────────────────────────────────┘
     │
     ▼
┌─────────────────────────────────────────────────────┐
│  Python: LLM 分析                                    │
│                                                      │
│  Prompt 结构:                                        │
│  ───────────────────────────────────────────────     │
│  # 已知信息                                          │
│  ## 相关人物                                         │
│  - 萧炎: [检索到的描述]                              │
│  - 美杜莎: [检索到的描述]                            │
│                                                      │
│  ## 相关历史事件                                     │
│  - 第42章: 萧炎与美杜莎初遇...                       │
│  - 第156章: 契约签订...                              │
│                                                      │
│  ## 相关设定                                         │
│  - 异火: [检索到的设定描述]                          │
│                                                      │
│  # 当前章节                                          │
│  [完整章节内容]                                      │
│                                                      │
│  # 任务                                              │
│  分析写作技法，提取人物/设定/事件...                 │
│  ───────────────────────────────────────────────     │
└─────────────────────────────────────────────────────┘
```

#### 12.5.3 Story Bible Update Sync

```
用户编辑人物 "萧炎" 的描述
     │
     ▼
┌─────────────────────┐
│  Rust: 更新 bible   │
│  保存新描述         │
└─────────────────────┘
     │
     ▼
┌─────────────────────┐
│  触发 Embedding 更新│
│  chunk_type=entity  │
│  重新生成该实体向量 │
└─────────────────────┘
```

### 12.6 Retrieval Strategy

#### 12.6.1 Multi-path Retrieval for Chapter Analysis

```rust
pub struct AnalysisContext {
    pub current_chapter: ChapterContent,
    pub related_characters: Vec<CharacterContext>,
    pub related_settings: Vec<SettingContext>,
    pub related_events: Vec<EventContext>,
    pub similar_passages: Vec<SimilarPassage>,
}

pub async fn build_analysis_context(
    chapter_id: ChapterId,
    vector_db: &VectorDb,
    bible_db: &BibleDb,
) -> Result<AnalysisContext> {
    let chapter = load_chapter(chapter_id).await?;
    
    // 1. 提取当前章节的实体 mention
    let mentions = extract_entity_mentions(&chapter.content);
    
    // 2. 检索相关人物（从故事圣经 + 向量检索）
    let mut characters = vec![];
    for mention in mentions.characters {
        if let Some(char) = bible_db.find_character(&mention).await? {
            let history = vector_db.search(
                query: &mention,
                filter: ChunkType::Paragraph,
                entity_filter: Some(char.id),
                top_k: 5,
            ).await?;
            characters.push(CharacterContext { char, history });
        }
    }
    
    // 3. 检索相似历史段落（语义相似）
    let chapter_embedding = generate_embedding(&chapter.content).await?;
    let similar = vector_db.search_by_vector(
        embedding: chapter_embedding,
        exclude_chapter: chapter_id,
        top_k: 10,
    ).await?;
    
    Ok(AnalysisContext {
        current_chapter: chapter,
        related_characters: characters,
        related_settings: settings,
        related_events: events,
        similar_passages: similar,
    })
}
```

#### 12.6.2 Hybrid Search

```rust
pub async fn hybrid_search(
    query: &str,
    vector_db: &VectorDb,
    fts_db: &FtsDb,
    top_k: usize,
) -> Result<Vec<SearchResult>> {
    // 并行执行向量搜索和关键词搜索
    let (vector_results, keyword_results) = tokio::join!(
        vector_db.search(query, top_k * 2),
        fts_db.search(query, top_k * 2),
    );
    
    // RRF (Reciprocal Rank Fusion) 合并结果
    let merged = rrf_merge(vector_results?, keyword_results?, k=60);
    
    Ok(merged.into_iter().take(top_k).collect())
}
```

### 12.7 RAG-enabled Features

| 功能 | 说明 |
|------|------|
| **智能实体识别** | 自动关联"老者"→"药老" |
| **关系演变追踪** | 检索角色互动历史，分析关系变化 |
| **伏笔检测** | 相似描述匹配，发现伏笔-回收对 |
| **时间线重建** | 检索时间标记锚点，构建准确时间线 |
| **语义搜索** | 用户查询"所有战斗场景"→返回相关段落 |
| **风格分析** | 检索相似风格段落，分析作者技法 |

### 12.8 Project Structure Changes

#### 12.8.1 New Rust Modules

```
src-tauri/src/
├── core/
│   └── embedding.rs          [NEW] Chunk, ChunkType, VectorEntry
│
├── storage/
│   ├── vectors.rs            [NEW] 向量存储层
│   │   ├── init_vector_db()
│   │   ├── insert_chunks()
│   │   ├── search_similar()
│   │   └── update_chunk()
│   └── (schema in code)      # vectors.db schema is defined in Rust (storage/vectors.rs)
│
├── retrieval/
│   ├── mod.rs
│   ├── search.rs             [EXISTING] 全文搜索
│   ├── vector_search.rs      [NEW] 向量检索
│   │   ├── search_by_text()
│   │   ├── search_by_entities()
│   │   └── hybrid_search()
│   └── context_builder.rs    [NEW] 分析上下文构建
│       ├── build_analysis_context()
│       └── build_query_context()
│
├── commands/
│   └── embedding.rs          [NEW] 
│       ├── generate_chapter_embeddings()
│       ├── search_similar_chunks()
│       └── rebuild_embeddings()
```

#### 12.8.2 New Python Modules

```
python/loom/
├── embedding/                [NEW]
│   ├── __init__.py
│   ├── base.py               # EmbeddingProvider 基类
│   ├── local.py              # 本地模型 (bge-m3 via sentence-transformers)
│   ├── openai.py             # OpenAI API
│   └── chunker.py            # 中文分块逻辑
│
├── retrieval/                [NEW]
│   ├── __init__.py
│   └── context.py            # 上下文检索逻辑
```

### 12.9 Embedding Configuration

```rust
/// Embedding 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    pub provider: EmbeddingProvider,
    pub model: String,
    pub dimensions: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmbeddingProvider {
    Local,              // bge-m3 via sentence-transformers
    OpenAI,             // text-embedding-3-small
    Custom(String),     // 自定义 API 端点
}
```

```toml
# 离线模式（推荐）
[embedding]
provider = "local"
model = "BAAI/bge-m3"
dimensions = 1024

# 在线模式
[embedding]
provider = "openai"
model = "text-embedding-3-small"
dimensions = 1536
```

### 12.10 IPC Protocol Changes

#### 12.10.1 New Tauri Commands

```typescript
// Embedding 生成
invoke('generate_chapter_embeddings', { chapterId }) -> void
invoke('rebuild_book_embeddings', { bookId }) -> void

// 向量检索
invoke('search_similar', { query, topK, filters }) -> SimilarChunk[]
invoke('get_analysis_context', { chapterId }) -> AnalysisContext

// 语义搜索
invoke('semantic_search', { bookId, query, topK }) -> SearchResult[]
```

#### 12.10.2 New Python JSON-RPC Methods

```python
@rpc_method
def chunk_text(text: str, chunk_size: int = 400) -> List[ChunkInfo]:
    """将文本分块"""
    pass

@rpc_method  
def generate_embeddings(texts: List[str]) -> List[List[float]]:
    """批量生成 embedding"""
    pass

@rpc_method
def generate_embedding(text: str) -> List[float]:
    """单条 embedding"""
    pass
```

### 12.11 Performance Considerations

| 方面 | 设计 |
|------|------|
| **内存** | Embedding 模型仅在需要时加载，生成完卸载 |
| **存储** | 每本书 +50-200MB (取决于章节数) |
| **导入速度** | 后台异步生成，不阻塞 UI |
| **检索延迟** | sqlite-vec 单次查询 <50ms |
| **批量处理** | Embedding 生成批量处理 (batch=32) |

---

## 13. Technical Considerations

### 13.1 Memory Management

| Strategy | Implementation |
|----------|----------------|
| 一书一库 | 每本书独立 SQLite，按需加载 |
| 流式读取 | 章节内容流式读取，不全量加载 |
| 按需渲染 | 虚拟滚动长列表 |
| 卡片分页 | 收件箱/圣经分页加载 |

### 13.2 Performance Targets

| Metric | Target |
|--------|--------|
| 启动时间 | < 2s |
| 书库加载 | < 500ms (100本书) |
| 章节加载 | < 200ms |
| 分析速度 | < 30s/章 |
| 内存占用 | < 200MB |

---

## 14. Success Metrics

### 14.1 Functional

| Metric | Target |
|--------|--------|
| 支持书籍数量 | 无限制 |
| 支持章节数量 | 单书 2000+ 章 |
| 证据覆盖率 | 100% 卡片有证据 |
| 入库准确率 | 用户接受率 > 80% |

### 14.2 User Experience

| Metric | Target |
|--------|--------|
| 导入成功率 | > 95% |
| 分析完成率 | > 99% |
| 用户留存 | 周活 > 50% |

---

*End of Document*
