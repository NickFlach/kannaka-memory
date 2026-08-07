# Harness — zero-overlap-anomaly

Status: approved (by delegation, 2026-08-01 — standing "steer the bus" authority)

Identical to recall-paraphrase-regression (see ../recall-paraphrase-regression/harness.md):
real `kannaka recall "<query>" --top-k 10`, release binary v0.13.0 linux/musl
(sha-verified, knob strings grep-guarded at image build), production knobs
0.0/1.0 set explicitly, driven by the same `run_probes.py` adapter via Harbor's
oracle agent. Single-turn, deterministic, no credentials anywhere.

Only difference: the probe file — 33 zero-overlap queries instead of the 50 v2
paraphrases.
