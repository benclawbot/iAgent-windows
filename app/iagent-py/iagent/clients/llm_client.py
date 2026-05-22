"""MiniMax SSE stream parser and streaming HTTP client.

This module provides two layers:

1. A pure parser — ``parse_minimax_sse_stream`` — that decodes MiniMax
   streaming API SSE bytes into text-delta strings. Zero Qt / async / network
   dependencies; unit-tested in isolation.
2. ``LLMClient`` — a ``QObject`` that POSTs to the MiniMax API directly
   using ``httpx.AsyncClient`` with streaming enabled, accumulates
   SSE chunks, feeds complete event blocks through the parser, and emits
   ``delta`` / ``done`` / ``error`` Qt signals.
"""

from __future__ import annotations

import asyncio
import json
import logging
from typing import Iterator

import httpx
from PySide6.QtCore import QObject, Signal

logger = logging.getLogger(__name__)


def parse_minimax_sse_stream(raw: bytes) -> Iterator[str]:
    """Parse a MiniMax SSE byte stream and yield text delta strings.

    MiniMax streaming returns SSE events where each data line is a JSON
    object with a ``choices`` array containing ``delta`` objects with ``content``.
    Format: ``data: {"choices": [{"delta": {"content": "..."}}]}\n\n``

    Events ``heartbeat``, ``error``, ``interaction_block``, and ``message_stop``
    are ignored.

    Args:
        raw: Raw SSE bytes from the MiniMax streaming API.

    Yields:
        Text fragments extracted from ``delta.content`` strings.
    """
    if not raw:
        return

    text = raw.decode("utf-8", errors="replace")

    # Split on blank lines to get individual SSE events.
    chunks = text.split("\n\n")

    for chunk in chunks:
        chunk = chunk.strip()
        if not chunk:
            continue

        # MiniMax SSE format: "data: {...}\n\n"
        # Each chunk may contain multiple "data:" lines.
        for line in chunk.splitlines():
            line = line.strip()
            if not line.startswith("data:"):
                continue

            data_str = line[len("data:"):].strip()
            if not data_str:
                continue

            try:
                payload = json.loads(data_str)
            except json.JSONDecodeError:
                continue

            # Skip non-choice events (heartbeat, error, message_stop, etc.)
            if "choices" not in payload:
                continue

            choices = payload.get("choices", [])
            for choice in choices:
                delta = choice.get("delta", {})
                content = delta.get("content")
                if content:
                    yield content


def parse_anthropic_sse_stream(raw: bytes) -> Iterator[str]:
    """Parse Anthropic-style SSE bytes and yield text delta fragments."""
    if not raw:
        return

    text = raw.decode("utf-8", errors="replace")
    chunks = text.split("\n\n")

    for chunk in chunks:
        chunk = chunk.strip()
        if not chunk:
            continue
        for line in chunk.splitlines():
            line = line.strip()
            if not line.startswith("data:"):
                continue

            payload_str = line[len("data:"):].strip()
            if not payload_str:
                continue

            try:
                payload = json.loads(payload_str)
            except json.JSONDecodeError:
                continue

            if payload.get("type") != "content_block_delta":
                continue
            delta = payload.get("delta", {})
            if delta.get("type") != "text_delta":
                continue
            fragment = delta.get("text")
            if fragment:
                yield fragment


class LLMClient(QObject):
    """Streaming HTTP client for the MiniMax Messages API.

    Emits Qt signals as text deltas arrive so the UI can update in real-time.
    Direct API call — no worker proxy needed.

    Signals:
        delta(str): Emitted for each text fragment as it streams in.
        done(str):  Emitted once with the full accumulated response text.
        error(str): Emitted when any exception occurs during the request.
    """

    delta = Signal(str)
    done = Signal(str)
    error = Signal(str)

    def __init__(self, api_key: str, *, parent: QObject | None = None) -> None:
        super().__init__(parent)
        self._api_key = api_key
        # Use MiniMax OpenAI-compatible endpoint. The minimaxi.com web host
        # returns redirect/HTML responses for API paths.
        self._base_url = "https://api.minimax.io"

    # ------------------------------------------------------------------
    # Public async API
    # ------------------------------------------------------------------

    async def send(
        self,
        messages: list[dict],
        system: str,
        model: str = "MiniMax-M2.7",
        max_tokens: int = 1024,
    ) -> str:
        """POST a streaming completion and return the full response text.

        Args:
            messages: MiniMax ``messages`` array (user/assistant role messages).
            system:    System prompt string (injected as a system message).
            model:     Model identifier (default: ``MiniMax-M2.7``).
            max_tokens: Maximum tokens to generate.

        Returns:
            The fully accumulated response text.

        Raises:
            httpx.HTTPStatusError: If the API returns a non-2xx status.
            asyncio.CancelledError: If the caller cancels the task.
        """
        url = f"{self._base_url}/v1/chat/completions"

        # Inject system prompt as first message
        all_messages = [{"role": "system", "content": system}]
        all_messages.extend(messages)

        body = {
            "model": model,
            "max_tokens": max_tokens,
            "stream": True,
            "reasoning_split": True,
            "messages": all_messages,
        }

        full_text = ""
        buf = b""

        try:
            async with httpx.AsyncClient(timeout=120.0, follow_redirects=True) as client:
                async with client.stream(
                    "POST",
                    url,
                    json=body,
                    headers={
                        "Authorization": f"Bearer {self._api_key}",
                        "Content-Type": "application/json",
                        "Accept": "text/event-stream",
                    },
                ) as response:
                    response.raise_for_status()

                    async for chunk in response.aiter_bytes():
                        buf += chunk

                        # Split on double-newline SSE boundaries. Keep any
                        # incomplete trailing fragment in *buf* for the next
                        # iteration.
                        while b"\n\n" in buf:
                            event_block, buf = buf.split(b"\n\n", 1)
                            for text_fragment in parse_minimax_sse_stream(
                                event_block + b"\n\n"
                            ):
                                self.delta.emit(text_fragment)
                                full_text += text_fragment

            # Flush any remaining bytes in the buffer (final event may lack a
            # trailing blank line).
            if buf.strip():
                for text_fragment in parse_minimax_sse_stream(buf):
                    self.delta.emit(text_fragment)
                    full_text += text_fragment

            self.done.emit(full_text)
            return full_text

        except asyncio.CancelledError:
            logger.debug("LLMClient.send() cancelled")
            raise

        except Exception as exc:
            self.error.emit(str(exc))
            raise
