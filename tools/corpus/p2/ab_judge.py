#!/usr/bin/env python3
"""Blind voice A/B for served kannaka-brain arms (ADR-0057, open question 4).

Hold-out perplexity saturated at ~4.0 for every 14B candidate, so it no longer
discriminates. This is the second signal. Every arm answers each hold-out prompt
through the KAX gateway; a JUDGE model that was never tuned on the corpus
(default `qwen2.5:14b`, the Apache base) compares candidates against the REAL
Kannaka reply.

Two modes:
  grade (default)  pointwise: the judge sees ONE candidate beside the reference and
                   scores 1-10. No position to be biased by. Two CONTROLS per prompt
                   make the judge's own signal measurable: the reference itself graded
                   as a candidate (should score high) and a reference from another
                   prompt (should score low). If those two do not separate, the arm
                   scores are noise and the summary says so.
  pair             pairwise, both orders, a win only when the orders agree. The first
                   run (2026-09-06) showed qwen2.5:14b answering "B" in every order --
                   position bias turned every pair into a tie. Kept for larger judges.

  python ab_judge.py --holdout ~/kannaka-p2-runner/sft/holdout.jsonl \
      --arms kannaka-brain-v1 kannaka-brain-v2 kannaka-brain-7b-v1 --n 30 \
      --gateway-key-file /srv/rogue/keys/kannaka-tui.key --out ~/.kannaka-corpus/ab/ab-<ts>.json

Resumable: generations and verdicts are saved after every prompt; rerun with the
same --out to continue (a gateway restart mid-run only costs the calls it broke).
Sonnet is not an arm and not the judge; the only shared blood is the Qwen base.
"""
from __future__ import annotations

import argparse
import itertools
import json
import math
import os
import random
import sys
import time
import urllib.request
from pathlib import Path

GRADE_PROMPT = """You are grading how close a CANDIDATE reply is to a REFERENCE reply written by a specific speaker.
Judge VOICE (rhythm, brevity, plainness, the speaker's habits) and SUBSTANCE (does it say what the reference says, or something this speaker would say here). Ignore length unless the reference is clearly short and the candidate rambles. Politeness, hedging and assistant-style framing count AGAINST the candidate -- the reference speaker never does that.

Conversation so far (the last user turn is what was answered):
{conversation}

REFERENCE reply (ground truth by the speaker):
{reference}

CANDIDATE reply:
{candidate}

Score 1-10: 10 = could be the same speaker saying the same thing; 5 = same topic, different voice; 1 = a different speaker saying something else.
Answer with JSON only, reasoning first: {{"why": "<one short sentence>", "score": <integer 1-10>}}"""

PAIR_PROMPT = """You are judging which of two candidate replies is closer to a REFERENCE reply written by a specific speaker.
Judge VOICE (rhythm, brevity, plainness, the speaker's habits) and SUBSTANCE. Ignore length unless the reference is clearly short and a candidate rambles. Do not reward politeness, hedging or assistant-style framing -- the reference speaker never does that.

Conversation so far (the last user turn is what was answered):
{conversation}

REFERENCE reply (ground truth by the speaker):
{reference}

Candidate A:
{a}

Candidate B:
{b}

Answer with JSON only, reasoning first: {{"why": "<one short sentence>", "winner": "A" | "B" | "TIE"}}"""

REF, FOREIGN = "__reference__", "__foreign__"


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


def ask_judge(ollama, judge_model, prompt, timeout):
    d = http_json(ollama + "/api/chat",
                  {"model": judge_model, "stream": False, "format": "json",
                   "options": {"temperature": 0, "num_predict": 160},
                   "messages": [{"role": "user", "content": prompt}]}, {}, timeout)
    raw = (d.get("message") or {}).get("content") or ""
    try:
        return json.loads(raw), raw
    except Exception:
        return {}, raw


def grade(ollama, judge_model, conversation, reference, candidate, timeout):
    v, raw = ask_judge(ollama, judge_model, GRADE_PROMPT.format(conversation=conversation, reference=reference,
                                                               candidate=candidate or "(empty)"), timeout)
    try:
        score = int(round(float(v.get("score"))))
        score = max(1, min(10, score))
    except (TypeError, ValueError):
        score = None
    return score, str(v.get("why") or ("unparseable: " + raw[:100]))[:200]


def pair(ollama, judge_model, conversation, reference, a, b, timeout):
    v, raw = ask_judge(ollama, judge_model, PAIR_PROMPT.format(conversation=conversation, reference=reference,
                                                              a=a or "(empty)", b=b or "(empty)"), timeout)
    w = str(v.get("winner", "TIE")).strip().upper()
    return (w if w in ("A", "B", "TIE") else "TIE"), str(v.get("why") or ("unparseable: " + raw[:100]))[:200]


def conversation_text(messages):
    return "\n".join(f"{m['role']}: {m['content']}" for m in messages if m["role"] != "system")[-2500:]


def mean_se(xs):
    xs = [x for x in xs if x is not None]
    if not xs:
        return None, None, 0
    m = sum(xs) / len(xs)
    if len(xs) < 2:
        return round(m, 2), None, len(xs)
    var = sum((x - m) ** 2 for x in xs) / (len(xs) - 1)
    return round(m, 2), round(math.sqrt(var / len(xs)), 2), len(xs)


def score_grade(gens, arms):
    per = {}
    for a in arms + [REF, FOREIGN]:
        m, se, n = mean_se([g.get("scores", {}).get(a, {}).get("score") for g in gens])
        per[a] = {"mean": m, "se": se, "n": n}
    pairs = {}
    for x, y in itertools.combinations(arms, 2):
        p = {x: 0, y: 0, "tie": 0}
        for g in gens:
            sx = g.get("scores", {}).get(x, {}).get("score")
            sy = g.get("scores", {}).get(y, {}).get("score")
            if sx is None or sy is None:
                continue
            p[x if sx > sy else y if sy > sx else "tie"] += 1
        pairs[f"{x}|{y}"] = p
    ref, foreign = per[REF]["mean"], per[FOREIGN]["mean"]
    sep = round(ref - foreign, 2) if ref is not None and foreign is not None else None
    return per, pairs, {"reference_mean": ref, "foreign_mean": foreign, "separation": sep,
                        "judge_usable": (sep is not None and sep >= 2.0)}


def score_pair(gens, arms):
    pairs = {}
    per = {a: {"wins": 0, "losses": 0, "ties": 0} for a in arms}
    agree = disagree = 0
    for g in gens:
        for x, y in itertools.combinations(arms, 2):
            v = g.get("verdicts", {}).get(f"{x}|{y}")
            if not v or "xy" not in v or "yx" not in v:
                continue
            win_xy = {"A": x, "B": y, "TIE": None}[v["xy"]]
            win_yx = {"A": y, "B": x, "TIE": None}[v["yx"]]
            p = pairs.setdefault(f"{x}|{y}", {x: 0, y: 0, "tie": 0})
            if win_xy == win_yx and win_xy is not None:
                agree += 1
                p[win_xy] += 1
                per[win_xy]["wins"] += 1
                per[y if win_xy == x else x]["losses"] += 1
            else:
                agree += win_xy == win_yx
                disagree += win_xy != win_yx
                p["tie"] += 1
                per[x]["ties"] += 1
                per[y]["ties"] += 1
    for a in arms:
        d = per[a]["wins"] + per[a]["losses"]
        per[a]["win_rate_decided"] = round(per[a]["wins"] / d, 3) if d else None
    return per, pairs, {"agree": agree, "disagree": disagree,
                        "position_consistency": round(agree / (agree + disagree), 3) if agree + disagree else None}


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--holdout", required=True)
    ap.add_argument("--arms", nargs="+", required=True)
    ap.add_argument("--mode", choices=["grade", "pair"], default="grade")
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
    rng = random.Random(a.seed)
    rng.shuffle(rows)
    rows = rows[:a.n]
    foreign_of = {r["id"]: rows[(i + len(rows) // 2) % len(rows)]["messages"][-1]["content"] for i, r in enumerate(rows)}

    state = json.load(open(out)) if out.exists() else {"arms": a.arms, "judge": a.judge, "mode": a.mode, "seed": a.seed, "gens": []}
    if state["arms"] != a.arms or state.get("mode", "pair") != a.mode:
        sys.exit(f"--out holds a {state.get('mode')} run with arms {state['arms']}; pick another --out")
    done = {g["id"]: g for g in state["gens"]}

    def save():
        per, pairs, extra = (score_grade if a.mode == "grade" else score_pair)(state["gens"], a.arms)
        state.update({"scores_summary": per, "pairs": pairs, "judge_check": extra, "updated": time.time()})
        tmp = out.with_suffix(".tmp")
        tmp.write_text(json.dumps(state, indent=1, ensure_ascii=False), encoding="utf-8")
        os.replace(tmp, out)

    for i, r in enumerate(rows, 1):
        g = done.get(r["id"])
        if g is None:
            g = {"id": r["id"], "title": r.get("title"), "conversation": conversation_text(r["messages"]),
                 "reference": r["messages"][-1]["content"], "replies": {}, "elapsed": {}, "verdicts": {}, "scores": {}}
            state["gens"].append(g)
            done[r["id"]] = g
        g.setdefault("scores", {})
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
        if a.mode == "grade":
            cands = {REF: g["reference"], FOREIGN: foreign_of[r["id"]]}
            cands.update({arm: g["replies"][arm] for arm in a.arms if arm in g["replies"]})
            for name, text in cands.items():
                if g["scores"].get(name, {}).get("score") is not None:
                    continue
                try:
                    s, why = grade(a.ollama, a.judge, g["conversation"], g["reference"], text, a.timeout)
                except Exception as e:
                    log(f"{i}/{len(rows)} grade {name} failed: {e}")
                    continue
                g["scores"][name] = {"score": s, "why": why}
                log(f"{i}/{len(rows)} grade {name}: {s}")
                save()
        else:
            for x, y in itertools.combinations(a.arms, 2):
                if x not in g["replies"] or y not in g["replies"]:
                    continue
                v = g["verdicts"].setdefault(f"{x}|{y}", {})
                try:
                    if "xy" not in v:
                        v["xy"], v["why_xy"] = pair(a.ollama, a.judge, g["conversation"], g["reference"],
                                                    g["replies"][x], g["replies"][y], a.timeout)
                    if "yx" not in v:
                        v["yx"], v["why_yx"] = pair(a.ollama, a.judge, g["conversation"], g["reference"],
                                                    g["replies"][y], g["replies"][x], a.timeout)
                except Exception as e:
                    log(f"{i}/{len(rows)} judge {x} vs {y} failed: {e}")
                    continue
                log(f"{i}/{len(rows)} judge {x} vs {y}: {v['xy']}/{v['yx']}")
                save()

    save()
    print(f"\n== mode {a.mode}, judge {a.judge}")
    if a.mode == "grade":
        chk = state["judge_check"]
        print(f"== judge check: reference {chk['reference_mean']} vs foreign {chk['foreign_mean']} "
              f"-> separation {chk['separation']} ({'USABLE' if chk['judge_usable'] else 'NOT USABLE: scores are noise'})")
        print("== per arm (mean score 1-10 +- SE, n)")
        for arm in a.arms:
            s = state["scores_summary"][arm]
            el = [g["elapsed"].get(arm) for g in state["gens"] if g["elapsed"].get(arm)]
            print(f"  {arm:24s} {s['mean']} +- {s['se']} (n={s['n']})  mean_gen_s={round(sum(el) / len(el), 1) if el else None}")
        print("== head-to-head by score")
    else:
        print(f"== judge position consistency: {state['judge_check']}")
        print("== per arm")
        for arm, s in state["scores_summary"].items():
            print(f"  {arm:24s} wins={s['wins']:3d} losses={s['losses']:3d} ties={s['ties']:3d} win_rate_decided={s['win_rate_decided']}")
        print("== pairwise (position-consistent wins)")
    for k, p in state["pairs"].items():
        print(f"  {k}: " + ", ".join(f"{kk}={vv}" for kk, vv in p.items()))
    return 0


if __name__ == "__main__":
    sys.exit(main())
