"""
Persistent IPC client for connecting the Python dock to the Rust iAgent backend.

Replaces subprocess-per-request (iagent run --json) with a single persistent
TCP connection to the running iagent server on port 7643.

Usage:
    client = PersistentIPCClient()
    await client.connect()
    task_id = await client.send_message("hello world")
    async for event in client.events(task_id):
        ...
    await client.disconnect()
"""
from __future__ import annotations

import asyncio
import contextlib
import json
import logging
import os
import time
from collections.abc import AsyncIterator
from dataclasses import dataclass, field
from enum import Enum
from pathlib import Path
from typing import Any

logger = logging.getLogger(__name__)


# ── Path helpers ──────────────────────────────────────────────────────────────

def get_runtime_dir() -> Path:
    if os.name == "nt":
        base = os.environ.get("LOCALAPPDATA", str(Path.home() / "AppData" / "Local"))
        return Path(base) / "iAgent"
    return Path.home() / ".config" / "iAgent"


def get_socket_path() -> Path:
    return get_runtime_dir() / "iagent.sock"


# ── Event enums (both new and legacy names) ────────────────────────────────────

class EventType(Enum):
    CONNECTED = "connected"
    TEXT_DELTA = "text_delta"
    TOOL_START = "tool_start"
    TOOL_DONE = "tool_done"
    ERROR = "error"
    DONE = "done"


# ── Legacy event dataclasses (used by test_ipc_full.py) ─────────────────────────
# These map to the new StreamEvent system.

@dataclass
class AgentStreamEvent:
    """Legacy: agent_stream event with .content field."""
    content: str
    is_system: bool = False

@dataclass
class AgentCompletedEvent:
    """Legacy: agent_completed event with .run_id, .message_id, .agent_output."""
    run_id: str
    message_id: str = ""
    agent_output: str = ""

@dataclass
class ErrorEvent:
    """Legacy: error event with .error field."""
    error: str
    fatal: bool = False

@dataclass
class ConnectedEvent:
    """Legacy: connected event with .server_version."""
    server_version: str = ""

@dataclass
class MessageRequest:
    """Legacy: request wrapper for sending messages."""
    content: str
    context_id: str | None = None
    images: list[str] | None = None


# ── New-style stream events ─────────────────────────────────────────────────────

@dataclass
class StreamEvent:
    event_type: EventType
    content: str = ""
    metadata: dict = field(default_factory=dict)


@dataclass
class CompletionResult:
    run_id: str
    session_id: str
    final_text: str
    usage: dict


# ── parse_server_event: maps JSON to legacy dataclasses ────────────────────────
# Used by test_ipc_full.py protocol parsing tests.

def parse_server_event(json_str: str) -> (
    AgentStreamEvent | AgentCompletedEvent | ErrorEvent | ConnectedEvent | StreamEvent
):
    """Parse a raw JSON server message into the appropriate event type."""
    try:
        msg = json.loads(json_str)
    except json.JSONDecodeError:
        return StreamEvent(event_type=EventType.ERROR, content="malformed JSON")

    msg_type = msg.get("type", "")

    if msg_type == "agent_stream":
        return AgentStreamEvent(
            content=str(msg.get("content", "")),
            is_system=bool(msg.get("is_system", False)),
        )
    elif msg_type in ("agent_completed", "done"):
        return AgentCompletedEvent(
            run_id=str(msg.get("run_id", "")),
            message_id=str(msg.get("message_id", "")),
            agent_output=str(msg.get("agent_output") or msg.get("content") or ""),
        )
    elif msg_type == "error":
        return ErrorEvent(
            error=str(msg.get("error", "unknown")),
            fatal=bool(msg.get("fatal", False)),
        )
    elif msg_type == "connected":
        return ConnectedEvent(server_version=str(msg.get("version", "")))
    elif msg_type == "text_delta":
        return AgentStreamEvent(content=str(msg.get("content", "")), is_system=False)
    else:
        return StreamEvent(
            event_type=EventType.ERROR,
            content=f"unknown message type: {msg_type!r}",
            metadata=msg,
        )


# ── Persistent IPC client ─────────────────────────────────────────────────────

class PersistentIPCClient:
    """
    A single persistent connection to the iagent server.

    Replaces subprocess-per-request with a reusable async connection over TCP
    (port 7643) — the same transport the Rust server exposes for its socket listener.
    """

    def __init__(
        self,
        host: str = "127.0.0.1",
        port: int = 7643,
        timeout: float = 30.0,
    ):
        self.host = host
        self.port = port
        self.timeout = timeout

        self._reader: asyncio.StreamReader | None = None
        self._writer: asyncio.StreamWriter | None = None
        self._connected = False
        self._session_id: str | None = None

        # Per-task state
        self._pending: dict[str, asyncio.Future[dict]] = {}
        self._event_queues: dict[str, asyncio.Queue[Any]] = {}

        # Background reader task
        self._read_task: asyncio.Task | None = None
        self._send_lock = asyncio.Lock()

    # ── Connection ──────────────────────────────────────────────────────────────

    async def connect(self, timeout: float | None = None) -> bool:
        """Establish a persistent connection to the iagent server."""
        if self._connected:
            return True

        timeout = timeout or self.timeout

        try:
            self._reader, self._writer = await asyncio.wait_for(
                asyncio.open_connection(self.host, self.port),
                timeout=timeout,
            )
            self._connected = True
            logger.info("PersistentIPCClient connected to %s:%s", self.host, self.port)
            self._read_task = asyncio.create_task(self._read_loop())
            return True

        except TimeoutError:
            logger.warning("PersistentIPCClient connect timeout")
            return False
        except Exception as exc:
            logger.warning("PersistentIPCClient connect failed: %s", exc)
            return False

    async def disconnect(self) -> None:
        """Close the connection and cancel the background reader."""
        self._connected = False

        if self._read_task:
            self._read_task.cancel()
            with contextlib.suppress(asyncio.CancelledError):
                await self._read_task
            self._read_task = None

        if self._writer:
            try:
                self._writer.close()
                await self._writer.wait_closed()
            except Exception:
                pass
            self._writer = None
            self._reader = None

        for fut in self._pending.values():
            if not fut.done():
                fut.set_exception(ConnectionError("disconnected"))
        self._pending.clear()
        self._event_queues.clear()

        logger.info("PersistentIPCClient disconnected")

    def is_connected(self) -> bool:
        return self._connected

    @property
    def socket_path(self) -> str:
        """Alias for compatibility with old API."""
        return f"{self.host}:{self.port}"

    # ── Send messages ──────────────────────────────────────────────────────────

    async def send_message(
        self,
        message: str,
        context_id: str | None = None,
        images: list[str] | None = None,
    ) -> str:
        """
        Send a message to the server and return a task_id.

        Use events(task_id) to stream the response.
        """
        if not self._connected:
            raise ConnectionError("not connected")

        task_id = f"py-{int(time.time() * 1000)}-{len(message) % 1000:03d}"

        request: dict[str, object] = {
            "type": "message",
            "task_id": task_id,
            "content": message,
            "context_id": context_id or task_id,
        }
        if images:
            request["images"] = images

        self._event_queues[task_id] = asyncio.Queue()
        self._pending[task_id] = asyncio.get_running_loop().create_future()

        payload = json.dumps(request) + "\n"

        async with self._send_lock:
            writer = self._writer
            if writer and not writer.is_closing():
                writer.write(payload.encode())
                await writer.drain()

        return task_id

    async def cancel(self, task_id: str) -> bool:
        """Send a cancel request for an in-progress task."""
        if not self._connected:
            return False

        request = {"type": "cancel", "task_id": task_id}
        payload = json.dumps(request) + "\n"

        try:
            async with self._send_lock:
                writer = self._writer
                if writer and not writer.is_closing():
                    writer.write(payload.encode())
                    await writer.drain()
            return True
        except Exception as exc:
            logger.warning("cancel failed: %s", exc)
            return False

    # ── Legacy API: parameterless events() iterator ────────────────────────────
    # test_ipc_full.py calls client.events() with no argument, yielding ALL events.

    async def events(self) -> AsyncIterator[Any]:
        """
        Yield ALL server events as legacy dataclass instances (no task_id filter).
        Used by test_ipc_full.py. For new code, use events(task_id).
        """
        queue: asyncio.Queue[Any] = asyncio.Queue()
        self._event_queues["_global"] = queue

        try:
            while self._connected:
                event = await queue.get()
                # Convert StreamEvent → legacy types
                yield _stream_to_legacy(event)
                if isinstance(event, StreamEvent) and event.event_type in (
                    EventType.DONE, EventType.ERROR
                ):
                    break
        finally:
            self._event_queues.pop("_global", None)

    # ── Per-task events ─────────────────────────────────────────────────────────

    def events_for_task(self, task_id: str) -> AsyncIterator[Any]:
        """Yield streaming events for a specific task_id. Closes on DONE or ERROR."""
        return self._events_generator(task_id)

    async def _events_generator(self, task_id: str) -> AsyncIterator[Any]:
        """Async generator that reads from the per-task event queue."""
        queue = self._event_queues.get(task_id)
        if queue is None:
            return

        while True:
            event = await queue.get()
            yield _stream_to_legacy(event)
            if isinstance(event, StreamEvent) and event.event_type in (
                EventType.DONE, EventType.ERROR
            ):
                break

    # ── Background read loop ────────────────────────────────────────────────────

    async def _read_loop(self) -> None:
        """Background task: read newline-delimited JSON from the server."""
        buffer = ""
        try:
            while self._connected and self._reader:
                try:
                    data = await asyncio.wait_for(
                        self._reader.read(4096),
                        timeout=30.0,
                    )
                except TimeoutError:
                    continue

                if not data:
                    logger.info("Server closed connection")
                    break

                buffer += data.decode("utf-8", errors="replace")

                while "\n" in buffer:
                    line, buffer = buffer.split("\n", 1)
                    if not line.strip():
                        continue
                    try:
                        msg = json.loads(line)
                    except json.JSONDecodeError:
                        logger.warning("malformed JSON from server: %s", line[:100])
                        continue

                    self._dispatch(msg)

        except asyncio.CancelledError:
            pass
        except Exception as exc:
            logger.warning("read loop error: %s", exc)
        finally:
            self._connected = False

    def _dispatch(self, msg: dict) -> None:
        """Route a server message to the appropriate handler."""
        msg_type = str(msg.get("type", ""))
        task_id = str(msg.get("task_id") or msg.get("context_id") or msg.get("run_id", ""))

        if msg_type == "connected":
            self._session_id = str(msg.get("session_id", ""))
            event = StreamEvent(
                event_type=EventType.CONNECTED,
                metadata={"session_id": self._session_id, "version": str(msg.get("version", ""))},
            )
            self._push_to_all(event)

        elif msg_type in ("agent_stream", "text_delta"):
            content = str(msg.get("content", ""))
            event = StreamEvent(event_type=EventType.TEXT_DELTA, content=content, metadata=msg)
            self._push_event(task_id, event)

        elif msg_type == "tool_start":
            event = StreamEvent(
                event_type=EventType.TOOL_START,
                metadata={"name": str(msg.get("tool", "")), "input": msg.get("input", {})},
            )
            self._push_event(task_id, event)

        elif msg_type == "tool_done":
            event = StreamEvent(
                event_type=EventType.TOOL_DONE,
                metadata={"name": str(msg.get("tool", "")), "output": msg.get("output", {})},
            )
            self._push_event(task_id, event)

        elif msg_type in ("agent_completed", "done"):
            final_text = str(msg.get("text") or msg.get("agent_output") or msg.get("content", ""))

            if task_id in self._pending:
                self._pending[task_id].set_result({
                    "run_id": str(msg.get("run_id", task_id)),
                    "session_id": str(msg.get("session_id", self._session_id or "")),
                    "text": final_text,
                    "usage": msg.get("usage", {}),
                })

            event = StreamEvent(event_type=EventType.DONE, content=final_text, metadata=msg)
            self._push_event(task_id, event)
            # legacy completion event is produced by _stream_to_legacy() in the async generator
            self._cleanup_task(task_id)

        elif msg_type == "error":
            error_msg = str(msg.get("error", "unknown error"))
            event = StreamEvent(event_type=EventType.ERROR, content=error_msg, metadata=msg)

            if task_id in self._pending:
                self._pending[task_id].set_exception(RuntimeError(error_msg))

            self._push_event(task_id, event)
            self._cleanup_task(task_id)

        elif msg_type == "session_id":
            self._session_id = str(msg.get("session_id", ""))
            event = StreamEvent(
                event_type=EventType.CONNECTED,
                metadata={"session_id": self._session_id},
            )
            self._push_to_all(event)

        elif msg_type == "cancel_ack":
            event = StreamEvent(
                event_type=EventType.ERROR,
                content=f"Cancelled: {msg.get('reason', 'user cancelled')}",
                metadata=msg,
            )
            self._push_event(task_id, event)
            self._cleanup_task(task_id)

    def _push_event(self, task_id: str, event: StreamEvent) -> None:
        if task_id in self._event_queues:
            try:
                self._event_queues[task_id].put_nowait(event)
            except asyncio.QueueFull:
                logger.warning("event queue full for task %s", task_id)

    def _push_to_all(self, event: StreamEvent) -> None:
        for q in self._event_queues.values():
            with contextlib.suppress(asyncio.QueueFull):
                q.put_nowait(event)

    def _cleanup_task(self, task_id: str) -> None:
        self._event_queues.pop(task_id, None)
        self._pending.pop(task_id, None)

    # ── Convenience helpers ────────────────────────────────────────────────────

    async def ping(self) -> bool:
        """Send a ping to verify the server is alive."""
        if not self._connected:
            return False
        try:
            writer = self._writer
            if writer and not writer.is_closing():
                writer.write(b'{"type":"ping"}\n')
                await writer.drain()
                return True
        except Exception:
            pass
        return False

    async def get_state(self) -> dict:
        """Return basic server state (session_id, is_processing)."""
        return {
            "session_id": self._session_id or "",
            "is_processing": bool(self._pending),
            "connected": self._connected,
            "host": self.host,
            "port": self.port,
        }

    async def get_history(self) -> list[dict]:
        """Return conversation history. Currently returns empty — server-side needed."""
        # This requires a history endpoint on the Rust side; stub for now.
        return []

    async def get_session_id(self) -> str | None:
        return self._session_id


def _stream_to_legacy(event: StreamEvent | Any) -> Any:
    """Convert a StreamEvent to a legacy dataclass for test compatibility."""
    if isinstance(event, StreamEvent):
        if event.event_type == EventType.TEXT_DELTA:
            return AgentStreamEvent(content=event.content, is_system=False)
        elif event.event_type == EventType.TOOL_START:
            name = event.metadata.get("name", "")
            return AgentStreamEvent(content=f"[tool_start] {name}", is_system=True)
        elif event.event_type == EventType.TOOL_DONE:
            name = event.metadata.get("name", "")
            return AgentStreamEvent(content=f"[tool_done] {name}", is_system=True)
        elif event.event_type == EventType.DONE:
            return AgentCompletedEvent(
                run_id=event.metadata.get("run_id", ""),
                agent_output=event.content,
            )
        elif event.event_type == EventType.ERROR:
            return ErrorEvent(error=event.content)
        elif event.event_type == EventType.CONNECTED:
            return ConnectedEvent(server_version=event.metadata.get("version", ""))
    return event


# ── Backward-compatible aliases ───────────────────────────────────────────────

IagentClient = PersistentIPCClient
IAgentIPCClient = PersistentIPCClient

__all__ = [
    "PersistentIPCClient",
    "IagentClient",
    "IAgentIPCClient",
    "StreamEvent",
    "EventType",
    "CompletionResult",
    # Legacy test types
    "AgentStreamEvent",
    "AgentCompletedEvent",
    "ErrorEvent",
    "ConnectedEvent",
    "MessageRequest",
    "parse_server_event",
]
