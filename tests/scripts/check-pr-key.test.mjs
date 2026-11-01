// check-pr-key.mjs exports nothing and runs on import, so its title and branch
// rules are exercised through the CLI (exit code and output).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const SCRIPT = fileURLToPath(new URL('../../scripts/check-pr-key.mjs', import.meta.url));

function run(title, branch, author) {
  const args = [SCRIPT, '--title', title, '--branch', branch];
  if (author !== undefined) args.push('--author', author);
  const r = spawnSync(process.execPath, args, { encoding: 'utf8' });
  return { code: r.status, out: r.stdout, err: r.stderr };
}

const ok = (title, branch, author) => {
  const r = run(title, branch, author);
  assert.equal(r.code, 0, `expected pass for ${JSON.stringify([title, branch])}: ${r.err}`);
  return r;
};
const bad = (title, branch, pattern) => {
  const r = run(title, branch);
  assert.equal(r.code, 1, `expected fail for ${JSON.stringify([title, branch])}`);
  if (pattern) assert.match(r.err, pattern);
  return r;
};

test('accepts a well-formed title and branch', () => {
  const r = ok('feat(ui): add widget (TKM-12)', 'feature/TKM-12-add-widget');
  assert.match(r.out, /ok \(TKM-12\)/);
});

test('accepts every allowed type and branch prefix', () => {
  const types = [
    'feat', 'fix', 'docs', 'style', 'refactor', 'perf', 'test', 'build', 'ci', 'chore', 'revert',
  ];
  for (const t of types) ok(`${t}: do thing (TKM-1)`, 'chore/TKM-1-do-thing');
  const prefixes = ['feature', 'bugfix', 'hotfix', 'refactor', 'chore', 'docs', 'test', 'ci'];
  for (const p of prefixes) ok('fix: do thing (TKM-1)', `${p}/TKM-1-do-thing`);
});

test('accepts optional scope forms and breaking marker', () => {
  ok('fix(api/v2): thing (TKM-4)', 'bugfix/TKM-4-thing');
  ok('fix(my-scope.x): thing (TKM-4)', 'bugfix/TKM-4-thing');
  ok('feat!: thing (TKM-4)', 'feature/TKM-4-thing');
  ok('feat(ui)!: thing (TKM-4)', 'feature/TKM-4-thing');
});

test('rejects a title without a type', () => {
  bad('add widget (TKM-12)', 'feature/TKM-12-add-widget', /title must be/);
  bad('(ui): add widget (TKM-12)', 'feature/TKM-12-add-widget', /title must be/);
});

test('rejects an unknown type', () => {
  bad('feature: add widget (TKM-12)', 'feature/TKM-12-add-widget', /title must be/);
});

test('rejects an empty scope', () => {
  bad('feat(): add widget (TKM-12)', 'feature/TKM-12-add-widget', /title must be/);
});

test('rejects a title missing the key or with a malformed key', () => {
  const b = 'feature/TKM-12-add-widget';
  bad('feat: add widget', b, /title must be/);
  bad('feat: add widget (TKM-)', b, /title must be/);
  bad('feat: add widget (tkm-12)', b, /title must be/);
  bad('feat: add widget TKM-12', b, /title must be/);
  bad('feat: (TKM-12)', b, /title must be/);
});

test('rejects a title with the key not at the end', () => {
  bad('feat: add (TKM-12) widget', 'feature/TKM-12-add-widget', /title must be/);
});

test('rejects a title with a missing space after the colon', () => {
  bad('feat:add widget (TKM-12)', 'feature/TKM-12-add-widget', /title must be/);
});

test('rejects a branch with a wrong prefix, missing key or bad slug', () => {
  const t = 'feat: add widget (TKM-12)';
  bad(t, 'wip/TKM-12-add-widget', /branch must match/);
  bad(t, 'feature/add-widget', /branch must match/);
  bad(t, 'feature/TKM-12', /branch must match/);
  bad(t, 'feature/TKM-12-', /branch must match/);
  bad(t, 'feature/TKM-12-Add-Widget', /branch must match/);
  bad(t, 'feature/TKM-12-add_widget', /branch must match/);
  bad(t, 'feature/TKM-12-add--widget', /branch must match/);
  bad(t, 'feature/tkm-12-add-widget', /branch must match/);
  bad(t, 'TKM-12-add-widget', /branch must match/);
});

test('rejects a title key that differs from the branch key', () => {
  bad('feat: add widget (TKM-12)', 'feature/TKM-13-add-widget', /differs from branch key/);
});

test('reports both title and branch problems at once', () => {
  const r = bad('nope', 'nope');
  assert.match(r.err, /title must be/);
  assert.match(r.err, /branch must match/);
});

test('trims surrounding whitespace in title and branch', () => {
  ok('  feat: add widget (TKM-12)  ', '  feature/TKM-12-add-widget ');
});

test('skips register refresh, release and dependabot branches', () => {
  for (const b of ['register/refresh', 'release/v0.1.0', 'dependabot/npm_and_yarn/foo-1.2.3']) {
    const r = ok('anything goes', b);
    assert.match(r.out, /skipped/);
  }
});

test('does not skip near-miss branches', () => {
  bad('anything goes', 'register/refresh-2');
  bad('anything goes', 'register/other');
  bad('anything goes', 'releases/v1');
  bad('anything goes', 'xrelease/v1');
});

test('skips dependabot authors regardless of branch', () => {
  const r = ok('Bump foo', 'some-branch', 'dependabot[bot]');
  assert.match(r.out, /skipped/);
  ok('Bump foo', 'some-branch', 'Dependabot');
});

test('does not skip other authors', () => {
  const r = run('Bump foo', 'some-branch', 'someone');
  assert.equal(r.code, 1);
});
