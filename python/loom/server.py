import asyncio
import json
import sys
import threading
import os
from pathlib import Path
from typing import Any, Optional

from .embedding import (
    ChineseChunker,
    EmbeddingProvider,
    OpenAIEmbeddingProvider,
    GeminiEmbeddingProvider,
)
from .providers import ProviderConfig, create_provider
from .agents import AgentConfig, AnalysisContext, create_agent
from .logger import log_info, log_error, log_exception, init_logging

_embedding_provider: Optional[EmbeddingProvider] = None
_chunker = ChineseChunker()

# Global cancellation flag for analysis (thread-safe)
_analysis_cancelled = threading.Event()
_current_analysis_id: Optional[str] = None

# Cancel flag file path (cross-platform solution)
_cancel_flag_file: Optional[Path] = None


def _get_cancel_flag_path() -> Path:
    """Get the path to the cancel flag file"""
    # Use temp directory for the cancel flag
    if sys.platform == "win32":
        temp_dir = Path(os.environ.get("TEMP", os.environ.get("TMP", ".")))
    else:
        temp_dir = Path("/tmp")
    return temp_dir / "narrative_loom_cancel_flag"


def _check_cancel_flag_file() -> bool:
    """Check if the cancel flag file exists"""
    flag_path = _get_cancel_flag_path()
    return flag_path.exists()


def _clear_cancel_flag_file():
    """Remove the cancel flag file"""
    flag_path = _get_cancel_flag_path()
    try:
        if flag_path.exists():
            flag_path.unlink()
            log_info(f"[cancel] Cleared cancel flag file: {flag_path}")
    except Exception as e:
        log_error(f"[cancel] Failed to clear cancel flag file: {e}")


def get_embedding_provider(config: Optional[dict] = None) -> EmbeddingProvider:
    global _embedding_provider

    if config is None:
        raise ValueError("Embedding config is required")

    provider_type = config.get("provider", "")

    if provider_type == "openai":
        api_key = config.get("api_key", "")
        model = config.get("model", "text-embedding-3-small")
        base_url = config.get("base_url", "https://api.openai.com/v1")
        dimensions = config.get("dimensions")
        if dimensions is not None:
            try:
                dimensions = int(dimensions)
            except Exception:
                raise ValueError(f"Invalid dimensions value: {dimensions}")
        return OpenAIEmbeddingProvider(
            api_key=api_key,
            model_name=model,
            base_url=base_url,
            dimensions=dimensions,
        )
    elif provider_type == "gemini":
        api_key = config.get("api_key", "")
        model = config.get("model", "text-embedding-004")
        proxy_url = config.get("proxy_url")
        return GeminiEmbeddingProvider(api_key=api_key, model_name=model, proxy_url=proxy_url)
    elif provider_type == "disabled":
        raise ValueError("Embedding is disabled in settings")
    else:
        raise ValueError(f"Unsupported embedding provider: {provider_type}")


def handle_request(request: dict[str, Any]) -> dict[str, Any]:
    method = request.get("method", "")
    params = request.get("params", {})
    request_id = request.get("id")

    try:
        if method == "ping":
            result = "pong"
        elif method == "analyze_chapter":
            # analyze_chapter is async, need to run it
            result = asyncio.run(analyze_chapter(params))
        elif method == "analyze_single_agent":
            # analyze a single agent type
            result = asyncio.run(analyze_single_agent(params))
        elif method == "cancel_analysis":
            result = cancel_analysis(params)
        elif method == "chunk_text":
            result = chunk_text(params)
        elif method == "generate_embedding":
            result = generate_embedding(params)
        elif method == "generate_embeddings":
            result = generate_embeddings(params)
        elif method == "fetch_embedding_models":
            result = fetch_embedding_models(params)
        else:
            return {
                "jsonrpc": "2.0",
                "error": {"code": -32601, "message": f"Method not found: {method}"},
                "id": request_id,
            }

        return {"jsonrpc": "2.0", "result": result, "id": request_id}

    except Exception as e:
        import traceback

        error_msg = f"{str(e)}\n{traceback.format_exc()}"
        return {
            "jsonrpc": "2.0",
            "error": {"code": -32000, "message": error_msg},
            "id": request_id,
        }


def cancel_analysis(params: dict[str, Any]) -> dict[str, Any]:
    """
    取消当前正在进行的分析任务

    params:
        - analysis_id: 分析任务 ID (可选，用于验证)
    """
    global _analysis_cancelled, _current_analysis_id

    analysis_id = params.get("analysis_id")

    log_info(f"[cancel_analysis] Received cancel request, current_id={_current_analysis_id}, request_id={analysis_id}")

    # Set the cancellation flag
    _analysis_cancelled.set()

    log_info("[cancel_analysis] Cancellation flag set")

    return {
        "cancelled": True,
        "analysis_id": _current_analysis_id,
    }


def is_analysis_cancelled() -> bool:
    """检查分析是否已被取消（检查内存标志和文件标志）"""
    # Check both the in-memory flag and the file flag
    if _analysis_cancelled.is_set():
        return True
    if _check_cancel_flag_file():
        log_info("[cancel] Detected cancel flag file, setting cancellation")
        _analysis_cancelled.set()
        return True
    return False


def reset_cancellation():
    """重置取消标志，用于开始新的分析任务"""
    global _analysis_cancelled
    _analysis_cancelled.clear()
    _clear_cancel_flag_file()


def _build_prompt_prefix_suffix(prompt_cards: list[dict]) -> tuple[Optional[str], Optional[str]]:
    """
    从 prompt_cards 构建 prefix 和 suffix 文本

    Args:
        prompt_cards: 提示词卡片列表，每个卡片包含 enabled, position, order, content

    Returns:
        (prefix_text, suffix_text) 元组
    """
    if not prompt_cards:
        return None, None

    # 过滤启用的卡片并按 (position, order) 排序
    enabled_cards = [c for c in prompt_cards if c.get("enabled", False)]
    enabled_cards.sort(key=lambda c: (c.get("position", "prefix"), c.get("order", 0)))

    # 分别构建 prefix 和 suffix
    prefix_parts = []
    suffix_parts = []

    for card in enabled_cards:
        content = card.get("content", "").strip()
        if not content:
            continue

        position = card.get("position", "prefix")
        if position == "prefix":
            prefix_parts.append(content)
        else:
            suffix_parts.append(content)

    prefix_text = "\n\n".join(prefix_parts) if prefix_parts else None
    suffix_text = "\n\n".join(suffix_parts) if suffix_parts else None

    return prefix_text, suffix_text


async def analyze_chapter(params: dict[str, Any]) -> dict[str, Any]:
    """
    分析章节内容，返回技法卡片和知识卡片

    params:
        - content: 章节内容
        - chapter_index: 章节序号
        - chapter_title: 章节标题 (可选)
        - book_title: 书名
        - agent_configs: 每种分析类型的 Agent 配置 (新格式)
        - provider_config: LLM Provider 配置 (旧格式，保持兼容)
        - analysis_types: 分析类型列表 (可选，默认 ["technique", "character", "setting", "event"])
    """
    global _current_analysis_id

    # Reset cancellation flag at the start of a new analysis
    reset_cancellation()

    # Generate a unique analysis ID
    import uuid
    _current_analysis_id = str(uuid.uuid4())[:8]

    content = params.get("content", "")
    chapter_index = params.get("chapter_index", 1)
    chapter_title = params.get("chapter_title")
    book_title = params.get("book_title", "未知书籍")
    agent_configs = params.get("agent_configs", {})
    legacy_provider_config = params.get("provider_config", {})
    analysis_types = params.get("analysis_types", ["technique", "character", "setting", "event"])
    prompt_cards = params.get("prompt_cards", [])

    # Build global prefix/suffix from prompt cards
    system_prompt_prefix, system_prompt_suffix = _build_prompt_prefix_suffix(prompt_cards)
    if system_prompt_prefix:
        log_info(f"[analyze_chapter] Global system prompt prefix: {len(system_prompt_prefix)} chars")
    if system_prompt_suffix:
        log_info(f"[analyze_chapter] Global system prompt suffix: {len(system_prompt_suffix)} chars")

    log_info("")
    log_info("=" * 60)
    log_info(f"[analyze_chapter] Starting analysis for '{book_title}' chapter {chapter_index} (id={_current_analysis_id})")
    log_info("=" * 60)
    log_info(f"[analyze_chapter] Content length: {len(content)} chars")
    log_info(f"[analyze_chapter] Agent configs received: {list(agent_configs.keys())}")
    log_info(f"[analyze_chapter] Analysis types to process: {analysis_types}")
    log_info("")

    # 构建分析上下文
    context = AnalysisContext(
        chapter_content=content,
        chapter_index=chapter_index,
        chapter_title=chapter_title,
        book_title=book_title,
        previous_summary=params.get("previous_summary"),
        known_characters=params.get("known_characters"),
        known_settings=params.get("known_settings"),
        known_events=params.get("known_events"),
        current_profile=params.get("current_profile"),  # For style analysis incremental refinement
    )

    results = {
        "techniques": [],
        "characters": [],
        "settings": [],
        "events": [],
        "style": {},
        "cancelled": False,
    }

    # Track created providers for cleanup
    providers_to_close = []

    try:
        # 运行各个分析 Agent
        for idx, analysis_type in enumerate(analysis_types):
            # Check for cancellation before starting each agent
            if is_analysis_cancelled():
                log_info(f"[analyze_chapter] Analysis cancelled before agent {analysis_type}")
                results["cancelled"] = True
                break

            log_info("")
            log_info("=" * 40)
            log_info(f"[analyze_chapter] Processing Agent {idx+1}/{len(analysis_types)}: {analysis_type}")
            log_info("=" * 40)

            # Check if we have a specific agent config for this analysis type
            if analysis_type in agent_configs:
                agent_cfg = agent_configs[analysis_type]
                provider_cfg_data = agent_cfg.get("provider_config", {})

                log_info(f"[analyze_chapter] Using agent config for {analysis_type}:")
                log_info(f"  - provider_name: {provider_cfg_data.get('name')}")
                log_info(f"  - api_base: {provider_cfg_data.get('api_base')}")
                log_info(f"  - api_format: {provider_cfg_data.get('api_format')}")
                log_info(f"  - model: {agent_cfg.get('model')}")

                # Create provider from agent-specific config
                provider_cfg = ProviderConfig(
                    name=provider_cfg_data.get("name", "default"),
                    api_base=provider_cfg_data.get("api_base", "https://api.openai.com/v1"),
                    api_key=provider_cfg_data.get("api_key", ""),
                    default_model=provider_cfg_data.get("model", "gpt-4o-mini"),
                    api_format=provider_cfg_data.get("api_format", "openai"),
                    max_retries=provider_cfg_data.get("max_retries", 3),
                )
                provider = create_provider(provider_cfg)
                providers_to_close.append(provider)

                # Create agent config with specific settings
                agent_config = AgentConfig(
                    name=f"{analysis_type}_agent",
                    kind=analysis_type,
                    provider_name=provider_cfg.name,
                    model=agent_cfg.get("model", provider_cfg.default_model),
                    temperature=agent_cfg.get("temperature", 0.7),
                    max_tokens=agent_cfg.get("max_tokens"),
                    system_prompt=agent_cfg.get("system_prompt"),
                    system_prompt_prefix=system_prompt_prefix,
                    system_prompt_suffix=system_prompt_suffix,
                )
            else:
                log_info(f"[analyze_chapter] No agent config for {analysis_type}, using legacy provider")

                # Fallback to legacy provider_config
                provider_cfg = ProviderConfig(
                    name=legacy_provider_config.get("name", "default"),
                    api_base=legacy_provider_config.get("api_base", "https://api.openai.com/v1"),
                    api_key=legacy_provider_config.get("api_key", ""),
                    default_model=legacy_provider_config.get("model", "gpt-4o-mini"),
                    api_format=legacy_provider_config.get("api_format", "openai"),
                    max_retries=legacy_provider_config.get("max_retries", 3),
                )
                provider = create_provider(provider_cfg)
                providers_to_close.append(provider)

                agent_config = AgentConfig(
                    name=f"{analysis_type}_agent",
                    kind=analysis_type,
                    provider_name=provider_cfg.name,
                    model=provider_cfg.default_model,
                    temperature=0.7,
                    system_prompt_prefix=system_prompt_prefix,
                    system_prompt_suffix=system_prompt_suffix,
                )

            try:
                agent = create_agent(agent_config, provider)
                log_info(f"[analyze_chapter] Running agent {analysis_type}...")
                result = await agent.analyze(context)

                # Check for cancellation after agent completes
                if is_analysis_cancelled():
                    log_info(f"[analyze_chapter] Analysis cancelled after agent {analysis_type}")
                    results["cancelled"] = True
                    break

                log_info(f"[analyze_chapter] Agent {analysis_type} raw result: {result}")

                # 合并结果
                if analysis_type == "technique":
                    results["techniques"] = result.get("techniques", [])
                    log_info(f"[analyze_chapter] Got {len(results['techniques'])} techniques")
                elif analysis_type == "character":
                    results["characters"] = result.get("characters", [])
                    log_info(f"[analyze_chapter] Got {len(results['characters'])} characters")
                elif analysis_type == "setting":
                    results["settings"] = result.get("settings", [])
                    log_info(f"[analyze_chapter] Got {len(results['settings'])} settings")
                elif analysis_type == "event":
                    results["events"] = result.get("events", [])
                    log_info(f"[analyze_chapter] Got {len(results['events'])} events")
                elif analysis_type == "style":
                    results["style"] = result
                    log_info(f"[analyze_chapter] Got style analysis result")
            except Exception as e:
                # 单个 Agent 失败不影响其他
                log_exception(f"[analyze_chapter] Agent {analysis_type} FAILED", e)
    finally:
        # Close all created providers
        for provider in providers_to_close:
            await provider.close()
        # Reset current analysis ID
        _current_analysis_id = None

    if results["cancelled"]:
        log_info(f"[analyze_chapter] Analysis was cancelled. Partial results: techniques={len(results['techniques'])}, characters={len(results['characters'])}, settings={len(results['settings'])}, events={len(results['events'])}")
    else:
        log_info(f"[analyze_chapter] Final results: techniques={len(results['techniques'])}, characters={len(results['characters'])}, settings={len(results['settings'])}, events={len(results['events'])}")
    return results


async def analyze_single_agent(params: dict[str, Any]) -> dict[str, Any]:
    """
    分析单个 agent 类型

    params:
        - content: 章节内容
        - chapter_index: 章节序号
        - chapter_title: 章节标题 (可选)
        - book_title: 书名
        - agent_type: 分析类型 ("technique", "character", "setting", "event")
        - agent_config: Agent 配置 (可选)
        - known_characters: 已知人物列表 (可选)
        - known_settings: 已知设定列表 (可选)
        - known_events: 已知事件列表 (可选)
    """
    content = params.get("content", "")
    chapter_index = params.get("chapter_index", 1)
    chapter_title = params.get("chapter_title")
    book_title = params.get("book_title", "未知书籍")
    agent_type = params.get("agent_type", "technique")
    agent_cfg = params.get("agent_config", {})
    prompt_cards = params.get("prompt_cards", [])

    # Build global prefix/suffix from prompt cards
    system_prompt_prefix, system_prompt_suffix = _build_prompt_prefix_suffix(prompt_cards)

    log_info(f"[analyze_single_agent] {agent_type} | '{book_title}' ch.{chapter_index} | content={len(content)}chars")

    # 构建分析上下文
    context = AnalysisContext(
        chapter_content=content,
        chapter_index=chapter_index,
        chapter_title=chapter_title,
        book_title=book_title,
        previous_summary=params.get("previous_summary"),
        known_characters=params.get("known_characters"),
        known_settings=params.get("known_settings"),
        known_events=params.get("known_events"),
        current_profile=params.get("current_profile"),  # For style analysis incremental refinement
    )

    result = {
        "agent_type": agent_type,
        "success": False,
        "error": None,
        "data": [],
    }

    provider = None
    try:
        # Create provider from agent config
        provider_cfg_data = agent_cfg.get("provider_config", {})

        if provider_cfg_data:
            api_key = provider_cfg_data.get("api_key", "")
            log_info(f"[analyze_single_agent] Config: provider={provider_cfg_data.get('name')}, format={provider_cfg_data.get('api_format')}, model={agent_cfg.get('model')}, api_key={'yes' if api_key else 'no'}")

            provider_cfg = ProviderConfig(
                name=provider_cfg_data.get("name", "default"),
                api_base=provider_cfg_data.get("api_base", "https://api.openai.com/v1"),
                api_key=api_key,
                default_model=provider_cfg_data.get("model", "gpt-4o-mini"),
                api_format=provider_cfg_data.get("api_format", "openai"),
                max_retries=provider_cfg_data.get("max_retries", 3),
            )
            provider = create_provider(provider_cfg)

            agent_config = AgentConfig(
                name=f"{agent_type}_agent",
                kind=agent_type,
                provider_name=provider_cfg.name,
                model=agent_cfg.get("model", provider_cfg.default_model),
                temperature=agent_cfg.get("temperature", 0.7),
                max_tokens=agent_cfg.get("max_tokens"),
                system_prompt=agent_cfg.get("system_prompt"),
                system_prompt_prefix=system_prompt_prefix,
                system_prompt_suffix=system_prompt_suffix,
            )
        else:
            log_error("[analyze_single_agent] No agent config provided")
            result["error"] = "No agent config provided"
            return result

        agent = create_agent(agent_config, provider)
        agent_result = await agent.analyze(context)

        # Extract result based on agent type
        if agent_type == "technique":
            result["data"] = agent_result.get("techniques", [])
        elif agent_type == "character":
            result["data"] = agent_result.get("characters", [])
        elif agent_type == "setting":
            result["data"] = agent_result.get("settings", [])
        elif agent_type == "event":
            result["data"] = agent_result.get("events", [])
        elif agent_type == "style":
            # Style returns the entire result as the style profile
            result["data"] = agent_result

        result["success"] = True
        # Log item count
        if agent_type == "style":
            item_count = len(result['data'].get('key_observations', [])) if isinstance(result['data'], dict) else 0
        else:
            item_count = len(result['data']) if isinstance(result['data'], list) else 0
        log_info(f"[analyze_single_agent] {agent_type} completed | {item_count} items")

    except Exception as e:
        log_exception(f"[analyze_single_agent] Agent {agent_type} FAILED", e)
        result["error"] = str(e)

    finally:
        if provider:
            await provider.close()

    return result


def chunk_text(params: dict[str, Any]) -> dict[str, Any]:
    text = params.get("text", "")
    max_chunk_size = params.get("max_chunk_size", 400)
    min_chunk_size = params.get("min_chunk_size", 50)

    chunker = ChineseChunker(
        max_chunk_size=max_chunk_size,
        min_chunk_size=min_chunk_size,
    )

    chunks = chunker.chunk_text(text)

    return {
        "chunks": [
            {
                "content": chunk.content,
                "char_start": chunk.char_start,
                "char_end": chunk.char_end,
                "index": chunk.index,
            }
            for chunk in chunks
        ]
    }


def generate_embedding(params: dict[str, Any]) -> dict[str, Any]:
    text = params.get("text", "")
    config = params.get("config")

    provider = get_embedding_provider(config)
    embedding = provider.embed(text)

    return {
        "embedding": embedding,
        "dimensions": len(embedding),
        "model": provider.model_name,
    }


def generate_embeddings(params: dict[str, Any]) -> dict[str, Any]:
    texts = params.get("texts", [])
    config = params.get("config")

    log_info(f"[generate_embeddings] Received {len(texts)} texts, config: {config}")

    if not texts:
        return {"embeddings": [], "dimensions": 0, "model": ""}

    try:
        provider = get_embedding_provider(config)
        log_info(f"[generate_embeddings] Using provider: {type(provider).__name__}, model: {provider.model_name}")
        embeddings = provider.embed_batch(texts)
        log_info(f"[generate_embeddings] Generated {len(embeddings)} embeddings")

        return {
            "embeddings": embeddings,
            "dimensions": len(embeddings[0]) if embeddings else 0,
            "model": provider.model_name,
        }
    except Exception as e:
        log_error(f"[generate_embeddings] Error: {e}")
        raise


def fetch_embedding_models(params: dict[str, Any]) -> dict[str, Any]:
    """
    获取可用的 Embedding 模型列表

    params:
        - provider: 服务提供商 (gemini, openai)
        - api_key: API Key
        - proxy_url: 代理地址 (for Gemini)
        - base_url: API Base URL (for OpenAI)
    """
    provider_type = params.get("provider", "local")
    api_key = params.get("api_key", "")
    proxy_url = params.get("proxy_url")
    base_url = params.get("base_url", "https://api.openai.com/v1")

    log_info(f"[fetch_embedding_models] Fetching models for provider: {provider_type}")

    try:
        if provider_type == "gemini":
            return _fetch_gemini_models(api_key, proxy_url)
        elif provider_type == "openai":
            return _fetch_openai_models(api_key, base_url)
        else:
            return {
                "success": False,
                "models": [],
                "message": f"不支持的 provider: {provider_type}",
            }
    except Exception as e:
        log_exception("[fetch_embedding_models] Failed to fetch models", e)
        return {
            "success": False,
            "models": [],
            "message": str(e),
        }


def _fetch_gemini_models(api_key: str, proxy_url: Optional[str]) -> dict[str, Any]:
    """使用 google-genai 库获取 Gemini 模型列表"""
    try:
        from google import genai
    except ImportError:
        return {
            "success": False,
            "models": [],
            "message": "google-genai 未安装。请运行: pip install google-genai",
        }

    log_info(f"[fetch_gemini_models] Creating Gemini client with proxy: {proxy_url}")

    http_options = {"timeout": 60000}
    if proxy_url:
        http_options["client_args"] = {"proxy": proxy_url}

    try:
        client = genai.Client(api_key=api_key, http_options=http_options)

        log_info("[fetch_gemini_models] Listing models...")
        models = []
        for model in client.models.list():
            model_name = model.name if hasattr(model, 'name') else str(model)
            display_name = model.display_name if hasattr(model, 'display_name') else model_name

            # 只筛选 embedding 模型
            if 'embed' in model_name.lower() or 'embed' in display_name.lower():
                model_id = model_name.replace("models/", "")
                models.append({
                    "id": model_id,
                    "name": display_name,
                })
                log_info(f"[fetch_gemini_models] Found embedding model: {model_id}")

        log_info(f"[fetch_gemini_models] Found {len(models)} embedding models")

        if not models:
            return {
                "success": False,
                "models": [],
                "message": "未找到 Embedding 模型",
            }

        return {
            "success": True,
            "models": models,
            "message": "获取成功",
        }

    except Exception as e:
        error_msg = str(e)
        if "proxy" in error_msg.lower() or "connect" in error_msg.lower():
            error_msg = f"连接失败，请检查代理配置: {error_msg}"
        return {
            "success": False,
            "models": [],
            "message": error_msg,
        }


def _fetch_openai_models(api_key: str, base_url: str) -> dict[str, Any]:
    """使用 OpenAI API 获取模型列表"""
    import httpx

    log_info(f"[fetch_openai_models] Fetching models from: {base_url}")

    try:
        url = f"{base_url.rstrip('/')}/models"
        headers = {"Authorization": f"Bearer {api_key}"}

        with httpx.Client(timeout=30) as client:
            response = client.get(url, headers=headers)
            response.raise_for_status()
            data = response.json()

        models = []
        for model in data.get("data", []):
            model_id = model.get("id", "")
            if "embed" in model_id.lower():
                models.append({
                    "id": model_id,
                    "name": model_id,
                })
                log_info(f"[fetch_openai_models] Found embedding model: {model_id}")

        log_info(f"[fetch_openai_models] Found {len(models)} embedding models")

        if not models:
            return {
                "success": False,
                "models": [],
                "message": "未找到 Embedding 模型",
            }

        return {
            "success": True,
            "models": models,
            "message": "获取成功",
        }

    except Exception as e:
        return {
            "success": False,
            "models": [],
            "message": f"连接失败: {str(e)}",
        }


def main() -> int:
    # Initialize logging
    init_logging()
    log_info("Python sidecar main loop starting")

    # Ensure stdin/stdout use UTF-8 encoding (important for Windows)
    import io
    if hasattr(sys.stdin, 'buffer'):
        sys.stdin = io.TextIOWrapper(sys.stdin.buffer, encoding='utf-8')
    if hasattr(sys.stdout, 'buffer'):
        sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8', newline='\n')

    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue

        try:
            request = json.loads(line)
            response = handle_request(request)
            print(json.dumps(response, ensure_ascii=False), flush=True)
        except json.JSONDecodeError as e:
            log_error(f"JSON parse error: {e}, line length: {len(line)}, first 200 chars: {line[:200]}")
            error_response = {
                "jsonrpc": "2.0",
                "error": {"code": -32700, "message": f"Parse error: {e}"},
                "id": None,
            }
            print(json.dumps(error_response), flush=True)

    return 0


if __name__ == "__main__":
    sys.exit(main())
