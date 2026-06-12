#!/usr/bin/env node
/**
 * todo — parse status-tagged todo lines from a markdown file.
 *
 * Usage:
 *   node scripts/todo/index.mjs <file.md> [flags]
 *   nub  scripts/todo/index.mjs <file.md> [flags]
 *
 * Status markers:
 *   [ ]  todo / not started
 *   [/]  in progress
 *   [x]  done (case-insensitive)
 *   [-]  cancelled / dropped
 *   [>]  deferred / forwarded
 *   [?]  question / blocked-on-answer
 *
 * Flags:
 *   --pending / --todo      show [ ] items
 *   --in-progress / --wip   show [/] items
 *   --done                  show [x] items
 *   --cancelled / --dropped show [-] items
 *   --deferred              show [>] items
 *   --question / --blocked  show [?] items
 *   --not-done              show the live, actionable set: [ ] + [/] + [?]
 *                           (excludes done, cancelled, and deferred)
 *   --section <substring>   limit to todos under headings matching substring
 *   --counts                print tally only, then exit
 *   --json                  machine-readable JSON output
 *   --help / -h             show this help
 *
 * If no status filter is given, all statuses are shown.
 * Fenced-code-block contents are excluded from parsing.
 */

import { readFileSync } from 'node:fs';

// ── status model ─────────────────────────────────────────────────────────────

// marker char (lowercased) → canonical status name
const MARKER_STATUS = {
  ' ': 'pending',
  '/': 'in-progress',
  'x': 'done',
  '-': 'cancelled',
  '>': 'deferred',
  '?': 'question',
};
const STATUS_SYMBOL = {
  pending: '[ ]',
  'in-progress': '[/]',
  done: '[x]',
  cancelled: '[-]',
  deferred: '[>]',
  question: '[?]',
};
const STATUS_ORDER = ['pending', 'in-progress', 'question', 'deferred', 'done', 'cancelled'];

// ── arg parsing ──────────────────────────────────────────────────────────────

const args = process.argv.slice(2);

if (args.includes('--help') || args.includes('-h') || args.length === 0) {
  console.log(`Usage: node scripts/todo/index.mjs <file.md> [flags]

Status filters (combinable; default = all):
  --pending, --todo       [ ] not started
  --in-progress, --wip    [/] in progress
  --done                  [x] complete
  --cancelled, --dropped  [-] cancelled / dropped
  --deferred              [>] deferred / forwarded
  --question, --blocked   [?] question / blocked-on-answer
  --not-done              live actionable set: [ ] + [/] + [?]

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

const wanted = new Set();
if (args.includes('--pending') || args.includes('--todo')) wanted.add('pending');
if (args.includes('--in-progress') || args.includes('--wip')) wanted.add('in-progress');
if (args.includes('--done')) wanted.add('done');
if (args.includes('--cancelled') || args.includes('--dropped')) wanted.add('cancelled');
if (args.includes('--deferred')) wanted.add('deferred');
if (args.includes('--question') || args.includes('--blocked')) wanted.add('question');
if (args.includes('--not-done')) ['pending', 'in-progress', 'question'].forEach(s => wanted.add(s));
const anyFilter = wanted.size > 0;

const countsOnly = args.includes('--counts');
const jsonMode = args.includes('--json');

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

// optional indent + optional list marker + status box + text.
// Box char class covers all six markers (`-` last so it's literal).
const TODO_RE = /^(\s*)(?:[-*]\s*)?\[([ /xX>?-])\]\s+(.+)/;
const HEADING_RE = /^(#{1,6})\s+(.+)/;
const FENCE_RE = /^(`{3,}|~{3,})/;

const todos = [];
let currentHeading = null;
let inFence = false;
let fenceMarker = null;

for (let i = 0; i < lines.length; i++) {
  const line = lines[i];
  const lineNo = i + 1;

  // Track fenced code blocks (close must match the opening fence char).
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

  // Heading context.
  const headingMatch = line.match(HEADING_RE);
  if (headingMatch) {
    currentHeading = headingMatch[2].trim();
    continue;
  }

  // Todo lines.
  const todoMatch = line.match(TODO_RE);
  if (!todoMatch) continue;

  const status = MARKER_STATUS[todoMatch[2].toLowerCase()];
  const text = todoMatch[3].trim();
  todos.push({ line: lineNo, status, text, section: currentHeading });
}

// ── filter ───────────────────────────────────────────────────────────────────

let results = todos;
if (anyFilter) results = results.filter(t => wanted.has(t.status));
if (sectionFilter) {
  const sub = sectionFilter.toLowerCase();
  results = results.filter(t => t.section && t.section.toLowerCase().includes(sub));
}

// ── counts ───────────────────────────────────────────────────────────────────

if (countsOnly) {
  const counts = Object.fromEntries(STATUS_ORDER.map(s => [s, 0]));
  for (const t of todos) counts[t.status]++;
  if (jsonMode) {
    console.log(JSON.stringify(counts));
  } else {
    const width = Math.max(...STATUS_ORDER.map(s => s.length));
    for (const s of STATUS_ORDER) console.log(`${(s + ':').padEnd(width + 1)} ${counts[s]}`);
  }
  process.exit(0);
}

// ── output ───────────────────────────────────────────────────────────────────

if (results.length === 0) {
  console.log('no todos found');
  process.exit(0);
}

if (jsonMode) {
  console.log(JSON.stringify(results, null, 2));
  process.exit(0);
}

const maxLine = results.reduce((m, t) => Math.max(m, t.line), 0);
const lineWidth = String(maxLine).length;
for (const t of results) {
  const lineLabel = `L${String(t.line).padStart(lineWidth)}`;
  const section = t.section ? ` [${t.section}]` : '';
  console.log(`${lineLabel}  ${STATUS_SYMBOL[t.status]}  ${t.text}${section}`);
}
