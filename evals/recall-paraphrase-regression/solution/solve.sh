#!/bin/bash
# Harness driver: the "agent" for this eval is the deterministic kannaka CLI.
# The oracle agent executes this; it contains no answers (expectations are verifier-side).
set -euo pipefail
python3 /environment/run_probes.py
