# 0xSCADA Flux Integration Scaffold

**Date:** 2026-02-19  
**Issue:** #260  
**Status:** Scaffolded, awaiting ADR-0015 from Arc and Nick's review

## Files Created

All under `server/services/flux/`:

| File | Purpose |
|------|---------|
| `index.ts` | Module entry point, re-exports |
| `types.ts` | Config, Flux wire types, mapping types, pending update types |
| `flux-publisher.ts` | Core publisher — batches updates, read-merge-write, retry on failure |
| `entity-mapper.ts` | Maps 0xSCADA sites/assets/historian readings → Flux entity IDs & properties |
| `__tests__/flux-publisher.test.ts` | Unit tests for batching, retry, auth header |

## Design Decisions Made

- **Entity naming:** `scada/site/{siteId}`, `scada/asset/{siteId}/{assetId}`
- **Batching:** In-memory Map accumulates updates per entity, flushes on interval (default 5s) or when batch size exceeds 200
- **Read-merge-write:** Before writing telemetry, reads existing entity from Flux to preserve command properties set by other agents
- **Retry:** Failed batches are re-queued for next cycle
- **Auth:** Optional Bearer token via `FLUX_AUTH_TOKEN` env var
- **Historian readings:** Flattened as `tag/{tagId}` properties on the asset entity

## TODOs Blocked on ADR-0015

1. Confirm Flux batch mutation API endpoint and wire format
2. Entity lifecycle — who creates/deletes entities?
3. WebSocket vs HTTP for publish transport
4. Event sourcing vs current-state publishing
5. Conflict resolution for concurrent writers
6. Bulk-read endpoint for efficient read-merge-write
7. Tag data: flatten onto asset entity vs separate entities per tag
8. Alarm state: property on asset or separate entity?
9. Import Flux SDK types if a shared package exists

## Codebase Context

- 0xSCADA uses Drizzle ORM with PostgreSQL (`shared/schema.ts`)
- Existing entity types: sites, assets, historian_data, alarms, event_anchors
- Asset types: TRANSFORMER, BREAKER, MCC, FEEDER, INVERTER, PLC, SENSOR, PUMP, VALVE
- WebSocket event streaming already exists (`server/websocket/`) — Flux publisher could subscribe to the same event pipeline
- ADR pattern: decisions stored in `docs/decisions/ADR-NNNN-*.md`
