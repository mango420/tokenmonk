#!/usr/bin/env node
// Verifies that a pull request carries an issue key.
//   title:  ends with "(TKM-<n>)"
//   branch: "<prefix>/TKM-<n>-<kebab>", same <n> as the title
// Bot and release branches are skipped.
//
// Usage: node scripts/check-pr-key.mjs --title <t> --branch <b> [--author <login>]

const PREFIXES = ['feature', 'bugfix', 'hotfix', 'refactor', 'chore', 'docs', 'test', 'ci'];
const BRANCH_RE = new RegExp(`^(?:${PREFIXES.join('|')})/TKM-(\\d+)-[a-z0-9]+(?:-[a-z0-9]+)*$`);
const TITLE_RE = /\(TKM-(\d+)\)$/;
const SKIP_BRANCH_RE = /^(?:dependabot|release)\//;

const args = process.argv.slice(2);
const arg = (flag) => {
  const i = args.indexOf(flag);
  return i >= 0 ? args[i + 1] : undefined;
};

const title = (arg('--title') ?? '').trim();
const branch = (arg('--branch') ?? '').trim();
const author = arg('--author') ?? '';

if (SKIP_BRANCH_RE.test(branch) || /^dependabot(\[bot\])?$/i.test(author)) {
  console.log(`pr-key: skipped (${branch || author})`);
  process.exit(0);
}

const problems = [];
const t = TITLE_RE.exec(title);
const b = BRANCH_RE.exec(branch);
if (!t) problems.push(`title must end with "(TKM-<n>)": ${JSON.stringify(title)}`);
if (!b) {
  problems.push(
    `branch must match "<${PREFIXES.join('|')}>/TKM-<n>-<kebab>": ${JSON.stringify(branch)}`,
  );
}
if (t && b && t[1] !== b[1]) {
  problems.push(`title key TKM-${t[1]} differs from branch key TKM-${b[1]}`);
}

if (problems.length) {
  console.error('pr-key: failed');
  for (const p of problems) console.error(`  - ${p}`);
  process.exit(1);
}
console.log(`pr-key: ok (TKM-${t[1]})`);
