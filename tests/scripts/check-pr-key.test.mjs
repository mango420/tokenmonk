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
  const r = ok('feat(ui): add widget (TKM-12)', 'TKM-12');
  assert.match(r.out, /ok \(TKM-12\)/);
});

test('accepts every allowed type', () => {
  const types = [
    'feat', 'fix', 'docs', 'style', 'refactor', 'perf', 'test', 'build', 'ci', 'chore', 'revert',
  ];
  for (const t of types) ok(`${t}: do thing (TKM-1)`, 'TKM-1');
});

test('accepts optional scope forms and breaking marker', () => {
  ok('fix(api/v2): thing (TKM-4)', 'TKM-4');
  ok('fix(my-scope.x): thing (TKM-4)', 'TKM-4');
  ok('feat!: thing (TKM-4)', 'TKM-4');
  ok('feat(ui)!: thing (TKM-4)', 'TKM-4');
});

test('rejects a title without a type', () => {
  bad('add widget (TKM-12)', 'TKM-12', /title must be/);
  bad('(ui): add widget (TKM-12)', 'TKM-12', /title must be/);
});

test('rejects an unknown type', () => {
  bad('feature: add widget (TKM-12)', 'TKM-12', /title must be/);
});

test('rejects an empty scope', () => {
  bad('feat(): add widget (TKM-12)', 'TKM-12', /title must be/);
});

test('rejects a title missing the key or with a malformed key', () => {
  const b = 'TKM-12';
  bad('feat: add widget', b, /title must be/);
  bad('feat: add widget (TKM-)', b, /title must be/);
  bad('feat: add widget (tkm-12)', b, /title must be/);
  bad('feat: add widget TKM-12', b, /title must be/);
  bad('feat: (TKM-12)', b, /title must be/);
});

test('rejects a title with the key not at the end', () => {
  bad('feat: add (TKM-12) widget', 'TKM-12', /title must be/);
});

test('rejects a title with a missing space after the colon', () => {
  bad('feat:add widget (TKM-12)', 'TKM-12', /title must be/);
});

test('rejects anything but the bare issue key as branch', () => {
  const t = 'feat: add widget (TKM-12)';
  bad(t, 'feature/TKM-12-add-widget', /branch must be exactly/);
  bad(t, 'feature/TKM-12', /branch must be exactly/);
  bad(t, 'TKM-12-add-widget', /branch must be exactly/);
  bad(t, 'TKM-12-', /branch must be exactly/);
  bad(t, 'tkm-12', /branch must be exactly/);
  bad(t, 'TKM-', /branch must be exactly/);
  bad(t, 'TKM-12a', /branch must be exactly/);
  bad(t, 'add-widget', /branch must be exactly/);
});

test('rejects a title key that differs from the branch key', () => {
  bad('feat: add widget (TKM-12)', 'TKM-13', /differs from branch key/);
});

test('reports both title and branch problems at once', () => {
  const r = bad('nope', 'nope');
  assert.match(r.err, /title must be/);
  assert.match(r.err, /branch must be exactly/);
});

test('trims surrounding whitespace in title and branch', () => {
  ok('  feat: add widget (TKM-12)  ', '  TKM-12 ');
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

function runBase(title, branch, base, def = 'dev') {
  const r = spawnSync(
    process.execPath,
    [SCRIPT, '--title', title, '--branch', branch, '--base', base, '--default-branch', def],
    { encoding: 'utf8' },
  );
  return { code: r.status, out: r.stdout, err: r.stderr };
}
const okBase = (title, branch, base) => {
  const r = runBase(title, branch, base);
  assert.equal(r.code, 0, `expected pass for ${JSON.stringify([title, branch, base])}: ${r.err}`);
  return r;
};
const badBase = (title, branch, base, pattern) => {
  const r = runBase(title, branch, base);
  assert.equal(r.code, 1, `expected fail for ${JSON.stringify([title, branch, base])}`);
  if (pattern) assert.match(r.err, pattern);
};

test('promotion PR dev -> main needs the exact title and no key', () => {
  okBase('chore(release): promote dev to main', 'dev', 'main');
  badBase('chore(release): promote dev to main (TKM-1)', 'dev', 'main', /promotion PR title/);
  badBase('promote dev', 'dev', 'main', /promotion PR title/);
});

test('back-merge PR main -> dev needs the exact title and no key', () => {
  okBase('chore: back-merge main into dev', 'main', 'dev');
  badBase('chore: back-merge main into dev (TKM-1)', 'main', 'dev', /back-merge PR title/);
  badBase('merge main', 'main', 'dev', /back-merge PR title/);
});

test('into main only dev, fix/revert TKM branches and release-please branches pass', () => {
  okBase('fix!: patch it (TKM-9)', 'TKM-9', 'main');
  badBase('fixup: patch it (TKM-9)', 'TKM-9', 'main', /into main must come from dev/);
  okBase('fix(app): patch it (TKM-9)', 'TKM-9', 'main');
  okBase('revert: undo it (TKM-9)', 'TKM-9', 'main');
  okBase('chore(main): release 0.4.0', 'release-please--branches--main', 'main');
  badBase('feat: add widget (TKM-12)', 'TKM-12', 'main', /into main must come from dev/);
  badBase('chore: x (TKM-12)', 'TKM-12', 'main', /into main must come from dev/);
});

test('hotfix into main keeps the key rules', () => {
  badBase('fix: patch it', 'TKM-9', 'main', /title must be/);
  badBase('fix: patch it (TKM-8)', 'TKM-9', 'main', /differs from branch key/);
});

test('dev or main as head is only special for the matching base', () => {
  badBase('chore(release): promote dev to main', 'dev', 'dev', /branch must be exactly/);
  badBase('chore: back-merge main into dev', 'main', 'main', /into main must come from dev/);
});

test('into dev keeps the key rules and bot skips', () => {
  okBase('feat(ui): add widget (TKM-12)', 'TKM-12', 'dev');
  badBase('feat: add widget', 'TKM-12', 'dev', /title must be/);
  badBase('feat: add widget (TKM-12)', 'wip/x', 'dev', /branch must be exactly/);
  okBase('anything', 'register/refresh', 'dev');
  // Into main the bot skips only count while main is the default branch.
  assert.equal(runBase('anything', 'register/refresh', 'main', 'main').code, 0);
  assert.equal(runBase('anything', 'release/v1', 'main', 'main').code, 0);
  badBase('anything', 'register/refresh', 'main', /into main must come from dev/);
  badBase('anything', 'release/v1', 'main', /into main must come from dev/);
});

test('source rule for main is off while main is the default branch', () => {
  const t = 'feat: add widget (TKM-12)';
  const b = 'TKM-12';
  assert.equal(runBase(t, b, 'main', 'main').code, 0);
  // Without the flag the default is main as well.
  const r = spawnSync(process.execPath, [SCRIPT, '--title', t, '--branch', b, '--base', 'main'], {
    encoding: 'utf8',
  });
  assert.equal(r.status, 0, r.stderr);
  // Key rules still apply.
  assert.equal(runBase('feat: add widget', b, 'main', 'main').code, 1);
});

test('source rule for main is on once dev is the default branch', () => {
  const r = runBase('feat: add widget (TKM-12)', 'TKM-12', 'main', 'dev');
  assert.equal(r.code, 1);
  assert.match(r.err, /into main must come from dev/);
});
