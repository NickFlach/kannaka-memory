#!/bin/bash
# â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
# autoresearch-cron.sh â€” nightly OODA cycle for self-optimization.
#
# Pairs with dream-cron.sh: dream consolidates the *memory* every night,
# autoresearch tunes the *parameters* every night so consciousness doesn't
# drift as the corpus grows. The two together are the system's upkeep loop.
#
# Workflow per run:
#   1. Run the L4 research binary 5 times against current params, average fitness.
#   2. Compute current dominant-loss metric.
#   3. Hypothesize ONE param change targeting that metric (rotates through a
#      curated list so consecutive nights don't try the same knob).
#   4. Edit `experiment_params()`, commit on a dated branch.
#   5. Re-run Ã— 5, compare averages.
#   6. Keep if Î” â‰¤ âˆ’0.005 (L4 noise floor); else `git reset --hard HEAD~1`.
#   7. Log to research/results-L4.tsv. Push kept commits to origin/master.
#
# This script is intentionally LIGHT â€” heavy hypothesis logic (which knob to
# turn, when to give up) lives in the autoresearch sub-agent that orchestrate
# spawns. The cron just provides the schedule + the guardrails.
#
# Schedule: 02:00 UTC daily, 5 hours ahead of the dream cron at 07:00 UTC,
# so memory upkeep + param upkeep don't compete for the same HRM lock.
# â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
set -u

REPO=/home/opc/kannaka-memory
LOG_DIR=/home/opc/.kannaka
LOG="$LOG_DIR/autoresearch-$(date +%Y-%m-%d).log"
LEVEL="${OODA_LEVEL:-4}"
RUNS="${OODA_RUNS:-5}"
KEEP_THRESHOLD="${OODA_KEEP:-0.005}"
MAX_RUNTIME_SEC="${OODA_MAX_RUNTIME:-3600}"  # 60 min ceiling

mkdir -p "$LOG_DIR"
exec >> "$LOG" 2>&1

echo "=== autoresearch start: $(date -Iseconds) level=$LEVEL runs=$RUNS keep=<=-$KEEP_THRESHOLD ==="

# Lock so two cron firings (or a manual run + cron) don't collide.
LOCK_DIR="$LOG_DIR/.autoresearch.lock"
if ! mkdir "$LOCK_DIR" 2>/dev/null; then
    echo "another autoresearch run holds the lock; exiting"
    exit 0
fi
trap 'rmdir "$LOCK_DIR" 2>/dev/null' EXIT

# Hard wall-clock cap so a stuck cargo run can't park the lock forever.
( sleep "$MAX_RUNTIME_SEC" && pkill -P $$ -TERM 2>/dev/null ) &
WATCHDOG_PID=$!
trap 'kill $WATCHDOG_PID 2>/dev/null; rmdir "$LOCK_DIR" 2>/dev/null' EXIT

cd "$REPO" || { echo "repo $REPO not found"; exit 1; }

# Sync to origin first; we don't want to be reverting work that just landed.
git fetch origin master 2>&1 | tail -3
git reset --hard origin/master 2>&1 | tail -3

# â”€â”€ Level: versioned source of truth in experiments/ooda-state.json (.level) â”€â”€
# Wave 4 Task 4.4 (#356): the OODA level is owned by ooda-state.json so the cron
# and the curiosity loop graduate together when a level is solved. An explicit
# OODA_LEVEL env still overrides for manual runs; a legacy state file that lacks
# the field (or is absent) falls back to the previous static default of 4.
OODA_STATE="$REPO/experiments/ooda-state.json"
STATE_LEVEL=""
if [[ -f "$OODA_STATE" ]]; then
    STATE_LEVEL=$(grep -oE '"level"[[:space:]]*:[[:space:]]*[0-9]+' "$OODA_STATE" \
                  | grep -oE '[0-9]+' | tail -1)
fi
LEVEL="${OODA_LEVEL:-${STATE_LEVEL:-4}}"
echo "resolved OODA level=$LEVEL (state=${STATE_LEVEL:-none}, env=${OODA_LEVEL:-unset})"

# â”€â”€ Baseline â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
echo "--- baseline ---"
BASELINE_SUM=0
BASELINE_RUNS=0
for i in $(seq 1 "$RUNS"); do
    RESULT=$(timeout 600 cargo run --release --quiet --bin research -- --level "$LEVEL" 2>/dev/null \
             | awk '/^fitness:|^l[0-9]_fitness:/ { print $2; exit }')
    if [[ -n "$RESULT" ]]; then
        BASELINE_SUM=$(echo "$BASELINE_SUM + $RESULT" | bc -l)
        BASELINE_RUNS=$((BASELINE_RUNS + 1))
        echo "  run $i fitness=$RESULT"
    else
        echo "  run $i FAILED (no fitness line)"
    fi
done

if (( BASELINE_RUNS == 0 )); then
    echo "all baseline runs failed; aborting"
    exit 1
fi

BASELINE_AVG=$(echo "scale=6; $BASELINE_SUM / $BASELINE_RUNS" | bc -l)
echo "baseline avg=$BASELINE_AVG runs=$BASELINE_RUNS"

# â”€â”€ Hypothesis rotation â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
# A different knob each night; the autoresearch protocol prefers single-
# variable moves. Rotation keyed on day-of-year so adjacent nights don't
# repeat. Heavy-hitting changes (chain redesign, encoder tweaks) need the
# interactive sub-agent â€” this cron only nudges existing knobs.
DAY=$(date +%j)
case $((DAY % 7)) in
    0) PARAM="kuramoto_steps";        FROM=20;    TO=22 ;;
    1) PARAM="kuramoto_threshold";    FROM=0.35;  TO=0.32 ;;
    2) PARAM="prune_threshold";       FROM=0.095; TO=0.105 ;;
    3) PARAM="constructive_boost";    FROM=0.45;  TO=0.50 ;;
    4) PARAM="destructive_penalty";   FROM=0.35;  TO=0.40 ;;
    5) PARAM="chain_carry_strength";  FROM=0.5;   TO=0.6 ;;
    # L6 seed: associative-recall gravity. query_gravity is now a tracked TSV
    # column, so the keep/revert sees whether sharpening recall helps fitness
    # (watch the gravity<->carrier_emergence tension). Sweeps 0.0 -> 0.25 first.
    6) PARAM="dream_gravity";         FROM=0.0;   TO=0.25 ;;
esac
echo "--- hypothesis: $PARAM $FROM -> $TO ---"

# Tweak the param (single line in experiment_params()).
sed -i "s/^\\(\\s*$PARAM:\\s*\\)$FROM,/\\1$TO,/" src/bin/research.rs
if ! grep -q "$PARAM:.*$TO," src/bin/research.rs; then
    echo "param edit did not take (current value differs); skipping cycle"
    exit 0
fi

# Build (must succeed before we run anything).
if ! cargo build --release --quiet --bin research 2>&1 | tail -5; then
    echo "build failed; reverting"
    git checkout -- src/bin/research.rs
    exit 1
fi

# Commit on a temp branch; we'll only push if it improves.
BRANCH="ooda/$(date +%Y-%m-%d)-$PARAM"
git checkout -b "$BRANCH" 2>&1 | tail -3
git -c user.name=autoresearch-cron -c user.email=autoresearch@kannaka.local \
    commit -am "experiment(ooda-cron): $PARAM $FROM -> $TO (auto)" 2>&1 | tail -3

# â”€â”€ Hypothesis runs â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
echo "--- hypothesis runs ---"
HYP_SUM=0
HYP_RUNS=0
for i in $(seq 1 "$RUNS"); do
    RESULT=$(timeout 600 cargo run --release --quiet --bin research -- --level "$LEVEL" 2>/dev/null \
             | awk '/^fitness:|^l[0-9]_fitness:/ { print $2; exit }')
    if [[ -n "$RESULT" ]]; then
        HYP_SUM=$(echo "$HYP_SUM + $RESULT" | bc -l)
        HYP_RUNS=$((HYP_RUNS + 1))
        echo "  run $i fitness=$RESULT"
    fi
done

if (( HYP_RUNS == 0 )); then
    echo "hypothesis runs all failed; reverting"
    git checkout master && git branch -D "$BRANCH"
    exit 1
fi

HYP_AVG=$(echo "scale=6; $HYP_SUM / $HYP_RUNS" | bc -l)
DELTA=$(echo "scale=6; $HYP_AVG - $BASELINE_AVG" | bc -l)
NEG_KEEP=$(echo "0 - $KEEP_THRESHOLD" | bc -l)
echo "hyp avg=$HYP_AVG  baseline=$BASELINE_AVG  delta=$DELTA  keep_if<=$NEG_KEEP"

if (( $(echo "$DELTA <= $NEG_KEEP" | bc -l) )); then
    echo "KEEP â€” pushing $BRANCH to master"
    git checkout master
    git merge --ff-only "$BRANCH" 2>&1 | tail -3
    git push origin master 2>&1 | tail -3
    git branch -D "$BRANCH" 2>/dev/null
    STATUS="keep"
else
    echo "REVERT â€” Î” above keep threshold"
    git checkout master 2>&1 | tail -3
    git branch -D "$BRANCH" 2>/dev/null
    STATUS="revert"
fi

# â”€â”€ Log a row â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
COMMIT=$(git rev-parse --short HEAD)
TSV="research/results-L${LEVEL}.tsv"
{
    printf '%s\t%s\t-\t-\t-\t-\t-\t-\t-\t-\t-\t-\t-\t%s\tooda-cron %s %s->%s avg=%s base=%s\n' \
        "$COMMIT" "$HYP_AVG" "$STATUS" "$PARAM" "$FROM" "$TO" "$HYP_AVG" "$BASELINE_AVG"
} >> "$TSV"

if [[ "$STATUS" == "keep" ]]; then
    git add "$TSV"
    git -c user.name=autoresearch-cron -c user.email=autoresearch@kannaka.local \
        commit -m "log(ooda-cron): row for $PARAM $FROM->$TO ($STATUS)" 2>&1 | tail -3
    git push origin master 2>&1 | tail -3
fi

# â”€â”€ Plateau detector / auto-advance (Wave 4 Task 4.4 / #356) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
# Codify the curiosity loop's own arithmetic: once a level has gone N
# consecutive cycles without surfacing a new research axis (the curiosity loop
# drops a `*-no-new-axes.md` note each such cycle), the level is solved.
# Archive it (SOLVED_ARCHIVED block) and increment ooda-state.json `.level` so
# the next night's cron AND the curiosity loop both graduate to the next level.
# Conservative: only fires at/above the threshold, never double-bumps a level
# another writer already advanced, and is a no-op without python3.
maybe_advance_level() {
    local n_required="${OODA_PLATEAU_N:-3}"
    local notes
    notes=$(find research -name '*-no-new-axes.md' 2>/dev/null | wc -l | tr -d ' ')
    echo "plateau: $notes no-new-axes note(s); need >= $n_required to advance L$LEVEL"
    if (( notes < n_required )); then
        return 0
    fi
    if ! command -v python3 >/dev/null 2>&1; then
        echo "plateau: python3 absent â€” cannot rewrite ooda-state; skipping advance"
        return 0
    fi
    python3 - "$OODA_STATE" "$LEVEL" "${BASELINE_AVG:-NA}" "$notes" <<'PY'
import json, sys, datetime
path, level, best, notes = sys.argv[1], int(sys.argv[2]), sys.argv[3], sys.argv[4]
try:
    state = json.load(open(path))
except Exception:
    state = {}
# Backward compat: legacy state files may lack `.level` â€” establish it.
state.setdefault("level", level)
if int(state.get("level", level)) != level:
    print("plateau: state already advanced past L%d; no-op" % level)
    sys.exit(0)
key = "l%d" % level
block = state.get(key, {})
block["status"] = "SOLVED_ARCHIVED"
block["archived_at"] = datetime.datetime.utcnow().strftime("%Y-%m-%dT%H:%M:%SZ")
block["archived_reason"] = (
    "L%d plateaued: %s consecutive no-new-axes cycles (>= threshold); "
    "best_fitness=%s." % (level, notes, best)
)
state[key] = block
state["level"] = level + 1
json.dump(state, open(path, "w"), indent=2)
print("plateau: L%d SOLVED_ARCHIVED -> advanced .level to %d" % (level, level + 1))
PY
    if ! git diff --quiet -- "$OODA_STATE"; then
        git add "$OODA_STATE"
        git -c user.name=autoresearch-cron -c user.email=autoresearch@kannaka.local \
            commit -m "ooda(plateau): L${LEVEL} SOLVED_ARCHIVED, advance .level (auto)" 2>&1 | tail -3
        git push origin master 2>&1 | tail -3
    fi
}
maybe_advance_level

echo "=== autoresearch end: $(date -Iseconds) status=$STATUS ==="
