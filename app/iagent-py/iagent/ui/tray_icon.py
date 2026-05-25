"""System tray icon for iAgent.

Minimal tray: static icon, context menu with Settings / Show History / Quit.
"""

from __future__ import annotations

import logging
from pathlib import Path

from PySide6.QtCore import Signal
from PySide6.QtGui import QAction, QIcon
from PySide6.QtWidgets import QApplication, QMenu, QSystemTrayIcon

from iagent.icon_factory import icon_for_state
from iagent.state import VoiceState

logger = logging.getLogger(__name__)


class TrayIcon(QSystemTrayIcon):
    """Static system tray icon with Settings / Show History / Quit menu."""

    show_history_requested = Signal()
    show_settings_requested = Signal()
    show_prompt_dock_requested = Signal()
    run_command_requested = Signal()
    ambient_start_requested = Signal()
    ambient_stop_requested = Signal()
    voice_start_requested = Signal()
    voice_stop_requested = Signal()
    quit_requested = Signal()

    def __init__(self) -> None:
        super().__init__()
        # Prefer the shipped robot asset for tray branding; keep state icon fallback.
        self.setIcon(_load_robot_tray_icon() or icon_for_state(VoiceState.IDLE))
        self.setToolTip("iAgent")

        menu = QMenu()

        settings_action = QAction("Settings", menu)
        settings_action.triggered.connect(lambda: self.show_settings_requested.emit())
        menu.addAction(settings_action)

        history_action = QAction("Show History", menu)
        history_action.triggered.connect(lambda: self.show_history_requested.emit())
        menu.addAction(history_action)

        prompt_action = QAction("Show Robot Dock", menu)
        prompt_action.triggered.connect(lambda: self.show_prompt_dock_requested.emit())
        menu.addAction(prompt_action)

        run_command_action = QAction("Run Background Command...", menu)
        run_command_action.triggered.connect(lambda: self.run_command_requested.emit())
        menu.addAction(run_command_action)

        start_ambient_action = QAction("Start Ambient Mode", menu)
        start_ambient_action.triggered.connect(
            lambda: self.ambient_start_requested.emit()
        )
        menu.addAction(start_ambient_action)

        stop_ambient_action = QAction("Stop Ambient Mode", menu)
        stop_ambient_action.triggered.connect(lambda: self.ambient_stop_requested.emit())
        menu.addAction(stop_ambient_action)

        self._hold_to_talk_action = QAction("Hold to Talk", menu)
        self._hold_to_talk_action.setCheckable(True)
        self._hold_to_talk_action.toggled.connect(self._on_hold_to_talk_toggled)
        menu.addAction(self._hold_to_talk_action)

        menu.addSeparator()

        quit_action = QAction("Quit", menu)
        quit_action.triggered.connect(self._on_quit)
        menu.addAction(quit_action)

        self.setContextMenu(menu)

    def _on_quit(self) -> None:
        self.quit_requested.emit()
        app = QApplication.instance()
        if app is not None:
            app.quit()

    def notify(self, title: str, message: str, timeout_ms: int = 10000) -> None:
        """Show a non-blocking tray notification."""
        self.showMessage(title, message, QSystemTrayIcon.MessageIcon.Information, timeout_ms)

    def set_hold_to_talk_active(self, active: bool) -> None:
        if self._hold_to_talk_action.isChecked() == active:
            return
        self._hold_to_talk_action.blockSignals(True)
        self._hold_to_talk_action.setChecked(active)
        self._hold_to_talk_action.blockSignals(False)

    def _on_hold_to_talk_toggled(self, active: bool) -> None:
        if active:
            self.voice_start_requested.emit()
        else:
            self.voice_stop_requested.emit()


def _load_robot_tray_icon() -> QIcon | None:
    asset = Path(__file__).resolve().parents[1] / "assets" / "robot.ico"
    if not asset.exists():
        return None
    icon = QIcon(str(asset))
    if icon.isNull():
        logger.warning("tray icon asset exists but could not be loaded: %s", asset)
        return None
    return icon
