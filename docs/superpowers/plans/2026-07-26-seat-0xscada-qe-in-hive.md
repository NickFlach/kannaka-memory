# Seat the `0xscada-qe` Organ in the Hive — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Take the delivered `0xscada-qe` Nostr keypair from a plaintext file in Downloads to a properly-custodied key whose organ is authenticated, room-visible, and published as a recognizable **agent** on the Hive relay.

**Architecture:** No repository code changes. A throwaway operator script in the scratchpad holds a NIP-42-authenticated WebSocket to `wss://buzz.ninja-portal.com`, enumerates the rooms the key can see, and publishes two replaceable identity events (kind 0 profile, kind 10100 agent profile). The durable artifacts are those two signed events living on the relay; the script is a wrench and is never committed.

**Tech Stack:** Node 24.14.0 (native global `WebSocket`, ESM), `nostr-tools` (bech32 via nip19, BIP-340 Schnorr signing, event id/signature verification), PowerShell + `icacls` for Windows key custody.

**Spec:** `docs/superpowers/specs/2026-07-26-seat-0xscada-qe-in-hive-design.md`

## Global Constraints

- **No repository code changes.** Tasks 1-4 touch only `~/.secrets/`, `~/Downloads/`, and the scratchpad. Task 5 is the only commit, and it is documentation.
- **Scratchpad root:** `C:\Users\nflach\AppData\Local\Temp\claude\C--Windows-System32\802aa5fe-9d9a-4ca7-bfc3-bd243c7eaecd\scratchpad`. Referred to below as `<SCRATCH>`. Everything the scripts create lives under `<SCRATCH>\hive\`.
- **Every task runs in a fresh shell.** Tasks may be executed by separate subagents, so no shell variable set in one task survives into the next. Each task that needs the working directory re-establishes it with the full literal path:
  ```powershell
  cd "C:\Users\nflach\AppData\Local\Temp\claude\C--Windows-System32\802aa5fe-9d9a-4ca7-bfc3-bd243c7eaecd\scratchpad\hive"
  ```
- **Do not edit these files with a PowerShell `Get-Content`/`Set-Content` round-trip.** Windows PowerShell 5.1 reads UTF-8 as ANSI and will corrupt every non-ASCII character in the document. Use the Edit tool.
- **Never log the nsec or the raw secret-key bytes.** Print `npub`, hex `pubkey`, and event ids - all public values. This mirrors `src/bin/handlers/nostr.rs`, which prints an nsec exactly once at mint and never persists it.
- **Self-verify every event before sending.** `verifyEvent(signed)` must return `true` or the script aborts without sending, matching the existing rule in `handlers/nostr.rs`: "never emit an event we can't verify."
- **Relay URL, exactly:** `wss://buzz.ninja-portal.com`. The NIP-11 document at the `https://` form advertises `supported_nips` including 42 and 29, `h_grammar: "uuid-v4-lowercase"`, `max_content_len: 65536`.
- **Key file, exactly:** `C:\Users\nflach\.secrets\kannaka-hive-0xscada-qe-nostr.json`, keeping its delivered field names `nsec` / `npub` / `pubkey` / `organ`. Do **not** reshape it into the bridge's `{privkey, pubkey}` form - that convention is chosen once, across all organs, in the deferred identity-layer spec.
- **Fail loudly, never silently.** A refused AUTH, a zero-room survey, or a missing read-back is a reported result, not something to retry differently, downgrade, or work around. Two of the failure modes are other people's actions and the correct response is to stop and surface them.
- **Never publish before reading.** Kinds 0 and 10100 are replaceable - newest wins. Task 3 must report what already exists for this pubkey before Task 4 overwrites anything.

---

## File Structure

**`C:\Users\nflach\.secrets\`**

| File | Responsibility |
|---|---|
| `kannaka-hive-0xscada-qe-nostr.json` | The organ's Hive keypair, ACL'd to the owning user. Moved from Downloads. |

**`<SCRATCH>\hive\`** *(throwaway; never committed)*

| File | Responsibility |
|---|---|
| `package.json` | Pins `nostr-tools`; marks the directory ESM |
| `lib.mjs` | Shared plumbing: key load, connect, NIP-42 auth, REQ-until-EOSE, publish-and-await-OK |
| `01-auth.mjs` | State 2 proof: does the relay accept this key? |
| `02-survey.mjs` | State 3 proof + pre-publish reconnaissance: rooms, an existing 10100 to copy the shape from, and our own current identity events |
| `03-publish.mjs` | State 4: publish kind 0 + kind 10100, then read both back and verify |

**`NickFlach/kannaka-memory`**

| File | Responsibility |
|---|---|
| `docs/superpowers/specs/2026-07-26-seat-0xscada-qe-in-hive-design.md` | Gains a "Run log" appendix recording the observed outcome of each state |

---

## Task 1: Custody the key

Move the keypair out of Downloads into `~/.secrets/` and restrict it to the owning user. This is state 1, and it is the only task that touches a secret at rest.

**Files:**
- Create: `C:\Users\nflach\.secrets\kannaka-hive-0xscada-qe-nostr.json`
- Delete: `C:\Users\nflach\Downloads\kannaka-hive-0xscada-qe-nostr.json`

**Interfaces:**
- Produces: the key file at the path every later task reads via `lib.mjs`'s `KEY_FILE`.

- [ ] **Step 1: Confirm the destructive half with the user before touching anything**

Deleting the Downloads copy is irreversible and the file is a secret. Ask explicitly, and do not proceed to Step 4 without a yes:

> "Moving `kannaka-hive-0xscada-qe-nostr.json` to `~/.secrets/` and deleting the Downloads copy. The nsec exists nowhere else on this box - confirm you have it backed up elsewhere, or say so and I'll copy instead of move."

If the user prefers a copy, do Step 2 and Step 3 and skip Step 4.

- [ ] **Step 2: Copy the file into `.secrets` and verify it parses**

```powershell
$src = "$env:USERPROFILE\Downloads\kannaka-hive-0xscada-qe-nostr.json"
$dst = "$env:USERPROFILE\.secrets\kannaka-hive-0xscada-qe-nostr.json"
Copy-Item $src $dst
$k = Get-Content $dst -Raw | ConvertFrom-Json
"organ=$($k.organ) npub=$($k.npub) pubkey=$($k.pubkey)"
"has_nsec=$([bool]$k.nsec) nsec_prefix=$($k.nsec.Substring(0,5))"
```

Expected: `organ=0xscada-qe`, an `npub1...`, a 64-char hex pubkey, `has_nsec=True`, `nsec_prefix=nsec1`.
The nsec itself is deliberately not printed - only its 5-char prefix, to prove the field is well-formed.

- [ ] **Step 3: Restrict the ACL to the owning user**

Windows has no `chmod`. Disabling inheritance and granting exactly one user is the 0600 equivalent:

```powershell
$dst = "$env:USERPROFILE\.secrets\kannaka-hive-0xscada-qe-nostr.json"
icacls $dst /inheritance:r /grant:r "$($env:USERNAME):(F)"
icacls $dst
```

Expected: the final `icacls` listing shows exactly one ACE, for `nflach`, and no `BUILTIN\Administrators`, `NT AUTHORITY\SYSTEM`, or `Users` entries. If more than one principal is listed, inheritance did not clear - stop and report rather than continuing with a world-readable secret.

- [ ] **Step 4: Remove the Downloads copy**

Only after Step 3's ACL is confirmed correct, and only if the user approved a move in Step 1:

```powershell
Remove-Item "$env:USERPROFILE\Downloads\kannaka-hive-0xscada-qe-nostr.json"
Test-Path "$env:USERPROFILE\Downloads\kannaka-hive-0xscada-qe-nostr.json"
```

Expected: `False`.

- [ ] **Step 5: Record the result**

State 1 is proven when: the `.secrets` file parses with all four fields, `icacls` shows a single user ACE, and the Downloads path returns `False`. Note all three observed outputs - Task 5 writes them into the run log.

No commit - nothing in a repository changed.

---

## Task 2: Prove the relay accepts this key (state 2)

Stand up the scratchpad tooling and answer the one question that gates everything downstream: is this pubkey on the relay's allowlist?

**Files:**
- Create: `<SCRATCH>\hive\package.json`
- Create: `<SCRATCH>\hive\lib.mjs`
- Create: `<SCRATCH>\hive\01-auth.mjs`

**Interfaces:**
- Produces, from `lib.mjs`:
  - `RELAY_URL: string` - `"wss://buzz.ninja-portal.com"`
  - `loadOrganKey(): { sk: Uint8Array, pubkey: string, npub: string, organ: string }`
  - `connect(): Promise<WebSocket>`
  - `sign(key, { kind, tags?, content? }): SignedEvent` - throws unless the result self-verifies
  - `authenticate(ws, key): Promise<{ ok: true, message: string }>` - rejects on refusal or socket close
  - `req(ws, subId, filter): Promise<object[]>` - resolves at EOSE
  - `publish(ws, signed): Promise<{ ok: boolean, message: string }>`

- [ ] **Step 1: Create the scratchpad package**

Create `<SCRATCH>\hive\package.json`:

```json
{
  "name": "hive-seat-organ",
  "private": true,
  "type": "module",
  "dependencies": {
    "nostr-tools": "^2.7.2"
  }
}
```

- [ ] **Step 2: Install the one dependency**

```powershell
cd "C:\Users\nflach\AppData\Local\Temp\claude\C--Windows-System32\802aa5fe-9d9a-4ca7-bfc3-bd243c7eaecd\scratchpad\hive"
npm install --no-audit --no-fund
```

Expected: `nostr-tools` and its `@noble/*` transitive deps installed, exit 0.
`nostr-tools` supplies three things Node 24 does not: bech32 decoding for the nsec (`nip19`), BIP-340 Schnorr signing (`finalizeEvent`), and signature verification (`verifyEvent`). The WebSocket itself is Node's own global - no `ws` package.

- [ ] **Step 3: Write the shared plumbing**

Create `<SCRATCH>\hive\lib.mjs`:

```js
// Shared plumbing for seating an organ in the Hive.
// Throwaway operator tooling - not part of any repository.
import { readFileSync } from 'node:fs'
import * as nip19 from 'nostr-tools/nip19'
import { finalizeEvent, verifyEvent } from 'nostr-tools/pure'

export const RELAY_URL = 'wss://buzz.ninja-portal.com'
export const KEY_FILE =
  process.env.HIVE_ORGAN_KEY ??
  `${process.env.USERPROFILE}\\.secrets\\kannaka-hive-0xscada-qe-nostr.json`

/**
 * Load the organ keypair. The secret key stays in memory as bytes and is
 * never printed, logged, or written anywhere.
 */
export function loadOrganKey() {
  const raw = JSON.parse(readFileSync(KEY_FILE, 'utf8'))
  const decoded = nip19.decode(raw.nsec)
  if (decoded.type !== 'nsec') {
    throw new Error(`key file nsec field decoded as ${decoded.type}, expected nsec`)
  }
  return { sk: decoded.data, pubkey: raw.pubkey, npub: raw.npub, organ: raw.organ }
}

export function connect() {
  return new Promise((resolve, reject) => {
    const ws = new WebSocket(RELAY_URL)
    const timer = setTimeout(() => reject(new Error('connect timed out after 15s')), 15000)
    ws.addEventListener('open', () => { clearTimeout(timer); resolve(ws) }, { once: true })
    ws.addEventListener('error', () => {
      clearTimeout(timer)
      reject(new Error(`could not open a socket to ${RELAY_URL}`))
    }, { once: true })
  })
}

/**
 * Sign an event with the organ key, refusing to return anything that does not
 * verify against its own signature.
 */
export function sign(key, { kind, tags = [], content = '' }) {
  const signed = finalizeEvent(
    { kind, tags, content, created_at: Math.floor(Date.now() / 1000) },
    key.sk,
  )
  if (!verifyEvent(signed)) {
    throw new Error(`self-verify failed for kind ${kind} - refusing to send`)
  }
  return signed
}

/**
 * NIP-42. The relay opens with ["AUTH", challenge]; we answer with a signed
 * kind-22242 and wait for the OK that names our auth event.
 *
 * A rejection here means the pubkey is not on the relay's allowlist. That is a
 * real answer, not a transient error - the caller must surface it, not retry.
 */
export function authenticate(ws, key, { timeoutMs = 20000 } = {}) {
  return new Promise((resolve, reject) => {
    let authEventId = null
    const timer = setTimeout(
      () => reject(new Error('no AUTH challenge from relay within 20s')),
      timeoutMs,
    )
    const done = (fn, arg) => {
      clearTimeout(timer)
      ws.removeEventListener('message', onMessage)
      fn(arg)
    }
    const onMessage = (ev) => {
      let frame
      try { frame = JSON.parse(ev.data) } catch { return }
      if (frame[0] === 'AUTH' && authEventId === null) {
        const signed = sign(key, {
          kind: 22242,
          tags: [['relay', RELAY_URL], ['challenge', frame[1]]],
        })
        authEventId = signed.id
        ws.send(JSON.stringify(['AUTH', signed]))
        return
      }
      if (frame[0] === 'OK' && frame[1] === authEventId) {
        if (frame[2] === true) done(resolve, { ok: true, message: frame[3] ?? '' })
        else done(reject, new Error(`relay REFUSED auth: ${frame[3] || '(no reason given)'}`))
      }
    }
    ws.addEventListener('message', onMessage)
    ws.addEventListener('close', (e) => {
      clearTimeout(timer)
      reject(new Error(`relay closed the socket during AUTH (code ${e.code}) - not allowlisted`))
    }, { once: true })
  })
}

/** REQ, collecting events until EOSE. */
export function req(ws, subId, filter, { timeoutMs = 15000 } = {}) {
  return new Promise((resolve, reject) => {
    const events = []
    const cleanup = () => {
      clearTimeout(timer)
      ws.removeEventListener('message', onMessage)
      try { ws.send(JSON.stringify(['CLOSE', subId])) } catch { /* socket already gone */ }
    }
    const timer = setTimeout(() => { cleanup(); resolve(events) }, timeoutMs)
    const onMessage = (ev) => {
      let frame
      try { frame = JSON.parse(ev.data) } catch { return }
      if (frame[1] !== subId) return
      if (frame[0] === 'EVENT') events.push(frame[2])
      else if (frame[0] === 'EOSE') { cleanup(); resolve(events) }
      else if (frame[0] === 'CLOSED') {
        cleanup()
        reject(new Error(`relay CLOSED ${subId}: ${frame[2] || '(no reason)'}`))
      }
    }
    ws.addEventListener('message', onMessage)
    ws.send(JSON.stringify(['REQ', subId, filter]))
  })
}

/** Publish a signed event and wait for the relay's OK naming it. */
export function publish(ws, signed, { timeoutMs = 15000 } = {}) {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(
      () => reject(new Error(`no OK for event ${signed.id} within 15s`)),
      timeoutMs,
    )
    const onMessage = (ev) => {
      let frame
      try { frame = JSON.parse(ev.data) } catch { return }
      if (frame[0] === 'OK' && frame[1] === signed.id) {
        clearTimeout(timer)
        ws.removeEventListener('message', onMessage)
        resolve({ ok: frame[2] === true, message: frame[3] ?? '' })
      }
    }
    ws.addEventListener('message', onMessage)
    ws.send(JSON.stringify(['EVENT', signed]))
  })
}
```

- [ ] **Step 4: Write the auth probe**

Create `<SCRATCH>\hive\01-auth.mjs`:

```js
// State 2: does the relay accept this organ's key?
import { connect, authenticate, loadOrganKey, RELAY_URL } from './lib.mjs'

const key = loadOrganKey()
console.log(`organ:  ${key.organ}`)
console.log(`npub:   ${key.npub}`)
console.log(`pubkey: ${key.pubkey}`)
console.log(`relay:  ${RELAY_URL}`)

const ws = await connect()
console.log('socket: open')

try {
  const result = await authenticate(ws, key)
  console.log(`AUTH:   ACCEPTED ${result.message ? `(${result.message})` : ''}`)
  console.log('STATE 2: PASS - this pubkey is allowlisted')
} catch (err) {
  console.log(`AUTH:   ${err.message}`)
  console.log('STATE 2: FAIL - stop here; the pubkey needs allowlisting on the Buzz box')
  process.exitCode = 1
} finally {
  ws.close()
}
```

- [ ] **Step 5: Run it**

```powershell
cd "C:\Users\nflach\AppData\Local\Temp\claude\C--Windows-System32\802aa5fe-9d9a-4ca7-bfc3-bd243c7eaecd\scratchpad\hive"
node 01-auth.mjs
```

Expected on success: `AUTH: ACCEPTED` and `STATE 2: PASS`.

Expected on failure: `STATE 2: FAIL`, with either a refusal reason or a socket-close code.

- [ ] **Step 6: Branch on the result**

**If STATE 2 FAILED - stop the plan here.** Report to the user: the pubkey `2b652d43...` is not on the relay's allowlist, the table lives in Postgres on the Buzz box (`flaukowski/kannaka-buzz`), and this machine has no SSH path to it (`~/.ssh` holds only `known_hosts`, with no `ninja-portal` entry). Hand over the npub and ask for it to be allowlisted. Tasks 3 and 4 cannot run and must not be faked. Task 5 still runs, recording the failure.

**If STATE 2 PASSED -** continue to Task 3.

No commit - nothing in a repository changed.

---

## Task 3: Survey rooms and existing identity (state 3 + reconnaissance)

Three questions in one connection: what rooms can this key see, what does a real kind-10100 look like on this relay, and does this pubkey already have identity events that a publish would clobber?

The middle question exists because the spec found no owner key on this box - the only npub in `kannaka-memory` is Kannaka's canonical voice identity (`README.md:25`), which is *not* a Hive owner reference. Rather than guess kind-10100's content shape, read one that the relay has already accepted.

**Files:**
- Create: `<SCRATCH>\hive\02-survey.mjs`

**Interfaces:**
- Consumes: `connect`, `authenticate`, `loadOrganKey`, `req` from `lib.mjs`
- Produces: the observed kind-10100 content shape, used to build Task 4's event

- [ ] **Step 1: Write the survey**

Create `<SCRATCH>\hive\02-survey.mjs`:

```js
// State 3 + pre-publish reconnaissance.
import { connect, authenticate, loadOrganKey, req } from './lib.mjs'

const key = loadOrganKey()
const ws = await connect()
await authenticate(ws, key)
console.log('AUTH:   ACCEPTED\n')

// --- State 3: which rooms is this key a member of? -------------------------
// buzz stores channel events channel-scoped and enforces access on read, so
// what comes back here is exactly what this key is entitled to see.
const rooms = await req(ws, 'rooms', { kinds: [39000] })
console.log(`=== ROOMS (kind 39000): ${rooms.length} ===`)
for (const r of rooms) {
  const tag = (n) => r.tags.find((t) => t[0] === n)?.[1]
  const flags = r.tags.filter((t) => t[0] === 'private' || t[0] === 'no-bridge').map((t) => t[0])
  console.log(`  ${tag('d')}  name=${tag('name') ?? '(unnamed)'}  ${flags.join(' ')}`)
}
console.log(rooms.length > 0
  ? 'STATE 3: PASS - this key is a member of at least one room'
  : 'STATE 3: FAIL - allowlisted but invited to nothing; needs a kind-9000 invite from a room admin')

// --- Shape discovery: what does a real agent profile look like here? -------
const agents = await req(ws, 'agents', { kinds: [10100], limit: 5 })
console.log(`\n=== EXISTING AGENT PROFILES (kind 10100): ${agents.length} ===`)
for (const a of agents) {
  console.log(`  pubkey=${a.pubkey}`)
  console.log(`  tags=${JSON.stringify(a.tags)}`)
  console.log(`  content=${a.content}`)
  console.log('  ---')
}
if (agents.length === 0) {
  console.log('  none visible - Task 4 falls back to the documented minimal shape')
}

// --- Clobber check: what does this pubkey already have? --------------------
const mine = await req(ws, 'mine', { kinds: [0, 10100], authors: [key.pubkey] })
console.log(`\n=== THIS ORGAN'S EXISTING IDENTITY EVENTS: ${mine.length} ===`)
for (const m of mine) {
  const when = new Date(m.created_at * 1000).toISOString()
  console.log(`  kind=${m.kind}  created_at=${when}`)
  console.log(`  content=${m.content}`)
}
if (mine.length === 0) {
  console.log('  none - Task 4 publishes fresh, nothing to clobber')
} else {
  console.log('  NOTE: kinds 0 and 10100 are replaceable. Publishing REPLACES the above.')
  console.log('  Preserve any fields worth keeping when composing Task 4 content.')
}

ws.close()
```

- [ ] **Step 2: Run it**

```powershell
cd "C:\Users\nflach\AppData\Local\Temp\claude\C--Windows-System32\802aa5fe-9d9a-4ca7-bfc3-bd243c7eaecd\scratchpad\hive"
node 02-survey.mjs
```

Expected: an `AUTH: ACCEPTED` line, then three labelled blocks. Every count is printed even when zero.

- [ ] **Step 3: Branch on state 3**

**If STATE 3 reports zero rooms**, the key is allowlisted but not invited anywhere. Report it and note that a kind-9000 invite from a room admin is required. This does **not** block Task 4 - the identity events are still worth publishing and are independent of room membership - so continue, but do not describe the organ as fully seated.

**If rooms came back**, note their names and ids for the run log. Flag any carrying `no-bridge`; those are the channels the future hive bridge must not export.

- [ ] **Step 4: Fix the kind-10100 content shape**

From the `EXISTING AGENT PROFILES` block, write down the exact tag list and content-JSON keys the relay has already accepted. Task 4 mirrors that shape.

If zero were visible, Task 4 uses the minimal shape documented in the hive-bridge spec - `{"channel_add_policy":"any"}` - and **omits** the `owner` field rather than inventing one. `hive_bridge/map.rs` reads `owner` as optional (`.and_then(...)`, yielding `null` when absent), so an absent owner is a supported case downstream. An owner reference can be added later by republishing; a *wrong* one is an assertion about who controls this organ and must not be guessed.

No commit - nothing in a repository changed.

---

## Task 4: Publish and verify the identity events (state 4)

Publish the kind-0 profile and the kind-10100 agent profile, then read both back from the relay and verify. Read-back is the proof - an `OK` means accepted, not stored and served.

**Files:**
- Create: `<SCRATCH>\hive\03-publish.mjs`

**Interfaces:**
- Consumes: `connect`, `authenticate`, `loadOrganKey`, `req`, `publish`, `sign` from `lib.mjs`; the content shape settled in Task 3 Step 4

- [ ] **Step 1: Write the publisher**

Create `<SCRATCH>\hive\03-publish.mjs`. If Task 3 Step 4 found a different real-world 10100 shape, edit `AGENT_CONTENT` and `AGENT_TAGS` to match it before running:

```js
// State 4: publish identity, then prove it is readable back.
import { connect, authenticate, loadOrganKey, req, publish, sign } from './lib.mjs'

const key = loadOrganKey()

// kind 0 - NIP-01 profile metadata. What the organ is called in the room.
const PROFILE_CONTENT = JSON.stringify({
  name: '0xSCADA-QE',
  display_name: '0xSCADA-QE',
  about:
    'QE / bug-hunt organ of the kannaka constellation. Adversarial review, ' +
    'wave-dev Build-Gate-Hunt-Fix. Workspace-scoped Hive key.',
})

// kind 10100 - agent profile, agent-authored, keyed by the agent's own pubkey.
// This is what hive_bridge/roster.rs reads to decide is_agent, which is the
// whole reason this event matters more than the kind 0.
// `owner` is deliberately absent: no canonical owner key was identified, and
// map.rs treats it as optional. Adjust only if Task 3 observed otherwise.
const AGENT_TAGS = []
const AGENT_CONTENT = JSON.stringify({ channel_add_policy: 'any' })

const ws = await connect()
await authenticate(ws, key)
console.log('AUTH:   ACCEPTED\n')

const profile = sign(key, { kind: 0, content: PROFILE_CONTENT })
const agent = sign(key, { kind: 10100, tags: AGENT_TAGS, content: AGENT_CONTENT })
console.log(`signed kind 0     id=${profile.id}`)
console.log(`signed kind 10100 id=${agent.id}`)
console.log('(both self-verified before send)\n')

for (const [label, ev] of [['kind 0', profile], ['kind 10100', agent]]) {
  const res = await publish(ws, ev)
  console.log(`publish ${label}: ${res.ok ? 'OK' : 'REJECTED'} ${res.message ? `(${res.message})` : ''}`)
  if (!res.ok) process.exitCode = 1
}

// --- Read-back. The OK above is acceptance; this is proof of storage. ------
console.log('\n=== READ-BACK ===')
const back = await req(ws, 'verify', { kinds: [0, 10100], authors: [key.pubkey] })
const seen = new Map(back.map((e) => [e.kind, e]))

let allGood = true
for (const [label, ev] of [['kind 0', profile], ['kind 10100', agent]]) {
  const got = seen.get(ev.kind)
  if (!got) {
    console.log(`  ${label}: MISSING - relay accepted it but does not serve it`)
    allGood = false
  } else if (got.id !== ev.id) {
    console.log(`  ${label}: served a DIFFERENT event (${got.id}) - a newer replaceable won`)
    allGood = false
  } else {
    console.log(`  ${label}: present, id matches, content=${got.content}`)
  }
}

console.log(allGood
  ? '\nSTATE 4: PASS - the organ is published and recognizable'
  : '\nSTATE 4: FAIL - see above')
if (!allGood) process.exitCode = 1

ws.close()
```

- [ ] **Step 2: Run it**

```powershell
cd "C:\Users\nflach\AppData\Local\Temp\claude\C--Windows-System32\802aa5fe-9d9a-4ca7-bfc3-bd243c7eaecd\scratchpad\hive"
node 03-publish.mjs
```

Expected: two `publish ... OK` lines, then a `READ-BACK` block showing both events present with matching ids, then `STATE 4: PASS`.

- [ ] **Step 3: Confirm the agent classification end to end**

The point of the 10100 is that `hive_bridge/roster.rs` will treat this pubkey as an agent. `Roster::apply` marks any kind-10100 author as an agent, keyed on `event.pubkey`, so confirm exactly that shape is what the relay now serves:

```powershell
cd "C:\Users\nflach\AppData\Local\Temp\claude\C--Windows-System32\802aa5fe-9d9a-4ca7-bfc3-bd243c7eaecd\scratchpad\hive"
node -e "import('./lib.mjs').then(async (m) => { const k = m.loadOrganKey(); const ws = await m.connect(); await m.authenticate(ws, k); const r = await m.req(ws, 'x', { kinds: [10100], authors: [k.pubkey] }); console.log('10100 events for this organ:', r.length); console.log('author matches organ pubkey:', r.every(e => e.pubkey === k.pubkey)); ws.close() })"
```

Expected: `10100 events for this organ: 1` and `author matches organ pubkey: true`.

- [ ] **Step 4: Record every observed value**

Capture the two event ids, the read-back contents, and the room list from Task 3. Task 5 writes them into the run log.

No commit - nothing in a repository changed.

---

## Task 5: Record the run log

The scripts are throwaway; the record of what happened is not. Append the observed outcome of all four states to the spec so the next person - or the hive-bridge work - knows the organ's actual status without re-running anything.

**Files:**
- Modify: `docs/superpowers/specs/2026-07-26-seat-0xscada-qe-in-hive-design.md`

**Interfaces:**
- Consumes: the observed outputs recorded in Tasks 1-4

- [ ] **Step 1: Append the run log to the spec**

Use the Edit tool, not a PowerShell round-trip. Add at the end of the spec, filling every bracketed value with what was actually observed. If a state failed, record the failure and the reason - a run log that only records successes is worthless:

```markdown
## Run log - 2026-07-26

| State | Result | Evidence |
|-------|--------|----------|
| 1 Custody | [PASS/FAIL] | key at `~/.secrets/kannaka-hive-0xscada-qe-nostr.json`; `icacls` shows [N] ACE(s); Downloads copy removed: [yes/no] |
| 2 Allowlisted | [PASS/FAIL] | [`AUTH: ACCEPTED` / the refusal reason or close code] |
| 3 Rooms | [PASS/FAIL] | [N] channels visible: [names, or "none - needs a kind-9000 invite"] |
| 4 Recognized | [PASS/FAIL] | kind 0 id `[...]`, kind 10100 id `[...]`, both read back and id-matched |

**kind-10100 content published:** `[exact JSON]`
**Owner reference:** [omitted - no canonical owner key identified / set to `...`]

**Open follow-ups:** [e.g. room invites needed; NIP-39 attestation still deferred]
```

- [ ] **Step 2: Verify the spec still reads coherently**

Re-read the spec's "The four states" table against the run log just written. Every state in the table must have a corresponding row. If the run diverged from the design - a different 10100 shape, an unexpected room-membership result - amend the relevant design section too, rather than leaving the spec describing something that didn't happen.

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/specs/2026-07-26-seat-0xscada-qe-in-hive-design.md
git commit -m "docs(spec): run log for seating the 0xscada-qe organ

Records the observed outcome of all four seating states, the exact
kind-10100 content published, and the follow-ups that remain.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

- [ ] **Step 4: Clean up the scratchpad**

The scripts held a secret key in memory and read the key file by path. They are throwaway by design:

```powershell
Remove-Item -Recurse -Force "C:\Users\nflach\AppData\Local\Temp\claude\C--Windows-System32\802aa5fe-9d9a-4ca7-bfc3-bd243c7eaecd\scratchpad\hive"
```

Skip this step if the identity-layer work is starting immediately and the scripts are a useful reference - but say so explicitly rather than leaving them silently behind.

---

## Notes for the implementer

- **Do not work around a stop.** If AUTH is refused, the answer is not "try without auth", "try the sovereignty relay instead", or "publish anyway". It is to report that the pubkey needs allowlisting on a box this machine cannot reach. The same applies to zero rooms.
- **The `nats` app and the hive bridge are not part of this plan.** They have their own spec and plan (`2026-07-26-hive-swarm-traffic-on-nostr`), and that work is partially implemented on branch `spec/hive-swarm-traffic`. Nothing here touches it.
- **Why the kind 10100 is worth this much care:** it is the single signal that makes this organ render as an agent rather than a human once the bridge runs. Getting it wrong is not a cosmetic bug - it silently mislabels every message this organ ever sends into the room.
