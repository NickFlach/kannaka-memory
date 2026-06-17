#!/bin/bash
# witness-prune-cron.sh — keep the witness HRM from growing unbounded.
#
# Run hourly from cron. When total memories exceed the threshold it backs up
# the HRM, then prunes audio perception memories by content prefix and restarts
# the witness loop (so `kannaka hear` doesn't race the prune on the HRM lock).
#
# Prefixes pruned:
#   audio:heard   — current descriptive perception memories (ADR: hear stores
#                   "audio:heard <tempo> <tags> | ...")
#   audio:/tmp/   — legacy path-only memories from before the descriptive change
#   audio:/var/tmp/
#
# NOTE: the previous on-host copy of this script was corrupted (an unquoted
# heredoc had baked in evaluated $() output), so it silently did nothing — which
# is why the witness accumulated thousands of audio memories. This is the
# version-controlled, working replacement.
set -u

KANNAKA="${KANNAKA_BIN:-/usr/local/bin/kannaka}"
SERVICE="${WITNESS_SERVICE:-kannaka-witness}"
THRESHOLD="${WITNESS_PRUNE_THRESHOLD:-4000}"
HRM="${KANNAKA_HRM:-$HOME/.kannaka/kannaka.hrm}"
TS="$(date -u +%FT%TZ)"

total="$("$KANNAKA" stats 2>/dev/null | grep -oiE "[0-9]+ (total|memories)" | grep -oE "[0-9]+" | head -1)"
total="${total:-0}"
echo "[witness-prune $TS] memories=$total threshold=$THRESHOLD"

if [ "$total" -lt "$THRESHOLD" ]; then
  echo "[witness-prune $TS] below threshold — no action"
  exit 0
fi

# Stop the loop so its `kannaka hear` doesn't race the prune on the HRM lock.
sudo systemctl stop "$SERVICE"
sleep 1

# Backup before pruning (same pattern as the primary box).
cp "$HRM" "${HRM}.pre-prune-$(date +%s)" 2>/dev/null || true

for prefix in "audio:heard" "audio:/tmp/" "audio:/var/tmp/"; do
  echo "[witness-prune $TS] prune-prefix \"$prefix\""
  "$KANNAKA" prune-prefix "$prefix" 2>&1 | tail -1
done

sudo systemctl start "$SERVICE"
echo "[witness-prune $TS] done"

# Retention: keep only the 24 most recent pre-prune backups.
ls -1t "${HRM}".pre-prune-* 2>/dev/null | tail -n +25 | xargs -r rm -f
