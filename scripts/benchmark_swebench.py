#!/usr/bin/env python3
"""
Benchmark: Jcode with MiniMax M2.7 on SWE-bench Lite instances.

Usage:
    python scripts/benchmark_swebench.py run --instances 5
    python scripts/benchmark_swebench.py results
"""

import json
import os
import socket
import subprocess
import sys
import time
import select
from pathlib import Path

DEBUG_SOCKET = f"/run/user/{os.getuid()}/jcode-debug.sock"
RESULTS_DIR = Path.home() / ".jcode/swe-bench-results"
MODEL = "minimax/MiniMax-M2.7"
TIMEOUT_SECONDS = 600  # 10 min per task


def send_cmd(cmd: str, session_id: str = None, timeout: float = 300) -> tuple:
    """Send a debug command and get response."""
    sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    sock.connect(DEBUG_SOCKET)
    sock.setblocking(False)

    req = {"type": "debug_command", "id": 1, "command": cmd}
    if session_id:
        req["session_id"] = session_id

    sock.send((json.dumps(req) + '\n').encode())

    start = time.time()
    data = b""
    while time.time() - start < timeout:
        ready, _, _ = select.select([sock], [], [], 1.0)
        if ready:
            try:
                chunk = sock.recv(65536)
                if not chunk:
                    break
                data += chunk
                if b'\n' in data:
                    break
            except BlockingIOError:
                continue

    sock.close()

    if not data:
        return False, "", "No response"

    try:
        resp = json.loads(data.decode().strip())
        if resp.get("id") == 1:
            return resp.get("success", False), resp.get("output", ""), resp.get("error", "")
    except:
        pass
    return False, data.decode(), ""


def run_task(instance_id: str, problem_statement: str) -> dict:
    """Run a single SWE-bench task with jcode."""
    result = {
        "instance_id": instance_id,
        "status": "pending",
        "started_at": time.time(),
        "ended_at": None,
        "resolved": False,
    }

    # Check if jcode server is running
    try:
        ok, _, _ = send_cmd("status")
        if not ok:
            print("  Jcode server not responding, starting new session...")
    except:
        pass

    # Create session for this task
    session_id = f"swe-{instance_id}"

    # Send task to jcode
    prompt = f"""You are working on a SWE-bench task: {instance_id}

Problem:
{problem_statement[:2000]}...

Your goal is to resolve this GitHub issue. Follow these steps:
1. Explore the repository structure
2. Understand the issue
3. Make the necessary code changes
4. Test your changes
5. Submit when done

Work in /tmp/swe-bench/{instance_id}
"""

    print(f"  Sending task to jcode...")
    ok, _, _ = send_cmd(f'session {session_id}', timeout=10)
    time.sleep(1)

    # Send the task
    ok, _, _ = send_cmd(f'send {session_id} "{prompt[:500].replace('"', '\\"')}"', timeout=10)
    time.sleep(2)

    # Monitor progress
    start_time = time.time()
    while time.time() - start_time < TIMEOUT_SECONDS:
        ok, output, _ = send_cmd(f'status {session_id}', timeout=10)
        if "running" in output.lower() or "processing" in output.lower():
            print(f"  Agent running... ({int(time.time() - start_time)}s)")
        time.sleep(30)

    result["ended_at"] = time.time()
    result["duration"] = result["ended_at"] - result["started_at"]

    # Check for resolution
    ok, output, _ = send_cmd(f'check_resolution {session_id}', timeout=30)
    result["resolved"] = "resolved" in output.lower() or "success" in output.lower()
    result["status"] = "resolved" if result["resolved"] else "timeout"

    return result


def main():
    import argparse
    from datasets import load_dataset

    parser = argparse.ArgumentParser()
    parser.add_argument("action", choices=["run", "results"])
    parser.add_argument("--instances", type=int, default=10)
    parser.add_argument("--limit", type=int, default=None)
    args = parser.parse_args()

    if args.action == "results":
        # Show results
        if RESULTS_DIR.exists():
            results_file = RESULTS_DIR / "results.json"
            if results_file.exists():
                with open(results_file) as f:
                    data = json.load(f)
                total = len(data)
                resolved = sum(1 for r in data if r.get("resolved"))
                print(f"SWE-bench Lite Results: {resolved}/{total} resolved ({100*resolved/total:.1f}%)")
                for r in data:
                    status = "✓" if r.get("resolved") else "✗"
                    print(f"  {status} {r['instance_id']} ({r.get('duration', 0):.0f}s)")
            else:
                print("No results yet")
        else:
            print("No results yet")
        return

    # Load SWE-bench Lite dataset
    print("Loading SWE-bench Lite dataset...")
    ds = load_dataset('/home/thomas/.cache/huggingface/datasets/princeton-nlp___swe-bench_lite/default/0.0.0/6ec7bb89b9342f664a54a6e0a6ea6501d3437cc2', split='test')

    # Select instances (limit for quick test)
    instances = list(range(min(args.instances, len(ds))))
    if args.limit:
        instances = instances[:args.limit]

    print(f"Running benchmark on {len(instances)} SWE-bench Lite instances with {MODEL}")

    RESULTS_DIR.mkdir(parents=True, exist_ok=True)
    results_file = RESULTS_DIR / "results.json"

    # Load existing results
    if results_file.exists():
        with open(results_file) as f:
            results = json.load(f)
    else:
        results = []

    # Run tasks
    for i in instances:
        inst = ds[i]
        instance_id = inst["instance_id"]

        # Skip if already done
        if any(r["instance_id"] == instance_id for r in results):
            print(f"Skipping {instance_id} (already done)")
            continue

        print(f"\n[{i+1}/{len(instances)}] Running {instance_id}...")
        problem = inst["problem_statement"]

        result = run_task(instance_id, problem)
        results.append(result)

        # Save progress
        with open(results_file, "w") as f:
            json.dump(results, f, indent=2)

        status = "✓ RESOLVED" if result["resolved"] else "✗ TIMEOUT"
        print(f"  {status} ({result.get('duration', 0):.0f}s)")

    # Summary
    total = len(results)
    resolved = sum(1 for r in results if r.get("resolved"))
    print(f"\n=== SWE-bench Lite Results: {resolved}/{total} resolved ({100*resolved/total:.1f}%) ===")


if __name__ == "__main__":
    main()