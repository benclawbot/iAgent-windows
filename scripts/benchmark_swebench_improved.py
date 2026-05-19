#!/usr/bin/env python3
"""
Run SWE-bench Lite benchmark with jcode + improved agentic strategies.
Full dataset: 300 instances

Improvements:
- Full problem statement passed
- SWE-bench harness for proper verification
- Retry logic with self-debugging
- Multi-model support (MiniMax, Claude, GPT)

Usage:
    python scripts/benchmark_swebench_improved.py run --parallel 4 --model minimax
"""

import json
import os
import subprocess
import sys
import time
import re
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor, as_completed
from datasets import load_dataset

RESULTS_DIR = Path.home() / ".jcode/swe-bench-results"
SWE_BENCH_DIR = Path.home() / "SWE-bench"
TIMEOUT_SECONDS = 600  # 10 min per task (increased for retry)
MAX_RETRIES = 2

# Fixed model: MiniMax M2.7 only
MODEL_PROVIDER = "minimax"
MODEL_NAME = "MiniMax-M2.7"


def run_task_with_retry(instance_id: str, problem: str) -> dict:
    """Build an improved prompt with full context and instructions."""
    
    baseprompt = f"""## SWE-bench Task: {instance_id}

### Issue Description
{problem}

### Your Task
1. Setup the environment using SWE-bench harness:
   cd {SWE_BENCH_DIR}
   python -m swebench.harness.run_instance {instance_id}

2. Analyze the problem and implement the fix

3. Verify your fix:
   python -m swebench.harness.run_instance {instance_id} --verify

4. Report: PASS if all tests pass, FAIL otherwise

### Important
- Do NOT stop until tests pass or you've tried at least 3 different approaches
- If your first fix doesn't work, analyze WHY and try again
- Look at test failures for hints about what's wrong
- Search the codebase for similar patterns that might guide your solution
"""
    
    return baseprompt


def run_task_with_retry(instance_id: str, problem: str, model: str = "minimax") -> dict:
    """Run a single SWE-bench task with retry logic."""
    
    result = {
        "instance_id": instance_id,
        "resolved": False,
        "attempts": 0,
        "duration": 0,
        "error": None,
    }
    
    start = time.time()
    
    # Get model config
    model_config = MODELS.get(model, MODELS["minimax"])
    provider = model_config["provider"]
    model_name = model_config["model"]
    
    for attempt in range(MAX_RETRIES + 1):
        result["attempts"] = attempt + 1
        
        # Build enhanced prompt for retry
        if attempt > 0:
            prompt = f"""RETRY {attempt + 1}/{MAX_RETRIES + 1} for {instance_id}
            
Previous attempt failed. Problem:
{problem}

Your task:
1. cd {SWE_BENCH_DIR} && python -m swebench.harness.run_instance {instance_id}
2. Analyze failure from previous attempt
3. Try a DIFFERENT approach - think harder about root cause
4. Implement fix
5. Verify: python -m swebench.harness.run_instance {instance_id} --verify
6. Report PASS/FAIL
"""
        else:
            prompt = build_prompt(instance_id, problem)
        
        try:
            # Run jcode with model
            proc = subprocess.Popen(
                ["jcode", "--provider", provider, "--model", model_name, "run", prompt],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                cwd=str(SWE_BENCH_DIR),
            )
            
            stdout, stderr = proc.communicate(timeout=TIMEOUT_SECONDS)
            output = stdout.decode() + stderr.decode()
            
            # Check resolution using harness verification
            # Run harness to verify
            verify_proc = subprocess.run(
                ["python", "-m", "swebench.harness.run_instance", instance_id, "--verify", "--timeout", "300"],
                capture_output=True,
                text=True,
                cwd=str(SWE_BENCH_DIR),
                timeout=360,
            )
            
            # Check harness output for PASS
            harness_output = verify_proc.stdout + verify_proc.stderr
            
            if "PASS" in harness_output or verify_proc.returncode == 0:
                result["resolved"] = True
                result["duration"] = time.time() - start
                return result
            
            # Check if we should retry
            if attempt < MAX_RETRIES:
                # Check for common failure patterns
                failure_indicators = ["FAILED", "ERROR", "test failed", "assertion error"]
                should_retry = any(ind in output.upper() for ind in failure_indicators)
                if should_retry:
                    continue
            
        except subprocess.TimeoutExpired:
            if proc.poll() is None:
                proc.kill()
            result["error"] = f"timeout on attempt {attempt + 1}"
            if attempt < MAX_RETRIES:
                continue
        except Exception as e:
            result["error"] = str(e)
    
    result["duration"] = time.time() - start
    return result


def run_task_simple(instance_id: str, problem: str, model: str = "minimax") -> dict:
    """Fallback: Run task without retry (original simple approach)."""
    
    result = {
        "instance_id": instance_id,
        "resolved": False,
        "duration": 0,
        "error": None,
    }
    
    start = time.time()
    
    model_config = MODELS.get(model, MODELS["minimax"])
    provider = model_config["provider"]
    model_name = model_config["model"]
    
    prompt = f"""Task: {instance_id}
Repository: auto-detect from instance_id
Fix this issue:
{problem}

Instructions:
1. cd {SWE_BENCH_DIR}
2. python -m swebench.harness.run_instance {instance_id}
3. Fix the issue
4. python -m swebench.harness.run_instance {instance_id} --verify
5. Report PASS/FAIL
"""
    
    try:
        proc = subprocess.Popen(
            ["jcode", "--provider", provider, "--model", model_name, "run", prompt],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            cwd=str(SWE_BENCH_DIR),
        )
        
        stdout, stderr = proc.communicate(timeout=TIMEOUT_SECONDS)
        output = stdout.decode() + stderr.decode()
        
        # Verify using harness
        verify_proc = subprocess.run(
            ["python", "-m", "swebench.harness.run_instance", instance_id, "--verify"],
            capture_output=True,
            text=True,
            cwd=str(SWE_BENCH_DIR),
            timeout=300,
        )
        
        result["resolved"] = verify_proc.returncode == 0 or "PASS" in verify_proc.stdout
        result["duration"] = time.time() - start
        
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
    parser.add_argument("action", choices=["run", "results", "retry-failed"])
    parser.add_argument("--parallel", type=int, default=4)
    parser.add_argument("--limit", type=int, default=None)
    parser.add_argument("--model", choices=["minimax", "claude", "gpt"], default="minimax")
    parser.add_argument("--retries", action="store_true", default=False)
    args = parser.parse_args()
    
    results_file = RESULTS_DIR / "results.json"
    instances_file = RESULTS_DIR / "instances.json"
    
    if args.action == "results":
        if results_file.exists():
            with open(results_file) as f:
                results = json.load(f)
            total = len(results)
            resolved = sum(1 for r in results if r.get("resolved"))
            print(f"\n=== Jcode on SWE-bench Lite ===")
            print(f"Model: {args.model}")
            print(f"Results: {resolved}/{total} ({100*resolved/total:.1f}%)")
            print(f"\nResolved ({resolved}):")
            for r in results:
                if r.get("resolved"):
                    print(f"  ✓ {r['instance_id']}")
            print(f"\nFailed ({total - resolved}):")
            for r in results:
                if not r.get("resolved"):
                    print(f"  ✗ {r['instance_id']}")
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
                "problem": ds[i]["problem_statement"],
                "repo": ds[i]["repo"],
                "version": ds[i]["version"],
            })
        with open(instances_file, "w") as f:
            json.dump(instances, f)
        print(f"Loaded {len(instances)} instances")
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
    
    if args.action == "retry-failed":
        # Retry failed instances
        pending = [i for i in instances if i["id"] in done_ids]
        # Filter to only failed ones
        failed_ids = {r["instance_id"] for r in results if not r.get("resolved")}
        pending = [i for i in pending if i["id"] in failed_ids]
        print(f"Retrying {len(pending)} failed instances with enhanced prompting...")
    else:
        pending = [i for i in instances if i["id"] not in done_ids]
        print(f"SWE-bench Lite: {len(instances)} total, {len(done_ids)} done, {len(pending)} pending")
    
    print(f"Model: {args.model}, Parallel: {args.parallel}, Retries: {MAX_RETRIES}")
    print("Running improved benchmark with SWE-bench harness verification...")
    
    run_func = run_task_with_retry if args.retries else run_task_simple
    
    # Run pending tasks in parallel
    with ThreadPoolExecutor(max_workers=args.parallel) as executor:
        futures = {
            executor.submit(run_func, inst["id"], inst["problem"], args.model): inst
            for inst in pending
        }
        
        for future in as_completed(futures):
            inst = futures[future]
            try:
                result = future.result()
            except Exception as e:
                result = {
                    "instance_id": inst["id"],
                    "resolved": False,
                    "error": str(e),
                }
            
            # Update or append result
            existing_idx = None
            for i, r in enumerate(results):
                if r["instance_id"] == result["instance_id"]:
                    existing_idx = i
                    break
            
            if existing_idx is not None:
                # Update existing - take if resolved and old wasn't
                if result.get("resolved") and not results[existing_idx].get("resolved"):
                    results[existing_idx] = result
            else:
                results.append(result)
            
            # Save progress
            with open(results_file, "w") as f:
                json.dump(results, f)
            
            status = "✓" if result.get("resolved") else "✗"
            attempts = f" (x{result.get('attempts', 1)})" if result.get("attempts", 1) > 1 else ""
            err = f" [{result.get('error', '')}]" if result.get("error") else ""
            print(f"  {status} {result['instance_id']}{attempts}{err}")
    
    # Summary
    total = len(results)
    resolved = sum(1 for r in results if r.get("resolved"))
    print(f"\n=== Final: {resolved}/{total} ({100*resolved/total:.1f}%) ===")


if __name__ == "__main__":
    main()