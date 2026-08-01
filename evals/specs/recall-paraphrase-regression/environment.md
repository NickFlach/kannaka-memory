# Environment — recall-paraphrase-regression

Status: approved (by delegation, 2026-08-01 — Nick: "steer the bus"; first run baseline-002 audited)

Dependencies:
- HRM store — FROZEN. Byte-for-byte snapshot of `~/.kannaka/kannaka.hrm` (48 MB,
  609 memories) taken at task-build time. sha256 recorded in
  `environment/data/SNAPSHOT.sha256`; the Verifier refuses to score a run whose recorded
  snapshot hash differs (a different corpus is a different eval).
- No other dependencies. No NATS/swarm (network none), no LLM, no external services.

Backend contract:
- Interface: the CLI reads `$KANNAKA_DATA_DIR/kannaka.hrm`; `recall` prints a JSON array
  of `{id, content, ...}` to stdout (stderr carries the load banner).
- Effects: `recall` calls `record_retrieval()` which bumps `retrieval_count` and can
  persist the store — so the store is treated as MUTABLE.
- Reset: each trial copies the pristine snapshot into a fresh work dir and points
  `KANNAKA_DATA_DIR` at the copy. Idempotent; the snapshot itself is never opened by the CLI.

Data:
- `environment/data/kannaka.hrm` — the snapshot. NOT committed to git (gitignored;
  binary blob + personal content). Regeneration: `environment/snapshot.ps1` re-copies from
  the live dir and rewrites the hash — doing so REBASELINES the eval and requires
  re-validating that all 50 expected UUIDs still exist in the store.
- `environment/data/config.toml` — minimal synthetic config: default HRM settings only,
  no API keys, no NATS credentials, synthetic `agent_id` (`evalbot`). The live
  `~/.kannaka/config.toml` and key files (`anthropic-new.key`, `cascade-pat.txt`, `.git`,
  dolt-memory) are NEVER copied into the image.
- `/task/probes.json` — the 50 paraphrase queries from `scripts/probes/probes-v2.json`
  with the `expect` field STRIPPED. Expected UUIDs live only in `tests/expected.json`
  (Verifier side, not in the Harness-visible image layer).

Isolation:
- Network: none (`network_mode` locked down; recall is a local operation and swarm
  connect attempts fail open).
- Filesystem: container-local; per-trial store copy.
- Identity: synthetic agent id.
- Privacy: the HRM snapshot contains Nick's personal memory content. The task image is
  built and run locally ONLY — never pushed to any registry, and `evals/jobs/` output is
  reviewed before any sharing.

Fidelity limits:
- Clock NOT frozen. The binary is musl-static, so LD_PRELOAD/libfaketime cannot
  intercept time; with `KANNAKA_RECALL_TEMPORAL_EXP=1.0` the temporal ranking term scores
  memory timestamps against real now(), so scores drift slowly as the snapshot ages.
  Run date is recorded; the pass gate carries margin for this. If drift ever moves the
  metric materially, the fix is to rebaseline, not to loosen the gate silently.
- Dream/consolidation state is whatever the live store had at snapshot time; dreams do
  not run inside the eval.
