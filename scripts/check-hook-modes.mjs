#!/usr/bin/env node
// Fails when a tracked file in .githooks/ is not stored with the executable
// bit (mode 100755). Git silently skips non-executable hooks, so the guard
// would never run locally.
//
// Usage: node scripts/check-hook-modes.mjs

import { execFileSync } from 'node:child_process';

const out = execFileSync('git', ['ls-files', '-s', '--', '.githooks'], { encoding: 'utf8' });
const entries = out
  .split('\n')
  .filter(Boolean)
  .map((line) => {
    const [meta, path] = line.split('\t');
    return { mode: meta.split(' ')[0], path };
  });

if (entries.length === 0) {
  console.error('check-hook-modes: no tracked files in .githooks/');
  process.exit(1);
}

const bad = entries.filter((e) => e.mode !== '100755');
if (bad.length > 0) {
  console.error('check-hook-modes: hooks must be mode 100755 (git update-index --chmod=+x <file>):');
  for (const e of bad) console.error(`  ${e.mode} ${e.path}`);
  process.exit(1);
}
console.log(`check-hook-modes: ${entries.length} hook(s) executable`);
