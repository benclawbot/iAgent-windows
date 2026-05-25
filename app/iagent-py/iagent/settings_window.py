"""
Settings window for the iAgent dock.
Allows user to select provider, model, and enter API key.
"""
import os
import shutil
import subprocess
import sys
from pathlib import Path

from PyQt6.QtCore import Qt
from PyQt6.QtWidgets import (
    QCheckBox,
    QComboBox,
    QDialog,
    QGridLayout,
    QHBoxLayout,
    QLabel,
    QLineEdit,
    QMessageBox,
    QPushButton,
    QTabWidget,
    QVBoxLayout,
    QWidget,
)

PROVIDERS = [
    ("MiniMax", "minimax"),
    ("OpenAI", "openai"),
    ("Anthropic", "anthropic"),
    ("DeepSeek", "deepseek"),
    ("Google Gemini", "gemini"),
    ("OpenRouter", "openrouter"),
]

MODEL_MAP = {
    "minimax": ["MiniMax-Text-01", "MiniMax-Text-01-flash"],
    "openai": ["gpt-4o", "gpt-4o-mini", "gpt-4-turbo", "gpt-3.5-turbo"],
    "anthropic": ["claude-sonnet-4-20250514", "claude-opus-4-20250514"],
    "deepseek": ["deepseek-chat", "deepseek-coder"],
    "gemini": ["gemini-2.5-pro-preview-06-05", "gemini-2.0-flash"],
    "openrouter": ["anthropic/claude-sonnet-4", "openai/gpt-4o"],
}


def get_settings_path():
    if os.name == "nt":
        base = os.environ.get("LOCALAPPDATA", str(Path.home() / "AppData" / "Local"))
        return Path(base) / "iAgent" / "settings.toml"
    return Path.home() / ".config" / "iAgent" / "settings.toml"


class SettingsWindow(QDialog):
    def __init__(self, ipc_client=None, parent=None):
        super().__init__(parent)
        self.ipc_client = ipc_client
        self._settings = self._load_settings()
        self.setWindowTitle("iAgent Settings")
        self.setMinimumWidth(480)
        self.setMinimumHeight(360)
        self._build_ui()
        self._populate()

    def _load_settings(self):
        defaults = {
            "provider": "minimax",
            "model": "MiniMax-Text-01",
            "api_key": "",
            "auto_start": True,
            "start_minimized": False,
            "always_on_top": False,
        }
        p = get_settings_path()
        if p.exists():
            try:
                for line in p.read_text().splitlines():
                    for key in defaults:
                        prefix = key + " = "
                        if line.startswith(prefix):
                            val = line[len(prefix):].strip().strip('"').strip("'")
                            if key in ("provider", "model", "api_key"):
                                defaults[key] = val
                            else:
                                defaults[key] = val.lower() in ("true", "1", "yes")
            except Exception:
                pass
        return defaults

    def _build_ui(self):
        layout = QVBoxLayout(self)
        tabs = QTabWidget()
        layout.addWidget(tabs)

        # Provider tab
        pt = QWidget()
        pl = QGridLayout(pt)
        pl.setColumnStretch(1, 1)

        pl.addWidget(QLabel("Provider:"), 0, 0, Qt.AlignmentFlag.AlignRight)
        self.provider_combo = QComboBox()
        self.provider_combo.currentIndexChanged.connect(self._on_provider_changed)
        pl.addWidget(self.provider_combo, 0, 1)

        pl.addWidget(QLabel("Model:"), 1, 0, Qt.AlignmentFlag.AlignRight)
        self.model_combo = QComboBox()
        pl.addWidget(self.model_combo, 1, 1)

        pl.addWidget(QLabel("API Key:"), 2, 0, Qt.AlignmentFlag.AlignRight)
        self.api_key_input = QLineEdit()
        self.api_key_input.setEchoMode(QLineEdit.EchoMode.Password)
        pl.addWidget(self.api_key_input, 2, 1)

        tabs.addTab(pt, "Provider")

        # General tab
        gt = QWidget()
        gl = QGridLayout(gt)
        gl.setColumnStretch(0, 1)

        self.auto_start_check = QCheckBox("Start backend automatically on launch")
        gl.addWidget(self.auto_start_check, 0, 0)

        self.start_minimized_check = QCheckBox("Start minimized to system tray")
        gl.addWidget(self.start_minimized_check, 1, 0)

        self.always_on_top_check = QCheckBox("Window always on top")
        gl.addWidget(self.always_on_top_check, 2, 0)

        tabs.addTab(gt, "General")

        # Personal tab
        personal_tab = QWidget()
        personal_layout = QGridLayout(personal_tab)
        personal_layout.setColumnStretch(0, 1)

        self.personal_status_label = QLabel("Not checked")
        self.personal_status_label.setTextInteractionFlags(
            Qt.TextInteractionFlag.TextSelectableByMouse
        )
        self.personal_status_label.setWordWrap(True)
        personal_layout.addWidget(self.personal_status_label, 0, 0, 1, 4)

        refresh_personal_btn = QPushButton("Refresh")
        refresh_personal_btn.clicked.connect(self._refresh_personal_status)
        personal_layout.addWidget(refresh_personal_btn, 1, 0)

        tick_personal_btn = QPushButton("Run One Tick")
        tick_personal_btn.clicked.connect(self._run_personal_tick)
        personal_layout.addWidget(tick_personal_btn, 1, 1)

        start_personal_btn = QPushButton("Start Daemon")
        start_personal_btn.clicked.connect(self._start_personal_daemon)
        personal_layout.addWidget(start_personal_btn, 1, 2)

        open_personal_btn = QPushButton("Open Folder")
        open_personal_btn.clicked.connect(self._open_personal_folder)
        personal_layout.addWidget(open_personal_btn, 1, 3)

        tabs.addTab(personal_tab, "Personal")

        # Buttons
        btn_w = QWidget()
        hl = QHBoxLayout(btn_w)
        hl.addStretch()
        save_btn = QPushButton("Save")
        save_btn.clicked.connect(self._save)
        cancel_btn = QPushButton("Cancel")
        cancel_btn.clicked.connect(self.reject)
        hl.addWidget(save_btn)
        hl.addWidget(cancel_btn)
        layout.addWidget(btn_w)

    def _populate(self):
        for label, value in PROVIDERS:
            self.provider_combo.addItem(label, value)
        idx = self.provider_combo.findData(self._settings["provider"])
        if idx >= 0:
            self.provider_combo.setCurrentIndex(idx)
        self._on_provider_changed()
        idx = self.model_combo.findText(self._settings["model"])
        if idx >= 0:
            self.model_combo.setCurrentIndex(idx)
        self.api_key_input.setText(self._settings.get("api_key", ""))
        self.auto_start_check.setChecked(self._settings.get("auto_start", True))
        self.start_minimized_check.setChecked(self._settings.get("start_minimized", False))
        self.always_on_top_check.setChecked(self._settings.get("always_on_top", False))
        self._refresh_personal_status()

    def _on_provider_changed(self):
        provider = self.provider_combo.currentData()
        self.model_combo.clear()
        for m in MODEL_MAP.get(provider, []):
            self.model_combo.addItem(m)

    def _save(self):
        sp = get_settings_path()
        sp.parent.mkdir(parents=True, exist_ok=True)

        provider = self.provider_combo.currentData()
        model = self.model_combo.currentText()
        api_key = self.api_key_input.text()
        auto_start = self.auto_start_check.isChecked()
        start_minimized = self.start_minimized_check.isChecked()
        always_on_top = self.always_on_top_check.isChecked()

        lines = [
            "provider = \"" + provider + "\"",
            "model = \"" + model + "\"",
            "api_key = \"" + api_key + "\"",
            "auto_start = " + ("true" if auto_start else "false"),
            "start_minimized = " + ("true" if start_minimized else "false"),
            "always_on_top = " + ("true" if always_on_top else "false"),
        ]
        sp.write_text("\n".join(lines) + "\n")

        if self.ipc_client:
            try:
                payload = {
                    "type": "save_settings",
                    "id": 1,
                    "settings": {
                        "provider": provider,
                        "model": model,
                        "api_key": api_key,
                        "auto_start": auto_start,
                        "start_minimized": start_minimized,
                        "always_on_top": always_on_top,
                    },
                }
                import json
                msg = "\n" + chr(29) + json.dumps(payload) + chr(30)
                self.ipc_client.send_message(msg, timeout=5)
            except Exception:
                pass

        self.accept()

    def _iagent_command(self):
        for key in ("IAGENT_BIN", "JCODE_BIN", "IAGENT_JCODE_BIN"):
            value = os.environ.get(key)
            if value:
                path = Path(value)
                if path.exists():
                    return str(path)
                return value

        for name in ("iagent", "jcode"):
            resolved = shutil.which(name)
            if resolved:
                return resolved

        if os.name == "nt":
            base = os.environ.get("LOCALAPPDATA", str(Path.home() / "AppData" / "Local"))
            installed = Path(base) / "iAgent" / "app" / "iagent.exe"
            if installed.exists():
                return str(installed)

        return None

    def _run_iagent(self, args, timeout=10):
        command = self._iagent_command()
        if not command:
            raise RuntimeError("iAgent executable not found")

        creationflags = 0
        if os.name == "nt":
            creationflags = getattr(subprocess, "CREATE_NO_WINDOW", 0)

        return subprocess.run(
            [command, *args],
            capture_output=True,
            text=True,
            timeout=timeout,
            creationflags=creationflags,
        )

    def _refresh_personal_status(self):
        try:
            result = self._run_iagent(["personal-daemon", "--status"], timeout=8)
            text = result.stdout.strip() if result.returncode == 0 else result.stderr.strip()
            self.personal_status_label.setText(text or "Personal daemon status unavailable")
        except Exception as exc:
            self.personal_status_label.setText(str(exc))

    def _run_personal_tick(self):
        try:
            result = self._run_iagent(["personal-daemon", "--once", "--headless"], timeout=20)
            if result.returncode != 0:
                raise RuntimeError(result.stderr.strip() or "Personal tick failed")
            QMessageBox.information(self, "Personal", result.stdout.strip() or "Tick complete")
            self._refresh_personal_status()
        except Exception as exc:
            QMessageBox.warning(self, "Personal", str(exc))

    def _start_personal_daemon(self):
        command = self._iagent_command()
        if not command:
            QMessageBox.warning(self, "Personal", "iAgent executable not found")
            return

        creationflags = 0
        if os.name == "nt":
            creationflags = getattr(subprocess, "CREATE_NO_WINDOW", 0)

        try:
            subprocess.Popen(
                [command, "personal-daemon", "--headless"],
                stdin=subprocess.DEVNULL,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                creationflags=creationflags,
            )
            QMessageBox.information(self, "Personal", "Personal daemon started")
            self._refresh_personal_status()
        except Exception as exc:
            QMessageBox.warning(self, "Personal", str(exc))

    def _open_personal_folder(self):
        folder = Path(os.environ.get("JCODE_HOME", str(Path.home() / ".jcode"))) / "personal"
        folder.mkdir(parents=True, exist_ok=True)
        try:
            if os.name == "nt":
                os.startfile(folder)
            elif sys.platform == "darwin":
                subprocess.Popen(["open", str(folder)])
            else:
                subprocess.Popen(["xdg-open", str(folder)])
        except Exception as exc:
            QMessageBox.warning(self, "Personal", str(exc))
