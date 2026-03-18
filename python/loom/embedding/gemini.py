"""Google Gemini Embedding Provider"""

import time
import logging
from typing import List, Optional

from .base import EmbeddingProvider

logger = logging.getLogger(__name__)


class GeminiEmbeddingProvider(EmbeddingProvider):
    """Google Gemini embedding provider using the google-genai library."""

    # Rate limit handling constants
    MAX_RETRIES = 5
    INITIAL_BACKOFF = 1.0  # seconds
    MAX_BACKOFF = 60.0  # seconds
    BACKOFF_MULTIPLIER = 2.0

    def __init__(
        self,
        api_key: str,
        model_name: str = "text-embedding-004",
        proxy_url: Optional[str] = None,
    ):
        self._api_key = api_key
        self._model_name = model_name
        self._proxy_url = proxy_url
        self._client = None

    def _get_client(self):
        if self._client is not None:
            return self._client

        try:
            from google import genai
        except ImportError:
            raise ImportError(
                "google-genai is required for Gemini embedding. "
                "Install with: pip install google-genai"
            )

        http_options = {"timeout": 60000}
        if self._proxy_url:
            http_options["client_args"] = {"proxy": self._proxy_url}

        self._client = genai.Client(api_key=self._api_key, http_options=http_options)
        return self._client

    @property
    def dimensions(self) -> int:
        model_dims = {
            "text-embedding-004": 768,
            "text-embedding-preview-0815": 768,
            "embedding-001": 768,
        }
        return model_dims.get(self._model_name, 768)

    @property
    def model_name(self) -> str:
        return self._model_name

    def embed(self, text: str) -> List[float]:
        return self.embed_batch([text])[0]

    def embed_batch(self, texts: List[str]) -> List[List[float]]:
        client = self._get_client()
        return self._embed_with_retry(client, texts)

    def _embed_with_retry(self, client, texts: List[str]) -> List[List[float]]:
        """Execute embedding request with exponential backoff retry for rate limits."""
        backoff = self.INITIAL_BACKOFF
        last_exception = None

        for attempt in range(self.MAX_RETRIES):
            try:
                response = client.models.embed_content(
                    model=self._model_name,
                    contents=texts,
                )
                return [emb.values for emb in response.embeddings]
            except Exception as e:
                error_str = str(e).lower()
                # Check if this is a rate limit error (429 / RESOURCE_EXHAUSTED)
                if "429" in error_str or "resource_exhausted" in error_str or "rate" in error_str:
                    last_exception = e
                    if attempt < self.MAX_RETRIES - 1:
                        logger.warning(
                            f"Gemini API rate limit hit (attempt {attempt + 1}/{self.MAX_RETRIES}), "
                            f"retrying in {backoff:.1f}s..."
                        )
                        time.sleep(backoff)
                        backoff = min(backoff * self.BACKOFF_MULTIPLIER, self.MAX_BACKOFF)
                        continue
                # For non-rate-limit errors, or final retry, raise immediately
                raise

        # All retries exhausted
        raise last_exception or Exception("Gemini embedding failed after retries")
