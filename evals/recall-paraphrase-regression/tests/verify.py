#!/usr/bin/env python3
"""Deterministic verifier for recall-paraphrase-regression.

Recomputes recall@10 / MRR / nDCG@10 from the raw per-probe result arrays in the
rollout against the verifier-side expected UUID sets. Never trusts adapter-computed
scores (there are none) and never reuses environment success helpers.

Exit codes: 0 = scored (reward written; gate verdict in metrics.json),
            3 = infrastructure error (no reward written).
"""
import argparse
import hashlib
import json
import math
import os
import sys

GATE = 0.20  # calibrated 2026-08-01: first frozen run (baseline-002) scored 0.24 = 12/50 hits;
# gate = baseline minus two probes of headroom for temporal-term clock drift.


def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def infra_fail(msg):
    print(f"INFRASTRUCTURE ERROR: {msg}", file=sys.stderr)
    sys.exit(3)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--rollout", default="/app/rollout.json")
    ap.add_argument("--expected", default="/tests/expected.json")
    ap.add_argument("--store", default="/environment/data/kannaka.hrm")
    ap.add_argument("--pinned-sha", default="/tests/SNAPSHOT.sha256")
    ap.add_argument("--metrics-out", default="/logs/verifier/metrics.json")
    ap.add_argument("--reward-out", default="/logs/verifier/reward.txt")
    ap.add_argument("--skip-store-check", action="store_true", help="fixture calibration only")
    args = ap.parse_args()

    if not os.path.exists(args.rollout):
        infra_fail(f"rollout missing: {args.rollout}")
    with open(args.rollout) as f:
        rollout = json.load(f)
    with open(args.expected) as f:
        expected = json.load(f)

    # ── infrastructure guards ────────────────────────────────────────────────
    if not args.skip_store_check:
        with open(args.pinned_sha) as f:
            pinned = f.read().split()[0].strip()
        actual = sha256(args.store)
        if actual != pinned:
            infra_fail(f"pristine snapshot hash mismatch: {actual} != pinned {pinned} — different corpus, different eval")
        if rollout.get("store_copy_sha256") != pinned:
            infra_fail("adapter ran against a store copy that does not match the pinned snapshot")

    results = {r["id"]: r for r in rollout.get("results", [])}
    missing = [pid for pid in expected if pid not in results]
    unparsed = [pid for pid, r in results.items() if not r.get("parse_ok")]
    if missing:
        infra_fail(f"{len(missing)} probes absent from rollout: {missing[:5]}")
    if unparsed:
        infra_fail(f"{len(unparsed)} probes produced unparseable output: {unparsed[:5]}")

    # ── metrics (same definitions as scripts/recall-harness.mjs) ─────────────
    per_probe, recalls, mrrs, ndcgs = [], [], [], []
    for pid, expect in expected.items():
        relevant = set(expect)
        ids = [item.get("id") for item in results[pid]["results"]][:10]
        rank = next((i + 1 for i, x in enumerate(ids) if x in relevant), 0)
        hits = sum(1 for x in ids if x in relevant)
        recall = hits / len(relevant) if relevant else 0.0
        mrr = 1.0 / rank if rank else 0.0
        dcg = sum(1.0 / math.log2(i + 2) for i, x in enumerate(ids) if x in relevant)
        idcg = sum(1.0 / math.log2(i + 2) for i in range(min(len(relevant), 10)))
        ndcg = dcg / idcg if idcg else 0.0
        recalls.append(recall); mrrs.append(mrr); ndcgs.append(ndcg)
        per_probe.append({"probe": pid, "first_rank": rank, "recall_at_10": recall})

    mean = lambda xs: sum(xs) / len(xs) if xs else 0.0
    metrics = {
        "recall_at_10": round(mean(recalls), 4),
        "mrr": round(mean(mrrs), 4),
        "ndcg_at_10": round(mean(ndcgs), 4),
        "probes": len(per_probe),
        "hits": sum(1 for p in per_probe if p["first_rank"]),
        "gate": GATE,
        "gate_passed": mean(recalls) >= GATE,
        "binary": rollout.get("binary"),
        "knobs": rollout.get("knobs"),
        "run_started": rollout.get("started"),
        "per_probe": per_probe,
    }

    os.makedirs(os.path.dirname(args.metrics_out), exist_ok=True)
    with open(args.metrics_out, "w") as f:
        json.dump(metrics, f, indent=1)
    with open(args.reward_out, "w") as f:
        f.write(f"{metrics['recall_at_10']}\n")

    print(f"recall@10={metrics['recall_at_10']}  mrr={metrics['mrr']}  ndcg@10={metrics['ndcg_at_10']}  "
          f"hits={metrics['hits']}/{metrics['probes']}  gate({GATE}) {'PASSED' if metrics['gate_passed'] else 'FAILED'}")
    sys.exit(0)


if __name__ == "__main__":
    main()
