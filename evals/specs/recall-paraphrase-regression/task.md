# Task — recall-paraphrase-regression

Status: approved (by delegation, 2026-08-01 — Nick: "steer the bus"; first run baseline-002 audited)

Capability: semantic paraphrase recall — given a query that deliberately shares minimal
vocabulary with its target memory, the production recall pipeline (ChiralMedium, knobs at
production settings 0.0/1.0) returns the target within the top 10, measured over a frozen
609-memory corpus with 50 UUID-pinned probes.

Request (`instruction.md`): run the probe suite — invoke `kannaka recall "<query>"
--top-k 10` for every query in `/task/probes.json` with the fixed knob environment and
write raw per-probe results to `/output/rollout.json`. The Harness is a deterministic CLI
driven by the adapter; the instruction is fixed, single-turn.

Initial conditions: frozen HRM snapshot (hash-pinned), probe queries WITHOUT expectations,
knob env fixed, no network.

Why this requires the capability: probes are paraphrases built to defeat token overlap
(see build-probes-v2.mjs header), and ground truth was pinned via literal `kannaka search`
— a different mechanism than resonance — so the Harness cannot pass by lexical match or by
reading the answer anywhere in its environment. Expected UUIDs are not present in any
Harness-visible layer.

Pass iff: over the 50 probes, recall@10 >= GATE, where GATE is calibrated from this
eval's own first frozen-environment run (live-store reference: 0.24; provisional GATE 0.20
until calibration replaces it) — AND all infrastructure guards held: knob strings present
in the binary, snapshot hash matched, 50/50 probes produced parseable output.

Verifier: deterministic only (no LLM judge). Recomputes recall@10, MRR, nDCG@10 from the
raw per-probe result arrays against `tests/expected.json`. Primary reward = recall@10
(continuous 0..1); pass/fail vs GATE and per-probe ranks in metadata. Guard failures are
infrastructure errors (no agent score), not reward 0.

Verifier evidence: `/output/rollout.json` (raw CLI outputs + provenance) plus
`tests/expected.json` (probe id -> expected UUID set) plus the recorded snapshot hash.

Verifier fixtures (calibrated before the real run): a synthetic rollout whose result
lists contain the expected UUIDs at plausible ranks -> pass with the right metric; the
same rollout with shuffled/foreign ids -> fail with reward ~0.

Accepted alternatives: none needed — outcome is a computed metric, not semantic judgment.

Limitation (stated, not hidden): probes were written by the person who read the targets;
paraphrase discipline mitigates lexical leakage but this is not a blind relevance set.
The eval measures regression against a pinned corpus+probe pair, not absolute recall
quality. Temporal-term clock drift is a known slow bias (see environment.md).
