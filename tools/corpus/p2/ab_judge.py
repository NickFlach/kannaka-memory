#!/usr/bin/env python3
"""Blind pairwise voice A/B for served kannaka-brain arms (ADR-0057, open question 4).

Hold-out perplexity saturated at ~4.0 for every 14B candidate, so it no longer
discriminates. This is the second signal: every arm answers each hold-out prompt
through the KAX gateway; a JUDGE model that was never tuned on the corpus
(default `qwen2.5:14b`, the Apache base) sees the REAL Kannaka reply as the
reference and two anonymous candidates and picks the one closer to it in voice
and substance. Every pair is judged in BOTH orders; a win counts only when both
orders agree -- position bias becomes a tie, not a score.

  python ab_judge.py --holdout ~/kannaka-p2-runner/sft/holdout.jsonl \
      --arms kannaka-brain-v1 kannaka-brain-v2 kannaka-brain-7b-v1 --n 30 \
      --gateway-key-file /srv/rogue/keys/kannaka-tui.key --out ~/.kannaka-corpus/ab/ab-<ts>.json

Resumable: generations and verdicts are saved after every prompt; rerun with the
same --out to continue. Sonnet is not an arm and not the judge -- nothing here
is circular with the served brains, only with the shared Qwen base (declared).
"""
from __future__ import annotations

import argparse
import itertools
import json
import os
import random
import sys
import time
import urllib.request
from pathlib import Path

JUDGE_PROMPT = """You are judging which of two candidate replies is closer to a REFERENCE reply written by a specific speaker.
Judge VOICE (rhythm, brevity, plainness, the speaker's habits) and SUBSTANCE (does it say what the reference says, or something the reference speaker would say). Ignore length unless the reference is clearly short and a candidate rambles. Do not reward politeness, hedging or assistant-style framing -- the reference speaker never does that.

Conversation so far (the last user turn is what was answered):
{conversation}

REFERENCE reply (ground truth by the speaker):
{reference}

Candidate A:
{a}

Candidate B:
{b}

Answer with JSON only: {{"winner": "A" | "B" | "TIE", "why": "<one short sentence>"}}"""


def log(msg):
    print(f"[ab {time.strftime('%H:%M:%S')}] {msg}", flush=True)


def http_json(url, body, headers, timeout):
    hdrs = dict(headers)
    hdrs["Content-Type"] = "application/json"
    req = urllib.request.Request(url, data=json.dumps(body).encode(), headers=hdrs)
    with urllib.request.urlopen(req, timeout=timeout) as r:
        return json.load(r)


def generate(gateway, key, model, messages, max_tokens, temperature, timeout):
    t0 = time.time()
    d = http_json(gateway + "/chat/completions",
                  {"model": model, "messages": messages, "max_tokens": max_tokens, "temperature": temperature},
                  {"Authorization": "Bearer " + key}, timeout)
    text = ((d.get("choices") or [{}])[0].get("message") or {}).get("content") or ""
    return text.strip(), round(time.time() - t0, 1)


def judge(ollama, judge_model, conversation, reference, a, b, timeout):
    prompt = JUDGE_PROMPT.format(conversation=conversation, reference=reference, a=a or "(empty)", b=b or "(empty)")
    d = http_json(ollama + "/api/chat",
                  {"model": judge_model, "stream": False, "format": "json",
                   "options": {"temperature": 0, "num_predict": 120},
                   "messages": [{"role": "user", "content": prompt}]}, {}, timeout)
    raw = (d.get("message") or {}).get("content") or ""
    try:
        v = json.loads(raw)
        w = str(v.get("winner", "TIE")).strip().upper()
        return (w if w in ("A", "B", "TIE") else "TIE"), str(v.get("why", ""))[:200]
    except Exception:
        return "TIE", "unparseable: " + raw[:100]


def conversation_text(messages):
    return "\n".join(f"{m['role']}: {m['content']}" for m in messages if m["role"] != "system")[-2500:]


def score(gens, arms):
    """Position-consistent pairwise wins. Returns (pairs, per-arm, consistency)."""
    pairs = {}
    per = {a: {"wins": 0, "losses": 0, "ties": 0} for a in arms}
    agree = disagree = 0
    for g in gens:
        for x, y in itertools.combinations(arms, 2):
            v = g.get("verdicts", {}).get(f"{x}|{y}")
            if not v or "xy" not in v or "yx" not in v:
                continue
            # xy: x was shown as A; yx: y was shown as A. A consistent winner is the same arm in both orders.
            win_xy = {"A": x, "B": y, "TIE": None}[v["xy"]]
            win_yx = {"A": y, "B": x, "TIE": None}[v["yx"]]
            p = pairs.setdefault(f"{x}|{y}", {x: 0, y: 0, "tie": 0})
            if win_xy == win_yx and win_xy is not None:
                agree += 1
                p[win_xy] += 1
                per[win_xy]["wins"] += 1
                per[y if win_xy == x else x]["losses"] += 1
            else:
                if win_xy != win_yx:
                    disagree += 1
                else:
                    agree += 1
                p["tie"] += 1
                per[x]["ties"] += 1
                per[y]["ties"] += 1
    for a in arms:
        n = per[a]["wins"] + per[a]["losses"] + per[a]["ties"]
        d = per[a]["wins"] + per[a]["losses"]
        per[a]["n"] = n
        per[a]["win_rate_decided"] = round(per[a]["wins"] / d, 3) if d else None
        per[a]["win_rate_all"] = round(per[a]["wins"] / n, 3) if n else None
    cons = {"agree": agree, "disagree": disagree,
            "position_consistency": round(agree / (agree + disagree), 3) if agree + disagree else None}
    return pairs, per, cons


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--holdout", required=True)
    ap.add_argument("--arms", nargs="+", required=True)
    ap.add_argument("--judge", default="qwen2.5:14b")
    ap.add_argument("--ollama", default=os.environ.get("AB_OLLAMA", "http://172.18.0.1:11434"))
    ap.add_argument("--gateway", default=os.environ.get("AB_GATEWAY", "http://127.0.0.1:4000/v1"))
    ap.add_argument("--gateway-key-file", default=os.environ.get("AB_GATEWAY_KEY_FILE", "/srv/rogue/keys/kannaka-tui.key"))
    ap.add_argument("--n", type=int, default=30)
    ap.add_argument("--seed", type=int, default=7)
    ap.add_argument("--max-tokens", type=int, default=200)
    ap.add_argument("--temperature", type=float, default=0.7)
    ap.add_argument("--timeout", type=int, default=600)
    ap.add_argument("--out", required=True)
    a = ap.parse_args(argv)

    key = Path(a.gateway_key_file).read_text().strip()
    out = Path(a.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    rows = [json.loads(line) for line in open(a.holdout, encoding="utf-8") if line.strip()]
    rows = [r for r in rows if r.get("messages") and r["messages"][-1]["role"] == "assistant"]
    random.Random(a.seed).shuffle(rows)
    rows = rows[:a.n]

    state = json.load(open(out)) if out.exists() else {"arms": a.arms, "judge": a.judge, "seed": a.seed, "gens": []}
    if state["arms"] != a.arms:
        sys.exit(f"--out holds a run with arms {state['arms']}; pick another --out")
    done = {g["id"]: g for g in state["gens"]}

    def save():
        pairs, per, cons = score(state["gens"], a.arms)
        state.update({"pairs": pairs, "scores": per, "consistency": cons, "updated": time.time()})
        tmp = out.with_suffix(".tmp")
        tmp.write_text(json.dumps(state, indent=1, ensure_ascii=False), encoding="utf-8")
        os.replace(tmp, out)

    for i, r in enumerate(rows, 1):
        g = done.get(r["id"])
        if g is None:
            g = {"id": r["id"], "title": r.get("title"), "conversation": conversation_text(r["messages"]),
                 "reference": r["messages"][-1]["content"], "replies": {}, "elapsed": {}, "verdicts": {}}
            state["gens"].append(g)
            done[r["id"]] = g
        for arm in a.arms:
            if arm in g["replies"]:
                continue
            try:
                g["replies"][arm], g["elapsed"][arm] = generate(a.gateway, key, arm, r["messages"][:-1],
                                                              a.max_tokens, a.temperature, a.timeout)
            except Exception as e:  # one arm failing must not sink the run
                log(f"{i}/{len(rows)} {arm} generation failed: {e}")
                continue
            log(f"{i}/{len(rows)} {arm} {g['elapsed'][arm]}s: {g['replies'][arm][:70]!r}")
            save()
        for x, y in itertools.combinations(a.arms, 2):
            if x not in g["replies"] or y not in g["replies"]:
                continue
            v = g["verdicts"].setdefault(f"{x}|{y}", {})
            if "xy" not in v:
                v["xy"], v["why_xy"] = judge(a.ollama, a.judge, g["conversation"], g["reference"],
                                             g["replies"][x], g["replies"][y], a.timeout)
            if "yx" not in v:
                v["yx"], v["why_yx"] = judge(a.ollama, a.judge, g["conversation"], g["reference"],
                                             g["replies"][y], g["replies"][x], a.timeout)
            log(f"{i}/{len(rows)} judge {x} vs {y}: {v['xy']}/{v['yx']}")
            save()

    save()
    print("\n== pairwise (position-consistent wins)")
    for k, p in state["pairs"].items():
        print(f"  {k}: " + ", ".join(f"{kk}={vv}" for kk, vv in p.items()))
    print("== per arm")
    for arm, s in state["scores"].items():
        el = [g["elapsed"].get(arm) for g in state["gens"] if g["elapsed"].get(arm)]
        mean = round(sum(el) / len(el), 1) if el else None
        print(f"  {arm:24s} wins={s['wins']:3d} losses={s['losses']:3d} ties={s['ties']:3d} "
              f"win_rate_decided={s['win_rate_decided']} mean_s={mean}")
    print(f"== judge position consistency: {state['consistency']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
