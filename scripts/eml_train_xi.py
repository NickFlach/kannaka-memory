"""L4.S0 symbolic regression of xi_diversity_boost using depth-3 EML trees.

Odrzywolek (arXiv:2603.21852v2, April 2026): eml(x, y) = exp(x) - ln(y) paired
with constant 1 is a continuous Sheffer stroke — universal for elementary
mathematics. A master tree with softmax-normalized leaf and input logits can be
trained with Adam to recover closed-form formulas.

Phase A: replicate the current hand-coded xi_diversity_boost formula.
Phase B: discover a discriminating formula (high variance, monotonic in
repulsion, mean in [0.2, 0.8]) from scratch.

A depth-3 binary tree has 7 internal nodes and 8 leaves. Each leaf is a 3-way
softmax over {1, s, r}. Each internal-node input is a 4-way softmax over
{1, s, r, f_prev} where f_prev is the previous internal node's output (acting
like a residual). At depth-3 total ~80 real parameters.

Outputs go through a final sigmoid in Phase B so they live in [0, 1].
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
OUT_PATH = EXP / "eml_xi_tree.json"


# ---------------------------------------------------------------------------
# EML tree
# ---------------------------------------------------------------------------

DEPTH = 3
N_INTERNAL = 2 ** DEPTH - 1  # 7
N_LEAVES = 2 ** DEPTH        # 8

# Leaf vocab: {1, s, r} (3 tokens).
# Internal-input vocab: {1, s, r, f_prev} (4 tokens).
LEAF_VOCAB = 3
INPUT_VOCAB = 4

EXP_CLAMP = 8.0
LN_FLOOR = 1e-6
LN_CEIL = 1e8


def eml(lhs: torch.Tensor, rhs: torch.Tensor) -> torch.Tensor:
    """EML operator: exp(L) - ln(R) with domain clamps."""
    l = torch.clamp(lhs, min=-EXP_CLAMP, max=EXP_CLAMP)
    r = torch.clamp(rhs, min=LN_FLOOR, max=LN_CEIL)
    return torch.exp(l) - torch.log(r)


def soft_one_hot(logits: torch.Tensor, temperature: float) -> torch.Tensor:
    return torch.softmax(logits / max(temperature, 1e-6), dim=-1)


class EmlTree(torch.nn.Module):
    """Depth-3 EML master tree.

    Tree layout (heap indexing starts at 1):
      - Internal nodes: 1..7 (root = 1). 2 children at 2k, 2k+1.
      - Leaves: indexes 8..15 (children of 4, 5, 6, 7). We realize them as
        soft mixes over LEAF_VOCAB primitives (1, s, r).
      - Each non-leaf input (children of nodes 1, 2, 3 that are themselves
        internal) is a 4-way mix over {1, s, r, f_prev}. f_prev is the value
        produced at the previous internal node in evaluation order, acting as
        a residual / re-use channel consistent with Odrzywolek §4.3.

    Evaluation order: post-order DFS (children before parents). f_prev starts
    at 0 and updates after every internal-node computation.
    """

    def __init__(self) -> None:
        super().__init__()
        # 8 leaves, each a 3-way softmax over {1, s, r}
        self.leaf_logits = torch.nn.Parameter(torch.zeros(N_LEAVES, LEAF_VOCAB))
        # Internal-node inputs are only used for nodes whose children are also
        # internal. Level 0: node 1, inputs = (output of node 2, output of node 3).
        # Level 1: nodes 2, 3, inputs = (output of node 4, 5) and (6, 7).
        # For every internal node we still allow an input-mix over {1, s, r,
        # f_prev, child_output}. To keep the parameter count near 80 and the
        # symbolic interpretation clean, we instead apply a 4-way softmax over
        # {1, s, r, child_raw} to every input (6 inputs total for non-leaf
        # children, i.e. nodes 1..3 have 2 inputs each → 6 mix logits) where
        # "child_raw" is the value flowing up from the subtree.
        # Simpler: every one of the 14 input slots to the 7 internal nodes gets
        # a 4-way soft mix over {1, s, r, f_prev}. f_prev is the residual and
        # represents "re-use the subtree output".
        self.input_logits = torch.nn.Parameter(torch.zeros(N_INTERNAL, 2, INPUT_VOCAB))
        # Final bias + output scale (Phase B wraps in sigmoid; Phase A does not).
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
        """Evaluate the tree on (s, r) batches of shape [N]."""
        one = torch.ones_like(s)
        leaf_primitives = torch.stack([one, s, r], dim=-1)  # [N, 3]

        # Compute leaf values.
        leaf_mix = soft_one_hot(self.leaf_logits, temperature)  # [8, 3]
        # leaf_values[k, n] = sum_p leaf_mix[k, p] * leaf_primitives[n, p]
        leaf_values = leaf_primitives @ leaf_mix.T  # [N, 8]

        # Walk internal nodes bottom-up: 7,6,5,4,3,2,1 (heap reverse order).
        # Node k's children:
        #   if 2k >= N_INTERNAL+1 → both children are leaves (leaf indices:
        #     2k - N_INTERNAL - 1 and 2k - N_INTERNAL, shifted by -1 to 0-index)
        #   else both children are internal.
        # Depth-3 with N_INTERNAL=7: nodes 4,5,6,7 have leaf children; 1,2,3
        # have internal children.
        internal_values: list[torch.Tensor | None] = [None] * (N_INTERNAL + 1)
        f_prev = torch.zeros_like(s)

        for k in range(N_INTERNAL, 0, -1):
            left_child = 2 * k
            right_child = 2 * k + 1
            if left_child > N_INTERNAL:
                # Children are leaves. Leaf idx: left_child - (N_INTERNAL + 1).
                # For k=4: children are heap nodes 8, 9 → leaves 0, 1.
                leaf_l = left_child - (N_INTERNAL + 1)
                leaf_r = right_child - (N_INTERNAL + 1)
                child_l = leaf_values[:, leaf_l]
                child_r = leaf_values[:, leaf_r]
            else:
                child_l = internal_values[left_child]
                child_r = internal_values[right_child]
                assert child_l is not None and child_r is not None

            # Each input to the eml gate is a 4-way softmax over
            # {1, s, r, child}. This is the Odrzywolek convention: every input
            # is a soft selection over atomic primitives plus the subtree
            # value flowing up. f_prev is unused here (kept in the interface
            # so the snap/readable logic can reinterpret it later if needed).
            mix = soft_one_hot(self.input_logits[k - 1], temperature)  # [2, 4]
            input_vocab_l = torch.stack([one, s, r, child_l], dim=-1)  # [N, 4]
            input_vocab_r = torch.stack([one, s, r, child_r], dim=-1)  # [N, 4]
            lhs = (input_vocab_l * mix[0]).sum(dim=-1)
            rhs = (input_vocab_r * mix[1]).sum(dim=-1)

            node_val = eml(lhs, rhs)
            internal_values[k] = node_val
            f_prev = node_val  # residual carry (currently unused, kept for API)
        _ = f_prev

        root = internal_values[1]
        assert root is not None
        out = self.out_scale * root + self.out_bias
        if sigmoid_out:
            out = torch.sigmoid(out)
        return out

    # -------- symbolic extraction --------

    @torch.no_grad()
    def snap(self) -> None:
        """Snap softmaxes to their argmax (one-hot)."""
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
    def readable(self, sigmoid_out: bool) -> tuple[str, str]:
        """Return (human-readable, rust) source for the argmax-snapped tree."""
        leaf_tokens = ["1", "s", "r"]
        input_tokens_human = ["1", "s", "r", "c"]  # c = child
        input_tokens_rust = ["1.0_f32", "sim", "rep", "c"]

        leaf_choice = self.leaf_logits.argmax(dim=-1).tolist()
        input_choice = self.input_logits.argmax(dim=-1).tolist()

        def leaf_expr(leaf_idx: int, kind: str) -> str:
            tok = leaf_tokens[leaf_choice[leaf_idx]] if kind == "human" else (
                "1.0_f32" if leaf_choice[leaf_idx] == 0
                else "sim" if leaf_choice[leaf_idx] == 1
                else "rep"
            )
            return tok

        def input_expr(node_k: int, side: int, child_expr: str, kind: str) -> str:
            choice = input_choice[node_k - 1][side]
            toks = input_tokens_human if kind == "human" else input_tokens_rust
            if choice == 3:  # child
                return child_expr
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
                # leaves
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
        rust = f"{scale:.4f}_f32 * ({rust_core}) + {bias:.4f}_f32"
        if sigmoid_out:
            human = f"sigmoid({human})"
            rust = f"1.0 / (1.0 + (-({rust})).exp())"
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
# Phase A: replicate current formula
# ---------------------------------------------------------------------------

def phase_a(sim: torch.Tensor, rep: torch.Tensor, boost: torch.Tensor) -> dict:
    torch.manual_seed(0)
    model = EmlTree()
    opt = torch.optim.Adam(model.parameters(), lr=0.01)
    n_steps = 4000
    anneal_start = 2000  # long hardening window so argmax doesn't collapse
    best_snapped_mse = float("inf")
    best_soft_mse = float("inf")
    best_state = None
    for step in range(n_steps):
        if step < anneal_start:
            temp = 1.0
        else:
            frac = (step - anneal_start) / max(1, n_steps - anneal_start)
            # Exponential temperature anneal from 1.0 → 0.02.
            temp = max(0.02, math.exp(math.log(0.02) * frac))
        out = model(sim, rep, temperature=temp, sigmoid_out=False)
        loss = torch.nn.functional.mse_loss(out, boost)
        opt.zero_grad()
        loss.backward()
        torch.nn.utils.clip_grad_norm_(model.parameters(), 5.0)
        opt.step()
        best_soft_mse = min(best_soft_mse, float(loss.item()))
        if step % 500 == 0 or step == n_steps - 1:
            # Evaluate an argmax snapshot periodically so we can pick the best.
            saved = {k: v.detach().clone() for k, v in model.state_dict().items()}
            model.snap()
            with torch.no_grad():
                snap_out = model(sim, rep, temperature=0.01, sigmoid_out=False)
                snap_mse = torch.nn.functional.mse_loss(snap_out, boost).item()
            if snap_mse < best_snapped_mse:
                best_snapped_mse = snap_mse
                best_state = {k: v.detach().clone() for k, v in model.state_dict().items()}
            model.load_state_dict(saved)
            print(
                f"[A] step {step:4d} soft_mse={loss.item():.6f} "
                f"snap_mse={snap_mse:.6f} temp={temp:.3f}"
            )

    if best_state is not None:
        model.load_state_dict(best_state)
    else:
        model.snap()
    with torch.no_grad():
        snapped = model(sim, rep, temperature=0.01, sigmoid_out=False)
        snapped_mse = torch.nn.functional.mse_loss(snapped, boost).item()
    return {
        "soft_final_mse": float(loss.item()),
        "soft_best_mse": float(best_soft_mse),
        "final_mse": float(snapped_mse),
        "can_represent_current_formula_soft": bool(best_soft_mse < 1e-2),
        "can_represent_current_formula_snapped": bool(snapped_mse < 1e-3),
    }


# ---------------------------------------------------------------------------
# Phase B: discrimination
# ---------------------------------------------------------------------------

def _phase_b_score(snapped, rep):
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


def phase_b(sim: torch.Tensor, rep: torch.Tensor, current_boost: torch.Tensor) -> dict:
    torch.manual_seed(1)
    model = EmlTree()
    opt = torch.optim.Adam(model.parameters(), lr=0.01)
    n_steps = 4000
    hardening_start = 2000

    # Sort by repulsion for monotonicity penalty.
    order = torch.argsort(rep)
    rep_sorted = rep[order]
    sim_sorted = sim[order]

    best_score = -math.inf
    best_state = None
    best_metrics = None
    best_soft_var = 0.0

    for step in range(n_steps):
        if step < hardening_start:
            temp = 1.0
        else:
            frac = (step - hardening_start) / max(1, n_steps - hardening_start)
            temp = max(0.02, math.exp(math.log(0.02) * frac))

        # Use raw (no sigmoid) during training; penalize range violations so
        # that the snapped tree lives naturally in [0, 1] without relying on a
        # saturating nonlinearity.
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

        # Keep raw outputs within [0, 1] so clamp is not doing the work.
        range_pen = (
            torch.nn.functional.relu(-raw).pow(2).mean()
            + torch.nn.functional.relu(raw - 1.0).pow(2).mean()
        )

        sat_pen = torch.nn.functional.relu(0.05 - var_out).pow(2) * 10.0

        loss = -var_out + 10.0 * mono_pen + 10.0 * mean_pen + 5.0 * range_pen + sat_pen
        opt.zero_grad()
        loss.backward()
        torch.nn.utils.clip_grad_norm_(model.parameters(), 5.0)
        opt.step()
        # Track best soft variance (mean in band + monotone soft).
        with torch.no_grad():
            if 0.2 <= float(mean_out.item()) <= 0.8 and float(mono_pen.item()) < 1e-4:
                if float(var_out.item()) > best_soft_var:
                    best_soft_var = float(var_out.item())

        if step % 250 == 0 or step == n_steps - 1:
            saved = {k: v.detach().clone() for k, v in model.state_dict().items()}
            model.snap()
            with torch.no_grad():
                snap_raw = model(sim, rep, temperature=0.01, sigmoid_out=False)
                snap_out = torch.clamp(snap_raw, 0.0, 1.0)
            score, var_s, mean_s, mono_s = _phase_b_score(snap_out, rep)
            if score > best_score:
                best_score = score
                best_state = {
                    k: v.detach().clone() for k, v in model.state_dict().items()
                }
                best_metrics = (var_s, mean_s, mono_s)
            model.load_state_dict(saved)
            if step % 500 == 0 or step == n_steps - 1:
                print(
                    f"[B] step {step:4d} soft_loss={loss.item():.4f} "
                    f"soft_var={var_out.item():.4f} soft_mean={mean_out.item():.4f} "
                    f"snap_var={var_s:.4f} snap_mean={mean_s:.4f} mono={mono_s} "
                    f"temp={temp:.3f}"
                )

    if best_state is not None:
        model.load_state_dict(best_state)
    else:
        model.snap()
    with torch.no_grad():
        raw_snapped = model(sim, rep, temperature=0.01, sigmoid_out=False)
        snapped = torch.clamp(raw_snapped, 0.0, 1.0)
        trained_var = float(snapped.var(unbiased=False).item())
        current_var = float(current_boost.var(unbiased=False).item())
        mean_snap = float(snapped.mean().item())
        sorted_out = snapped[torch.argsort(rep)]
        mono_ok = bool((sorted_out[1:] >= sorted_out[:-1] - 1e-6).all().item())

    human, rust = model.readable(sigmoid_out=False)
    # Wrap with clamp in the human/rust forms since we removed sigmoid.
    human = f"clamp01({human})"
    rust = f"({rust}).clamp(0.0_f32, 1.0_f32)"
    return {
        "trained_variance": trained_var,
        "soft_best_variance": float(best_soft_var),
        "current_variance": current_var,
        "variance_ratio": trained_var / max(current_var, 1e-12),
        "soft_variance_ratio": float(best_soft_var) / max(current_var, 1e-12),
        "mean_output": mean_snap,
        "monotonic_increasing_in_repulsion": mono_ok,
        "snapped_formula_readable": human,
        "snapped_formula_rust": rust,
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

    print("\n=== Phase A: replicate current formula ===")
    a_result = phase_a(sim, rep, boost)

    print("\n=== Phase B: discrimination ===")
    b_result = phase_b(sim, rep, boost)

    result = {"phase_a": a_result, "phase_b": b_result}
    OUT_PATH.write_text(json.dumps(result, indent=2))
    print(f"\nwrote {OUT_PATH}")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
