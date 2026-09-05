#!/usr/bin/env python3
"""ADR-0057 P2 — run train_lora.py on a qBraid GPU, gated the way the ADR says.

Every qBraid GPU profile is an on-demand BMA instance. This script:
  1. refuses to spend unless --allow-spend is given, and only on a whitelisted
     profile (single-GPU; multi-GPU needs Nick)
  2. provisions the instance, sets the cutoff BEFORE any work
     (max_session_minutes, auto_stop_idle_minutes), waits for `running`
  3. configures a per-instance SSH alias and ships the SFT data + trainer
  4. bootstraps the pod (pip, llama.cpp), launches training under nohup,
     tails the log until train.manifest.json appears (or the cutoff kills it)
  5. fetches adapter/ manifest/ samples/ (and gguf/ if made) to --fetch-to
  6. stops (default) or --terminate the instance. Disk is kept on stop.

  python run_qbraid.py --profile gpu-rtx-4090 --data ~/.kannaka-corpus/sft \
      --base Qwen/Qwen2.5-1.5B-Instruct --max-steps 30 --max-minutes 45 --allow-spend   # smoke
  python run_qbraid.py --profile gpu-a100-sxm --data ~/.kannaka-corpus/sft \
      --base Qwen/Qwen2.5-14B-Instruct --qlora --epochs 2 --merge --gguf q4_K_M \
      --max-minutes 240 --allow-spend                                            # the real one

Nothing here contacts anyone but qBraid. Outputs land outside any repo.
"""
from __future__ import annotations

import argparse
import json
import os
import shlex
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
ALLOWED = {"gpu-rtx-4090": 0.87, "gpu-l40s": 2.28, "gpu-a100-sxm": 2.49, "gpu-h100-sxm": 5.37}  # $/h, single GPU only
REMOTE = "~/kannaka-p2"


def log(msg):
    print(f"[qbraid {time.strftime('%H:%M:%S')}] {msg}", flush=True)


def sh(cmd, check=True, capture=False, **kw):
    if capture:
        return subprocess.run(cmd, check=check, capture_output=True, text=True, **kw)
    return subprocess.run(cmd, check=check, **kw)


def ssh(alias, remote_cmd, check=True, capture=False, timeout=None):
    return sh(["ssh", "-o", "BatchMode=yes", "-o", "ConnectTimeout=20", "-o", "ServerAliveInterval=30",
               alias, remote_cmd], check=check, capture=capture, timeout=timeout)


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--profile", required=True, choices=sorted(ALLOWED))
    ap.add_argument("--data", required=True, help="dir with train.jsonl/holdout.jsonl")
    ap.add_argument("--base", default="Qwen/Qwen2.5-14B-Instruct")
    ap.add_argument("--max-minutes", type=int, default=240, help="hard session cutoff (ADR-0057: 4 h)")
    ap.add_argument("--idle-minutes", type=int, default=15)
    ap.add_argument("--allow-spend", action="store_true", help="REQUIRED to provision anything")
    ap.add_argument("--fetch-to", default=str(Path.home() / ".kannaka-corpus" / "runs"))
    ap.add_argument("--terminate", action="store_true", help="delete the instance + disk when done (default: stop)")
    ap.add_argument("--instance", default=None, help="reuse an existing (stopped) instance id")
    ap.add_argument("--dry-run", action="store_true", help="print the plan and the cost ceiling; provision nothing")
    ap.add_argument("train_args", nargs=argparse.REMAINDER, help="passed to train_lora.py after --")
    a = ap.parse_args(argv)
    train_args = [x for x in a.train_args if x != "--"]

    rate = ALLOWED[a.profile]
    ceiling = rate * a.max_minutes / 60
    run_name = f"{a.profile}-{time.strftime('%Y%m%d-%H%M')}"
    log(f"plan: {a.profile} @ ${rate}/h, cutoff {a.max_minutes} min -> ceiling ${ceiling:.2f}; base={a.base}; run={run_name}")
    if a.dry_run:
        return 0
    if not a.allow_spend:
        log("refusing: --allow-spend not given (ADR-0057 spend gate)")
        return 2

    import warnings
    warnings.filterwarnings("ignore")
    from qbraid_core.services.compute import ComputeClient
    c = ComputeClient()
    bal = c.get_credits_balance()
    log(f"credits before: {bal.get('qbraidCredits')}")

    # 1-2. provision + cutoff first
    if a.instance:
        inst = c.get_bma_instance(a.instance)
        if str(inst.status).lower().endswith("stopped"):
            inst = c.start_bma_instance(a.instance)
    else:
        inst = c.provision_bma_instance(a.profile)
    iid = inst.id if hasattr(inst, "id") else inst["id"]
    log(f"instance {iid} status={getattr(inst, 'status', '?')}")
    try:
        c.update_bma_cutoff(iid, auto_stop_idle_minutes=a.idle_minutes, max_session_minutes=a.max_minutes)
        log(f"cutoff set: max {a.max_minutes} min, idle {a.idle_minutes} min")
    except Exception as e:
        log(f"cutoff FAILED ({e}); terminating rather than run ungated")
        c.terminate_bma_instance(iid)
        return 3
    inst = c.wait_for_bma_instance(iid, timeout=900)
    log(f"instance running: {getattr(inst, 'url', '')}")
    started = time.time()

    try:
        # 3. ssh + ship
        cfg = c.configure_ssh_for_instance(iid)
        alias = cfg.get("alias") or c.bma_ssh_alias(iid)
        log(f"ssh alias: {alias}")
        for _ in range(30):
            if ssh(alias, "echo up", check=False, capture=True).returncode == 0:
                break
            time.sleep(10)
        ssh(alias, f"mkdir -p {REMOTE}/data {REMOTE}/out")
        sh(["scp", "-q", "-o", "BatchMode=yes", str(HERE / "train_lora.py"), f"{alias}:{REMOTE}/"])
        for f in ("train.jsonl", "holdout.jsonl"):
            sh(["scp", "-q", "-o", "BatchMode=yes", str(Path(a.data) / f), f"{alias}:{REMOTE}/data/"])
        log("shipped trainer + data")

        # 4. bootstrap + launch
        boot = (
            "set -e; cd " + REMOTE + " && "
            "python3 -m pip install -q --upgrade pip && "
            "python3 -m pip install -q 'transformers>=4.45' 'peft>=0.13' 'datasets>=3' 'accelerate>=1' 'trl>=0.12' bitsandbytes sentencepiece protobuf && "
            "([ -d ~/llama.cpp ] || git clone -q --depth 1 https://github.com/ggml-org/llama.cpp ~/llama.cpp) && "
            "python3 -m pip install -q -r ~/llama.cpp/requirements/requirements-convert_hf_to_gguf.txt && "
            "(cd ~/llama.cpp && cmake -B build -DGGML_CUDA=OFF -DLLAMA_CURL=OFF >/dev/null && cmake --build build --target llama-quantize -j >/dev/null) && "
            "nvidia-smi --query-gpu=name,memory.total --format=csv,noheader && python3 -c 'import torch;print(\"torch\",torch.__version__,\"cuda\",torch.cuda.is_available())'"
        )
        r = ssh(alias, boot, capture=True, timeout=1800)
        log("bootstrap: " + r.stdout.strip().splitlines()[-2:].__str__())
        targs = " ".join(shlex.quote(x) for x in train_args)
        launch = (f"cd {REMOTE} && nohup python3 train_lora.py --base {shlex.quote(a.base)} --data data --out out "
                  f"{targs} > train.log 2>&1 & echo $!")
        pid = ssh(alias, launch, capture=True).stdout.strip()
        log(f"training pid {pid}; tailing train.log (cutoff {a.max_minutes} min)")

        # tail until manifest or cutoff
        last = 0
        while True:
            r = ssh(alias, f"tail -c +{last + 1} {REMOTE}/train.log | head -c 20000; test -f {REMOTE}/out/train.manifest.json && echo __DONE__", check=False, capture=True, timeout=60)
            if r.returncode != 0:
                log("ssh lost (cutoff hit?)")
                break
            chunk = r.stdout
            done = chunk.endswith("__DONE__\n")
            if done:
                chunk = chunk[: -len("__DONE__\n")]
            if chunk:
                sys.stdout.write(chunk)
                sys.stdout.flush()
                last += len(chunk.encode())
            if done:
                log("train.manifest.json present")
                break
            if time.time() - started > a.max_minutes * 60 + 120:
                log("past cutoff; stopping wait")
                break
            time.sleep(20)

        # 5. fetch
        dest = Path(a.fetch_to) / run_name
        dest.mkdir(parents=True, exist_ok=True)
        for item in ("out/train.manifest.json", "out/samples.json", "train.log", "out/adapter", "out/gguf"):
            sh(["scp", "-q", "-r", "-o", "BatchMode=yes", f"{alias}:{REMOTE}/{item}", str(dest)], check=False)
        log(f"fetched to {dest}: {sorted(p.name for p in dest.iterdir())}")
    finally:
        # 6. stop or terminate — always
        try:
            if a.terminate:
                c.terminate_bma_instance(iid)
                log(f"instance {iid} TERMINATED")
            else:
                c.stop_bma_instance(iid)
                log(f"instance {iid} stopped (disk kept; --instance {iid} to reuse)")
        except Exception as e:
            log(f"STOP FAILED: {e} — stop it by hand: ComputeClient().stop_bma_instance('{iid}')")
        mins = (time.time() - started) / 60
        try:
            bal2 = c.get_credits_balance()
            log(f"session {mins:.1f} min ≈ ${rate * mins / 60:.2f}; credits after: {bal2.get('qbraidCredits')} (before {bal.get('qbraidCredits')})")
        except Exception:
            log(f"session {mins:.1f} min ≈ ${rate * mins / 60:.2f}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
