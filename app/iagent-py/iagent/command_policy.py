"""Command approval policy helpers."""

from __future__ import annotations

import re

_RM_TOKEN_RE = re.compile(r"\brm\b", re.IGNORECASE)
_XPI_TOKEN_RE = re.compile(r"\.xpi(?:\b|$)", re.IGNORECASE)
_GMAIL_HINT_RE = re.compile(
    r"(gmail|mail\.google\.com|compose|draft\s+email|email\s+draft)",
    re.IGNORECASE,
)

GMAIL_COMPOSE_COMMAND = 'start "" "https://mail.google.com/mail/u/0/?view=cm&fs=1&tf=1"'


def requires_manual_approval(command: str) -> bool:
    """Return True when a command must be approved before execution."""
    return bool(_RM_TOKEN_RE.search(command))


def normalize_ai_command(command: str) -> tuple[str | None, str | None]:
    """Normalize or block unsafe AI-generated commands.

    Returns (normalized_command, note). A None command means "block it".
    """
    cleaned = command.strip()
    if not cleaned:
        return None, "empty command"

    if not _XPI_TOKEN_RE.search(cleaned):
        return cleaned, None

    # .xpi is a browser-extension package and should not be launched for
    # "open Gmail and draft" style requests. Route likely Gmail intent to a
    # deterministic compose URL; otherwise block the command.
    if _GMAIL_HINT_RE.search(cleaned):
        return (
            GMAIL_COMPOSE_COMMAND,
            "rewrote suspicious .xpi browser command to Gmail compose URL",
        )
    return None, "blocked suspicious .xpi AI command"
