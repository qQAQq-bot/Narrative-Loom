import httpx
import json
from typing import Optional
from .base import LLMProvider, ProviderConfig, CompletionRequest, CompletionResponse, Message


class OllamaProvider(LLMProvider):
    """Provider for local Ollama API."""

    def __init__(self, config: ProviderConfig):
        super().__init__(config)
        # Ollama doesn't require API key, but we accept it for consistency
        self.client = httpx.AsyncClient(
            base_url=config.api_base.rstrip("/"),
            headers=self._build_headers(),
            timeout=300.0,  # Longer timeout for local models
        )

    def _build_headers(self) -> dict:
        headers = {
            "Content-Type": "application/json",
        }
        if self.config.extra_headers:
            headers.update(self.config.extra_headers)
        return headers

    async def complete(self, request: CompletionRequest) -> CompletionResponse:
        payload = self._build_payload(request)

        response = await self._send_request(payload)

        # Ollama chat endpoint returns message directly
        message = response.get("message", {})
        content = message.get("content", "")

        return CompletionResponse(
            content=content,
            model=response.get("model", self.get_model(request)),
            usage={
                "prompt_tokens": response.get("prompt_eval_count", 0),
                "completion_tokens": response.get("eval_count", 0),
                "total_tokens": response.get("prompt_eval_count", 0) + response.get("eval_count", 0),
            },
            finish_reason=response.get("done_reason", "stop"),
        )

    async def complete_json(self, request: CompletionRequest, schema: dict) -> dict:
        # Add JSON instruction to the last user message
        if request.messages and request.messages[-1].role == "user":
            request.messages[
                -1
            ].content += "\n\nRespond with valid JSON matching the required schema. Output only the JSON object, no additional text."

        # Try to use Ollama's JSON format if available
        request.response_format = {"type": "json_object"}

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
        # Convert messages to Ollama chat format
        messages = []
        for msg in request.messages:
            messages.append({
                "role": msg.role,
                "content": msg.content
            })

        payload = {
            "model": self.get_model(request),
            "messages": messages,
            "stream": False,
            "options": {
                "temperature": request.temperature,
            }
        }

        if request.max_tokens:
            payload["options"]["num_predict"] = request.max_tokens

        # Ollama supports JSON format mode
        if request.response_format and request.response_format.get("type") == "json_object":
            payload["format"] = "json"

        return payload

    async def _send_request(self, payload: dict, retries: int = 3) -> dict:
        last_error = None
        endpoint = "/api/chat"
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
                if e.response.status_code >= 500:
                    # Server error, wait and retry
                    import asyncio
                    await asyncio.sleep(2 ** attempt)
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

    async def check_model_available(self, model: str) -> bool:
        """Check if a model is available locally."""
        try:
            response = await self.client.get("/api/tags")
            response.raise_for_status()
            data = response.json()
            models = [m.get("name", "") for m in data.get("models", [])]
            return model in models or any(model in m for m in models)
        except Exception:
            return False

    async def list_models(self) -> list[str]:
        """List all available local models."""
        try:
            response = await self.client.get("/api/tags")
            response.raise_for_status()
            data = response.json()
            return [m.get("name", "") for m in data.get("models", [])]
        except Exception:
            return []
