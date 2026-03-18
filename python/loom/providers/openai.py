"""
OpenAI Chat Completions API Provider.

This provider implements the standard OpenAI Chat Completions API format.
API Documentation: https://doc.ai-api.chat/openai-chat/
"""

import httpx
import json
import time
from typing import AsyncIterator
from .base import LLMProvider, ProviderConfig, CompletionRequest, CompletionResponse, Message
from ..logger import log_info, log_error


class OpenAIProvider(LLMProvider):
    """Provider for OpenAI Chat Completions API (/v1/chat/completions endpoint)."""

    def __init__(self, config: ProviderConfig):
        super().__init__(config)
        self.timeout = 120.0

    def _headers(self) -> dict:
        headers = {
            "Authorization": f"Bearer {self.config.api_key}",
            "Content-Type": "application/json",
        }
        if self.config.extra_headers:
            headers.update(self.config.extra_headers)
        return headers

    def _build_payload(self, request: CompletionRequest) -> dict:
        """Build payload for Chat Completions API."""
        messages = [{"role": m.role, "content": m.content} for m in request.messages]

        payload = {
            "model": self.get_model(request),
            "messages": messages,
        }

        if request.temperature is not None:
            payload["temperature"] = request.temperature

        if request.max_tokens:
            payload["max_completion_tokens"] = request.max_tokens

        return payload

    async def complete(self, request: CompletionRequest) -> CompletionResponse:
        """Complete using Chat Completions API (non-streaming)."""
        payload = self._build_payload(request)
        url = f"{self.config.api_base.rstrip('/')}/v1/chat/completions"

        log_info(f"[LLM:OpenAI] POST {url}")
        log_info(f"[LLM:OpenAI] Model: {payload.get('model')}")
        log_info(f"[LLM:OpenAI] Messages: {len(payload.get('messages', []))} messages")

        start_time = time.time()
        try:
            async with httpx.AsyncClient(timeout=self.timeout) as client:
                response = await client.post(url, json=payload, headers=self._headers())
                response.raise_for_status()
                result = response.json()

                choice = result["choices"][0]
                content = choice.get("message", {}).get("content", "")
                duration = time.time() - start_time
                log_info(f"[LLM:OpenAI] Response OK ({duration:.2f}s), content length: {len(content)}")

                usage = result.get("usage", {})
                return CompletionResponse(
                    content=content,
                    model=result.get("model", self.get_model(request)),
                    usage={
                        "prompt_tokens": usage.get("prompt_tokens", 0),
                        "completion_tokens": usage.get("completion_tokens", 0),
                        "total_tokens": usage.get("total_tokens", 0),
                    },
                    finish_reason=choice.get("finish_reason", "stop"),
                )
        except httpx.TimeoutException as e:
            duration = time.time() - start_time
            error_msg = f"LLM 请求超时 ({self.timeout}s): {e}"
            log_error(f"[LLM:OpenAI] {error_msg}")
            raise Exception(error_msg) from e
        except httpx.HTTPStatusError as e:
            duration = time.time() - start_time
            error_msg = f"LLM API 错误 (HTTP {e.response.status_code}): {e.response.text[:200]}"
            log_error(f"[LLM:OpenAI] {error_msg}")
            raise Exception(error_msg) from e
        except httpx.ConnectError as e:
            duration = time.time() - start_time
            error_msg = f"无法连接到 LLM 服务 ({self.config.api_base}): {e}"
            log_error(f"[LLM:OpenAI] {error_msg}")
            raise Exception(error_msg) from e
        except Exception as e:
            duration = time.time() - start_time
            log_error(f"[LLM:OpenAI] Error ({duration:.2f}s): {e}")
            raise

    async def complete_json(self, request: CompletionRequest, schema: dict) -> dict:
        """Complete with structured JSON output."""
        payload = self._build_payload(request)
        url = f"{self.config.api_base.rstrip('/')}/v1/chat/completions"

        # Add structured output format
        schema_with_strict = self._add_additional_properties_false(schema)
        payload["response_format"] = {
            "type": "json_schema",
            "json_schema": {
                "name": "analysis_result",
                "schema": schema_with_strict,
                "strict": True,
            }
        }

        log_info(f"[LLM:OpenAI] POST {url} (JSON mode)")
        log_info(f"[LLM:OpenAI] Model: {payload.get('model')}")
        log_info(f"[LLM:OpenAI] Messages: {len(payload.get('messages', []))} messages")

        start_time = time.time()
        try:
            async with httpx.AsyncClient(timeout=self.timeout) as client:
                response = await client.post(url, json=payload, headers=self._headers())
                response.raise_for_status()
                result = response.json()

                choice = result["choices"][0]
                content = choice.get("message", {}).get("content", "")
                duration = time.time() - start_time
                log_info(f"[LLM:OpenAI] Response OK ({duration:.2f}s), content length: {len(content)}")

                return json.loads(content)
        except httpx.TimeoutException as e:
            duration = time.time() - start_time
            error_msg = f"LLM 请求超时 ({self.timeout}s): {e}"
            log_error(f"[LLM:OpenAI] {error_msg}")
            raise Exception(error_msg) from e
        except httpx.HTTPStatusError as e:
            duration = time.time() - start_time
            error_msg = f"LLM API 错误 (HTTP {e.response.status_code}): {e.response.text[:200]}"
            log_error(f"[LLM:OpenAI] {error_msg}")
            raise Exception(error_msg) from e
        except httpx.ConnectError as e:
            duration = time.time() - start_time
            error_msg = f"无法连接到 LLM 服务 ({self.config.api_base}): {e}"
            log_error(f"[LLM:OpenAI] {error_msg}")
            raise Exception(error_msg) from e
        except json.JSONDecodeError as e:
            log_error(f"[LLM:OpenAI] Invalid JSON response: {e}")
            raise ValueError(f"Invalid JSON response: {e}") from e
        except Exception as e:
            duration = time.time() - start_time
            log_error(f"[LLM:OpenAI] Error ({duration:.2f}s): {e}")
            raise

    async def complete_stream(self, request: CompletionRequest) -> AsyncIterator[str]:
        """Complete using Chat Completions API with streaming."""
        payload = self._build_payload(request)
        payload["stream"] = True
        url = f"{self.config.api_base.rstrip('/')}/v1/chat/completions"

        log_info(f"[LLM:OpenAI] POST {url} (streaming)")
        log_info(f"[LLM:OpenAI] Model: {payload.get('model')}")

        async with httpx.AsyncClient(timeout=self.timeout) as client:
            async with client.stream("POST", url, json=payload, headers=self._headers()) as response:
                response.raise_for_status()

                async for line in response.aiter_lines():
                    if not line:
                        continue

                    if line.startswith("data:"):
                        data_str = line[5:].strip()
                        if data_str == "[DONE]":
                            break
                        if not data_str:
                            continue

                        try:
                            data = json.loads(data_str)
                            choices = data.get("choices", [])
                            if choices:
                                delta = choices[0].get("delta", {})
                                content = delta.get("content")
                                if content:
                                    yield content
                                if choices[0].get("finish_reason"):
                                    break
                        except json.JSONDecodeError:
                            continue

    def _add_additional_properties_false(self, schema: dict) -> dict:
        """Recursively add additionalProperties: false to all object types."""
        if not isinstance(schema, dict):
            return schema

        result = schema.copy()

        if result.get("type") == "object":
            result["additionalProperties"] = False
            if "properties" in result:
                result["properties"] = {
                    k: self._add_additional_properties_false(v)
                    for k, v in result["properties"].items()
                }

        if "items" in result:
            result["items"] = self._add_additional_properties_false(result["items"])

        for key in ("anyOf", "oneOf", "allOf"):
            if key in result:
                result[key] = [self._add_additional_properties_false(s) for s in result[key]]

        return result

    async def close(self):
        """Close the provider (no-op for this implementation)."""
        pass
