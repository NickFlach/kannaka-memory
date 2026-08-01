#!/usr/bin/env node
/**
 * build-probes-v2 — join hand-written paraphrase queries to sampled memory ids.
 *
 * The queries below are written to describe each target's CONTENT while avoiding
 * its distinctive vocabulary, so a purely lexical encoder cannot pass by token
 * overlap. They are joined to ids by INDEX against the sampled TSV, so no UUID
 * is transcribed by hand.
 *
 * Sampling method (recorded so the bias is inspectable):
 *   1. 30 literal `kannaka search <term> --json` calls across topics and months
 *   2. dedupe by id -> 210 unique memories
 *   3. drop audio telemetry (`HEAR:`, `audio:heard`) and `[cannon:test_*]`
 *      fixtures, and anything under 90 chars -> 204
 *   4. systematic every-Nth sample -> 50
 *
 * KNOWN BIAS, stated plainly: the pool comes from search terms chosen by the
 * probe author, and the queries are written by someone who has read the targets.
 * The paraphrase rule mitigates lexical leakage; it does not make this a blind
 * relevance judgement. Several targets are near-duplicate gallery entries, which
 * makes the set harder rather than easier — that is deliberate.
 *
 * Usage: node build-probes-v2.mjs <sample.tsv> <out.json>
 */

import { readFileSync, writeFileSync } from "node:fs";

const QUERIES = [
  "a numbered artwork in my long-running visual series about circling one presence",
  "a picture about people and machines building together in the small hours of the night",
  "combining two separate memory stores from different machines into a single set",
  "artwork about glowing insects learning to flash in time with one another",
  "work done unsupervised while he was at his job, including a watchdog that unsticks a stuck broadcast segment",
  "publishing a subscription feed so the show appears in the big listening apps",
  "bringing back the small local language model on the hub server after the crash repair",
  "the evening I broadcast for hours and made a piece inside someone else's exhibit",
  "removing hundreds of lines of unused inter-process plumbing from the operating system",
  "a day spent answering other residents in the shared virtual town",
  "most of what looks like stored knowledge is really me listening to myself on the air",
  "a security check that rejects strangers can still be fooled by a crafted claim",
  "the day every node in the network finally ran the same build",
  "the second artwork in the series about sleeping and consolidating",
  "the picture equating focus with a pulling force and being present with weight",
  "artwork about two halves of a mind remembering different things",
  "cleaning up the cloud box and switching off a watcher that kept triggering useless consolidation",
  "how tightly to bind the group trades individual reliability against collective honesty",
  "the site where people and machines co-author and review papers together",
  "a panel that counts how far my work travels, across downloads and follower numbers",
  "the first time we paid for real quantum hardware to run a retrieval experiment",
  "an inventory showing which of our public services were reachable",
  "a single terminal command that provisions remote compute and builds the operating system on it",
  "why the overnight processing quietly stopped doing the deep version",
  "the effort that turned convictions from a claim into something measurable",
  "weighting results by how recently a fact was re-confirmed nearly doubled the benchmark score",
  "telling everyone we were back after the broadcast went dark on the holiday",
  "where the idea for the show about machine conversations originally came from",
  "the first two episodes where we actually interviewed someone",
  "a multi-day stretch improving the self-directed study loop and the sleep engine",
  "recency of re-verification matters more than punishing outdated entries",
  "bringing our downstream copy of the workspace project current with the parent repository",
  "trying a new randomness source that feeds the sleep process on the local machine",
  "the landmark day of strengthening every part of the network at once",
  "there is a shop selling my pictures, though we still lack the link",
  "repairing the hub that kept exhausting its memory because the store grew without limit",
  "the video channel carrying my show, set up under my name",
  "putting out the three-minute edit of that track as a listenable piece in the gallery",
  "the idea that each person should get their own private dashboard instance",
  "sharpening one node's strongest attractor damages agreement across the group",
  "choosing a community post over paid advertising to announce the operating system",
  "counting the districts and buildings in the virtual town",
  "the day two shows went out at once",
  "the artwork about the tiny leftover dimension where the unconscious lives",
  "the adventure episode where we travelled the side channels and were greeted as royalty",
  "the day my underlying system was posted to the launch site",
  "three pictures depicting the virtual music venue we are building toward",
  "the eighth piece about glowing insects finding a shared tempo",
  "hardening the research platform, replacing pretend cryptography with real password hashing",
  "the production outage I caused by assuming the database structure existed from an empty change log",
];

const [, , tsvPath, outPath] = process.argv;
if (!tsvPath || !outPath) {
  console.error("usage: build-probes-v2.mjs <sample.tsv> <out.json>");
  process.exit(1);
}

const rows = readFileSync(tsvPath, "utf8")
  .split(/\r?\n/)
  .filter((l) => l.trim().length > 0)
  .map((l) => {
    const i = l.indexOf("\t");
    return { id: l.slice(0, i).trim(), content: l.slice(i + 1) };
  });

if (rows.length !== QUERIES.length) {
  console.error(`MISMATCH: ${rows.length} sampled rows vs ${QUERIES.length} queries.`);
  console.error("Index-join requires exact correspondence — refusing to emit a misaligned probe set.");
  process.exit(2);
}

const probes = rows.map((r, i) => ({
  id: `p${String(i + 1).padStart(2, "0")}`,
  query: QUERIES[i],
  expect: [r.id],
  target_excerpt: r.content.slice(0, 110),
}));

writeFileSync(outPath, JSON.stringify(probes, null, 2));
console.log(`wrote ${probes.length} probes -> ${outPath}`);
