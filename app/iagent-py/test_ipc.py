"""
Integration test for iAgent IPC client.

Tests:
1. Starting iAgent server as background daemon
2. Python client connecting via Unix socket
3. Send message and receive streaming events
4. Config file-based auth (API key)
"""

import asyncio
import json
import os
import sys
import tempfile
import time

# Add iagent package to path
sys.path.insert(0, os.path.join(os.path.dirname(__file__)))

from iagent.ipc_client import IAgentClient, EventType, connect


async def test_basic_connection():
    """Test: connect to server, ping, get state."""
    print("\n=== Test 1: Basic Connection ===")
    
    # First check if server is running
    import subprocess
    try:
        proc = subprocess.run(
            ["powershell", "-Command", "Get-NetTCPConnection -LocalPort 7643 -ErrorAction SilentlyContinue"],
            capture_output=True, text=True, timeout=5
        )
        if proc.stdout.strip():
            print("  Gateway port 7643 is in use")
        else:
            print("  Gateway port 7643 is NOT in use (server not running)")
    except:
        pass
    
    client = IAgentClient()
    try:
        await client.connect()
        print(f"  Connected to {client.socket_path}")
        
        # Ping
        result = await client.ping()
        print(f"  Ping: {'OK' if result else 'FAILED'}")
        
        # Get state
        state = await client.get_state()
        print(f"  GetState: session_id={state.get('session_id', 'unknown')}, is_processing={state.get('is_processing', 'unknown')}")
        
        await client.disconnect()
        print("  Disconnected: OK")
        return True
    except ConnectionError as e:
        print(f"  Connection failed: {e}")
        return False
    except Exception as e:
        print(f"  Error: {e}")
        return False


async def test_send_message():
    """Test: send a message and stream response events."""
    print("\n=== Test 2: Send Message ===")
    
    client = IAgentClient()
    try:
        await client.connect()
        
        # Subscribe to events first
        print("  Subscribing...")
        event_count = 0
        last_event_type = None
        
        async for event in client.send_message("Hello, what can you do?"):
            event_count += 1
            last_event_type = event.event_type
            
            if event.event_type == EventType.TEXT_DELTA:
                print(f"  [text_delta] {event.content[:80]}...", flush=True)
            elif event.event_type == EventType.TOOL_START:
                print(f"  [tool_start] {event.metadata.get('name', 'unknown')}")
            elif event.event_type == EventType.TOOL_DONE:
                print(f"  [tool_done] {event.metadata.get('name', 'unknown')}")
            elif event.event_type == EventType.ERROR:
                print(f"  [error] {event.metadata.get('message', 'unknown error')}")
            elif event.event_type == EventType.DONE:
                print(f"  [done] id={event.id}")
            elif event.event_type == EventType.CONNECTION_TYPE:
                print(f"  [connection_type] {event.metadata.get('connection', 'unknown')}")
            elif event.event_type == EventType.SESSION_ID:
                print(f"  [session_id] {event.metadata.get('session_id', 'unknown')}")
            
            # Safety: stop after 30 events to avoid infinite loop
            if event_count >= 30:
                print("  (stopping after 30 events)")
                break
        
        print(f"  Total events: {event_count}, last: {last_event_type}")
        await client.disconnect()
        return last_event_type == EventType.DONE
    except Exception as e:
        print(f"  Error: {e}")
        import traceback
        traceback.print_exc()
        return False


async def test_get_history():
    """Test: retrieve conversation history."""
    print("\n=== Test 3: Get History ===")
    
    client = IAgentClient()
    try:
        await client.connect()
        
        history = await client.get_history()
        print(f"  History: {len(history)} messages")
        for i, msg in enumerate(history[:3]):
            role = msg.get('role', 'unknown')
            content = msg.get('content', '')
            if len(content) > 60:
                content = content[:60] + "..."
            print(f"    [{i}] {role}: {content}")
        if len(history) > 3:
            print(f"    ... ({len(history) - 3} more)")
        
        await client.disconnect()
        return True
    except Exception as e:
        print(f"  Error: {e}")
        return False


async def test_server_daemon():
    """Test: start iAgent server as daemon subprocess."""
    print("\n=== Test 4: Server Daemon ===")
    
    # Find iagent binary
    possible_paths = [
        "C:/Users/thoma/iagent-windows/target/release/iagent.exe",
        "C:/Users/thoma/iagent-windows/target/debug/iagent.exe",
        os.path.expanduser("%LOCALAPPDATA%/iAgent/bin/iagent.exe"),
    ]
    
    binary_path = None
    for p in possible_paths:
        if os.path.exists(p):
            binary_path = p
            break
    
    if not binary_path:
        print("  ERROR: iAgent binary not found at any known path")
        return False
    
    print(f"  Binary: {binary_path}")
    
    # Start server in background
    import subprocess
    proc = subprocess.Popen(
        [binary_path, "serve"],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        cwd=os.path.dirname(binary_path),
        creationflags=subprocess.CREATE_NEW_PROCESS_GROUP if sys.platform == 'win32' else 0,
    )
    
    print(f"  Server started (pid={proc.pid}), waiting for startup...")
    await asyncio.sleep(3)
    
    # Check if process is still running
    if proc.poll() is not None:
        stdout, stderr = proc.communicate()
        print(f"  Server exited early: code={proc.returncode}")
        print(f"  stdout: {stdout[:500]}")
        print(f"  stderr: {stderr[:500]}")
        return False
    
    print("  Server running, connecting client...")
    
    # Now connect with our client
    client = IAgentClient()
    try:
        await client.connect()
        result = await client.ping()
        print(f"  Ping: {'OK' if result else 'FAILED'}")
        await client.disconnect()
        success = result
    except Exception as e:
        print(f"  Client error: {e}")
        success = False
    
    # Kill the server
    print("  Stopping server...")
    proc.terminate()
    try:
        proc.wait(timeout=5)
    except subprocess.TimeoutExpired:
        proc.kill()
    
    return success


async def main():
    print("=" * 60)
    print("iAgent IPC Integration Tests")
    print("=" * 60)
    
    results = {}
    
    # Test 1: Basic connection (requires server to be running)
    results["basic_connection"] = await test_basic_connection()
    
    # Test 2: Send message (requires server to be running)
    results["send_message"] = await test_send_message()
    
    # Test 3: History (requires server to be running)
    results["get_history"] = await test_get_history()
    
    # Test 4: Server daemon (starts server from scratch)
    results["server_daemon"] = await test_server_daemon()
    
    print("\n" + "=" * 60)
    print("RESULTS")
    print("=" * 60)
    for name, passed in results.items():
        status = "PASS" if passed else "FAIL"
        print(f"  {name}: {status}")
    
    all_passed = all(results.values())
    print(f"\nOverall: {'ALL PASSED' if all_passed else 'SOME FAILED'}")
    return 0 if all_passed else 1


if __name__ == "__main__":
    sys.exit(asyncio.run(main()))