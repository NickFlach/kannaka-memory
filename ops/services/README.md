# Constellation services — how the install works

There are **two tiers**, and they answer "where do kannaka-attention and
kannaka-eye live in the install":

## Tier 1 — the kannaka binary (`install.sh` / GitHub Releases)

`install.sh` downloads the `kannaka` binary into `~/.local/bin`. That single
binary already contains:

- the HRM engine, `remember`/`recall`/`dream`/`hear`/`see`/`classify`
- **`kannaka attention serve`** — the sparse-attention beam. The
  **`kannaka-attention` crate is a path dependency compiled into the binary**
  (`Cargo.toml: kannaka-attention = { path = "../kannaka-attention" }`), so
  there is nothing separate to install — installing kannaka installs attention.
- the glyph-gravity recall path (the `glyph` feature is now default).

So: **kannaka-attention is already "in the install."** What's left is choosing
to *run* it as a daemon — that's a systemd unit, not another download.

## Tier 2 — constellation services (per-host daemons)

These are long-running roles a node opts into. Each is the SAME binary invoked
with a different subcommand, wired as a systemd unit:

| Service | Command | Unit |
|---|---|---|
| Attention beam | `kannaka attention serve` | `kannaka-attention.service` (here) |
| Remote recall | `kannaka swarm serve` | `kannaka-swarm-serve.service` |
| Substrate | `kannaka substrate run` | `kannaka-substrate.service` |
| Inbox | `kannaka inbox serve` | `kannaka-inbox.service` |

`kannaka-eye` is the exception: it's a **separate Node service** (not part of
the binary — it has a WebGL UI and its own glyph emitter). It's deployed from
its own repo (`NickFlach/kannaka-eye`, see that repo's `ops/`), and it's the
*producer* whose glyph events `attention serve` consumes. Eye → glyph →
`KANNAKA.attention.eye` → attention beam → instant recall.

## Install the attention daemon

```bash
# binary first (if not already):  curl -sSf https://install.ninja-portal.com/kannaka | sh
bash ops/services/install-services.sh          # installs + enables kannaka-attention
```

Or by hand:

```bash
sudo install -m644 ops/services/kannaka-attention.service /etc/systemd/system/
sudo systemctl daemon-reload && sudo systemctl enable --now kannaka-attention
```

Gravity is on at gain 0.5 via the unit's `KANNAKA_GLYPH_GRAVITY`; set it to 0 to
disable without code changes. On a build node, point `ExecStart` at the cargo
target binary instead of `~/.local/bin/kannaka`.
