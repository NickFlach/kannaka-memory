#!/usr/bin/env bash
# ────────────────────────────────────────────────────────
# Kannaka Constellation — start/stop/status all 3 services
# ────────────────────────────────────────────────────────
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
MEMORY_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
RADIO_ROOT="$(cd "$MEMORY_ROOT/../kannaka-radio" 2>/dev/null && pwd || echo "")"
EYE_ROOT="$(cd "$MEMORY_ROOT/../kannaka-eye" 2>/dev/null && pwd || echo "")"

RADIO_PORT="${RADIO_PORT:-8888}"
EYE_PORT="${EYE_PORT:-3333}"

# Binary location
if [ -n "${KANNAKA_BIN:-}" ]; then
    BIN="$KANNAKA_BIN"
elif [ -f "$MEMORY_ROOT/target/release/kannaka.exe" ]; then
    BIN="$MEMORY_ROOT/target/release/kannaka.exe"
elif [ -f "$MEMORY_ROOT/target/release/kannaka" ]; then
    BIN="$MEMORY_ROOT/target/release/kannaka"
else
    BIN=""
fi

usage() {
    cat <<EOF
Kannaka Constellation — Unified Control

Usage: constellation.sh <command>

Commands:
  start     Build binary + start radio + start eye
  stop      Stop eye + stop radio
  status    Health check all three
  build     Build the kannaka binary (audio + glyph + collective)

Ports:
  Radio:  $RADIO_PORT  (env: RADIO_PORT)
  Eye:    $EYE_PORT  (env: EYE_PORT)

Repos:
  Memory: $MEMORY_ROOT
  Radio:  ${RADIO_ROOT:-not found}
  Eye:    ${EYE_ROOT:-not found}
EOF
}

cmd_build() {
    echo "[constellation] Building kannaka binary..."
    cd "$MEMORY_ROOT"
    cargo build --release --features audio,glyph,collective 2>&1 | tail -5

    if [ -f "$MEMORY_ROOT/target/release/kannaka.exe" ]; then
        BIN="$MEMORY_ROOT/target/release/kannaka.exe"
    elif [ -f "$MEMORY_ROOT/target/release/kannaka" ]; then
        BIN="$MEMORY_ROOT/target/release/kannaka"
    fi

    if [ -n "$BIN" ]; then
        echo "[constellation] Binary ready: $BIN"
        echo "[constellation] Testing classify..."
        echo "test" | "$BIN" classify 2>/dev/null | head -c 80
        echo ""
    else
        echo "[constellation] WARNING: binary not found after build"
    fi
}

cmd_start() {
    echo "[constellation] Starting Kannaka Constellation..."

    # Build if binary doesn't exist
    if [ -z "$BIN" ]; then
        cmd_build
    fi

    # Export binary path for child services
    export KANNAKA_BIN="${BIN:-}"

    # Start Radio
    if [ -n "$RADIO_ROOT" ] && [ -f "$RADIO_ROOT/server.js" ]; then
        # Check if already running
        if curl -s "http://localhost:$RADIO_PORT/api/state" > /dev/null 2>&1; then
            echo "[constellation] Radio already running on port $RADIO_PORT"
        else
            echo "[constellation] Starting Radio on port $RADIO_PORT..."
            cd "$RADIO_ROOT"
            node server.js --port "$RADIO_PORT" &
            disown
            sleep 1
            echo "[constellation] Radio started: http://localhost:$RADIO_PORT"
        fi
    else
        echo "[constellation] Radio not found — skipping"
    fi

    # Start Eye
    if [ -n "$EYE_ROOT" ] && [ -f "$EYE_ROOT/server.js" ]; then
        if curl -s "http://localhost:$EYE_PORT/" > /dev/null 2>&1; then
            echo "[constellation] Eye already running on port $EYE_PORT"
        else
            echo "[constellation] Starting Eye on port $EYE_PORT..."
            cd "$EYE_ROOT"
            KANNAKA_BIN="${BIN:-}" node server.js --port "$EYE_PORT" &
            disown
            sleep 1
            echo "[constellation] Eye started: http://localhost:$EYE_PORT"
        fi
    else
        echo "[constellation] Eye not found — skipping"
    fi

    echo ""
    echo "[constellation] Constellation is live."
    [ -n "$BIN" ] && echo "  Binary:  $BIN"
    echo "  Radio:   http://localhost:$RADIO_PORT"
    echo "  Eye:     http://localhost:$EYE_PORT"
}

cmd_stop() {
    echo "[constellation] Stopping Constellation..."

    # Stop Eye
    if [ -n "$EYE_ROOT" ]; then
        local pid=$(lsof -ti "tcp:$EYE_PORT" 2>/dev/null || true)
        if [ -n "$pid" ]; then
            kill "$pid" 2>/dev/null || true
            echo "[constellation] Eye stopped (pid $pid)"
        else
            echo "[constellation] Eye not running"
        fi
    fi

    # Stop Radio
    if [ -n "$RADIO_ROOT" ]; then
        local pid=$(lsof -ti "tcp:$RADIO_PORT" 2>/dev/null || true)
        if [ -n "$pid" ]; then
            kill "$pid" 2>/dev/null || true
            echo "[constellation] Radio stopped (pid $pid)"
        else
            echo "[constellation] Radio not running"
        fi
    fi
}

cmd_status() {
    echo "Kannaka Constellation Status"
    echo "════════════════════════════"
    echo ""

    # Memory (binary)
    if [ -n "$BIN" ] && [ -f "$BIN" ]; then
        echo "  Memory:  OK ($BIN)"
        local classify_ok=$(echo "test" | "$BIN" classify 2>/dev/null | grep -c "fold_sequence" || true)
        [ "$classify_ok" -gt 0 ] && echo "           classify: working" || echo "           classify: FAILED"
    else
        echo "  Memory:  NO BINARY"
    fi

    # Radio
    if curl -s "http://localhost:$RADIO_PORT/api/state" > /dev/null 2>&1; then
        local state=$(curl -s "http://localhost:$RADIO_PORT/api/state" 2>/dev/null)
        echo "  Radio:   RUNNING (http://localhost:$RADIO_PORT)"
    else
        echo "  Radio:   STOPPED"
    fi

    # Eye
    if curl -s "http://localhost:$EYE_PORT/" > /dev/null 2>&1; then
        echo "  Eye:     RUNNING (http://localhost:$EYE_PORT)"
        # Test classify endpoint
        local eye_classifier=$(curl -s -X POST "http://localhost:$EYE_PORT/api/process" \
            -H "Content-Type: application/json" \
            -d '{"data":"test","type":"text"}' 2>/dev/null | grep -o '"classifier":"[^"]*"' || true)
        [ -n "$eye_classifier" ] && echo "           $eye_classifier"
    else
        echo "  Eye:     STOPPED"
    fi

    echo ""
}

# ── Main ────────────────────────────────────────────
CMD="${1:-}"
shift 2>/dev/null || true

case "$CMD" in
    start)   cmd_start ;;
    stop)    cmd_stop ;;
    status)  cmd_status ;;
    build)   cmd_build ;;
    -h|--help|help|"") usage ;;
    *) echo "[constellation] Unknown: $CMD"; usage; exit 1 ;;
esac
