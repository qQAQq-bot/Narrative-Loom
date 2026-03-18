from .base import BaseAgent, AgentConfig, AnalysisContext
from ..providers import Message

CHARACTER_SYSTEM_PROMPT = """你是一位专业的文学分析师，专注于分析小说中的人物角色。

你需要识别和分析：
- 人物的基本信息（姓名、别名、身份）
- 人物的性格特征
- 人物的外貌描写
- 人物之间的关系
- 人物在故事中的角色（主角、配角、反派等）

对于每个识别出的人物，你需要：
1. 准确记录人物信息
2. 引用原文证据
3. 按维度分析人物特征（外貌、性格、背景、能力、目标、状态）

【重要】人物命名规则：
- 每个人物必须是独立个体，不要创建组合名称
- 错误示例：❌ "张三 & 李四"、❌ "王五和赵六"、❌ "甲与乙"
- 正确做法：为每个人物创建单独的条目
- 如果看到多个人物一起出现，请分别为他们创建独立条目"""

CHARACTER_USER_TEMPLATE = """请分析以下章节中出现的人物：

【书名】{book_title}
【章节】第{chapter_index}章 {chapter_title}

{known_characters_section}

【正文】
{content}

请以 JSON 格式输出分析结果，包含 characters 数组，每个人物包含：
- name: 人物姓名（必须是单个人物，禁止使用"&"、"和"、"与"、"、"连接多人）
- aliases: 别名列表
- description: 人物简要描述（一句话概括）
- description_structured: 结构化描述，包含以下维度（仅填写本章有明确信息的维度）：
  - appearance: 外貌描写（身高、体型、相貌特征、穿着风格）
  - personality: 性格特征（性格描述、行为习惯）
  - background: 背景信息（身份、来历、过往经历）
  - abilities: 能力技能（特殊能力、专业技能）
  - goals: 目标动机（当前目标、追求的东西）
  - status: 当前状态（处境、状况、心理状态）
- traits: 性格特征关键词列表
- role: 角色类型 (protagonist/antagonist/supporting/minor)
- relationships: 与其他人物的关系
- evidence: 原文证据列表
- is_new: 是否为本章新出现的人物"""


class CharacterAgent(BaseAgent):
    def __init__(self, config: AgentConfig, provider):
        if not config.system_prompt:
            config.system_prompt = CHARACTER_SYSTEM_PROMPT
        super().__init__(config, provider)

    async def analyze(self, context: AnalysisContext) -> dict:
        known_section = ""
        if context.known_characters:
            known_section = "【已知人物】\n"
            for char in context.known_characters[:20]:
                known_section += f"- {char.get('name', '')}: {char.get('description', '')[:100]}\n"

        user_prompt = CHARACTER_USER_TEMPLATE.format(
            book_title=context.book_title,
            chapter_index=context.chapter_index,
            chapter_title=context.chapter_title or "",
            known_characters_section=known_section,
            content=context.chapter_content,
        )

        messages = self.build_messages(context, user_prompt)

        schema = {
            "type": "object",
            "properties": {
                "characters": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": {"type": "string"},
                            "aliases": {"type": "array", "items": {"type": "string"}},
                            "description": {"type": "string"},
                            "description_structured": {
                                "type": "object",
                                "properties": {
                                    "appearance": {"type": "string"},
                                    "personality": {"type": "string"},
                                    "background": {"type": "string"},
                                    "abilities": {"type": "string"},
                                    "goals": {"type": "string"},
                                    "status": {"type": "string"},
                                },
                            },
                            "traits": {"type": "array", "items": {"type": "string"}},
                            "role": {"type": "string"},
                            "relationships": {"type": "object"},
                            "evidence": {"type": "array", "items": {"type": "string"}},
                            "is_new": {"type": "boolean"},
                        },
                        "required": ["name", "evidence"],
                    },
                }
            },
            "required": ["characters"],
        }

        return await self.complete_json(messages, schema)
