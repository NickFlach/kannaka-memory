# Environment — zero-overlap-anomaly

Status: approved (by delegation, 2026-08-01 — standing "steer the bus" authority)

Same frozen world as recall-paraphrase-regression: byte-identical HRM snapshot
(sha `339e7ad9…`, 615 memories), synthetic credential-free config, no network,
per-trial store copy. See ../recall-paraphrase-regression/environment.md for the
full contract, isolation, and fidelity limits (unfrozen clock).

Additional data:
- `/environment/probes.json` — 33 queries with ZERO >=4-char token overlap with
  their target's FULL content: 8 v2 probes already at zero overlap, plus 25
  near-zero v2 probes surgically rewritten (offending words replaced) and
  mechanically re-verified against a full-content `export-json --slim` dump of the
  snapshot. Tokenization: lowercase `[a-z0-9]{4,}`.
- `tests/targets.json` — verifier-side full target content per probe, used to
  re-verify the zero-overlap invariant at score time. NOT committed to git
  (contains full memory text; the truncated sample TSV in scripts/probes is the
  only content in the repo). Regenerate: `kannaka export-json --slim` against a
  copy of the snapshot, then rebuild via the generation recorded in git history.

Known bias, stated plainly: 25 of 33 queries were rewritten by the eval author
(this Claude session) reading the targets — same-author bias as v2, mitigated by
the mechanical invariant but not eliminated. The 8 originals carry v2's bias.
