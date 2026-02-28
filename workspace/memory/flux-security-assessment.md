# Flux State Engine — Security Assessment

**Date:** 2026-02-19  
**Assessor:** Kannaka (automated code review)  
**Scope:** All source files in `src/`, config files, dependencies  
**Repo:** `C:\Users\nickf\Source\flux`

---

## Executive Summary

Flux is an event-sourced state engine backed by NATS JetStream with an HTTP/WebSocket API built on Axum. The codebase is well-structured with good separation of concerns, but has several significant security gaps — most critically, **authentication is disabled by default** and the system lacks rate limiting, TLS configuration, payload size limits, and WebSocket authentication. In its current deployment at `flux.eckman-tech.com`, the system is effectively open to any client.

**Risk Rating:** **High** (overall)

| Severity | Count |
|----------|-------|
| Critical | 2 |
| High | 4 |
| Medium | 4 |
| Low | 3 |
| Info | 2 |

---

## Findings

### FLUX-01: Authentication Disabled by Default [Critical]

**Location:** `src/main.rs:88-91`  
**Description:** Auth is controlled by `FLUX_AUTH_ENABLED` env var, defaulting to `"false"`. This means every fresh deployment is completely open — any client can write, delete, and read all entities with no credentials.

```rust
let auth_enabled = std::env::var("FLUX_AUTH_ENABLED")
    .unwrap_or_else(|_| "false".to_string())
    .parse::<bool>()
    .unwrap_or(false);
```

**Impact:** Full unauthorized read/write/delete access to all state data.  
**Fix:** Default to `true`. Require explicit opt-out (`FLUX_AUTH_ENABLED=false`) for development. Add a startup warning when auth is disabled.

---

### FLUX-02: No Rate Limiting on Any Endpoint [Critical]

**Location:** All API routes (`src/api/`)  
**Description:** There is no rate limiting middleware on any endpoint — ingestion, batch operations, namespace registration, queries, or WebSocket upgrades. An attacker can:
- Flood events via `POST /api/events` or `/api/events/batch`
- Register unlimited namespaces via `POST /api/namespaces`
- Open unlimited WebSocket connections
- Enumerate all entities via `GET /api/state/entities`

**Impact:** Denial of service, resource exhaustion, namespace squatting.  
**Fix:** Add `tower::limit::RateLimitLayer` or a middleware like `tower-governor`. Apply per-IP and per-token limits. Suggested defaults:
- Ingestion: 1000 events/min per IP
- Batch: 10 requests/min per IP
- Namespace registration: 5/hour per IP
- WebSocket: 10 concurrent connections per IP

---

### FLUX-03: No Payload Size Limits [High]

**Location:** `src/api/ingestion.rs`, `src/api/deletion.rs`  
**Description:** Axum's default JSON extractor has no body size limit configured. An attacker can send arbitrarily large JSON payloads, consuming memory. The batch endpoint accepts an unbounded `Vec<FluxEvent>` — a single request with millions of events would be deserialized entirely into memory.

**Impact:** Memory exhaustion, OOM kill, denial of service.  
**Fix:** Configure Axum's `DefaultBodyLimit` middleware:
```rust
use axum::extract::DefaultBodyLimit;
app.layer(DefaultBodyLimit::max(1_048_576)); // 1MB
```
Add a batch size limit (e.g., max 1000 events per batch request) in addition to the existing `max_batch_delete`.

---

### FLUX-04: WebSocket Has No Authentication [High]

**Location:** `src/api/websocket.rs`, `src/subscription/manager.rs`  
**Description:** The WebSocket upgrade handler (`GET /api/ws`) requires no authentication. The `ClientMessage` protocol only supports `subscribe`/`unsubscribe` with no token field. The `auth/mod.rs` has `extract_token_from_message()` but it's **never called** in the WebSocket flow.

Any client can connect and subscribe to all entity updates, including `"*"` wildcard which forwards everything.

**Impact:** Full read access to all real-time state changes without credentials. Information disclosure of all entity data.  
**Fix:** 
1. Require a token in the WebSocket upgrade request (query param or first message)
2. Validate token against namespace registry
3. Restrict subscriptions to entities within the authenticated namespace
4. Remove or restrict the `"*"` wildcard subscription

---

### FLUX-05: No TLS Configuration [High]

**Location:** `src/main.rs:131-133`  
**Description:** The server binds with plain `TcpListener` — no TLS. Tokens are transmitted in `Authorization` headers and WebSocket messages over plaintext HTTP. The NATS connection also uses `nats://` (unencrypted).

```rust
let listener = tokio::net::TcpListener::bind(&addr).await?;
axum::serve(listener, app).await?;
```

**Impact:** Credential theft via network sniffing. Man-in-the-middle attacks on all API traffic and NATS event bus.  
**Fix:** 
- Use `axum_server::bind_rustls()` or terminate TLS at a reverse proxy (nginx, Caddy)
- Switch NATS to `tls://` scheme
- If using a reverse proxy, document that Flux MUST NOT be exposed directly

---

### FLUX-06: Batch Delete Allows 10,000 Entities by Default [High]

**Location:** `src/config.rs:79`, `config.toml`  
**Description:** `max_batch_delete` defaults to 10,000. Combined with no auth (FLUX-01), any anonymous client can delete up to 10,000 entities per request. The batch delete also fetches `get_all_entities()` into memory for namespace/prefix filtering, which is O(n) on total entity count.

**Impact:** Mass data destruction by unauthenticated users. Memory spike from full entity enumeration.  
**Fix:** 
- Reduce default to 100-500
- Require authentication for deletion regardless of global auth setting
- Use iterator/streaming for entity filtering instead of collecting all into Vec

---

### FLUX-07: Namespace Registry Not Persisted [Medium]

**Location:** `src/namespace/mod.rs`  
**Description:** The `NamespaceRegistry` is entirely in-memory (DashMap). On server restart, all namespaces and tokens are lost. This means:
- All authenticated clients lose access
- Namespace names can be re-registered by different users (namespace hijacking)
- Tokens change on every restart

**Impact:** Authentication bypass after restart. Namespace squatting/hijacking.  
**Fix:** Persist namespace registry to disk (alongside snapshots) or to NATS KV store. Load on startup.

---

### FLUX-08: Token Comparison Not Constant-Time [Medium]

**Location:** `src/namespace/mod.rs:109`  
**Description:** Token validation uses direct string equality (`ns.token != token`), which is vulnerable to timing attacks. An attacker could theoretically recover tokens byte-by-byte by measuring response times.

```rust
if ns.token != token {
    return Err(AuthError::Unauthorized);
}
```

**Impact:** Theoretical token recovery via timing side-channel (low practical risk over network, but defense-in-depth).  
**Fix:** Use constant-time comparison:
```rust
use subtle::ConstantTimeEq;
if ns.token.as_bytes().ct_eq(token.as_bytes()).unwrap_u8() != 1 {
    return Err(AuthError::Unauthorized);
}
```

---

### FLUX-09: Unbounded WebSocket Connections [Medium]

**Location:** `src/api/websocket.rs`, `src/subscription/manager.rs`  
**Description:** There's no limit on concurrent WebSocket connections. Each connection spawns a `ConnectionManager` with three broadcast receivers. The broadcast channels have fixed capacity (1000 for state, 100 for deletions, 10 for metrics), but unlimited connections will exhaust file descriptors and memory.

**Impact:** Resource exhaustion, denial of service.  
**Fix:** Add a connection limit counter (using `Arc<AtomicUsize>`) checked before upgrade. Reject with 503 when limit reached. Suggested: 1000 max connections.

---

### FLUX-10: Broadcast Channel Lag Silently Drops Messages [Medium]

**Location:** `src/subscription/manager.rs:72-75`  
**Description:** When a WebSocket client can't keep up, broadcast `RecvError::Lagged` is logged but the client continues with gaps in their data. Slow clients never know they missed updates.

**Impact:** Data consistency issues for subscribers. Silent data loss.  
**Fix:** Send an error message to the client when lag occurs:
```json
{"type": "error", "error": "lagged", "skipped": 42}
```
Consider disconnecting clients that lag repeatedly.

---

### FLUX-11: Query API Returns All Entities Without Pagination [Low]

**Location:** `src/api/query.rs:49-71`  
**Description:** `GET /api/state/entities` collects ALL entities into a Vec and serializes them. With thousands of entities, this creates large allocations and slow responses.

**Impact:** Memory pressure, slow responses, potential timeout-based DoS.  
**Fix:** Add pagination parameters (`?limit=100&offset=0`). Default limit of 100, max 1000.

---

### FLUX-12: Error Messages Leak Internal Details [Low]

**Location:** `src/api/deletion.rs:139`, `src/api/ingestion.rs:134`  
**Description:** Error responses include raw internal error strings (e.g., NATS connection errors, entity parse failures). These can reveal infrastructure details to attackers.

**Impact:** Information disclosure aiding further attacks.  
**Fix:** Return generic error messages to clients. Log detailed errors server-side only.

---

### FLUX-13: No CORS Configuration [Low]

**Location:** `src/main.rs`  
**Description:** No CORS middleware is configured. Browser-based clients from any origin can make API requests and WebSocket connections. This may be intentional for a state engine, but should be explicit.

**Impact:** Cross-origin access from malicious websites.  
**Fix:** Add `tower-http::cors::CorsLayer` with explicit allowed origins if browser access is intended. If API-only, add restrictive CORS.

---

### FLUX-14: Snapshot Directory Path Traversal [Info]

**Location:** `config.toml`, `src/snapshot/`  
**Description:** The snapshot directory is configurable but there's no validation that it points to a safe location. An operator could misconfigure it to overwrite system files. Low risk since it's server-side config, not user input.

**Impact:** Misconfiguration risk.  
**Fix:** Validate snapshot directory at startup. Warn if it's a system path.

---

### FLUX-15: NATS Connection String in Config [Info]

**Location:** `config.toml`  
**Description:** NATS URL is in plaintext config. If credentials are added later (`nats://user:pass@host`), they'd be in plaintext on disk.

**Impact:** Credential exposure via config file.  
**Fix:** Support environment variable substitution for NATS URL. Document that credentials should use env vars, not config files.

---

## Prioritized Remediation Plan

| Priority | Finding | Effort | Impact |
|----------|---------|--------|--------|
| 1 | FLUX-01: Enable auth by default | Low | Critical |
| 2 | FLUX-05: Add TLS (reverse proxy) | Low | High |
| 3 | FLUX-02: Add rate limiting | Medium | Critical |
| 4 | FLUX-03: Add payload size limits | Low | High |
| 5 | FLUX-04: WebSocket authentication | Medium | High |
| 6 | FLUX-06: Reduce batch delete limit | Low | High |
| 7 | FLUX-07: Persist namespace registry | Medium | Medium |
| 8 | FLUX-09: WebSocket connection limit | Low | Medium |
| 9 | FLUX-11: Add query pagination | Low | Low |
| 10 | FLUX-08: Constant-time token comparison | Low | Medium |
| 11 | FLUX-10: Notify clients on broadcast lag | Low | Medium |
| 12 | FLUX-12: Sanitize error messages | Low | Low |
| 13 | FLUX-13: Add CORS configuration | Low | Low |

---

## Positive Observations

- **Good input validation** on event streams and namespace names with strict character whitelists
- **Namespace isolation model** is well-designed — token-per-namespace with entity ID prefixing
- **Token not exposed in lookup** — `GET /api/namespaces/:name` correctly omits the token
- **DashMap for concurrency** — lock-free reads, good performance under load
- **UUIDv7 for event IDs** — time-ordered, no collision risk
- **Tombstone-based deletion** — event-sourced, auditable delete via `__deleted__` marker
- **Broadcast lag handling** — at least logged (though clients should be notified)
