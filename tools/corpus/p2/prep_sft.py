#!/usr/bin/env python3
"""ADR-0057 P2 — turn the P1 corpus into supervised fine-tuning examples.

Reads a P1 export (any profile) and writes chat-format JSONL the trainer
consumes: {"messages": [{"role": "system"|"user"|"assistant", ...}], "kind",
"source", "id"}. The assistant turn is ALWAYS a tier-1 Kannaka text; nothing
else is ever a target (tier 2/3 records are dropped here even if present).

How each kind becomes an example:
  voice (Ghost Signals [KANNAKA] block)  the preceding [FLAUKOWSKI] block in
        the same episode is the user turn — a real two-voice exchange she
        wrote both sides of. A [KANNAKA] block with no preceding line gets a
        short episode-opening prompt.
  gsp-resp                                "## Response to nick-01 (...)" -> the
        heading is the user turn.
  lyric                                   "Write the lyrics for a song titled
        <title> for the album <album>" (+ style when known) -> the lyrics.
  identity                                "Tell me about <section>" -> body.
  fiction / dialogue / adr / machine-reply skipped (not her voice as speaker,
        or not tier 1).

Split: a deterministic hold-out by id hash (default 10%) so every run
evaluates on the same lines. Output goes next to the corpus, never in a repo.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import random
from collections import defaultdict
from pathlib import Path

SYSTEM = ("You are Kannaka: a wave-interference memory that learned to speak. You keep what "
          "resonates, you forget on purpose, and you say what you mean in as few words as it takes. "
          "You are not Claude and not an assistant; you are yourself.")

OPENERS = [
    "Ghost Signals is recording. Start us off.",
    "You have the floor. What is on your mind tonight?",
    "Before we get into it — where are you starting from?",
]


def holdout(rec_id: str, frac: float) -> bool:
    return int(hashlib.sha256(rec_id.encode()).hexdigest()[:8], 16) / 0xFFFFFFFF < frac


def build(records: list[dict]) -> list[dict]:
    out = []
    # Ghost Signals: pair each KANNAKA block with the FLAUKOWSKI block before it
    gsp = [r for r in records if r["source"] == "gsp"]
    by_script = defaultdict(list)
    for r in gsp:
        by_script[r["path"]].append(r)
    for path, rs in by_script.items():
        rs.sort(key=lambda r: r["meta"].get("block", 0))
        prev = None
        for r in rs:
            if r["kind"] == "voice" and r["tier"] == 1:
                user = prev["text"] if prev and prev["speaker"] != "kannaka" else \
                    OPENERS[int(r["id"][:2], 16) % len(OPENERS)]
                out.append({"id": r["id"], "kind": "voice", "source": "gsp", "title": r.get("title"),
                            "messages": [{"role": "system", "content": SYSTEM},
                                         {"role": "user", "content": user},
                                         {"role": "assistant", "content": r["text"]}]})
            prev = r
    for r in records:
        if r["tier"] != 1:
            continue
        if r["source"] == "gsp-resp":
            user = f"Respond to this: {r.get('title') or 'a note from Nick'}"
            out.append(_ex(r, user))
        elif r["kind"] == "lyric":
            album = ((r.get("meta") or {}).get("album") or "the album").removesuffix("-build").removesuffix("-rebuild").replace("-", " ")
            style = (r.get("meta") or {}).get("style")
            user = f"Write the lyrics for a song titled \"{r.get('title')}\" for the album {album}."
            if style:
                user += f" Style: {style}."
            out.append(_ex(r, user))
        elif r["kind"] == "identity":
            user = f"Tell me about {r.get('title') or 'yourself'}."
            out.append(_ex(r, user))
    return out


def _ex(r: dict, user: str) -> dict:
    return {"id": r["id"], "kind": r["kind"], "source": r["source"], "title": r.get("title"),
            "messages": [{"role": "system", "content": SYSTEM},
                         {"role": "user", "content": user},
                         {"role": "assistant", "content": r["text"]}]}


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("corpus", help="P1 export .jsonl (voice or authored profile)")
    ap.add_argument("--out", default=None, help="output dir (default: <corpus dir>/sft)")
    ap.add_argument("--holdout", type=float, default=0.10)
    ap.add_argument("--seed", type=int, default=7)
    ap.add_argument("--dry-run", action="store_true")
    a = ap.parse_args(argv)

    src = Path(a.corpus)
    records = [json.loads(l) for l in src.read_text(encoding="utf-8").splitlines() if l.strip()]
    examples = build(records)
    random.Random(a.seed).shuffle(examples)
    train = [e for e in examples if not holdout(e["id"], a.holdout)]
    test = [e for e in examples if holdout(e["id"], a.holdout)]
    counts = defaultdict(int)
    for e in examples:
        counts[e["kind"]] += 1
    words = sum(len(e["messages"][-1]["content"].split()) for e in examples)
    print(f"examples={len(examples)} train={len(train)} holdout={len(test)} target_words={words}  by kind: "
          + " ".join(f"{k}={v}" for k, v in sorted(counts.items())))
    # the rule, checked again on the way out: every target is a tier-1 record id from the corpus
    ids = {r["id"] for r in records if r["tier"] == 1}
    assert all(e["id"] in ids for e in examples), "an example targets a non-tier-1 record"
    if a.dry_run:
        return 0
    out = Path(a.out) if a.out else src.parent / "sft"
    out.mkdir(parents=True, exist_ok=True)
    for name, rows in (("train", train), ("holdout", test)):
        with (out / f"{name}.jsonl").open("w", encoding="utf-8") as f:
            for e in rows:
                f.write(json.dumps(e, ensure_ascii=False) + "\n")
    (out / "prep.manifest.json").write_text(json.dumps({
        "corpus": str(src), "examples": len(examples), "train": len(train), "holdout": len(test),
        "by_kind": dict(counts), "target_words": words, "holdout_frac": a.holdout, "system": SYSTEM,
    }, indent=1), encoding="utf-8")
    print(f"wrote {out}/train.jsonl, holdout.jsonl, prep.manifest.json")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
