# Flux API Reference

**Base URL:** Set via `FLUX_URL` environment variable (default: `http://localhost:3000`)

---

## Event Ingestion

### POST /api/events

Publish a single event to create or update an entity.

```json
{
  "stream": "sensors",
  "source": "my-agent",
  "timestamp": 1707900000000,
  "payload": {
    "entity_id": "temp-sensor-01",
    "properties": {
      "temperature": 22.5,
      "unit": "celsius"
    }
  }
}
```

**Response (200):**
```json
{
  "eventId": "019c5c88-5386-7ae0-ab4d-80a8c1ce631a",
  "stream": "sensors"
}
```

**Fields:**
- `stream` (required) — logical namespace
- `source` (required) — who published this event
- `timestamp` (optional) — Unix epoch milliseconds, defaults to now
- `payload.entity_id` (required) — target entity
- `payload.properties` (required) — key-value pairs to merge into entity state

### POST /api/events/batch

Publish multiple events atomically.

```json
{
  "events": [
    {"stream": "sensors", "source": "agent-01", "payload": {"entity_id": "s-01", "properties": {"temp": 22}}},
    {"stream": "sensors", "source": "agent-01", "payload": {"entity_id": "s-02", "properties": {"temp": 23}}}
  ]
}
```

---

## State Queries

### GET /api/state/entities

List all entities. Optional `?prefix=` filter.

```bash
curl http://localhost:3000/api/state/entities
curl http://localhost:3000/api/state/entities?prefix=host-
```

**Response:** Array of entity objects with `id`, `properties`, `lastUpdated`.

### GET /api/state/entities/:id

Get a single entity by ID.

**Response (200):**
```json
{
  "id": "temp-sensor-01",
  "properties": {"temperature": 22.5, "unit": "celsius"},
  "lastUpdated": "2026-02-14T12:00:00Z"
}
```

**Response (404):** Entity not found.

---

## Entity Deletion

### DELETE /api/state/entities/:id

Delete a single entity. Event-sourced (tombstone event persists deletion across restarts).

**Response (200):**
```json
{"entity_id": "temp-sensor-01", "eventId": "..."}
```

### POST /api/state/entities/delete

Batch delete by filter. Choose one filter:

```json
{"prefix": "loadtest-"}
{"namespace": "sandbox"}
{"entity_ids": ["id1", "id2", "id3"]}
```

**Response (200):**
```json
{"deleted": 3, "failed": 0, "errors": []}
```

**Limit:** Max 10,000 entities per batch (configurable).

---

## WebSocket

### WS /api/ws

Real-time subscriptions for state updates, metrics, and deletions.

**Subscribe:**
```json
{"type": "subscribe", "entity_id": "sensor-01"}
{"type": "subscribe", "entity_id": "*"}
```

**Receive state updates:**
```json
{"type": "state_update", "entity_id": "sensor-01", "property": "temp", "value": 22.5, "timestamp": "..."}
```

**Receive metrics (every 2s):**
```json
{"type": "metrics_update", "timestamp": "...", "entities": {"total": 15}, "events": {"total": 50000, "rate_per_second": 120.5}, "websocket": {"connections": 3}, "publishers": {"active": 5}}
```

**Entity deleted:**
```json
{"type": "entity_deleted", "entity_id": "sensor-01", "timestamp": "..."}
```

---

## Namespaces & Auth

When `FLUX_AUTH_ENABLED=true`:
- Entities use format: `namespace/entity-id`
- Bearer token required: `Authorization: Bearer <token>`
- Tokens are scoped to namespaces
- `GET /api/state/namespaces` lists available namespaces

When auth is disabled (default): open access, no tokens needed.
