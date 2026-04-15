"""L4.S12 depth-4 EML symbolic regression retry for xi_diversity_boost.

Follow-up to L4.S0 (depth-3) which established that depth-3 EML could
SOFT-fit the current xi_diversity_boost formula but could not SNAP to it.
The Odrzywolek paper (arXiv:2603.21852v2) reports ~25% blind-snap recovery
at depth 3-4, so depth-4 gives a larger discrete search space.

Depth-4 binary tree:
  - 15 internal nodes (heap indices 1..15)
  - 16 leaves (heap indices 16..31)
  - Each leaf: 3-way softmax over {1, s, r}
  - Each internal-node input: 4-way softmax over {1, s, r, child_raw}
  - 15*2*4 + 16*3 = 168 softmax params (+ scale/bias)

10 independent random restarts per phase; best-by-final-loss retained.
"""

from __future__ import annotations

import json
import math
import subprocess
import sys
from pathlib import Path

try:
    import torch
except ImportError:
    subprocess.run(
        [sys.executable, "-m", "pip", "install", "torch", "--quiet"],
        check=True,
    )
    import torch  # type: ignore  # noqa: E402

torch.set_default_dtype(torch.float64)

REPO = Path(__file__).resolve().parents[1]
EXP = REPO / "experiments"
PAIRS_PATH = EXP / "xi_pairs.json"
OUT_PATH = EXP / "eml_xi_tree_d4.json"


# ---------------------------------------------------------------------------
# EML depth-4 tree
# ---------------------------------------------------------------------------

DEPTH = 4
N_INTERNAL = 2 ** DEPTH - 1  # 15
N_LEAVES = 2 ** DEPTH        # 16

LEAF_VOCAB = 3     # {1, s, r}
INPUT_VOCAB = 4    # {1, s, r, child}

EXP_CLAMP = 8.0
LN_FLOOR = 1e-6
LN_CEIL = 1e8

N_RESTARTS = 10
PHASE_A_STEPS = 10000
PHASE_A_ANNEAL_START = 8500  # last 1500 steps push to one-hot
PHASE_B_STEPS = 10000
PHASE_B_HARDENING_START = 8500
LR = 0.005


def eml(lhs: torch.Tensor, rhs: torch.Tensor) -> torch.Tensor:
    l = torch.clamp(lhs, min=-EXP_CLAMP, max=EXP_CLAMP)
    r = torch.clamp(rhs, min=LN_FLOOR, max=LN_CEIL)
    return torch.exp(l) - torch.log(r)


def soft_one_hot(logits: torch.Tensor, temperature: float) -> torch.Tensor:
    return torch.softmax(logits / max(temperature, 1e-6), dim=-1)


class EmlTreeD4(torch.nn.Module):
    """Depth-4 EML master tree."""

    def __init__(self) -> None:
        super().__init__()
        # Small random init so different seeds explore different basins.
        self.leaf_logits = torch.nn.Parameter(torch.randn(N_LEAVES, LEAF_VOCAB) * 0.3)
        self.input_logits = torch.nn.Parameter(
            torch.randn(N_INTERNAL, 2, INPUT_VOCAB) * 0.3
        )
        self.out_scale = torch.nn.Parameter(torch.tensor(1.0))
        self.out_bias = torch.nn.Parameter(torch.tensor(0.0))

    def n_params(self) -> int:
        return sum(p.numel() for p in self.parameters())

    def forward(
        self,
        s: torch.Tensor,
        r: torch.Tensor,
        temperature: float = 1.0,
        sigmoid_out: bool = False,
    ) -> torch.Tensor:
        one = torch.ones_like(s)
        leaf_primitives = torch.stack([one, s, r], dim=-1)  # [N, 3]

        leaf_mix = soft_one_hot(self.leaf_logits, temperature)  # [16, 3]
        leaf_values = leaf_primitives @ leaf_mix.T  # [N, 16]

        internal_values: list[torch.Tensor | None] = [None] * (N_INTERNAL + 1)

        # Bottom-up evaluation: node indices N_INTERNAL..1
        for k in range(N_INTERNAL, 0, -1):
            left_child = 2 * k
            right_child = 2 * k + 1
            if left_child > N_INTERNAL:
                # Leaves: heap indices left_child, right_child → leaf slots
                leaf_l = left_child - (N_INTERNAL + 1)
                leaf_r = right_child - (N_INTERNAL + 1)
                child_l = leaf_values[:, leaf_l]
                child_r = leaf_values[:, leaf_r]
            else:
                child_l = internal_values[left_child]
                child_r = internal_values[right_child]
                assert child_l is not None and child_r is not None

            mix = soft_one_hot(self.input_logits[k - 1], temperature)  # [2, 4]
            input_vocab_l = torch.stack([one, s, r, child_l], dim=-1)
            input_vocab_r = torch.stack([one, s, r, child_r], dim=-1)
            lhs = (input_vocab_l * mix[0]).sum(dim=-1)
            rhs = (input_vocab_r * mix[1]).sum(dim=-1)

            node_val = eml(lhs, rhs)
            internal_values[k] = node_val

        root = internal_values[1]
        assert root is not None
        out = self.out_scale * root + self.out_bias
        if sigmoid_out:
            out = torch.sigmoid(out)
        return out

    @torch.no_grad()
    def snap(self) -> None:
        leaf_idx = self.leaf_logits.argmax(dim=-1)
        self.leaf_logits.zero_()
        for i, idx in enumerate(leaf_idx):
            self.leaf_logits[i, idx] = 20.0

        input_idx = self.input_logits.argmax(dim=-1)
        self.input_logits.zero_()
        for k in range(N_INTERNAL):
            for side in range(2):
                self.input_logits[k, side, input_idx[k, side]] = 20.0

    @torch.no_grad()
    def readable(self, sigmoid_out: bool, clamp01: bool = False) -> tuple[str, str]:
        leaf_tokens_human = ["1", "s", "r"]
        input_tokens_human = ["1", "s", "r", "c"]
        leaf_tokens_rust = ["1.0_f32", "sim", "rep"]
        input_tokens_rust = ["1.0_f32", "sim", "rep", "c"]

        leaf_choice = self.leaf_logits.argmax(dim=-1).tolist()
        input_choice = self.input_logits.argmax(dim=-1).tolist()

        def leaf_expr(leaf_idx: int, kind: str) -> str:
            choice = leaf_choice[leaf_idx]
            return (leaf_tokens_human if kind == "human" else leaf_tokens_rust)[choice]

        def input_expr(node_k: int, side: int, child_expr: str, kind: str) -> str:
            choice = input_choice[node_k - 1][side]
            if choice == 3:
                return child_expr
            toks = input_tokens_human if kind == "human" else input_tokens_rust
            return toks[choice]

        def eml_expr(lhs: str, rhs: str, kind: str) -> str:
            if kind == "human":
                return f"(exp({lhs}) - ln({rhs}))"
            return (
                f"(({lhs}).clamp(-8.0, 8.0).exp() "
                f"- ({rhs}).clamp(1e-6, 1e8).ln())"
            )

        def build_node(k: int, kind: str) -> str:
            left = 2 * k
            right = 2 * k + 1
            if left > N_INTERNAL:
                child_l = leaf_expr(left - (N_INTERNAL + 1), kind)
                child_r = leaf_expr(right - (N_INTERNAL + 1), kind)
            else:
                child_l = build_node(left, kind)
                child_r = build_node(right, kind)
            lhs = input_expr(k, 0, child_l, kind)
            rhs = input_expr(k, 1, child_r, kind)
            return eml_expr(lhs, rhs, kind)

        human_core = build_node(1, "human")
        rust_core = build_node(1, "rust")
        scale = float(self.out_scale.item())
        bias = float(self.out_bias.item())
        human = f"{scale:.4f} * {human_core} + {bias:.4f}"
        rust = f"{scale:.4f}_f64 * ({rust_core}) + {bias:.4f}_f64"
        if sigmoid_out:
            human = f"sigmoid({human})"
            rust = f"1.0 / (1.0 + (-({rust})).exp())"
        if clamp01:
            human = f"clamp01({human})"
            rust = f"({rust}).clamp(0.0_f64, 1.0_f64)"
        return human, rust


# ---------------------------------------------------------------------------
# data loading
# ---------------------------------------------------------------------------

def load_pairs() -> tuple[torch.Tensor, torch.Tensor, torch.Tensor]:
    with open(PAIRS_PATH, "r", encoding="utf-8") as f:
        raw = json.load(f)
    sim = torch.tensor([p["sim"] for p in raw])
    rep = torch.tensor([p["repulsion"] for p in raw])
    boost = torch.tensor([p["current_boost"] for p in raw])
    return sim, rep, boost


# ---------------------------------------------------------------------------
# Phase A: replicate current formula (MSE against boost)
# ---------------------------------------------------------------------------

def phase_a_single(
    seed: int,
    sim: torch.Tensor,
    rep: torch.Tensor,
    boost: torch.Tensor,
) -> dict:
    torch.manual_seed(seed)
    model = EmlTreeD4()
    opt = torch.optim.Adam(model.parameters(), lr=LR)

    best_soft_mse = float("inf")
    best_snap_mse = float("inf")
    best_snap_state = None
    last_loss = float("nan")

    for step in range(PHASE_A_STEPS):
        if step < PHASE_A_ANNEAL_START:
            temp = 1.0
        else:
            frac = (step - PHASE_A_ANNEAL_START) / max(
                1, PHASE_A_STEPS - PHASE_A_ANNEAL_START
            )
            temp = max(0.02, math.exp(math.log(0.02) * frac))

        out = model(sim, rep, temperature=temp, sigmoid_out=False)
        loss = torch.nn.functional.mse_loss(out, boost)
        if not torch.isfinite(loss):
            return {
                "seed": seed,
                "diverged": True,
                "soft_mse": float("inf"),
                "snap_mse": float("inf"),
            }
        opt.zero_grad()
        loss.backward()
        torch.nn.utils.clip_grad_norm_(model.parameters(), 5.0)
        opt.step()
        last_loss = float(loss.item())
        best_soft_mse = min(best_soft_mse, last_loss)

        if step % 500 == 0 or step == PHASE_A_STEPS - 1:
            saved = {k: v.detach().clone() for k, v in model.state_dict().items()}
            model.snap()
            with torch.no_grad():
                snap_out = model(sim, rep, temperature=0.01, sigmoid_out=False)
                snap_mse = float(torch.nn.functional.mse_loss(snap_out, boost).item())
            if math.isfinite(snap_mse) and snap_mse < best_snap_mse:
                best_snap_mse = snap_mse
                best_snap_state = {
                    k: v.detach().clone() for k, v in model.state_dict().items()
                }
            model.load_state_dict(saved)

    # Final snapshot from best snap state.
    if best_snap_state is not None:
        model.load_state_dict(best_snap_state)
        human, rust = model.readable(sigmoid_out=False)
    else:
        model.snap()
        human, rust = model.readable(sigmoid_out=False)

    return {
        "seed": seed,
        "diverged": False,
        "soft_mse": best_soft_mse,
        "snap_mse": best_snap_mse,
        "final_soft_mse": last_loss,
        "snapped_formula_readable": human,
        "snapped_formula_rust": rust,
    }


def phase_a(
    sim: torch.Tensor, rep: torch.Tensor, boost: torch.Tensor
) -> dict:
    print(f"\n=== Phase A: replicate current formula ({N_RESTARTS} restarts) ===")
    results = []
    for seed in range(N_RESTARTS):
        r = phase_a_single(seed, sim, rep, boost)
        status = "DIVERGED" if r.get("diverged") else "ok"
        print(
            f"[A] seed={seed} {status} "
            f"soft_mse={r['soft_mse']:.6f} snap_mse={r['snap_mse']:.6f}"
        )
        if not r.get("diverged"):
            results.append(r)

    # Best by soft, best by snap.
    if not results:
        return {
            "best_soft_mse": float("inf"),
            "best_snap_mse": float("inf"),
            "n_restarts": N_RESTARTS,
            "n_successful": 0,
            "best_snapped_formula_readable": "",
            "best_snapped_formula_rust": "",
        }
    best_soft = min(r["soft_mse"] for r in results)
    best_snap_row = min(results, key=lambda r: r["snap_mse"])
    return {
        "best_soft_mse": float(best_soft),
        "best_snap_mse": float(best_snap_row["snap_mse"]),
        "n_restarts": N_RESTARTS,
        "n_successful": len(results),
        "best_snap_seed": int(best_snap_row["seed"]),
        "best_snapped_formula_readable": best_snap_row["snapped_formula_readable"],
        "best_snapped_formula_rust": best_snap_row["snapped_formula_rust"],
    }


# ---------------------------------------------------------------------------
# Phase B: discrimination
# ---------------------------------------------------------------------------

def _phase_b_snap_score(snapped: torch.Tensor, rep: torch.Tensor) -> tuple[float, float, float, bool]:
    var = float(snapped.var(unbiased=False).item())
    mean = float(snapped.mean().item())
    sorted_out = snapped[torch.argsort(rep)]
    mono_ok = bool((sorted_out[1:] >= sorted_out[:-1] - 1e-6).all().item())
    mean_in_band = 0.2 <= mean <= 0.8
    penalty = 0.0
    if not mean_in_band:
        penalty += 1.0
    if not mono_ok:
        penalty += 1.0
    return var - penalty, var, mean, mono_ok


def phase_b_single(
    seed: int,
    sim: torch.Tensor,
    rep: torch.Tensor,
    current_boost: torch.Tensor,
    current_var: float,
) -> dict:
    torch.manual_seed(1000 + seed)
    model = EmlTreeD4()
    opt = torch.optim.Adam(model.parameters(), lr=LR)

    order = torch.argsort(rep)
    rep_sorted = rep[order]
    sim_sorted = sim[order]

    best_soft_var = 0.0
    best_score = -math.inf
    best_state = None
    best_snap_metrics = (0.0, 0.0, False)

    for step in range(PHASE_B_STEPS):
        if step < PHASE_B_HARDENING_START:
            temp = 1.0
        else:
            frac = (step - PHASE_B_HARDENING_START) / max(
                1, PHASE_B_STEPS - PHASE_B_HARDENING_START
            )
            temp = max(0.02, math.exp(math.log(0.02) * frac))

        raw = model(sim, rep, temperature=temp, sigmoid_out=False)
        raw_sorted = model(sim_sorted, rep_sorted, temperature=temp, sigmoid_out=False)
        out = torch.clamp(raw, 0.0, 1.0)
        out_sorted = torch.clamp(raw_sorted, 0.0, 1.0)

        mean_out = out.mean()
        var_out = out.var(unbiased=False)

        diffs = out_sorted[1:] - out_sorted[:-1]
        mono_pen = torch.nn.functional.relu(-diffs).pow(2).mean()

        mean_pen = (
            torch.nn.functional.relu(0.2 - mean_out).pow(2)
            + torch.nn.functional.relu(mean_out - 0.8).pow(2)
        )
        range_pen = (
            torch.nn.functional.relu(-raw).pow(2).mean()
            + torch.nn.functional.relu(raw - 1.0).pow(2).mean()
        )
        sat_pen = torch.nn.functional.relu(0.05 - var_out).pow(2) * 10.0

        loss = -var_out + 10.0 * mono_pen + 10.0 * mean_pen + 5.0 * range_pen + sat_pen
        if not torch.isfinite(loss):
            return {
                "seed": seed,
                "diverged": True,
                "soft_var": 0.0,
                "snap_var": 0.0,
                "snap_mean": 0.0,
                "snap_mono": False,
            }
        opt.zero_grad()
        loss.backward()
        torch.nn.utils.clip_grad_norm_(model.parameters(), 5.0)
        opt.step()

        with torch.no_grad():
            if 0.2 <= float(mean_out.item()) <= 0.8 and float(mono_pen.item()) < 1e-4:
                if float(var_out.item()) > best_soft_var:
                    best_soft_var = float(var_out.item())

        if step % 500 == 0 or step == PHASE_B_STEPS - 1:
            saved = {k: v.detach().clone() for k, v in model.state_dict().items()}
            model.snap()
            with torch.no_grad():
                snap_raw = model(sim, rep, temperature=0.01, sigmoid_out=False)
                snap_out = torch.clamp(snap_raw, 0.0, 1.0)
            score, var_s, mean_s, mono_s = _phase_b_snap_score(snap_out, rep)
            if math.isfinite(score) and score > best_score:
                best_score = score
                best_state = {
                    k: v.detach().clone() for k, v in model.state_dict().items()
                }
                best_snap_metrics = (var_s, mean_s, mono_s)
            model.load_state_dict(saved)

    if best_state is not None:
        model.load_state_dict(best_state)
    else:
        model.snap()
    with torch.no_grad():
        raw_snapped = model(sim, rep, temperature=0.01, sigmoid_out=False)
        snapped = torch.clamp(raw_snapped, 0.0, 1.0)
        trained_var = float(snapped.var(unbiased=False).item())
        mean_snap = float(snapped.mean().item())
        sorted_out = snapped[torch.argsort(rep)]
        mono_ok = bool((sorted_out[1:] >= sorted_out[:-1] - 1e-6).all().item())

    human, rust = model.readable(sigmoid_out=False, clamp01=True)

    beats_current = (
        trained_var > current_var
        and 0.2 <= mean_snap <= 0.8
        and mono_ok
    )
    return {
        "seed": seed,
        "diverged": False,
        "soft_var": float(best_soft_var),
        "snap_var": trained_var,
        "snap_mean": mean_snap,
        "snap_mono": mono_ok,
        "beats_current": beats_current,
        "snapped_formula_readable": human,
        "snapped_formula_rust": rust,
    }


def phase_b(
    sim: torch.Tensor, rep: torch.Tensor, boost: torch.Tensor
) -> dict:
    current_var = float(boost.var(unbiased=False).item())
    print(f"\n=== Phase B: discrimination ({N_RESTARTS} restarts) ===")
    print(f"  current variance = {current_var:.6f}")
    results = []
    for seed in range(N_RESTARTS):
        r = phase_b_single(seed, sim, rep, boost, current_var)
        status = "DIVERGED" if r.get("diverged") else "ok"
        print(
            f"[B] seed={seed} {status} "
            f"soft_var={r['soft_var']:.4f} "
            f"snap_var={r['snap_var']:.4f} "
            f"snap_mean={r['snap_mean']:.4f} "
            f"mono={r['snap_mono']} "
            f"beats_current={r.get('beats_current', False)}"
        )
        if not r.get("diverged"):
            results.append(r)

    if not results:
        return {
            "best_soft_variance": 0.0,
            "best_snap_variance": 0.0,
            "current_formula_variance": current_var,
            "best_soft_ratio_vs_current": 0.0,
            "best_snap_ratio_vs_current": 0.0,
            "n_restarts": N_RESTARTS,
            "n_successful": 0,
            "restarts_that_beat_current": 0,
            "best_snapped_formula_readable": "",
            "best_snapped_formula_rust": "",
        }

    best_soft = max(r["soft_var"] for r in results)
    # Best snap: prefer those that satisfy constraints, fall back to pure variance.
    valid = [
        r for r in results
        if r["snap_mono"] and 0.2 <= r["snap_mean"] <= 0.8
    ]
    if valid:
        best_snap = max(valid, key=lambda r: r["snap_var"])
    else:
        best_snap = max(results, key=lambda r: r["snap_var"])
    beats = sum(1 for r in results if r.get("beats_current"))

    return {
        "best_soft_variance": float(best_soft),
        "best_snap_variance": float(best_snap["snap_var"]),
        "best_snap_mean": float(best_snap["snap_mean"]),
        "best_snap_monotonic": bool(best_snap["snap_mono"]),
        "current_formula_variance": current_var,
        "best_soft_ratio_vs_current": float(best_soft) / max(current_var, 1e-12),
        "best_snap_ratio_vs_current": float(best_snap["snap_var"]) / max(current_var, 1e-12),
        "n_restarts": N_RESTARTS,
        "n_successful": len(results),
        "restarts_that_beat_current": int(beats),
        "best_snap_seed": int(best_snap["seed"]),
        "best_snapped_formula_readable": best_snap["snapped_formula_readable"],
        "best_snapped_formula_rust": best_snap["snapped_formula_rust"],
    }


# ---------------------------------------------------------------------------
# main
# ---------------------------------------------------------------------------

def main() -> None:
    sim, rep, boost = load_pairs()
    print(f"loaded {sim.numel()} pairs")
    print(
        f"input stats: sim in [{sim.min():.3f},{sim.max():.3f}] "
        f"rep in [{rep.min():.3f},{rep.max():.3f}] "
        f"boost in [{boost.min():.3f},{boost.max():.3f}] "
        f"boost var={boost.var(unbiased=False).item():.6f}"
    )
    # Report model size once.
    probe = EmlTreeD4()
    print(f"depth-4 model params: {probe.n_params()}")

    a = phase_a(sim, rep, boost)
    b = phase_b(sim, rep, boost)

    result = {
        "depth": DEPTH,
        "phase_a": {
            "best_soft_mse": a["best_soft_mse"],
            "best_snap_mse": a["best_snap_mse"],
            "n_restarts": a["n_restarts"],
            "n_successful": a["n_successful"],
            "best_snap_seed": a.get("best_snap_seed"),
            "best_snapped_formula_readable": a["best_snapped_formula_readable"],
            "best_snapped_formula_rust": a["best_snapped_formula_rust"],
        },
        "phase_b": {
            "best_soft_variance": b["best_soft_variance"],
            "best_snap_variance": b["best_snap_variance"],
            "current_formula_variance": b["current_formula_variance"],
            "best_soft_ratio_vs_current": b["best_soft_ratio_vs_current"],
            "best_snap_ratio_vs_current": b["best_snap_ratio_vs_current"],
            "n_restarts": b["n_restarts"],
            "n_successful": b["n_successful"],
            "restarts_that_beat_current": b["restarts_that_beat_current"],
            "best_snap_seed": b.get("best_snap_seed"),
            "best_snap_mean": b.get("best_snap_mean"),
            "best_snap_monotonic": b.get("best_snap_monotonic"),
            "best_snapped_formula_readable": b["best_snapped_formula_readable"],
            "best_snapped_formula_rust": b["best_snapped_formula_rust"],
        },
    }
    OUT_PATH.write_text(json.dumps(result, indent=2))
    print(f"\nwrote {OUT_PATH}")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
