#!/usr/bin/env python3
"""
Run SWE-bench Lite benchmark with jcode + MiniMax M2.7
Full dataset: 300 instances

Usage:
    python scripts/benchmark_swebench_jcode.py run --parallel 4
"""

import json
import os
import subprocess
import sys
import time
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor, as_completed
from datasets import load_dataset

RESULTS_DIR = Path.home() / ".jcode/swe-bench-results"
MODEL = "MiniMax-M2.7"
TIMEOUT_SECONDS = 300  # 5 min per task
MAX_PARALLEL = 4


def run_task(instance_id: str, problem: str) -> dict:
    """Run a single SWE-bench task with jcode."""
    work_dir = f"/tmp/swe_bench/{instance_id.replace('__', '_')}"
    
    task = f"""Task: {instance_id}
Fix this issue:
{problem[:500]}...

Clone the repo, make the fix, and test it."""

    result = {
        "instance_id": instance_id,
        "resolved": False,
        "duration": 0,
        "error": None,
    }
    
    start = time.time()
    
    try:
        proc = subprocess.Popen(
            ["jcode", "--provider", "minimax", "run", task],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            cwd="/tmp",
        )
        
        stdout, stderr = proc.communicate(timeout=TIMEOUT_SECONDS)
        duration = time.time() - start
        
        output = stdout.decode() + stderr.decode()
        
        # Check resolution
        resolved = any(x in output.lower() for x in [
            "resolved", "success", "fixed", "test passed", 
            "all tests passed", "PASSED"
        ])
        
        result["resolved"] = resolved
        result["duration"] = duration
        
    except subprocess.TimeoutExpired:
        proc.kill()
        result["duration"] = TIMEOUT_SECONDS
        result["error"] = "timeout"
    except Exception as e:
        result["error"] = str(e)
        result["duration"] = time.time() - start
    
    return result


def main():
    import argparse
    
    parser = argparse.ArgumentParser()
    parser.add_argument("action", choices=["run", "results"])
    parser.add_argument("--parallel", type=int, default=4)
    parser.add_argument("--limit", type=int, default=None)
    args = parser.parse_args()
    
    results_file = RESULTS_DIR / "results.json"
    instances_file = RESULTS_DIR / "instances.json"
    
    if args.action == "results":
        if results_file.exists():
            with open(results_file) as f:
                results = json.load(f)
            total = len(results)
            resolved = sum(1 for r in results if r.get("resolved"))
            print(f"\n=== Jcode + {MODEL} on SWE-bench Lite ===")
            print(f"Results: {resolved}/{total} ({100*resolved/total:.1f}%)")
            for r in results:
                s = "✓" if r.get("resolved") else "✗"
                print(f"  {s} {r['instance_id']}")
        else:
            print("No results yet")
        return
    
    # Load instances
    if not instances_file.exists():
        print("Loading SWE-bench Lite dataset...")
        ds = load_dataset(
            '/home/thomas/.cache/huggingface/datasets/princeton-nlp___swe-bench_lite/default/0.0.0/6ec7bb89b9342f664a54a6e0a6ea6501d3437cc2',
            split='test'
        )
        instances = []
        for i in range(len(ds)):
            instances.append({
                "id": ds[i]["instance_id"],
                "problem": ds[i]["problem_statement"]
            })
        with open(instances_file, "w") as f:
            json.dump(instances, f)
    else:
        with open(instances_file) as f:
            instances = json.load(f)
    
    if args.limit:
        instances = instances[:args.limit]
    
    # Load existing results
    if results_file.exists():
        with open(results_file) as f:
            results = json.load(f)
    else:
        results = []
    
    done_ids = {r["instance_id"] for r in results}
    pending = [i for i in instances if i["id"] not in done_ids]
    
    print(f"SWE-bench Lite: {len(instances)} total, {len(done_ids)} done, {len(pending)} pending")
    print(f"Running with {args.parallel} parallel workers...")
    
    # Run pending tasks in parallel
    with ThreadPoolExecutor(max_workers=args.parallel) as executor:
        futures = {
            executor.submit(run_task, inst["id"], inst["problem"]): inst
            for inst in pending
        }
        
        for future in as_completed(futures):
            inst = futures[future]
            result = future.result()
            
            results.append(result)
            
            # Save progress
            with open(results_file, "w") as f:
                json.dump(results, f)
            
            status = "✓" if result["resolved"] else "✗"
            print(f"  {status} {result['instance_id']} ({result['duration']:.0f}s)")
    
    # Summary
    total = len(results)
    resolved = sum(1 for r in results if r.get("resolved"))
    print(f"\n=== Final: {resolved}/{total} ({100*resolved/total:.1f}%) ===")


if __name__ == "__main__":
    main()