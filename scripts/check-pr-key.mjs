#!/usr/bin/env node
// Verifies that a pull request carries an issue key.
//   title:  Conventional Commit "<type>(<scope>)?!?: <short> (TKM-<n>)"
//   branch: "<prefix>/TKM-<n>-<kebab>", same <n> as the title
// Bot and release branches are skipped.
// Branch flow (docs/RELEASE-FLOW.md): dev -> main is the promotion PR, main -> dev the
// back-merge; both have fixed titles and need no key. Into main only dev, hotfix/TKM-<n>-*
// and release-please branches are accepted.
//
// Usage: node scripts/check-pr-key.mjs --title <t> --branch <b> [--base <b>] [--author <login>]
// --base defaults to dev (the integration branch).

const PREFIXES = ['feature', 'bugfix', 'hotfix', 'refactor', 'chore', 'docs', 'test', 'ci'];
const BRANCH_RE = new RegExp(`^(?:${PREFIXES.join('|')})/TKM-(\\d+)-[a-z0-9]+(?:-[a-z0-9]+)*$`);
const TYPES = [
  'feat', 'fix', 'docs', 'style', 'refactor', 'perf', 'test', 'build', 'ci', 'chore', 'revert',
];
// Conventional Commit: <type>(<scope>)?!?: <short> (TKM-<n>)
const TITLE_RE = new RegExp(
  `^(?:${TYPES.join('|')})(?:\\([a-zA-Z0-9][\\w./-]*\\))?!?: \\S.*? \\(TKM-(\\d+)\\)$`,
);
// The register refresh and release-please branches are opened by automation and have no issue key.
const PROMOTE_TITLE = 'chore(release): promote dev to main';
const BACKMERGE_TITLE = 'chore: back-merge main into dev';
const SKIP_BRANCH_RE = /^(?:(?:dependabot|release)\/|release-please--|register\/refresh$)/;

const args = process.argv.slice(2);
const arg = (flag) => {
  const i = args.indexOf(flag);
  return i >= 0 ? args[i + 1] : undefined;
};

const title = (arg('--title') ?? '').trim();
const branch = (arg('--branch') ?? '').trim();
const author = arg('--author') ?? '';
const base = (arg('--base') ?? 'dev').trim();

const fail = (...problems) => {
  console.error('pr-key: failed');
  for (const p of problems) console.error(`  - ${p}`);
  process.exit(1);
};

if (SKIP_BRANCH_RE.test(branch) || /^dependabot(\[bot\])?$/i.test(author)) {
  console.log(`pr-key: skipped (${branch || author})`);
  process.exit(0);
}

if (branch === 'dev' && base === 'main') {
  if (title !== PROMOTE_TITLE) {
    fail(`promotion PR title must be exactly "${PROMOTE_TITLE}": ${JSON.stringify(title)}`);
  }
  console.log('pr-key: ok (promotion)');
  process.exit(0);
}
if (branch === 'main' && base === 'dev') {
  if (title !== BACKMERGE_TITLE) {
    fail(`back-merge PR title must be exactly "${BACKMERGE_TITLE}": ${JSON.stringify(title)}`);
  }
  console.log('pr-key: ok (back-merge)');
  process.exit(0);
}
if (base === 'main' && !/^hotfix\/TKM-\d+-/.test(branch)) {
  fail(
    `PRs into main must come from dev (title "${PROMOTE_TITLE}"), a hotfix/TKM-<n>-<kebab> branch or a release-please branch; got ${JSON.stringify(branch)}`,
  );
}

const problems = [];
const t = TITLE_RE.exec(title);
const b = BRANCH_RE.exec(branch);
if (!t) {
  problems.push(
    `title must be "<${TYPES.join('|')}>(<scope>)?: <short> (TKM-<n>)": ${JSON.stringify(title)}`,
  );
}
if (!b) {
  problems.push(
    `branch must match "<${PREFIXES.join('|')}>/TKM-<n>-<kebab>": ${JSON.stringify(branch)}`,
  );
}
if (t && b && t[1] !== b[1]) {
  problems.push(`title key TKM-${t[1]} differs from branch key TKM-${b[1]}`);
}

if (problems.length) fail(...problems);
console.log(`pr-key: ok (TKM-${t[1]})`);
