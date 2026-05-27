"""System tray integration for iAgent desktop dock."""
import os
import sys

from PyQt5.QtGui import QColor, QIcon, QPixmap
from PyQt5.QtWidgets import QAction, QMenu, QSystemTrayIcon


def get_icon_path():
    return os.path.join(os.path.dirname(__file__), "assets", "icon.png")


def create_tray(window, show_settings=None, quit_callback=None):
    tray = QSystemTrayIcon(window)

    icon_path = get_icon_path()
    if os.path.exists(icon_path):
        tray.setIcon(QIcon(icon_path))
    else:
        # Fallback: simple green dot
        pixmap = QPixmap(32, 32)
        pixmap.fill(QColor(0, 200, 83))
        tray.setIcon(QIcon(pixmap))

    tray.setToolTip("iAgent Desktop")

    menu = QMenu()

    status = QAction("iAgent - Running")
    status.setEnabled(False)
    menu.addAction(status)
    menu.addSeparator()

    if show_settings:
        settings_action = QAction("Settings...", tray)
        settings_action.triggered.connect(show_settings)
        menu.addAction(settings_action)

    menu.addSeparator()
    quit_action = QAction("Quit", tray)
    if quit_callback:
        quit_action.triggered.connect(quit_callback)
    else:
        quit_action.triggered.connect(lambda: sys.exit(0))
    menu.addAction(quit_action)

    tray.setContextMenu(menu)
    tray.show()
    return tray
