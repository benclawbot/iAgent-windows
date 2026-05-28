"""Config loader for iAgent.

Reads config.toml from the OS-appropriate per-user config directory via
platformdirs. Validates required fields.
"""

from __future__ import annotations

import shutil
import tomllib
from dataclasses import dataclass
from pathlib import Path

ALLOWED_HOTKEYS = {"ctrl+alt", "right_ctrl"}
ALLOWED_TTS_PROVIDERS = {"elevenlabs", "piper"}
ALLOWED_LOG_LEVELS = {"DEBUG", "INFO", "WARNING", "ERROR"}


class ConfigError(Exception):
    """Raised when the config file cannot be loaded or fails validation."""


@dataclass(frozen=True)
class Config:
    minimax_api_key: str
    worker_url: str | None
    assemblyai_api_key: str | None
    hotkey: str
    tts_provider: str
    eleven_labs_api_key: str | None
    eleven_labs_voice_id: str | None
    log_level: str
    knowledge_dir: Path | None
    allow_foreground_typing: bool
    iagent_path: Path | None

    @classmethod
    def from_path(cls, path: Path) -> Config:
        try:
            raw = path.read_bytes()
        except OSError as exc:
            raise ConfigError(f"cannot read config file at {path}: {exc}") from exc
        try:
            data = tomllib.loads(raw.decode("utf-8"))
        except (tomllib.TOMLDecodeError, UnicodeDecodeError) as exc:
            raise ConfigError(f"cannot parse TOML at {path}: {exc}") from exc

        # MiniMax API key (required)
        minimax_api_key = data.get("minimax_api_key", "").strip()
        if not minimax_api_key:
            raise ConfigError(
                "minimax_api_key is required. Get one at https://platform.minimax.chat"
            )

        worker_url = data.get("worker_url", "").strip() or None
        if worker_url is not None and not (
            worker_url.startswith("http://") or worker_url.startswith("https://")
        ):
            raise ConfigError(
                "worker_url must start with http:// or https:// "
                '(example: "https://your-worker.workers.dev")'
            )
        assemblyai_api_key = data.get("assemblyai_api_key", "").strip() or None

        hotkey = data.get("hotkey", "ctrl+alt")
        if hotkey not in ALLOWED_HOTKEYS:
            raise ConfigError(
                f"hotkey must be one of {sorted(ALLOWED_HOTKEYS)}, got {hotkey!r}"
            )

        tts_provider = data.get("tts_provider", "piper")
        if tts_provider not in ALLOWED_TTS_PROVIDERS:
            raise ConfigError(
                f"tts_provider must be one of {sorted(ALLOWED_TTS_PROVIDERS)}, got {tts_provider!r}"
            )

        eleven_labs_api_key = data.get("eleven_labs_api_key", "").strip() or None
        eleven_labs_voice_id = data.get("eleven_labs_voice_id", "").strip() or None

        log_level = data.get("log_level", "INFO")
        if log_level not in ALLOWED_LOG_LEVELS:
            raise ConfigError(
                f"log_level must be one of {sorted(ALLOWED_LOG_LEVELS)}, got {log_level!r}"
            )

        allow_foreground_typing_raw = data.get("allow_foreground_typing", False)
        if not isinstance(allow_foreground_typing_raw, bool):
            raise ConfigError(
                "allow_foreground_typing must be true or false"
            )
        allow_foreground_typing = allow_foreground_typing_raw

        iagent_path_raw = data.get("iagent_path")
        iagent_path: Path | None = None
        if isinstance(iagent_path_raw, str) and iagent_path_raw.strip():
            candidate = Path(iagent_path_raw.strip()).expanduser()
            if candidate.is_dir():
                exe_candidate = candidate / "iagent.exe"
                bin_candidate = candidate / "iagent"
                if exe_candidate.is_file():
                    candidate = exe_candidate
                elif bin_candidate.is_file():
                    candidate = bin_candidate
            if not candidate.is_file():
                raise ConfigError(
                    "iagent_path must point to an iagent executable file "
                    "(or a directory containing iagent.exe)"
                )
            iagent_path = candidate.resolve()

        knowledge_dir_raw = data.get("knowledge_dir")
        if isinstance(knowledge_dir_raw, str) and knowledge_dir_raw.strip():
            knowledge_dir = Path(knowledge_dir_raw)
        else:
            knowledge_dir = path.parent / "knowledge"

        if not knowledge_dir.is_dir():
            knowledge_dir = None

        return cls(
            minimax_api_key=minimax_api_key,
            worker_url=worker_url,
            assemblyai_api_key=assemblyai_api_key,
            hotkey=hotkey,
            tts_provider=tts_provider,
            eleven_labs_api_key=eleven_labs_api_key,
            eleven_labs_voice_id=eleven_labs_voice_id,
            log_level=log_level,
            knowledge_dir=knowledge_dir,
            allow_foreground_typing=allow_foreground_typing,
            iagent_path=iagent_path,
        )

    @staticmethod
    def ensure_exists(target_path: Path, example_path: Path) -> bool:
        """Copy example to target if target does not exist. Returns True if created."""
        if target_path.exists():
            return False
        target_path.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(example_path, target_path)
        return True
