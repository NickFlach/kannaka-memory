# Kannaka Memory Database System 👻

A persistent SQLite memory system for OpenClaw agent continuity across context window compactions.

## Overview

This system replaces slow, unstructured markdown memory files with a fast, queryable database that acts as working memory. When context resets, agents can instantly reboot into their previous state.

## Database Location

- **Database:** `C:\Users\nickf\.openclaw\workspace\memory\kannaka.db`  
- **Helper Scripts:** `C:\Users\nickf\.openclaw\workspace\memory\db\`

## Schema Design

### Table: `working_memory`
The HOT CACHE - what's actively happening. Query this first on reboot.

```sql
CREATE TABLE working_memory (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    key TEXT UNIQUE NOT NULL,           -- e.g. "active_task", "current_project"
    value TEXT NOT NULL,                 -- JSON or plain text
    category TEXT DEFAULT 'general',     -- task, context, state, decision  
    priority INTEGER DEFAULT 0,          -- higher = more important to reload
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now')),
    expires_at TEXT                       -- optional TTL for ephemeral state
);
```

### Table: `events`
Structured event log - searchable facts, not prose.

```sql  
CREATE TABLE events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT DEFAULT (datetime('now')),
    category TEXT NOT NULL,              -- music, code, conversation, decision, task
    event_type TEXT NOT NULL,            -- "track_assigned", "pr_reviewed", "task_completed"
    subject TEXT,                         -- what/who this is about
    detail TEXT NOT NULL,                 -- the actual information (JSON or text)
    session_id TEXT,                      -- which session this happened in
    source TEXT                           -- where this info came from
);
```

### Table: `entities`
Knowledge graph nodes - people, projects, repos, tracks, concepts.

```sql
CREATE TABLE entities (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT UNIQUE NOT NULL,           -- canonical name
    type TEXT NOT NULL,                   -- person, project, repo, track, concept, tool
    attributes TEXT,                      -- JSON blob of properties
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);
```

### Table: `relationships`
Edges between entities.

```sql
CREATE TABLE relationships (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    from_entity TEXT NOT NULL,            -- entity name
    relation TEXT NOT NULL,               -- "created_by", "friend_of", "part_of", "depends_on"
    to_entity TEXT NOT NULL,              -- entity name
    context TEXT,                          -- why/how
    created_at TEXT DEFAULT (datetime('now'))
);
```

### Table: `lessons`
Things learned - mistakes, insights, preferences.

```sql
CREATE TABLE lessons (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    lesson TEXT NOT NULL,
    category TEXT,                         -- technical, social, process, creative
    learned_from TEXT,                     -- what event/situation taught this
    created_at TEXT DEFAULT (datetime('now'))
);
```

## Helper Scripts

All scripts use Python since sqlite3 CLI isn't available on this system.

### `query.ps1` - General SQL Query
```powershell
.\query.ps1 "SELECT * FROM working_memory ORDER BY priority DESC;"
```

### `reboot.ps1` - Context Reboot Summary  
Outputs compact summary for injection into fresh contexts:
```powershell
.\reboot.ps1
```

This queries:
- Working memory (by priority)
- Recent events (last 2 hours)  
- Key entities
- Recent lessons

### `set-working.ps1` - Set Working Memory
```powershell
.\set-working.ps1 -key "active_task" -value "Building memory system" -category "task" -priority 10
.\set-working.ps1 -key "temp_state" -value "debugging" -expiresAt "2026-02-16 00:00:00"
```

### `log-event.ps1` - Log Event
```powershell
.\log-event.ps1 -category "music" -type "track_assigned" -subject "Album2" -detail "Assigned 13 tracks to Resonance Patterns"
.\log-event.ps1 -category "code" -type "pr_merged" -detail "0xSCADA dependabot PR merged successfully"
```

### `get-entity.ps1` - Query Entity + Relationships
```powershell
.\get-entity.ps1 "Nick"
.\get-entity.ps1 "open-resonance-collective"
```

## Usage Patterns

### On Context Reboot
1. Run `.\reboot.ps1` 
2. Inject output into new context
3. Agent instantly knows current state

### During Operation
```powershell
# Update current task
.\set-working.ps1 -key "active_task" -value "Debugging auth flow" -priority 8

# Log important events  
.\log-event.ps1 -category "code" -type "bug_found" -subject "auth_service" -detail "JWT expiry not handled properly"

# Record lessons learned
.\query.ps1 "INSERT INTO lessons (lesson, category) VALUES ('Always check JWT expiry handling', 'technical');"

# Check what we know about someone
.\get-entity.ps1 "Corey Stevens"
```

### Maintenance Queries
```powershell
# Clean up expired working memory
.\query.ps1 "DELETE FROM working_memory WHERE expires_at < datetime('now');"

# Find all music-related events this week
.\query.ps1 "SELECT * FROM events WHERE category='music' AND timestamp > date('now', '-7 days');"

# Get relationship graph for a project
.\query.ps1 "SELECT from_entity, relation, to_entity FROM relationships WHERE from_entity='open-resonance-collective' OR to_entity='open-resonance-collective';"
```

## Current Data

The database is seeded with existing knowledge:

**Entities:** Nick, Kannaka, Corey Stevens, Kilted Weirdo, various projects (0xSCADA, ghostOS, etc.), albums (Ghost Signals, Resonance Patterns), concepts

**Working Memory:**
- `active_task`: Open Resonance Collective repo setup
- `current_project`: open-resonance-collective  
- `music_wave`: Wave 2 - Resonance Patterns track list drafted
- `waiting_on`: Nick to listen to Resonance Patterns candidates

**Recent Lessons:**
- Always verify sub-agent work before reporting done
- Check what's already on main before creating PRs
- Git workflow and Windows file system tips

## Design Principles

- **Fast reboot:** Critical state in `working_memory` table
- **Structured events:** Facts not prose, easily queryable
- **Knowledge graph:** Entities + relationships for context
- **Lessons learned:** Avoid repeating mistakes
- **Token efficient:** Compact summaries for context injection

## Extending the System

Add new entity types, relationship types, or working memory categories as needed. The schema is flexible and can grow with the agent's knowledge.

Consider adding indexes for new query patterns that become common.

## Troubleshooting

If scripts fail, check:
1. Database file exists and is readable
2. Python is available and working
3. SQL syntax is correct (single quotes must be escaped)

For manual database access:
```python
python -c "import sqlite3; conn = sqlite3.connect('C:/Users/nickf/.openclaw/workspace/memory/kannaka.db'); cursor = conn.cursor(); # your queries here"
```