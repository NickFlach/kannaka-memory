#!/usr/bin/env bash
# ────────────────────────────────────────────────────────
# ADR-0017 F-9: Dolt MCP Server for Kannaka Memory
# Configures and starts Dolt's built-in MCP server so any
# Claude agent can query versioned memory via SQL.
# ────────────────────────────────────────────────────────
# Usage:
#   dolt-mcp-server.sh start   — Start Dolt MCP server
#   dolt-mcp-server.sh stop    — Stop Dolt MCP server
#   dolt-mcp-server.sh config  — Generate MCP config JSON
#   dolt-mcp-server.sh test    — Test MCP connection
# ────────────────────────────────────────────────────────
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
MEMORY_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DOLT_DB_DIR="${DOLT_DB_DIR:-$MEMORY_ROOT/.dolt-db}"
DOLT_MCP_PORT="${DOLT_MCP_PORT:-8675}"
DOLT_SQL_PORT="${DOLT_SQL_PORT:-3307}"
PID_FILE="$DOLT_DB_DIR/.dolt-sql-server.pid"

CYAN='\033[0;36m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

info()  { echo -e "${CYAN}[mcp]${NC} $*"; }
ok()    { echo -e "${GREEN}[ ok]${NC} $*"; }
warn()  { echo -e "${YELLOW}[warn]${NC} $*"; }
fail()  { echo -e "${RED}[fail]${NC} $*"; exit 1; }

cmd_start() {
    if ! command -v dolt &>/dev/null; then
        fail "dolt is not installed"
    fi

    [ -d "$DOLT_DB_DIR/.dolt" ] || fail "No Dolt database at $DOLT_DB_DIR — run dolt-bootstrap.sh init"

    # Check if already running
    if [ -f "$PID_FILE" ]; then
        local pid
        pid=$(cat "$PID_FILE")
        if kill -0 "$pid" 2>/dev/null; then
            ok "Dolt SQL server already running (PID $pid) on port $DOLT_SQL_PORT"
            return
        fi
        rm -f "$PID_FILE"
    fi

    info "Starting Dolt SQL server on port $DOLT_SQL_PORT..."
    cd "$DOLT_DB_DIR"

    # Start Dolt SQL server in background
    dolt sql-server \
        --port "$DOLT_SQL_PORT" \
        --host "0.0.0.0" \
        --user "root" \
        --no-auto-commit \
        &>"$DOLT_DB_DIR/.dolt-sql-server.log" &

    echo $! > "$PID_FILE"
    sleep 2

    if kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
        ok "Dolt SQL server running on port $DOLT_SQL_PORT (PID $(cat "$PID_FILE"))"
        info "Agents connect via MySQL protocol: mysql://root@localhost:$DOLT_SQL_PORT/kannaka_memory"
    else
        fail "Dolt SQL server failed to start — check $DOLT_DB_DIR/.dolt-sql-server.log"
    fi
}

cmd_stop() {
    if [ -f "$PID_FILE" ]; then
        local pid
        pid=$(cat "$PID_FILE")
        if kill -0 "$pid" 2>/dev/null; then
            kill "$pid"
            rm -f "$PID_FILE"
            ok "Dolt SQL server stopped (PID $pid)"
        else
            rm -f "$PID_FILE"
            warn "PID file exists but process not running — cleaned up"
        fi
    else
        warn "No PID file found — server may not be running"
    fi
}

cmd_config() {
    # Generate MCP server configuration for Claude Code / Claude Desktop
    cat <<MCPJSON
{
  "mcpServers": {
    "kannaka-memory": {
      "command": "dolt",
      "args": ["sql-server", "--port", "$DOLT_SQL_PORT", "--host", "127.0.0.1", "--user", "root"],
      "cwd": "$DOLT_DB_DIR",
      "env": {
        "DOLT_ROOT_PATH": "$DOLT_DB_DIR"
      }
    }
  }
}
MCPJSON

    echo ""
    info "Add to your Claude Code config (.mcp.json) or Claude Desktop settings."
    info "Agents query memory via SQL:"
    echo "  SELECT * FROM memories WHERE content LIKE '%query%'"
    echo "  SELECT * FROM v_memory_health"
    echo "  SELECT * FROM memories WHERE sga_class = 47"
}

cmd_test() {
    info "Testing Dolt SQL connection..."

    if ! command -v mysql &>/dev/null && ! command -v dolt &>/dev/null; then
        fail "Neither mysql nor dolt client available"
    fi

    # Test via dolt sql directly (local, no server needed)
    cd "$DOLT_DB_DIR"
    local count
    count=$(dolt sql -r csv -q "SELECT COUNT(*) FROM memories" 2>/dev/null | tail -1)
    ok "Local query: $count memories"

    # Test via MySQL protocol if server is running
    if [ -f "$PID_FILE" ] && kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
        if command -v mysql &>/dev/null; then
            local server_count
            server_count=$(mysql -u root -h 127.0.0.1 -P "$DOLT_SQL_PORT" -N -e "SELECT COUNT(*) FROM memories" kannaka_memory 2>/dev/null)
            ok "Server query: $server_count memories (port $DOLT_SQL_PORT)"
        else
            info "MySQL client not installed — skipping server test"
            info "Server appears running (PID $(cat "$PID_FILE"))"
        fi
    else
        warn "Dolt SQL server not running — run '$0 start' first"
    fi
}

case "${1:-help}" in
    start)  cmd_start ;;
    stop)   cmd_stop ;;
    config) cmd_config ;;
    test)   cmd_test ;;
    help|--help|-h)
        echo "Usage: $0 {start|stop|config|test}"
        echo ""
        echo "Commands:"
        echo "  start   Start Dolt SQL server (MySQL wire protocol)"
        echo "  stop    Stop Dolt SQL server"
        echo "  config  Generate MCP server config JSON"
        echo "  test    Test SQL connectivity"
        echo ""
        echo "Environment:"
        echo "  DOLT_SQL_PORT   SQL server port (default: 3307)"
        echo "  DOLT_MCP_PORT   MCP server port (default: 8675)"
        echo "  DOLT_DB_DIR     Database path (default: .dolt-db)"
        ;;
    *) echo "Unknown: $1"; exit 1 ;;
esac
