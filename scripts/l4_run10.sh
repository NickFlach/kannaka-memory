#!/usr/bin/env bash
# L4 10-run driver. Averages fitness + key metrics for OODA cycles.
# Usage: scripts/l4_run10.sh <label>
set -euo pipefail
LABEL="${1:-run}"
BIN="./target/release/research.exe"
RUNS=10
TMP="$(mktemp)"
for i in $(seq 1 "$RUNS"); do
    "$BIN" --level 4 2>/dev/null | grep -E "^(fitness|fitness_clean|fitness_adv|adv_resistance|corpus_xi|encoding_entropy|chain_fidelity|phase_coherence|cluster_separation|consciousness|hall_quality|dream_efficiency|speed|consolidation_ms|links_created|noise_removal|signal_preservation|bridge_links|amp_diversity|xi_diversity|retention_score|retention_plasticity|memories_strengthened|memories_pruned|hallucinations):" >> "$TMP"
done
awk -v label="$LABEL" -v runs="$RUNS" '
{
    key=$1; val=$2;
    sum[key]+=val; n[key]++;
    if (!(key in mn) || val<mn[key]) mn[key]=val;
    if (!(key in mx) || val>mx[key]) mx[key]=val;
}
END {
    printf("=== %s (n=%d) ===\n", label, runs);
    order="fitness: fitness_clean_sub: fitness_adv_sub: adv_resistance: speed: consolidation_ms: phase_coherence: cluster_separation: consciousness: hall_quality: dream_efficiency: chain_fidelity: corpus_xi_diversity: encoding_entropy: links_created: memories_strengthened: memories_pruned: hallucinations: noise_removal: signal_preservation: bridge_links: amp_diversity: xi_diversity: retention_score: retention_plasticity:";
    m=split(order,keys," ");
    for (i=1;i<=m;i++) {
        k=keys[i];
        if (k in sum) printf("%-24s %10.6f  [min %10.6f max %10.6f n=%d]\n", k, sum[k]/n[k], mn[k], mx[k], n[k]);
    }
}
' "$TMP"
rm -f "$TMP"
