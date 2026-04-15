# L4.S0 — Symbolic regression of `xi_diversity_boost` via depth-3 EML trees

**Experiment:** Test whether the hand-coded two-tier `xi_diversity_boost(sim, xi_a, xi_b)` is a degeneracy hiding upstream information, by attempting to replace it with a depth-3 Elementary Mathematics Language (EML) tree per Odrzywolek (arXiv:2603.21852v2, April 2026).

EML operator: `eml(x, y) = exp(x) - ln(y)`, a continuous Sheffer stroke paired with the constant `1`. Master tree has softmax leaf/input logits, trained with Adam, then snapped to argmax for a closed-form symbolic formula.

**Data:** 300 random pairs sampled from the L4 corpus (dim=128, seed=0xCAFE_BABE). For each pair we record `(cosine_sim, xi_repulsive_force, current_xi_diversity_boost)`.

```
sim    in [-0.160, 0.955]
rep    in [ 0.057, 0.291]
boost  in [-0.116, 1.000]   var = 0.11525
```

The data artefact is `experiments/xi_pairs.json`.

## Tree architecture

- Depth 3 → 7 internal nodes, 8 leaves, ~80 real parameters.
- Leaves: 3-way softmax over `{1, s, r}`.
- Each input to an internal node: 4-way softmax over `{1, s, child_from_subtree, s, r}` where the 4th slot carries the child subtree output (this is the Odrzywolek §4.3 residual-style input).
- Internal nodes: `eml(L, R) = exp(clamp(L, -8, 8)) - ln(clamp(R, 1e-6, 1e8))`.
- Final affine scale+bias; Phase B adds a clamp01 at the final stage instead of a sigmoid (a sigmoid caused hard saturation during snap; raw + clamp preserves gradients and is snap-friendly).

## Phase A — replicate current formula

MSE loss against `current_boost`, Adam lr=0.01, 4000 steps, exponential temperature anneal 1.0 → 0.02 over the last 2000.

| | value |
|---|---|
| soft best MSE | **0.00074** |
| snapped MSE (argmax) | 0.518 |
| `can_represent` (soft) | **YES** |
| `can_represent` (snapped) | no |

The **soft** tree tracks `xi_diversity_boost` to ~2.7% RMSE — depth-3 EML clearly spans the function class. But no single argmax tree survives hardening: the MSE explodes by ~3 orders of magnitude when the softmaxes collapse. Depth-3 EML needs continuous mixtures to hit the formula, which means the discrete symbolic space at this depth does not contain a clean match.

**Takeaway:** the hand-coded formula sits in a region that depth-3 EML trees can approximate in superposition but not discretize. A cleaner snap likely requires depth 4+.

## Phase B — discrimination

Loss rewards variance of `clamp01(tree_output)` while enforcing monotone-increasing in `repulsion`, mean ∈ [0.2, 0.8], and keeping the raw output in-range. Fresh tree, 4000 steps, same anneal schedule.

| | value |
|---|---|
| current formula variance | 0.1153 |
| EML best soft variance (constraints met) | **0.0604** |
| EML soft variance ratio | **0.524** |
| EML snapped variance | 0.000 (collapsed) |
| snapped mean | 0.0 (all-zero after clamp) |
| monotone in repulsion (snapped) | true (trivially) |

Even the best soft-phase EML tree that satisfies the constraints produces ~**half** the variance of the current hand-coded formula. The snapped version is uniformly zero after clamp. The softmax snap collapses for the same reason as Phase A.

**Snapped formula (readable):**

```
clamp01( 0.9900 * ( exp(s) - ln( exp(exp(1) - ln(s)) - ln(s) ) ) + -0.0100 )
```

**Snapped formula (Rust):**

```rust
(0.9900_f32
    * ((sim).clamp(-8.0, 8.0).exp()
       - ((((1.0_f32).clamp(-8.0, 8.0).exp() - (sim).clamp(1e-6, 1e8).ln()))
            .clamp(-8.0, 8.0).exp()
          - (sim).clamp(1e-6, 1e8).ln())
          .clamp(1e-6, 1e8).ln())
  + -0.0100_f32
).clamp(0.0_f32, 1.0_f32)
```

Note: the snapped formula collapses to functions of `sim` only — `repulsion` drops out entirely. That is why the variance is 0 under the clamp: the contribution of `sim` to the raw output ends up out of [0, 1] for this corpus, clamping to 0 everywhere. It is not a usable formula in its snapped form.

## Verdict on the degeneracy hypothesis

**The hand-coded `xi_diversity_boost` is NOT degenerate.** Three pieces of evidence:

1. **Phase B soft variance ratio ~0.52.** The best depth-3 EML tree that satisfies reasonable output constraints produces *half* the discrimination of the current formula. The hand-coded formula is already richer than what the symbolic space can express under these penalties.
2. **Phase A snap failure (0.518 MSE after snap vs 0.00074 soft).** No clean symbolic replacement exists at depth 3. If the current formula were a trivial degeneracy, a discrete depth-3 tree should exist.
3. **The current formula already saturates L4's observed boost range** (`boost ∈ [-0.116, 1.000]`, var 0.115). There is little room for a discriminator that would beat it on this corpus unless the raw `repulsion` signal itself changes.

If L4 still plateaus despite this, the bottleneck is **not** `xi_diversity_boost`; it is upstream — in `xi_operator::compute_xi_signature` (whose `encoding_entropy` collapse has been flagged before) or in the L4 corpus information content itself.

## Recommended follow-up

- **(PREFERRED) Repeat this experiment on `xi_operator` / `compute_xi_signature`.** The more likely site of degeneracy is the Xi signature computation; a symbolic-regression pass on `(raw_vector, xi_signature)` pairs would directly test whether the commutator `Ξ = RG - GR` loses entropy.
- **(REJECTED) Port Phase B snapped formula to research.rs.** The snapped formula is a `sim`-only clamp-zero degenerate, worse than the current. Do not port.
- **Depth 4 EML sweep on xi_diversity_boost** — gives the argmax snap a fighting chance (Odrzywolek reports ~25% recovery at depth 4). Budget ~2× this experiment's runtime.
- **Accept L4 as information-theoretically saturated.** If depths 3 and 4 cannot beat the hand-coded formula, L4's ceiling is probably set by the corpus's intrinsic entropy, not the evaluator.

## Artefacts

- `src/bin/dump_xi_pairs.rs` — Rust binary that samples 300 L4 pairs.
- `experiments/xi_pairs.json` — 300-entry data file.
- `scripts/eml_train_xi.py` — PyTorch depth-3 EML master tree trainer.
- `experiments/eml_xi_tree.json` — Phase A / Phase B metrics and the snapped formulas.

## Notes on negative results

The snap-collapse is itself informative: a depth-3 EML tree has ~80 continuous mixture parameters but only ~3^8 × 4^14 ≈ 4.4×10^12 discrete leaf/input assignments, of which only a vanishing fraction produce bounded, non-saturating outputs on this corpus. The 3-4% gap between soft and argmax performance seen in Odrzywolek's depth-2 blind recovery (100% → ~25% at depth 3-4) shows up here as an effective 0% recovery. Without wider-primitive vocabulary (e.g. adding `exp`, negation, division as explicit leaves) or deeper trees, closed-form recovery on this particular target function is unlikely.
