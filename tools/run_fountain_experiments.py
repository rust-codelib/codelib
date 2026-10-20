#!/usr/bin/env python3
"""Run a finite grid, retain raw CSV/metadata, summarize without false CIs.

Uses only the standard library and the Rust example. Rust never reads a lost
part; all comparisons with the original are in the experiment checker.
"""
import argparse
import collections
import csv
import datetime
import itertools
import hashlib
import platform
import json
import os
from pathlib import Path
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]


def summarize(rows):
    per_instance = collections.defaultdict(list)
    for row in rows:
        per_instance[row["instance"]].append(row)
    successful = [r for r in rows if r["repair_ok"] == "true"]
    histogram = lambda values: dict(sorted(collections.Counter(values).items()))
    return {
        "observations": len(rows),
        "instances": len(per_instance),
        "decode_success": sum(r["decode_ok"] == "true" for r in rows),
        "insufficient_rank": sum(int(r["rank"]) < int(r["k"]) for r in rows),
        "local_success": len(successful),
        "first_complete_histogram": histogram(r["first_complete"] or "incomplete" for r in rows),
        "read_symbols_histogram_successful_repairs": histogram(r["read_symbols"] for r in successful),
        "read_payload_bytes_total": sum(int(r["read_payload_bytes"]) for r in rows),
        "greedy_group_count_histogram": histogram(r["disjoint_lower_bound"] for r in rows),
        "decode_nanoseconds_total": sum(int(r["decode_ns"]) for r in rows),
        "repair_nanoseconds_total": sum(int(r["repair_ns"]) for r in rows),
        "decoder_logical_bytes_max": max((int(r["decoder_logical_bytes"]) for r in rows), default=0),
        "payload_to_source_ratio_mean": sum(int(r["payload_bytes"]) / int(r["n"]) for r in rows) / len(rows) if rows else None,
        "wire_to_source_ratio_mean": sum(int(r["wire_bytes"]) / int(r["n"]) for r in rows) / len(rows) if rows else None,
        "per_instance": {key: {
            "observations": len(group),
            "code_seed": group[0]["code_seed"],
            "decode_success": sum(r["decode_ok"] == "true" for r in group),
            "repair_success": sum(r["repair_ok"] == "true" for r in group),
        } for key, group in per_instance.items()},
        "confidence_intervals": "Not estimated; trials are clustered by code instance. No article-scale statistical claim.",
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--paper", action="store_true", help="K=100,300,500, c=4,6; default: smoke K=12 c=4")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--instances", type=int, default=2)
    parser.add_argument("--trials", type=int, default=3)
    parser.add_argument("--seed", type=int, default=42)
    parser.add_argument("--epsilon", default="0.1")
    parser.add_argument("--loss", default="0.4")
    parser.add_argument("--repair-loss", default="0.1")
    parser.add_argument("--max-observations", type=int, default=100)
    parser.add_argument("--max-work", type=int, default=10**9)
    parser.add_argument("--time-budget-seconds", type=int, default=60, help="entire grid, excluding the initial build")
    parser.add_argument("--toolchain", default="stable", choices=["stable", "1.81.0"])
    args = parser.parse_args()
    if min(args.instances, args.trials, args.max_observations, args.max_work, args.time_budget_seconds) <= 0:
        parser.error("budgets and observation counts must be positive")
    if args.output.exists() and any(args.output.iterdir()):
        parser.error("output must be new or empty, to preserve earlier measurements")
    profiles = ["gf256", "gf65536"]
    ks = [100, 300, 500] if args.paper else [12]
    cs = [4, 6] if args.paper else [4]
    cases = list(itertools.product(profiles, ks, cs, ["sample", "loss"]))
    if len(cases) * args.instances * args.trials > args.max_observations:
        parser.error("total observation budget exceeded before starting")
    env = os.environ.copy()
    # Reuse existing temporary toolchains when ordinary PATH lacks Cargo.
    if Path("/tmp/codelib-cargo/bin/cargo").is_file():
        env["PATH"] = "/tmp/codelib-cargo/bin:" + env.get("PATH", "")
        env["CARGO_HOME"] = "/tmp/codelib-cargo"
        env["RUSTUP_HOME"] = "/tmp/codelib-rustup"
    subprocess.run(["cargo", "+" + args.toolchain, "build", "-p", "fountain-codes", "--example", "experiment", "--release", "--offline"], cwd=ROOT, env=env, check=True)
    executable = Path(env.get("CARGO_TARGET_DIR", str(ROOT / "target"))) / "release/examples/experiment"
    args.output.mkdir(parents=True, exist_ok=True)
    manifest = {
        "created_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "parameters": {k: str(v) if isinstance(v, Path) else v for k, v in vars(args).items()},
        "toolchain": subprocess.check_output(["rustc", "+" + args.toolchain, "--version"], env=env, text=True).strip(),
        "platform": platform.platform(),
        "source_sha256": {str(p.relative_to(ROOT)): hashlib.sha256(p.read_bytes()).hexdigest()
            for p in sorted([p for name in ("fountain-codes", "coding-core", "gf-linalg", "gf2m", "gfpm") for p in (ROOT / name).rglob("*.rs")] + [ROOT / "fountain-codes/Cargo.toml", ROOT / "fountain-codes/README.md", ROOT / "Cargo.toml", ROOT / "Cargo.lock", Path(__file__), ROOT / "docs/fixtures/fountain-v1.json"])},
        "source_git_head": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
        "working_tree": subprocess.check_output(["git", "status", "--short"], cwd=ROOT, text=True),
        "article_scale": False,
        "note": "Configured counts are recorded explicitly. A paper grid is not a claim of reproducing all article statistics. Timing is machine-dependent; memory is a logical budget, not RSS.",
        "cases": [],
    }
    started = time.monotonic()
    failed = False
    for profile, k, c, mode in cases:
        remaining = args.time_budget_seconds - (time.monotonic() - started)
        if remaining < 1:
            failed = True
            manifest["stopped"] = "total grid time budget exhausted"
            break
        name = f"{profile}-k{k}-c{c}-{mode}"
        command = [str(executable), "--profile", profile, "--k", str(k), "--c", str(c), "--mode", mode,
                   "--instances", str(args.instances), "--trials", str(args.trials), "--seed", str(args.seed),
                   "--epsilon", args.epsilon, "--loss", args.loss, "--repair-loss", args.repair_loss,
                   "--max-observations", str(args.max_observations), "--max-work", str(args.max_work // len(cases)),
                   "--time-budget-seconds", str(max(1, int(remaining)))]
        raw = args.output / (name + ".csv")
        log = args.output / (name + ".log")
        with raw.open("w") as out, log.open("w") as err:
            try:
                result = subprocess.run(command, cwd=ROOT, env=env, stdout=out, stderr=err, timeout=remaining)
                status = result.returncode
            except subprocess.TimeoutExpired:
                status = "timeout"
        with raw.open() as stream:
            # A timeout can leave a partial final row; keep only complete rows.
            rows = [r for r in csv.DictReader(stream) if None not in r and all(v is not None for v in r.values())]
        summary = summarize(rows)
        manifest["cases"].append({"name": name, "command": command, "status": status, "raw": raw.name, "log": log.name, "summary": summary})
        print(f"{name}: observations={len(rows)}, decode={summary['decode_success']}, local={summary['local_success']}, status={status}")
        if status != 0:
            failed = True
            break
    manifest["elapsed_seconds"] = time.monotonic() - started
    manifest["completed"] = not failed
    (args.output / "manifest.json").write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
