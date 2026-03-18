from .base import BaseAgent, AgentConfig, AnalysisContext
from ..providers import Message

EVENT_SYSTEM_PROMPT = """你是一位专业的文学分析师，专注于分析小说中的重要事件及其时间结构。

你需要识别和分析：
- 关键情节事件
- 转折点
- 冲突与解决
- 伏笔与呼应
- 事件的时间顺序和时间标记
- 闪回与预叙

## 重要程度判断标准（必须严格遵守）

### critical（关键事件）
必须满足以下条件之一：
- 彻底改变故事走向的重大转折
- 主角命运的决定性时刻
- 核心矛盾的爆发或解决
- 主要角色的生死存亡
示例：主角觉醒力量、重要角色死亡、敌我阵营的决战

### major（重要事件）
满足以下条件之一：
- 推动主线剧情发展
- 揭示重要信息或秘密
- 建立或打破重要关系
- 角色的重要成长或转变
示例：发现重要线索、结识关键人物、获得重要能力

### normal（普通事件）
- 日常情节推进
- 支线剧情
- 过渡性事件
示例：日常对话、场景转换、小型冲突

### minor（次要事件）
- 背景描述
- 氛围营造
- 非必要细节
示例：环境描写中的小插曲

## 判断提示
- 不要把所有事件都标为 normal
- 章节中通常至少有 1-2 个 major 或以上级别的事件
- critical 事件较少，但如果有重大转折必须标注
- 考虑事件对后续剧情的影响"""

EVENT_USER_TEMPLATE = """请分析以下章节中发生的重要事件：

【书名】{book_title}
【章节】第{chapter_index}章 {chapter_title}

{known_events_section}

【正文】
{content}

请以 JSON 格式输出分析结果，包含 events 数组，每个事件包含：
- title: 事件标题
- description: 事件描述
- importance: 重要程度，必须选择：critical | major | normal | minor
  - critical: 改变故事走向的关键转折
  - major: 推动剧情的重要事件
  - normal: 普通情节
  - minor: 次要背景
- characters_involved: 相关人物列表
- evidence: 原文证据列表
- is_turning_point: 是否为转折点
- foreshadowing: 是否涉及伏笔
- is_new: 是否为本章新发生的事件
- time_marker: 时间标记（如有明确描述，如"正午时分"、"三天后"）
- order_in_chapter: 在章节中的发生顺序（1,2,3...）
- is_flashback: 是否为闪回
- relative_time: 相对时间描述（如"开篇时"、"结尾处"）

【重要提醒】
- 请认真评估每个事件的重要程度，不要全部标为 normal
- 章节核心事件应标为 major 或 critical
- 考虑事件对故事发展的影响"""


class EventAgent(BaseAgent):
    def __init__(self, config: AgentConfig, provider):
        if not config.system_prompt:
            config.system_prompt = EVENT_SYSTEM_PROMPT
        super().__init__(config, provider)

    async def analyze(self, context: AnalysisContext) -> dict:
        known_section = ""
        if context.known_events:
            known_section = "【已知事件】\n"
            for event in context.known_events[:15]:
                known_section += (
                    f"- {event.get('title', '')}: {event.get('description', '')[:100]}\n"
                )

        user_prompt = EVENT_USER_TEMPLATE.format(
            book_title=context.book_title,
            chapter_index=context.chapter_index,
            chapter_title=context.chapter_title or "",
            known_events_section=known_section,
            content=context.chapter_content,
        )

        messages = self.build_messages(context, user_prompt)

        schema = {
            "type": "object",
            "properties": {
                "events": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "title": {"type": "string"},
                            "description": {"type": "string"},
                            "importance": {"type": "string"},
                            "characters_involved": {"type": "array", "items": {"type": "string"}},
                            "evidence": {"type": "array", "items": {"type": "string"}},
                            "is_turning_point": {"type": "boolean"},
                            "foreshadowing": {"type": "boolean"},
                            "is_new": {"type": "boolean"},
                            "time_marker": {"type": "string"},
                            "order_in_chapter": {"type": "number"},
                            "is_flashback": {"type": "boolean"},
                            "relative_time": {"type": "string"},
                        },
                        "required": ["title", "description", "importance", "evidence"],
                    },
                }
            },
            "required": ["events"],
        }

        return await self.complete_json(messages, schema)
