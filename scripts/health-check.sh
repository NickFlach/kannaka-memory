#!/bin/bash
# NATS health check script for monitoring (QS-8, #59)
# Returns exit code 0 if healthy, 1 if unhealthy.
# Use with: systemd watchdog, Uptime Robot, or custom monitoring.

set -euo pipefail

NATS_HOST="${NATS_HOST:-localhost}"
NATS_PORT="${NATS_PORT:-4222}"
NATS_MONITOR="${NATS_MONITOR:-http://localhost:8222}"
TIMEOUT=5

# Check 1: TCP port reachable
if ! timeout ${TIMEOUT} bash -c "echo > /dev/tcp/${NATS_HOST}/${NATS_PORT}" 2>/dev/null; then
    echo "FAIL: NATS port ${NATS_PORT} not reachable"
    exit 1
fi

# Check 2: HTTP monitoring endpoint
if command -v curl &>/dev/null; then
    HTTP_STATUS=$(curl -s -o /dev/null -w "%{http_code}" --connect-timeout ${TIMEOUT} "${NATS_MONITOR}/varz" 2>/dev/null || echo "000")
    if [ "${HTTP_STATUS}" != "200" ]; then
        echo "FAIL: NATS monitoring endpoint returned ${HTTP_STATUS}"
        exit 1
    fi

    # Check 3: JetStream health
    JS_STATUS=$(curl -s --connect-timeout ${TIMEOUT} "${NATS_MONITOR}/jsz" 2>/dev/null || echo "{}")
    JS_DISABLED=$(echo "${JS_STATUS}" | grep -c '"disabled":true' 2>/dev/null || echo "0")
    if [ "${JS_DISABLED}" != "0" ]; then
        echo "WARN: JetStream is disabled"
        # Don't fail — JetStream is optional
    fi

    # Check 4: Connection count
    CONNECTIONS=$(curl -s --connect-timeout ${TIMEOUT} "${NATS_MONITOR}/connz?limit=0" 2>/dev/null | grep -o '"num_connections":[0-9]*' | head -1 | cut -d: -f2 || echo "0")
    echo "OK: NATS healthy. Connections: ${CONNECTIONS}"
else
    echo "OK: NATS port reachable (curl not available for full check)"
fi

exit 0
