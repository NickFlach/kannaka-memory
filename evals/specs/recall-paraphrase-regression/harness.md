# Harness — recall-paraphrase-regression

Status: approved (by delegation, 2026-08-01 — Nick: "steer the bus"; first run baseline-002 audited)

Entrypoint: `kannaka recall "<query>" --top-k 10` — the real CLI, production dispatch
`resonate_query` -> ChiralMedium. NOT `recall_resonance_readonly` (flat medium, different
scorer — see scripts/recall-harness.mjs header for why that distinction is load-bearing).

Source: release asset `kannaka-linux-x86_64` from tag v0.13.0 (musl static), sha256
verified against the published `.sha256`. Both recall knobs are required: the image build
runs `grep -a` for `KANNAKA_RECALL_ENERGY_EXP` and `KANNAKA_RECALL_TEMPORAL_EXP` on the
binary and FAILS if either is absent (ADR-0050 commit 64fb12a is an ancestor of v0.13.0,
so both should be present — but we verify, per the stale-binary lesson). Fallback if the
guard fails: musl build from a pinned commit, recorded here.

Preserved behavior: ranking pipeline exactly as shipped. Knobs are set explicitly (never
inherited) to the production configuration on Nick's machine:
`KANNAKA_RECALL_ENERGY_EXP=0.0`, `KANNAKA_RECALL_TEMPORAL_EXP=1.0`.

Adapter: `environment/run-probes.mjs` — reads `/task/probes.json` (queries only, no
expectations), invokes the CLI once per probe with the fixed knob env, records per-probe
raw stdout JSON, parse status, and binary provenance (path, sha256, `--version`, knob env)
to `/output/rollout.json`. It contains no expected UUIDs and makes no scoring decisions.

Session: single-turn, deterministic; one adapter invocation per trial.

Credentials: none. No model call in the Harness; the Verifier is deterministic, so no
judge credentials exist anywhere in the task.

Recorded evidence: per-probe raw result arrays (id, content-prefix, score), CLI exit
codes, stderr tails, resolved knob env, binary provenance, snapshot sha256, run timestamp.

Reconstruction differences: (1) binary is release v0.13.0 linux/musl, not the locally
built Windows binary (commit 4ca342a) that produced the 2026-08-01 live baseline — ranking
changes between the tag and HEAD would show up as a delta, which is why the gate is
calibrated on this eval's own first frozen run, not on the live 0.24. (2) OS differs
(Linux container vs Windows host); the HRM format is platform-independent.
