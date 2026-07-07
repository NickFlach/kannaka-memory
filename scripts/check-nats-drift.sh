#!/usr/bin/env bash
# check-nats-drift.sh — detect drift between the committed NATS config and the
# DEPLOYED /etc/nats/nats.conf on the swarm host.
#
# It compares only the authorization ACL (the publish/subscribe subject
# allow-lists) with all secrets REDACTED — it never prints or diffs passwords.
# Exit 0 = no ACL drift, exit 1 = drift (printed), exit 2 = usage/connection error.
#
# Why: `anon` on the live server was deliberately widened for the open swarm
# (KANNAKA.events.>, memory.new, consciousness, substrate.absorb). If the
# committed baseline ever drifts narrower again, deploying it would silently
# revert that widening and break the swarm. Run this in cron/CI to catch it.
#
# Usage:
#   SWARM_SSH="opc@170.9.238.136" \
#   SWARM_KEY="$HOME/.ssh/ninja-portal-ed25519" \
#   scripts/check-nats-drift.sh
#
# CI note: requires SSH reach to the swarm host (SWARM_SSH/SWARM_KEY as secrets).
# If the runner cannot reach it, run this as a scheduled job on a host that can.
set -euo pipefail

HERE="$(cd "$(dirname "$0")/.." && pwd)"
COMMITTED="$HERE/config/nats-server.conf"
SSH_TGT="${SWARM_SSH:-}"
SSH_KEY="${SWARM_KEY:-$HOME/.ssh/ninja-portal-ed25519}"
REMOTE_CONF="${REMOTE_CONF:-/etc/nats/nats.conf}"

[ -f "$COMMITTED" ] || { echo "error: committed config not found: $COMMITTED" >&2; exit 2; }
[ -n "$SSH_TGT" ] || { echo "error: set SWARM_SSH=user@host" >&2; exit 2; }

# Redact any secret-bearing value before anything is printed or compared.
redact() { sed -E 's/(password|token|seed|nkey)([[:space:]]*[:=][[:space:]]*)("?)[^"[:space:]]+/\1\2\3REDACTED/Ig'; }
# Extract the set of quoted subject tokens (the ACL surface), sorted + unique.
acl() { grep -oE '"(KANNAKA|QUEEN|queen|_INBOX|\$JS)[^"]*"' | sort -u; }

LIVE="$(ssh -i "$SSH_KEY" -o BatchMode=yes -o ConnectTimeout=12 -o StrictHostKeyChecking=accept-new \
          "$SSH_TGT" "sudo -n cat '$REMOTE_CONF' 2>/dev/null || cat '$REMOTE_CONF'")" \
  || { echo "error: could not read $REMOTE_CONF on $SSH_TGT" >&2; exit 2; }

if diff -u \
     <(redact < "$COMMITTED" | acl) \
     <(printf '%s\n' "$LIVE" | redact | acl) ; then
  echo "OK: committed config/nats-server.conf ACL matches deployed $REMOTE_CONF."
  exit 0
else
  echo "" >&2
  echo "DRIFT: committed nats-server.conf ACL differs from deployed (diff above:" >&2
  echo "  '-' = committed only, '+' = live only). Reconcile before any deploy." >&2
  exit 1
fi
