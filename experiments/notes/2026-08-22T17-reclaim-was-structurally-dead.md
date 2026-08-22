# Both reclaim paths were structurally dead — measured across the constellation

Measured 2026-08-22 over 17 instrumented deep dreams on 5 HRMs (kannaka-prime, the O3
prime replica, gossipghost, the O2 witness, and the local substrate), before/after with
fresh-process `observe --json`.

## Finding 1 — the evict threshold was in the wrong units

`shortterm_evict = 0.15`, tested against `amplitude`. Measured amplitudes:

| node | weakest active | mean |
|---|---|---|
| kannaka-prime | **0.60** | 0.93 |
| local | — | **1.73** |

Nothing in any live corpus came within 4× of the threshold, so `would_evict` was
**structurally always 0** on every node. Controlled proof — same substrate, same dream,
only the env changed:

| `KANNAKA_SHORTTERM_EVICT` | plan line |
|---|---|
| 0.15 (default) | `ShortTerm 0/184 would evict` |
| 0.65 | `ShortTerm 176/198 would evict` (projected 252 of 432) |

`would_decay` was also a mislabel: it echoed the ShortTerm *headcount*, so the digest read
"186 would decay" on a node where nothing decayed at all — the metric concealed the bug it
should have exposed.

Root cause: ADR-0036's **M3 salience decay was never implemented**. Without decay, amplitude
only ever goes up (a nightly dream strengthens ~14k wavefronts and prunes 0), so no trace can
descend toward any fixed floor.

## Finding 2 — the merge cap permanently stranded the largest redundancy

Local substrate, belief on, `CONSOLIDATE=on`:

```
3 redundant groups would merge, absorbing 9 wavefronts
⚠ absorb cap engaged: criteria found 4 groups / 217 absorbable,
  but the per-pass cap admitted only 3 groups / 9 absorbed.
```

**217 of 654 wavefronts (33%) met the redundancy criteria; 9 merged.** Groups were admitted
whole-or-not-at-all in descending cohesion, so the one ~208-member group could never fit under
the 20% cap — and since nothing about it changes, it was skipped again every night, forever.

Together these two explain unbounded substrate growth on every node regardless of how
faithfully the nightly dream runs, and they are the origin of the O1 disk pressure.

## Fix — borrowed from `kannaka-crystal`'s dream engine

`kannaka-crystal` (flaukowski/kannaka-crystal, `src/dream.rs`) solved the calibration problem
already: it thresholds at a **percentile of the observed stability distribution**, never at an
absolute constant, and it **attenuates** (`u *= 0.65`) rather than removing.

1. **M3 salience decay, percentile-based** (`compute_decay_set`). Rank-based, not value-based,
   so a corpus where many amplitudes are exactly equal — the live case, audio telemetry lands
   on identical amplitudes — still fades the intended fraction instead of all-or-nothing.
   A percentile has no scale to get wrong. Soft multiply, never a removal; Pinned/LongTerm,
   anything ever retrieved, and merge participants are all excluded. Defaults 0.50 / 0.90
   (gentler than crystal's 0.65 because kannaka dreams nightly): a 0.60 trace crosses the 0.15
   floor in ~14 unretrieved nights, at which point the *existing* conservative evict finally
   becomes reachable. **The absolute threshold was never raised.**
2. **Partial admission of an oversized group.** Admit the representative plus its most-cohesive
   members up to remaining capacity. The cap is honoured exactly; the group drains over
   several nights rather than never.
3. `would_decay` now reports actual attenuations.

Both are shared by the plan and apply paths, preserving dry-run/apply parity. Opt-outs:
`KANNAKA_SHORTTERM_DECAY_PCTL=0`, `KANNAKA_MERGE_PARTIAL=0`.

## Still open — the settle phase (recrystallization)

Crystal's dream ends with `engine.resonate(50)` — a short free evolution "so the reshaped field
relaxes into a self-consistent state before anyone probes it." **kannaka-memory has no
equivalent: it persists immediately after reshaping.**

That is visible in the measurements. Cluster structure melts and recrystallizes —
O3 `18→4→3→**21**→8→6`, O2 witness `25→**6**→**52**→42` — and phi peaks exactly at the
recrystallization (O2: 0.099 → **0.476**, +380%, Dormant → Aware). But nothing controls *when*
the field is written, so a substrate is routinely persisted mid-melt, at 3 clusters instead of 21.

A settle pass before flush is the natural next borrowing. Note that crystal's morphological
primitives (Echo Ring, Harmonic Bridge, Phase Knot …) do **not** transplant directly — they are
shape heuristics over a 2-D grid, and the HRM has no grid. The transferable ideas are the
*discipline* (observe → percentile → reshape → settle), not the primitive catalogue.
