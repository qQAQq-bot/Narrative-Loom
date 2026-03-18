from .base import LLMProvider, ProviderConfig, CompletionRequest, CompletionResponse, Message
from .openai import OpenAIProvider
from .openai_responses import OpenAIResponsesProvider, OpenAIResponsesProviderWithReasoning
from .anthropic import AnthropicProvider
from .ollama import OllamaProvider


def create_provider(config: ProviderConfig) -> LLMProvider:
    """Create a provider instance based on the api_format configuration.

    Supported formats:
    - "openai": OpenAI Chat Completions API and compatible services (DeepSeek, etc.)
    - "openai_responses": OpenAI Responses API (newer API with structured output support)
    - "anthropic": Anthropic Claude API
    - "ollama": Local Ollama API
    """
    api_format = config.api_format.lower()

    if api_format == "openai":
        return OpenAIProvider(config)
    elif api_format == "openai_responses":
        return OpenAIResponsesProvider(config)
    elif api_format == "anthropic":
        return AnthropicProvider(config)
    elif api_format == "ollama":
        return OllamaProvider(config)
    else:
        raise ValueError(f"Unsupported API format: {config.api_format}. Supported formats: openai, openai_responses, anthropic, ollama")


__all__ = [
    "LLMProvider",
    "ProviderConfig",
    "CompletionRequest",
    "CompletionResponse",
    "Message",
    "OpenAIProvider",
    "OpenAIResponsesProvider",
    "OpenAIResponsesProviderWithReasoning",
    "AnthropicProvider",
    "OllamaProvider",
    "create_provider",
]
