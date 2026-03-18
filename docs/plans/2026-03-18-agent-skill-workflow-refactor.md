# Agent Runtime Refactor to Skill + Workflow Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 在保留前端 `Agent` 文案和现有命令入口兼容的前提下，将分析运行时重构为 `Skill + Workflow` 架构，移除 silent fallback，并增加显式 run/step trace。

**Architecture:** 保持 `Tauri/Vue -> Rust Commands -> Python Sidecar -> LLM Provider` 主链路不变。Rust 侧新增 `SkillRegistry + WorkflowRunner + ContextResolver + RunStore` 作为控制面，Python 侧新增 `SkillExecutor + PromptRenderer + SchemaValidator + Postprocessors` 作为执行面；旧 `AgentConfig` 和 `TaskBindings` 先做兼容映射，不直接暴露内部 `Skill` 概念到 UI。

**Tech Stack:** Vue 3 + Pinia、Tauri 2 / Rust、SQLite（`book.db`）、Python 3.10 sidecar（httpx / pydantic）、Cargo test、pytest、vue-tsc / eslint。

---

### Task 1: 为现状行为补 run 状态骨架与错误码枚举

**Files:**
- Create: `src-tauri/src/core/run_status.rs`
- Modify: `src-tauri/src/core/mod.rs`
- Modify: `src-tauri/src/commands/analysis.rs`
- Test: `src-tauri/src/core/run_status.rs`

**Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::{AnalysisErrorCode, RunStatus};

    #[test]
    fn test_run_status_and_error_code_wire_format() {
        assert_eq!(RunStatus::Degraded.as_str(), "degraded");
        assert_eq!(AnalysisErrorCode::SkillBindingMissing.as_str(), "SKILL_BINDING_MISSING");
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test core::run_status::tests::test_run_status_and_error_code_wire_format -- --nocapture`
Expected: FAIL，提示 `run_status` 模块或枚举不存在。

**Step 3: Write minimal implementation**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    Pending,
    Running,
    Success,
    Degraded,
    Failed,
    Cancelled,
}

impl RunStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Success => "success",
            Self::Degraded => "degraded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalysisErrorCode {
    SkillBindingMissing,
    SkillNotFound,
    ProviderNotFound,
    ModelInvalid,
    SchemaValidationFailed,
    EmbeddingRequiredButUnavailable,
    ContextRequiredButUnavailable,
    SidecarCallFailed,
}
```

**Step 4: Run test to verify it passes**

Run: `cd src-tauri && cargo test core::run_status::tests -- --nocapture`
Expected: PASS。

**Step 5: Commit**

```bash
git add src-tauri/src/core/run_status.rs src-tauri/src/core/mod.rs src-tauri/src/commands/analysis.rs
git commit -m "feat(runtime): add run status and analysis error codes"
```

---

### Task 2: 去掉 Rust 侧默认 provider fallback

**Files:**
- Modify: `src-tauri/src/commands/analysis.rs`
- Test: `src-tauri/src/commands/analysis.rs`（新增 `#[cfg(test)]` 单元测试）

**Step 1: Write the failing test**

```rust
#[test]
fn test_build_agent_configs_fails_when_binding_missing() {
    let err = resolve_agent_configs_for_analysis(/* bindings missing */).unwrap_err();
    assert!(err.contains("SKILL_BINDING_MISSING"));
}
```

**Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test commands::analysis::tests::test_build_agent_configs_fails_when_binding_missing -- --nocapture`
Expected: FAIL，当前逻辑会 fallback 到第一个 enabled provider。

**Step 3: Write minimal implementation**

```rust
if agent_configs.is_empty() {
    return Err(format!(
        "SKILL_BINDING_MISSING: no task binding resolved for requested analysis types"
    ));
}
```

并将 `build agent configs` 逻辑抽成单独 helper，便于测试。

**Step 4: Run test to verify it passes**

Run: `cd src-tauri && cargo test commands::analysis::tests::test_build_agent_configs_fails_when_binding_missing -- --nocapture`
Expected: PASS。

**Step 5: Commit**

```bash
git add src-tauri/src/commands/analysis.rs
git commit -m "fix(runtime): fail fast when analysis bindings are missing"
```

---

### Task 3: 去掉 Python 侧 legacy provider fallback

**Files:**
- Modify: `python/loom/server.py`
- Test: `python/tests/test_server_runtime.py`

**Step 1: Write the failing test**

```python
import pytest
from loom.server import analyze_chapter

@pytest.mark.asyncio
async def test_analyze_chapter_fails_without_agent_config():
    result = await analyze_chapter({
        "content": "text",
        "book_title": "b",
        "chapter_index": 1,
        "analysis_types": ["technique"],
        "agent_configs": {},
    })
    assert result["cancelled"] is False
    assert result.get("error_code") == "SKILL_CONFIG_UNRESOLVED"
```

**Step 2: Run test to verify it fails**

Run: `cd python && pytest tests/test_server_runtime.py::test_analyze_chapter_fails_without_agent_config -q`
Expected: FAIL，当前逻辑会走 legacy provider fallback。

**Step 3: Write minimal implementation**

```python
if analysis_type not in agent_configs:
    raise RuntimeError("SKILL_CONFIG_UNRESOLVED: missing agent config for analysis type")
```

并删除 `legacy_provider_config` 运行时 fallback 分支，只在迁移脚本中保留兼容读取。

**Step 4: Run test to verify it passes**

Run: `cd python && pytest tests/test_server_runtime.py::test_analyze_chapter_fails_without_agent_config -q`
Expected: PASS。

**Step 5: Commit**

```bash
git add python/loom/server.py python/tests/test_server_runtime.py
git commit -m "fix(sidecar): remove legacy provider fallback from analysis runtime"
```

---

### Task 4: 删除 dummy embedding fallback 并区分 required/optional context

**Files:**
- Modify: `src-tauri/src/commands/embedding.rs`
- Modify: `src-tauri/src/commands/analysis.rs`
- Test: `src-tauri/src/commands/embedding.rs`
- Test: `src-tauri/src/commands/analysis.rs`

**Step 1: Write the failing test**

```rust
#[tokio::test]
async fn test_generate_real_embedding_returns_error_on_provider_failure() {
    let err = generate_real_embedding_strict("hello").await.unwrap_err();
    assert!(err.contains("Embedding generation failed"));
}
```

**Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test commands::embedding::tests::test_generate_real_embedding_returns_error_on_provider_failure -- --nocapture`
Expected: FAIL，当前逻辑会生成 dummy embedding。

**Step 3: Write minimal implementation**

```rust
async fn generate_real_embedding_strict(text: &str) -> Result<Vec<f32>, String> {
    crate::sidecar::generate_embedding(text)
        .await
        .map_err(|e| format!("Embedding generation failed: {}", e))
}
```

并在分析上下文构建中区分：
- `required` 的 embedding/context 获取失败 -> `Err`
- `optional` 的 similar passages 获取失败 -> 记录 `degraded` 原因并继续

**Step 4: Run test to verify it passes**

Run: `cd src-tauri && cargo test commands::embedding::tests commands::analysis::tests -- --nocapture`
Expected: PASS。

**Step 5: Commit**

```bash
git add src-tauri/src/commands/embedding.rs src-tauri/src/commands/analysis.rs
git commit -m "fix(runtime): remove dummy embedding fallback and surface degraded context"
```

---

### Task 5: 新增 SkillPack 核心模型与资源加载器

**Files:**
- Create: `src-tauri/src/core/skill.rs`
- Create: `src-tauri/src/workflow/registry.rs`
- Modify: `src-tauri/src/core/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/src/core/skill.rs`
- Create: `src-tauri/resources/skills/builtin/.gitkeep`

**Step 1: Write the failing test**

```rust
#[test]
fn test_skill_pack_manifest_deserializes() {
    let json = r#"{
        \"id\": \"character_extraction_v1\",
        \"version\": \"1.0.0\",
        \"task_type\": \"character_extraction\",
        \"enabled\": true
    }"#;
    let skill: SkillPack = serde_json::from_str(json).unwrap();
    assert_eq!(skill.id, "character_extraction_v1");
}
```

**Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test core::skill::tests::test_skill_pack_manifest_deserializes -- --nocapture`
Expected: FAIL。

**Step 3: Write minimal implementation**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillPack {
    pub id: String,
    pub version: String,
    pub task_type: String,
    pub enabled: bool,
    pub provider_policy: Option<ProviderPolicy>,
    pub context_policy: Option<ContextPolicy>,
    pub output_contract: Option<OutputContract>,
}
```

并提供 `SkillRegistry::load_builtin_skills()`，从 `src-tauri/resources/skills/builtin` 读取 manifest。

**Step 4: Run test to verify it passes**

Run: `cd src-tauri && cargo test core::skill::tests -- --nocapture`
Expected: PASS。

**Step 5: Commit**

```bash
git add src-tauri/src/core/skill.rs src-tauri/src/workflow/registry.rs src-tauri/src/core/mod.rs src-tauri/src/lib.rs src-tauri/resources/skills/builtin
git commit -m "feat(skill): add skill pack models and registry"
```

---

### Task 6: 把五个内置 Agent 迁移为 built-in skill 资源

**Files:**
- Create: `src-tauri/resources/skills/builtin/technique_analysis_v1/manifest.json`
- Create: `src-tauri/resources/skills/builtin/technique_analysis_v1/system.md`
- Create: `src-tauri/resources/skills/builtin/technique_analysis_v1/user.md`
- Create: `src-tauri/resources/skills/builtin/technique_analysis_v1/schema.json`
- Create: `src-tauri/resources/skills/builtin/character_extraction_v1/manifest.json`
- Create: `src-tauri/resources/skills/builtin/character_extraction_v1/system.md`
- Create: `src-tauri/resources/skills/builtin/character_extraction_v1/user.md`
- Create: `src-tauri/resources/skills/builtin/character_extraction_v1/schema.json`
- Create: `src-tauri/resources/skills/builtin/setting_extraction_v1/*`
- Create: `src-tauri/resources/skills/builtin/event_extraction_v1/*`
- Create: `src-tauri/resources/skills/builtin/style_analysis_v1/*`
- Test: `src-tauri/src/workflow/registry.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn test_builtin_skill_registry_contains_five_analysis_skills() {
    let registry = SkillRegistry::load_builtin_skills_from_resources().unwrap();
    assert!(registry.get("technique_analysis_v1").is_some());
    assert!(registry.get("character_extraction_v1").is_some());
    assert!(registry.get("setting_extraction_v1").is_some());
    assert!(registry.get("event_extraction_v1").is_some());
    assert!(registry.get("style_analysis_v1").is_some());
}
```

**Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test workflow::registry::tests::test_builtin_skill_registry_contains_five_analysis_skills -- --nocapture`
Expected: FAIL。

**Step 3: Write minimal implementation**

将当前 [python/loom/agents/technique.py](/mnt/e/rust_project/narrative-loom/python/loom/agents/technique.py)、[python/loom/agents/character.py](/mnt/e/rust_project/narrative-loom/python/loom/agents/character.py)、[python/loom/agents/setting.py](/mnt/e/rust_project/narrative-loom/python/loom/agents/setting.py)、[python/loom/agents/event.py](/mnt/e/rust_project/narrative-loom/python/loom/agents/event.py)、[python/loom/agents/style.py](/mnt/e/rust_project/narrative-loom/python/loom/agents/style.py) 中的 prompt/schema 搬到资源目录。

**Step 4: Run test to verify it passes**

Run: `cd src-tauri && cargo test workflow::registry::tests -- --nocapture`
Expected: PASS。

**Step 5: Commit**

```bash
git add src-tauri/resources/skills/builtin src-tauri/src/workflow/registry.rs
git commit -m "feat(skill): migrate built-in analysis agents to resource-backed skills"
```

---

### Task 7: 新增 Agent -> Skill 兼容映射层，保持 UI 文案不变

**Files:**
- Create: `src-tauri/src/core/agent_alias.rs`
- Modify: `src-tauri/src/core/agent.rs`
- Modify: `src-tauri/src/storage/config.rs`
- Modify: `src-tauri/src/commands/settings.rs`
- Modify: `src/stores/settings.ts`
- Test: `src-tauri/src/core/agent_alias.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn test_builtin_agent_id_maps_to_builtin_skill_id() {
    assert_eq!(map_agent_id_to_skill_id("character-extraction"), Some("character_extraction_v1"));
}
```

**Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test core::agent_alias::tests::test_builtin_agent_id_maps_to_builtin_skill_id -- --nocapture`
Expected: FAIL。

**Step 3: Write minimal implementation**

```rust
pub fn map_agent_id_to_skill_id(agent_id: &str) -> Option<&'static str> {
    match agent_id {
        "technique-analysis" => Some("technique_analysis_v1"),
        "character-extraction" => Some("character_extraction_v1"),
        "setting-extraction" => Some("setting_extraction_v1"),
        "event-extraction" => Some("event_extraction_v1"),
        "style-analysis" => Some("style_analysis_v1"),
        _ => None,
    }
}
```

同时保持 `settings::get_agents/save_agent/delete_agent` 对旧 `AgentConfig` 的兼容返回。

**Step 4: Run test to verify it passes**

Run: `cd src-tauri && cargo test core::agent_alias::tests -- --nocapture`
Expected: PASS。

**Step 5: Commit**

```bash
git add src-tauri/src/core/agent_alias.rs src-tauri/src/core/agent.rs src-tauri/src/storage/config.rs src-tauri/src/commands/settings.rs src/stores/settings.ts
git commit -m "feat(compat): keep agent-facing API while mapping runtime to skills"
```

---

### Task 8: 引入 WorkflowSpec 与 WorkflowRegistry

**Files:**
- Create: `src-tauri/src/core/workflow.rs`
- Modify: `src-tauri/src/workflow/registry.rs`
- Create: `src-tauri/resources/workflows/builtin/chapter_analysis_v2/manifest.json`
- Create: `src-tauri/resources/workflows/builtin/single_skill_analysis_v1/manifest.json`
- Test: `src-tauri/src/core/workflow.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn test_workflow_spec_deserializes_steps() {
    let json = r#"{
        \"id\": \"chapter_analysis_v2\",
        \"version\": \"1.0.0\",
        \"target\": \"chapter\",
        \"error_policy\": \"fail_fast\",
        \"steps\": [\"prepare_context\", \"run:character_extraction_v1\"]
    }"#;
    let spec: WorkflowSpec = serde_json::from_str(json).unwrap();
    assert_eq!(spec.steps.len(), 2);
}
```

**Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test core::workflow::tests::test_workflow_spec_deserializes_steps -- --nocapture`
Expected: FAIL。

**Step 3: Write minimal implementation**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSpec {
    pub id: String,
    pub version: String,
    pub target: String,
    pub error_policy: String,
    pub steps: Vec<String>,
}
```

并实现 `WorkflowRegistry::load_builtin_workflows()`。

**Step 4: Run test to verify it passes**

Run: `cd src-tauri && cargo test core::workflow::tests workflow::registry::tests -- --nocapture`
Expected: PASS。

**Step 5: Commit**

```bash
git add src-tauri/src/core/workflow.rs src-tauri/src/workflow/registry.rs src-tauri/resources/workflows/builtin
git commit -m "feat(workflow): add workflow specs and builtin registry"
```

---

### Task 9: 新增 ContextResolver，按 skill context policy 裁剪上下文

**Files:**
- Create: `src-tauri/src/workflow/context_resolver.rs`
- Modify: `src-tauri/src/commands/analysis.rs`
- Modify: `src-tauri/src/retrieval/context_builder.rs`
- Test: `src-tauri/src/workflow/context_resolver.rs`
- Test: `src-tauri/tests/entity_recall_context_integration.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn test_context_resolver_skips_known_events_when_policy_off() {
    let resolved = resolve_context(/* policy with known_events off */).unwrap();
    assert!(resolved.known_events.is_none());
}
```

**Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test workflow::context_resolver::tests::test_context_resolver_skips_known_events_when_policy_off -- --nocapture`
Expected: FAIL。

**Step 3: Write minimal implementation**

```rust
pub struct ResolvedAnalysisContext {
    pub known_characters: Option<serde_json::Value>,
    pub known_settings: Option<serde_json::Value>,
    pub known_events: Option<serde_json::Value>,
    pub degraded_reasons: Vec<String>,
}
```

并将当前 [analysis.rs](/mnt/e/rust_project/narrative-loom/src-tauri/src/commands/analysis.rs) 中统一构建上下文的逻辑搬到 `ContextResolver`。

**Step 4: Run test to verify it passes**

Run: `cd src-tauri && cargo test workflow::context_resolver::tests src-tauri/tests/entity_recall_context_integration.rs -- --nocapture`
Expected: PASS。

**Step 5: Commit**

```bash
git add src-tauri/src/workflow/context_resolver.rs src-tauri/src/commands/analysis.rs src-tauri/src/retrieval/context_builder.rs src-tauri/tests/entity_recall_context_integration.rs
git commit -m "feat(workflow): resolve analysis context from skill policies"
```

---

### Task 10: 新增 RunStore 并扩展 book.db schema / migration

**Files:**
- Create: `src-tauri/src/storage/run_store.rs`
- Modify: `src-tauri/src/storage/schema.sql`
- Modify: `src-tauri/src/storage/migration.rs`
- Modify: `src-tauri/src/storage/mod.rs`
- Test: `src-tauri/tests/run_store_integration.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn test_analysis_run_and_step_run_persist() {
    let store = make_temp_run_store();
    let run_id = store.create_run(/* ... */).unwrap();
    store.create_step_run(run_id.clone(), /* ... */).unwrap();
    let run = store.get_run(&run_id).unwrap().unwrap();
    assert_eq!(run.status, "running");
}
```

**Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test --test run_store_integration -- --nocapture`
Expected: FAIL，表或 store 不存在。

**Step 3: Write minimal implementation**

在 `schema.sql` 新增：

```sql
CREATE TABLE IF NOT EXISTS analysis_runs (
    id TEXT PRIMARY KEY,
    workflow_id TEXT NOT NULL,
    workflow_version TEXT NOT NULL,
    chapter_id TEXT,
    status TEXT NOT NULL,
    error_code TEXT,
    error_message TEXT,
    degraded_json TEXT,
    started_at TEXT NOT NULL,
    finished_at TEXT
);

CREATE TABLE IF NOT EXISTS analysis_step_runs (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL,
    step_id TEXT NOT NULL,
    skill_id TEXT,
    skill_version TEXT,
    provider TEXT,
    model TEXT,
    status TEXT NOT NULL,
    duration_ms INTEGER,
    input_meta_json TEXT,
    error_code TEXT,
    error_message TEXT,
    degraded_json TEXT,
    FOREIGN KEY (run_id) REFERENCES analysis_runs(id) ON DELETE CASCADE
);
```

**Step 4: Run test to verify it passes**

Run: `cd src-tauri && cargo test --test run_store_integration -- --nocapture`
Expected: PASS。

**Step 5: Commit**

```bash
git add src-tauri/src/storage/run_store.rs src-tauri/src/storage/schema.sql src-tauri/src/storage/migration.rs src-tauri/src/storage/mod.rs src-tauri/tests/run_store_integration.rs
git commit -m "feat(runtime): persist workflow runs and step traces"
```

---

### Task 11: 引入 Python SkillExecutor / PromptRenderer / SchemaValidator

**Files:**
- Create: `python/loom/execution/skill_executor.py`
- Create: `python/loom/execution/prompt_renderer.py`
- Create: `python/loom/execution/schema_validator.py`
- Create: `python/tests/test_skill_executor.py`
- Modify: `python/loom/server.py`

**Step 1: Write the failing test**

```python
import pytest
from loom.execution.skill_executor import execute_skill

@pytest.mark.asyncio
async def test_execute_skill_renders_prompt_and_validates_schema(fake_provider):
    result = await execute_skill(
        skill={"id": "technique_analysis_v1", "output_contract": {"mode": "json_schema", "schema": {"type": "object"}}},
        context={"book_title": "b", "chapter_index": 1, "chapter_content": "text"},
        provider=fake_provider,
    )
    assert isinstance(result, dict)
```

**Step 2: Run test to verify it fails**

Run: `cd python && pytest tests/test_skill_executor.py::test_execute_skill_renders_prompt_and_validates_schema -q`
Expected: FAIL。

**Step 3: Write minimal implementation**

```python
async def execute_skill(skill: dict, context: dict, provider) -> dict:
    messages = render_messages(skill, context)
    result = await provider.complete_json(build_request(skill, messages), skill["output_contract"]["schema"])
    validate_output(result, skill["output_contract"]["schema"])
    return result
```

并将 `server.py` 中的 `create_agent(...).analyze(...)` 逐步替换为 `execute_skill(...)`。

**Step 4: Run test to verify it passes**

Run: `cd python && pytest tests/test_skill_executor.py -q`
Expected: PASS。

**Step 5: Commit**

```bash
git add python/loom/execution python/loom/server.py python/tests/test_skill_executor.py
git commit -m "feat(sidecar): add skill executor, prompt renderer, and schema validator"
```

---

### Task 12: 新增 WorkflowRunner 并统一单 skill / 章节 / 批量分析入口

**Files:**
- Create: `src-tauri/src/workflow/runner.rs`
- Modify: `src-tauri/src/commands/analysis.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/tests/workflow_runner_integration.rs`

**Step 1: Write the failing test**

```rust
#[tokio::test]
async fn test_workflow_runner_executes_single_skill_workflow() {
    let result = run_single_skill_workflow_for_test("character_extraction_v1").await.unwrap();
    assert_eq!(result.status, "success");
}
```

**Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test --test workflow_runner_integration -- --nocapture`
Expected: FAIL。

**Step 3: Write minimal implementation**

```rust
pub struct WorkflowRunner {
    skill_registry: SkillRegistry,
    workflow_registry: WorkflowRegistry,
}

impl WorkflowRunner {
    pub async fn run(&self, workflow_id: &str, request: WorkflowRequest) -> Result<WorkflowRunResult, String> {
        // resolve workflow -> resolve skill(s) -> resolve context -> call sidecar -> persist run
    }
}
```

并将：
- `analyze_single_agent` 改为内部调用 `single_skill_analysis_v1`
- `analyze_chapter` 改为内部调用 `chapter_analysis_v2`
- `batch_analyze_chapters` 改为批量调度 workflow，而不是直接逐个调用旧逻辑

**Step 4: Run test to verify it passes**

Run: `cd src-tauri && cargo test --test workflow_runner_integration -- --nocapture`
Expected: PASS。

**Step 5: Commit**

```bash
git add src-tauri/src/workflow/runner.rs src-tauri/src/commands/analysis.rs src-tauri/src/lib.rs src-tauri/tests/workflow_runner_integration.rs
git commit -m "feat(workflow): unify single-agent, chapter, and batch analysis execution"
```

---

### Task 13: 前端保持 Agent 文案，新增 degraded / failed 可视状态

**Files:**
- Modify: `src/stores/analysis.ts`
- Modify: `src/stores/settings.ts`
- Modify: `src/components/settings/TaskBindings.vue`
- Modify: `src/views/SettingsView.vue`
- Test: `npm run type-check`
- Test: `npm run lint`

**Step 1: Write the failing test / type assertion**

```ts
const status: 'success' | 'degraded' | 'failed' = payload.status
```

并补一个最小 store 单元断言文件，如果项目决定引入前端测试框架，则放到 `src/stores/__tests__/analysis-status.spec.ts`；若本轮不引入，则以 `type-check + lint` 作为门禁。

**Step 2: Run verification to see failure**

Run: `npm run type-check`
Expected: FAIL，当前类型未覆盖 `degraded`。

**Step 3: Write minimal implementation**

- `settings.ts` 继续暴露 `AgentConfig`，但增加内部 `runtimeSkillId` 可选字段
- `analysis.ts` 的 run/progress 状态新增 `degraded`
- `TaskBindings.vue` 文案保持“绑定的 Agent”，不改用户术语
- UI 在分析结果和进度提示中显示 `degraded` 原因

**Step 4: Run verification to see success**

Run: `npm run type-check && npm run lint`
Expected: PASS。

**Step 5: Commit**

```bash
git add src/stores/analysis.ts src/stores/settings.ts src/components/settings/TaskBindings.vue src/views/SettingsView.vue
git commit -m "feat(ui): preserve agent wording while surfacing degraded runtime status"
```

---

### Task 14: 新增配置迁移与兼容保存路径

**Files:**
- Modify: `src-tauri/src/storage/config.rs`
- Modify: `src-tauri/src/commands/settings.rs`
- Test: `src-tauri/tests/config_skill_migration.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn test_legacy_agent_config_migrates_to_skill_aliases() {
    let migrated = migrate_legacy_agents_to_skill_aliases(sample_legacy_agents());
    assert_eq!(migrated.bindings.get("character-extraction"), Some(&"character_extraction_v1".to_string()));
}
```

**Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test --test config_skill_migration -- --nocapture`
Expected: FAIL。

**Step 3: Write minimal implementation**

```rust
pub fn migrate_legacy_agents_to_skill_aliases(/* ... */) -> SkillAliasMigrationResult {
    // map built-in agent ids to skill ids and preserve user-facing agent config
}
```

并确保：
- 旧 `agents.json` 仍可读
- 新格式保存为 `skills.json` 或 `runtime_skills.json`
- `get_agents` 继续返回 UI 需要的数据

**Step 4: Run test to verify it passes**

Run: `cd src-tauri && cargo test --test config_skill_migration -- --nocapture`
Expected: PASS。

**Step 5: Commit**

```bash
git add src-tauri/src/storage/config.rs src-tauri/src/commands/settings.rs src-tauri/tests/config_skill_migration.rs
git commit -m "feat(config): migrate legacy agents to runtime skill aliases"
```

---

### Task 15: 端到端回归与可选 MCP 边界层占位

**Files:**
- Create: `src-tauri/src/bin/narrative_loom_mcp.rs`
- Create: `src-tauri/tests/analysis_runtime_regression.rs`
- Modify: `README.md`
- Modify: `docs/plans/2026-03-18-agent-skill-workflow-refactor.md`（如执行中需回填）

**Step 1: Write the failing test**

```rust
#[tokio::test]
async fn test_analysis_runtime_exposes_degraded_without_hidden_fallback() {
    let result = run_analysis_with_missing_optional_context().await.unwrap();
    assert_eq!(result.status, "degraded");
    assert!(result.degraded_reasons.len() > 0);
}
```

**Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test --test analysis_runtime_regression -- --nocapture`
Expected: FAIL。

**Step 3: Write minimal implementation**

- 增加只读 MCP 占位二进制，先不接入主链路，只复用 service 层查询 `book/chapter/run`
- 补 README 中的运行时说明：`Agent` 为 UI 名称，内部运行时基于 `Skill + Workflow`
- 回归测试覆盖：missing binding、missing provider、optional context degraded、success trace persisted

**Step 4: Run test to verify it passes**

Run: `cd src-tauri && cargo test --test analysis_runtime_regression -- --nocapture`
Expected: PASS。

**Step 5: Commit**

```bash
git add src-tauri/src/bin/narrative_loom_mcp.rs src-tauri/tests/analysis_runtime_regression.rs README.md
git commit -m "feat(runtime): add regression coverage and optional MCP boundary stub"
```

---

### Final Verification

**Step 1: Run Rust unit + integration tests**

Run: `cd src-tauri && cargo test -- --nocapture`
Expected: PASS，新增 runtime / workflow / migration / regression tests 全部通过。

**Step 2: Run Rust compile check**

Run: `cd src-tauri && cargo check`
Expected: PASS。

**Step 3: Run Python tests**

Run: `cd python && pytest -q`
Expected: PASS。

**Step 4: Run frontend static verification**

Run: `npm run type-check && npm run lint`
Expected: PASS。

**Step 5: Manual smoke checklist**

- 打开设置页，仍然显示 `Agent` 文案
- 任务绑定页仍显示“绑定的 Agent”
- 单 Agent 分析能跑通
- 章节分析能跑通
- 批量分析能跑通
- 缺 binding / provider 时明确失败
- optional context 缺失时显示 `degraded`
- run trace 能在数据库中查到

**Step 6: Final commit**

```bash
git add src-tauri src python src README.md docs/plans/2026-03-18-agent-skill-workflow-refactor.md
git commit -m "refactor(runtime): migrate analysis runtime to skill and workflow architecture"
```
