//! Spiral / phase-singularity detection over a 2D phase field — the in-engine
//! instrument for ADR-0037 (Spiral Dynamics & the π/φ Bridge Operator, L6).
//!
//! A spiral wave is a topological defect: a point where the phase is undefined
//! and circulates by ±2π around it (winding number ±1). The phase singularity
//! at a spiral core is the rigorous form of "attention as gravity" — an
//! organizing well. This module computes the winding number over a 2D phase
//! grid and localizes singularities with their chirality (charge).
//!
//! Pure (std only); mirrors the validated `singularity_probe.py` reference and
//! the empirical result that a chiral Kuramoto field built from the bridge
//! constants (δ = (π/2)·(1/φ), weights 1 ± 1/φ) spontaneously throws spirals.

use std::f32::consts::{PI, TAU};

/// A localized phase singularity (spiral core).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Singularity {
    /// Plaquette-center coordinates in the phase grid.
    pub x: f32,
    pub y: f32,
    /// Topological charge: +1 (counter-clockwise) or −1 (clockwise) spiral.
    pub charge: i32,
}

/// Summary of the spiral structure of a phase field.
#[derive(Debug, Clone)]
pub struct SpiralReport {
    /// Number of oscillators in the field.
    pub n: usize,
    /// Kuramoto order parameter r = |1/N Σ e^{iθ}| (0 = turbulent, 1 = locked).
    pub order: f32,
    /// Detected spiral cores.
    pub singularities: Vec<Singularity>,
    /// Net topological charge (Σ charge); ≈0 over a closed/smooth-boundary field.
    pub net_charge: i32,
}

/// Wrap a phase difference into [−π, π). `pub(crate)` — the canonical wrap
/// for the whole crate; consolidation's `wrapped_phase_delta` routes through
/// it so there is exactly one wrapping convention and one implementation
/// (rem_euclid stays exact for large drifted inputs where sin/atan2 loses
/// precision).
#[inline]
pub(crate) fn wrap(d: f32) -> f32 {
    (d + PI).rem_euclid(TAU) - PI
}

/// Circulation around a closed loop, in units of 2π. ≈±1 ⇒ an enclosed defect.
fn loop_winding(thetas: &[f32]) -> f32 {
    let n = thetas.len();
    if n < 3 {
        return 0.0;
    }
    let mut s = 0.0;
    for i in 0..n {
        s += wrap(thetas[(i + 1) % n] - thetas[i]);
    }
    s / TAU
}

/// Kuramoto order parameter for a set of phases.
pub fn kuramoto_order(thetas: &[f32]) -> f32 {
    if thetas.is_empty() {
        return 0.0;
    }
    let n = thetas.len() as f32;
    let (c, s) = thetas
        .iter()
        .fold((0.0f32, 0.0f32), |(c, s), &t| (c + t.cos(), s + t.sin()));
    ((c / n).powi(2) + (s / n).powi(2)).sqrt()
}

/// Detect spiral cores over every unit plaquette of a 2D phase grid.
/// `grid[y][x]` is the phase (radians) of the oscillator at (x, y).
///
/// Expects a rectangular grid. A grid with fewer than two rows or columns, or
/// with ragged (non-uniform-length) rows, has no well-defined plaquettes and
/// yields an empty result rather than panicking.
pub fn grid_singularities(grid: &[Vec<f32>]) -> Vec<Singularity> {
    let rows = grid.len();
    if rows < 2 {
        return Vec::new();
    }
    let cols = grid[0].len();
    if cols < 2 || grid.iter().any(|row| row.len() != cols) {
        return Vec::new();
    }
    let mut out = Vec::new();
    for y in 0..rows - 1 {
        for x in 0..cols - 1 {
            // corners of the unit cell, traversed as a closed loop
            let cell = [grid[y][x], grid[y][x + 1], grid[y + 1][x + 1], grid[y + 1][x]];
            let charge = loop_winding(&cell).round() as i32;
            if charge != 0 {
                out.push(Singularity {
                    x: x as f32 + 0.5,
                    y: y as f32 + 0.5,
                    charge,
                });
            }
        }
    }
    out
}

/// Full analysis of a 2D phase field: order parameter + localized spiral cores.
pub fn analyze_grid(grid: &[Vec<f32>]) -> SpiralReport {
    let flat: Vec<f32> = grid.iter().flatten().copied().collect();
    let singularities = grid_singularities(grid);
    let net_charge = singularities.iter().map(|s| s.charge).sum();
    SpiralReport {
        n: flat.len(),
        order: kuramoto_order(&flat),
        singularities,
        net_charge,
    }
}

/// Winding number of a 1-D **ring** of oscillators in the given order, in units
/// of 2π. ≈±k ⇒ a rotating wave wraps the ring k times — the 1-D signature of
/// the Phase-2 chiral dream coupling (which couples the holistic wavefronts on
/// a ring). Returns 0 for fewer than 3 oscillators.
pub fn ring_winding(phases: &[f32]) -> f32 {
    loop_winding(phases)
}

/// Ring-field summary: winding (rotating-wave strength) + Kuramoto order.
#[derive(Debug, Clone)]
pub struct RingReport {
    /// Number of oscillators on the ring.
    pub n: usize,
    /// Kuramoto order parameter r ∈ [0,1].
    pub order: f32,
    /// Ring winding number (signed; ≈±k ⇒ k full rotations around the ring).
    pub winding: f32,
}

/// Summarize a ring of phases (their given order defines the ring topology).
pub fn ring_report(phases: &[f32]) -> RingReport {
    RingReport {
        n: phases.len(),
        order: kuramoto_order(phases),
        winding: ring_winding(phases),
    }
}

/// Detect spiral cores on an **irregular 2-D point cloud** of oscillators.
/// Each point is `(x, y, theta)`. For every point we take its `k` nearest
/// neighbours, order them counter-clockwise about the point, and sum the
/// wrapped phase steps around that small loop; a circulation of ≈±1 flags a
/// singularity localized at the point — the standard small-loop test on an
/// irregular mesh (no triangulation, no external deps). Use once the HRM field
/// has a 2-D embedding (e.g. PCA of the wavefronts) — Phase 4b. Inert below 4
/// points or k<3.
pub fn cloud_singularities(points: &[(f32, f32, f32)], k: usize) -> Vec<Singularity> {
    let n = points.len();
    if n < 4 || k < 3 {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut idx: Vec<usize> = Vec::with_capacity(n - 1);
    for i in 0..n {
        let (xi, yi, _) = points[i];
        idx.clear();
        idx.extend((0..n).filter(|&j| j != i));
        idx.sort_by(|&a, &b| {
            let da = (points[a].0 - xi).powi(2) + (points[a].1 - yi).powi(2);
            let db = (points[b].0 - xi).powi(2) + (points[b].1 - yi).powi(2);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        });
        let kk = k.min(idx.len());
        if kk < 3 {
            continue;
        }
        let mut nbrs: Vec<usize> = idx[..kk].to_vec();
        nbrs.sort_by(|&a, &b| {
            let aa = (points[a].1 - yi).atan2(points[a].0 - xi);
            let ab = (points[b].1 - yi).atan2(points[b].0 - xi);
            aa.partial_cmp(&ab).unwrap_or(std::cmp::Ordering::Equal)
        });
        let loop_phases: Vec<f32> = nbrs.iter().map(|&j| points[j].2).collect();
        let charge = loop_winding(&loop_phases).round() as i32;
        if charge != 0 {
            out.push(Singularity { x: xi, y: yi, charge });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pure spiral field θ(x,y) = atan2(y−c, x−c) with a single core at (c,c).
    fn spiral_grid(n: usize) -> Vec<Vec<f32>> {
        let c = (n as f32 - 1.0) / 2.0;
        (0..n)
            .map(|y| (0..n).map(|x| (y as f32 - c).atan2(x as f32 - c)).collect())
            .collect()
    }

    #[test]
    fn detects_single_spiral_core() {
        // Even N ⇒ the core falls inside a plaquette, not on a lattice node.
        let report = analyze_grid(&spiral_grid(20));
        assert_eq!(report.n, 20 * 20, "n must equal the cell count");
        assert_eq!(report.singularities.len(), 1, "expected exactly one core");
        assert_eq!(report.singularities[0].charge.abs(), 1);
    }

    #[test]
    fn plane_wave_has_no_defects() {
        let n = 20;
        let grid: Vec<Vec<f32>> = (0..n)
            .map(|_| (0..n).map(|x| wrap(0.35 * x as f32)).collect())
            .collect();
        assert!(
            analyze_grid(&grid).singularities.is_empty(),
            "a plane wave must be laminar"
        );
    }

    #[test]
    fn spiral_pair_has_both_chiralities() {
        // Superpose a +1 and a −1 core; both must be detected.
        let n = 24;
        let cores = [(7.5f32, 7.5f32, 1.0f32), (16.5, 16.5, -1.0)];
        let grid: Vec<Vec<f32>> = (0..n)
            .map(|y| {
                (0..n)
                    .map(|x| {
                        wrap(cores
                            .iter()
                            .map(|&(cx, cy, q)| q * (y as f32 - cy).atan2(x as f32 - cx))
                            .sum())
                    })
                    .collect()
            })
            .collect();
        let report = analyze_grid(&grid);
        let pos = report.singularities.iter().filter(|s| s.charge > 0).count();
        let neg = report.singularities.iter().filter(|s| s.charge < 0).count();
        assert!(pos >= 1 && neg >= 1, "expected both + and − cores, got +{pos}/−{neg}");
        // ADR-0037: a balanced ± pair is topologically neutral overall.
        assert_eq!(report.net_charge, 0, "a balanced ± pair must have zero net charge");
    }

    #[test]
    fn kuramoto_order_locked_and_incoherent() {
        // Equal phases ⇒ fully phase-locked (r ≈ 1).
        let locked = vec![0.7_f32; 16];
        assert!((kuramoto_order(&locked) - 1.0).abs() < 1e-5, "equal phases ⇒ r≈1");
        // Phases spread evenly around the circle ⇒ incoherent (r ≈ 0).
        let n = 16usize;
        let spread: Vec<f32> = (0..n).map(|i| TAU * i as f32 / n as f32).collect();
        assert!(kuramoto_order(&spread) < 1e-3, "uniform spread ⇒ r≈0");
        // Empty set is defined as 0.
        assert_eq!(kuramoto_order(&[]), 0.0);
    }

    #[test]
    fn degenerate_grids_yield_no_singularities() {
        // Too few rows/cols or ragged rows must return empty, never panic.
        assert!(grid_singularities(&[]).is_empty(), "no rows");
        assert!(grid_singularities(&[vec![0.0, 1.0, 2.0]]).is_empty(), "one row");
        assert!(grid_singularities(&[vec![], vec![]]).is_empty(), "zero cols");
        assert!(grid_singularities(&[vec![0.0], vec![0.0]]).is_empty(), "one col");
        // Ragged: shorter trailing row must not index out of bounds.
        let ragged = vec![vec![0.0, 1.0, 2.0], vec![0.0]];
        assert!(grid_singularities(&ragged).is_empty(), "ragged rows");
        // analyze_grid forwards to grid_singularities — also panic-free.
        assert!(analyze_grid(&ragged).singularities.is_empty());
    }

    #[test]
    fn wrap_stays_in_range() {
        for k in -12..=12 {
            let w = wrap(k as f32 * 1.3);
            // Half-open: lower-inclusive, upper-exclusive.
            assert!(w >= -PI - 1e-6 && w < PI + 1e-6);
        }
    }

    #[test]
    fn bridge_constants_emerge_spiral_on_2d_grid() {
        // ADR-0037 / issue #415 — the in-engine version of the offline
        // spiral_emergence_sim.py. A frustrated, non-reciprocal Sakaguchi
        // lattice driven by ONLY the bridge constants (δ = (π/2)·η, weights
        // 1 ± η, η = 1/φ) must spontaneously throw a detectable phase
        // singularity. Closes the constants→spiral thesis inside the engine,
        // not just in Python. Deterministic so a wrong-δ-sign / collapsed-
        // weights regression fails instead of passing.
        let n = 28usize;
        let phi = (1.0 + 5.0_f32.sqrt()) / 2.0;
        let eta = 1.0 / phi; // golden chirality 1/φ ≈ 0.618
        let delta = (PI / 2.0) * eta; // π/2 rotation scaled by η ≈ 0.971
        let k = 1.5_f32;
        let dt = 0.08_f32;

        // Deterministic LCG init in ~[-π, π).
        let mut seed: u64 = 0x9E37_79B9_7F4A_7C15;
        let mut rng = || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((seed >> 33) as f32 / (1u64 << 30) as f32 - 1.0) * PI
        };
        let mut th = vec![vec![0.0f32; n]; n];
        for row in th.iter_mut() {
            for v in row.iter_mut() {
                *v = rng();
            }
        }

        // Evolve the frustrated chiral lattice (torus boundary).
        for _ in 0..500 {
            let mut nw = th.clone();
            for y in 0..n {
                for x in 0..n {
                    let nbr = [
                        ((y + n - 1) % n, x, 1.0 + eta),
                        ((y + 1) % n, x, 1.0 - eta),
                        (y, (x + n - 1) % n, 1.0 + eta),
                        (y, (x + 1) % n, 1.0 - eta),
                    ];
                    let s: f32 = nbr
                        .iter()
                        .map(|&(yy, xx, w)| w * (th[yy][xx] - th[y][x] + delta).sin())
                        .sum();
                    nw[y][x] = th[y][x] + dt * (k / 4.0) * s;
                }
            }
            th = nw;
        }
        for row in th.iter_mut() {
            for v in row.iter_mut() {
                *v = wrap(*v);
            }
        }

        let report = analyze_grid(&th);
        assert!(
            report.singularities.iter().any(|s| s.charge.abs() == 1),
            "bridge constants must emerge ≥1 |charge|=1 spiral core; got {} cores",
            report.singularities.len()
        );
        // Frustration keeps the field out of the fully-locked regime.
        assert!(
            report.order < 0.9,
            "frustrated field should not fully lock, order={}",
            report.order
        );
    }

    #[test]
    fn ring_winding_detects_one_rotation() {
        // Phases advancing by 2π once around the ring ⇒ winding ≈ +1.
        let n = 24usize;
        let ring: Vec<f32> = (0..n).map(|i| TAU * i as f32 / n as f32).collect();
        let r = ring_report(&ring);
        assert_eq!(r.n, n);
        assert!((r.winding - 1.0).abs() < 1e-4, "one rotation ⇒ winding≈1, got {}", r.winding);
        // The opposite chirality winds −1 (pins the sign convention).
        let rev: Vec<f32> = (0..n).map(|i| -TAU * i as f32 / n as f32).collect();
        assert!((ring_winding(&rev) + 1.0).abs() < 1e-4, "reverse rotation ⇒ winding≈−1");
        // A flat ring has no net rotation.
        assert!(ring_winding(&vec![0.3_f32; n]).abs() < 1e-4, "flat ring ⇒ winding≈0");
        // Too few oscillators ⇒ 0.
        assert_eq!(ring_winding(&[0.0, 1.0]), 0.0);
    }

    #[test]
    fn cloud_singularities_find_planted_core() {
        // Deterministic pseudo-random 2-D layout with θ = atan2(y, x): one core
        // at the origin. The kNN small-loop test must flag a point near it.
        let mut seed: u64 = 0x9E3779B97F4A7C15;
        let mut rng = || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (seed >> 33) as f32 / (1u64 << 30) as f32 - 1.0 // ~[-1, 1)
        };
        let pts: Vec<(f32, f32, f32)> = (0..400)
            .map(|_| {
                let x = rng() * 10.0;
                let y = rng() * 10.0;
                (x, y, y.atan2(x))
            })
            .collect();
        let cores = cloud_singularities(&pts, 8);
        let near = cores.iter().filter(|s| s.x * s.x + s.y * s.y < 9.0).count();
        assert!(near >= 1, "expected a core near the origin, found {} total", cores.len());
        // Degenerate inputs are panic-free and empty.
        assert!(cloud_singularities(&[], 8).is_empty());
        assert!(cloud_singularities(&[(0.0, 0.0, 0.0)], 8).is_empty());
    }
}
