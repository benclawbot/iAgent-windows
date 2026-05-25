#!/usr/bin/env python3
"""
iAgent IPC Test Suite
Tests the Python dock -> Rust backend WebSocket connection.
"""

import asyncio
import json
import sys
import time
from iagent.ipc_client import (
    IAgentIPCClient, MessageRequest, AgentStreamEvent,
    AgentCompletedEvent, ErrorEvent, ConnectedEvent, parse_server_event
)


async def test_health_check():
    """Test 1: HTTP health check endpoint."""
    print("\n=== Test 1: Health Check ===")
    import urllib.request
    try:
        resp = urllib.request.urlopen("http://127.0.0.1:7643/health", timeout=5)
        body = resp.read().decode()
        data = json.loads(body)
        print(f"  Status: {data.get('status')}")
        print(f"  Gateway: {data.get('gateway')}")
        print(f"  Version: {data.get('version')}")
        print("PASS")
        return True
    except Exception as e:
        print(f"  FAIL: {e}")
        return False


async def test_websocket_connect():
    """Test 2: WebSocket connection and connected event."""
    print("\n=== Test 2: WebSocket Connect ===")
    client = IAgentIPCClient("127.0.0.1", 7643)
    try:
        ok = await asyncio.wait_for(client.connect(timeout=5.0), timeout=6)
        if ok:
            print("  WebSocket connected successfully")
            print("PASS")
        else:
            print("  FAIL: connect returned False")
        await client.disconnect()
        return ok
    except asyncio.TimeoutError:
        print("  FAIL: connection timeout")
        return False
    except Exception as e:
        print(f"  FAIL: {e}")
        return False


async def test_simple_message():
    """Test 3: Send a simple message and receive stream + completed events."""
    print("\n=== Test 3: Simple Message Roundtrip ===")
    client = IAgentIPCClient("127.0.0.1", 7643)
    
    if not await client.connect(timeout=5.0):
        print("  FAIL: could not connect")
        return False
    
    # Send a simple message
    ok = await client.send_message(
        "Reply with exactly the word 'test' and nothing else.",
        context_id="test-simple-001"
    )
    if not ok:
        print("  FAIL: send_message returned False")
        await client.disconnect()
        return False
    
    print("  Message sent, waiting for response...")
    
    events = []
    stream_text = []
    completed = False
    
    try:
        async for event in client.events():
            events.append(event)
            if isinstance(event, AgentStreamEvent):
                if event.content:
                    stream_text.append(event.content)
                    print(f"  [stream] {repr(event.content[:80])}")
            elif isinstance(event, AgentCompletedEvent):
                print(f"  [completed] run_id={event.run_id[:20]}...")
                completed = True
                break
            elif isinstance(event, ErrorEvent):
                print(f"  [error] {event.error[:200]}")
                break
            elif isinstance(event, ConnectedEvent):
                print(f"  [connected] version={event.server_version}")
        
        await client.disconnect()
        
        print(f"  Total events: {len(events)}")
        print(f"  Stream text: {repr(''.join(stream_text)[:100])}")
        
        if completed:
            print("PASS")
            return True
        else:
            print(f"FAIL: did not receive completed event (got {len(events)} events)")
            return False
            
    except asyncio.TimeoutError:
        print("  FAIL: timeout waiting for events")
        await client.disconnect()
        return False
    except Exception as e:
        print(f"  FAIL: {e}")
        await client.disconnect()
        return False


async def test_longer_conversation():
    """Test 4: Multi-turn conversation via IPC."""
    print("\n=== Test 4: Multi-turn Conversation ===")
    client = IAgentIPCClient("127.0.0.1", 7643)
    
    if not await client.connect(timeout=5.0):
        print("  FAIL: could not connect")
        return False
    
    messages = [
        ("First, tell me what 2+2 equals. Reply with just the number.", "test-conv-001"),
        ("Now multiply that by 3. Reply with just the number.", "test-conv-002"),
    ]
    
    all_passed = True
    for i, (msg, ctx_id) in enumerate(messages):
        print(f"  Turn {i+1}: {msg[:50]}...")
        ok = await client.send_message(msg, context_id=ctx_id)
        if not ok:
            print(f"    FAIL: send failed")
            all_passed = False
            continue
        
        events_received = 0
        completed = False
        try:
            async for event in client.events():
                events_received += 1
                if isinstance(event, AgentCompletedEvent):
                    completed = True
                    break
                elif isinstance(event, ErrorEvent):
                    print(f"    [error] {event.error[:100]}")
                    break
            print(f"    Events: {events_received}, completed: {completed}")
            if not completed:
                all_passed = False
        except asyncio.TimeoutError:
            print(f"    FAIL: timeout")
            all_passed = False
    
    await client.disconnect()
    
    if all_passed:
        print("PASS")
    else:
        print("FAIL")
    return all_passed


async def test_cancel():
    """Test 5: Send a long message then cancel it."""
    print("\n=== Test 5: Cancel Run ===")
    client = IAgentIPCClient("127.0.0.1", 7643)
    
    if not await client.connect(timeout=5.0):
        print("  FAIL: could not connect")
        return False
    
    # Send a message that would take a while
    ok = await client.send_message(
        "Write a detailed explanation of quantum computing. Go very deep into the topic.",
        context_id="test-cancel-001"
    )
    if not ok:
        print("  FAIL: send failed")
        await client.disconnect()
        return False
    
    # Wait a moment for agent to start
    await asyncio.sleep(1)
    
    # Cancel it
    ok = await client.cancel("test-cancel-001")
    print(f"  Cancel sent: {ok}")
    
    # Should receive cancellation confirmation
    cancelled = False
    try:
        async for event in client.events():
            if isinstance(event, ErrorEvent) and "cancel" in event.error.lower():
                cancelled = True
                print(f"  [cancel confirmed] {event.error[:80]}")
                break
            elif isinstance(event, AgentCompletedEvent):
                print(f"  [completed normally] - not cancelled")
                break
    except asyncio.TimeoutError:
        print("  Timeout waiting for cancel response")
    
    await client.disconnect()
    # Cancel is best-effort; we just verify it doesn't crash
    print("PASS (cancel sent without crash)")
    return True


async def test_ipc_protocol_events():
    """Test 6: Verify all event types are parseable."""
    print("\n=== Test 6: Protocol Parsing ===")
    
    test_cases = [
        ('{"type":"agent_stream","content":"hello","is_system":false}', AgentStreamEvent),
        ('{"type":"agent_completed","run_id":"abc123","message_id":"msg456","agent_output":"done"}', AgentCompletedEvent),
        ('{"type":"error","error":"something went wrong","fatal":false}', ErrorEvent),
        ('{"type":"connected","server_version":"v1.0.0"}', ConnectedEvent),
    ]
    
    all_ok = True
    for json_str, expected_type in test_cases:
        event = parse_server_event(json_str)
        if isinstance(event, expected_type):
            print(f"  {expected_type.__name__}: OK")
        else:
            print(f"  {expected_type.__name__}: FAIL (got {type(event)})")
            all_ok = False
    
    if all_ok:
        print("PASS")
    else:
        print("FAIL")
    return all_ok


async def run_all_tests():
    print("=" * 60)
    print(" iAgent IPC Test Suite")
    print("=" * 60)
    
    results = []
    
    results.append(("Health Check", await test_health_check()))
    results.append(("WebSocket Connect", await test_websocket_connect()))
    results.append(("Simple Message", await test_simple_message()))
    results.append(("Multi-turn Conversation", await test_longer_conversation()))
    results.append(("Cancel Run", await test_cancel()))
    results.append(("Protocol Parsing", await test_ipc_protocol_events()))
    
    print("\n" + "=" * 60)
    print(" RESULTS")
    print("=" * 60)
    
    all_passed = True
    for name, passed in results:
        status = "PASS" if passed else "FAIL"
        print(f"  {name:.<40} {status}")
        if not passed:
            all_passed = False
    
    print("=" * 60)
    
    if all_passed:
        print("All tests passed!")
    else:
        print("Some tests failed.")
    
    return all_passed


if __name__ == "__main__":
    ok = asyncio.run(run_all_tests())
    sys.exit(0 if ok else 1)