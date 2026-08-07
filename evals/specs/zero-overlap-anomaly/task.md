# Task — zero-overlap-anomaly

Status: approved (by delegation, 2026-08-01 — standing "steer the bus" authority)

Capability: non-lexical (semantic) recall — retrieving a target memory into the top 10
when the query shares zero >=4-char tokens with the target's full content. A pure
bag-of-hashed-tokens encoder scores zero here by construction; any hit is evidence of a
semantic mechanism (codebook projection, chiral hemispheres, or consolidation-formed
association — the "short-lived glimpses" phenomenon).

Request: run the 33-probe suite through the standard adapter (instruction.md).

Why this requires the capability: the zero-overlap invariant is enforced twice — at probe
generation and re-verified by the verifier from full target content — so lexical token
match cannot produce a hit. Expected UUIDs and target content exist only verifier-side.

Pass iff: >= 1 zero-overlap probe hits top-10 (anomaly PRESENT), with all infrastructure
guards holding (snapshot hash, 33/33 parseable, invariant intact). Reward = recall@10
over the 33 probes (continuous), so improvements in semantic reach move the number even
while the gate stays existential. Tighten the gate after the first frozen run.

Reference prediction: on baseline-002 data, 2 of the 8 original zero-overlap probes hit
(p31@3, p43@6). The 25 rewritten probes are unmeasured — 4 of their sources (p11, p29,
p38, p45) hit with their original wording; whether they survive de-lexicalization is
exactly what this eval measures.

Verifier: deterministic only; evidence = raw rollout + tests/expected.json +
tests/targets.json + pinned snapshot hash. Fixtures: valid rollout (targets at plausible
ranks) must pass; wrong-IDs rollout must fail; an overlap-breaking probe must
infra-error (exit 3), not score.

Limitation: same-author probe bias (see environment.md); 33 probes is directional.
