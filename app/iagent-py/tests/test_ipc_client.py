#!/usr/bin/env python3
"""
IPC client unit tests — no server required.

Tests the PersistentIPCClient class logic without needing a live iAgent server.
Covers: protocol parsing, event dispatch, state transitions, connection lifecycle.
"""

import asyncio
import json
import sys
from pathlib import Path
from unittest.mock import AsyncMock, MagicMock, patch

# ── bootstrap ─────────────────────────────────────────────────────────────────

sys.path.insert(0, str(Path(__file__).parent.parent))
from iagent.ipc_client import (
    AgentCompletedEvent,
    AgentStreamEvent,
    ConnectedEvent,
    ErrorEvent,
    EventType,
    IagentClient,
    IAgentIPCClient,
    MessageRequest,
    PersistentIPCClient,
    StreamEvent,
    _stream_to_legacy,
    parse_server_event,
)

# ── helpers ────────────────────────────────────────────────────────────────────

def mkevent(type_: str, **fields) -> str:
    return json.dumps({"type": type_, **fields})


# ── 1. parse_server_event ──────────────────────────────────────────────────────

def test_parse_agent_stream():
    e = parse_server_event(mkevent("agent_stream", content="hello world", is_system=False))
    assert isinstance(e, AgentStreamEvent), type(e)
    assert e.content == "hello world"
    assert e.is_system is False

def test_parse_text_delta():
    e = parse_server_event(mkevent("text_delta", content="hi"))
    assert isinstance(e, AgentStreamEvent), type(e)
    assert e.content == "hi"

def test_parse_agent_completed():
    e = parse_server_event(mkevent("agent_completed", run_id="abc123",
                                    message_id="msg456", agent_output="done"))
    assert isinstance(e, AgentCompletedEvent), type(e)
    assert e.run_id == "abc123"
    assert e.message_id == "msg456"
    assert e.agent_output == "done"

def test_parse_done_alias():
    e = parse_server_event(mkevent("done", run_id="r1", content="final output"))
    assert isinstance(e, AgentCompletedEvent), type(e)
    assert e.run_id == "r1"

def test_parse_error():
    e = parse_server_event(mkevent("error", error="something broke", fatal=True))
    assert isinstance(e, ErrorEvent), type(e)
    assert e.error == "something broke"
    assert e.fatal is True

def test_parse_connected():
    e = parse_server_event(mkevent("connected", session_id="sess1", version="v1.2.0"))
    assert isinstance(e, ConnectedEvent), type(e)
    assert e.server_version == "v1.2.0"

def test_parse_unknown_type():
    e = parse_server_event(mkevent("foo_bar", x=1))
    assert isinstance(e, StreamEvent), type(e)
    assert e.event_type == EventType.ERROR
    assert "foo_bar" in e.content

def test_parse_empty_type():
    e = parse_server_event(mkevent("", content="random"))
    assert isinstance(e, StreamEvent), type(e)
    assert e.event_type == EventType.ERROR
    assert "unknown message type" in e.content

def test_parse_malformed_json():
    e = parse_server_event("not valid json{")
    assert isinstance(e, StreamEvent), type(e)
    assert e.event_type == EventType.ERROR
    assert "malformed JSON" in e.content


# ── 2. aliases ────────────────────────────────────────────────────────────────

def test_aliases():
    assert IAgentIPCClient is PersistentIPCClient
    assert IagentClient is PersistentIPCClient


# ── 3. initial state ─────────────────────────────────────────────────────────

def test_initial_state():
    c = PersistentIPCClient()
    assert c.is_connected() is False
    assert c._connected is False
    assert c._pending == {}

    c2 = PersistentIPCClient(host="192.168.1.1", port=9000)
    assert c2.host == "192.168.1.1"
    assert c2.port == 9000
    assert c2.timeout == 30.0

    c3 = PersistentIPCClient(timeout=5.0)
    assert c3.timeout == 5.0


# ── 4. send_message raises when disconnected ─────────────────────────────────

def test_send_message_disconnected():
    async def _test():
        c = PersistentIPCClient()
        try:
            await c.send_message("hello")
            raise AssertionError("Should have raised ConnectionError")
        except ConnectionError:
            pass
    asyncio.run(_test())


# ── 5. cancel raises when disconnected ───────────────────────────────────────

def test_cancel_disconnected():
    async def _test():
        c = PersistentIPCClient()
        ok = await c.cancel("task-1")
        assert ok is False
    asyncio.run(_test())


# ── 6. _stream_to_legacy ────────────────────────────────────────────────────

def test_stream_to_legacy_text_delta():
    se = StreamEvent(event_type=EventType.TEXT_DELTA, content="partial")
    out = _stream_to_legacy(se)
    assert isinstance(out, AgentStreamEvent), type(out)
    assert out.content == "partial"
    assert out.is_system is False

def test_stream_to_legacy_done():
    se = StreamEvent(event_type=EventType.DONE, content="final result",
                     metadata={"run_id": "run-42"})
    out = _stream_to_legacy(se)
    assert isinstance(out, AgentCompletedEvent), type(out)
    assert out.run_id == "run-42"

def test_stream_to_legacy_error():
    se = StreamEvent(event_type=EventType.ERROR, content="oops")
    out = _stream_to_legacy(se)
    assert isinstance(out, ErrorEvent), type(out)
    assert out.error == "oops"

def test_stream_to_legacy_connected():
    se = StreamEvent(event_type=EventType.CONNECTED,
                     metadata={"version": "v2.0", "session_id": "s1"})
    out = _stream_to_legacy(se)
    assert isinstance(out, ConnectedEvent), type(out)
    assert out.server_version == "v2.0"

def test_stream_to_legacy_tool_start():
    se = StreamEvent(event_type=EventType.TOOL_START, metadata={"name": "shell"})
    out = _stream_to_legacy(se)
    assert isinstance(out, AgentStreamEvent), type(out)
    assert out.is_system is True
    assert "shell" in out.content

def test_stream_to_legacy_tool_done():
    se = StreamEvent(event_type=EventType.TOOL_DONE, metadata={"name": "shell"})
    out = _stream_to_legacy(se)
    assert isinstance(out, AgentStreamEvent), type(out)
    assert out.is_system is True

def test_stream_to_legacy_already_legacy():
    e = ErrorEvent(error="already error")
    out = _stream_to_legacy(e)
    assert out is e  # pass-through


# ── 7. socket_path property ───────────────────────────────────────────────────

def test_socket_path_property():
    c = PersistentIPCClient(host="127.0.0.1", port=7643)
    assert c.socket_path == "127.0.0.1:7643"
    c2 = PersistentIPCClient(host="example.com", port=8080)
    assert c2.socket_path == "example.com:8080"


# ── 8. dispatch routes correctly ────────────────────────────────────────────

def test_dispatch_stream_event():
    c = PersistentIPCClient()
    c._event_queues["task-1"] = asyncio.Queue()

    msg = {"type": "text_delta", "content": "hi", "task_id": "task-1"}
    c._dispatch(msg)

    assert "task-1" in c._event_queues
    # drain queue within async context
    async def drain():
        got = []
        while True:
            try:
                item = c._event_queues["task-1"].get_nowait()
                got.append(item)
            except asyncio.QueueEmpty:
                break
        return got
    got = asyncio.run(drain())
    assert len(got) == 1
    assert isinstance(got[0], StreamEvent)
    assert got[0].content == "hi"

def test_dispatch_done_sets_pending():
    loop = asyncio.new_event_loop()
    asyncio.set_event_loop(loop)
    try:
        c = PersistentIPCClient()
        c._event_queues["task-2"] = asyncio.Queue()
        future = loop.create_future()
        c._pending["task-2"] = future

        msg = {"type": "done", "content": "result", "task_id": "task-2", "run_id": "run-abc"}
        c._dispatch(msg)

        # Future was set before _cleanup_task removed the entry
        assert future.done()
        result = future.result()
        assert result["text"] == "result"
        assert result["run_id"] == "run-abc"
        # _cleanup_task removed the task
        assert "task-2" not in c._pending
    finally:
        loop.close()

def test_dispatch_error_sets_pending_exception():
    loop = asyncio.new_event_loop()
    asyncio.set_event_loop(loop)
    try:
        c = PersistentIPCClient()
        c._event_queues["task-3"] = asyncio.Queue()
        future = loop.create_future()
        c._pending["task-3"] = future

        msg = {"type": "error", "error": "failed", "task_id": "task-3"}
        c._dispatch(msg)

        assert future.done()
        try:
            future.result()
            raise AssertionError("Should have raised")
        except RuntimeError as exc:
            assert "failed" in str(exc)
        assert "task-3" not in c._pending
    finally:
        loop.close()

def test_dispatch_connected_sets_session_id():
    c = PersistentIPCClient()
    msg = {"type": "session_id", "session_id": "sess-xyz"}
    c._dispatch(msg)
    assert c._session_id == "sess-xyz"

def test_dispatch_cancel_ack():
    loop = asyncio.new_event_loop()
    asyncio.set_event_loop(loop)
    try:
        c = PersistentIPCClient()
        c._event_queues["task-4"] = asyncio.Queue()
        future = loop.create_future()
        c._pending["task-4"] = future

        msg = {"type": "cancel_ack", "task_id": "task-4", "reason": "user cancelled"}
        c._dispatch(msg)

        # cancel_ack pushes an ERROR event then removes task from pending and queues
        assert "task-4" not in c._pending
        assert "task-4" not in c._event_queues
        # Future is never resolved by cancel_ack (caller reads from events iterator instead)
        assert not future.done()
    finally:
        loop.close()

def test_dispatch_tool_start():
    loop = asyncio.new_event_loop()
    asyncio.set_event_loop(loop)
    try:
        c = PersistentIPCClient()
        c._event_queues["task-5"] = asyncio.Queue()
        future = loop.create_future()
        c._pending["task-5"] = future

        msg = {"type": "tool_start", "tool": "shell", "input": {"cmd": "ls"}, "task_id": "task-5"}
        c._dispatch(msg)

        async def drain():
            got = []
            while True:
                try:
                    item = c._event_queues["task-5"].get_nowait()
                    got.append(item)
                except asyncio.QueueEmpty:
                    break
            return got
        got = asyncio.run(drain())
        assert len(got) == 1
        assert got[0].event_type == EventType.TOOL_START
        assert got[0].metadata["name"] == "shell"
    finally:
        loop.close()


# ── 9. _cleanup_task ─────────────────────────────────────────────────────────

def test_cleanup_task():
    loop = asyncio.new_event_loop()
    asyncio.set_event_loop(loop)
    try:
        c = PersistentIPCClient()
        c._event_queues["t1"] = asyncio.Queue()
        future = loop.create_future()
        c._pending["t1"] = future
        c._cleanup_task("t1")
        assert "t1" not in c._event_queues
        assert "t1" not in c._pending
    finally:
        loop.close()


# ── 10. ping not connected ───────────────────────────────────────────────────

def test_ping_disconnected():
    async def _test():
        c = PersistentIPCClient()
        ok = await c.ping()
        assert ok is False
    asyncio.run(_test())


# ── 11. get_state disconnected ───────────────────────────────────────────────

def test_get_state_disconnected():
    async def _test():
        c = PersistentIPCClient()
        state = await c.get_state()
        assert state["connected"] is False
        assert state["session_id"] == ""
        assert state["is_processing"] is False
        assert state["port"] == 7643
    asyncio.run(_test())


# ── 12. connect/disconnect lifecycle (mocked TCP) ────────────────────────────

def test_connect_then_disconnect():
    async def _test():
        c = PersistentIPCClient()

        mock_reader = AsyncMock()
        mock_reader.read = AsyncMock(return_value=b"")
        mock_writer = AsyncMock()
        mock_writer.close = MagicMock()   # sync mock — matches real StreamWriter.close()
        mock_writer.wait_closed = AsyncMock()
        mock_writer.is_closing = MagicMock(return_value=False)

        with patch(
            "asyncio.open_connection",
            new_callable=AsyncMock,
            return_value=(mock_reader, mock_writer),
        ):
            ok = await c.connect(timeout=2.0)
            assert ok is True
            assert c.is_connected() is True
            assert c._read_task is not None
            # second connect is no-op
            ok2 = await c.connect()
            assert ok2 is True
            # disconnect
            await c.disconnect()
            assert c.is_connected() is False
            mock_writer.close.assert_called_once()
            # second disconnect is no-op
            await c.disconnect()

    asyncio.run(_test())

def test_connect_timeout():
    async def _test():
        c = PersistentIPCClient()
        with patch("asyncio.open_connection", new_callable=AsyncMock,
                   side_effect=asyncio.TimeoutError):
            ok = await c.connect(timeout=0.5)
            assert ok is False
            assert c.is_connected() is False
    asyncio.run(_test())

def test_connect_failure():
    async def _test():
        c = PersistentIPCClient()
        with patch("asyncio.open_connection", new_callable=AsyncMock,
                   side_effect=OSError("refused")):
            ok = await c.connect(timeout=2.0)
            assert ok is False
            assert c.is_connected() is False
    asyncio.run(_test())


# ── 13. MessageRequest dataclass ──────────────────────────────────────────────

def test_message_request():
    req = MessageRequest(content="hello", context_id="ctx1", images=["img1"])
    assert req.content == "hello"
    assert req.context_id == "ctx1"
    assert req.images == ["img1"]
    j = json.dumps({"type": "message", "content": req.content,
                    "context_id": req.context_id, "images": req.images})
    parsed = json.loads(j)
    assert parsed["content"] == "hello"


# ── run ───────────────────────────────────────────────────────────────────────

if __name__ == "__main__":
    tests = [
        test_parse_agent_stream,
        test_parse_text_delta,
        test_parse_agent_completed,
        test_parse_done_alias,
        test_parse_error,
        test_parse_connected,
        test_parse_unknown_type,
        test_parse_empty_type,
        test_parse_malformed_json,
        test_aliases,
        test_initial_state,
        test_send_message_disconnected,
        test_cancel_disconnected,
        test_stream_to_legacy_text_delta,
        test_stream_to_legacy_done,
        test_stream_to_legacy_error,
        test_stream_to_legacy_connected,
        test_stream_to_legacy_tool_start,
        test_stream_to_legacy_tool_done,
        test_stream_to_legacy_already_legacy,
        test_socket_path_property,
        test_dispatch_stream_event,
        test_dispatch_done_sets_pending,
        test_dispatch_error_sets_pending_exception,
        test_dispatch_connected_sets_session_id,
        test_dispatch_cancel_ack,
        test_dispatch_tool_start,
        test_cleanup_task,
        test_ping_disconnected,
        test_get_state_disconnected,
        test_connect_then_disconnect,
        test_connect_timeout,
        test_connect_failure,
        test_message_request,
    ]

    failed = []
    for fn in tests:
        try:
            fn()
            print(f"  PASS  {fn.__name__}")
        except Exception as e:
            print(f"  FAIL  {fn.__name__}: {e}")
            failed.append((fn.__name__, e))

    print()
    if failed:
        print(f"{len(failed)}/{len(tests)} FAILED")
        for name, exc in failed:
            print(f"  {name}: {exc}")
        sys.exit(1)
    else:
        print(f"All {len(tests)} tests passed!")
        sys.exit(0)
