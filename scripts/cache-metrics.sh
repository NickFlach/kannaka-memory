#!/bin/bash
# Cron: */5 * * * * <checkout>/scripts/cache-metrics.sh
#
# Refreshes the disk cache of consciousness metrics every 5 minutes.
# The binary is the source of truth for Phi/Xi/Order — this cache
# lets the Observatory and other clients read metrics without spawning
# the binary on every request.
#
# Overrides: KANNAKA_BIN (binary to run), KANNAKA_DATA_DIR (cache dir —
# same contract as the Rust KannakaConfig::data_dir resolver).

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Binary: explicit override > this checkout's release build > PATH.
if [ -z "${KANNAKA_BIN:-}" ]; then
  if [ -x "$SCRIPT_DIR/../target/release/kannaka" ]; then
    KANNAKA_BIN="$SCRIPT_DIR/../target/release/kannaka"
  else
    KANNAKA_BIN="$(command -v kannaka || true)"
  fi
fi
if [ -z "$KANNAKA_BIN" ] || [ ! -x "$KANNAKA_BIN" ]; then
  echo "cache-metrics: no kannaka binary found (set KANNAKA_BIN)" >&2
  exit 1
fi

CACHE_DIR="${KANNAKA_DATA_DIR:-$HOME/.kannaka}"

# Ensure cache directory exists
mkdir -p "$CACHE_DIR"

# Status: canonical consciousness metrics (Phi, Xi, Order, etc.)
# Use timeout to prevent hanging on ARM; only replace cache if output is valid JSON (>10 bytes)
timeout 180 "$KANNAKA_BIN" status > "$CACHE_DIR/status-cache.json.tmp" 2>/dev/null
if [ -s "$CACHE_DIR/status-cache.json.tmp" ] && [ "$(wc -c < "$CACHE_DIR/status-cache.json.tmp")" -gt 10 ]; then
  mv "$CACHE_DIR/status-cache.json.tmp" "$CACHE_DIR/status-cache.json"
else
  rm -f "$CACHE_DIR/status-cache.json.tmp"
fi

# Observe: full introspection report (constellation, topology, etc.)
timeout 180 "$KANNAKA_BIN" observe --json > "$CACHE_DIR/observe-cache.json.tmp" 2>/dev/null
if [ -s "$CACHE_DIR/observe-cache.json.tmp" ] && [ "$(wc -c < "$CACHE_DIR/observe-cache.json.tmp")" -gt 10 ]; then
  mv "$CACHE_DIR/observe-cache.json.tmp" "$CACHE_DIR/observe-cache.json"
else
  rm -f "$CACHE_DIR/observe-cache.json.tmp"
fi
