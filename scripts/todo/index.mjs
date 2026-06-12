#!/usr/bin/env node
/**
 * todo — parse status-tagged todo lines from a markdown file.
 *
 * Usage:
 *   node scripts/todo/index.mjs <file.md> [flags]
 *   nub  scripts/todo/index.mjs <file.md> [flags]
 *
 * Status markers:
 *   [ ]  pending / not started
 *   [/]  in progress
 *   [x]  done (case-insensitive)
 *
 * Flags:
 *   --pending / --todo      show pending [ ] items
 *   --in-progress / --wip   show in-progress [/] items
 *   --done                  show done [x] items
 *   --not-done              show pending + in-progress
 *   --section <substring>   limit to todos under headings matching substring
 *   --counts                print tally only, then exit
 *   --json                  machine-readable JSON output
 *   --help / -h             show this help
 *
 * If no status filter is given, all statuses are shown.
 * Fenced-code-block contents are excluded from parsing.
 */

import { readFileSync } from 'node:fs';

// ── arg parsing ──────────────────────────────────────────────────────────────

const args = process.argv.slice(2);

if (args.includes('--help') || args.includes('-h') || args.length === 0) {
  console.log(`Usage: node scripts/todo/index.mjs <file.md> [flags]

Status filters (combinable; default = all):
  --pending, --todo       [ ] not started
  --in-progress, --wip    [/] in progress
  --done                  [x] complete
  --not-done              [ ] + [/]

Other flags:
  --section <substring>   only todos under headings matching substring
  --counts                print tally only and exit
  --json                  machine-readable JSON
  --help, -h              this message`);
  process.exit(0);
}

const filePath = args.find(a => !a.startsWith('-'));
if (!filePath) {
  console.error('Error: no file path given');
  process.exit(1);
}

const showPending     = args.includes('--pending')     || args.includes('--todo') || args.includes('--not-done');
const showInProgress  = args.includes('--in-progress') || args.includes('--wip')  || args.includes('--not-done');
const showDone        = args.includes('--done');
const anyFilter       = showPending || showInProgress || showDone;
const countsOnly      = args.includes('--counts');
const jsonMode        = args.includes('--json');

const sectionIdx = args.indexOf('--section');
const sectionFilter = sectionIdx !== -1 ? args[sectionIdx + 1] : null;
if (sectionIdx !== -1 && !sectionFilter) {
  console.error('Error: --section requires a substring argument');
  process.exit(1);
}

// ── file read ────────────────────────────────────────────────────────────────

let src;
try {
  src = readFileSync(filePath, 'utf8');
} catch (err) {
  console.error(`Cannot read file: ${filePath}\n${err.message}`);
  process.exit(1);
}

// ── parse ────────────────────────────────────────────────────────────────────

const lines = src.split('\n');

// Regex: optional indent + optional list marker + status box + text
// Matches: "  - [ ] do something"  or  "[ ] do something"  or  "  * [x] done"
const TODO_RE = /^(\s*)(?:[-*]\s*)?\[([ /xX])\]\s+(.+)/;
const HEADING_RE = /^(#{1,6})\s+(.+)/;
const FENCE_RE = /^(`{3,}|~{3,})/;

const todos = [];
let currentHeading = null;
let inFence = false;
let fenceMarker = null;

for (let i = 0; i < lines.length; i++) {
  const line = lines[i];
  const lineNo = i + 1;

  // Track fenced code blocks (must match opening fence marker length/char).
  const fenceMatch = line.match(FENCE_RE);
  if (fenceMatch) {
    if (!inFence) {
      inFence = true;
      fenceMarker = fenceMatch[1][0]; // ` or ~
    } else if (line.trimStart().startsWith(fenceMarker)) {
      inFence = false;
      fenceMarker = null;
    }
    continue;
  }

  if (inFence) continue;

  // Update current heading context.
  const headingMatch = line.match(HEADING_RE);
  if (headingMatch) {
    currentHeading = headingMatch[2].trim();
    continue;
  }

  // Match todo lines.
  const todoMatch = line.match(TODO_RE);
  if (!todoMatch) continue;

  const marker = todoMatch[2].toLowerCase();
  const status = marker === ' ' ? 'pending' : marker === '/' ? 'in-progress' : 'done';
  const text = todoMatch[3].trim();

  todos.push({ line: lineNo, status, text, section: currentHeading });
}

// ── filter ───────────────────────────────────────────────────────────────────

let results = todos;

if (anyFilter) {
  results = results.filter(t =>
    (showPending    && t.status === 'pending') ||
    (showInProgress && t.status === 'in-progress') ||
    (showDone       && t.status === 'done')
  );
}

if (sectionFilter) {
  const sub = sectionFilter.toLowerCase();
  results = results.filter(t => t.section && t.section.toLowerCase().includes(sub));
}

// ── counts ───────────────────────────────────────────────────────────────────

if (countsOnly) {
  const counts = { pending: 0, 'in-progress': 0, done: 0 };
  for (const t of todos) counts[t.status]++;
  if (jsonMode) {
    console.log(JSON.stringify(counts));
  } else {
    console.log(`pending:     ${counts.pending}`);
    console.log(`in-progress: ${counts['in-progress']}`);
    console.log(`done:        ${counts.done}`);
  }
  process.exit(0);
}

// ── output ───────────────────────────────────────────────────────────────────

if (results.length === 0) {
  console.log('no todos found');
  process.exit(0);
}

const STATUS_SYMBOL = { pending: '[ ]', 'in-progress': '[/]', done: '[x]' };

if (jsonMode) {
  console.log(JSON.stringify(results, null, 2));
  process.exit(0);
}

// Align line numbers.
const maxLine = results.reduce((m, t) => Math.max(m, t.line), 0);
const lineWidth = String(maxLine).length;

for (const t of results) {
  const lineLabel = `L${String(t.line).padStart(lineWidth)}`;
  const section   = t.section ? ` [${t.section}]` : '';
  console.log(`${lineLabel}  ${STATUS_SYMBOL[t.status]}  ${t.text}${section}`);
}
