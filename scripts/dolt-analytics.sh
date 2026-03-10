#!/usr/bin/env bash
# ────────────────────────────────────────────────────────
# ADR-0017 F-10: Memory Analytics Dashboard
# Creates SQL views on DoltHub for memory health monitoring
# ────────────────────────────────────────────────────────
# Usage: dolt-analytics.sh [install|query <view>|status]
# ────────────────────────────────────────────────────────
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
MEMORY_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DOLT_DB_DIR="${DOLT_DB_DIR:-$MEMORY_ROOT/.dolt-db}"

CYAN='\033[0;36m'
GREEN='\033[0;32m'
NC='\033[0m'

info()  { echo -e "${CYAN}[analytics]${NC} $*"; }
ok()    { echo -e "${GREEN}[  ok]${NC} $*"; }

cmd_install() {
    info "Installing analytics views..."
    cd "$DOLT_DB_DIR"

    # V1: Memory Health — amplitude distribution, decay rates, consciousness proxy
    dolt sql -q "
    CREATE OR REPLACE VIEW v_memory_health AS
    SELECT
        COUNT(*) as total_memories,
        SUM(CASE WHEN amplitude > 0.5 THEN 1 ELSE 0 END) as strong_memories,
        SUM(CASE WHEN amplitude <= 0.1 THEN 1 ELSE 0 END) as ghost_memories,
        SUM(CASE WHEN hallucinated = 1 THEN 1 ELSE 0 END) as hallucinations,
        SUM(CASE WHEN disputed = 1 THEN 1 ELSE 0 END) as disputed,
        ROUND(AVG(amplitude), 4) as avg_amplitude,
        ROUND(AVG(frequency), 4) as avg_frequency,
        ROUND(AVG(decay_rate), 8) as avg_decay_rate,
        MIN(created_at) as oldest_memory,
        MAX(created_at) as newest_memory,
        COUNT(DISTINCT origin_agent) as agent_count
    FROM memories
    " 2>/dev/null
    ok "v_memory_health"

    # V2: Dream History — consolidation stats from commit messages
    dolt sql -q "
    CREATE OR REPLACE VIEW v_dream_history AS
    SELECT
        commit_hash,
        date as dream_date,
        message as dream_report
    FROM dolt_log
    WHERE message LIKE 'dream%' OR message LIKE 'pre-dream%'
    ORDER BY date DESC
    LIMIT 50
    " 2>/dev/null
    ok "v_dream_history"

    # V3: Agent Contributions — memories per origin agent
    dolt sql -q "
    CREATE OR REPLACE VIEW v_agent_contributions AS
    SELECT
        COALESCE(origin_agent, 'local') as agent,
        COUNT(*) as memory_count,
        ROUND(AVG(amplitude), 4) as avg_amplitude,
        SUM(CASE WHEN hallucinated = 1 THEN 1 ELSE 0 END) as hallucinations,
        SUM(CASE WHEN disputed = 1 THEN 1 ELSE 0 END) as disputed,
        MAX(sync_version) as max_sync_version,
        MIN(created_at) as first_memory,
        MAX(created_at) as last_memory
    FROM memories
    GROUP BY COALESCE(origin_agent, 'local')
    ORDER BY memory_count DESC
    " 2>/dev/null
    ok "v_agent_contributions"

    # V4: SGA Distribution — class frequency across all memories
    dolt sql -q "
    CREATE OR REPLACE VIEW v_sga_distribution AS
    SELECT
        sga_class as class_index,
        sga_centroid_h2 as h2,
        sga_centroid_d as d,
        sga_centroid_l as l,
        COUNT(*) as memory_count,
        ROUND(AVG(amplitude), 4) as avg_amplitude
    FROM memories
    WHERE sga_class IS NOT NULL
    GROUP BY sga_class, sga_centroid_h2, sga_centroid_d, sga_centroid_l
    ORDER BY memory_count DESC
    " 2>/dev/null
    ok "v_sga_distribution"

    # V5: Layer Distribution — temporal depth analysis
    dolt sql -q "
    CREATE OR REPLACE VIEW v_layer_distribution AS
    SELECT
        layer_depth,
        CASE layer_depth
            WHEN 0 THEN 'episodic'
            WHEN 1 THEN 'short-term'
            WHEN 2 THEN 'long-term'
            WHEN 3 THEN 'deep'
            ELSE 'archival'
        END as layer_name,
        COUNT(*) as memory_count,
        ROUND(AVG(amplitude), 4) as avg_amplitude,
        SUM(CASE WHEN hallucinated = 1 THEN 1 ELSE 0 END) as hallucinations
    FROM memories
    GROUP BY layer_depth
    ORDER BY layer_depth
    " 2>/dev/null
    ok "v_layer_distribution"

    # V6: Quarantine Status — dispute overview
    dolt sql -q "
    CREATE OR REPLACE VIEW v_quarantine_status AS
    SELECT
        COALESCE(resolution, 'pending') as status,
        COUNT(*) as dispute_count,
        ROUND(AVG(phase_diff), 4) as avg_phase_diff,
        MIN(created_at) as oldest,
        MAX(created_at) as newest
    FROM quarantine
    GROUP BY COALESCE(resolution, 'pending')
    " 2>/dev/null
    ok "v_quarantine_status"

    # V7: Skip Link Network — connection density
    dolt sql -q "
    CREATE OR REPLACE VIEW v_skip_link_network AS
    SELECT
        link_type,
        COUNT(*) as link_count,
        ROUND(AVG(weight), 4) as avg_weight,
        ROUND(MIN(weight), 4) as min_weight,
        ROUND(MAX(weight), 4) as max_weight
    FROM skip_links
    GROUP BY link_type
    ORDER BY link_count DESC
    " 2>/dev/null
    ok "v_skip_link_network"

    # Commit the views
    dolt add .
    dolt commit -m "analytics: install 7 dashboard views (ADR-0017 F-10)" --allow-empty \
        --author "Kannaka Agent <kannaka@local>" 2>/dev/null || true

    echo ""
    info "7 analytics views installed. Query with:"
    echo "  dolt sql -r tabular -q 'SELECT * FROM v_memory_health'"
    echo "  dolt sql -r tabular -q 'SELECT * FROM v_agent_contributions'"
    echo "  dolt sql -r tabular -q 'SELECT * FROM v_sga_distribution'"
    echo "  dolt sql -r tabular -q 'SELECT * FROM v_layer_distribution'"
    echo "  dolt sql -r tabular -q 'SELECT * FROM v_dream_history'"
    echo "  dolt sql -r tabular -q 'SELECT * FROM v_quarantine_status'"
    echo "  dolt sql -r tabular -q 'SELECT * FROM v_skip_link_network'"
}

cmd_query() {
    local view="${1:-v_memory_health}"
    cd "$DOLT_DB_DIR"
    dolt sql -r tabular -q "SELECT * FROM $view"
}

cmd_status() {
    cd "$DOLT_DB_DIR"
    info "Analytics Dashboard"
    echo ""

    echo "=== Memory Health ==="
    dolt sql -r tabular -q "SELECT * FROM v_memory_health" 2>/dev/null || echo "  (view not installed)"
    echo ""

    echo "=== Agent Contributions ==="
    dolt sql -r tabular -q "SELECT * FROM v_agent_contributions" 2>/dev/null || echo "  (view not installed)"
    echo ""

    echo "=== SGA Distribution (top 10) ==="
    dolt sql -r tabular -q "SELECT * FROM v_sga_distribution LIMIT 10" 2>/dev/null || echo "  (view not installed)"
    echo ""

    echo "=== Layer Distribution ==="
    dolt sql -r tabular -q "SELECT * FROM v_layer_distribution" 2>/dev/null || echo "  (view not installed)"
}

case "${1:-help}" in
    install) cmd_install ;;
    query)   cmd_query "${2:-v_memory_health}" ;;
    status)  cmd_status ;;
    help|--help|-h)
        echo "Usage: $0 {install|query <view>|status}"
        echo ""
        echo "Views:"
        echo "  v_memory_health       Amplitude distribution, decay rates"
        echo "  v_dream_history       Dream consolidation log"
        echo "  v_agent_contributions Memories per agent"
        echo "  v_sga_distribution    SGA class frequency"
        echo "  v_layer_distribution  Temporal depth analysis"
        echo "  v_quarantine_status   Dispute overview"
        echo "  v_skip_link_network   Connection density"
        ;;
    *) echo "Unknown: $1. Run '$0 help'"; exit 1 ;;
esac
