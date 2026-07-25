#!/usr/bin/env node
/**
 * Thin launcher: exec the native kannaka binary fetched by install.js, passing
 * through argv and forwarding its exit code / signals. Keeps `npx kannaka` and
 * a global install behaving exactly like the native CLI.
 */
"use strict";

const { spawnSync } = require("child_process");
const path = require("path");
const fs = require("fs");

const ext = process.platform === "win32" ? ".exe" : "";
const bin = path.join(__dirname, `kannaka-bin${ext}`);

if (!fs.existsSync(bin)) {
  console.error(
    "kannaka: native binary not found — the postinstall download did not run or failed.\n" +
      "Reinstall with network access (e.g. `npm install -g kannaka`), or install directly:\n" +
      "  curl -sSf https://install.ninja-portal.com/kannaka | sh",
  );
  process.exit(1);
}

const res = spawnSync(bin, process.argv.slice(2), { stdio: "inherit" });
if (res.error) {
  console.error(`kannaka: ${res.error.message}`);
  process.exit(1);
}
if (res.signal) {
  // Re-raise the terminating signal so shells report it correctly.
  process.kill(process.pid, res.signal);
  return;
}
process.exit(res.status == null ? 1 : res.status);
