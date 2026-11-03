// check-hook-modes.mjs inspects the git index of the current working directory,
// so each case runs the script inside a throwaway repo. Index entries are
// written with `update-index --cacheinfo`, which sets the mode independently
// of the filesystem (works the same on Windows).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const SCRIPT = fileURLToPath(new URL('../../scripts/check-hook-modes.mjs', import.meta.url));
const REPO_ROOT = fileURLToPath(new URL('../..', import.meta.url));

function git(cwd, args, input) {
  const r = spawnSync('git', args, { cwd, encoding: 'utf8', input });
  assert.equal(r.status, 0, `git ${args.join(' ')} failed: ${r.stderr}`);
  return r.stdout.trim();
}

function runScript(cwd) {
  const r = spawnSync(process.execPath, [SCRIPT], { encoding: 'utf8', cwd });
  return { code: r.status, out: r.stdout, err: r.stderr };
}

// entries: [{ path, mode }] staged under a fresh temp repo.
function withRepo(entries, fn) {
  const dir = mkdtempSync(path.join(tmpdir(), 'hook-modes-test-'));
  try {
    git(dir, ['init', '-q']);
    for (const { path: file, mode } of entries) {
      const sha = git(dir, ['hash-object', '-w', '--stdin'], '#!/bin/sh\nexit 0\n');
      git(dir, ['update-index', '--add', '--cacheinfo', `${mode},${sha},${file}`]);
    }
    return fn(dir);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

test('passes on the hooks tracked in this repository', () => {
  const r = runScript(REPO_ROOT);
  assert.equal(r.code, 0, r.err);
  assert.match(r.out, /hook\(s\) executable/);
});

test('passes when every hook is mode 100755', () => {
  const r = withRepo(
    [
      { path: '.githooks/pre-demo', mode: '100755' },
      { path: '.githooks/post-demo', mode: '100755' },
    ],
    runScript,
  );
  assert.equal(r.code, 0, r.err);
  assert.match(r.out, /2 hook\(s\) executable/);
});

test('fails and names a hook that lost its executable bit (100644)', () => {
  const r = withRepo(
    [
      { path: '.githooks/pre-demo', mode: '100755' },
      { path: '.githooks/post-demo', mode: '100644' },
    ],
    runScript,
  );
  assert.notEqual(r.code, 0);
  assert.match(r.err, /100644 \.githooks\/post-demo/);
  assert.doesNotMatch(r.err, /pre-demo/);
});

test('fails when .githooks/ has no tracked files', () => {
  const r = withRepo([{ path: 'README.md', mode: '100644' }], runScript);
  assert.notEqual(r.code, 0);
  assert.match(r.err, /no tracked files/);
});

test('ignores files outside .githooks/', () => {
  const r = withRepo(
    [
      { path: '.githooks/pre-demo', mode: '100755' },
      { path: 'docs/notes.md', mode: '100644' },
    ],
    runScript,
  );
  assert.equal(r.code, 0, r.err);
});
