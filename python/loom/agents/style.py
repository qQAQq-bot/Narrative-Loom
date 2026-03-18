from .base import BaseAgent, AgentConfig, AnalysisContext

STYLE_SYSTEM_PROMPT = """你是一位专业的文学风格分析专家，专注于识别和分析作家的独特写作风格特征。

你需要从以下8个维度深入分析文本的写作风格：

1. 词汇特征 (vocabulary)
   - 正式程度：formal(正式)/semi_formal(半正式)/colloquial(口语化)/mixed(混合)
   - 独特词汇：作者偏好使用的特色词汇
   - 词汇倾向：简洁/华丽/专业/方言等
   - 选词模式：动词倾向、形容词使用等
   - 时代标记：是否有时代特征的用词

2. 句式结构 (sentence_structure)
   - 典型长度：short(短)/medium(中等)/long(长)/varied(变化)
   - 复杂程度：simple(简单)/moderate(适度)/complex(复杂)/highly_complex(高度复杂)
   - 节奏模式：句子长短交替规律
   - 标点风格：标点使用特点
   - 段落结构：段落组织方式

3. 叙事声音 (narrative_voice)
   - 叙述视角：first_person(第一人称)/third_limited(第三人称有限)/third_omniscient(第三人称全知)/second_person(第二人称)/multiple(多视角)
   - 时态使用：past(过去时)/present(现在时)/mixed(混合)
   - 叙述者特征：叙述者的个性和语调
   - 介入程度：minimal(最少)/moderate(适度)/frequent(频繁)
   - 可靠性：reliable(可靠)/unreliable(不可靠)/ambiguous(模糊)

4. 对话风格 (dialogue_style)
   - 对话比例：low(低)/moderate(适度)/high(高)
   - 对话标签：对话标签的使用模式
   - 语速节奏：对话的节奏特点
   - 角色区分：角色语言的差异化程度
   - 潜台词使用：潜台词和言外之意的运用

5. 描写风格 (description_style)
   - 感官偏好：偏重的感官类型（视觉/听觉/触觉等）
   - 细节程度：minimal(极简)/selective(精选)/detailed(详细)/exhaustive(详尽)
   - 比喻使用：比喻手法的频率和类型
   - 比喻示例：典型比喻的例子
   - 意象模式：常用的意象和象征

6. 节奏控制 (pacing)
   - 整体节奏：slow(慢)/moderate(中等)/fast(快)/variable(变化)
   - 场景转换：场景切换的处理方式
   - 张力营造：紧张感的构建技巧
   - 动静比例：动作场景与反思场景的平衡

7. 情感表达 (emotional_expression)
   - 主导情绪：整体的情感色彩
   - 情绪范围：情感的变化幅度
   - 展示vs叙述：mostly_show(多展示)/balanced(平衡)/mostly_tell(多叙述)
   - 情感克制：understated(克制)/balanced(平衡)/expressive(表达充分)
   - 氛围技法：营造氛围的手法

8. 主题元素 (thematic_elements)
   - 重复母题：反复出现的主题
   - 象征模式：象征手法的运用
   - 主题关切：作者关注的核心议题

请仔细分析文本，为每个维度提供准确的分析结果。
务必引用原文作为证据支持你的分析。
如果提供了已有的风格档案，请在此基础上验证、修正或补充新发现的特征。"""

STYLE_USER_TEMPLATE = """请深入分析以下章节的写作风格特征：

【书名】{book_title}
【章节】第{chapter_index}章 {chapter_title}
{profile_context}
【正文】
{content}

请以 JSON 格式输出完整的风格分析结果，严格按照以下结构：

{{
    "vocabulary": {{
        "formality_level": "formal/semi_formal/colloquial/mixed",
        "distinctive_vocabulary": ["词汇1", "词汇2"],
        "vocabulary_tendencies": "描述词汇使用倾向",
        "word_choice_patterns": "描述选词模式",
        "period_markers": "描述时代特征用词，如无则为null"
    }},
    "sentence_structure": {{
        "typical_length": "short/medium/long/varied",
        "complexity_level": "simple/moderate/complex/highly_complex",
        "rhythm_patterns": "描述句子节奏模式",
        "punctuation_style": "描述标点使用风格",
        "paragraph_structure": "描述段落组织方式"
    }},
    "narrative_voice": {{
        "perspective": "first_person/third_limited/third_omniscient/second_person/multiple",
        "tense": "past/present/mixed",
        "narrator_characteristics": "描述叙述者特征",
        "intrusion_level": "minimal/moderate/frequent",
        "reliability": "reliable/unreliable/ambiguous"
    }},
    "dialogue_style": {{
        "dialogue_proportion": "low/moderate/high",
        "dialogue_tags": "描述对话标签使用模式",
        "speech_rhythm": "描述对话节奏",
        "character_voice_differentiation": "描述角色语言区分程度",
        "subtext_usage": "描述潜台词使用"
    }},
    "description_style": {{
        "sensory_preferences": ["视觉", "听觉"],
        "detail_level": "minimal/selective/detailed/exhaustive",
        "metaphor_usage": "描述比喻使用频率和类型",
        "metaphor_examples": ["比喻例子1", "比喻例子2"],
        "imagery_patterns": "描述常用意象模式"
    }},
    "pacing": {{
        "overall_tempo": "slow/moderate/fast/variable",
        "scene_transitions": "描述场景转换方式",
        "tension_building": "描述张力营造手法",
        "action_vs_reflection_balance": "描述动静比例"
    }},
    "emotional_expression": {{
        "dominant_mood": "描述主导情绪",
        "mood_range": "描述情绪变化范围",
        "show_vs_tell": "mostly_show/balanced/mostly_tell",
        "emotional_restraint": "understated/balanced/expressive",
        "atmosphere_techniques": "描述氛围营造技法"
    }},
    "thematic_elements": {{
        "recurring_motifs": ["母题1", "母题2"],
        "symbolic_patterns": "描述象征模式",
        "thematic_concerns": ["主题1", "主题2"]
    }},
    "key_observations": ["关键发现1", "关键发现2", "关键发现3"],
    "notable_techniques": ["技法1", "技法2"],
    "sample_passages": [
        {{"text": "原文引用1", "annotation": "说明这段引用体现了什么风格特征", "dimension": "相关维度"}},
        {{"text": "原文引用2", "annotation": "说明", "dimension": "相关维度"}},
        {{"text": "原文引用3", "annotation": "说明", "dimension": "相关维度"}}
    ]
}}

注意：
1. sample_passages 最多提供3个最能体现作者风格的典型段落，需标注相关维度
2. 所有枚举值必须使用指定的英文选项
3. 描述性字段使用中文
4. 确保引用的原文片段准确无误
5. 如果是基于已有档案分析，请特别关注新发现的特征或与已有特征的差异"""

# Template for when there's an existing profile
PROFILE_CONTEXT_TEMPLATE = """
【已有风格档案】（请在此基础上验证、修正或补充）
已分析章节数：{analyzed_chapters}
主要特征摘要：
- 词汇正式程度：{formality_level}
- 句式复杂度：{complexity_level}
- 叙事视角：{perspective}
- 对话比例：{dialogue_proportion}
- 整体节奏：{overall_tempo}
- 情感克制：{emotional_restraint}

请特别关注：
1. 本章是否有与上述特征不同的表现（如有，请标注为变化）
2. 是否发现新的风格特征
3. 已有特征在本章的具体体现
"""


def build_profile_context(current_profile: dict) -> str:
    """Build profile context string from existing profile."""
    if not current_profile:
        return ""

    try:
        # Extract key features from profile
        vocab = current_profile.get("vocabulary", {})
        sentence = current_profile.get("sentence_structure", {})
        narrative = current_profile.get("narrative_voice", {})
        dialogue = current_profile.get("dialogue_style", {})
        pacing = current_profile.get("pacing", {})
        emotional = current_profile.get("emotional_expression", {}) or current_profile.get("emotional_tone", {})
        metadata = current_profile.get("analysis_metadata", {})

        return PROFILE_CONTEXT_TEMPLATE.format(
            analyzed_chapters=metadata.get("chapters_analyzed", "未知"),
            formality_level=vocab.get("formality_level", "未知"),
            complexity_level=sentence.get("complexity_level", "未知"),
            perspective=narrative.get("perspective", "未知"),
            dialogue_proportion=dialogue.get("dialogue_proportion", "未知"),
            overall_tempo=pacing.get("overall_tempo", "未知"),
            emotional_restraint=emotional.get("emotional_restraint", "未知"),
        )
    except Exception:
        return ""


class StyleAgent(BaseAgent):
    """写作风格分析 Agent，生成详细的风格特征 JSON，支持增量精炼"""

    def __init__(self, config: AgentConfig, provider):
        if not config.system_prompt:
            config.system_prompt = STYLE_SYSTEM_PROMPT
        # Use lower temperature for more consistent analysis
        if config.temperature > 0.5:
            config.temperature = 0.3
        super().__init__(config, provider)

    async def analyze(self, context: AnalysisContext) -> dict:
        # Build profile context if available
        profile_context = ""
        if hasattr(context, 'current_profile') and context.current_profile:
            profile_context = build_profile_context(context.current_profile)

        user_prompt = STYLE_USER_TEMPLATE.format(
            book_title=context.book_title,
            chapter_index=context.chapter_index,
            chapter_title=context.chapter_title or "",
            content=context.chapter_content,
            profile_context=profile_context,
        )

        messages = self.build_messages(context, user_prompt)

        schema = {
            "type": "object",
            "properties": {
                "vocabulary": {
                    "type": "object",
                    "properties": {
                        "formality_level": {
                            "type": "string",
                            "enum": ["formal", "semi_formal", "colloquial", "mixed"]
                        },
                        "distinctive_vocabulary": {
                            "type": "array",
                            "items": {"type": "string"}
                        },
                        "vocabulary_tendencies": {"type": "string"},
                        "word_choice_patterns": {"type": "string"},
                        "period_markers": {"type": ["string", "null"]}
                    },
                    "required": ["formality_level", "distinctive_vocabulary", "vocabulary_tendencies", "word_choice_patterns"]
                },
                "sentence_structure": {
                    "type": "object",
                    "properties": {
                        "typical_length": {
                            "type": "string",
                            "enum": ["short", "medium", "long", "varied"]
                        },
                        "complexity_level": {
                            "type": "string",
                            "enum": ["simple", "moderate", "complex", "highly_complex"]
                        },
                        "rhythm_patterns": {"type": "string"},
                        "punctuation_style": {"type": "string"},
                        "paragraph_structure": {"type": "string"}
                    },
                    "required": ["typical_length", "complexity_level", "rhythm_patterns", "punctuation_style", "paragraph_structure"]
                },
                "narrative_voice": {
                    "type": "object",
                    "properties": {
                        "perspective": {
                            "type": "string",
                            "enum": ["first_person", "third_limited", "third_omniscient", "second_person", "multiple"]
                        },
                        "tense": {
                            "type": "string",
                            "enum": ["past", "present", "mixed"]
                        },
                        "narrator_characteristics": {"type": "string"},
                        "intrusion_level": {
                            "type": "string",
                            "enum": ["minimal", "moderate", "frequent"]
                        },
                        "reliability": {
                            "type": "string",
                            "enum": ["reliable", "unreliable", "ambiguous"]
                        }
                    },
                    "required": ["perspective", "tense", "narrator_characteristics", "intrusion_level", "reliability"]
                },
                "dialogue_style": {
                    "type": "object",
                    "properties": {
                        "dialogue_proportion": {
                            "type": "string",
                            "enum": ["low", "moderate", "high"]
                        },
                        "dialogue_tags": {"type": "string"},
                        "speech_rhythm": {"type": "string"},
                        "character_voice_differentiation": {"type": "string"},
                        "subtext_usage": {"type": "string"}
                    },
                    "required": ["dialogue_proportion", "dialogue_tags", "speech_rhythm", "character_voice_differentiation", "subtext_usage"]
                },
                "description_style": {
                    "type": "object",
                    "properties": {
                        "sensory_preferences": {
                            "type": "array",
                            "items": {"type": "string"}
                        },
                        "detail_level": {
                            "type": "string",
                            "enum": ["minimal", "selective", "detailed", "exhaustive"]
                        },
                        "metaphor_usage": {"type": "string"},
                        "metaphor_examples": {
                            "type": "array",
                            "items": {"type": "string"}
                        },
                        "imagery_patterns": {"type": "string"}
                    },
                    "required": ["sensory_preferences", "detail_level", "metaphor_usage", "metaphor_examples", "imagery_patterns"]
                },
                "pacing": {
                    "type": "object",
                    "properties": {
                        "overall_tempo": {
                            "type": "string",
                            "enum": ["slow", "moderate", "fast", "variable"]
                        },
                        "scene_transitions": {"type": "string"},
                        "tension_building": {"type": "string"},
                        "action_vs_reflection_balance": {"type": "string"}
                    },
                    "required": ["overall_tempo", "scene_transitions", "tension_building", "action_vs_reflection_balance"]
                },
                "emotional_expression": {
                    "type": "object",
                    "properties": {
                        "dominant_mood": {"type": "string"},
                        "mood_range": {"type": "string"},
                        "show_vs_tell": {
                            "type": "string",
                            "enum": ["mostly_show", "balanced", "mostly_tell"]
                        },
                        "emotional_restraint": {
                            "type": "string",
                            "enum": ["understated", "balanced", "expressive"]
                        },
                        "atmosphere_techniques": {"type": "string"}
                    },
                    "required": ["dominant_mood", "mood_range", "show_vs_tell", "emotional_restraint", "atmosphere_techniques"]
                },
                "thematic_elements": {
                    "type": "object",
                    "properties": {
                        "recurring_motifs": {
                            "type": "array",
                            "items": {"type": "string"}
                        },
                        "symbolic_patterns": {"type": "string"},
                        "thematic_concerns": {
                            "type": "array",
                            "items": {"type": "string"}
                        }
                    },
                    "required": ["recurring_motifs", "symbolic_patterns", "thematic_concerns"]
                },
                "key_observations": {
                    "type": "array",
                    "items": {"type": "string"}
                },
                "notable_techniques": {
                    "type": "array",
                    "items": {"type": "string"}
                },
                "sample_passages": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "text": {"type": "string"},
                            "annotation": {"type": "string"},
                            "dimension": {"type": "string"}
                        },
                        "required": ["text", "annotation", "dimension"]
                    },
                    "maxItems": 3
                }
            },
            "required": [
                "vocabulary",
                "sentence_structure",
                "narrative_voice",
                "dialogue_style",
                "description_style",
                "pacing",
                "emotional_expression",
                "thematic_elements",
                "key_observations",
                "notable_techniques",
                "sample_passages"
            ]
        }

        return await self.complete_json(messages, schema)
