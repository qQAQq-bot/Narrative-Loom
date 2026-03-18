from .base import BaseAgent, AgentConfig, AnalysisContext
from ..providers import Message

SETTING_SYSTEM_PROMPT = """你是一位专业的文学分析师，专注于分析小说中的世界设定。

你需要识别和分析以下类型的设定（请严格按照分类标准）：

## 分类标准（非常重要，必须严格遵守）

### location（地点）- 仅限地理位置
只有以下才能归类为 location：
- 城市、国家、地区（如：圣纳黎、王都、北方大陆）
- 建筑物（如：酒馆、城堡、神殿）
- 自然地点（如：森林、山脉、河流）
- 具体场所（如：广场、街道、房间）

### item（物品）- 具体的物件
以下必须归类为 item：
- 货币（如：金币、银币、圣纳黎金币）
- 书籍文献（如：魔法书、契约书、族谱）
- 武器装备（如：圣剑、魔杖、铠甲）
- 特殊道具（如：纹章、印章、信物）
- 日常物品（如：马车、药剂）

### organization（组织）- 人员团体
以下必须归类为 organization：
- 公会、帮派（如：冒险者公会、盗贼团）
- 商业团体（如：马戏团、商会）
- 军事组织（如：骑士团、军队）
- 政治机构（如：议会、王室）

### worldview（世界观）- 抽象规则和制度
以下必须归类为 worldview：
- 社会制度（如：奴隶制度、贵族制度、阶级制度）
- 魔法体系（如：元素魔法系统、魔力等级）
- 种族设定（如：精灵族特性、龙人种族）
- 世界规则（如：契约法则、神灵信仰）

### era（时代）- 历史时期
以下归类为 era：
- 历史时期（如：魔法纪元、黑暗时代）
- 时代背景（如：战后重建期）

## 常见错误提醒
- ❌ "金币"归类为 location → ✅ 应为 item
- ❌ "奴隶制度"归类为 location → ✅ 应为 worldview
- ❌ "马戏团"归类为 location → ✅ 应为 organization
- ❌ "契约"归类为 location → ✅ 应为 item

## 命名规则
- 每个设定必须是独立概念，不要创建组合名称
- 错误示例：❌ "龙人种 & 精灵族"、❌ "王都和圣纳黎"
- 正确做法：为每个设定创建单独的条目"""

SETTING_USER_TEMPLATE = """请分析以下章节中出现的世界设定：

【书名】{book_title}
【章节】第{chapter_index}章 {chapter_title}

{known_settings_section}

【正文】
{content}

请以 JSON 格式输出分析结果，包含 settings 数组。
注意：为避免输出过长，每种类型最多提取3个最重要的设定，每个设定的 evidence 最多2条。

每个设定包含：
- type: 设定类型，必须严格选择：location | item | organization | worldview | era
- name: 设定名称（必须是单个设定，禁止使用"&"、"和"、"与"、"、"连接多个）
- description: 设定简要描述（一句话概括，50字以内）
- description_structured: 结构化描述，包含以下维度（仅填写本章有明确信息的维度）：
  - physical: 物理特征（外观、大小、结构、材质）
  - atmosphere: 氛围环境（气氛、环境感受）
  - history: 历史背景（起源、历史事件）
  - function: 功能用途（作用、使用方式）
  - rules: 规则限制（使用规则、相关法则）
  - inhabitants: 相关人员（居住者、使用者、相关群体）
  - status: 当前状态（现状、变化）
- properties: 详细属性（最多3个关键属性）
- evidence: 原文证据列表（最多2条，每条50字以内）
- is_new: 是否为本章新出现的设定

【再次强调分类】
- 货币、书籍、武器、道具 → type: "item"
- 公会、帮派、马戏团、军队 → type: "organization"
- 制度、体系、种族规则 → type: "worldview"
- 城市、建筑、自然地点 → type: "location"
- 历史时期 → type: "era\""""


class SettingAgent(BaseAgent):
    def __init__(self, config: AgentConfig, provider):
        if not config.system_prompt:
            config.system_prompt = SETTING_SYSTEM_PROMPT
        super().__init__(config, provider)

    async def analyze(self, context: AnalysisContext) -> dict:
        known_section = ""
        if context.known_settings:
            known_section = "【已知设定】\n"
            for setting in context.known_settings[:20]:
                known_section += (
                    f"- {setting.get('name', '')}: {setting.get('description', '')[:100]}\n"
                )

        user_prompt = SETTING_USER_TEMPLATE.format(
            book_title=context.book_title,
            chapter_index=context.chapter_index,
            chapter_title=context.chapter_title or "",
            known_settings_section=known_section,
            content=context.chapter_content,
        )

        messages = self.build_messages(context, user_prompt)

        schema = {
            "type": "object",
            "properties": {
                "settings": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "type": {
                                "type": "string",
                                "enum": ["location", "item", "organization", "worldview", "era"]
                            },
                            "name": {"type": "string"},
                            "description": {"type": "string"},
                            "description_structured": {
                                "type": "object",
                                "properties": {
                                    "physical": {"type": "string"},
                                    "atmosphere": {"type": "string"},
                                    "history": {"type": "string"},
                                    "function": {"type": "string"},
                                    "rules": {"type": "string"},
                                    "inhabitants": {"type": "string"},
                                    "status": {"type": "string"},
                                },
                            },
                            "properties": {"type": "object"},
                            "evidence": {"type": "array", "items": {"type": "string"}},
                            "is_new": {"type": "boolean"},
                        },
                        "required": ["type", "name", "evidence"],
                    },
                }
            },
            "required": ["settings"],
        }

        return await self.complete_json(messages, schema)
