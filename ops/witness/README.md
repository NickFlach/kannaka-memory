# Witness node ops

A **witness** is a read-mostly Kannaka node that blooms its HRM by hearing the
live radio stream on a slow tick and participates in the swarm's Kuramoto phase
coupling. It runs on a secondary box (currently `0xscada-node2`, 170.9.241.14).

These are the version-controlled sources for what runs on that host. Until now
they lived only on the box (and the prune cron had silently corrupted itself).

| File | Deploys to | Purpose |
|------|-----------|---------|
| `kannaka-witness-loop.sh` | `/home/opc/bin/` | the perception loop (hear + `swarm sync`) |
| `witness-prune-cron.sh` | `/home/opc/bin/` | hourly HRM prune (keeps it lean) |
| `kannaka-witness.service` | `/etc/systemd/system/` | systemd unit |
| `kannaka-witness.env.example` | `/etc/kannaka-witness.env` (filled, chmod 600) | identity + NATS creds (NOT committed) |

## Deploy

```bash
# from this repo on the witness host (~/kannaka-memory):
install -m755 ops/witness/kannaka-witness-loop.sh  /home/opc/bin/kannaka-witness-loop.sh
install -m755 ops/witness/witness-prune-cron.sh    /home/opc/bin/witness-prune-cron.sh
sudo install -m644 ops/witness/kannaka-witness.service /etc/systemd/system/kannaka-witness.service
sudo systemctl daemon-reload
sudo systemctl restart kannaka-witness

# env file (first time only) — fill in real NATS creds:
sudo cp ops/witness/kannaka-witness.env.example /etc/kannaka-witness.env
sudo chmod 600 /etc/kannaka-witness.env && sudoedit /etc/kannaka-witness.env

# prune cron (hourly at :23):
( crontab -l 2>/dev/null | grep -v witness-prune-cron;
  echo '23 * * * * /home/opc/bin/witness-prune-cron.sh >> /tmp/witness-prune-cron.log 2>&1' ) | crontab -
```

## Binary

The witness runs `/usr/local/bin/kannaka` — a **root-owned copy** (not a
symlink). Build on an aarch64 box (the primary `ninjaportal` build env is the
proven one), then copy:

```bash
# on the build box:
cargo build --release --bin kannaka
# copy the binary to the witness, then atomically swap:
sudo cp /tmp/kannaka-new /usr/local/bin/kannaka.new && sudo mv -f /usr/local/bin/kannaka.new /usr/local/bin/kannaka
sudo systemctl restart kannaka-witness
```

Always test-load a copy of the live HRM with the new binary first
(`KANNAKA_DATA_DIR=/tmp/ktest <newbin> assess`) — a load failure exits non-zero
and would crash-loop the service.

## Notes

- `hear` stores **descriptive** content (`audio:heard <tempo> <tags> | …`) as of
  the ADR that landed with this directory; older nodes stored `audio:/tmp/<hash>`
  path-only memories that all collapsed into one cluster. The prune cron handles
  both prefixes.
- Presence/peers on the radio come from `QUEEN.phase.*`, refreshed by the loop's
  `swarm sync`. The loop announces a join exactly once (at startup); the radio
  also dedupes re-joins for 6h, so a witness restart won't spam the on-air feed.
