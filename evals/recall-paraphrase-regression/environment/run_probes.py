#!/usr/bin/env python3
"""Probe driver (Harness adapter). Runs the real `kannaka recall` CLI once per probe
against a per-trial copy of the frozen store and records raw outputs + provenance.

Contains no expected UUIDs and makes no scoring decisions.
"""
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
from datetime import datetime, timezone

BIN = "/environment/kannaka"
DATA_SRC = "/environment/data"
PROBES = "/environment/probes.json"
OUT = "/app/rollout.json"

# Production knob configuration. Set explicitly, never inherited (recall-harness rule 3).
KNOBS = {"KANNAKA_RECALL_ENERGY_EXP": "0.0", "KANNAKA_RECALL_TEMPORAL_EXP": "1.0"}


def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def main():
    probes = json.load(open(PROBES))

    # Per-trial store copy: recall's record_retrieval() may persist, so the pristine
    # snapshot under /environment is never opened by the CLI.
    work = tempfile.mkdtemp(prefix="kannaka-eval-")
    hrm_copy = os.path.join(work, "kannaka.hrm")
    shutil.copy(os.path.join(DATA_SRC, "kannaka.hrm"), hrm_copy)
    cfg = open(os.path.join(DATA_SRC, "config.toml")).read().replace("__HRM_PATH__", hrm_copy)
    with open(os.path.join(work, "config.toml"), "w") as f:
        f.write(cfg)

    env = {**os.environ, **KNOBS, "KANNAKA_DATA_DIR": work}

    try:
        version = subprocess.run([BIN, "--version"], capture_output=True, text=True, timeout=60).stdout.strip().splitlines()[0]
    except Exception as e:  # provenance stays honest rather than failing the run
        version = f"unknown ({e})"

    rollout = {
        "started": datetime.now(timezone.utc).isoformat(),
        "binary": {"path": BIN, "sha256": sha256(BIN), "version": version},
        "store_copy_sha256": sha256(os.path.join(work, "kannaka.hrm")),
        "knobs": KNOBS,
        "results": [],
    }

    # Optional facet-on arm: FACET_BACKFILL=1 decomposes the per-trial store COPY
    # via the eval-side driver (public backfill_facets API; no CLI exists yet).
    # Runs after the copy hash is recorded — the pristine snapshot is never touched.
    if os.environ.get("FACET_BACKFILL") == "1":
        bf = subprocess.run(
            ["/environment/facet-backfill"],
            capture_output=True, text=True, timeout=600, env=env,
        )
        if bf.returncode != 0:
            print(f"BACKFILL FAILED: {bf.stderr[-500:]}", file=sys.stderr)
            sys.exit(1)
        rollout["backfill"] = json.loads(bf.stdout.strip().splitlines()[-1])
        print(f"backfill: {rollout['backfill']}", file=sys.stderr)

    for p in probes:
        rec = {"id": p["id"], "query": p["query"]}
        try:
            proc = subprocess.run(
                [BIN, "recall", p["query"], "--top-k", "10"],
                capture_output=True, text=True, timeout=300, env=env,
            )
            rec["exit_code"] = proc.returncode
            rec["stderr_tail"] = proc.stderr[-500:]
            try:
                parsed = json.loads(proc.stdout)
                rec["results"] = parsed if isinstance(parsed, list) else []
                rec["parse_ok"] = isinstance(parsed, list)
            except json.JSONDecodeError:
                rec["results"] = []
                rec["parse_ok"] = False
                rec["stdout_tail"] = proc.stdout[-500:]
        except subprocess.TimeoutExpired:
            rec.update(exit_code=-1, parse_ok=False, results=[], stderr_tail="TIMEOUT")
        rollout["results"].append(rec)
        print(f"{p['id']}: {'ok' if rec['parse_ok'] else 'PARSE-FAIL'} ({len(rec['results'])} results)", file=sys.stderr)

    rollout["finished"] = datetime.now(timezone.utc).isoformat()
    with open(OUT, "w") as f:
        json.dump(rollout, f, indent=1)
    print(f"wrote {OUT} ({len(rollout['results'])} probes)")


if __name__ == "__main__":
    main()
