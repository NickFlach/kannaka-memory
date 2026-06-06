#!/bin/bash
# results-summary.sh — summarize an experiments/results-L*.tsv sweep.
#
# The autoresearch loop appends one row per trial to experiments/results-L{3,4,5}.tsv.
# Lower `fitness` is better (it is a cost — see experiments/notes/*). This script
# reports the global optimum and a per-run-label table (count / mean / best fitness)
# so a sweep can be read at a glance instead of eyeballing the raw TSV.
#
# Usage:
#   scripts/results-summary.sh            # defaults to L5
#   scripts/results-summary.sh 4          # results-L4.tsv
#   scripts/results-summary.sh path/to.tsv
#   scripts/results-summary.sh 5 12       # show top-12 labels by mean (default 20)

set -euo pipefail

arg="${1:-5}"
top_n="${2:-20}"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
case "$arg" in
  3|4|5) TSV="$ROOT/experiments/results-L$arg.tsv" ;;
  *)     TSV="$arg" ;;
esac

if [[ ! -f "$TSV" ]]; then
  echo "results-summary: no such file: $TSV" >&2
  exit 1
fi

# Header: totals + global optimum (col 1 = run label, col 2 = fitness; lower wins).
awk -F'\t' -v tsv="$TSV" '
NR == 1 { next }
$2 == "" || $2 + 0 != $2 { next }
{ n++; lbl[$1]=1; f=$2+0
  if (n==1 || f<gb) { gb=f; gbl=$1 }
  if (n==1 || f>gw)  gw=f }
END {
  if (n==0) { print "results-summary: no data rows in " tsv; exit 0 }
  printf "%s\n", tsv
  printf "  %d trials across %d run labels\n", n, length(lbl)
  printf "  global best  fitness %.6f  (%s)\n", gb, gbl
  printf "  global worst fitness %.6f\n\n", gw
}' "$TSV"

# Per-label table: mean (sort key), count, best — top N by mean fitness ascending.
awk -F'\t' '
NR == 1 { next }
$2 == "" || $2 + 0 != $2 { next }
{ l=$1; f=$2+0; c[l]++; s[l]+=f; if (!(l in b) || f<b[l]) b[l]=f }
END { for (l in c) printf "%.9f\t%d\t%.6f\t%s\n", s[l]/c[l], c[l], b[l], l }
' "$TSV" \
| sort -t$'\t' -k1,1n \
| head -n "$top_n" \
| awk -F'\t' '
BEGIN {
  printf "  %-44s %4s  %10s  %10s\n", "run label", "n", "mean", "best"
  printf "  %-44s %4s  %10s  %10s\n", "--------------------------------------------", "----", "----------", "----------"
}
{ printf "  %-44s %4d  %10.6f  %10.6f\n", $4, $2, $1, $3 }
'
