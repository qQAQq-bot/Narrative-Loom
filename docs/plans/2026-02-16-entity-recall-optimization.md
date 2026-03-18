# Entity Recall Optimization Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 提升章节分析阶段对 `character / setting / event` 三类实体的召回率，并保持上下文构建时延与兼容性可控。

**Architecture:** 在 `ContextBuilder` 上引入“多路候选召回（vector + keyword + history）→ 统一融合打分 → 实体覆盖重排 → Prompt 预算裁剪”的召回漏斗。保留现有命令接口，新增可观测诊断与回归指标，确保可验证演进。

**Tech Stack:** Rust（`src-tauri`）、SQLite（`book.db` + `vectors.db`）、Python sidecar embedding、Cargo test（unit + integration）。

---

### Task 1: 引入召回调参结构（不改行为）

**Files:**
- Modify: `src-tauri/src/retrieval/context_builder.rs`
- Modify: `src-tauri/src/retrieval/mod.rs`
- Test: `src-tauri/src/retrieval/context_builder.rs`（`#[cfg(test)]`）

**Step 1: Write the failing test**

```rust
#[test]
fn test_recall_tuning_defaults() {
    let tuning = EntityRecallTuning::default();
    assert_eq!(tuning.candidate_chunks_per_query, 60);
    assert_eq!(tuning.max_passages, 12);
    assert_eq!(tuning.min_character_coverage, 8);
    assert_eq!(tuning.min_setting_coverage, 6);
    assert_eq!(tuning.min_event_coverage, 6);
}
```

**Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test retrieval::context_builder::tests::test_recall_tuning_defaults -- --nocapture`  
Expected: FAIL，提示 `EntityRecallTuning` 未定义。

**Step 3: Write minimal implementation**

```rust
#[derive(Debug, Clone)]
pub struct EntityRecallTuning {
    pub candidate_chunks_per_query: usize,
    pub max_passages: usize,
    pub min_character_coverage: usize,
    pub min_setting_coverage: usize,
    pub min_event_coverage: usize,
}

impl Default for EntityRecallTuning {
    fn default() -> Self {
        Self {
            candidate_chunks_per_query: 60,
            max_passages: 12,
            min_character_coverage: 8,
            min_setting_coverage: 6,
            min_event_coverage: 6,
        }
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cd src-tauri && cargo test retrieval::context_builder::tests::test_recall_tuning_defaults -- --nocapture`  
Expected: PASS。

**Step 5: Commit**

```bash
git add src-tauri/src/retrieval/context_builder.rs src-tauri/src/retrieval/mod.rs
git commit -m "feat(retrieval): add entity recall tuning defaults"
```

---

### Task 2: 新增多路融合模块（纯函数优先）

**Files:**
- Create: `src-tauri/src/retrieval/recall_fusion.rs`
- Modify: `src-tauri/src/retrieval/mod.rs`
- Test: `src-tauri/src/retrieval/recall_fusion.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn test_fuse_chunk_scores_rrf_and_boosts() {
    let fused = fuse_chunk_scores(vec![/* vector */], vec![/* keyword */], vec![/* history */]);
    assert_eq!(fused[0].chunk_id, "c1");
    assert!(fused[0].score > fused[1].score);
}
```

**Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test retrieval::recall_fusion::tests::test_fuse_chunk_scores_rrf_and_boosts -- --nocapture`  
Expected: FAIL，提示 `fuse_chunk_scores` 未定义。

**Step 3: Write minimal implementation**

```rust
pub fn fuse_chunk_scores(
    vector: Vec<ScoredChunk>,
    keyword: Vec<ScoredChunk>,
    history: Vec<ScoredChunk>,
) -> Vec<FusedChunk> {
    // 先按 chunk_id 聚合，再按权重计算分数，最后排序
}
```

**Step 4: Run test to verify it passes**

Run: `cd src-tauri && cargo test retrieval::recall_fusion::tests -- --nocapture`  
Expected: PASS。

**Step 5: Commit**

```bash
git add src-tauri/src/retrieval/recall_fusion.rs src-tauri/src/retrieval/mod.rs
git commit -m "feat(retrieval): add multi-channel recall fusion"
```

---

### Task 3: 实现实体覆盖重排（防单类挤占）

**Files:**
- Modify: `src-tauri/src/retrieval/context_builder.rs`
- Test: `src-tauri/src/retrieval/context_builder.rs`（或新建 `src-tauri/src/retrieval/context_builder_tests.rs`）

**Step 1: Write the failing test**

```rust
#[test]
fn test_coverage_rerank_keeps_minimum_per_type() {
    let result = apply_entity_coverage_quota(/* candidates */, /* tuning */);
    assert!(result.characters.len() >= 8);
    assert!(result.settings.len() >= 6);
    assert!(result.events.len() >= 6);
}
```

**Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test retrieval::context_builder::tests::test_coverage_rerank_keeps_minimum_per_type -- --nocapture`  
Expected: FAIL。

**Step 3: Write minimal implementation**

```rust
fn apply_entity_coverage_quota(
    ranked: RankedEntities,
    tuning: &EntityRecallTuning,
) -> RankedEntities {
    // 先保底，再按分数补齐
}
```

**Step 4: Run test to verify it passes**

Run: `cd src-tauri && cargo test retrieval::context_builder::tests::test_coverage_rerank_keeps_minimum_per_type -- --nocapture`  
Expected: PASS。

**Step 5: Commit**

```bash
git add src-tauri/src/retrieval/context_builder.rs
git commit -m "feat(retrieval): add per-type entity coverage reranking"
```

---

### Task 4: 别名与归一化提及增强

**Files:**
- Modify: `src-tauri/src/retrieval/vector_search.rs`
- Modify: `src-tauri/src/commands/analysis.rs`（必要时透传 aliases）
- Test: `src-tauri/src/retrieval/vector_search.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn test_extract_entity_mentions_supports_alias() {
    // 实体主名“费舍尔·贝纳维德斯”，文本只出现“费舍尔”
    let mentions = extract_entity_mentions(text, &entities_with_alias);
    assert!(mentions.iter().any(|m| m.entity_id == "char_1"));
}
```

**Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test retrieval::vector_search::tests::test_extract_entity_mentions_supports_alias -- --nocapture`  
Expected: FAIL。

**Step 3: Write minimal implementation**

```rust
fn text_contains_entity_or_alias(text_lower: &str, entity: &EntityInfoExt) -> bool {
    // 主名 + aliases + 归一化名称匹配
}
```

**Step 4: Run test to verify it passes**

Run: `cd src-tauri && cargo test retrieval::vector_search::tests::test_extract_entity_mentions_supports_alias -- --nocapture`  
Expected: PASS。

**Step 5: Commit**

```bash
git add src-tauri/src/retrieval/vector_search.rs src-tauri/src/commands/analysis.rs
git commit -m "feat(retrieval): improve entity mention matching with aliases"
```

---

### Task 5: 引入 fallback 权重切换（降低波动）

**Files:**
- Modify: `src-tauri/src/retrieval/vector_search.rs`
- Modify: `src-tauri/src/retrieval/recall_fusion.rs`
- Test: `src-tauri/src/retrieval/recall_fusion.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn test_fallback_mode_increases_keyword_weight() {
    let weights = RecallWeights::for_mode(RecallMode::FallbackEmbedding);
    assert!(weights.keyword > weights.vector);
}
```

**Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test retrieval::recall_fusion::tests::test_fallback_mode_increases_keyword_weight -- --nocapture`  
Expected: FAIL。

**Step 3: Write minimal implementation**

```rust
pub enum RecallMode {
    Normal,
    FallbackEmbedding,
}
```

**Step 4: Run test to verify it passes**

Run: `cd src-tauri && cargo test retrieval::recall_fusion::tests::test_fallback_mode_increases_keyword_weight -- --nocapture`  
Expected: PASS。

**Step 5: Commit**

```bash
git add src-tauri/src/retrieval/vector_search.rs src-tauri/src/retrieval/recall_fusion.rs
git commit -m "feat(retrieval): switch fusion weights under fallback embedding mode"
```

---

### Task 6: 将融合链路接入 `build_smart_context`

**Files:**
- Modify: `src-tauri/src/retrieval/context_builder.rs`
- Modify: `src-tauri/src/commands/analysis.rs`
- Test: `src-tauri/tests/entity_recall_context_integration.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn test_build_smart_context_returns_balanced_entities() {
    let ctx = build_context_for_fixture("chapter_100");
    assert!(ctx.known_characters.len() >= 8);
    assert!(ctx.known_settings.len() >= 6);
    assert!(ctx.known_events.len() >= 6);
}
```

**Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test entity_recall_context_integration -- --nocapture`  
Expected: FAIL。

**Step 3: Write minimal implementation**

```rust
// 在 build_smart_context 中替换原单路 candidates 逻辑，接入 recall_fusion + coverage rerank
```

**Step 4: Run test to verify it passes**

Run: `cd src-tauri && cargo test entity_recall_context_integration -- --nocapture`  
Expected: PASS。

**Step 5: Commit**

```bash
git add src-tauri/src/retrieval/context_builder.rs src-tauri/src/commands/analysis.rs src-tauri/tests/entity_recall_context_integration.rs
git commit -m "feat(retrieval): integrate fused multi-channel recall into smart context"
```

---

### Task 7: 上下文预算裁剪与优先级验证

**Files:**
- Modify: `src-tauri/src/retrieval/context_builder.rs`
- Test: `src-tauri/src/retrieval/context_builder.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn test_prompt_budget_prioritizes_entity_summaries_before_passages() {
    let prompt = format_context_for_prompt(&ctx, 800);
    assert!(prompt.contains("【已知人物】"));
    assert!(prompt.contains("【已知设定】"));
    assert!(prompt.contains("【已知事件】"));
}
```

**Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test retrieval::context_builder::tests::test_prompt_budget_prioritizes_entity_summaries_before_passages -- --nocapture`  
Expected: FAIL。

**Step 3: Write minimal implementation**

```rust
// 预算裁剪顺序：entity summaries > similar passages
```

**Step 4: Run test to verify it passes**

Run: `cd src-tauri && cargo test retrieval::context_builder::tests::test_prompt_budget_prioritizes_entity_summaries_before_passages -- --nocapture`  
Expected: PASS。

**Step 5: Commit**

```bash
git add src-tauri/src/retrieval/context_builder.rs
git commit -m "feat(retrieval): enforce prompt budget priority for entity summaries"
```

---

### Task 8: 回归指标与最小门禁

**Files:**
- Create: `src-tauri/tests/entity_recall_metrics.rs`
- Create: `src-tauri/tests/fixtures/entity_recall/`（最小样本）
- Modify: `README.md`（增加开发验证命令）

**Step 1: Write the failing test**

```rust
#[test]
fn test_macro_recall_not_below_baseline() {
    let report = run_entity_recall_metrics();
    assert!(report.macro_recall >= 0.72);
}
```

**Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test entity_recall_metrics -- --nocapture`  
Expected: FAIL（基线未建立或指标不足）。

**Step 3: Write minimal implementation**

```rust
pub struct RecallReport {
    pub character_recall: f32,
    pub setting_recall: f32,
    pub event_recall: f32,
    pub macro_recall: f32,
}
```

**Step 4: Run test to verify it passes**

Run: `cd src-tauri && cargo test entity_recall_metrics -- --nocapture`  
Expected: PASS。

**Step 5: Commit**

```bash
git add src-tauri/tests/entity_recall_metrics.rs src-tauri/tests/fixtures/entity_recall README.md
git commit -m "test(retrieval): add entity recall regression metrics and baseline gate"
```

---

## Final Verification Sequence

Run in order:

1. `cd src-tauri && cargo test retrieval:: -- --nocapture`
2. `cd src-tauri && cargo test entity_recall_context_integration -- --nocapture`
3. `cd src-tauri && cargo test entity_recall_metrics -- --nocapture`
4. `cd src-tauri && cargo check`

Expected:
- 所有新增测试通过；
- 无编译错误；
- `macro_recall` 不低于基线阈值。

---

## Notes for Execution

- 全程严格 `TDD`：先失败测试，再最小实现，再重构。  
- 每个任务独立完成与验证，不跨任务“捎带修改”。  
- 若环境缺失测试依赖，先补依赖再继续，不得跳过失败测试阶段。  

