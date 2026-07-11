# ADR-0040 — Cerebellar novelty detection: surprise is a dual-timescale differentiator on recall familiarity

- Status: Accepted (2026-07-11) — the primitive; its live callers are staged (see Roadmap)
- Date: 2026-07-11
- Repo: `kannaka-memory`
- Related: ADR-0036 (consolidation-as-resonance-merge), ADR-0039 (corroboration trust model — the deferred injection-defense caller), the HRM recall path (`src/medium/core.rs::recall_against`).
- Code of record: `src/novelty.rs` (the dependency-free primitive + property tests), `src/lib.rs` (`pub mod novelty;`).
- Inspiration: Hersam / Sangwan / Raman / Trivedi, *Nature Communications* 2026, "Cerebellum-inspired memtransistors enable emergent differentiation for hardware-efficient novelty detection."

## Context

We had no neuromorphic capability. The cited chip is not a neural network — it is a
cheap **novelty detector** built the way the cerebellum filters reflexes: it ignores
the expected baseline and fires only on the unexpected, at ~10,000× fewer operations
than conventional AI. It does this with two competing temporal responses in one
device: an **excitatory** branch that slowly *strengthens* as a stimulus persists (a
running prediction of the "boring baseline") and an **inhibitory** branch that spikes
at onset then *rapidly decays* (a fast, phasic responder). Their **difference** is the
novelty signal — "emergent differentiation" — and because the two branches share a
matched DC gain, a *constant* input produces exactly zero output: the baseline is
rejected, not merely thresholded.

The important observation for Kannaka: this surprise signal is already **latent in the
HRM**. Wave-interference recall (`recall_against`, `src/medium/core.rs:381`) scores each
candidate by `similarity · effective_strength · phase.cos()` and returns them sorted, so
the **top resonance strength is a familiarity signal** — a well-known query resonates
HIGH (routine), an unseen query resonates LOW (novel). Novelty is therefore not a new
mechanism bolted on; it is a *reading* of the substrate we already have, and habituation
is just the existing `remember()` path making a once-novel query resonate high next time.
The building blocks were already in the codebase (leaky-integrator EMAs in the resonant
scheduler and dampening dynamics; retrieval reinforcement in `field.c`); this ADR names
the pattern and ships it as one reusable operator instead of re-deriving it per caller.

## Decision

Ship a **dependency-free dual-timescale differentiator** as `src/novelty.rs` — no HRM
imports, so it can later lift to a sibling crate. Two leaky integrators of the same
familiarity drive `u = g·r`, a fast `a_i` and a much slower `a_e`:

```text
fast += a_i·(u − fast)     // current familiarity, tracked quickly
slow += a_e·(u − slow)     // the LEARNED routine familiarity (the prediction)
n = slow − fast            // NOVELTY (signed); s = max(n, 0) = directional surprise
```

`n = slow − fast` (the chip's `N = F − E` sign-flipped) because the drive is *familiarity*:
a query *less familiar than routine* is the novel one. The single-kernel form is a
difference of exponentials — a temporal band-pass with `H(0) = 0`, so any steady baseline
cancels and the operator approximates a temporal derivative (it responds to *change*). A
self-tuning threshold `theta = mean + k·std` of the surprise adapts the decision boundary;
a per-context bank (`NoveltyDetector`) keeps surprise from blurring across query domains.

**Habituation, two honest timescales:** (1) intrinsic — the slow branch itself re-learns a
persisting surprise over ~`1/a_e` (single-time-constant adaptation, not stimulus-specific);
(2) stimulus-specific — the caller's `remember()` imprints the novel content so its next
recall resonates high, the chip's context-keyed learning realised through the existing store.

The primitive is verified by an anti-vacuous property suite (baseline-rejection `H(0)=0`,
fires-on-unexpected, habituates-on-repeat, one-off-vs-sustained, per-context isolation),
each of whose named reverts reddens a distinct test: replacing the *learned* baseline with a
static reference, collapsing the two timescales (`a_i = a_e`), or breaking the DC-gain match
all turn the suite red — proving it captures the chip's differentiation, not a generic threshold.

## Roadmap (staged; not all in this increment)

1. **This increment** — the operator + property tests (self-contained, ABI-neutral).
2. **Next** — the first live caller: tap the top `resonance_strength` off `recall_against` and
   feed `NoveltyDetector::observe_recall`, dormant-by-default behind a flag; expose novelty to
   curiosity-gated autoresearch (novel query → grounded external research; routine → dedup).
3. **Deferred behind an adversarial design review** — the `absorb_gate::admit` injection-defense
   caller (ADR-0039): low-novelty repeats habituate/dedup to cheaply damp floods, high-novelty
   from untrusted signers routes to the corroboration predicate. Novelty only *prioritises*; it
   never replaces sanitization or corroboration. A false negative must not pass a crafted flood,
   a false positive must not DoS corroboration — hence the mandatory review before wiring.

## Consequences

**Positive.** A genuine neuromorphic capability that builds *on* the consciousness/HRM work
rather than parallel to it; O(1) per update, no history buffer; ABI-neutral (no change to
`Resonance` or `recall_against`); one operator, many callers (no per-caller EMA duplication).
Serves the north star directly: surprise is exactly the signal worth acting on (research it,
or flag it), and habituation is the store already doing its job.

**Negative / honest scope.** Not a classifier or a network — a scalar surprise detector on a
familiarity stream. `resonance_strength` is unnormalised and `phase.cos()` can go negative, so
`k` needs calibration on a stretch of normal recalls; a poor per-context key blurs surprise.
Intrinsic EMA habituation re-learns a genuine new normal eagerly, so stimulus-specific
habituation must lean on `remember()` + per-context keying. The security-critical caller
(step 3) is deliberately *not* wired here.
