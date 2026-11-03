#!/usr/bin/env node
// Repository hygiene check. Fails when tracked/staged files or commit
// messages contain things that must never reach the repository.
//
// Usage:
//   node scripts/guard.mjs                 same as --all
//   node scripts/guard.mjs --all           all tracked files + full history messages
//   node scripts/guard.mjs --staged        staged files (optionally --msg <file>)
//   node scripts/guard.mjs --msg <file>    a commit message file
//   node scripts/guard.mjs --range a..b    files changed and messages in a range
//   node scripts/guard.mjs --pre-push      read refs from stdin (git pre-push hook)

import { execFileSync } from 'node:child_process';
import { readFileSync, existsSync } from 'node:fs';
import os from 'node:os';

const ZERO = /^0+$/;
const MAX_BYTES = 5 * 1024 * 1024;

function git(args, opts = {}) {
  return execFileSync('git', args, {
    encoding: opts.buffer ? 'buffer' : 'utf8',
    maxBuffer: 256 * 1024 * 1024,
    stdio: ['ignore', 'pipe', 'ignore'],
  });
}

function gitOr(args, fallback) {
  try {
    return git(args).trim();
  } catch {
    return fallback;
  }
}

function lines(s) {
  return s.split(/\r?\n/).filter(Boolean);
}

function escapeRe(s) {
  return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

// ---------- rules ----------

// Paths that must never be tracked. Patterns are tested against the
// repo-relative path with forward slashes.
const PATH_RULES = [
  { re: /(^|\/)\.claude(\/|$)/, why: 'local assistant config' },
  { re: /(^|\/)CLAUDE\.md$/, why: 'local assistant notes' },
  { re: /(^|\/)\.mcp\.json$/, why: 'local tool config' },
  { re: /(^|\/)\.vscode(\/|$)/, why: 'editor config' },
  { re: /(^|\/)\.idea(\/|$)/, why: 'editor config' },
  { re: /\.(db|sqlite|sqlite3)(-wal|-shm|-journal)?$/i, why: 'database file' },
  { re: /(^|\/)\.env(\.|$)/, why: 'env file' },
  {
    re: /\.jsonl$/i,
    why: 'log file outside test fixtures',
    allow: (p) => p.startsWith('src-tauri/tests/fixtures/'),
  },
];

// Phrases are assembled at runtime so this file does not trip itself.
const PHRASES = [
  { re: new RegExp('Co-' + 'Authored-' + 'By', 'i'), why: 'co-author trailer' },
  { re: new RegExp('Claude-' + 'Session', 'i'), why: 'session trailer' },
  { re: new RegExp('Generated ' + 'with'), why: 'generator attribution' },
  { re: new RegExp(String.fromCodePoint(0x1f916), 'u'), why: 'robot emoji' },
  { re: new RegExp('-----BEGIN ' + '[A-Z0-9 ]*' + 'PRIVATE KEY-----'), why: 'private key' },
  { re: new RegExp('sk-' + 'ant-' + '[A-Za-z0-9_-]{8,}'), why: 'API key' },
];

// Commit-message lines that are removed before the message is scanned.
// GitHub appends this trailer when a maintainer squash-merges a PR whose
// commits the Actions bot authored (release-please, register refresh). It names
// the GitHub Actions bot, not an assistant, so it is the one co-author trailer
// allowed. Matched as a whole line; every other co-author trailer stays blocked.
const ALLOWED_MESSAGE_LINES = [
  new RegExp(
    '^' + 'Co-' + 'authored-' + 'by: ' +
      escapeRe('github-actions[bot] <41898282+github-actions[bot]@users.noreply.github.com>') +
      '[ \\t]*$',
    'gim',
  ),
];

function stripAllowedLines(text) {
  return ALLOWED_MESSAGE_LINES.reduce((t, re) => t.replace(re, ''), text);
}

// Placeholder user names that may appear in synthetic example paths.
const PLACEHOLDER_USERS = new Set([
  'example', 'user', 'username', 'you', 'me', 'runner', 'test', 'dev', 'alice', 'bob',
]);

const HOME_PATH = /(?:\b[A-Za-z]:[\\/]{1,2}Users|(?:^|[\s"'`(=,:])\/(?:Users|home))[\\/]{1,2}([^\\/\s"'`<>:*|?,;)]+)/g;

function identityRules() {
  const rules = [];
  const email = gitOr(['config', 'user.email'], '');
  const name = gitOr(['config', 'user.name'], '');
  if (email) rules.push({ re: new RegExp(escapeRe(email), 'i'), why: 'git user.email' });
  if (name && name.length >= 3) {
    rules.push({ re: new RegExp(`\\b${escapeRe(name)}\\b`, 'i'), why: 'git user.name' });
  }
  const home = os.homedir();
  if (home && home.length > 3) {
    for (const v of new Set([home, home.replace(/\\/g, '/'), home.replace(/\\/g, '\\\\')])) {
      rules.push({ re: new RegExp(escapeRe(v), 'i'), why: 'local home path' });
    }
  }
  return rules;
}

const CONTENT_RULES = [...PHRASES, ...identityRules()];

function scanText(text) {
  const hits = [];
  for (const r of CONTENT_RULES) {
    if (r.re.test(text)) hits.push(r.why);
  }
  for (const m of text.matchAll(HOME_PATH)) {
    const user = m[1].toLowerCase();
    if (PLACEHOLDER_USERS.has(user) || /^[$%<{[]/.test(user)) continue;
    hits.push('absolute home path');
    break;
  }
  return hits;
}

function isBinary(buf) {
  const n = Math.min(buf.length, 8000);
  for (let i = 0; i < n; i++) if (buf[i] === 0) return true;
  return false;
}

// ---------- checks ----------

const problems = [];

function checkPath(p) {
  for (const r of PATH_RULES) {
    if (r.re.test(p) && !(r.allow && r.allow(p))) {
      problems.push(`${p}: tracked ${r.why}`);
    }
  }
}

function checkContent(label, buf) {
  if (!buf || buf.length === 0 || buf.length > MAX_BYTES || isBinary(buf)) return;
  for (const why of scanText(buf.toString('utf8'))) problems.push(`${label}: ${why}`);
}

function checkMessage(label, text) {
  for (const why of scanText(stripAllowedLines(text))) problems.push(`commit message ${label}: ${why}`);
}

function readBlob(spec) {
  try {
    return git(['show', spec], { buffer: true });
  } catch {
    return null;
  }
}

function checkAll() {
  for (const p of lines(git(['ls-files', '-z']).replace(/\0/g, '\n'))) {
    checkPath(p);
    if (existsSync(p)) checkContent(p, readFileSync(p));
  }
  if (gitOr(['rev-parse', '--verify', '-q', 'HEAD'], '')) checkCommits(['HEAD']);
}

function checkStaged() {
  const files = lines(git(['diff', '--cached', '--name-only', '-z', '--diff-filter=ACMR']).replace(/\0/g, '\n'));
  for (const p of files) {
    checkPath(p);
    checkContent(p, readBlob(`:${p}`));
  }
}

function checkCommits(revArgs) {
  const shas = lines(gitOr(['rev-list', ...revArgs], ''));
  for (const sha of shas) {
    checkMessage(sha.slice(0, 8), git(['log', '-1', '--format=%B', sha]));
  }
}

function checkRange(range) {
  const [a, b] = range.split('..');
  if (!a || !b) throw new Error(`invalid range: ${range}`);
  const files = lines(git(['diff', '--name-only', '-z', '--diff-filter=ACMR', a, b]).replace(/\0/g, '\n'));
  for (const p of files) {
    checkPath(p);
    checkContent(`${p}@${b.slice(0, 8)}`, readBlob(`${b}:${p}`));
  }
  checkCommits([`${a}..${b}`]);
}

function checkTree(sha) {
  for (const p of lines(git(['ls-tree', '-r', '--name-only', '-z', sha]).replace(/\0/g, '\n'))) {
    checkPath(p);
    checkContent(`${p}@${sha.slice(0, 8)}`, readBlob(`${sha}:${p}`));
  }
}

function checkPrePush() {
  const input = readFileSync(0, 'utf8');
  for (const line of lines(input)) {
    const [, localSha, , remoteSha] = line.split(/\s+/);
    if (!localSha || ZERO.test(localSha)) continue;
    if (!remoteSha || ZERO.test(remoteSha)) {
      // New remote ref: scan the whole tree and every commit not yet on the remote.
      checkTree(localSha);
      checkCommits([localSha, '--not', '--remotes']);
    } else {
      checkRange(`${remoteSha}..${localSha}`);
    }
  }
}

// ---------- main ----------

const args = process.argv.slice(2);
const argValue = (flag) => {
  const i = args.indexOf(flag);
  return i >= 0 ? args[i + 1] : undefined;
};

try {
  let ran = false;
  if (args.includes('--staged')) {
    checkStaged();
    ran = true;
  }
  const msgFile = argValue('--msg');
  if (msgFile) {
    checkMessage('(new)', readFileSync(msgFile, 'utf8').replace(/^#.*$/gm, ''));
    ran = true;
  }
  const range = argValue('--range');
  if (range) {
    checkRange(range);
    ran = true;
  }
  if (args.includes('--pre-push')) {
    checkPrePush();
    ran = true;
  }
  if (args.includes('--all') || !ran) checkAll();
} catch (err) {
  console.error(`guard: ${err.message}`);
  process.exit(2);
}

if (problems.length) {
  console.error('guard: blocked');
  for (const p of [...new Set(problems)]) console.error(`  - ${p}`);
  process.exit(1);
}
console.log('guard: ok');
