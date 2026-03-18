from abc import ABC, abstractmethod
from dataclasses import dataclass
from typing import Any, Optional
import json


@dataclass
class ProviderConfig:
    name: str
    api_base: str
    api_key: str
    default_model: str
    api_format: str = "openai"
    extra_headers: Optional[dict] = None
    max_retries: int = 3  # 重试次数
    retry_delay: float = 1.0  # 重试间隔（秒）


@dataclass
class Message:
    role: str
    content: str


@dataclass
class CompletionRequest:
    messages: list[Message]
    model: Optional[str] = None
    temperature: float = 0.7
    max_tokens: Optional[int] = None
    response_format: Optional[dict] = None


@dataclass
class CompletionResponse:
    content: str
    model: str
    usage: dict
    finish_reason: str


class LLMProvider(ABC):
    def __init__(self, config: ProviderConfig):
        self.config = config

    @abstractmethod
    async def complete(self, request: CompletionRequest) -> CompletionResponse:
        pass

    @abstractmethod
    async def complete_json(self, request: CompletionRequest, schema: dict) -> dict:
        pass

    async def close(self):
        """Close any open connections. Override in subclasses if needed."""
        pass

    def get_model(self, request: CompletionRequest) -> str:
        return request.model or self.config.default_model
