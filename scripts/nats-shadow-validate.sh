#!/usr/bin/env bash
# nats-shadow-validate.sh — prove the ADR-0042 Phase-1a auth model on a SHADOW
# server (throwaway port + store_dir) WITHOUT touching production :4222.
#
# Boots nats-accounts.conf on :4999 with test passwords, then asserts:
#   1. `writer` CAN publish a memory subject.
#   2. a scoped non-writer (`radio`) CANNOT publish a memory subject.
#   3. `anon` CANNOT publish a control subject (KANNAKA.ask.>).
#   4. `anon` CAN publish the open memory lane (KANNAKA.events.>).
#   5. pub/sub round-trips for a normal subject.
# Requires: nats-server + nats CLI on PATH. Run on the Oracle (or any host).
set -u
CONF_SRC="${1:-$(dirname "$0")/../config/nats-accounts.conf}"
PORT=4999
DIR=$(mktemp -d)
CONF="$DIR/shadow.conf"
PASS="shadowtestpass"

# Materialize: only rewrite port + store_dir (small, safe seds). The $NATS_*
# password placeholders are left intact and expanded by nats-server from the
# environment (native NATS env-var expansion) — we export them all to $PASS.
sed -e "s#0.0.0.0:4222#127.0.0.1:$PORT#" "$CONF_SRC" \
  | sed -e "s#port: 4223#port: 4998#" \
  | sed -e "s#store_dir: .*#store_dir: $DIR/js#" > "$CONF"
# Every credential var the config references -> the shared test password.
for v in NATS_PASSWORD NATS_ANON_PASS NATS_WRITER_PASS NATS_SERVE_PASS \
         NATS_RADIO_PASS NATS_PRESENCE_PASS NATS_RESPONDER_PASS NATS_EYE_PASS \
         NATS_KANNAKTOPUS_PASS NATS_UIBRIDGE_PASS; do export "$v=$PASS"; done

echo "== config syntax check =="
nats-server -t -c "$CONF" || { echo "SYNTAX FAIL"; exit 1; }

echo "== boot shadow server on :$PORT =="
nats-server -c "$CONF" >"$DIR/server.log" 2>&1 &
SRV=$!
trap 'kill $SRV 2>/dev/null; rm -rf "$DIR"' EXIT
for i in $(seq 1 20); do nats --server "nats://127.0.0.1:$PORT" server ping >/dev/null 2>&1 && break; sleep 0.5; done

U(){ echo "nats://$1:$PASS@127.0.0.1:$PORT"; }
pass=0; fail=0
check(){ # desc expect(allow|deny) user subject
  local desc="$1" expect="$2" user="$3" subj="$4"
  if nats --server "$(U "$user")" pub "$subj" "test" >/dev/null 2>&1; then got=allow; else got=deny; fi
  if [ "$got" = "$expect" ]; then echo "  PASS: $desc ($user -> $subj = $got)"; pass=$((pass+1));
  else echo "  FAIL: $desc ($user -> $subj = $got, expected $expect)"; fail=$((fail+1)); fi
}

echo "== assertions =="
check "writer publishes memory"          allow writer "KANNAKA.memory.new"
check "radio DENIED memory publish"      deny  radio  "KANNAKA.memory.new"
check "anon DENIED control (ask)"        deny  anon   "KANNAKA.ask.kannaka-prime"
check "anon ALLOWED open memory lane"    allow anon   "KANNAKA.events.test"
check "anon ALLOWED consciousness"       allow anon   "KANNAKA.consciousness"
check "presence publishes obc events"    allow presence "KANNAKA.events.obc.dm_message"
check "presence DENIED memory publish"   deny  presence "KANNAKA.memory.new"
check "compat internal still god (1a)"   allow kannaka_internal "KANNAKA.memory.new"

echo "== result: $pass passed, $fail failed =="
[ "$fail" -eq 0 ] && echo "SHADOW VALIDATION PASSED" || { echo "SHADOW VALIDATION FAILED"; exit 1; }
