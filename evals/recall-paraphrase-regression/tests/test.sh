#!/bin/bash
# Verifier entrypoint. Stdlib-only (network_mode = none — no downloads here).
# Reward = recall@10 (continuous). Exit 3 from verify.py = infrastructure error,
# which must surface as a verifier failure, not a zero agent score.
set -uo pipefail
mkdir -p /logs/verifier
python3 /tests/verify.py
code=$?
if [ $code -eq 3 ]; then
  echo "verifier reported infrastructure error" >&2
  exit 3
fi
exit $code
