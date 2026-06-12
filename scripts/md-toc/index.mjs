#!/usr/bin/env node
/**
 * md-toc — dump a markdown file's heading structure with exact line ranges.
 *
 * Usage:
 *   node scripts/md-toc/index.mjs <file.md>
 *   nub  scripts/md-toc/index.mjs <file.md>
 *
 * Output (one line per heading, indented by depth):
 *   L7-14   ## How this doc works
 *   L16-92  ## Status board
 *
 * A heading's section runs from its own line to the line just before the next
 * heading of the same or higher level (lower depth number), or EOF.
 * Fenced-code-block `#` lines are NOT picked up — mdast-util-from-markdown
 * parses properly and its AST nodes carry position.start.line / .end.line.
 */

import { readFileSync } from 'node:fs';
import { fromMarkdown } from 'mdast-util-from-markdown';

const filePath = process.argv[2];
if (!filePath) {
  console.error('Usage: node scripts/md-toc/index.mjs <file.md>');
  process.exit(1);
}

let src;
try {
  src = readFileSync(filePath, 'utf8');
} catch (err) {
  console.error(`Cannot read file: ${filePath}\n${err.message}`);
  process.exit(1);
}

const tree = fromMarkdown(src);
const totalLines = src.split('\n').length;

// Collect all headings with their start line and depth.
const headings = [];
for (const node of tree.children) {
  if (node.type === 'heading') {
    headings.push({
      depth: node.depth,
      line: node.position.start.line,
      // Flatten all inline children to plain text.
      text: flattenText(node),
    });
  }
}

// Compute end line for each heading's section.
// A section ends just before the next heading of equal or higher level (lower depth number).
const sections = headings.map((h, i) => {
  let end = totalLines;
  for (let j = i + 1; j < headings.length; j++) {
    if (headings[j].depth <= h.depth) {
      end = headings[j].line - 1;
      break;
    }
  }
  return { ...h, end };
});

// Format and print.
const maxRangeLen = sections.reduce((m, s) => {
  const label = `L${s.line}-${s.end}`;
  return Math.max(m, label.length);
}, 0);

for (const s of sections) {
  const indent = '  '.repeat(s.depth - 1);
  const prefix = '#'.repeat(s.depth);
  const label = `L${s.line}-${s.end}`.padEnd(maxRangeLen);
  console.log(`${label}  ${indent}${prefix} ${s.text}`);
}

/** Recursively flatten all text/inlineCode children to a plain string. */
function flattenText(node) {
  if (node.type === 'text' || node.type === 'inlineCode') return node.value;
  if (node.children) return node.children.map(flattenText).join('');
  return '';
}
