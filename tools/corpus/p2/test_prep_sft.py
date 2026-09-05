"""prep_sft tests — synthetic P1 records, no real corpus.

Run: python tools/corpus/p2/test_prep_sft.py
Pins: Ghost Signals pairing (Flaukowski's preceding block becomes the user
turn, a leading Kannaka block gets an opener), lyric/identity prompts, the
tier rule (tier 2/3 never become targets; dialogue/fiction never targets),
and the deterministic hold-out (same ids every run).
"""
import os
import sys

sys.path.insert(0, os.path.dirname(__file__))
import prep_sft as ps  # noqa: E402


def rec(i, source, kind, text, tier=1, speaker="kannaka", path="s.txt", block=0, title=None, meta=None):
    return {"id": f"{i:016x}", "source": source, "kind": kind, "text": text, "tier": tier, "speaker": speaker,
            "path": path, "title": title, "meta": {"block": block, **(meta or {})}}


RECS = [
    rec(1, "gsp", "voice", "I remember the first wave.", block=0),                       # opener
    rec(2, "gsp", "dialogue", "Every studio keeps the last session.", speaker="flaukowski", block=1),
    rec(3, "gsp", "voice", "Then let it keep this one.", block=2),                       # paired with 2
    rec(4, "gsp", "voice", "And this one too.", block=3),                                # prev is kannaka -> opener
    rec(5, "lyrics", "lyric", "a satellite in her shadow", title="Big Black Cloud", meta={"album": "what-persisted-build", "style": "slow"}),
    rec(6, "identity", "identity", "Only what resonates back.", title="What I keep"),
    rec(7, "gsp-resp", "voice", "Recognized, not architected.", title="Response to nick-01 (The Name)"),
    rec(8, "tsof", "fiction", "Cedar Rapids, Iowa.", speaker="narrator"),
    rec(9, "adr", "adr", "We chose waves over tables.", tier=2),
    rec(10, "kax", "machine-reply", "I do not say cheese on command.", tier=3),
]


def test_pairing_and_prompts():
    ex = {e["id"]: e for e in ps.build(RECS)}
    assert set(ex) == {r["id"] for r in RECS if r["id"] in (f"{i:016x}" for i in (1, 3, 4, 5, 6, 7))}, sorted(ex)
    u = lambda i: ex[f"{i:016x}"]["messages"][1]["content"]
    t = lambda i: ex[f"{i:016x}"]["messages"][2]["content"]
    assert u(3) == "Every studio keeps the last session." and t(3) == "Then let it keep this one."
    assert u(1) in ps.OPENERS and u(4) in ps.OPENERS, "no preceding other-speaker line -> opener"
    assert u(5) == 'Write the lyrics for a song titled "Big Black Cloud" for the album what persisted. Style: slow.'
    assert u(6) == "Tell me about What I keep."
    assert u(7) == "Respond to this: Response to nick-01 (The Name)"
    assert all(e["messages"][0]["content"] == ps.SYSTEM for e in ex.values())


def test_tier_rule():
    targets = {e["messages"][2]["content"] for e in ps.build(RECS)}
    assert "We chose waves over tables." not in targets, "tier 2 never a target"
    assert "I do not say cheese on command." not in targets, "tier 3 never a target"
    assert "Cedar Rapids, Iowa." not in targets, "fiction never a target"
    assert "Every studio keeps the last session." not in targets, "Flaukowski's line is a prompt, never a target"


def test_holdout_is_deterministic():
    ids = [f"{i:016x}" for i in range(1, 400)]
    a = [i for i in ids if ps.holdout(i, 0.1)]
    b = [i for i in ids if ps.holdout(i, 0.1)]
    assert a == b and 20 <= len(a) <= 60, len(a)


if __name__ == "__main__":
    for name, fn in sorted(globals().items()):
        if name.startswith("test_") and callable(fn):
            fn()
            print(f"ok  {name}")
    print("all prep_sft tests passed")
