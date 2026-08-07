#!/usr/bin/env python3
"""Deterministic verifier for zero-overlap-anomaly.

Measures whether the recall pipeline retrieves targets whose full content shares
ZERO >=4-char tokens with the query — retrieval a purely lexical encoder cannot do.

Infrastructure guards: snapshot hash pinned; the zero-overlap invariant itself is
recomputed here (same tokenization as the probe generator) so a probe edit that
breaks the premise can never be scored as if it held.

Exit codes: 0 = scored, 3 = infrastructure error (no reward).
"""
import argparse
import hashlib
import json
import math
import os
import re
import sys

GATE_MIN_HITS = 1  # anomaly-present gate: >=1 zero-overlap hit in top-10.
# Live-analysis reference: 2/8 original probes hit (p31@3, p43@6) in baseline-002.
# Calibrate a tighter gate after the first frozen run of this 33-probe set.

TOKEN = re.compile(r"[a-z0-9]{4,}")


def toks(s):
    return set(TOKEN.findall(s.lower()))


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
    ap.add_argument("--targets", default="/tests/targets.json")
    ap.add_argument("--store", default="/environment/data/kannaka.hrm")
    ap.add_argument("--pinned-sha", default="/tests/SNAPSHOT.sha256")
    ap.add_argument("--metrics-out", default="/logs/verifier/metrics.json")
    ap.add_argument("--reward-out", default="/logs/verifier/reward.txt")
    ap.add_argument("--skip-store-check", action="store_true")
    args = ap.parse_args()

    if not os.path.exists(args.rollout):
        infra_fail(f"rollout missing: {args.rollout}")
    with open(args.rollout) as f:
        rollout = json.load(f)
    with open(args.expected) as f:
        expected = json.load(f)
    with open(args.targets) as f:
        targets = json.load(f)

    if not args.skip_store_check:
        with open(args.pinned_sha) as f:
            pinned = f.read().split()[0].strip()
        if sha256(args.store) != pinned:
            infra_fail("pristine snapshot hash mismatch — different corpus, different eval")
        if rollout.get("store_copy_sha256") != pinned:
            infra_fail("adapter ran against a store copy that does not match the pinned snapshot")

    results = {r["id"]: r for r in rollout.get("results", [])}
    missing = [pid for pid in expected if pid not in results]
    unparsed = [pid for pid, r in results.items() if not r.get("parse_ok")]
    if missing:
        infra_fail(f"{len(missing)} probes absent from rollout: {missing[:5]}")
    if unparsed:
        infra_fail(f"{len(unparsed)} probes unparseable: {unparsed[:5]}")

    # ── zero-overlap invariant (the premise of the whole eval) ───────────────
    for pid, t in targets.items():
        q = toks(results[pid]["query"])
        overlap = sorted(w for w in q if any(w in toks(c) for c in t["contents"]))
        if overlap:
            infra_fail(f"zero-overlap invariant broken for {pid}: shares {overlap}")

    # ── metrics ──────────────────────────────────────────────────────────────
    per_probe, recalls, mrrs, ndcgs = [], [], [], []
    for pid, expect in expected.items():
        relevant = set(expect)
        ids = [item.get("id") for item in results[pid]["results"]][:10]
        rank = next((i + 1 for i, x in enumerate(ids) if x in relevant), 0)
        hits = sum(1 for x in ids if x in relevant)
        recall = hits / len(relevant) if relevant else 0.0
        dcg = sum(1.0 / math.log2(i + 2) for i, x in enumerate(ids) if x in relevant)
        idcg = sum(1.0 / math.log2(i + 2) for i in range(min(len(relevant), 10)))
        recalls.append(recall)
        mrrs.append(1.0 / rank if rank else 0.0)
        ndcgs.append(dcg / idcg if idcg else 0.0)
        per_probe.append({"probe": pid, "kind": targets[pid]["kind"],
                          "source_probe": targets[pid]["source_probe"], "first_rank": rank})

    mean = lambda xs: sum(xs) / len(xs) if xs else 0.0
    hit_list = [p for p in per_probe if p["first_rank"]]
    metrics = {
        "recall_at_10": round(mean(recalls), 4),
        "mrr": round(mean(mrrs), 4),
        "ndcg_at_10": round(mean(ndcgs), 4),
        "probes": len(per_probe),
        "hits": len(hit_list),
        "hit_detail": [{"probe": p["probe"], "rank": p["first_rank"], "kind": p["kind"]} for p in hit_list],
        "gate_min_hits": GATE_MIN_HITS,
        "gate_passed": len(hit_list) >= GATE_MIN_HITS,
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

    print(f"recall@10={metrics['recall_at_10']}  hits={metrics['hits']}/{metrics['probes']}  "
          f"anomaly {'PRESENT' if metrics['gate_passed'] else 'ABSENT'} "
          f"(gate: >={GATE_MIN_HITS} zero-overlap hit)  detail={metrics['hit_detail']}")
    sys.exit(0)


if __name__ == "__main__":
    main()
