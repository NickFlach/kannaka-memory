#!/bin/bash
# Verifier entrypoint. Stdlib-only (network_mode = no-network).
# Reward = recall@10 over the zero-overlap set; exit 3 = infrastructure error.
set -uo pipefail
mkdir -p /logs/verifier
python3 /tests/verify.py
code=$?
if [ $code -eq 3 ]; then
  echo "verifier reported infrastructure error" >&2
  exit 3
fi
exit $code
