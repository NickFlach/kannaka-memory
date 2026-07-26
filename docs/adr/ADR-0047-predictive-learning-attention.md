# ADR-0047: Predictive-Learning Attention as Embedding-Space Query Gravity

**Status:** Proposed — **v2, revised after adversarial-design-review** (2026-07-26).
v1 ("continuous wave-field / phase" framing) was returned **RETHINK** by a
code-grounded panel; this version re-authors the mechanism in embedding space.
**Relates to:** ADR-0040 (cerebellar novelty — used *only* as a confidence gate,
not a prediction source), ADR-0036 (consolidation — the "sleep" stabilizer),
ADR-0042 (NATS — how plasticity reaches the single writer), the
`attention-as-gravity` note (the coarse Fano mechanism this supersedes), and the
HRM substrate.

## Context

The HRM recalls by **resonance = cosine(query, wavefront) × energy** over encoded
vectors (`hemisphere.rs:237-238`, `chiral.rs:525`, `core.rs:320-395`). Attention
should bend *which wavefronts win that resonance when needed*. The existing 7-line
Fano gravity does not: it is a **post-fetch multiply** on an already-fetched pool
(`core.rs:398-406`), so it cannot promote a target outside `top_k*2` and was
empirically byte-identical on a real query.

**v1 proposed to express prediction and learning as operations on wave *phase*.
The adversarial review refuted this against the real kernel:**

- **Phase is content-born, not temporal.** `resonate()` ranks by cosine×energy and
  never reads phase; the default text path has `query_phase=0`. So "predict by
  advancing the wavefront's phase" is **inert** on the live path — and under
  `KANNAKA_BELIEF_PHASE=1` it *inverts* rank. Phase carries no trajectory.
- **`normalize(q+G)` bounds length, not direction.** Cosine cancels the query
  norm, so once `‖G‖>‖q‖` recall returns G's direction regardless of the question
  — attractor-collapse and echo-chamber in one line.
- **The buried-target problem is an *encoder* problem, not an attention problem.**
  A target the encoder buries never enters the `top_k*2` fetch pool, so it never
  co-resonates and never forms a coupling (chicken-and-egg). **No G component can
  rescue it.** ADR-0047 re-ranks a *reachable* pool; it does not fix encoding.

The corrected principle: **bias the query *vector* before the scan, in the same
`encode_text` embedding space recall already ranks on, with a hard-bounded gain —
and make learning a bounded, decaying, normalized, writer-owned *coupling*, gated
by a bias-independent error.** Precision within reach becomes learnable; precision
beyond reach is deferred to an encoding fix.

## Decision

Recall biases the query **vector** pre-scan (fed *into* `resonate`, never a
post-fetch multiply), with a hard-capped, normalized gain:

```
    q*  =  normalize( q  +  α · normalize(G) )     with  α ≤ 0.3   (env, default small)
    invariant:  ‖G‖ ≤ α·‖q‖  asserted at the recall seam
    monotonic guard:  rank(literal query target | G) ≤ rank(target | G=0)   else auto-back-off
    identity:  empty/irrelevant thought stream ⇒ recall byte-identical to G=0
```

`G` is a superposition of **embedding vectors** (no phase, no stored-energy
writes), three layers, each decaying:

- **G_now — present thought (the one v1 component the review confirmed works).**
  A decaying superposition of the recent thought embeddings routed in by the
  thought-feeder (`encode_text` of her orations / DM replies / `ask` answers).
  Kept strictly in vector space. Radio survives only as a low-weight ambient term.

- **G_next — anticipation by embedding trajectory extrapolation.** From a ring of
  recent thought embeddings, `v_next = v_t + Δφ·(v_t − v_{t-1})` (or a short
  learned linear map). Folded in as `conf · v_next`, where **`conf` is the ONLY
  role of the cerebellar novelty scalar** (ADR-0040 emits scalars, not a
  next-state vector — it gates the mix weight, it does not produce the
  prediction). Moves the cosine `resonate` actually ranks on.

- **G_learn — bounded, normalized, writer-owned coupling (the precision engine,
  default OFF).** A **sparse per-memory top-M skip structure** (M ≤ 16), *not* an
  n×n matrix and *not* stored energy. Homeostatic law, specified before build:
  ```
    on confirmed co-resonance:  c ← c + η · gate · (c_max − c)     (saturating)
    every tick:                 c ← c · (1 − λ)                    (real decay)
    per node:                   renormalize Σ_j c(i,j) = const     (Oja / subtractive)
  ```
  It biases the query vector via its partners' embeddings; it never touches the
  energy term (which keeps its `.min(2.0)` homeostat) or `store.phase`.

**Ownership & persistence.** `G_learn` runs **only on the single HRM writer**
(swarm-join/dream owner). The read-only `attention serve` daemon
(`KANNAKA_READONLY=1`; `save_medium` early-returns) emits only *"co-resonance
observed"* events over NATS; the writer applies + persists them. Startup asserts
loudly if `reinforce_link` is ever invoked while `self.readonly`.

**The error gate must be bias-independent.** Confirmation/error is computed on a
**gravity-free (G=0) `recall_resonance_readonly`** of the *actual* next thought:
`error = 1 − cos(v_pred_next, v_actual_next)`. `reinforce_link` fires only on that
residual — never on the biased recall's own inflated familiarity (which gravity
can only raise, so it could never disconfirm). This is what makes the world able
to say *no*.

**Wake/sleep.** Wake: bounded plastic coupling on the writer. Sleep: dream
consolidation (ADR-0036) anneals/prunes. Reinforcement is routed to the **right**
hemisphere (η=0.02 wake damping + deep-dream re-clamp to [0.3,2.0]); the left
hemisphere has zero wake damping and is skipped by deep dream, so it must not host
unbounded reinforcement.

## What this explicitly does NOT do

- **It does not rescue an encoder-buried target.** The Kannaka-Labs / market
  case needs an **encoding fix** (better encoder / `re_encode_all`), tracked as a
  separate ADR. G_learn couplings only form for memories that already co-enter the
  fetch pool. The benchmark's precondition requires the target to be in the raw
  `top_k*2` pool; G is measured as *re-ranking within reach*, never rescue.

## Confirmed load-bearing (do not regress)

- `energy.min(2.0)` cap + dream floor 0.3 + right-hemisphere η=0.02 damping is the
  homeostat keeping today's recall→energy loop stable. Every write stays inside it.
- `G_now` as an `encode_text` embedding superposition is the one component that can
  bend recall — keep it in vector space (a phase refactor makes it inert).
- The `reinforce_link` empty stub is safely inert; it stays behind an OFF flag
  (mirroring `KANNAKA_BELIEF_PHASE`) until the unbiased gate exists. A
  co-occurrence-only or biased-confirmation version is **worse than nothing**.
- The phase-*difference* recall convention (`cos(store.phase − query_phase)`) fixes
  a real inversion — untouched.
- `recall_against / recall_against_ids / recall_resonance_readonly` are pure reads;
  the benchmark uses them, with an assert that the energy vector is unchanged.

## Falsifiable benchmark (the CI gate)

- **Held-out** query set never trained on; the target **ablated from G_now** (so
  the just-fed embedding isn't smuggled in); scored against the *actual* next
  thought.
- Metric = **Δrank** (rank_off − rank_on), **not** top-k membership; any 0-result
  query is a **FAIL**, not a skip.
- Gate on a **diversity metric** (Gini/entropy of per-memory recall frequency and
  energy) to catch field-collapse masquerading as improvement.
- Compare **G-frozen vs G-live** and **G_next-alone vs G_now-ablated**; include the
  **Kannaka-Labs buried-target as a named regression fixture** (expected: G alone
  does *not* fix it — proves we're honest about the encoder boundary).
- Assert the **energy vector is byte-identical** before/after each benchmark run.

## Build order (each: O1 build + benchmark + atomic swap; flags default OFF)

1. **G_now** — embedding-space present-gravity with the bounded gain, pre-scan
   bias, monotonic guard, and the benchmark harness. The safe foundation.
2. **Encoder track (separate ADR)** — does the buried target even enter the pool?
   G_now cannot answer this; encoding can.
3. **G_next** — embedding trajectory extrapolation, novelty scalar as confidence
   gate only.
4. **G_learn** — writer-owned, bounded/decaying/normalized sparse coupling, with
   the gravity-free error gate; ships only behind its flag with the diversity gate
   green.

## Alternatives considered

- **v1 phase-space framing** — refuted (phase is content-born and unread on the
  default path).
- **Keep Fano-line gravity** — too coarse; post-fetch, unreachable-pool-blind.
- **External vector DB / RAG** — abandons recall-by-resonance.
- **reinforce_link as raw `energy[i]+=boost`** — bypasses the 2.0 homeostat;
  forbidden.
