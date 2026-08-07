#!/usr/bin/env node
/**
 * recall-harness — measure recall quality against the REAL production path.
 *
 * WHY THIS EXISTS
 * ---------------
 * Every claim made about recall quality before this harness was an anecdote:
 * a human eyeballing a top-3 and deciding whether it "looked right". That
 * produced, in one session, six confident wrong diagnoses — including an A/B
 * run against a binary that predated the feature under test by three hours.
 *
 * So this harness is built around three rules:
 *
 *   1. EXERCISE THE PRODUCTION PATH. It shells out to the real `kannaka recall`
 *      CLI, which dispatches through `resonate_query` -> ChiralMedium. It does
 *      NOT call `recall_resonance_readonly`, which runs the FLAT medium and is
 *      therefore a different scorer than production. Measuring the wrong path
 *      is how you get a confident number about nothing.
 *
 *   2. RECORD WHAT PRODUCED THE NUMBER. Binary path, mtime, size, `--version`,
 *      and the fully-resolved knob values for every arm are written into the
 *      results. A result whose provenance is unknown is not a result.
 *
 *   3. NEVER INHERIT KNOBS. Every arm sets every knob explicitly. The ambient
 *      environment may already have KANNAKA_RECALL_* set (it does, on this
 *      machine), which would silently make "baseline" mean something else.
 *
 * GROUND TRUTH
 * ------------
 * Probe expectations are pinned by memory UUID, established with `kannaka
 * search` (literal substring/token match) — a DIFFERENT mechanism from
 * resonance. A probe query should then be a PARAPHRASE that shares as little
 * vocabulary with the target as possible, so a lexical encoder cannot pass it
 * by token overlap alone.
 *
 * KNOWN LIMITATION (do not paper over this): probes written by the same person
 * who knows the target are subject to selection bias. The paraphrase rule
 * mitigates it; it does not eliminate it. Treat small probe sets as directional.
 *
 * CONTAMINATION
 * -------------
 * CLI recall calls `record_retrieval()`, which bumps `retrieval_count` (it does
 * NOT touch amplitude/energy — verified). Ranking does not currently read
 * retrieval_count, so probing is believed non-perturbing, but arms are run in
 * INTERLEAVED order per probe rather than arm-by-arm so that any drift affects
 * all arms equally instead of systematically favouring whichever ran first.
 *
 * USAGE
 *   node scripts/recall-harness.mjs [--probes FILE] [--k 10] [--out DIR] [--bin PATH]
 */

import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync, mkdirSync, statSync, existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(HERE, "..");

// ── args ──────────────────────────────────────────────────────────────────
const argv = process.argv.slice(2);
const arg = (name, dflt) => {
  const i = argv.indexOf(name);
  return i >= 0 && argv[i + 1] ? argv[i + 1] : dflt;
};

const PROBES = resolve(arg("--probes", join(HERE, "probes", "probes-v1.json")));
const K = Number.parseInt(arg("--k", "10"), 10);
const OUT_DIR = resolve(arg("--out", join(REPO, "experiments")));
const BIN = resolve(
  arg("--bin", process.env.KANNAKA_BIN || join(process.env.USERPROFILE || process.env.HOME || ".", ".local", "bin", process.platform === "win32" ? "kannaka.exe" : "kannaka")),
);

/**
 * Arms. Every arm sets EVERY knob — see rule 3. `baseline` is the shipped
 * default (energy_exp 1.0 = full rich-get-richer, temporal off), NOT whatever
 * happens to be in the ambient environment.
 */
const ARMS = [
  { name: "baseline", env: { KANNAKA_RECALL_ENERGY_EXP: "1.0", KANNAKA_RECALL_TEMPORAL_EXP: "0.0" } },
  { name: "energy_neutral", env: { KANNAKA_RECALL_ENERGY_EXP: "0.0", KANNAKA_RECALL_TEMPORAL_EXP: "0.0" } },
  { name: "temporal_only", env: { KANNAKA_RECALL_ENERGY_EXP: "1.0", KANNAKA_RECALL_TEMPORAL_EXP: "1.0" } },
  { name: "both", env: { KANNAKA_RECALL_ENERGY_EXP: "0.0", KANNAKA_RECALL_TEMPORAL_EXP: "1.0" } },
];

// ── provenance ────────────────────────────────────────────────────────────
function binaryProvenance() {
  if (!existsSync(BIN)) throw new Error(`kannaka binary not found: ${BIN}`);
  const st = statSync(BIN);
  let version = "unknown";
  try {
    version = execFileSync(BIN, ["--version"], { encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] })
      .split("\n")[0]
      .trim();
  } catch { /* keep "unknown" rather than failing the run */ }
  return { path: BIN, version, mtime: st.mtime.toISOString(), bytes: st.size };
}

/** Which knob strings are actually compiled into this binary. */
function knobsCompiledIn() {
  const buf = readFileSync(BIN);
  const hay = buf.toString("latin1");
  const names = ["KANNAKA_RECALL_ENERGY_EXP", "KANNAKA_RECALL_TEMPORAL_EXP", "KANNAKA_CHIRAL_RECALL"];
  return Object.fromEntries(names.map((n) => [n, hay.includes(n)]));
}

function lastDreamIso() {
  try {
    const out = execFileSync(BIN, ["status"], { encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] });
    return JSON.parse(out).last_dream ?? null;
  } catch { return null; }
}

// ── recall ────────────────────────────────────────────────────────────────
function recall(query, k, env) {
  const out = execFileSync(BIN, ["recall", query, "--top-k", String(k)], {
    encoding: "utf8",
    // stderr carries the HRM load banner; keep it off stdout so JSON parses.
    stdio: ["ignore", "pipe", "ignore"],
    env: { ...process.env, ...env },
    maxBuffer: 32 * 1024 * 1024,
  });
  try {
    const parsed = JSON.parse(out);
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}

// ── metrics ───────────────────────────────────────────────────────────────
/** Rank (1-based) of the first returned id that is relevant; 0 if none. */
function firstRelevantRank(ids, relevant) {
  for (let i = 0; i < ids.length; i++) if (relevant.has(ids[i])) return i + 1;
  return 0;
}

function recallAtK(ids, relevant, k) {
  if (relevant.size === 0) return null;
  let hit = 0;
  for (const id of ids.slice(0, k)) if (relevant.has(id)) hit++;
  return hit / relevant.size;
}

/** Binary-relevance nDCG@k. */
function ndcgAtK(ids, relevant, k) {
  if (relevant.size === 0) return null;
  let dcg = 0;
  ids.slice(0, k).forEach((id, i) => {
    if (relevant.has(id)) dcg += 1 / Math.log2(i + 2);
  });
  let idcg = 0;
  for (let i = 0; i < Math.min(relevant.size, k); i++) idcg += 1 / Math.log2(i + 2);
  return idcg === 0 ? null : dcg / idcg;
}

// ── run ───────────────────────────────────────────────────────────────────
function main() {
  const probes = JSON.parse(readFileSync(PROBES, "utf8"));
  if (!Array.isArray(probes) || probes.length === 0) {
    console.error(`no probes in ${PROBES}`);
    process.exit(1);
  }

  const prov = binaryProvenance();
  const compiled = knobsCompiledIn();
  const missing = Object.entries(compiled).filter(([, v]) => !v).map(([n]) => n);
  if (missing.length) {
    // The exact failure that invalidated an entire evening: measuring knobs the
    // binary does not contain. Refuse rather than produce a confident null.
    console.error(`REFUSING TO RUN — knobs absent from binary: ${missing.join(", ")}`);
    console.error(`  binary: ${prov.path} (${prov.version}, mtime ${prov.mtime})`);
    console.error("  rebuild at HEAD and reinstall, then re-run.");
    process.exit(2);
  }

  const meta = {
    started: new Date().toISOString(),
    binary: prov,
    knobs_compiled_in: compiled,
    last_dream: lastDreamIso(),
    k: K,
    probes_file: PROBES,
    probe_count: probes.length,
    arms: ARMS.map((a) => ({ name: a.name, env: a.env })),
    note: "arms interleaved per probe; expectations pinned by uuid via literal `kannaka search`",
  };

  const rows = [];
  const perArm = Object.fromEntries(ARMS.map((a) => [a.name, { recall: [], mrr: [], ndcg: [] }]));

  for (const p of probes) {
    const relevant = new Set(p.expect ?? []);
    // Interleave arms within a probe (rule: no arm systematically runs first).
    for (const armIdx of shuffledIndices(ARMS.length, hashSeed(p.id))) {
      const arm = ARMS[armIdx];
      const results = recall(p.query, K, arm.env);
      const ids = results.map((r) => r.id);
      const rank = firstRelevantRank(ids, relevant);
      const r = recallAtK(ids, relevant, K);
      const n = ndcgAtK(ids, relevant, K);
      const mrr = rank === 0 ? 0 : 1 / rank;

      perArm[arm.name].recall.push(r ?? 0);
      perArm[arm.name].mrr.push(mrr);
      perArm[arm.name].ndcg.push(n ?? 0);

      rows.push({
        probe: p.id,
        arm: arm.name,
        first_rank: rank,
        recall_at_k: r,
        mrr,
        ndcg_at_k: n,
        returned: ids.length,
        top1: results[0]?.content?.slice(0, 60) ?? "",
      });
      process.stderr.write(`  ${p.id} / ${arm.name}: rank=${rank || "-"}\n`);
    }
  }

  const mean = (xs) => (xs.length ? xs.reduce((a, b) => a + b, 0) / xs.length : 0);
  const summary = ARMS.map((a) => ({
    arm: a.name,
    recall_at_k: +mean(perArm[a.name].recall).toFixed(4),
    mrr: +mean(perArm[a.name].mrr).toFixed(4),
    ndcg_at_k: +mean(perArm[a.name].ndcg).toFixed(4),
  }));

  mkdirSync(OUT_DIR, { recursive: true });
  const stamp = meta.started.replace(/[:.]/g, "-");
  const jsonPath = join(OUT_DIR, `recall-harness-${stamp}.json`);
  const tsvPath = join(OUT_DIR, `recall-harness-${stamp}.tsv`);
  writeFileSync(jsonPath, JSON.stringify({ meta, summary, rows }, null, 2));
  writeFileSync(
    tsvPath,
    ["probe\tarm\tfirst_rank\trecall_at_k\tmrr\tndcg_at_k\treturned"]
      .concat(rows.map((r) => [r.probe, r.arm, r.first_rank, r.recall_at_k, r.mrr, r.ndcg_at_k, r.returned].join("\t")))
      .join("\n"),
  );

  console.log(`\nbinary: ${prov.version}  mtime=${prov.mtime}`);
  console.log(`probes: ${probes.length}   k=${K}   last_dream=${meta.last_dream ?? "never"}\n`);
  console.log("arm             recall@k    mrr     ndcg@k");
  for (const s of summary) {
    console.log(`${s.arm.padEnd(15)} ${String(s.recall_at_k).padEnd(11)} ${String(s.mrr).padEnd(7)} ${s.ndcg_at_k}`);
  }
  console.log(`\nwrote ${jsonPath}`);
  console.log(`wrote ${tsvPath}`);
  if (probes.length < 30) {
    console.log(`\nNOTE: ${probes.length} probes is DIRECTIONAL ONLY, not a measurement.`);
  }
}

/** Deterministic per-probe arm order so runs are reproducible. */
function hashSeed(s) {
  let h = 2166136261;
  for (const ch of String(s)) { h ^= ch.charCodeAt(0); h = Math.imul(h, 16777619); }
  return h >>> 0;
}
function shuffledIndices(n, seed) {
  const idx = Array.from({ length: n }, (_, i) => i);
  let s = seed || 1;
  for (let i = n - 1; i > 0; i--) {
    s = (Math.imul(s, 1103515245) + 12345) >>> 0;
    const j = s % (i + 1);
    [idx[i], idx[j]] = [idx[j], idx[i]];
  }
  return idx;
}

main();
