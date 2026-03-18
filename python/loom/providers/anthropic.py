import httpx
import json
from typing import Optional
from .base import LLMProvider, ProviderConfig, CompletionRequest, CompletionResponse, Message


class AnthropicProvider(LLMProvider):
    """Provider for Anthropic Claude API."""

    def __init__(self, config: ProviderConfig):
        super().__init__(config)
        self.client = httpx.AsyncClient(
            base_url=config.api_base.rstrip("/"),
            headers=self._build_headers(),
            timeout=120.0,
        )

    def _build_headers(self) -> dict:
        headers = {
            "x-api-key": self.config.api_key,
            "anthropic-version": "2023-06-01",
            "Content-Type": "application/json",
        }
        if self.config.extra_headers:
            headers.update(self.config.extra_headers)
        return headers

    async def complete(self, request: CompletionRequest) -> CompletionResponse:
        payload = self._build_payload(request)

        response = await self._send_request(payload)

        # Extract content from Anthropic's response format
        content_blocks = response.get("content", [])
        content = ""
        for block in content_blocks:
            if block.get("type") == "text":
                content += block.get("text", "")

        return CompletionResponse(
            content=content,
            model=response.get("model", self.get_model(request)),
            usage={
                "input_tokens": response.get("usage", {}).get("input_tokens", 0),
                "output_tokens": response.get("usage", {}).get("output_tokens", 0),
            },
            finish_reason=response.get("stop_reason", "end_turn"),
        )

    async def complete_json(self, request: CompletionRequest, schema: dict) -> dict:
        # Add JSON instruction to the last user message
        if request.messages and request.messages[-1].role == "user":
            request.messages[
                -1
            ].content += "\n\nRespond with valid JSON matching the required schema. Output only the JSON object, no additional text."

        response = await self.complete(request)

        # Try to extract JSON from the response
        content = response.content.strip()

        # Handle potential markdown code blocks
        if content.startswith("```json"):
            content = content[7:]
        elif content.startswith("```"):
            content = content[3:]
        if content.endswith("```"):
            content = content[:-3]
        content = content.strip()

        try:
            return json.loads(content)
        except json.JSONDecodeError as e:
            raise ValueError(f"Invalid JSON response: {e}\nContent: {content[:500]}")

    def _build_payload(self, request: CompletionRequest) -> dict:
        # Convert messages to Anthropic format
        # Anthropic uses system parameter separately
        system_content = None
        messages = []

        for msg in request.messages:
            if msg.role == "system":
                system_content = msg.content
            else:
                messages.append({
                    "role": msg.role,
                    "content": msg.content
                })

        payload = {
            "model": self.get_model(request),
            "messages": messages,
            "max_tokens": request.max_tokens or 4096,
        }

        if system_content:
            payload["system"] = system_content

        # Anthropic doesn't support temperature=0, use very small value instead
        if request.temperature == 0:
            payload["temperature"] = 0.01
        else:
            payload["temperature"] = request.temperature

        return payload

    async def _send_request(self, payload: dict, retries: int = 3) -> dict:
        last_error = None
        endpoint = "/v1/messages"
        full_url = f"{self.config.api_base.rstrip('/')}{endpoint}"
        model = payload.get("model", self.config.default_model)

        # Log request info to stderr (not stdout, which is used for JSON-RPC)
        import sys
        print(f"[LLM] Request to {full_url}", file=sys.stderr, flush=True)
        print(f"[LLM] Model: {model}", file=sys.stderr, flush=True)

        for attempt in range(retries):
            try:
                response = await self.client.post(endpoint, json=payload)
                response.raise_for_status()
                return response.json()
            except httpx.HTTPStatusError as e:
                last_error = e
                if e.response.status_code == 429:
                    # Rate limited, wait and retry
                    import asyncio
                    await asyncio.sleep(2 ** attempt)
                    continue
                elif e.response.status_code == 529:
                    # Overloaded, wait and retry
                    import asyncio
                    await asyncio.sleep(5 * (attempt + 1))
                    continue
                raise
            except httpx.RequestError as e:
                last_error = e
                import asyncio
                await asyncio.sleep(1)
                continue

        raise last_error

    async def close(self):
        await self.client.aclose()
