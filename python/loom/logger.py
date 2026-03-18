"""
Logging module for narrative-loom Python sidecar.

On Windows, writing to stderr while Rust is reading from the pipe can cause errors.
This module provides a file-based logging solution that writes to the api.log file.

Log file location: Determined by NARRATIVE_LOOM_LOGS_DIR environment variable,
                   or falls back to .narrative-loom/logs/api.log in cwd.
"""

import os
from datetime import datetime
from pathlib import Path
from typing import Optional

_log_file: Optional[Path] = None
_log_enabled: bool = True


def _get_log_path() -> Optional[Path]:
    """Get the path to the API log file.

    Uses NARRATIVE_LOOM_LOGS_DIR environment variable set by Rust sidecar manager.
    Falls back to .narrative-loom/logs/api.log in current working directory.
    """
    # First, try the environment variable set by Rust
    logs_dir_env = os.environ.get("NARRATIVE_LOOM_LOGS_DIR")
    if logs_dir_env:
        log_path = Path(logs_dir_env) / "api.log"
    else:
        # Fallback: current working directory
        cwd = Path.cwd()
        log_path = cwd / ".narrative-loom" / "logs" / "api.log"

    # Ensure directory exists
    log_path.parent.mkdir(parents=True, exist_ok=True)

    return log_path


def init_logging() -> None:
    """Initialize the logging system."""
    global _log_file
    _log_file = _get_log_path()


def log(message: str, level: str = "INFO") -> None:
    """
    Log a message to the api.log file.

    Args:
        message: The message to log
        level: Log level (INFO, WARN, ERROR, DEBUG)
    """
    global _log_file

    if not _log_enabled:
        return

    if _log_file is None:
        init_logging()

    if _log_file is None:
        return

    try:
        timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
        with open(_log_file, "a", encoding="utf-8") as f:
            f.write(f"[{timestamp}] [{level}] [Python] {message}\n")
            f.flush()
    except Exception as e:
        # Print to stderr for debugging logger issues
        import sys
        print(f"[Logger Error] Failed to write log: {e}", file=sys.stderr)


def log_info(message: str) -> None:
    """Log an info message."""
    log(message, "INFO")


def log_warn(message: str) -> None:
    """Log a warning message."""
    log(message, "WARN")


def log_error(message: str) -> None:
    """Log an error message."""
    log(message, "ERROR")


def log_debug(message: str) -> None:
    """Log a debug message."""
    log(message, "DEBUG")


def log_exception(message: str, exc: Exception) -> None:
    """Log an exception with traceback."""
    import traceback
    log(f"{message}: {exc}", "ERROR")
    log(traceback.format_exc(), "ERROR")


def disable_logging() -> None:
    """Disable logging."""
    global _log_enabled
    _log_enabled = False


def enable_logging() -> None:
    """Enable logging."""
    global _log_enabled
    _log_enabled = True
