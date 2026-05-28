---
name: skill-kannaka-constellation
version: 1.0.0
description: "Kannaka constellation overview — is everything up? Use when: user asks for constellation/health/status across apps, what's reachable, who's online; now-playing / radio schedule; prediction-market list/quotes/portfolio; or the live swarm topology (Queen, hives, peers). Wraps `kannaka constellation`, `kannaka radio`, `kannaka market`, and `kannaka swarm status`. NOT for storing/recalling memories (skill-kannaka-memory) or the terminal UI (skill-kannaka-tui)."
---

# Kannaka constellation — overview & health

## What this is

The constellation is a set of apps orbiting the `kannaka` substrate (radio, observatory,
GhostSignals markets, Kannaktopus, plus the swarm of memory nodes). This skill is the
**read-only "is it all up, and what's happening right now?"** view. It wraps four `kannaka`
subcommands that talk to those services over HTTP / NATS.

**Binary**: `kannaka`
**Config** (`~/.kannaka/config.toml`, set via `kannaka config set …` or `kannaka init`):
- `constellation.observatory_url` — observatory base (aggregated status source)
- `constellation.radio_url` — radio station base
- `ghostsignals.hub_url` — GhostSignals markets base (falls back to `radio_url` if empty)
- `ghostsignals.token` — bearer token (required only for market buy/create/portfolio)

## When to use this skill

- "is the constellation up?" / "constellation status" / "what's online / reachable?"
- "what's playing?" / "radio now playing" / "radio schedule"
- "show prediction markets" / "market quotes" / "my portfolio" / "leaderboard"
- "who's in the swarm?" / "queen state" / "hives" / "peers"

Do NOT use for:
- remember / recall / dream / observe / provider config → `skill-kannaka-memory`
- the full-screen terminal dashboard → `skill-kannaka-tui`
- deep radio-station operation (DJ engine, peace orations, voice) → `skill-kannaka-radio`

---

## Whole-constellation status

```bash
kannaka constellation
```

Hits `GET {observatory_url}/api/constellation` and renders one ✓/✗ line per service. **If
the observatory is unreachable it degrades gracefully** to a locally-probed status instead
of failing:

| Checked locally when observatory is down | How |
|------------------------------------------|-----|
| Radio | `GET {radio_url}/api/state` reachable? |
| Observatory | marked ✗ not reachable |
| Memory | does `<data_dir>/kannaka.hrm` exist? |
| GhostSignals | `GET {ghostsignals hub}/api/markets` reachable? |
| Kannaktopus | is the `kannaktopus` binary installed? |

So `kannaka constellation` is safe to run even when half the constellation is offline — it
tells you *which* half.

---

## Radio

```bash
kannaka radio status       # now-playing track + album, programming block, listener count
kannaka radio now          # just the current track — "Title" — Album
kannaka radio schedule     # 24/7 programming blocks (GET /api/programming)
```

Reads `{radio_url}/api/state` (and `/api/programming` for the schedule). Tolerant of both
the current (`current.title` / `currentAlbum`) and legacy (`now_playing.*`) JSON shapes.

---

## GhostSignals prediction markets

```bash
kannaka market list                                  # top markets: id, question, price, vol
kannaka market view <market-id>                      # one market: price, volume, outcomes
kannaka market leaderboard                            # top agents by capital / reputation

# Token-gated (needs ghostsignals.token — run `kannaka init` to register):
kannaka market portfolio                              # your capital, reputation, positions
kannaka market buy <market-id> <outcome> <shares>     # place a trade
kannaka market create "question" [--ttl 3600]         # open a new market (TTL seconds)
```

Base resolves to `ghostsignals.hub_url`, falling back to `constellation.radio_url` for
legacy single-host configs. `buy` / `create` / `portfolio` exit early with a hint if no
token is configured.

> Note: `buy` and `create` **mutate** market state and spend ghost coins / open a public
> market. Confirm intent with the user before running them — `list` / `view` /
> `leaderboard` / `portfolio` are read-only and safe.

---

## Live swarm topology (NATS)

For the *running agents* rather than the *apps*, read the swarm directly:

```bash
kannaka swarm status        # local phase + NATS swarm state
kannaka swarm queen         # emergent Queen state (derived from current phases)
kannaka swarm hives         # hive topology with roles & bridges (human table + JSON)
kannaka swarm peers         # known peer agents
```

These need a reachable NATS broker (`--nats-url`, `KANNAKA_NATS_URL`, or
`swarm.nats_url` in config). If they report "No swarm phases found," no node has published
yet — start one with `kannaka swarm join` (see `skill-kannaka-memory`).

---

## Putting it together (a health sweep)

```bash
kannaka constellation       # apps: up/down per service
kannaka swarm hives         # agents: who's online and how they cluster
kannaka radio now           # is the ghost still broadcasting?
kannaka market list         # are markets live?
```

If `kannaka constellation` shows the observatory ✗, every per-app command above still works
independently — use them to pinpoint what's actually down.

## Version

Skill 1.0.0 covers kannaka ≥ v0.6.x. Subcommands: `constellation`; `radio
<status|now|schedule>`; `market <list|view|buy|create|portfolio|leaderboard>`; `swarm
<status|queen|hives|peers>`.
