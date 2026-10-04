"""train_lora tests: --init-adapter, --eval-every, --max-holdout-rise (kannaka-loop-c1 prereg, section 2.3).

Run: python -m pytest tools/corpus/p2/test_train_lora.py
CPU only, offline: the base is a 1-layer Qwen3 (the c1 base family) with hidden size 16 and a
word-level tokenizer, both built in a temp dir, so nothing is downloaded. Needs torch, transformers,
peft, trl and datasets (the trainer's own deps); skipped where they are missing.
Pins the prereg's four trainer tests:
  1. a smoke from an init adapter saves an adapter that differs from the init adapter and from a fresh
     LoRA trained the same way (2 steps, not 1: step 1 is warmup at lr 0, see the test);
  2. a mismatched r is refused (and base, alpha, dropout and target modules, at the unit level);
  3. a per-example `chat_template_kwargs` column reaches the template;
  4. `--eval-every 1` on a 2-step smoke logs a hold-out loss after each step;
plus the stop rule (a rise over 10 percent, or a non-finite loss, stops the run with exit 4).
"""
import json
import math
import os
import sys
from pathlib import Path

import pytest

os.environ.setdefault("HF_HUB_OFFLINE", "1")
os.environ.setdefault("TRANSFORMERS_OFFLINE", "1")
sys.path.insert(0, os.path.dirname(__file__))
import train_lora as tl  # noqa: E402
from base_info import LORA_TARGET_REGEX  # noqa: E402

for _m in ("torch", "transformers", "peft", "trl", "datasets", "tokenizers", "safetensors"):
    pytest.importorskip(_m)

WORDS = ("you are kannaka a memory what do you remember the first wave i keep only what resonates back "
         "tell me about studio session it is not in my record hello again ghost signal").split()
MARK = "NOTHINK"
# ChatML-shaped, word-level. MARK is rendered when a caller passes enable_thinking=false, so a test can
# see whether the per-example kwargs reached the template.
TEMPLATE = ("{% if enable_thinking is defined and enable_thinking == false %}" + MARK + " {% endif %}"
            "{% for m in messages %}<|im_start|> {{ m['role'] }} {{ m['content'] }} <|im_end|> {% endfor %}"
            "{% if add_generation_prompt %}<|im_start|> assistant {% endif %}")


def _rows(n, seed):
    out = []
    for i in range(n):
        w = [WORDS[(seed * 7 + i * 3 + k) % len(WORDS)] for k in range(6)]
        out.append({"id": f"r{seed}-{i}", "kind": "voice", "messages": [
            {"role": "system", "content": "you are kannaka"},
            {"role": "user", "content": " ".join(w[:3])},
            {"role": "assistant", "content": " ".join(w[3:])}]})
    return out


@pytest.fixture(scope="module")
def tiny(tmp_path_factory):
    """(base_dir, data_dir): a tiny random Qwen3 + word-level tokenizer and an 8/4-row SFT set."""
    import torch
    from tokenizers import Tokenizer, models, pre_tokenizers
    from transformers import PreTrainedTokenizerFast, Qwen3Config, Qwen3ForCausalLM

    root = tmp_path_factory.mktemp("tiny")
    base, data = root / "base", root / "data"
    specials = ["<unk>", "<pad>", "<|im_start|>", "<|im_end|>"]
    vocab = {w: i for i, w in enumerate(specials + sorted(set(WORDS + ["system", "user", "assistant", MARK])))}
    tk = Tokenizer(models.WordLevel(vocab=vocab, unk_token="<unk>"))
    tk.pre_tokenizer = pre_tokenizers.WhitespaceSplit()
    tok = PreTrainedTokenizerFast(tokenizer_object=tk, unk_token="<unk>", pad_token="<pad>", eos_token="<|im_end|>")
    tok.add_special_tokens({"additional_special_tokens": ["<|im_start|>"]})
    tok.chat_template = TEMPLATE
    tok.save_pretrained(str(base))
    torch.manual_seed(0)
    cfg = Qwen3Config(vocab_size=len(vocab), hidden_size=16, intermediate_size=32, num_hidden_layers=1, head_dim=8,
                      num_attention_heads=2, num_key_value_heads=1, max_position_embeddings=128,
                      eos_token_id=vocab["<|im_end|>"], pad_token_id=vocab["<pad>"], tie_word_embeddings=False,
                      initializer_range=1.0)   # big logits, so a bad step can raise the hold-out loss
    Qwen3ForCausalLM(cfg).save_pretrained(str(base))
    data.mkdir()
    for name, rows in (("train.jsonl", _rows(8, 1)), ("holdout.jsonl", _rows(4, 2))):
        (data / name).write_text("".join(json.dumps(r) + "\n" for r in rows), encoding="utf-8")
    return base, data


def _run(tiny, out, *extra):
    base, data = tiny
    return tl.main(["--base", str(base), "--data", str(data), "--out", str(out), "--cpu-smoke", "--max-len", "64",
                    "--r", "4", "--batch", "2", "--grad-accum", "1", "--lr", "0.01", "--eval-samples", "0",
                    *extra])


def _weights(adapter_dir):
    from safetensors.torch import load_file
    return load_file(str(Path(adapter_dir) / "adapter_model.safetensors"))


def _differs(a, b):
    import torch
    assert a.keys() == b.keys(), (sorted(a)[:3], sorted(b)[:3])
    return any(not torch.equal(a[k], b[k]) for k in a)


# 1 ------------------------------------------------------------------------------------------------
# Two optimizer steps, not one: the trainer's cosine schedule has warmup_steps >= 1 and HF's warmup gives
# step 1 a learning rate of 0, so a 1-step run changes no weight (measured: a 1-step init adapter had
# lora_B all zero and the same hold-out loss as the base). Step 2 runs at the full --lr.
def _manifest(d):
    return json.loads((d / "train.manifest.json").read_text(encoding="utf-8"))


def test_init_adapter_continues_the_given_adapter(tiny, tmp_path):
    assert _run(tiny, tmp_path / "init", "--max-steps", "2") == 0           # the adapter to continue
    assert _run(tiny, tmp_path / "fresh", "--max-steps", "2") == 0          # a fresh LoRA, same flags/seed
    assert _run(tiny, tmp_path / "cont", "--max-steps", "2", "--init-adapter", str(tmp_path / "init" / "adapter")) == 0
    init, fresh, cont = (_weights(tmp_path / d / "adapter") for d in ("init", "fresh", "cont"))
    assert _differs(init, fresh) is False, "same flags and seed should give the same fresh adapter"
    assert _differs(cont, init), "continued adapter equals the init adapter: it was not trained"
    assert _differs(cont, fresh), "continued adapter equals a fresh LoRA: the init adapter was not loaded"
    # the continued run starts where the init run ended: its BEFORE is the init run's AFTER, not the base's
    assert _manifest(tmp_path / "cont")["holdout_loss"]["before"] == pytest.approx(
        _manifest(tmp_path / "init")["holdout_loss"]["after"], rel=1e-5)
    assert _manifest(tmp_path / "cont")["holdout_loss"]["before"] != pytest.approx(
        _manifest(tmp_path / "fresh")["holdout_loss"]["before"], rel=1e-5)
    man = _manifest(tmp_path / "cont")
    assert man["init_adapter"]["path"] == str(tmp_path / "init" / "adapter")
    assert len(man["init_adapter"]["sha256"]) == 64


# 2 ------------------------------------------------------------------------------------------------
def test_init_adapter_with_mismatched_r_is_refused(tiny, tmp_path, capsys):
    assert _run(tiny, tmp_path / "init", "--max-steps", "1") == 0           # r=4 (weights do not matter here)
    rc = _run(tiny, tmp_path / "cont", "--max-steps", "1", "--r", "8", "--alpha", "8",
              "--init-adapter", str(tmp_path / "init" / "adapter"))
    assert rc == 5
    assert "r 4 != --r 8" in capsys.readouterr().out
    assert not (tmp_path / "cont" / "adapter").exists()


GOOD = {"base_model_name_or_path": "Qwen/Qwen3-8B", "r": 32, "lora_alpha": 64, "lora_dropout": 0.05,
        "target_modules": LORA_TARGET_REGEX}


def test_adapter_mismatch_accepts_a_matching_config():
    assert tl.adapter_mismatch(dict(GOOD), "Qwen/Qwen3-8B", 32, 64, 0.05, LORA_TARGET_REGEX) == []
    lst = dict(GOOD, target_modules=["v_proj", "q_proj"])
    assert tl.adapter_mismatch(lst, "Qwen/Qwen3-8B", 32, 64, 0.05, ["q_proj", "v_proj"]) == []


@pytest.mark.parametrize("field,value,needle", [
    ("base_model_name_or_path", "Qwen/Qwen2.5-7B-Instruct", "base"),
    ("r", 16, "r 16"),
    ("lora_alpha", 32, "lora_alpha"),
    ("lora_dropout", 0.1, "lora_dropout"),
    ("target_modules", ["q_proj", "v_proj"], "target_modules"),
])
def test_adapter_mismatch_names_each_field(field, value, needle):
    bad = tl.adapter_mismatch(dict(GOOD, **{field: value}), "Qwen/Qwen3-8B", 32, 64, 0.05, LORA_TARGET_REGEX)
    assert len(bad) == 1 and needle in bad[0], bad


# 3 ------------------------------------------------------------------------------------------------
@pytest.mark.parametrize("completion_only", [False, True])
def test_chat_template_kwargs_column_reaches_the_template(tiny, tmp_path, completion_only):
    import dataclasses

    from transformers import AutoModelForCausalLM, AutoTokenizer
    from trl import SFTConfig, SFTTrainer
    base, data = tiny
    tok = AutoTokenizer.from_pretrained(str(base))
    mark = tok.convert_tokens_to_ids(MARK)
    rows = tl.load_jsonl(data / "train.jsonl")[:2]
    known = {f.name for f in dataclasses.fields(SFTConfig)}
    want = dict(output_dir=str(tmp_path / "ckpt"), max_steps=1, report_to=[], use_cpu=True, max_length=64)
    cfg = SFTConfig(**{k: v for k, v in want.items() if k in known})
    model = AutoModelForCausalLM.from_pretrained(str(base))
    for kwargs, expect in (({"enable_thinking": False}, True), ({}, False)):
        ds = tl.to_dataset(rows, kwargs, completion_only)
        tr = SFTTrainer(model=model, args=cfg, train_dataset=ds, processing_class=tok)
        ids = tr.train_dataset[0]["input_ids"]
        assert (mark in ids) == expect, (kwargs, tok.decode(ids))


# 4 ------------------------------------------------------------------------------------------------
def test_eval_every_1_logs_holdout_loss_after_each_step(tiny, tmp_path, capsys):
    assert _run(tiny, tmp_path / "run", "--max-steps", "2", "--eval-every", "1") == 0
    out = capsys.readouterr().out
    for step in (1, 2):
        assert f"holdout loss step {step}=" in out, out
    man = json.loads((tmp_path / "run" / "train.manifest.json").read_text(encoding="utf-8"))
    assert [p["step"] for p in man["holdout_loss_series"]] == [1, 2]
    assert all(math.isfinite(p["loss"]) for p in man["holdout_loss_series"])
    assert man["eval_every"] == 1


# stop rule ---------------------------------------------------------------------------------------
def test_diverged_is_more_than_ten_percent_over_before():
    assert not tl.diverged(2.0, 2.2, 0.10)          # exactly +10 percent: not "more than"
    assert tl.diverged(2.0, 2.2001, 0.10)
    assert not tl.diverged(2.0, 1.5, 0.10)
    assert tl.diverged(2.0, float("nan"), 0.10)
    assert tl.diverged(2.0, float("inf"), 0.10)


def test_holdout_rise_stops_the_run_and_saves_no_adapter(tiny, tmp_path, capsys):
    # lr 100 on a 16-wide model blows the hold-out loss up at the first step with a non-zero lr (step 2, see
    # above); the rule stops the run there, 4 of 6 steps early
    rc = _run(tiny, tmp_path / "run", "--max-steps", "6", "--eval-every", "1", "--lr", "100",
              "--max-holdout-rise", "0.10")
    out = capsys.readouterr().out
    assert rc == 4, out
    assert "DIVERGED at step 2" in out
    man = _manifest(tmp_path / "run")
    assert man["diverged"] == "step 2" and [p["step"] for p in man["holdout_loss_series"]] == [1, 2]
    assert man["holdout_loss_series"][1]["loss"] > 1.1 * man["holdout_loss"]["before"]
    assert not (tmp_path / "run" / "adapter").exists()


def test_holdout_rise_rule_is_off_by_default(tiny, tmp_path):
    # the same diverging run without --max-holdout-rise trains all 6 steps and saves its adapter
    assert _run(tiny, tmp_path / "run", "--max-steps", "6", "--eval-every", "1", "--lr", "100") == 0
    man = _manifest(tmp_path / "run")
    assert [p["step"] for p in man["holdout_loss_series"]] == [1, 2, 3, 4, 5, 6]
    assert max(p["loss"] for p in man["holdout_loss_series"]) > 1.1 * man["holdout_loss"]["before"]
    assert (tmp_path / "run" / "adapter" / "adapter_model.safetensors").exists()


if __name__ == "__main__":
    raise SystemExit(pytest.main([__file__, "-q"]))
