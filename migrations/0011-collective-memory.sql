-- ADR-0011: Collective Memory Architecture
-- Extends the memories table and adds three new tables for multi-agent coordination.
-- Safe to run on an existing Dolt database: all ALTER TABLE use ADD COLUMN IF NOT EXISTS
-- (Dolt supports MySQL syntax; standard MySQL does not have IF NOT EXISTS on ADD COLUMN,
--  so we use a stored procedure guard below for portability).

-- ---------------------------------------------------------------------------
-- Extend memories table
-- ---------------------------------------------------------------------------

ALTER TABLE memories
    ADD COLUMN origin_agent    VARCHAR(64)  NOT NULL DEFAULT 'local',
    ADD COLUMN sync_version    BIGINT       NOT NULL DEFAULT 0,
    ADD COLUMN merge_history   JSON                  DEFAULT '[]',
    ADD COLUMN last_consolidated_at DATETIME(6)      DEFAULT NULL,
    ADD COLUMN disputed        BOOLEAN      NOT NULL DEFAULT FALSE,
    ADD COLUMN glyph_content   JSON                  DEFAULT NULL;

CREATE INDEX IF NOT EXISTS idx_origin_agent ON memories (origin_agent);
CREATE INDEX IF NOT EXISTS idx_disputed     ON memories (disputed);

-- ---------------------------------------------------------------------------
-- sync_events — cross-agent event log
-- ---------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS sync_events (
    id          VARCHAR(36)  NOT NULL PRIMARY KEY,
    event_type  VARCHAR(32)  NOT NULL,   -- memory.stored, dream.completed, merge.proposed, etc.
    agent_id    VARCHAR(64)  NOT NULL,
    memory_id   VARCHAR(36)  DEFAULT NULL,
    metadata    JSON         DEFAULT NULL,
    created_at  DATETIME(6)  NOT NULL,
    synced_at   DATETIME(6)  DEFAULT NULL,
    INDEX idx_agent_time (agent_id, created_at),
    INDEX idx_memory    (memory_id),
    INDEX idx_event_type (event_type)
);

-- ---------------------------------------------------------------------------
-- agents — registry of known agents
-- ---------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS agents (
    agent_id        VARCHAR(64)  NOT NULL PRIMARY KEY,
    display_name    VARCHAR(128) DEFAULT NULL,
    trust_score     FLOAT        NOT NULL DEFAULT 0.5,
    last_sync       DATETIME(6)  DEFAULT NULL,
    branch_name     VARCHAR(128) DEFAULT NULL,   -- e.g. "kannaka/working"
    flux_entity     VARCHAR(64)  DEFAULT NULL,   -- Flux entity id for this agent
    embedding_model VARCHAR(64)  DEFAULT NULL,   -- e.g. "all-minilm" — must match for cosine sim
    capabilities    JSON         DEFAULT NULL,
    created_at      DATETIME(6)  NOT NULL
);

-- ---------------------------------------------------------------------------
-- quarantine — disputed memories pending review
-- ---------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS quarantine (
    id            VARCHAR(36)  NOT NULL PRIMARY KEY,
    memory_id_a   VARCHAR(36)  NOT NULL,
    memory_id_b   VARCHAR(36)  NOT NULL,
    agent_a       VARCHAR(64)  NOT NULL,
    agent_b       VARCHAR(64)  NOT NULL,
    similarity    FLOAT        NOT NULL,
    phase_diff    FLOAT        NOT NULL,
    dispute_count INT          NOT NULL DEFAULT 1,
    status        VARCHAR(16)  NOT NULL DEFAULT 'pending',  -- pending | resolved | escalated
    resolution    JSON         DEFAULT NULL,
    created_at    DATETIME(6)  NOT NULL,
    resolved_at   DATETIME(6)  DEFAULT NULL,
    FOREIGN KEY (memory_id_a) REFERENCES memories(id),
    FOREIGN KEY (memory_id_b) REFERENCES memories(id),
    INDEX idx_status     (status),
    INDEX idx_memory_a   (memory_id_a),
    INDEX idx_memory_b   (memory_id_b)
);
