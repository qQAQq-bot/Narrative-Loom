"""
OpenAI Responses API Provider.

This provider implements the newer OpenAI Responses API format.
API Documentation: https://doc.ai-api.chat/openai-responses
"""

import asyncio
import httpx
import json
import re
import time
from typing import Optional, AsyncIterator
from .base import LLMProvider, ProviderConfig, CompletionRequest, CompletionResponse, Message
from ..logger import log_info, log_error


def is_retryable_error(error: Exception) -> bool:
    """Check if an error is retryable."""
    if isinstance(error, httpx.TimeoutException):
        return True
    if isinstance(error, httpx.ConnectError):
        return True
    if isinstance(error, httpx.HTTPStatusError):
        # Retry on 5xx errors and 429 (rate limit)
        return error.response.status_code >= 500 or error.response.status_code == 429
    if isinstance(error, ValueError) and "Empty response" in str(error):
        return True
    return False


class OpenAIResponsesProvider(LLMProvider):
    """Provider for OpenAI Responses API (/v1/responses endpoint)."""

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
        """Build payload for Responses API."""
        instructions = None
        input_text = ""

        for m in request.messages:
            if m.role == "system":
                if instructions:
                    instructions += "\n" + m.content
                else:
                    instructions = m.content
            else:
                if input_text:
                    input_text += "\n"
                if m.role == "assistant":
                    input_text += f"助手: {m.content}"
                else:
                    input_text += m.content

        payload = {
            "model": self.get_model(request),
            "input": input_text,
        }

        if instructions:
            payload["instructions"] = instructions

        if request.temperature is not None and request.temperature != 0.7:
            payload["temperature"] = request.temperature

        if request.max_tokens:
            payload["max_output_tokens"] = request.max_tokens

        return payload

    def _extract_content(self, result: dict) -> str:
        """Extract content from Responses API response."""
        content = ""
        if "output" in result:
            for item in result["output"]:
                if item.get("type") == "message":
                    for c in item.get("content", []):
                        if c.get("type") == "output_text":
                            content += c.get("text", "")
                        elif c.get("type") == "text":
                            content += c.get("text", "")
        elif "output_text" in result:
            content = result["output_text"]
        elif "text" in result:
            content = result["text"]

        content = content.strip()

        # Try to extract JSON from markdown code block anywhere in content
        # Match ```json ... ``` or ``` ... ```
        json_match = re.search(r'```json\s*([\s\S]*?)```', content)
        if json_match:
            return json_match.group(1).strip()

        generic_match = re.search(r'```\s*([\s\S]*?)```', content)
        if generic_match:
            return generic_match.group(1).strip()

        # Fallback: original stripping logic for backward compatibility
        if content.startswith("```json"):
            content = content[7:]
        elif content.startswith("```"):
            content = content[3:]
        if content.endswith("```"):
            content = content[:-3]

        return content.strip()

    async def complete(self, request: CompletionRequest) -> CompletionResponse:
        """Complete using Responses API (non-streaming)."""
        payload = self._build_payload(request)
        url = f"{self.config.api_base.rstrip('/')}/v1/responses"

        log_info(f"[LLM:Responses] POST {url} | model={payload.get('model')} | input={len(payload.get('input', ''))}chars")
        log_info(f"[LLM:Responses] Request payload: {json.dumps(payload, ensure_ascii=False)}")

        start_time = time.time()
        try:
            async with httpx.AsyncClient(timeout=self.timeout) as client:
                response = await client.post(url, json=payload, headers=self._headers())
                response.raise_for_status()
                result = response.json()

                content = self._extract_content(result)
                duration = time.time() - start_time
                usage = result.get("usage", {})
                log_info(f"[LLM:Responses] OK ({duration:.2f}s) | output={len(content)}chars | tokens: in={usage.get('input_tokens', 0)}, out={usage.get('output_tokens', 0)}")

                return CompletionResponse(
                    content=content,
                    model=result.get("model", self.get_model(request)),
                    usage={
                        "input_tokens": usage.get("input_tokens", 0),
                        "output_tokens": usage.get("output_tokens", 0),
                        "total_tokens": usage.get("total_tokens", 0),
                    },
                    finish_reason=result.get("status", "completed"),
                )
        except httpx.TimeoutException as e:
            log_error(f"[LLM:Responses] Timeout ({self.timeout}s): {e}")
            raise Exception(f"LLM 请求超时 ({self.timeout}s)") from e
        except httpx.HTTPStatusError as e:
            log_error(f"[LLM:Responses] HTTP {e.response.status_code}: {e.response.text[:200]}")
            raise Exception(f"LLM API 错误 (HTTP {e.response.status_code})") from e
        except httpx.ConnectError as e:
            log_error(f"[LLM:Responses] Connect failed: {self.config.api_base}")
            raise Exception(f"无法连接到 LLM 服务") from e
        except Exception as e:
            duration = time.time() - start_time
            log_error(f"[LLM:Responses] Error ({duration:.2f}s): {e}")
            raise

    async def complete_json(self, request: CompletionRequest, schema: dict) -> dict:
        """Complete with structured JSON output using JSON schema mode with retry support."""
        max_retries = getattr(self.config, 'max_retries', 3)
        retry_delay = getattr(self.config, 'retry_delay', 1.0)
        last_error = None

        for attempt in range(max_retries + 1):
            try:
                return await self._do_complete_json(request, schema)
            except Exception as e:
                last_error = e
                if attempt < max_retries and is_retryable_error(e):
                    wait_time = retry_delay * (2 ** attempt)  # 指数退避
                    log_info(f"[LLM:Responses] Retry {attempt + 1}/{max_retries} after {wait_time:.1f}s: {e}")
                    await asyncio.sleep(wait_time)
                else:
                    if attempt > 0:
                        log_error(f"[LLM:Responses] All {max_retries} retries failed")
                    raise

        raise last_error

    async def _do_complete_json(self, request: CompletionRequest, schema: dict) -> dict:
        """Internal method to complete with structured JSON output."""
        payload = self._build_payload(request)
        url = f"{self.config.api_base.rstrip('/')}/v1/responses"

        # Add structured output format
        schema_with_strict = self._add_additional_properties_false(schema)
        payload["text"] = {
            "format": {
                "type": "json_schema",
                "name": "analysis_result",
                "schema": schema_with_strict,
                "strict": True,
            }
        }

        log_info(f"[LLM:Responses] POST {url} (JSON schema) | model={payload.get('model')} | input={len(payload.get('input', ''))}chars")
        log_info(f"[LLM:Responses] Request payload: {json.dumps(payload, ensure_ascii=False)}")

        start_time = time.time()
        async with httpx.AsyncClient(timeout=self.timeout) as client:
            response = await client.post(url, json=payload, headers=self._headers())
            response.raise_for_status()
            result = response.json()

            # Log full response for debugging
            log_info(f"[LLM:Responses] Raw response: {json.dumps(result, ensure_ascii=False)}")

            content = self._extract_content(result)
            duration = time.time() - start_time
            usage = result.get("usage", {})

            if not content:
                if usage.get("input_tokens", 0) == 0 and usage.get("output_tokens", 0) == 0:
                    log_error(f"[LLM:Responses] Empty response | status={result.get('status')} | tokens: in=0, out=0")
                else:
                    log_error(f"[LLM:Responses] Empty response | tokens: in={usage.get('input_tokens', 0)}, out={usage.get('output_tokens', 0)}")
                raise ValueError("Empty response from API")

            log_info(f"[LLM:Responses] OK ({duration:.2f}s) | output={len(content)}chars | tokens: in={usage.get('input_tokens', 0)}, out={usage.get('output_tokens', 0)}")

            return json.loads(content)

    async def complete_stream(self, request: CompletionRequest) -> AsyncIterator[str]:
        """Complete using Responses API with streaming."""
        payload = self._build_payload(request)
        payload["stream"] = True
        url = f"{self.config.api_base.rstrip('/')}/v1/responses"

        log_info(f"[LLM:Responses] POST {url} (streaming)")
        log_info(f"[LLM:Responses] Model: {payload.get('model')}")

        async with httpx.AsyncClient(timeout=self.timeout) as client:
            async with client.stream("POST", url, json=payload, headers=self._headers()) as response:
                response.raise_for_status()

                async for line in response.aiter_lines():
                    if not line or line.startswith("event:"):
                        continue

                    if line.startswith("data:"):
                        data_str = line[5:].strip()
                        if not data_str:
                            continue

                        try:
                            data = json.loads(data_str)
                            event_type = data.get("type", "")

                            if event_type == "response.output_text.delta":
                                delta = data.get("delta", "")
                                if delta:
                                    yield delta
                            elif event_type == "response.completed":
                                break
                            elif event_type == "error":
                                error_msg = data.get("error", {}).get("message", "Unknown error")
                                raise ValueError(f"Streaming error: {error_msg}")
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


class OpenAIResponsesProviderWithReasoning(OpenAIResponsesProvider):
    """Extended Responses API provider with reasoning token support."""

    def __init__(self, config: ProviderConfig, reasoning_effort: str = "medium"):
        super().__init__(config)
        self.reasoning_effort = reasoning_effort

    def _build_payload(self, request: CompletionRequest) -> dict:
        payload = super()._build_payload(request)
        payload["reasoning"] = {"effort": self.reasoning_effort}
        return payload

