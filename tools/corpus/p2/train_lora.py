#!/usr/bin/env python3
"""ADR-0057 P2 — LoRA / QLoRA fine-tune of an open-weight base on the SFT set.

Runs anywhere with torch: a qBraid A100 for the real adapter, a CPU with a
0.5B base for a pipeline smoke. The adapter (PEFT safetensors) is the
ownable artefact; optionally merges it into the base (bf16 HF dir) and
converts to GGUF + quantizes with llama.cpp so debain2's ollama can serve it
(ollama cannot load safetensors adapters for Qwen2 — merge is the path).

  python train_lora.py --base Qwen/Qwen2.5-14B-Instruct --data ~/sft --out ~/run-a100 \
      --qlora --epochs 2 --lr 1e-4 --r 32 --max-len 2048 --merge --gguf q4_K_M
  python train_lora.py --base Qwen/Qwen2.5-0.5B-Instruct --data ~/sft --out /tmp/smoke \
      --max-steps 4 --max-len 256 --r 4 --cpu-smoke

Metric: held-out loss / perplexity on the SAME lines every run (prep_sft's
deterministic hold-out), before and after training. Generation samples for
the voice A/B are written for a human/third-model judge; they are not the
metric.
"""
from __future__ import annotations

import argparse
import json
import math
import os
import subprocess
import sys
import time
from pathlib import Path


def log(msg):
    print(f"[train {time.strftime('%H:%M:%S')}] {msg}", flush=True)


def load_jsonl(p: Path):
    return [json.loads(l) for l in p.read_text(encoding="utf-8").splitlines() if l.strip()]


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--base", required=True, help="HF model id or local dir")
    ap.add_argument("--data", required=True, help="dir with train.jsonl + holdout.jsonl (prep_sft.py)")
    ap.add_argument("--out", required=True)
    ap.add_argument("--epochs", type=float, default=2.0)
    ap.add_argument("--max-steps", type=int, default=-1)
    ap.add_argument("--lr", type=float, default=1e-4)
    ap.add_argument("--r", type=int, default=32)
    ap.add_argument("--alpha", type=int, default=None, help="default 2*r")
    ap.add_argument("--dropout", type=float, default=0.05)
    ap.add_argument("--max-len", type=int, default=2048)
    ap.add_argument("--batch", type=int, default=2)
    ap.add_argument("--grad-accum", type=int, default=8)
    ap.add_argument("--qlora", action="store_true", help="4-bit base (bitsandbytes); needs CUDA")
    ap.add_argument("--cpu-smoke", action="store_true", help="tiny CPU run to prove the pipeline")
    ap.add_argument("--eval-samples", type=int, default=12, help="holdout prompts to generate for the A/B sheet")
    ap.add_argument("--merge", action="store_true", help="merge adapter into base -> <out>/merged (bf16)")
    ap.add_argument("--gguf", default=None, help="quant type (q4_K_M, q8_0, f16) -> <out>/gguf/ via llama.cpp")
    ap.add_argument("--llama-cpp", default=os.environ.get("LLAMA_CPP", str(Path.home() / "llama.cpp")))
    ap.add_argument("--seed", type=int, default=7)
    a = ap.parse_args(argv)

    import torch
    from datasets import Dataset
    from peft import LoraConfig, PeftModel, get_peft_model
    from transformers import AutoModelForCausalLM, AutoTokenizer, TrainingArguments, set_seed
    from trl import SFTConfig, SFTTrainer

    set_seed(a.seed)
    out = Path(a.out)
    out.mkdir(parents=True, exist_ok=True)
    data = Path(a.data)
    train_rows, hold_rows = load_jsonl(data / "train.jsonl"), load_jsonl(data / "holdout.jsonl")
    if a.cpu_smoke:
        train_rows, hold_rows = train_rows[:8], hold_rows[:4]
    log(f"base={a.base} train={len(train_rows)} holdout={len(hold_rows)} qlora={a.qlora} cpu_smoke={a.cpu_smoke}")

    tok = AutoTokenizer.from_pretrained(a.base)
    if tok.pad_token is None:
        tok.pad_token = tok.eos_token
    cuda = torch.cuda.is_available() and not a.cpu_smoke
    kw = {"dtype": torch.bfloat16 if cuda else torch.float32}
    if a.qlora:
        from transformers import BitsAndBytesConfig
        kw["quantization_config"] = BitsAndBytesConfig(load_in_4bit=True, bnb_4bit_quant_type="nf4",
                                                       bnb_4bit_compute_dtype=torch.bfloat16,
                                                       bnb_4bit_use_double_quant=True)
    if cuda:
        kw["device_map"] = {"": 0}
    model = AutoModelForCausalLM.from_pretrained(a.base, **kw)
    if a.qlora:
        from peft import prepare_model_for_kbit_training
        model = prepare_model_for_kbit_training(model)
    lcfg = LoraConfig(r=a.r, lora_alpha=a.alpha or 2 * a.r, lora_dropout=a.dropout, bias="none",
                      task_type="CAUSAL_LM",
                      target_modules=["q_proj", "k_proj", "v_proj", "o_proj", "gate_proj", "up_proj", "down_proj"])
    model = get_peft_model(model, lcfg)
    model.print_trainable_parameters()

    def to_text(rows):
        return Dataset.from_list([{"messages": r["messages"]} for r in rows])

    ds_train, ds_hold = to_text(train_rows), to_text(hold_rows)

    def heldout_loss(m) -> float:
        m.eval()
        tot, n = 0.0, 0
        with torch.no_grad():
            for r in hold_rows:
                ids = tok.apply_chat_template(r["messages"], tokenize=True, return_tensors="pt",
                                              truncation=True, max_length=a.max_len)
                if not isinstance(ids, torch.Tensor):
                    ids = ids["input_ids"]
                ids = ids.to(m.device)
                tot += m(input_ids=ids, labels=ids).loss.item()
                n += 1
        m.train()
        return tot / max(n, 1)

    before = heldout_loss(model)
    log(f"holdout loss BEFORE={before:.4f} ppl={math.exp(before):.2f}")

    # transformers/trl rename fields between releases (warmup_ratio -> warmup_steps,
    # max_seq_length -> max_length, ...). Build the kwargs and keep only the ones
    # THIS SFTConfig knows, so a pod that pip-installs "latest" cannot crash here.
    import dataclasses
    known = {f.name for f in dataclasses.fields(SFTConfig)}
    steps_guess = a.max_steps if a.max_steps > 0 else max(1, int(len(train_rows) * a.epochs / (a.batch * a.grad_accum)))
    want = dict(
        output_dir=str(out / "ckpt"), num_train_epochs=a.epochs, max_steps=a.max_steps,
        per_device_train_batch_size=a.batch, gradient_accumulation_steps=a.grad_accum,
        learning_rate=a.lr, lr_scheduler_type="cosine", warmup_steps=max(1, int(0.03 * steps_guess)),
        warmup_ratio=0.03, logging_steps=5, save_strategy="epoch", bf16=cuda, fp16=False,
        gradient_checkpointing=cuda, max_length=a.max_len, max_seq_length=a.max_len, packing=False,
        report_to=[], seed=a.seed, dataloader_pin_memory=cuda, use_cpu=not cuda,
    )
    if "warmup_steps" in known:
        want.pop("warmup_ratio", None)
    dropped = sorted(k for k in want if k not in known)
    cfg = SFTConfig(**{k: v for k, v in want.items() if k in known})
    if dropped:
        log(f"SFTConfig: ignored unknown fields {dropped}")
    trainer = SFTTrainer(model=model, args=cfg, train_dataset=ds_train, processing_class=tok)
    t0 = time.time()
    trainer.train()
    log(f"trained in {time.time() - t0:.0f}s")

    after = heldout_loss(model)
    log(f"holdout loss AFTER={after:.4f} ppl={math.exp(after):.2f}  (before {before:.4f} / {math.exp(before):.2f})")

    adapter = out / "adapter"
    model.save_pretrained(str(adapter))
    tok.save_pretrained(str(adapter))

    # generation samples for the blind A/B sheet (not the metric)
    samples = []
    model.eval()
    for r in hold_rows[: a.eval_samples]:
        prompt = tok.apply_chat_template(r["messages"][:-1], tokenize=False, add_generation_prompt=True)
        ids = tok(prompt, return_tensors="pt").to(model.device)
        with torch.no_grad():
            g = model.generate(**ids, max_new_tokens=48 if a.cpu_smoke else 200, do_sample=True,
                               temperature=0.8, top_p=0.95, pad_token_id=tok.pad_token_id)
        samples.append({"id": r["id"], "kind": r["kind"], "user": r["messages"][1]["content"],
                        "reference": r["messages"][2]["content"],
                        "generated": tok.decode(g[0][ids["input_ids"].shape[1]:], skip_special_tokens=True)})
    (out / "samples.json").write_text(json.dumps(samples, indent=1, ensure_ascii=False), encoding="utf-8")

    manifest = {"base": a.base, "train": len(train_rows), "holdout": len(hold_rows), "lora": {"r": a.r, "alpha": a.alpha or 2 * a.r},
                "epochs": a.epochs, "max_steps": a.max_steps, "lr": a.lr, "qlora": a.qlora,
                "holdout_loss": {"before": before, "after": after}, "holdout_ppl": {"before": math.exp(before), "after": math.exp(after)},
                "seconds": round(time.time() - t0), "adapter": str(adapter), "cuda": cuda,
                "device": torch.cuda.get_device_name(0) if cuda else "cpu"}

    if a.merge:
        log("merging adapter into base (bf16)")
        base = AutoModelForCausalLM.from_pretrained(a.base, dtype=torch.bfloat16 if cuda else torch.float32,
                                                    device_map={"": 0} if cuda else None)
        merged = PeftModel.from_pretrained(base, str(adapter)).merge_and_unload()
        mdir = out / "merged"
        merged.save_pretrained(str(mdir), safe_serialization=True)
        tok.save_pretrained(str(mdir))
        manifest["merged"] = str(mdir)
        if a.gguf:
            lc = Path(a.llama_cpp)
            conv = lc / "convert_hf_to_gguf.py"
            if not conv.exists():
                log(f"llama.cpp not at {lc}; skipping gguf (clone it and pass --llama-cpp)")
            else:
                gdir = out / "gguf"
                gdir.mkdir(exist_ok=True)
                import shutil as _sh
                # Disk hygiene on the pod (the 2026-09-06 weekly died merging on debain2's full disk):
                # the base cache goes as soon as the merge is saved, the intermediate is q8_0 (half of
                # f16) and the merged dir goes before quantizing -- peak = merged + q8 instead of
                # cache + merged + f16. The q4 that comes back is the only copy that matters.
                del base, merged
                if cuda:
                    torch.cuda.empty_cache()
                try:
                    from huggingface_hub import scan_cache_dir
                    for repo in scan_cache_dir().repos:
                        if repo.repo_id == a.base:
                            _sh.rmtree(repo.repo_path, ignore_errors=True)
                            log(f"purged base cache {repo.repo_path}")
                except Exception as e:  # best effort; the pod is ephemeral anyway
                    log(f"base cache purge skipped: {e}")
                quant = next((p for p in (lc / "build" / "bin" / "llama-quantize", lc / "llama-quantize") if p.exists()), None)
                q = gdir / f"kannaka-brain-{a.gguf}.gguf"
                if quant and a.gguf.lower() not in ("f16", "q8_0"):
                    inter = gdir / "kannaka-brain-q8_0.gguf"
                    subprocess.run([sys.executable, str(conv), str(mdir), "--outtype", "q8_0", "--outfile", str(inter)], check=True)
                    _sh.rmtree(mdir, ignore_errors=True)
                    manifest["merged"] = "removed before quantize"
                    subprocess.run([str(quant), "--allow-requantize", str(inter), str(q), a.gguf], check=True)
                    if q.stat().st_size < 100_000_000:
                        raise RuntimeError(f"quantized output is suspiciously small ({q.stat().st_size} bytes)")
                    inter.unlink(missing_ok=True)
                    manifest["gguf"] = str(q)
                    log(f"gguf {q.name} written ({q.stat().st_size / 1e9:.1f} GB); q8_0 + merged dir removed")
                else:
                    outtype = "q8_0" if a.gguf.lower() == "q8_0" else "f16"
                    subprocess.run([sys.executable, str(conv), str(mdir), "--outtype", outtype, "--outfile", str(q)], check=True)
                    manifest["gguf"] = str(q)
    (out / "train.manifest.json").write_text(json.dumps(manifest, indent=1), encoding="utf-8")
    log(f"done: {json.dumps({k: manifest[k] for k in ('holdout_ppl', 'seconds', 'adapter')})}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
