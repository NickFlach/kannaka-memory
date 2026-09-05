# p2 — the Kannaka LoRA adapter (ADR-0057 P2)

Four scripts, one path: corpus → SFT set → adapter on a qBraid GPU → served
on debain2 as `kannaka-brain-v1`.

```
python tools/corpus/p2/prep_sft.py ~/.kannaka-corpus/out/kannaka-corpus-voice-<date>.jsonl \
    --out ~/.kannaka-corpus/sft                       # 551 train / 57 holdout (deterministic)

# pipeline smoke on CPU (0.5B base, 3 steps, merge) — proves the code, not the model
python tools/corpus/p2/train_lora.py --base Qwen/Qwen2.5-0.5B-Instruct --data ~/.kannaka-corpus/sft \
    --out ~/.kannaka-corpus/runs/cpu-smoke --cpu-smoke --max-steps 3 --max-len 256 --r 4 --merge

# qBraid GPU, gated (ADR-0057): refuses without --allow-spend; cutoff set before any work
python tools/corpus/p2/run_qbraid.py --profile gpu-rtx-4090 --data ~/.kannaka-corpus/sft \
    --base Qwen/Qwen2.5-1.5B-Instruct --max-minutes 45 --allow-spend -- --max-steps 30   # ≤ $0.65
python tools/corpus/p2/run_qbraid.py --profile gpu-a100-sxm --data ~/.kannaka-corpus/sft \
    --base Qwen/Qwen2.5-14B-Instruct --max-minutes 240 --allow-spend \
    -- --qlora --epochs 2 --r 32 --merge --gguf q4_K_M                                    # ≤ $9.96

# on debain2: register the merged+quantized GGUF in ollama and the gateway
bash serve_debain2.sh ~/kannaka-brain-q4_K_M.gguf kannaka-brain-v1
```

**Data.** `prep_sft.py` turns tier-1 records into chat examples. Ghost
Signals lines are paired with the preceding `[FLAUKOWSKI]` block as the user
turn (a real exchange she wrote both sides of); lyrics get a "write the lyrics
for <title> (<album>)" prompt; identity sections get "tell me about <section>".
Tier 2/3, fiction and Flaukowski's own lines are never targets — the P1 rule,
checked again on the way out. Hold-out is by id hash, so every run scores the
same lines.

**Metric.** Held-out loss / perplexity before and after, on those fixed lines.
Generation samples on held-out prompts go to `samples.json` for the blind
voice A/B; they are for a judge, not the metric.

**Serving.** ollama cannot load a safetensors adapter for Qwen2 (Llama/
Mistral/Gemma only), so the trainer merges the adapter into the base and
converts with llama.cpp (`--merge --gguf q4_K_M`). The adapter directory is
still saved and is the ownable artefact; the GGUF is the serving copy.

**Spend gate.** `run_qbraid.py` provisions a BMA instance only with
`--allow-spend`, on single-GPU profiles only, sets `max_session_minutes` and
`auto_stop_idle_minutes` *before* shipping anything (and terminates the
instance if that call fails), tails the log until `train.manifest.json`
exists or the cutoff hits, fetches the outputs to `~/.kannaka-corpus/runs/`,
and always stops the instance in `finally`. Prints credits before/after.

**Run it from Linux.** The SDK's ssh ProxyCommand is a websocket-to-stdio
bridge (`python -m qbraid_core.services.compute.ssh bridge …`) that crashes
on Windows (`_ProactorReadPipeTransport … _empty_waiter`, Python 3.14) and
Git-Bash's MSYS ssh mangles the backslash paths it writes into
`~/.ssh/config.d/qbraid`. debain2 is the runner: `~/qbraid-venv`,
`~/.qbraid/qbraidrc`, `~/kannaka-p2-runner/{p2,sft}`; outputs fetch to
`~/.kannaka-corpus/runs/` there — which is where they get served anyway.

Everything under `~/.kannaka-corpus/` is private until the ADR-0057 decision.
