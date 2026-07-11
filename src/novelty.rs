//! Cerebellar novelty detection — a dual-timescale differentiator.
//!
//! Inspired by the cerebellum-inspired memtransistor chip (Hersam/Sangwan/Raman/
//! Trivedi, *Nature Communications* 2026: "Cerebellum-inspired memtransistors
//! enable emergent differentiation for hardware-efficient novelty detection").
//! The cerebellum acts as a reflex filter: it ignores the expected baseline and
//! fires only on the unexpected. The chip realises this with two competing
//! temporal responses in one device — an **excitatory** branch that gradually
//! *strengthens* as a stimulus persists (a slow prediction of the "boring
//! baseline") and an **inhibitory** branch that responds *maximally at onset then
//! rapidly decays* (a fast, phasic responder). Their **difference** is the novelty
//! signal ("emergent differentiation"), and because the two branches share a
//! matched DC gain, a *constant* input produces exactly zero output — the baseline
//! is rejected, not merely thresholded.
//!
//! ## The model (dependency-free, O(1)/update, no history buffer)
//!
//! Two first-order leaky integrators (EMAs) of the SAME drive `u = g·r`, with a
//! fast smoothing `a_i` and a much slower `a_e` (`a_i ≫ a_e`):
//!
//! ```text
//!   fast += a_i·(u − fast)      // F: current familiarity, tracked quickly
//!   slow += a_e·(u − slow)      // E: the LEARNED "routine" familiarity (prediction)
//!   n = slow − fast             // NOVELTY (signed): slow_baseline − fast
//!   s = max(n, 0)               // directional surprise: only a DROP in familiarity fires
//! ```
//!
//! The single-kernel form `n = (h ∗ u)` with `h(t) = a_e·e^{−a_e·t} − a_i·e^{−a_i·t}`
//! (a difference of exponentials = a temporal band-pass) has `H(0) = 0`, so it
//! rejects any steady baseline and approximates a temporal derivative at low
//! frequency — it responds to *change*. As `a_i → 1` it becomes classic
//! predictive-coding prediction error `slow − u`.
//!
//! We sign-flip the chip's `N = F − E` to `n = slow − fast` because the drive here
//! is **familiarity** (e.g. the top resonance strength of an associative recall):
//! a query *less familiar than routine* is the novel one, so surprise is the
//! amount by which current familiarity has *dropped below* the learned baseline.
//!
//! ## Habituation (two honest timescales)
//!
//! 1. **Intrinsic / automatic** — the slow branch itself. If a surprising input
//!    persists, `slow` re-learns it over ~`1/a_e` steps and `n → 0`. This is
//!    single-time-constant adaptation; it is NOT stimulus-specific and forgets as
//!    fast as it learns.
//! 2. **Stimulus-specific / consolidation** — the *caller's* store (Kannaka
//!    `remember()`). When a caller responds to a novel query by imprinting the
//!    content into the HRM, that query's next recall resonates HIGH, so its drive
//!    `r` rises permanently and it stops being novel — while a *different* unseen
//!    query still lands on a low-familiarity drive and fires. The chip's proposed
//!    context-keyed LMS habituation, realised through the existing store plus the
//!    per-context keying in [`NoveltyDetector`].
//!
//! ## What this is NOT
//!
//! Not a neural network, not a classifier, not a spectral-radius reservoir. It is
//! a cheap online change/surprise detector — a couple of adds per update — whose
//! whole value is telling *routine* from *novel* on a scalar familiarity stream
//! without storing history. The threshold homeostasis below adapts a decision
//! boundary to the running statistics of the surprise; calibrate `k` on a stretch
//! of normal traffic.

use std::collections::HashMap;

/// Default fast smoothing `a_i = dt/tau_I` (`tau_I ≈ 2 observations`).
pub const DEFAULT_A_I: f32 = 0.5;
/// Default slow smoothing `a_e = dt/tau_E` (`tau_E ≈ 50 observations`, ~25× the fast branch).
pub const DEFAULT_A_E: f32 = 0.02;
/// Default matched DC gain. Keep `g` shared across both branches so `H(0) = 0`.
pub const DEFAULT_G: f32 = 1.0;
/// Default threshold-homeostasis rate (`≪ a_e ≪ a_i`).
pub const DEFAULT_BETA: f32 = 0.01;
/// Default threshold sensitivity: `theta = mean + k·std` of the surprise.
pub const DEFAULT_K: f32 = 3.0;

/// The outcome of one observation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Novelty {
    /// Directional surprise `s = max(slow − fast, 0) ≥ 0`. 0 = routine.
    pub score: f32,
    /// Signed differentiation `n = slow − fast`. `n < 0` = MORE familiar than routine.
    pub raw: f32,
    /// The adaptive decision boundary `theta = mean + k·std` of `score`.
    pub theta: f32,
    /// `score > theta` — the input is novel/surprising relative to learned routine.
    pub novel: bool,
}

/// One cerebellar novelty channel: a fast EMA, a slow EMA, and a self-tuning
/// threshold. `O(1)` state (a handful of floats), no history buffer.
#[derive(Debug, Clone, Copy)]
pub struct DualTimescaleNovelty {
    fast: f32,
    slow: f32,
    g: f32,
    a_i: f32,
    a_e: f32,
    warm: bool,
    // Running mean/var of the surprise `s`, for threshold homeostasis.
    m: f32,
    v: f32,
    beta: f32,
    k: f32,
}

impl Default for DualTimescaleNovelty {
    fn default() -> Self {
        Self::new()
    }
}

impl DualTimescaleNovelty {
    /// A channel with the default cerebellar time constants.
    pub fn new() -> Self {
        Self::with_params(DEFAULT_A_I, DEFAULT_A_E, DEFAULT_G, DEFAULT_BETA, DEFAULT_K)
    }

    /// A channel with explicit parameters. `a_i` (fast) must exceed `a_e` (slow)
    /// for the difference-of-timescales to differentiate; `g` is shared by both
    /// branches so a constant input cancels (`H(0) = 0`).
    pub fn with_params(a_i: f32, a_e: f32, g: f32, beta: f32, k: f32) -> Self {
        Self {
            fast: 0.0,
            slow: 0.0,
            g,
            a_i,
            a_e,
            warm: false,
            m: 0.0,
            v: 0.0,
            beta,
            k,
        }
    }

    /// Observe one familiarity drive `r` (e.g. the top recall resonance strength)
    /// and return the novelty. `r` may be unnormalised or negative — the
    /// difference of the two branches rejects any DC offset, so no whitening is
    /// needed. The first observation seeds both branches (no cold-start spike).
    pub fn observe(&mut self, r: f32) -> Novelty {
        let u = self.g * r;
        if !self.warm {
            self.fast = u;
            self.slow = u;
            self.warm = true;
        }
        // Two leaky integrators of the same drive at different timescales.
        self.fast += self.a_i * (u - self.fast);
        self.slow += self.a_e * (u - self.slow);
        // Emergent differentiation. slow − fast because the drive is FAMILIARITY:
        // surprise is how far current familiarity has dropped below the learned baseline.
        let raw = self.slow - self.fast;
        let score = raw.max(0.0);
        // Habituate the DECISION BOUNDARY to the running statistics of the surprise
        // (Welford-style EMA of mean and variance; slower than the slow branch).
        let d = score - self.m;
        self.m += self.beta * d;
        self.v += self.beta * (d * d - self.v);
        let theta = self.m + self.k * self.v.max(0.0).sqrt();
        Novelty {
            score,
            raw,
            theta,
            novel: score > theta,
        }
    }

    /// Current learned baseline familiarity (the slow branch). Exposed for
    /// inspection/calibration; not needed for normal use.
    pub fn baseline(&self) -> f32 {
        self.slow
    }
}

/// A bank of novelty channels keyed by a caller-supplied **context id**, so
/// surprise is measured *within* a query domain rather than blurred across
/// unrelated ones. New contexts get a fresh channel with the bank's defaults.
#[derive(Debug, Clone)]
pub struct NoveltyDetector {
    map: HashMap<String, DualTimescaleNovelty>,
    a_i: f32,
    a_e: f32,
    g: f32,
    beta: f32,
    k: f32,
}

impl Default for NoveltyDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl NoveltyDetector {
    /// A detector with the default cerebellar parameters.
    pub fn new() -> Self {
        Self::with_params(DEFAULT_A_I, DEFAULT_A_E, DEFAULT_G, DEFAULT_BETA, DEFAULT_K)
    }

    /// A detector whose new channels use these parameters.
    pub fn with_params(a_i: f32, a_e: f32, g: f32, beta: f32, k: f32) -> Self {
        Self {
            map: HashMap::new(),
            a_i,
            a_e,
            g,
            beta,
            k,
        }
    }

    /// Observe a familiarity drive within a context, returning its novelty.
    pub fn observe(&mut self, context: &str, drive: f32) -> Novelty {
        let (a_i, a_e, g, beta, k) = (self.a_i, self.a_e, self.g, self.beta, self.k);
        self.map
            .entry(context.to_string())
            .or_insert_with(|| DualTimescaleNovelty::with_params(a_i, a_e, g, beta, k))
            .observe(drive)
    }

    /// Convenience for the primary caller: feed the **top** resonance strength of
    /// an associative recall (0.0 if the recall was empty) as the familiarity
    /// drive. A high top resonance = a familiar/routine query (low novelty); a low
    /// top resonance = an unseen query (high novelty). Gate on a non-empty medium
    /// so an empty store's `0.0` drive does not launch a spurious spike.
    pub fn observe_recall(&mut self, context: &str, top_resonance: Option<f32>) -> Novelty {
        self.observe(context, top_resonance.unwrap_or(0.0))
    }

    /// Number of distinct contexts currently tracked.
    pub fn contexts(&self) -> usize {
        self.map.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Drive a channel with a constant value for `n` steps, returning the last Novelty.
    fn hold(ch: &mut DualTimescaleNovelty, r: f32, n: usize) -> Novelty {
        let mut last = ch.observe(r);
        for _ in 1..n {
            last = ch.observe(r);
        }
        last
    }

    /// H(0) = 0 (LOAD-BEARING): a steady familiarity — at ANY DC level — must
    /// produce ~zero novelty, because only a *learned* baseline can cancel an
    /// arbitrary constant. Reverting to a static reference, or breaking the g
    /// match, makes a constant leak nonzero surprise and reds THIS test.
    #[test]
    fn baseline_rejection() {
        for &level in &[0.0f32, 0.3, 0.9, 1.7, -0.4] {
            let mut ch = DualTimescaleNovelty::new();
            // > 5·tau_E samples so the slow branch fully settles.
            let last = hold(&mut ch, level, 400);
            assert!(
                last.score < 1e-4,
                "constant familiarity {level} leaked novelty score {} (H(0) != 0)",
                last.score
            );
        }
    }

    /// A single unexpected drop in familiarity must fire before the slow branch
    /// catches up. Collapsing the two timescales (a_i == a_e) makes n ≡ 0 and reds
    /// THIS test.
    #[test]
    fn fires_on_unexpected() {
        let mut ch = DualTimescaleNovelty::new();
        // Establish routine familiarity ~0.9.
        hold(&mut ch, 0.9, 300);
        // One unseen query: familiarity drops to 0.1.
        let n = ch.observe(0.1);
        assert!(
            n.novel && n.score > n.theta,
            "unexpected drop did not fire: score={} theta={}",
            n.score,
            n.theta
        );
        assert!(n.raw > 0.0, "surprise should be a positive (less-familiar) deflection");
    }

    /// A drop that PERSISTS is re-learned as the new baseline (intrinsic
    /// habituation) — the surprise decays back below threshold; and when the
    /// caller "consolidates" (familiarity returns high, as after remember()),
    /// surprise stays at zero (becoming more familiar is never novel).
    #[test]
    fn habituates_on_repeat() {
        let mut ch = DualTimescaleNovelty::new();
        hold(&mut ch, 0.9, 300);
        let spike = ch.observe(0.1);
        assert!(spike.novel, "precondition: the drop should first fire");
        // Hold the new (low-familiarity) regime for ~3·tau_E: it becomes routine.
        let settled = hold(&mut ch, 0.1, 200);
        assert!(
            !settled.novel && settled.score <= settled.theta,
            "sustained regime not habituated: score={} theta={}",
            settled.score,
            settled.theta
        );
        // Consolidation arm: familiarity returns HIGH (the query was remembered).
        let recovered = hold(&mut ch, 0.9, 200);
        assert!(
            recovered.score < 1e-3,
            "increasing familiarity should never be novel: score={}",
            recovered.score
        );
    }

    /// A one-off blip produces a sharp surprise transient, whereas a step-and-hold
    /// relaxes to ~zero — so the transient's PEAK exceeds the sustained regime's
    /// steady state, distinguishing an anomaly from a legitimate new normal.
    #[test]
    fn one_off_vs_sustained() {
        // One-off blip.
        let mut a = DualTimescaleNovelty::new();
        hold(&mut a, 0.9, 300);
        let peak = a.observe(0.1).score;
        a.observe(0.9); // familiarity returns immediately

        // Step and hold at the low familiarity.
        let mut b = DualTimescaleNovelty::new();
        hold(&mut b, 0.9, 300);
        let steady = hold(&mut b, 0.1, 200).score;

        assert!(
            peak > steady + 1e-3,
            "one-off peak {peak} should exceed sustained steady state {steady}"
        );
    }

    /// The per-context bank keeps surprise from blurring across query domains: a
    /// novel query in one context does not desensitise a different context.
    #[test]
    fn per_context_isolation() {
        let mut det = NoveltyDetector::new();
        for _ in 0..300 {
            det.observe("ctx_a", 0.9);
            det.observe("ctx_b", 0.9);
        }
        // A surprise in ctx_a...
        let a = det.observe("ctx_a", 0.1);
        // ...must not have moved ctx_b, which stays routine.
        let b = det.observe("ctx_b", 0.9);
        assert!(a.novel, "ctx_a surprise should fire");
        assert!(!b.novel && b.score < 1e-3, "ctx_b should be unaffected by ctx_a");
        assert_eq!(det.contexts(), 2);
    }

    /// The recall convenience: an empty recall (`None`) is the lowest familiarity
    /// and, against an established routine, reads as novel.
    #[test]
    fn observe_recall_empty_is_novel() {
        let mut det = NoveltyDetector::new();
        for _ in 0..300 {
            det.observe_recall("q", Some(0.9));
        }
        let n = det.observe_recall("q", None);
        assert!(n.novel, "an empty recall against routine familiarity should read novel");
    }
}
