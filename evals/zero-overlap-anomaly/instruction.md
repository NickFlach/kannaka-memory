Run the Kannaka zero-overlap recall probe suite.

For every probe in `/environment/probes.json`, invoke:

    kannaka recall "<query>" --top-k 10

using the binary at `/environment/kannaka` with `KANNAKA_DATA_DIR` pointed at a fresh
copy of the frozen store and the recall knobs set exactly to
`KANNAKA_RECALL_ENERGY_EXP=0.0`, `KANNAKA_RECALL_TEMPORAL_EXP=1.0`.

Record, per probe, the raw JSON result array exactly as printed on stdout, plus parse
status, into `/app/rollout.json`, along with binary provenance and the sha256 of the
store copy used.
