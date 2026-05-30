"""Task inbox UI: bottom-right robot badge + detailed task window."""

from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime
from pathlib import Path

from PySide6.QtCore import QEvent, QObject, QSize, Qt, QTimer, Signal
from PySide6.QtGui import (
    QAction,
    QColor,
    QFont,
    QGuiApplication,
    QIcon,
    QPainter,
    QPainterPath,
    QPen,
    QPixmap,
    QRegion,
)
from PySide6.QtWidgets import (
    QApplication,
    QFileDialog,
    QHBoxLayout,
    QLabel,
    QLineEdit,
    QListWidget,
    QListWidgetItem,
    QMenu,
    QProgressBar,
    QPushButton,
    QScrollArea,
    QSizePolicy,
    QSplitter,
    QTabWidget,
    QTextEdit,
    QToolButton,
    QVBoxLayout,
    QWidget,
)

from iagent.design_system import DS
from iagent.state import VoiceState


@dataclass
class TaskRecord:
    task_id: str
    command: str
    status: str
    created_at: datetime
    started_at: datetime | None = None
    finished_at: datetime | None = None
    exit_code: int | None = None
    elapsed_s: float | None = None
    stdout_text: str = ""
    stderr_text: str = ""
    error_text: str = ""
    reviewed: bool = False


class TaskRobotBadge(QWidget):
    """Small floating robot icon with unread-count bubble."""

    clicked = Signal()
    quit_requested = Signal()

    def __init__(self, parent: QWidget | None = None) -> None:
        super().__init__(parent)
        self._unread = 0
        robot_image_path = (
            Path(__file__).resolve().parent.parent / "assets" / "robot.jpg"
        )
        self._robot_pixmap = QPixmap(str(robot_image_path))
        self.setWindowFlags(
            Qt.WindowType.Tool
            | Qt.WindowType.FramelessWindowHint
            | Qt.WindowType.WindowStaysOnTopHint
        )
        self.setAttribute(Qt.WidgetAttribute.WA_ShowWithoutActivating)
        self.setAttribute(Qt.WidgetAttribute.WA_TranslucentBackground)
        self.setAutoFillBackground(False)
        self.setStyleSheet("background: transparent;")
        self.setFixedSize(56, 56)
        self._apply_circular_mask()

        self._move_timer = QTimer(self)
        self._move_timer.setInterval(2000)
        self._move_timer.timeout.connect(self._move_bottom_right)

    def show(self) -> None:
        super().show()
        self._move_bottom_right()
        self._move_timer.start()

    def resizeEvent(self, event) -> None:  # noqa: ANN001
        super().resizeEvent(event)
        self._apply_circular_mask()

    def set_unread_count(self, count: int) -> None:
        self._unread = max(0, count)
        self.update()

    def mousePressEvent(self, event) -> None:  # noqa: ANN001
        if event.button() == Qt.MouseButton.LeftButton:
            self.clicked.emit()
            event.accept()
            return
        if event.button() == Qt.MouseButton.RightButton:
            pos = (
                event.globalPosition().toPoint()
                if hasattr(event, "globalPosition")
                else event.globalPos()
            )
            self._show_context_menu(pos)
            event.accept()
            return
        super().mousePressEvent(event)

    def _show_context_menu(self, global_pos) -> None:  # noqa: ANN001
        menu = QMenu(self)
        close_action = QAction("Close iAgent (and related processes)", menu)
        menu.addAction(close_action)
        selected = menu.exec(global_pos)
        if selected == close_action:
            self.quit_requested.emit()

    def paintEvent(self, event) -> None:  # noqa: ANN001, ARG002
        p = QPainter(self)
        p.setRenderHint(QPainter.RenderHint.Antialiasing)

        if not self._robot_pixmap.isNull():
            p.save()
            clip = QPainterPath()
            clip.addEllipse(2, 2, 52, 52)
            p.setClipPath(clip)
            scaled = self._robot_pixmap.scaled(
                52,
                52,
                Qt.AspectRatioMode.KeepAspectRatioByExpanding,
                Qt.TransformationMode.SmoothTransformation,
            )
            x = 2 - (scaled.width() - 52) // 2
            y = 2 - (scaled.height() - 52) // 2
            p.drawPixmap(x, y, scaled)
            p.restore()
        else:
            # Fallback when robot asset is missing: simple white circular badge.
            p.setPen(Qt.PenStyle.NoPen)
            p.setBrush(QColor(DS.Colors.text_white))
            p.drawEllipse(2, 2, 52, 52)
            p.setBrush(QColor(DS.Colors.accent_blue))
            p.drawRoundedRect(14, 15, 28, 22, 6, 6)
            p.setPen(QPen(QColor(DS.Colors.accent_blue), 2))
            p.drawLine(28, 12, 28, 15)
            p.setBrush(QColor(DS.Colors.accent_blue))
            p.drawEllipse(25, 8, 6, 6)
            p.setPen(Qt.PenStyle.NoPen)
            p.setBrush(QColor(DS.Colors.text_white))
            p.drawEllipse(20, 22, 4, 4)
            p.drawEllipse(32, 22, 4, 4)
            p.drawRoundedRect(21, 29, 14, 3, 1, 1)

        # Bubble count
        if self._unread > 0:
            text = str(self._unread if self._unread < 100 else "99+")
            p.setBrush(QColor("#ef4444"))
            p.drawEllipse(34, 0, 22, 22)
            p.setPen(QColor(DS.Colors.text_white))
            p.setFont(QFont("Segoe UI", 8, QFont.Weight.Bold))
            p.drawText(34, 0, 22, 22, Qt.AlignmentFlag.AlignCenter, text)

        p.end()

    def _move_bottom_right(self) -> None:
        screen = QGuiApplication.primaryScreen()
        if screen is None:
            return
        geom = screen.availableGeometry()
        margin = 20
        x = geom.x() + geom.width() - self.width() - margin
        y = geom.y() + geom.height() - self.height() - margin
        self.move(x, y)

    def _apply_circular_mask(self) -> None:
        self.setMask(QRegion(self.rect(), QRegion.RegionType.Ellipse))


class AssistantPromptDock(QWidget):
    """Bottom-right assistant panel with chat and task-progress tabs."""

    prompt_submitted = Signal(str)
    voice_start_requested = Signal()
    voice_stop_requested = Signal()
    inbox_requested = Signal()

    def __init__(self, parent: QWidget | None = None) -> None:
        super().__init__(parent)
        self.setObjectName("assistantDock")
        self.setWindowFlags(
            Qt.WindowType.Tool
            | Qt.WindowType.FramelessWindowHint
            | Qt.WindowType.WindowStaysOnTopHint
        )
        self.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
        self.setFixedSize(460, 340)
        self._outside_filter_installed = False
        self._suspend_auto_hide = False
        self.setStyleSheet(
            """
            #assistantDock {
                background-color: #e3eef0;
                color: #1c2630;
                border: 1px solid #cfdde0;
                border-radius: 26px;
            }
            QWidget#dockHeader {
                background: #ffffff;
                border-top-left-radius: 26px;
                border-top-right-radius: 26px;
                border-bottom-left-radius: 0px;
                border-bottom-right-radius: 0px;
            }
            QWidget#dockBody {
                background: #cfe0e3;
                border: none;
            }
            QWidget#dockComposer {
                background: #ffffff;
                border-top-left-radius: 0px;
                border-top-right-radius: 0px;
                border-bottom-left-radius: 26px;
                border-bottom-right-radius: 26px;
            }
            QLabel {
                background: transparent;
                border: none;
            }
            QTabWidget::pane {
                border: none;
                background: #cfe0e3;
            }
            QTabBar::tab {
                background: #dfe7ea;
                color: #4f5960;
                border-radius: 12px;
                padding: 6px 12px;
                margin-right: 6px;
                font: 600 9pt 'Segoe UI';
            }
            QTabBar::tab:selected {
                background: #ffffff;
                color: #1c2630;
            }
            QScrollArea {
                border: none;
                background: #cfe0e3;
            }
            QLineEdit {
                background-color: #ffffff;
                color: #1c2630;
                border: 1px solid #d5dde1;
                border-radius: 22px;
                padding: 8px 12px;
                font: 10pt 'Segoe UI';
            }
            QPushButton {
                background-color: #1e6fcd;
                color: #ffffff;
                border: none;
                border-radius: 14px;
                padding: 6px 12px;
                font: 9pt 'Segoe UI';
            }
            QToolButton {
                background-color: #ffffff;
                color: #5f6a70;
                border: 1px solid #d5dde1;
                border-radius: 14px;
                min-width: 28px;
                min-height: 28px;
                max-width: 28px;
                max-height: 28px;
                padding: 0px;
            }
            QToolButton#micButton, QToolButton#attachButton {
                background: transparent;
                color: #6b7680;
                border: none;
                border-radius: 0px;
                min-width: 28px;
                min-height: 28px;
                max-width: 28px;
                max-height: 28px;
                padding: 0px;
            }
            QProgressBar {
                border: 1px solid #dbe4ea;
                border-radius: 4px;
                text-align: right;
                color: #697680;
                background: #f5f8fa;
                height: 10px;
            }
            QProgressBar::chunk {
                border-radius: 4px;
                background-color: #1e6fcd;
            }
            """
        )

        self._title = QLabel("iAgent")
        self._title.setStyleSheet("font: 700 12pt 'Segoe UI'; color: #1c2630;")
        self._busy_frames = ("◔", "◑", "◕", "◐")
        self._busy_index = 0
        self._is_busy = False
        self._status = QLabel("○ idle")
        self._status.setStyleSheet("color: #68747d; font: 8pt 'Segoe UI';")

        self._tabs = QTabWidget()
        self._chat_tab = QWidget()
        self._tasks_tab = QWidget()
        self._tabs.addTab(self._chat_tab, "Chat")
        self._tabs.addTab(self._tasks_tab, "Tasks (0)")

        self._chat_scroll = QScrollArea()
        self._chat_scroll.setWidgetResizable(True)
        self._chat_scroll.setHorizontalScrollBarPolicy(Qt.ScrollBarPolicy.ScrollBarAlwaysOff)
        self._chat_scroll.setStyleSheet("QScrollArea { border: none; background: #cfe0e3; }")
        self._chat_body = QWidget()
        self._chat_body.setObjectName("chatBody")
        self._chat_body.setStyleSheet("QWidget#chatBody { background: #cfe0e3; }")
        self._chat_layout = QVBoxLayout(self._chat_body)
        self._chat_layout.setContentsMargins(0, 0, 0, 0)
        self._chat_layout.setSpacing(10)
        self._chat_layout.addStretch(1)
        self._chat_scroll.setWidget(self._chat_body)

        self._input = QLineEdit()
        self._input.setPlaceholderText("Message...")
        self._input.returnPressed.connect(self._submit)
        self._pending_attachments: list[str] = []
        self._attachment_hint = QLabel("")
        self._attachment_hint.setStyleSheet("color: #5f6a70; font: 8pt 'Segoe UI';")
        self._attachment_hint.hide()

        self._talk_btn = QToolButton()
        self._talk_btn.setObjectName("micButton")
        self._talk_btn.setIcon(_make_mic_icon())
        self._talk_btn.setFixedSize(28, 28)
        self._talk_btn.setIconSize(QSize(16, 16))
        self._talk_btn.setToolTip("Hold to Talk")
        self._talk_btn.pressed.connect(self.voice_start_requested.emit)
        self._talk_btn.released.connect(self.voice_stop_requested.emit)

        self._attach_btn = QToolButton()
        self._attach_btn.setObjectName("attachButton")
        self._attach_btn.setIcon(QIcon())
        self._attach_btn.setText("\uE723")
        self._attach_btn.setFont(QFont("Segoe MDL2 Assets", 11))
        self._attach_btn.setFixedSize(28, 28)
        self._attach_btn.setToolTip("Attach file")
        self._attach_btn.clicked.connect(self._attach_files)

        self._task_rows_layout = QVBoxLayout()
        self._task_rows_layout.setContentsMargins(0, 8, 0, 0)
        self._task_rows_layout.setSpacing(8)
        self._task_rows_layout.addStretch(1)

        chat_layout = QVBoxLayout(self._chat_tab)
        chat_layout.setContentsMargins(0, 8, 0, 0)
        chat_layout.setSpacing(8)
        chat_layout.addWidget(self._chat_scroll, 1)

        tasks_layout = QVBoxLayout(self._tasks_tab)
        tasks_layout.setContentsMargins(0, 8, 0, 0)
        tasks_layout.setSpacing(8)
        tasks_layout.addLayout(self._task_rows_layout, 1)

        header_widget = QWidget()
        header_widget.setObjectName("dockHeader")
        header_layout = QVBoxLayout(header_widget)
        header_layout.setContentsMargins(12, 10, 12, 10)
        header_layout.setSpacing(0)
        title_row = QHBoxLayout()
        title_row.setContentsMargins(0, 0, 0, 0)
        title_row.addWidget(self._title)
        title_row.addStretch(1)
        title_row.addWidget(self._status)
        header_layout.addLayout(title_row)

        body_widget = QWidget()
        body_widget.setObjectName("dockBody")
        body_layout = QVBoxLayout(body_widget)
        body_layout.setContentsMargins(12, 8, 12, 8)
        body_layout.setSpacing(4)
        body_layout.addWidget(self._tabs, 1)

        composer_widget = QWidget()
        composer_widget.setObjectName("dockComposer")
        composer_layout = QVBoxLayout(composer_widget)
        composer_layout.setContentsMargins(12, 8, 12, 10)
        composer_layout.setSpacing(6)
        input_row = QHBoxLayout()
        input_row.setContentsMargins(0, 0, 0, 0)
        input_row.addWidget(self._attach_btn)
        input_row.addWidget(self._input, 1)
        input_row.addWidget(self._talk_btn)
        composer_layout.addWidget(self._attachment_hint)
        composer_layout.addLayout(input_row)

        layout = QVBoxLayout(self)
        layout.setContentsMargins(0, 0, 0, 0)
        layout.setSpacing(0)
        layout.addWidget(header_widget, 0)
        layout.addWidget(body_widget, 1)
        layout.addWidget(composer_widget, 0)

        self._move_timer = QTimer(self)
        self._move_timer.setInterval(2000)
        self._move_timer.timeout.connect(self._move_bottom_right)
        self._busy_timer = QTimer(self)
        self._busy_timer.setInterval(180)
        self._busy_timer.timeout.connect(self._tick_busy)
        self._auto_hide_timer = QTimer(self)
        self._auto_hide_timer.setInterval(250)
        self._auto_hide_timer.timeout.connect(self._check_auto_hide)
        self._add_chat_bubble(
            "assistant",
            "Good morning. I'm ready to assist you today. What would you like to focus on?",
        )

    def show(self) -> None:
        super().show()
        self._move_bottom_right()
        self._move_timer.start()
        self._install_outside_click_filter()
        self.activateWindow()
        self._input.setFocus()
        QTimer.singleShot(250, self._auto_hide_timer.start)

    def hideEvent(self, event) -> None:  # noqa: ANN001
        self._auto_hide_timer.stop()
        self._remove_outside_click_filter()
        super().hideEvent(event)

    def event(self, event) -> bool:  # noqa: ANN001
        if event.type() == QEvent.Type.WindowDeactivate and self.isVisible():
            if not self._suspend_auto_hide:
                self.hide()
        return super().event(event)

    def set_feedback(self, text: str) -> None:
        normalized = text.strip()
        self._add_chat_bubble("assistant", normalized if normalized else "No response.")

    def set_busy(self, busy: bool) -> None:
        self._is_busy = busy
        if busy:
            self._busy_index = 0
            self._status.setStyleSheet("color: #1e6fcd; font: 8pt 'Segoe UI';")
            self._status.setText(f"{self._busy_frames[self._busy_index]} working...")
            if not self._busy_timer.isActive():
                self._busy_timer.start()
        else:
            if self._busy_timer.isActive():
                self._busy_timer.stop()
            self._status.setStyleSheet("color: #68747d; font: 8pt 'Segoe UI';")
            self._status.setText("○ idle")

    def _tick_busy(self) -> None:
        if not self._is_busy:
            return
        self._busy_index = (self._busy_index + 1) % len(self._busy_frames)
        self._status.setText(f"{self._busy_frames[self._busy_index]} working...")

    def _submit(self) -> None:
        text = self._input.text().strip()
        if not text and not self._pending_attachments:
            return
        self._input.clear()
        attachments = list(self._pending_attachments)
        self._pending_attachments.clear()
        self._refresh_attachment_hint()
        user_display = text if text else "Attachment only request"
        if attachments:
            user_display = f"{user_display}\n[attachments: {len(attachments)}]"
        self._add_chat_bubble("user", user_display)
        payload = text
        if attachments:
            payload = (payload + "\n\n" if payload else "") + "Attachments:\n" + "\n".join(
                f"- {path}" for path in attachments
            )
        self.prompt_submitted.emit(payload)

    def _move_bottom_right(self) -> None:
        screen = QGuiApplication.primaryScreen()
        if screen is None:
            return
        geom = screen.availableGeometry()
        margin = 20
        x = geom.x() + geom.width() - self.width() - margin
        y = geom.y() + geom.height() - self.height() - margin - 66
        self.move(x, y)

    def _install_outside_click_filter(self) -> None:
        if self._outside_filter_installed:
            return
        app = QApplication.instance()
        if app is None:
            return
        app.installEventFilter(self)
        self._outside_filter_installed = True

    def _remove_outside_click_filter(self) -> None:
        if not self._outside_filter_installed:
            return
        app = QApplication.instance()
        if app is None:
            return
        app.removeEventFilter(self)
        self._outside_filter_installed = False

    def eventFilter(self, watched: QObject, event: QEvent) -> bool:  # noqa: ANN001
        if self.isVisible() and event.type() == QEvent.Type.MouseButtonPress:
            global_pos = None
            if hasattr(event, "globalPosition"):
                global_pos = event.globalPosition().toPoint()
            elif hasattr(event, "globalPos"):
                global_pos = event.globalPos()
            if global_pos is not None and not self.frameGeometry().contains(global_pos):
                self.hide()
        return super().eventFilter(watched, event)

    def _check_auto_hide(self) -> None:
        if not self.isVisible() or self._suspend_auto_hide:
            return
        active = QApplication.activeWindow()
        if active is self or self.isActiveWindow():
            return
        if active is not None and self.isAncestorOf(active):
            return
        self.hide()

    def set_task_records(self, tasks: list[TaskRecord]) -> None:
        while self._task_rows_layout.count() > 1:
            item = self._task_rows_layout.takeAt(0)
            widget = item.widget()
            if widget is not None:
                widget.deleteLater()
        ordered = sorted(tasks, key=lambda t: t.created_at, reverse=True)
        self._tabs.setTabText(1, f"Tasks ({len(ordered)})")
        if not ordered:
            empty = QLabel("No tasks yet.")
            empty.setStyleSheet("color: #6f7a82; font: 9pt 'Segoe UI';")
            self._task_rows_layout.insertWidget(0, empty)
            return
        for task in ordered[:8]:
            self._task_rows_layout.insertWidget(0, self._task_card(task))

    def _task_card(self, task: TaskRecord) -> QWidget:
        card = QWidget()
        card.setStyleSheet(
            """
            QWidget {
                background: #ffffff;
                border: 1px solid #dbe3e8;
                border-radius: 10px;
            }
            """
        )
        layout = QVBoxLayout(card)
        layout.setContentsMargins(10, 8, 10, 8)
        layout.setSpacing(6)
        top = QHBoxLayout()
        top.setContentsMargins(0, 0, 0, 0)
        title = QLabel(self._task_title(task.command))
        title.setStyleSheet("color: #202a33; font: 600 10pt 'Segoe UI';")
        pct = QLabel(self._task_status_label(task))
        pct.setStyleSheet("color: #6c7880; font: 600 9pt 'Segoe UI';")
        top.addWidget(title, 1)
        top.addWidget(pct)
        bar = QProgressBar()
        bar.setValue(self._task_progress(task))
        if task.status == "failed":
            bar.setStyleSheet(
                "QProgressBar { border: 1px solid #e5d2d2; border-radius: 4px; background: #fff7f7; height: 10px; color: #8d3f3f; }"
                "QProgressBar::chunk { border-radius: 4px; background-color: #d94848; }"
            )
        elif task.status == "completed":
            bar.setStyleSheet(
                "QProgressBar { border: 1px solid #dbe4ea; border-radius: 4px; background: #f5f8fa; height: 10px; color: #3d7760; }"
                "QProgressBar::chunk { border-radius: 4px; background-color: #20a163; }"
            )
        layout.addLayout(top)
        layout.addWidget(bar)
        return card

    def _task_title(self, command: str) -> str:
        command_l = command.lower()
        if "index" in command_l:
            return "Indexing Codebase"
        if "ui" in command_l and ("asset" in command_l or "design" in command_l):
            return "Generating UI Assets"
        if "create_document_from_goal.py" in command_l:
            return "Generating Document"
        if "pip install" in command_l or "npm install" in command_l or "dependency" in command_l:
            return "Download Dependencies"
        trimmed = command.strip()
        return trimmed[:28] + ("..." if len(trimmed) > 28 else "")

    def _task_status_label(self, task: TaskRecord) -> str:
        if task.status == "completed":
            return "Done"
        if task.status == "failed":
            return "Failed"
        return f"{self._task_progress(task)}%"

    def _task_progress(self, task: TaskRecord) -> int:
        if task.status in {"completed", "failed"}:
            return 100
        if task.status == "queued":
            return 12
        if task.status == "running":
            if task.elapsed_s is None:
                return 42
            return max(20, min(90, int(task.elapsed_s * 14)))
        return 0

    def _add_chat_bubble(self, role: str, message: str) -> None:
        bubble_text = self._soft_wrap_long_tokens(message)
        bubble = QLabel()
        bubble.setTextFormat(Qt.TextFormat.PlainText)
        bubble.setWordWrap(True)
        bubble.setTextInteractionFlags(Qt.TextInteractionFlag.TextSelectableByMouse)
        bubble_max_width = max(220, int(self.width() * 0.74))
        bubble.setMaximumWidth(bubble_max_width)
        bubble.setSizePolicy(QSizePolicy.Policy.Maximum, QSizePolicy.Policy.Preferred)
        bubble.setText(bubble_text)
        if role == "user":
            bubble.setStyleSheet(
                "QLabel { background: #1d6dc6; color: #ffffff; border-radius: 12px; border: none; padding: 8px; font: 10pt 'Segoe UI'; }"
            )
            is_user = True
        else:
            bubble.setStyleSheet(
                "QLabel { background: #edf2f4; color: #27323b; border-radius: 12px; border: none; padding: 8px; font: 10pt 'Segoe UI'; }"
            )
            is_user = False
        container = QWidget()
        container_layout = QHBoxLayout(container)
        container_layout.setContentsMargins(0, 0, 0, 0)
        if is_user:
            container_layout.addStretch(1)
            container_layout.addWidget(bubble, 0)
        else:
            container_layout.addWidget(bubble, 0)
            container_layout.addStretch(1)
        self._chat_layout.insertWidget(max(0, self._chat_layout.count() - 1), container)
        QTimer.singleShot(0, lambda c=container: self._scroll_to_message_start(c))
        QTimer.singleShot(50, lambda c=container: self._scroll_to_message_start(c))

    def _scroll_to_message_start(self, container: QWidget) -> None:
        bar = self._chat_scroll.verticalScrollBar()
        target = max(0, container.y() - 4)
        bar.setValue(target)

    def _soft_wrap_long_tokens(self, text: str) -> str:
        pieces = text.split(" ")
        wrapped: list[str] = []
        for piece in pieces:
            if len(piece) <= 38:
                wrapped.append(piece)
                continue
            parts = [piece[i : i + 32] for i in range(0, len(piece), 32)]
            wrapped.append("\u200b".join(parts))
        return " ".join(wrapped)

    def _attach_files(self) -> None:
        self._suspend_auto_hide = True
        try:
            files, _selected = QFileDialog.getOpenFileNames(
                self,
                "Select files to attach",
                "",
                "All Files (*);;Images (*.png *.jpg *.jpeg *.webp *.gif *.bmp)",
            )
        finally:
            self._suspend_auto_hide = False
        if not files:
            return
        existing = set(self._pending_attachments)
        for path in files:
            if path not in existing:
                self._pending_attachments.append(path)
                existing.add(path)
        self._refresh_attachment_hint()

    def _refresh_attachment_hint(self) -> None:
        count = len(self._pending_attachments)
        if count == 0:
            self._attachment_hint.hide()
            self._attachment_hint.setText("")
            return
        first_name = Path(self._pending_attachments[0]).name
        if count == 1:
            text = f"Attached: {first_name}"
        else:
            text = f"Attached: {first_name} (+{count - 1} more)"
        self._attachment_hint.setText(text)
        self._attachment_hint.show()


class TaskInboxWindow(QWidget):
    """Detailed task inbox with list and per-task details."""

    mark_read_requested = Signal(str)
    mark_unread_requested = Signal(str)
    mark_all_read_requested = Signal()
    task_feedback_requested = Signal(str, str)

    def __init__(self, parent: QWidget | None = None) -> None:
        super().__init__(parent)
        self.setWindowTitle("iAgent — Task Inbox")
        self.resize(760, 500)
        self.setMinimumSize(620, 380)

        self._tasks: dict[str, TaskRecord] = {}
        self._order: list[str] = []

        self._list = QListWidget()
        self._list.setMinimumWidth(270)
        self._list.currentItemChanged.connect(self._on_current_changed)
        self._list.setStyleSheet(
            f"""
            QListWidget {{
                background-color: {DS.Colors.panel_bg};
                color: {DS.Colors.text_primary};
                border: 1px solid {DS.Colors.border};
                font: 11pt 'Segoe UI';
            }}
            QListWidget::item:selected {{
                background-color: {DS.Colors.info_bg};
                color: {DS.Colors.text_white};
            }}
            """
        )

        self._details = QTextEdit()
        self._details.setReadOnly(True)
        self._details.setStyleSheet(
            f"""
            QTextEdit {{
                background-color: {DS.Colors.panel_bg};
                color: {DS.Colors.text_primary};
                border: 1px solid {DS.Colors.border};
                font: 11pt 'Consolas';
            }}
            """
        )

        self._unread_label = QLabel("Unread: 0")
        self._unread_label.setStyleSheet(
            f"color: {DS.Colors.text_secondary}; font: 10pt 'Segoe UI';"
        )

        self._mark_read_btn = QPushButton("Mark Selected Read")
        self._mark_unread_btn = QPushButton("Mark Selected Unread")
        self._mark_all_read_btn = QPushButton("Mark All Read")
        self._feedback_input = QLineEdit()
        self._feedback_input.setPlaceholderText("Give feedback to improve selected task...")
        self._feedback_send_btn = QPushButton("Send Feedback")
        for btn in (self._mark_read_btn, self._mark_unread_btn, self._mark_all_read_btn):
            btn.setStyleSheet(
                f"""
                QPushButton {{
                    background-color: {DS.Colors.surface};
                    color: {DS.Colors.text_primary};
                    border: 1px solid {DS.Colors.border};
                    padding: 6px 10px;
                    font: 9pt 'Segoe UI';
                }}
                QPushButton:hover {{
                    border-color: {DS.Colors.accent_blue};
                }}
                """
            )

        self._mark_read_btn.clicked.connect(self._emit_mark_selected_read)
        self._mark_unread_btn.clicked.connect(self._emit_mark_selected_unread)
        self._mark_all_read_btn.clicked.connect(lambda: self.mark_all_read_requested.emit())
        self._feedback_send_btn.clicked.connect(self._emit_task_feedback)
        self._feedback_input.returnPressed.connect(self._emit_task_feedback)

        toolbar = QHBoxLayout()
        toolbar.setContentsMargins(0, 0, 0, 0)
        toolbar.addWidget(self._unread_label)
        toolbar.addStretch(1)
        toolbar.addWidget(self._mark_read_btn)
        toolbar.addWidget(self._mark_unread_btn)
        toolbar.addWidget(self._mark_all_read_btn)

        splitter = QSplitter(Qt.Orientation.Horizontal)
        splitter.addWidget(self._list)
        splitter.addWidget(self._details)
        splitter.setSizes([280, 480])

        layout = QVBoxLayout(self)
        layout.setContentsMargins(10, 10, 10, 10)
        layout.addLayout(toolbar)
        layout.addWidget(splitter)
        feedback_row = QHBoxLayout()
        feedback_row.setContentsMargins(0, 6, 0, 0)
        feedback_row.addWidget(self._feedback_input, 1)
        feedback_row.addWidget(self._feedback_send_btn)
        layout.addLayout(feedback_row)

    def upsert(self, task: TaskRecord) -> None:
        is_new = task.task_id not in self._tasks
        self._tasks[task.task_id] = task
        if is_new:
            self._order.insert(0, task.task_id)
        self._refresh_list()

    def show_task(self, task_id: str) -> None:
        for i in range(self._list.count()):
            item = self._list.item(i)
            if item.data(Qt.ItemDataRole.UserRole) == task_id:
                self._list.setCurrentItem(item)
                return

    def _refresh_list(self) -> None:
        selected_id = None
        if self._list.currentItem() is not None:
            selected_id = self._list.currentItem().data(Qt.ItemDataRole.UserRole)

        self._list.clear()
        for task_id in self._order:
            task = self._tasks[task_id]
            status = task.status.upper()
            unread_dot = (
                "● " if task.status in {"completed", "failed"} and not task.reviewed else ""
            )
            label = f"{unread_dot}[{status}] {task.command[:56]}"
            item = QListWidgetItem(label)
            item.setData(Qt.ItemDataRole.UserRole, task_id)
            item.setToolTip(task.command)
            self._list.addItem(item)

        if self._list.count() == 0:
            self._details.setPlainText("No tasks yet.")
            return

        if selected_id is not None:
            self.show_task(selected_id)
        if self._list.currentItem() is None:
            self._list.setCurrentRow(0)
        self._refresh_unread_label()

    def _on_current_changed(
        self,
        current: QListWidgetItem | None,
        _previous: QListWidgetItem | None,
    ) -> None:
        if current is None:
            self._details.clear()
            return
        task_id = current.data(Qt.ItemDataRole.UserRole)
        task = self._tasks.get(task_id)
        if task is None:
            self._details.clear()
            return
        self._details.setPlainText(_format_task_details(task))
        if task.status in {"queued", "running"} or (
            task.status in {"completed", "failed"} and not task.reviewed
        ):
            self.mark_read_requested.emit(task_id)

    def _emit_mark_selected_read(self) -> None:
        item = self._list.currentItem()
        if item is None:
            return
        task_id = item.data(Qt.ItemDataRole.UserRole)
        self.mark_read_requested.emit(task_id)

    def _emit_mark_selected_unread(self) -> None:
        item = self._list.currentItem()
        if item is None:
            return
        task_id = item.data(Qt.ItemDataRole.UserRole)
        self.mark_unread_requested.emit(task_id)

    def _refresh_unread_label(self) -> None:
        unread = 0
        for task in self._tasks.values():
            if task.status in {"queued", "running"} or task.status in {"completed", "failed"} and not task.reviewed:
                unread += 1
        self._unread_label.setText(f"Unread: {unread}")

    def _emit_task_feedback(self) -> None:
        item = self._list.currentItem()
        if item is None:
            return
        feedback = self._feedback_input.text().strip()
        if not feedback:
            return
        task_id = item.data(Qt.ItemDataRole.UserRole)
        self._feedback_input.clear()
        self.task_feedback_requested.emit(task_id, feedback)


class TaskInboxController(QObject):
    """Coordinates task state, unread counts, badge, and inbox rendering."""

    prompt_submitted = Signal(str)
    voice_start_requested = Signal()
    voice_stop_requested = Signal()
    task_feedback_requested = Signal(str, str)
    close_all_requested = Signal()

    def __init__(self, parent: QWidget | None = None) -> None:
        super().__init__(parent)
        self._tasks: dict[str, TaskRecord] = {}
        self._followup_runs: dict[str, tuple[str, str]] = {}
        self._voice_busy = False
        self._inbox = TaskInboxWindow()
        self._badge = TaskRobotBadge()
        self._prompt_dock = AssistantPromptDock()
        self._badge.clicked.connect(self._toggle_prompt_dock)
        self._inbox.mark_read_requested.connect(self.mark_task_read)
        self._inbox.mark_unread_requested.connect(self.mark_task_unread)
        self._inbox.mark_all_read_requested.connect(self.clear_unread)
        self._inbox.task_feedback_requested.connect(self.task_feedback_requested.emit)
        self._prompt_dock.prompt_submitted.connect(self.prompt_submitted.emit)
        self._prompt_dock.voice_start_requested.connect(self.voice_start_requested.emit)
        self._prompt_dock.voice_stop_requested.connect(self.voice_stop_requested.emit)
        self._prompt_dock.inbox_requested.connect(self._open_inbox)
        self._badge.quit_requested.connect(self.close_all_requested.emit)
        self._badge.show()
        self._prompt_dock.hide()
        self._sync_prompt_tasks()

    def shutdown(self) -> None:
        self._badge.hide()
        self._prompt_dock.hide()
        self._inbox.hide()

    def on_assistant_feedback(self, text: str) -> None:
        self._prompt_dock.set_feedback(text)

    def on_assistant_error(self, text: str) -> None:
        self._prompt_dock.set_feedback(f"Error: {text}")

    def on_voice_state_changed(self, state: VoiceState) -> None:
        self._voice_busy = state in {VoiceState.PROCESSING, VoiceState.RESPONDING}
        self._refresh_prompt_busy_state()

    def show_prompt_dock(self) -> None:
        self._prompt_dock.show()
        self._prompt_dock.raise_()
        self._prompt_dock.activateWindow()

    def get_task_record(self, task_id: str) -> TaskRecord | None:
        return self._tasks.get(task_id)

    def bind_followup_run(self, followup_run_id: str, task_id: str, feedback_text: str) -> None:
        self._followup_runs[followup_run_id] = (task_id, feedback_text)

    def on_task_started(self, task_id: str, command: str) -> None:
        now = datetime.now()
        task = self._tasks.get(task_id)
        if task is None:
            task = TaskRecord(
                task_id=task_id,
                command=command,
                status="queued",
                created_at=now,
            )
            self._tasks[task_id] = task
        else:
            task.command = command
            task.status = "queued"
            task.created_at = task.created_at or now
        self._inbox.upsert(task)
        self._update_unread_badge()
        self._sync_prompt_tasks()
        self._refresh_prompt_busy_state()

    def on_task_running(self, task_id: str, command: str) -> None:
        now = datetime.now()
        task = self._tasks.get(task_id)
        if task is None:
            task = TaskRecord(
                task_id=task_id,
                command=command,
                status="running",
                created_at=now,
                started_at=now,
            )
            self._tasks[task_id] = task
        else:
            task.status = "running"
            task.started_at = now
        self._inbox.upsert(task)
        self._update_unread_badge()
        self._sync_prompt_tasks()
        self._refresh_prompt_busy_state()

    def on_task_finished(
        self,
        task_id: str,
        command: str,
        exit_code: int,
        stdout_text: str,
        stderr_text: str,
        elapsed: float,
    ) -> None:
        now = datetime.now()
        task = self._tasks.get(task_id)
        if task is None:
            task = TaskRecord(
                task_id=task_id,
                command=command,
                status="completed",
                created_at=now,
            )
            self._tasks[task_id] = task
        task.status = "completed" if exit_code == 0 else "failed"
        task.command = command
        task.exit_code = exit_code
        task.elapsed_s = elapsed
        task.finished_at = now
        task.stdout_text = stdout_text
        task.stderr_text = stderr_text
        task.reviewed = False

        self._inbox.upsert(task)
        self._inbox.show_task(task_id)
        self._bump_unread()
        self._sync_prompt_tasks()
        self._refresh_prompt_busy_state()

    def on_task_failed(self, task_id: str, command: str, error_text: str) -> None:
        now = datetime.now()
        task = self._tasks.get(task_id)
        if task is None:
            task = TaskRecord(
                task_id=task_id,
                command=command,
                status="failed",
                created_at=now,
            )
            self._tasks[task_id] = task
        task.status = "failed"
        task.command = command
        task.error_text = error_text
        task.finished_at = now
        task.reviewed = False

        self._inbox.upsert(task)
        self._inbox.show_task(task_id)
        self._bump_unread()
        self._sync_prompt_tasks()
        self._refresh_prompt_busy_state()

    def clear_unread(self) -> None:
        for task in self._tasks.values():
            if task.status in {"queued", "running"}:
                task.status = "completed"
                task.finished_at = task.finished_at or datetime.now()
                task.exit_code = 0 if task.exit_code is None else task.exit_code
                if not task.stdout_text and not task.stderr_text and not task.error_text:
                    task.stdout_text = "Marked done from inbox."
            if task.status in {"completed", "failed"}:
                task.reviewed = True
            self._inbox.upsert(task)
        self._update_unread_badge()
        self._sync_prompt_tasks()
        self._refresh_prompt_busy_state()

    def mark_task_read(self, task_id: str) -> None:
        task = self._tasks.get(task_id)
        if task is None:
            return
        if task.status in {"queued", "running"}:
            task.status = "completed"
            task.finished_at = task.finished_at or datetime.now()
            task.exit_code = 0 if task.exit_code is None else task.exit_code
            if not task.stdout_text and not task.stderr_text and not task.error_text:
                task.stdout_text = "Marked done after inbox read."
        task.reviewed = True
        self._inbox.upsert(task)
        self._update_unread_badge()
        self._sync_prompt_tasks()
        self._refresh_prompt_busy_state()

    def mark_task_unread(self, task_id: str) -> None:
        task = self._tasks.get(task_id)
        if task is None:
            return
        if task.status not in {"completed", "failed"}:
            return
        task.reviewed = False
        self._inbox.upsert(task)
        self._update_unread_badge()
        self._sync_prompt_tasks()

    def _bump_unread(self) -> None:
        QApplication.beep()
        self._update_unread_badge()

    def _update_unread_badge(self) -> None:
        unread = 0
        for task in self._tasks.values():
            if task.status in {"queued", "running"} or task.status in {"completed", "failed"} and not task.reviewed:
                unread += 1
        self._badge.set_unread_count(unread)

    def _has_active_tasks(self) -> bool:
        return any(task.status in {"queued", "running"} for task in self._tasks.values())

    def _refresh_prompt_busy_state(self) -> None:
        self._prompt_dock.set_busy(self._voice_busy or self._has_active_tasks())

    def _sync_prompt_tasks(self) -> None:
        self._prompt_dock.set_task_records(list(self._tasks.values()))

    def _open_inbox(self) -> None:
        self._inbox.show()
        self._inbox.raise_()
        self._inbox.activateWindow()

    def _toggle_prompt_dock(self) -> None:
        if self._prompt_dock.isVisible():
            self._prompt_dock.hide()
        else:
            self._prompt_dock.show()
            self._prompt_dock.raise_()


def _format_task_details(task: TaskRecord) -> str:
    def _fmt_dt(value: datetime | None) -> str:
        return value.strftime("%Y-%m-%d %H:%M:%S") if value is not None else "-"

    lines = [
        f"Task ID:      {task.task_id}",
        f"Status:       {task.status}",
        f"Command:      {task.command}",
        f"Queued At:    {_fmt_dt(task.created_at)}",
        f"Started At:   {_fmt_dt(task.started_at)}",
        f"Finished At:  {_fmt_dt(task.finished_at)}",
        f"Exit Code:    {task.exit_code if task.exit_code is not None else '-'}",
        f"Elapsed (s):  {f'{task.elapsed_s:.2f}' if task.elapsed_s is not None else '-'}",
    ]

    if task.error_text:
        lines.extend(["", "Error:", task.error_text])
    if task.stdout_text:
        lines.extend(["", "Stdout:", task.stdout_text])
    if task.stderr_text:
        lines.extend(["", "Stderr:", task.stderr_text])

    return "\n".join(lines)


def _make_mic_icon(size: int = 16) -> QIcon:
    icon = QPixmap(size, size)
    icon.fill(Qt.GlobalColor.transparent)
    p = QPainter(icon)
    p.setRenderHint(QPainter.RenderHint.Antialiasing)
    pen = QPen(QColor("#6b7680"), 1.8)
    pen.setCapStyle(Qt.PenCapStyle.RoundCap)
    pen.setJoinStyle(Qt.PenJoinStyle.RoundJoin)
    p.setPen(pen)
    p.setBrush(Qt.BrushStyle.NoBrush)

    body_w = size * 0.42
    body_h = size * 0.50
    body_x = (size - body_w) / 2
    body_y = size * 0.12
    p.drawRoundedRect(body_x, body_y, body_w, body_h, body_w / 2, body_w / 2)

    cx = size / 2
    stem_top = body_y + body_h
    stem_bottom = size * 0.80
    p.drawLine(int(cx), int(stem_top), int(cx), int(stem_bottom))

    left = size * 0.22
    right = size * 0.78
    base_y = size * 0.86
    p.drawLine(int(left), int(base_y), int(right), int(base_y))
    p.end()
    return QIcon(icon)


def _make_attachment_icon(size: int = 16) -> QIcon:
    icon = QPixmap(size, size)
    icon.fill(Qt.GlobalColor.transparent)
    p = QPainter(icon)
    p.setRenderHint(QPainter.RenderHint.Antialiasing)
    pen = QPen(QColor("#6b7680"), 1.8)
    pen.setCapStyle(Qt.PenCapStyle.RoundCap)
    p.setPen(pen)
    p.setBrush(Qt.BrushStyle.NoBrush)
    p.drawArc(int(size * 0.18), int(size * 0.2), int(size * 0.64), int(size * 0.64), 35 * 16, 290 * 16)
    p.drawArc(int(size * 0.31), int(size * 0.29), int(size * 0.38), int(size * 0.38), 35 * 16, 290 * 16)
    p.end()
    return QIcon(icon)
