r"""Settings dialog for iAgent - provider/model/API key configuration.

Writes to %LOCALAPPDATA%\iAgent\settings.toml which the Rust backend
reads via iagent-settings crate.
"""

from __future__ import annotations

import logging
import os
import shutil
import subprocess
import sys
from pathlib import Path

from platformdirs import user_config_dir
from PySide6.QtCore import Qt, QTimer
from PySide6.QtWidgets import (
    QComboBox,
    QDialog,
    QFormLayout,
    QHBoxLayout,
    QLabel,
    QLineEdit,
    QMessageBox,
    QPushButton,
    QVBoxLayout,
)

logger = logging.getLogger(__name__)

# Available providers
PROVIDERS = [
    ("MiniMax", "minimax"),
    ("OpenAI", "openai"),
    ("OpenRouter", "openrouter"),
    ("Groq", "groq"),
]

# Default models per provider
DEFAULT_MODELS = {
    "minimax": "MiniMax-M2.7",
    "openai": "gpt-4o",
    "openrouter": "anthropic/claude-sonnet-4",
    "groq": "llama-3.3-70b-versatile",
}


class SettingsWindow(QDialog):
    """Settings dialog - provider dropdown, model input, API key input."""

    def __init__(self, parent=None):
        super().__init__(parent)
        self.setWindowTitle("iAgent Settings")
        self.setModal(True)
        self._server_process: subprocess.Popen | None = None
        self._ipc_client = None
        self._connect_task = None

        layout = QVBoxLayout(self)

        # Provider
        form = QFormLayout()
        self.provider_combo = QComboBox()
        for label, key in PROVIDERS:
            self.provider_combo.addItem(label, key)
        self.provider_combo.currentIndexChanged.connect(self._on_provider_changed)
        form.addRow("Provider:", self.provider_combo)

        # Model
        self.model_input = QLineEdit()
        self.model_input.setPlaceholderText("e.g. MiniMax-M2.7 or gpt-4o")
        form.addRow("Model:", self.model_input)

        # API Key
        self.api_key_input = QLineEdit()
        self.api_key_input.setEchoMode(QLineEdit.EchoMode.Password)
        self.api_key_input.setPlaceholderText("sk-...")
        form.addRow("API Key:", self.api_key_input)

        # API Base (optional)
        self.api_base_input = QLineEdit()
        self.api_base_input.setPlaceholderText("https://api.openai.com/v1 (optional)")
        form.addRow("API Base:", self.api_base_input)

        layout.addLayout(form)

        # Status label
        self.status_label = QLabel("")
        self.status_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        layout.addWidget(self.status_label)

        # Buttons
        btn_row = QHBoxLayout()
        self.save_btn = QPushButton("Save")
        self.save_btn.clicked.connect(self._save)
        cancel_btn = QPushButton("Cancel")
        cancel_btn.clicked.connect(self.reject)
        btn_row.addWidget(self.save_btn)
        btn_row.addWidget(cancel_btn)
        layout.addLayout(btn_row)

        self.resize(450, 200)
        self._load()

    def _config_path(self) -> Path:
        return user_config_dir("iAgent") / "settings.toml"

    def _load(self) -> None:
        """Load current settings from the config file."""
        config_path = self._config_path()
        if config_path.exists():
            try:
                content = config_path.read_text()
                for line in content.split("\n"):
                    if "=" in line:
                        key, val = line.split("=", 1)
                        val = val.strip().strip('"').strip("'")
                        if key == "provider":
                            idx = self.provider_combo.findData(val)
                            if idx >= 0:
                                self.provider_combo.setCurrentIndex(idx)
                        elif key == "model":
                            self.model_input.setText(val)
                        elif key == "api_key":
                            self.api_key_input.setText(val)
                        elif key == "api_base":
                            if val and val != "None":
                                self.api_base_input.setText(val)
            except Exception as e:
                logger.warning("Failed to load settings: %s", e)

    def _on_provider_changed(self) -> None:
        """Auto-fill default model when provider changes."""
        provider_key = self.provider_combo.currentData()
        if self.model_input.text() == "":
            default = DEFAULT_MODELS.get(provider_key, "")
            self.model_input.setText(default)

    def _save(self) -> None:
        """Save settings to config file and optionally restart server."""
        provider = self.provider_combo.currentData()
        model = self.model_input.text().strip()
        api_key = self.api_key_input.text().strip()
        api_base = self.api_base_input.text().strip()

        if not model:
            QMessageBox.warning(self, "Validation Error", "Model is required.")
            return

        config_dir = user_config_dir("iAgent")
        config_dir_path = Path(config_dir)
        config_dir_path.mkdir(parents=True, exist_ok=True)

        config_path = config_dir_path / "settings.toml"
        api_key_path = config_dir_path / "minimax.env"

        # Write settings.toml
        api_base_line = f'api_base = "{api_base}"' if api_base else 'api_base = null'
        config_content = f'''provider = "{provider}"
model = "{model}"
{api_base_line}
'''
        try:
            config_path.write_text(config_content)
        except Exception as e:
            QMessageBox.critical(self, "Save Error", f"Failed to write settings.toml: {e}")
            return

        # Write minimax.env (backend reads OPENAI_API_KEY from this file)
        if api_key:
            try:
                api_key_path.write_text(f"OPENAI_API_KEY={api_key}\n")
            except Exception as e:
                logger.warning("Failed to write API key file: %s", e)

        self.status_label.setText("Settings saved! Restart iAgent to apply.")
        self.status_label.repaint()

        QTimer.singleShot(1500, lambda: self.accept())


def show_settings(parent=None) -> None:
    """Show the settings dialog. Call from Qt main thread."""
    dlg = SettingsWindow(parent)
    dlg.exec()


def ensure_server_running() -> tuple[subprocess.Popen | None, Path]:
    """Ensure iagent serve is running. Returns (process, socket_path)."""
    import hashlib

    # Compute socket path
    stem = "iagent"
    h = hashlib.sha256(stem.encode()).hexdigest()[:16]
    if sys.platform == "win32":
        socket_path = Path(f"\\\\.\\pipe\\iagent-{h}")
    else:
        socket_path = Path("/tmp/iagent.sock")

    # Try to connect
    try:
        if sys.platform == "win32":
            import asyncio

            import nest_asyncio
            nest_asyncio.apply()

            async def try_connect():
                try:
                    reader, writer = await asyncio.wait_for(
                        asyncio.open_unix_connection(str(socket_path)),
                        timeout=1.0,
                    )
                    writer.close()
                    await writer.wait_closed()
                    return True
                except Exception:
                    return False

            if asyncio.run(try_connect()):
                return None, socket_path
        else:
            import socket
            s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            s.settimeout(1)
            try:
                s.connect(str(socket_path))
                s.close()
                return None, socket_path
            except OSError:
                pass
    except Exception:
        pass

    # Not running - start it
    binary = _find_iagent_binary()
    if not binary:
        raise RuntimeError("iagent.exe not found in PATH or local build")

    env = os.environ.copy()
    env["RUST_LOG"] = "warn"

    proc = subprocess.Popen(
        [str(binary), "serve"],
        env=env,
        creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0),
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        stdin=subprocess.DEVNULL,
    )

    # Wait for socket to appear
    import time
    for _ in range(20):
        time.sleep(0.5)
        try:
            if sys.platform == "win32":
                import asyncio
                if asyncio.run(try_connect()):
                    return proc, socket_path
            else:
                s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
                s.settimeout(1)
                try:
                    s.connect(str(socket_path))
                    s.close()
                    return proc, socket_path
                except OSError:
                    pass
        except Exception:
            pass

    raise RuntimeError("Failed to start iagent serve")


def _find_iagent_binary() -> Path | None:
    """Find iagent.exe in local build or PATH."""
    # Local build
    local = Path(__file__).parent.parent.parent / "target" / "debug" / "iagent.exe"
    if local.exists():
        return local

    # PATH
    for name in ("iagent.exe", "iagent"):
        found = shutil.which(name)
        if found:
            return Path(found)

    return None
