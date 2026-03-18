from abc import ABC, abstractmethod
from dataclasses import dataclass
from typing import Any, Optional
from ..providers import LLMProvider, CompletionRequest, Message
from ..logger import log_info, log_error, log_exception


@dataclass
class AgentConfig:
    name: str
    kind: str
    provider_name: str
    model: Optional[str] = None
    system_prompt: Optional[str] = None
    temperature: float = 0.7
    max_tokens: Optional[int] = None
    system_prompt_prefix: Optional[str] = None
    system_prompt_suffix: Optional[str] = None


@dataclass
class AnalysisContext:
    chapter_content: str
    chapter_index: int
    chapter_title: Optional[str]
    book_title: str
    previous_summary: Optional[str] = None
    known_characters: Optional[list[dict]] = None
    known_settings: Optional[list[dict]] = None
    known_events: Optional[list[dict]] = None
    current_profile: Optional[dict] = None  # Current style profile for incremental refinement


class BaseAgent(ABC):
    def __init__(self, config: AgentConfig, provider: LLMProvider):
        self.config = config
        self.provider = provider

    @abstractmethod
    async def analyze(self, context: AnalysisContext) -> dict:
        pass

    def build_messages(self, context: AnalysisContext, user_prompt: str) -> list[Message]:
        messages = []

        # Build final system prompt: prefix + agent prompt + suffix
        parts = []
        if self.config.system_prompt_prefix:
            parts.append(self.config.system_prompt_prefix.strip())
        if self.config.system_prompt:
            parts.append(self.config.system_prompt.strip())
        if self.config.system_prompt_suffix:
            parts.append(self.config.system_prompt_suffix.strip())

        final_system_prompt = "\n\n".join(p for p in parts if p)
        if final_system_prompt:
            messages.append(Message(role="system", content=final_system_prompt))

        messages.append(Message(role="user", content=user_prompt))

        return messages

    async def complete(self, messages: list[Message]) -> str:
        request = CompletionRequest(
            messages=messages,
            model=self.config.model,
            temperature=self.config.temperature,
            max_tokens=self.config.max_tokens,
        )
        response = await self.provider.complete(request)
        return response.content

    async def complete_json(self, messages: list[Message], schema: dict) -> dict:
        request = CompletionRequest(
            messages=messages,
            model=self.config.model,
            temperature=self.config.temperature,
            max_tokens=self.config.max_tokens,
        )

        # Log request in compact format
        sys_prompt_len = 0
        user_msg_len = 0
        for m in messages:
            if m.role == "system":
                sys_prompt_len = len(m.content)
            elif m.role == "user":
                user_msg_len = len(m.content)

        log_info(f"[Agent:{self.config.kind}] Request: model={self.config.model}, temp={self.config.temperature}, sys_prompt={sys_prompt_len}chars, user_msg={user_msg_len}chars")

        try:
            result = await self.provider.complete_json(request, schema)
            # Log result summary in one line
            if isinstance(result, dict):
                summary = ", ".join(f"{k}={len(v) if isinstance(v, list) else type(v).__name__}" for k, v in result.items())
                log_info(f"[Agent:{self.config.kind}] Response: {summary}")
            return result
        except Exception as e:
            log_error(f"[Agent:{self.config.kind}] ERROR: {e}")
            raise
