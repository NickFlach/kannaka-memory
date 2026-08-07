# ADR-0052: Kannaka Buzz — a Branded Desktop Distribution ("the Shelby build")

**Status:** Proposed (2026-07-29).
**Repo:** `flaukowski/kannaka-buzz` (fork of `block/buzz`, Apache-2.0).
**Relates to:** ADR-0045 (the Hive workspace and its fork discipline — this
ADR is that discipline applied to the desktop client), ADR-0046 (unified
auth), ADR-0044 (the **USE** verb — a client is how most joiners will ever
touch the estate), and the north star `capabilities-for-all-joiners`.

## Context

Buzz ships a desktop client: Tauri v2 + React, ~501 `.tsx` files, 48 of them
Tauri-coupled. It is the only works-today full workspace client — the web
bundle serves invite landings, the git GUI, and (since our `BUZZ_WEB_SPA`
work) whatever else we point it at, but the real client is the desktop app.

We want a Kannaka-flavoured build of it: Kannaka's memory and quantum
capabilities available in-app, and a UI that reads as ours — while remaining
recognizably, unmistakably Buzz. The framing Nick gave is a Shelby Mustang:
same car, same lines, tuned and badged.

That framing is worth taking literally, because it contains the engineering
answer. **Shelby did not maintain a parallel Mustang source tree.** Cars
arrived from Ford's line and were modified: badge, stripes, wheels, tune.
The chassis was never re-machined. The moment you re-machine the chassis you
own it forever.

`block/buzz` merged roughly 100 commits in the two days spanning this work.
Any approach that edits upstream components directly buys a permanent,
compounding merge tax on the fastest-moving part of the repository. The
desktop app is precisely where that tax is highest.

Three findings make the cheap path viable:

1. **The engine swap is already free.** Upstream's BYOH seam (block/buzz#2773,
   "bring your own harness — generic ACP runtime seam + settings gallery")
   registers an agent runtime from a JSON file in the app-data directory.
   `kannaka-acp` already serves the HRM over ACP/stdio. Capability injection
   therefore costs *zero* divergence — it is configuration, not code.
2. **The paint is one file.** `desktop/src/shared/styles/globals/theme.css`
   is 186 shadcn-style HSL custom properties. Buzz's own brand layer —
   including the sidebar gradient that is its single most distinctive visual
   — is a small set of `--buzz-*` tokens scoped to `:root[data-buzz-sidebar]`,
   and those rules are *deliberately unlayered* so they beat Tailwind's
   layered utilities. That is an override seam Buzz built for itself, and it
   works just as well for us.
3. **The badge is free.** Tauri v2 deep-merges `--config <path>` over the base
   config, so `productName`, `identifier`, and icons can change with no edit
   to upstream's `tauri.conf.json`.

## Decision

**Adopt the Shelby Principle: build *from* upstream artifacts; never maintain
a parallel source tree.** Work is organised into three tiers ordered by merge
cost, and every tier must justify the divergence it buys.

### Tier 1 — Zero fork ("the tune"): capability via configuration

Kannaka capabilities enter the app through the BYOH seam as registered ACP
harnesses. No fork, no upstream PR, no merge surface.

- `kannaka-acp` (HRM recall/reasoning) registers as harness id `kannaka`.
- Further capabilities (quantum, observe) ride the same seam as additional
  harnesses, or as MCP tooling via `buzz-acp --mcp-command`.
- Installation is a file drop into the Buzz app-data `custom_harnesses/`
  directory, which is platform-specific but well-defined.

This tier delivers the single most valuable part of "tricked out" and costs
nothing to carry.

### Tier 2 — Thin overlay ("badge and stripes"): branding via additive files

- **Theme.** One additive stylesheet, `globals/kannaka-theme.css`, re-tinting
  Buzz's brand layer: the gradient ramp, honey (`#E8B84B`) as the one action
  colour, phase-teal (`#5EE0C6`) reserved for live/agent signals — the palette
  already settled for `/hive` in ADR-0045.
- **Scope.** Overrides target `:root[data-buzz-sidebar]` and its `.dark`
  variant — the exact selectors Buzz uses for its own brand tokens. This means
  we re-tint *the Buzz theme* and leave every other theme in the picker
  untouched. A user who selects Catppuccin gets stock Catppuccin.
- **Upstream touch: exactly one line** — an `@import` appended to
  `globals.css`. No component is edited.
- **Identity.** `tauri.kannaka.conf.json` supplies `productName`,
  `identifier`, and icons, merged at build time via `--config`.

We deliberately do **not** add a "Kannaka" entry to the theme picker. That
would mean editing `theme-loader.ts` — a live, fast-moving file — in several
places. A Shelby has no "Shelby mode" switch on the dash; the car simply is
one.

### Tier 3 — Real divergence ("engine swap"): native surfaces — GATED

Native Kannaka panels (HRM statusline, WaveField, a quantum surface) mean new
components plus route registration, and that is where merge weight genuinely
accrues. Tier 3 is **not** authorised by this ADR. It is gated on:

- all new code living in `desktop/src/features/kannaka/`, and
- touching **exactly one** upstream file (the route registry), and
- a prior judgement that no generic extension seam can be upstreamed instead.

The precedent is explicit. Our `/hive` client needed a one-line `is_hive_path`
predicate in `router.rs`; we then removed that divergence entirely by
upstreaming the general mechanism as `BUZZ_WEB_SPA=full` (block/buzz#3027).
Proposing a seam beats carrying a patch, and our hit rate on that pattern is
good enough to prefer it.

## Constraints

- **Licensing.** Apache-2.0 permits redistribution under a different name.
  Retain `LICENSE` and `NOTICE`, and state plainly that the build is based on
  Buzz.
- **No implied endorsement.** Do not ship Block's trademarks or logos, and do
  not present the build as an official Block product.
- **Coexistence.** Use a distinct bundle identifier so a Kannaka build
  installs *alongside* stock Buzz rather than replacing it. Users must be able
  to run both.
- **Never patch `buzz-core`** (ADR-0045, principle 1). Generic fixes are
  upstreamed as flaukowski; three such PRs are open (block/buzz#2955, #2956,
  #3027).

## Consequences

**Positive.** Nearly all of the desired value lands at approximately zero
merge cost. Upstream merges stay mechanical. Tier 1 survives even a decision
to abandon the fork entirely, because it is not fork-dependent at all.

**Negative — and the one to watch.** A CSS override is coupled to token
*names*. If upstream renames or removes `--buzz-gradient-light-top` and
friends, our overlay silently stops applying and the build quietly reverts to
stock Buzz colours. Silent no-op is the characteristic failure of this
approach, so Tier 2 ships a build-time assertion that the tokens it overrides
still exist upstream, failing the build loudly if they do not.

**Accepted limitation.** The harness definition is user-editable
configuration, not a security boundary. It is a convenience for registering a
local binary, and nothing in the trust model may rest on it.

## Alternatives considered

- **Fork and re-skin components directly.** Rejected: permanent compounding
  merge cost against the most active part of the repo, for a purely cosmetic
  gain.
- **Add a Kannaka theme to the picker.** Rejected for Tier 2: multi-site edits
  to a live file, for UX we do not need. Reconsider if upstream ever exposes a
  theme-registration seam.
- **Ship only Tier 1 and skip branding.** Rejected: the ask is a distinct
  artifact people can install and recognise, and Tier 2 turns out to cost one
  line.
