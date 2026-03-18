from .base import BaseAgent, AgentConfig, AnalysisContext
from ..providers import Message

TECHNIQUE_SYSTEM_PROMPT = """你是一位专业的文学分析师，专注于分析小说中的写作技法。

你需要识别以下类型的写作技法：
- 叙事技法：视角转换、时间线操控、悬念设置、伏笔埋设
- 描写技法：环境渲染、人物刻画、心理描写、对话设计
- 结构技法：章节布局、情节节奏、转折设计、高潮铺垫
- 修辞技法：比喻、象征、对比、反讽

对于每个识别出的技法，你需要：
1. 准确命名技法类型
2. 解释其作用机制
3. 引用原文证据
4. 分析其效果"""

TECHNIQUE_USER_TEMPLATE = """请分析以下章节中使用的写作技法：

【书名】{book_title}
【章节】第{chapter_index}章 {chapter_title}

【正文】
{content}

请以 JSON 格式输出分析结果，包含 techniques 数组，每个技法包含：
- type: 技法类型
- title: 技法名称
- description: 技法描述
- mechanism: 作用机制
- evidence: 原文证据列表
- tags: 相关标签"""


class TechniqueAgent(BaseAgent):
    def __init__(self, config: AgentConfig, provider):
        if not config.system_prompt:
            config.system_prompt = TECHNIQUE_SYSTEM_PROMPT
        super().__init__(config, provider)

    async def analyze(self, context: AnalysisContext) -> dict:
        user_prompt = TECHNIQUE_USER_TEMPLATE.format(
            book_title=context.book_title,
            chapter_index=context.chapter_index,
            chapter_title=context.chapter_title or "",
            content=context.chapter_content,
        )

        messages = self.build_messages(context, user_prompt)

        schema = {
            "type": "object",
            "properties": {
                "techniques": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "type": {"type": "string"},
                            "title": {"type": "string"},
                            "description": {"type": "string"},
                            "mechanism": {"type": "string"},
                            "evidence": {"type": "array", "items": {"type": "string"}},
                            "tags": {"type": "array", "items": {"type": "string"}},
                        },
                        "required": ["type", "title", "description", "mechanism", "evidence"],
                    },
                }
            },
            "required": ["techniques"],
        }

        return await self.complete_json(messages, schema)
