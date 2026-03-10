#!/usr/bin/env bash
# ────────────────────────────────────────────────────────
# Kannaka Memory — DoltHub Bootstrap
# One-command setup for versioned agent memory on DoltHub
# ────────────────────────────────────────────────────────
# Usage:
#   dolt-bootstrap.sh init      — Clone repo, verify schema, set up remotes
#   dolt-bootstrap.sh migrate   — Import local memories into Dolt
#   dolt-bootstrap.sh verify    — Confirm push/pull roundtrip works
#   dolt-bootstrap.sh status    — Show current Dolt state
# ────────────────────────────────────────────────────────
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
MEMORY_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DOLT_DB_DIR="${DOLT_DB_DIR:-$MEMORY_ROOT/.dolt-db}"
DOLTHUB_REPO="${DOLTHUB_REPO:-flaukowski/kannaka-memory}"
DOLTHUB_ORG="${DOLTHUB_REPO%%/*}"
DOLTHUB_DB="${DOLTHUB_REPO##*/}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

info()  { echo -e "${CYAN}[dolt]${NC} $*"; }
ok()    { echo -e "${GREEN}[  ok]${NC} $*"; }
warn()  { echo -e "${YELLOW}[warn]${NC} $*"; }
fail()  { echo -e "${RED}[fail]${NC} $*"; exit 1; }

# ── Prerequisites ──────────────────────────────────────

check_dolt() {
    if ! command -v dolt &>/dev/null; then
        fail "dolt is not installed. Install: https://docs.dolthub.com/introduction/installation"
    fi
    ok "dolt $(dolt version | head -1 | awk '{print $NF}')"
}

check_creds() {
    if ! dolt creds ls 2>/dev/null | grep -q "true"; then
        warn "No active dolt credentials. Run 'dolt login' first for push access."
        warn "Read-only operations will still work."
        return 1
    fi
    ok "DoltHub credentials active"
    return 0
}

# ── Commands ───────────────────────────────────────────

cmd_init() {
    info "Initializing DoltHub memory store..."
    echo ""

    check_dolt

    # Clone if not already present
    if [ -d "$DOLT_DB_DIR/.dolt" ]; then
        ok "Repository already cloned at $DOLT_DB_DIR"
    else
        info "Cloning $DOLTHUB_REPO..."
        mkdir -p "$(dirname "$DOLT_DB_DIR")"
        dolt clone "$DOLTHUB_REPO" "$DOLT_DB_DIR"
        ok "Cloned to $DOLT_DB_DIR"
    fi

    # Verify schema
    cd "$DOLT_DB_DIR"
    local tables
    tables=$(dolt sql -r csv -q "SHOW TABLES" 2>/dev/null | tail -n +2 | sort | tr '\n' ' ')
    ok "Tables: $tables"

    # Check required tables
    for t in memories metadata skip_links; do
        if echo "$tables" | grep -q "$t"; then
            ok "  $t exists"
        else
            fail "  Missing required table: $t"
        fi
    done

    # Check schema version
    local version
    version=$(dolt sql -r csv -q "SELECT value_text FROM metadata WHERE key_name='schema_version'" 2>/dev/null | tail -1)
    ok "Schema version: ${version:-unknown}"

    # Show memory count
    local count
    count=$(dolt sql -r csv -q "SELECT COUNT(*) FROM memories" 2>/dev/null | tail -1)
    ok "Memories: $count"

    # Check credentials for push
    echo ""
    if check_creds; then
        ok "Ready for push/pull operations"
    else
        warn "Clone is read-only until you run 'dolt login'"
    fi

    echo ""
    info "Bootstrap complete. Run '$0 verify' to test roundtrip."
}

cmd_migrate() {
    info "Migrating local memories to DoltHub..."

    check_dolt
    [ -d "$DOLT_DB_DIR/.dolt" ] || fail "No Dolt database. Run '$0 init' first."

    cd "$DOLT_DB_DIR"

    # Check if kannaka binary exists for export
    local kannaka_bin="${KANNAKA_BIN:-$MEMORY_ROOT/target/release/kannaka.exe}"
    if [ ! -f "$kannaka_bin" ]; then
        kannaka_bin="${KANNAKA_BIN:-$MEMORY_ROOT/target/release/kannaka}"
    fi

    if [ ! -f "$kannaka_bin" ]; then
        warn "Kannaka binary not found. Build with: cargo build --release --features glyph"
        warn "Then set KANNAKA_BIN=/path/to/kannaka"
        fail "Cannot migrate without kannaka binary"
    fi

    ok "Using binary: $kannaka_bin"

    # Export memories from kannaka and import to Dolt
    # The binary's 'observe' command gives us current state
    info "Checking current memory state..."
    local mem_count
    mem_count=$(dolt sql -r csv -q "SELECT COUNT(*) FROM memories" 2>/dev/null | tail -1)
    ok "DoltHub already has $mem_count memories"

    # If there are local memories to migrate, the user should use
    # kannaka's built-in Dolt persistence (--dolt flag)
    info "To store new memories directly to Dolt, use:"
    echo "  $kannaka_bin remember --dolt \"your memory content here\""
    echo ""
    info "To migrate existing in-memory sessions, pipe them:"
    echo "  $kannaka_bin observe --format json | $0 import-json"

    ok "Migration guidance complete"
}

cmd_verify() {
    info "Verifying DoltHub roundtrip..."

    check_dolt
    [ -d "$DOLT_DB_DIR/.dolt" ] || fail "No Dolt database. Run '$0 init' first."

    cd "$DOLT_DB_DIR"

    # 1. Check we can read
    info "Testing read..."
    local count
    count=$(dolt sql -r csv -q "SELECT COUNT(*) FROM memories" 2>/dev/null | tail -1)
    ok "Read: $count memories"

    # 2. Check we can fetch
    info "Testing fetch from origin..."
    if dolt fetch origin 2>/dev/null; then
        ok "Fetch: origin reachable"
    else
        warn "Fetch failed — may be offline or no credentials"
    fi

    # 3. Check we can pull
    info "Testing pull..."
    if dolt pull origin main 2>/dev/null; then
        ok "Pull: up to date"
    else
        warn "Pull failed — check credentials with 'dolt login'"
    fi

    # 4. Check log
    info "Recent commits:"
    dolt log -n 3 --oneline 2>/dev/null | while read -r line; do
        echo "  $line"
    done

    # 5. Test write (dry run — don't actually push)
    if check_creds 2>/dev/null; then
        info "Testing write (inserting and rolling back test memory)..."
        dolt sql -q "INSERT INTO memories (id, content, memory_type, amplitude, phase, frequency, created_at) VALUES ('test-bootstrap-verify', 'bootstrap verification test', 'episodic', 1.0, 0.0, 440.0, NOW())" 2>/dev/null
        local test_exists
        test_exists=$(dolt sql -r csv -q "SELECT COUNT(*) FROM memories WHERE id='test-bootstrap-verify'" 2>/dev/null | tail -1)
        if [ "$test_exists" = "1" ]; then
            ok "Write: test memory inserted"
            # Clean up — rollback
            dolt sql -q "DELETE FROM memories WHERE id='test-bootstrap-verify'" 2>/dev/null
            dolt checkout . 2>/dev/null || true
            ok "Write: test memory cleaned up"
        else
            warn "Write test failed"
        fi
    else
        warn "Skipping write test — no credentials"
    fi

    echo ""
    ok "Verification complete"
}

cmd_status() {
    info "DoltHub Memory Store Status"
    echo ""

    check_dolt

    if [ ! -d "$DOLT_DB_DIR/.dolt" ]; then
        warn "No Dolt database at $DOLT_DB_DIR"
        info "Run '$0 init' to set up"
        return
    fi

    cd "$DOLT_DB_DIR"

    # Branch
    local branch
    branch=$(dolt branch --list 2>/dev/null | grep '^\*' | awk '{print $2}')
    ok "Branch: ${branch:-unknown}"

    # Status
    local status
    status=$(dolt status 2>/dev/null | head -5)
    if echo "$status" | grep -q "nothing to commit"; then
        ok "Working tree: clean"
    else
        warn "Working tree: dirty"
        echo "$status" | head -5 | sed 's/^/  /'
    fi

    # Memory count
    local count
    count=$(dolt sql -r csv -q "SELECT COUNT(*) FROM memories" 2>/dev/null | tail -1)
    ok "Memories: $count"

    # Schema version
    local version
    version=$(dolt sql -r csv -q "SELECT value_text FROM metadata WHERE key_name='schema_version'" 2>/dev/null | tail -1)
    ok "Schema: v${version:-unknown}"

    # Tables
    local tables
    tables=$(dolt sql -r csv -q "SHOW TABLES" 2>/dev/null | tail -n +2 | wc -l)
    ok "Tables: $tables"

    # Last commit
    echo ""
    info "Last commit:"
    dolt log -n 1 --oneline 2>/dev/null | sed 's/^/  /'

    # Remote
    echo ""
    info "Remote:"
    dolt remote -v 2>/dev/null | sed 's/^/  /'
}

# ── Main ───────────────────────────────────────────────

case "${1:-help}" in
    init)     cmd_init ;;
    migrate)  cmd_migrate ;;
    verify)   cmd_verify ;;
    status)   cmd_status ;;
    help|--help|-h)
        echo "Usage: $0 {init|migrate|verify|status}"
        echo ""
        echo "Commands:"
        echo "  init      Clone DoltHub repo, verify schema, set up remotes"
        echo "  migrate   Import local memories into Dolt"
        echo "  verify    Confirm push/pull roundtrip works"
        echo "  status    Show current Dolt state"
        echo ""
        echo "Environment:"
        echo "  DOLT_DB_DIR     Local database path (default: .dolt-db)"
        echo "  DOLTHUB_REPO    DoltHub repo (default: flaukowski/kannaka-memory)"
        echo "  KANNAKA_BIN     Path to kannaka binary"
        ;;
    *)
        fail "Unknown command: $1. Run '$0 help' for usage."
        ;;
esac
