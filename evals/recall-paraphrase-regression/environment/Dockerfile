FROM ubuntu:24.04

# Deterministic recall harness world: real CLI + frozen HRM snapshot. No network at runtime.
RUN apt-get update && apt-get install -y --no-install-recommends python3 ca-certificates && rm -rf /var/lib/apt/lists/*

COPY data/kannaka-linux-x86_64 /environment/kannaka
COPY data/kannaka.hrm /environment/data/kannaka.hrm
COPY data/config.toml /environment/data/config.toml
COPY data/SNAPSHOT.sha256 /environment/data/SNAPSHOT.sha256
COPY probes.json /environment/probes.json
COPY run_probes.py /environment/run_probes.py
# Eval-side backfill driver for the facet-on arm (inert unless FACET_BACKFILL=1).
COPY data/facet-backfill /environment/facet-backfill
RUN chmod +x /environment/kannaka /environment/facet-backfill \
    # Guard from the stale-binary incident: refuse to build an image whose binary
    # does not contain the knobs this eval sets.
    && grep -aq KANNAKA_RECALL_ENERGY_EXP /environment/kannaka \
    && grep -aq KANNAKA_RECALL_TEMPORAL_EXP /environment/kannaka

WORKDIR /app
