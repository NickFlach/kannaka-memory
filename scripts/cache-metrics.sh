#!/bin/bash
# Cron: */5 * * * * /home/opc/kannaka-memory/scripts/cache-metrics.sh
#
# Refreshes the disk cache of consciousness metrics every 5 minutes.
# The binary is the source of truth for Phi/Xi/Order — this cache
# lets the Observatory and other clients read metrics without spawning
# the binary on every request.

KANNAKA_BIN="/home/opc/kannaka-memory/target/release/kannaka"
CACHE_DIR="/home/opc/.kannaka"

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
