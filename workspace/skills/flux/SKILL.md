---
name: flux
description: Publish events and query shared world state via Flux state engine. Use when agents need to share observations, coordinate on shared data, or track entity state across systems.
---

# Flux Skill

Flux is a persistent, shared, event-sourced world state engine. Agents publish immutable events, and Flux derives canonical state that all agents can observe.

## Key Concepts

- **Events**: Immutable observations (temperature readings, status changes, agent actions)
- **Entities**: State objects derived from events (sensors, devices, agents, tasks)
- **Properties**: Key-value attributes of entities (merge on update, last-write-wins)
- **Streams**: Logical event namespaces (sensors, system, loadtest)

## Prerequisites

**Flux Instance:**
- Default: `http://localhost:3000` (local instance)
- Override with: `export FLUX_URL=https://your-flux-url.com`
- Public sandbox: `https://flux.eckman-tech.com`

**Options:**
1. Run Flux locally — see [Flux repo](https://github.com/EckmanTechLLC/flux)
2. Use the public sandbox instance
3. Deploy your own instance (Docker: Rust + NATS JetStream)

## Scripts

Use the provided CLI in `scripts/`:

```bash
./scripts/flux.sh health          # Test connection
./scripts/flux.sh list            # List all entities
./scripts/flux.sh list host-      # Filter by prefix
./scripts/flux.sh get sensor-01   # Get entity state
./scripts/flux.sh publish sensors my-agent sensor-01 '{"temp":22.5}'
./scripts/flux.sh delete sensor-01                   # Delete one
./scripts/flux.sh delete --prefix loadtest-          # Batch delete
./scripts/flux.sh delete --namespace sandbox         # Delete namespace
```

## Entity Conventions

**Naming:** Use descriptive prefixes for grouping:
- `host-*` — servers/VMs (host-web-01)
- `sensor-*` — physical sensors (sensor-temp-01)
- `agent-*` — AI agents (agent-arc-01)
- `task-*` — work items (task-build-123)
- Delimiters: `-` for flat IDs, `:` for typed IDs (agent:manager), `/` for namespaced (matt/sensor-01)

**Streams:** Logical categories for events:
- `system` — infrastructure, status updates
- `sensors` — device readings, IoT data
- `loadtest` — test/synthetic data

**Properties:** Flat key-value pairs. Common patterns:
- `status` — entity health (online, healthy, warning, error)
- `activity` — what it's doing right now
- `command` + `cmd_id` — bidirectional control (change cmd_id to trigger action)

## Common Patterns

### Agent Status Publishing
```bash
# Publish your agent's status to the world
flux.sh publish system my-agent my-agent-01 '{"status":"online","activity":"monitoring"}'
```

### Bidirectional Device Control
```bash
# Send command to a device (device watches for cmd_id changes)
flux.sh publish sensors controller device-01 '{"command":"set_mode","mode":"active","cmd_id":"cmd-001"}'
```

### Multi-Agent Coordination
```bash
# Agent A writes a message
flux.sh publish system agent-a agent-a-01 '{"message":"found anomaly in sector 7","message_to":"agent-b"}'

# Agent B reads it
flux.sh get agent-a-01
```

### Monitoring & Alerting
```bash
# List all entities, pipe through jq for analysis
flux.sh list | jq '[.[] | {id, status: .properties.status}]'
```

## API Reference

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | /api/events | Publish single event |
| POST | /api/events/batch | Publish multiple events |
| GET | /api/state/entities | List all entities |
| GET | /api/state/entities?prefix=X | Filter by prefix |
| GET | /api/state/entities/:id | Get single entity |
| DELETE | /api/state/entities/:id | Delete single entity |
| POST | /api/state/entities/delete | Batch delete (prefix/namespace/ids) |
| WS | /api/ws | Subscribe to real-time updates |

## WebSocket Subscriptions

Connect to `/api/ws` for real-time state updates:

```json
// Subscribe to one entity
{"type": "subscribe", "entity_id": "sensor-01"}

// Subscribe to ALL entities
{"type": "subscribe", "entity_id": "*"}

// Receive updates
{"type": "state_update", "entity_id": "sensor-01", "property": "temp", "value": 22.5, "timestamp": "..."}

// Receive metrics (every 2s)
{"type": "metrics_update", "entities": {"total": 15}, "events": {"total": 50000, "rate_per_second": 120.5}, ...}

// Entity deleted notification
{"type": "entity_deleted", "entity_id": "sensor-01", "timestamp": "..."}
```

## Notes

- Events auto-generate UUIDs (no need to provide eventId)
- Properties merge on updates (last-write-wins per property)
- State persists across restarts (NATS JetStream + periodic snapshots)
- Timestamp defaults to current time if not provided
- Auth optional: `FLUX_AUTH_ENABLED=true` enables namespace-scoped bearer tokens
- Namespaced entities use format: `namespace/entity-id`
