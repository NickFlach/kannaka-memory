# Ratzilla Flux Monitor — Build Summary

**Date:** 2026-02-19
**Location:** `C:\Users\nickf\Source\flux\ui-ratzilla\`

## What was built
A Ratzilla (Ratatui + WASM) real-time Flux dashboard to replace the old `ui/` HTML/JS dashboard.

## Files created
- `Cargo.toml` — deps: ratzilla 0.3, serde, serde_json, gloo-net, gloo-timers, web-sys, futures-util, chrono, js-sys, wasm-bindgen
- `index.html` — trunk-compatible with Fira Code font, dark theme
- `src/main.rs` — ~600 lines, full dashboard implementation
- `README.md` — build/serve instructions

## Features
1. **Entity List Panel** — sorted by last_updated, staleness color coding (green <60s, yellow <5min, red >30min), status badges
2. **Entity Detail Panel** — shows all properties of selected entity
3. **Agent Messages Panel** — filters entities with `message`+`message_to` props, shows chat-like log
4. **Metrics Bar** — events/sec, total entities, active publishers, WS connections, total events
5. **Header** — Flux branding + live/disconnected connection indicator
6. **Keyboard nav** — ↑↓/jk to browse, Tab to switch panels

## Technical approach
- HTTP GET `/api/state/entities` on startup for initial state
- WebSocket `/api/ws` with wildcard subscribe for real-time updates
- `gloo-net` for WASM-compatible HTTP + WebSocket
- `gloo-timers` for periodic `now_ms` refresh (staleness calc)
- Auto-reconnect on WebSocket disconnect (3s delay)
- State in `Rc<RefCell<AppState>>` per Ratzilla pattern

## To build
```sh
cd ui-ratzilla
cargo install --locked trunk
rustup target add wasm32-unknown-unknown
trunk serve  # dev on :8080
trunk build --release  # prod → dist/
```

## Notes
- Old `ui/` directory preserved (not deleted)
- App derives API URLs from `window.location` — works behind proxy or direct
- May need trunk proxy config to point at Flux during dev
