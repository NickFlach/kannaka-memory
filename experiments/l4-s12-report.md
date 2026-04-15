# L4.S12 — Depth-4 EML Symbolic Regression Retry on `corpus_xi_diversity`

Follow-up to L4.S0 (commit `8a15ee7`, depth-3). Paper reference:
Odrzywolek, arXiv:2603.21852v2 (April 2026).

## Setup

- **Depth:** 4 (15 internal EML nodes, 16 leaves, 170 params, ~2x depth-3)
- **Data:** `experiments/xi_pairs.json` (300 pairs, reused from L4.S0)
- **Optimizer:** Adam, lr=0.005
- **Steps/restart:** 10000 (hardening last 1500 steps, temperature 1.0 -> 0.02)
- **Restarts:** 10 per phase, independent random seeds
- **Operator:** `eml(L, R) = exp(clamp(L, -8, 8)) - ln(clamp(R, 1e-6, 1e8))`
- **Leaf vocab:** {1, s, r}; **input vocab:** {1, s, r, child}

Input stats: `sim in [-0.160, 0.955]`, `rep in [0.057, 0.291]`, `boost in [-0.116, 1.000]`, `boost var = 0.115254`.

## Phase A — replicate current formula (MSE loss against `current_boost`)

| Metric                | Depth-3 (L4.S0) | Depth-4 (L4.S12) |
|-----------------------|-----------------|-------------------|
| Best soft MSE         | 0.000742        | **6.95e-05**      |
| Best snap MSE         | 0.518           | **0.137**         |
| N successful restarts | 1 / 1           | 10 / 10           |

**Soft fit improved ~10x** (0.000742 -> 0.0000695), confirming the enlarged
discrete space contains trees that continuously approximate the hand-coded
formula more tightly.

**Snap MSE improved ~3.8x** (0.518 -> 0.137) but is still far from the
`<1e-3` threshold that would indicate a true discrete match. The best snapped
formula (seed 2) collapses to:

```
0.6485 * (exp(s) - ln(1)) + -0.2173
  = 0.6485 * exp(s) - 0.2173
```

This is a pure-`sim` univariate — it ignores `r` entirely. The discrete basin
closest to the continuous optimum is still a sim-only affine in `exp(s)`, the
same collapse failure mode we saw at depth-3. **Depth-4 did not find a snap
basin matching the full formula in 10 blind restarts.**

## Phase B — discrimination (variance maximization with monotonicity + mean-band constraints)

Target: beat current hand-coded variance `0.1153` with a tree output that is
monotone-increasing in `rep` and has snapped mean in `[0.2, 0.8]`.

| Metric                          | Depth-3 (L4.S0) | Depth-4 (L4.S12) |
|---------------------------------|-----------------|-------------------|
| Best soft variance              | 0.0604          | **0.2333**        |
| Best soft ratio vs current      | 0.524           | **2.024**         |
| Best snap variance              | 0.0 (collapsed) | **0.0178**        |
| Best snap ratio vs current      | 0.0             | 0.154             |
| Restarts that beat current snap | 0 / 1           | 0 / 10            |

**Soft variance now exceeds the current formula by ~2x** (0.2333 vs 0.1153) —
a major qualitative change. The depth-4 continuous space clearly contains
candidate trees that are **more discriminating** than the hand-coded formula.
Multiple independent restarts (seeds 0, 4, 6, 9) all found soft variances in
the 0.22 - 0.23 region, so this is not a lucky seed — it's a robust feature of
the expanded search space.

**However the snap collapse persists.** Six of ten restarts snapped to
degenerate constants (variance 0.0 with mean 0 or 1). The remaining four
produced tiny variance (0.001 - 0.047), and the two with variance > 0.04
violated monotonicity. The best admissible snap (seed 9) achieves only
`var=0.0178`, a factor of ~6.5x **below** the current formula.

Best admissible snapped Phase B formula (seed 9):

```
clamp01(1.3770 * (exp((exp(s) - ln((exp((exp(s) - ln(r))) - ln((exp(s) - ln(s)))))))
                  - ln(1))
        + 0.1146)
```

Rust:

```rust
(1.3770_f64
    * (((((sim).clamp(-8.0, 8.0).exp()
          - (((((sim).clamp(-8.0, 8.0).exp() - (rep).clamp(1e-6, 1e8).ln()))
                .clamp(-8.0, 8.0).exp()
              - (((sim).clamp(-8.0, 8.0).exp() - (sim).clamp(1e-6, 1e8).ln()))
                .clamp(1e-6, 1e8).ln()))
            .clamp(1e-6, 1e8).ln()))
          .clamp(-8.0, 8.0).exp()
        - (1.0_f64).clamp(1e-6, 1e8).ln()))
 + 0.1146_f64)
    .clamp(0.0_f64, 1.0_f64)
```

This tree does use `r` (unlike Phase A's snap), is monotone-increasing in
`rep`, has `snap_mean = 0.5293` well inside the `[0.2, 0.8]` band — and is
still dominated by the current hand-coded formula.

## Verdict

**Depth-4 is sufficient to *continuously* beat the current formula but not
sufficient to *discretely* snap to a better one in 10 blind restarts.**

The ~0x depth-3 -> 2.02x depth-4 jump in soft variance is large and real.
The paper's ~25% blind-snap rate at depth 3-4 predicts we would see at least
one snap success in 10 restarts; we saw zero. This suggests:

1. **The continuous optimum lives in a region of parameter space that does not
   discretize cleanly** onto the one-hot vertex set — i.e., the `softmax-snap`
   step destroys most of the achieved soft variance. Hardening over 1500 steps
   at this depth is not enough; the training process keeps multiple logits
   competitive throughout and collapses arbitrarily on snap.
2. **The current hand-coded formula is already near-optimal on the
   discrete-Sheffer-stroke lattice that depth-4 EML can represent**, at least
   for the smooth `sim/rep/current_boost` data distribution in
   `xi_pairs.json`. Beating it in snapped form almost certainly requires
   either (a) depth-5 (paper reports <1% recovery — expensive), (b) a richer
   operator vocabulary, or (c) a different atomic primitive set.
3. **The degeneracy hypothesis for `corpus_xi_diversity` is strengthened but
   not confirmed at depth 4.** Soft-space gains that cannot discretize indicate
   the upstream `xi_operator` continuous signal has real structure the
   hand-coded formula misses, but that structure is not expressible as a
   compact symbolic Sheffer-stroke tree. The encoding_entropy saturation is
   more likely an upstream `xi_operator` issue than a downstream
   `xi_diversity_boost` formula choice.

## Recommended follow-ups

Ranked by expected value:

1. **Accept the hand-coded formula for now.** It dominates every depth-4 snap
   we produced. Do NOT port the Phase B snap winner — it is worse than the
   current formula on every metric (variance 0.0178 vs 0.1153).
2. **Investigate `xi_operator` upstream** rather than the downstream boost
   formula. If soft variance doubles continuously but refuses to discretize,
   the encoding is smooth on the soft manifold and non-compositional on the
   discrete one — look for the signal in `xi_operator` itself.
3. **(Optional, expensive)** Try depth-5 with 20+ restarts and an even longer
   hardening schedule (3000+ steps) if the upstream investigation is
   inconclusive. Paper reports <1% recovery so this is gambling.
4. **Do NOT** port any Phase B formula to `src/bin/research.rs`. The current
   hand-coded formula wins.

## Runtime

Depth-4 x 10 restarts x 2 phases x 10000 steps completed in roughly 42 minutes
on CPU, comfortably inside the 60-minute budget.

## Artifacts

- `scripts/eml_train_xi_depth4.py` — depth-4 trainer (new)
- `experiments/eml_xi_tree_d4.json` — JSON results (new)
- `experiments/l4-s12-report.md` — this report (new)

Reference (unchanged):

- `scripts/eml_train_xi.py` — depth-3 trainer from L4.S0
- `experiments/eml_xi_tree.json` — depth-3 results from L4.S0
- `experiments/l4-s0-report.md` — depth-3 report
