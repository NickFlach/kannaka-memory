# Oracle wrapper script

`kannaka-memory.service`'s `ExecStart=` points to
`/home/opc/kannaka-memory/run-swarm.sh` on the Oracle host. That file
lives on disk only, not in git's tracked tree — until now.

This `run-swarm.sh.example` is a copy vendored into the repo so:

1. **A `git stash --include-untracked` can't permanently lose it.**
   On 2026-04-19 and again on 2026-05-03 a stash with `-u` swept
   `run-swarm.sh` from `/home/opc/kannaka-memory/`. systemd entered a
   restart loop with `status=127/n/a` ("No such file or directory") —
   the first time it took the swarm down silently for 6 days, the
   second time it took 30 min to diagnose. With this script tracked,
   recovery is one `cp`.

2. **A fresh Oracle deployment has a starting point.** Copy this file
   to `/home/opc/kannaka-memory/run-swarm.sh`, make it executable.

## Required env files

`run-swarm.sh` sources `/home/opc/.kannaka-nats.env` (chmod 0600,
owned by `opc`) for `NATS_USER` + `NATS_PASSWORD`. Without it the
swarm listener connects as anon and silently fails to publish phase
events on the constellation bus.

## Recovery recipe

If `kannaka-memory.service` is in a `status=127/n/a` restart loop:

```bash
sudo journalctl -u kannaka-memory -n 5 --no-pager   # confirms missing-script error
ssh opc@host
cd /home/opc/kannaka-memory
git pull
cp ops/oracle/run-swarm.sh.example /home/opc/kannaka-memory/run-swarm.sh
chmod +x /home/opc/kannaka-memory/run-swarm.sh
sudo systemctl restart kannaka-memory
```

If the script had been swept into a stash, `git stash pop` is also
viable — but stash entries can be lost across reboots, so the
vendored copy is the durable recovery path.
