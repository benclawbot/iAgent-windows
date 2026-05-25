from __future__ import annotations

from PySide6.QtCore import QObject, Qt, Signal
from PySide6.QtGui import QGuiApplication
from PySide6.QtWidgets import (
    QHBoxLayout,
    QLabel,
    QPushButton,
    QVBoxLayout,
    QWidget,
)

from iagent.proposals import ActionProposal


class ProposalPopup(QWidget):
    """Topmost floating card asking the user to validate or refuse an action."""

    accepted = Signal(object)
    rejected = Signal(object)

    def __init__(self, proposal: ActionProposal, parent: QWidget | None = None) -> None:
        super().__init__(parent)
        self.proposal = proposal
        self.setObjectName("proposalPopup")
        self.setWindowTitle("iAgent Proposal")
        self.setWindowFlags(
            Qt.WindowType.Tool
            | Qt.WindowType.FramelessWindowHint
            | Qt.WindowType.WindowStaysOnTopHint
        )
        self.setAttribute(Qt.WidgetAttribute.WA_ShowWithoutActivating)
        self.setFixedWidth(380)

        title = QLabel(proposal.title)
        title.setObjectName("proposalTitle")
        title.setWordWrap(True)

        body = QLabel(proposal.body)
        body.setObjectName("proposalBody")
        body.setWordWrap(True)
        body.setTextInteractionFlags(Qt.TextInteractionFlag.TextSelectableByMouse)

        validate = QPushButton("Validate")
        validate.setObjectName("proposalValidate")
        validate.clicked.connect(self._accept)

        refuse = QPushButton("Refuse")
        refuse.setObjectName("proposalRefuse")
        refuse.clicked.connect(self._reject)

        buttons = QHBoxLayout()
        buttons.setContentsMargins(0, 4, 0, 0)
        buttons.setSpacing(8)
        buttons.addWidget(refuse)
        buttons.addWidget(validate)

        layout = QVBoxLayout(self)
        layout.setContentsMargins(14, 12, 14, 12)
        layout.setSpacing(8)
        layout.addWidget(title)
        layout.addWidget(body)
        layout.addLayout(buttons)

        self.setStyleSheet(
            """
            QWidget#proposalPopup {
                background: #fbfcfd;
                border: 1px solid #b7c5cf;
                border-radius: 8px;
            }
            QLabel#proposalTitle {
                color: #17212b;
                font: 700 10pt 'Segoe UI';
            }
            QLabel#proposalBody {
                color: #33414d;
                font: 9pt 'Segoe UI';
                padding-top: 2px;
            }
            QPushButton {
                min-height: 28px;
                border-radius: 6px;
                font: 600 9pt 'Segoe UI';
                padding: 4px 12px;
            }
            QPushButton#proposalValidate {
                color: #ffffff;
                background: #176dc2;
                border: 1px solid #176dc2;
            }
            QPushButton#proposalRefuse {
                color: #384550;
                background: #eef3f6;
                border: 1px solid #cbd8df;
            }
            """
        )

    def keyPressEvent(self, event) -> None:  # noqa: ANN001
        if event.key() == Qt.Key.Key_Escape:
            self._reject()
            return
        super().keyPressEvent(event)

    def _accept(self) -> None:
        self.accepted.emit(self.proposal)

    def _reject(self) -> None:
        self.rejected.emit(self.proposal)


class ProposalPopupController(QObject):
    """Owns active proposal popups and emits user decisions."""

    proposal_accepted = Signal(object)
    proposal_rejected = Signal(object)

    def __init__(self, parent: QObject | None = None) -> None:
        super().__init__(parent)
        self._popups: dict[str, ProposalPopup] = {}

    def show_proposal(self, proposal: ActionProposal) -> None:
        if proposal.proposal_id in self._popups:
            self._popups[proposal.proposal_id].raise_()
            return

        popup = ProposalPopup(proposal)
        popup.accepted.connect(lambda p=proposal: self._resolve(p, True))
        popup.rejected.connect(lambda p=proposal: self._resolve(p, False))
        self._popups[proposal.proposal_id] = popup
        popup.show()
        self._reposition()

    def _resolve(self, proposal: ActionProposal, accepted: bool) -> None:
        popup = self._popups.pop(proposal.proposal_id, None)
        if popup is not None:
            popup.hide()
            popup.deleteLater()
        if accepted:
            self.proposal_accepted.emit(proposal)
        else:
            self.proposal_rejected.emit(proposal)
        self._reposition()

    def _reposition(self) -> None:
        screen = QGuiApplication.primaryScreen()
        if screen is None:
            return
        geom = screen.availableGeometry()
        margin = 20
        y = geom.y() + geom.height() - margin
        for popup in reversed(list(self._popups.values())):
            popup.adjustSize()
            y -= popup.height()
            x = geom.x() + geom.width() - popup.width() - margin
            popup.move(x, y)
            y -= 10
