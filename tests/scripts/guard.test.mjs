// guard.mjs runs on import, so its commit-message rules are exercised through
// the CLI (--msg <file>). Trailer keys are assembled at runtime so this file
// does not trip the guard's own content scan.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const SCRIPT = fileURLToPath(new URL('../../scripts/guard.mjs', import.meta.url));
const TRAILER = 'Co-' + 'authored-' + 'by';
const BOT = 'github-actions[bot] <41898282+github-actions[bot]@users.noreply.github.com>';

function runMsg(message) {
  const dir = mkdtempSync(path.join(tmpdir(), 'guard-test-'));
  try {
    const file = path.join(dir, 'msg.txt');
    writeFileSync(file, message);
    const r = spawnSync(process.execPath, [SCRIPT, '--msg', file], { encoding: 'utf8', cwd: dir });
    return { code: r.status, out: r.stdout, err: r.stderr };
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

test('passes a plain commit message', () => {
  const r = runMsg('ci(release): rebuild an existing tag (TKM-104)\n');
  assert.equal(r.code, 0, r.err);
});

test('allows the GitHub Actions bot co-author trailer from a squash merge', () => {
  const r = runMsg(`chore(main): release 0.3.0 (#87)\n\n${TRAILER}: ${BOT}\n`);
  assert.equal(r.code, 0, r.err);
});

test('allows the bot trailer with CRLF line endings and trailing spaces', () => {
  const r = runMsg(`chore(main): release 0.3.0 (#87)\r\n\r\n${TRAILER}: ${BOT}  \r\n`);
  assert.equal(r.code, 0, r.err);
});

test('blocks any other co-author trailer', () => {
  const r = runMsg(`feat: thing (TKM-1)\n\n${TRAILER}: Someone <someone@example.com>\n`);
  assert.equal(r.code, 1);
  assert.match(r.err, /co-author trailer/);
});

test('blocks another bot account in the trailer', () => {
  const r = runMsg(`feat: thing (TKM-1)\n\n${TRAILER}: other[bot] <1+other[bot]@users.noreply.github.com>\n`);
  assert.equal(r.code, 1);
  assert.match(r.err, /co-author trailer/);
});

test('still blocks a second trailer next to the allowed one', () => {
  const r = runMsg(
    `chore(main): release 0.3.0 (#87)\n\n${TRAILER}: ${BOT}\n${TRAILER}: Someone <someone@example.com>\n`,
  );
  assert.equal(r.code, 1);
  assert.match(r.err, /co-author trailer/);
});

test('does not allow the bot identity inside a longer line', () => {
  const r = runMsg(`feat: thing (TKM-1)\n\nnote ${TRAILER}: ${BOT}\n`);
  assert.equal(r.code, 1);
});
