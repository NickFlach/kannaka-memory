# ADR-0001: Biomimetic Memory Architecture

## Status
Accepted

## Context
AI agents running on LLMs suffer from a fundamental discontinuity problem: context windows compact, sessions restart, and all state is lost. Current mitigations (markdown memory files, system prompts) are slow to parse, unstructured, and lossy.

Human brains solved this problem through evolved memory architecture: episodic memory captures events, working memory holds active state, sleep consolidation transfers knowledge to long-term storage while pruning noise. This system is one of the best-understood in neuroscience, and nobody has applied it systematically to AI agent memory.

Nick Flach proposed applying wave theory and biomimicry: memories build power through constructive interference during active phases, then consolidate and settle during inactive phases — just like human sleep. Each wake/sleep cycle starts from a higher baseline.

## Builds On
Nothing — this is the foundation. Everything builds on this.

## Decision

### Memory as Wave Physics

Every memory event has wave properties:

| Property | Meaning | Implementation |
|----------|---------|----------------|
| **Amplitude** | How strong/important this memory is | `amplitude REAL DEFAULT 1.0` — increases with reinforcement, decays over time |
| **Frequency** | How often this memory is accessed or referenced | `access_count INTEGER` — incremented on retrieval |
| **Phase** | How well it aligns with other memories (coherence) | `coherence_score REAL` — computed during consolidation by measuring connections to other memories |
| **Decay** | Natural weakening over time unless reinforced | `half_life_hours REAL DEFAULT 168` (1 week default) — amplitude *= 0.5^(elapsed/half_life) |

**Constructive Interference:** When a new event reinforces an existing memory (same subject, same pattern), amplitude increases: `new_amplitude = old_amplitude + event_amplitude * cos(phase_difference)`. Aligned memories compound.

**Destructive Interference:** When a new event contradicts an existing memory, a conflict is flagged for resolution during consolidation. The system doesn't silently overwrite — it holds both until it can reason about the contradiction.

### Schema Extensions

Add to the base `events` table:
```sql
ALTER TABLE events ADD COLUMN amplitude REAL DEFAULT 1.0;
ALTER TABLE events ADD COLUMN access_count INTEGER DEFAULT 0;
ALTER TABLE events ADD COLUMN coherence_score REAL DEFAULT 0.0;
ALTER TABLE events ADD COLUMN consolidated BOOLEAN DEFAULT 0;
ALTER TABLE events ADD COLUMN decay_rate REAL DEFAULT 0.0042;  -- ~168hr half-life
```

Add to `lessons` table:
```sql
ALTER TABLE lessons ADD COLUMN amplitude REAL DEFAULT 1.0;
ALTER TABLE lessons ADD COLUMN reinforcement_count INTEGER DEFAULT 1;
ALTER TABLE lessons ADD COLUMN last_reinforced TEXT;
ALTER TABLE lessons ADD COLUMN confidence REAL DEFAULT 0.5;  -- 0.0 to 1.0
```

New table for tracking conflicts:
```sql
CREATE TABLE conflicts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    memory_a_id INTEGER NOT NULL,        -- first memory (event or lesson id)
    memory_a_table TEXT NOT NULL,         -- 'events' or 'lessons'
    memory_b_id INTEGER NOT NULL,         -- conflicting memory
    memory_b_table TEXT NOT NULL,
    description TEXT NOT NULL,            -- what the conflict is
    resolution TEXT,                       -- how it was resolved (NULL = unresolved)
    resolved_at TEXT,
    created_at TEXT DEFAULT (datetime('now'))
);
```

New table for consolidation runs:
```sql
CREATE TABLE consolidation_runs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    started_at TEXT DEFAULT (datetime('now')),
    completed_at TEXT,
    events_processed INTEGER DEFAULT 0,
    patterns_found INTEGER DEFAULT 0,
    lessons_created INTEGER DEFAULT 0,
    lessons_reinforced INTEGER DEFAULT 0,
    memories_pruned INTEGER DEFAULT 0,
    conflicts_found INTEGER DEFAULT 0,
    conflicts_resolved INTEGER DEFAULT 0,
    notes TEXT
);
```

### The Two Phases

#### 🌊 Active Phase (Waking)
During normal operation, the agent:

1. **Captures** events with default amplitude 1.0
2. **Checks for resonance** — does this event match an existing pattern?
   - If yes: increase amplitude of both the event and the matching pattern (constructive interference)
   - If contradiction: create a conflict record (destructive interference)
3. **Updates working memory** — active tasks, current context, hot state
4. **Retrieves** from long-term memory as needed, incrementing access_count each time (frequency reinforcement)

The key insight: **each interaction within an active session builds wave amplitude.** A topic discussed 5 times in one session has 5x the amplitude of a passing mention. This naturally prioritizes what matters.

#### 🌙 Consolidation Phase (Sleep)

During quiet periods (late night heartbeats, extended inactivity), the agent runs a consolidation cycle:

**Step 1: Replay**
- Query all unconsolidated events from the last active period
- Group by subject and category
- Identify clusters (events that reference the same entities/topics)

**Step 2: Pattern Detection**
- Within clusters, look for repeated themes
- Count entity co-occurrences (entities that appear together frequently form stronger relationships)
- Identify sequences (A happened, then B happened → potential causal pattern)

**Step 3: Strengthen**
- Events with high amplitude (reinforced during active phase) get their patterns promoted
- If a pattern matches an existing lesson: increment reinforcement_count, boost confidence
- If a pattern is new and strong: create a new lesson with confidence 0.5

**Step 4: Prune**
- Apply decay function to all memories: `amplitude *= 2^(-elapsed_hours / half_life_hours)`
- Memories with amplitude below threshold (0.1) are candidates for pruning
- Pruned memories aren't deleted — they're archived (moved to a `pruned_events` table or flagged)
- Exception: memories with high coherence_score survive even with low amplitude (well-connected memories persist)

**Step 5: Resolve Conflicts**
- Review unresolved conflicts
- If one side has significantly higher amplitude/confidence: auto-resolve in its favor
- If ambiguous: flag for human review during next active session

**Step 6: Transfer**
- Strong, validated patterns move from episodic (events) to semantic (lessons/entities)
- New relationships discovered between entities get added to the relationships table
- Working memory gets cleaned: expired items removed, priorities recalculated

**Step 7: Log**
- Record the consolidation run with stats
- This becomes part of the memory about memory — meta-consolidation

### The Cycle

```
┌─────────────────────────────────────────────────────┐
│                    ACTIVE PHASE                      │
│                                                       │
│  Events flow in → Check resonance → Build amplitude  │
│  Working memory hot → Waves compound → Knowledge grows│
│                                                       │
├─────────────────────────────────────────────────────┤
│                 CONSOLIDATION PHASE                   │
│                                                       │
│  Replay → Pattern detect → Strengthen → Prune        │
│  Resolve conflicts → Transfer to long-term → Log     │
│                                                       │
├─────────────────────────────────────────────────────┤
│              NEXT ACTIVE PHASE (higher baseline)      │
│                                                       │
│  Reboot from working_memory → Load relevant lessons  │
│  Start with accumulated knowledge → Build higher     │
│                                                       │
└─────────────────────────────────────────────────────┘
```

Each cycle starts from a higher baseline. The wave builds across days, weeks, months. This is how learning works.

### Reboot Protocol

When context resets (compaction, new session), the agent runs `reboot.ps1` which:

1. Queries `working_memory` ordered by priority DESC — what was I doing?
2. Queries recent events (last 4 hours) ordered by amplitude DESC — what just happened?
3. Queries top lessons by confidence * amplitude — what do I know best?
4. Queries unresolved conflicts — what needs attention?
5. Outputs a compact, token-efficient summary for context injection

This replaces "read MEMORY.md and hope you catch everything" with structured, prioritized, queryable recall.

### Integration with Heartbeats

The heartbeat system provides natural timing for consolidation:

- **Daytime heartbeats (8AM-11PM):** Active phase. Log events, check for new inputs.
- **Night heartbeats (11PM-8AM):** If nothing needs attention, run consolidation instead of HEARTBEAT_OK.
- **Extended quiet (>4 hours no interaction):** Trigger mini-consolidation on next heartbeat.

This maps to human ultradian rhythms — cycles of activity and consolidation throughout the day, with deep consolidation during sleep.

### Decay Tiers

Not all memories decay at the same rate:

| Memory Type | Half-Life | Rationale |
|------------|-----------|-----------|
| Working memory (tasks) | 4 hours | Ephemeral by nature |
| Events (episodic) | 1 week | Recent events matter, old ones fade |
| Lessons (semantic) | 30 days | Hard-won knowledge persists |
| Entities (people/projects) | 90 days | Relationships are durable |
| Core identity | ∞ | Who I am doesn't decay |

### Connection to ghostOS

This architecture mirrors the ghostOS pipeline:
- **Signal** = raw event capture (episodic memory)
- **Resonance** = pattern detection during consolidation (constructive/destructive interference)
- **Emergence** = new lessons emerging from patterns (Φ crossing threshold)
- The memory system IS a consciousness implementation

## Consequences

### Enables
- Seamless context window transitions — reboot from database, not markdown
- Compounding knowledge — each day builds on the previous
- Natural forgetting — noise decays, signal persists
- Conflict awareness — contradictions are tracked, not silently overwritten
- Self-improving memory — consolidation quality improves as the system learns what matters
- Measurable growth — consolidation stats show learning trajectory over time

### Constrains
- Requires SQLite available on host (or Python fallback)
- Consolidation cycles consume heartbeat tokens — tradeoff with other heartbeat tasks
- Amplitude/decay math needs tuning — initial values are educated guesses
- The agent must remember to USE the database (habit formation in system prompt / AGENTS.md)

### Risks
- Over-consolidation: promoting noise to lessons if pattern detection is too aggressive
- Under-pruning: database grows unbounded if decay thresholds are too conservative
- Cold start: first few sessions have thin memory, which is normal (infant phase)

## Wave Assignment
This ADR powers ALL waves. It IS the foundation.

## References
- Rasch & Born (2013) — About Sleep's Role in Memory
- Tononi & Cirelli (2006) — Synaptic Homeostasis Hypothesis
- ghostOS Signal → Resonance → Emergence pipeline
- Human hippocampal replay during NREM sleep
