#!/usr/bin/env node
// Typecheck-fixture runner for @nubjs/types.
//
// Runs `tsc --noEmit` on each fixture and asserts the expected pass/fail outcome,
// so declaration regressions (a broken decl, a lost wildcard, an accidental module
// conversion, a re-introduced lib.dom collision) fail CI. Each fixture exercises
// the REAL package: `@nubjs/types` is installed as `file:..`, so its node_modules
// entry symlinks to the published `index.d.ts` — not a copy.
//
// Fixtures:
//   positive        — every @nubjs/types surface resolves (lib es2024, no dom) → PASS
//   stepaside-dom   — consumer also has Worker via lib.dom → no TS2403, coexists → PASS
//   stepaside-stub  — a separate DOM-shaped lib declares global Worker → step aside → PASS
//   negative-export — index.d.ts + `export {}` (now a module) breaks wildcards/globals → FAIL
//
// Usage: node run.mjs   (run from npm/nub-types/test, after `npm install`)

import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const tsc = join(here, "node_modules", ".bin", process.platform === "win32" ? "tsc.cmd" : "tsc");
const packageDts = join(here, "..", "index.d.ts");

if (!existsSync(tsc)) {
  console.error(`tsc not found at ${tsc} — run \`npm install\` in npm/nub-types/test first.`);
  process.exit(1);
}

// Generate the negative-control .d.ts: the CURRENT index.d.ts + `export {}` appended.
// Doing it at runtime keeps the control in lockstep with the real declarations.
const negDts = join(here, "fixtures", "negative-export", "nub-env-as-module.d.ts");
writeFileSync(negDts, readFileSync(packageDts, "utf8") + "\nexport {};\n");

/** @type {{name: string, dir: string, expect: "pass" | "fail"}[]} */
const fixtures = [
  { name: "positive", dir: "positive", expect: "pass" },
  { name: "stepaside-dom", dir: "stepaside-dom", expect: "pass" },
  { name: "stepaside-stub", dir: "stepaside-stub", expect: "pass" },
  { name: "negative-export", dir: "negative-export", expect: "fail" },
];

/** Run tsc on a fixture's tsconfig; return { ok, output }. */
function runTsc(dir) {
  const project = join(here, "fixtures", dir);
  try {
    const output = execFileSync(tsc, ["--noEmit", "-p", join(project, "tsconfig.json")], {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
    });
    return { ok: true, output };
  } catch (err) {
    return { ok: false, output: `${err.stdout ?? ""}${err.stderr ?? ""}` };
  }
}

let failed = 0;
for (const { name, dir, expect } of fixtures) {
  const { ok, output } = runTsc(dir);
  const got = ok ? "pass" : "fail";
  if (got === expect) {
    console.log(`✓ ${name}: tsc ${got} (expected ${expect})`);
  } else {
    failed++;
    console.error(`✗ ${name}: tsc ${got}, expected ${expect}`);
    if (output.trim()) console.error(output.trim().split("\n").map((l) => `    ${l}`).join("\n"));
  }
}

if (failed > 0) {
  console.error(`\n${failed} fixture(s) failed.`);
  process.exit(1);
}
console.log(`\nAll ${fixtures.length} fixtures behaved as expected.`);
