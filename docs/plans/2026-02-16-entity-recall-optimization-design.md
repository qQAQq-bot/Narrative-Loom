# 全实体召回优化设计（Character / Setting / Event）

> 适用范围：`src-tauri` 检索与上下文构建链路；目标是提升章节分析阶段对人物、设定、事件三类实体的召回率。

## 1. 背景与问题

当前分析链路已经具备以下能力：
- 章节分析前构建 `known_characters / known_settings / known_events` 上下文；
- 使用向量检索段落并从 `entities_mentioned` 回捞实体；
- 将上下文注入 Python sidecar 的各类 Agent。

当前主要召回瓶颈：
- 候选池偏小，单路召回容易漏掉跨章节线索；
- 提及识别对别名、变体不友好；
- 三类实体在同一预算下会互相挤占，导致“单类强、其他类弱”；
- embedding 失败时回退策略存在召回波动。

## 2. 目标与非目标

### 2.1 目标

- 同时提升人物 / 设定 / 事件三类实体召回；
- 在不破坏现有命令接口的前提下，增强 `ContextBuilder` 召回能力；
- 保持可观测：能够说明召回来自哪条检索通道；
- 保障性能：上下文构建时延可控。

### 2.2 非目标

- 不改动前端页面交互；
- 不重构 Python Agent 输出 schema；
- 不引入新的外部服务依赖。

## 3. 当前链路（现状）

1. `analysis.rs` 在分析前调用 `build_smart_context`，默认上限为人物 20 / 设定 10 / 事件 15 / 段落 5。  
2. `ContextBuilder` 多查询向量检索 Paragraph，抽取 `entities_mentioned`，不足时用“最近实体”兜底。  
3. `known_*` 传入 sidecar，Python Agent 再按各自上限截断（例如人物最多 20）。  

结论：链路完整，但“候选召回深度 + 覆盖重排 + 别名匹配”还不够。

## 4. 目标方案：召回漏斗（推荐）

### Stage A：多路候选召回

- 向量召回（semantic）
- 关键词召回（keyword / FTS）
- 实体历史召回（history）

三路并行构建候选 `chunk` 集合，统一去重。

### Stage B：统一打分融合

对同一 `chunk_id` 做融合打分：

`chunk_score = 0.55 * vector_rrf + 0.30 * keyword_rrf + 0.15 * history_rrf + boosts`

其中 `boosts` 包含：
- `entities_mentioned` 命中加分；
- 别名命中加分；
- 距离当前章节近邻加分。

### Stage C：实体映射与覆盖重排

- 先按 `chunk_score` 映射候选实体；
- 再按三类实体配额重排，保证最低覆盖：
  - character：至少 8
  - setting：至少 6
  - event：至少 6

### Stage D：Prompt 预算打包

- 优先保留高置信实体摘要；
- 段落证据次之；
- 预算超限时按置信度和覆盖价值裁剪。

## 5. 参数默认值（第一版）

- `candidate_chunks_per_query = 60`
- `max_passages = 12`
- `entity_quota = { character: 24, setting: 16, event: 20 }`
- `min_type_coverage = { character: 8, setting: 6, event: 6 }`
- `alias_expansion = true`

## 6. 失败与降级策略

- 向量检索失败：降级为 keyword + history，不中断分析主流程；
- embedding fallback 触发：提高 keyword 权重，降低向量权重；
- 任一路失败都记录诊断日志（通道级计数 + 召回结果分布）。

## 7. 可观测性设计

每次上下文构建输出结构化诊断（日志即可）：
- 每路候选数（vector / keyword / history）；
- 去重后 chunk 数；
- 三类实体最终数与保底补齐数；
- fallback 是否触发；
- 召回耗时分段。

## 8. 验证与度量（严格 TDD）

核心指标：
- `Recall@K(character|setting|event)`
- `Macro Recall`（三类平均）
- `Fallback Rate`
- `Context Build P95`

测试分层：
- 单元测试：融合打分、覆盖重排、别名匹配、fallback 权重切换；
- 集成测试：`build_smart_context` 在固定样本上满足最小召回阈值；
- 回归门禁：`Macro Recall` 不得低于基线。

## 9. 分阶段实施建议

1. 先加可观测性与基线评测；  
2. 再做多路召回 + 融合；  
3. 再做覆盖重排与别名增强；  
4. 最后收敛性能与参数。  

## 10. 风险与缓解

- **风险**：召回提升导致 Prompt 噪声增加。  
  **缓解**：覆盖重排后再按预算裁剪，优先高置信实体摘要。

- **风险**：多路检索增加时延。  
  **缓解**：设置每路上限、并行执行、分段耗时监控。

- **风险**：fallback 场景下结果波动。  
  **缓解**：显式权重切换 + 日志告警 + 回归阈值。

