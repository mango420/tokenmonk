# Release flow

How a change gets from a pull request to a published release, and where the
gates sit. Related: [RELEASE-CHECKLIST.md](RELEASE-CHECKLIST.md) (the manual
test), [DECISIONS.md](DECISIONS.md) ("Dev and main branches", "Automatic
versioning with release-please", "Release ownership", "v0.1 without updater
artifacts").

Status: this flow takes effect when step 6 of the [cutover plan](#cutover-plan)
is done. Until then PRs still target `main` as before.

## Principles

- Two long-lived branches. `dev` takes all day-to-day work. `main` only ever
  contains code the maintainer has tested as an installed build.
- Two gates: the `dev` test build is tested before a promotion, the draft
  release is tested before it is published. Both use
  [RELEASE-CHECKLIST.md](RELEASE-CHECKLIST.md).
- release-please runs on `main` only. Nothing on `dev` bumps a version,
  creates a tag or a release.
- Never edit the release files on `dev`: version fields in `package.json`,
  `package-lock.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`,
  `src-tauri/tauri.conf.json`, plus `CHANGELOG.md` and
  `.release-please-manifest.json`. They change only through the release PR
  on `main`.
- Fix forward. Tags are never moved, deleted or recreated, and a published
  release is never replaced.
- Promotions and hotfixes are started by the maintainer only.

## Daily flow

1. Branch `<prefix>/TKM-<n>-<short-kebab>` from `origin/dev`.
2. PR with base `dev`, title `<type>(<scope>): <short> (TKM-<n>)`, body
   ending in `Closes #<n>`. CI, guard and PR-key checks run as on `main`
   today.
3. Squash-merge through the merge queue with the PR title as subject.
   Conventional Commit types still decide the next version once the change
   reaches `main` (`feat` minor, `fix` patch before 1.0; `chore`, `ci`,
   `docs`, `refactor`, `test` alone release nothing).

The feature register refresh PR targets `dev` as well.

## Test build

- A workflow runs on every push to `dev` and on manual dispatch. It builds
  macOS universal and Windows and uploads the installers as workflow
  artifacts. No release, no version bump, no tag, no prerelease version.
- Unsigned, like the releases (they only ever carried updater signatures,
  and test builds switch updater artifacts off). It needs no secrets.
- Artifacts are kept for 1 day (`retention-days: 1`, the platform
  minimum). Standard GitHub-hosted runners are free for public
  repositories.
- Planned as a reusable build (`workflow_call`) shared with `release.yml`,
  so the test build and the release build cannot drift apart.
- The maintainer downloads the artifacts from the run, installs them and
  works through [RELEASE-CHECKLIST.md](RELEASE-CHECKLIST.md) (or the items
  the change touches) on macOS and Windows.
- Caveat for testers: the test installer carries the same version as the
  last release. A Windows `.msi` with an equal version may not install over
  the existing installation, so uninstall first or use the NSIS installer
  (`-setup.exe`).

## Promotion PR

When the test build of `dev` has passed, the maintainer promotes it.

- Head `dev`, base `main`, title `chore(release): promote dev to main`.
- Merged with a **merge commit**, never squashed. A release-please dry-run
  on scratch branches showed: with a merge commit every `feat` and `fix`
  from `dev` keeps its own changelog entry, and the "Merge pull request"
  commit itself is ignored as unparseable. A squash promotion titled
  `chore` gave "No user facing commits", so no release, and the per-change
  granularity was lost.
- **Freeze rule.** Between merging a promotion and merging (or closing) the
  release PR it produces, nothing is merged into `dev`. Reason, verified by
  dry-run: release-please walks the `main` history by committer date and
  stops at the last release commit. A PR squashed into `dev` after
  promotion N, but before release PR N is merged, has an older committer
  date than that release commit and is silently dropped from the next
  release.
- The one exception follows from the same mechanism and was not
  dry-run on its own: a fix for the pending release may land on `dev` if
  it is promoted again before the release PR is merged (see
  [Rollback of a bad promotion](#rollback-of-a-bad-promotion)).
- A promotion with only `docs`, `chore`, `ci` or `test` commits creates no
  release PR, so no freeze is needed.

## Release PR

- After every push to `main`, release-please updates one open release PR
  (`chore(main): release x.y.z`, branch `release-please--*`) with the
  version bump in all version files and the new `CHANGELOG.md` section.
  Nobody edits it by hand; release-please rebuilds it on every push.
- Merge it through the merge queue, **squash** with the PR title as
  subject, as today. It has no TKM key; the PR-key check skips
  release-please branches.
- Merging it makes release-please create the tag `v<version>` and a
  **draft** GitHub release with the changelog as notes. `release.yml` runs
  on the tag (directly, or through the `dispatch-release` job when no
  `RELEASE_PLEASE_TOKEN` is set), builds macOS universal and Windows and
  uploads the installers to that draft.
- CI on the release PR: events created by the built-in `GITHUB_TOKEN` start
  no workflow runs, so without the secret `RELEASE_PLEASE_TOKEN` the release
  PR has no CI checks. Its gate is then a reviewer check of the version and
  changelog diff. Setting the secret (fine-grained token, this repository
  only) fixes that without a workflow change.

## Draft test

1. Wait until both `release.yml` build jobs have finished. Publishing
   earlier makes `tauri-action` create a second release.
2. Check the draft lists the four installers named in DECISIONS.md
   ("Release ownership").
3. Download them from the draft, install on a Mac and a Windows machine
   and work through [RELEASE-CHECKLIST.md](RELEASE-CHECKLIST.md). Review and,
   if needed, edit the notes on the draft.

If the draft test fails, the draft stays unpublished and the fix goes
forward (see [Fix-forward](#fix-forward)).

## Publish

Only when every item passes on both platforms does the maintainer publish
the draft by hand. It then becomes "latest", which is where the README
download link points.

Auto-update is on: releases carry `.sig` files and `latest.json`
(`UPDATER_ARTIFACTS` is `'true'` in `release.yml`). Check `latest.json` on
the draft before publishing; the in-app update check reads it from the
latest published release.

## Back-merge

After each stable release, `main` is merged back into `dev` so `dev` has
the new version files and `CHANGELOG.md`.

```bash
gh pr create --base dev --head main --title "chore: back-merge main into dev" --body "Back-merge after the stable release. Merge with a merge commit."
```

Merge it with a merge commit (`gh pr merge <n> --merge`), never squash.

- PR head `main`, base `dev`, title `chore: back-merge main into dev`.
- Merged with a **merge commit**. A squash back-merge causes add/add
  conflicts on later promotions.
- Dry-run result: no conflicts, because `dev` never edits the release
  files, and the next promotion counts nothing twice. The back-merge
  triggers no release (release-please runs on `main` only).

## Hotfix

Maintainer-triggered only, for a defect that cannot wait for the next
promotion.

1. Branch `hotfix/TKM-<n>-<short>` from `origin/main`, commit as `fix(...)`.
2. PR with base `main`, squash, normal title
   `fix(<scope>): <short> (TKM-<n>)`.
3. Merge the release PR that release-please now proposes (patch bump).
4. Install the draft, run the checklist (at least the affected items plus
   first start and tray), publish.
5. Back-merge `main` into `dev` as above.

A hotfix while a promotion's release PR is still open is accepted. The
patch release then also ships the promoted content. That content was
already tested as the `dev` test installer before the promotion (`main`
only holds tested code), and the draft is tested as usual before
publishing.

## Fix-forward

- A defect found in a draft or a published release is fixed through a
  normal `fix` PR into `dev`, tested in the test build, promoted, and
  shipped as the next patch release. Urgent cases use the hotfix path.
- Tags are never moved, deleted or recreated. A version number that was
  tagged is spent, even if its draft is never published.
- An unpublished draft whose build or test failed may be deleted as a
  GitHub release; its tag stays. A published release stays as it is; its
  notes may point to the fix release.
- Rebuilding an existing tag (for example after a CI-only failure) uses
  `release.yml` started from `main` with the `tag` input, never a new tag
  on the same version (DECISIONS.md, "Release ownership").

## Rollback of a bad promotion

A promotion turns out bad when the release PR or its draft shows a problem.

- Preferred: fix forward on `dev` and promote again while the release PR
  stays unmerged. release-please updates the release PR; users are
  unaffected until a draft is published.
- If `main` itself must be reverted: branch
  `hotfix/TKM-<n>-revert-promotion` from `origin/main`, run
  `git revert -m 1 <merge commit>` of the promotion, and open a PR into
  `main` titled `revert: <short> (TKM-<n>)`, squash-merged like a hotfix.
- Warning: the next back-merge carries the revert into `dev` as well, so
  those changes are then gone from `dev` too. To bring them back, revert
  the revert on `dev` after that back-merge (normal PR into `dev`), test it
  through the test build and promote again. Until then `dev` lacks those
  changes. Without the revert of the revert, a later promotion does not
  bring them back to `main`.
- Tags never move. A bad published release is fixed by the next patch
  release.

## Cutover plan

Ordered; each step starts only when the previous one is done.

1. This docs PR, merged into `main` (TKM-140).
2. No open PRs into `main` except the release-please and register refresh
   PRs.
3. Create `dev` from `main` and push it.
4. Workflow PRs into `main` (devops):
   - CI, guard and PR-key checks run for PRs into `dev` and into `main`.
   - PR-key rules for promotion and back-merge PRs: skip the key regex for
     heads `dev` and `main`, check their fixed titles, and enforce that PRs
     into `main` come only from `dev`, `hotfix/*` or `release-please--*`
     branches (rulesets cannot restrict the source branch).
   - The feature register refresh PR targets `dev`.
   - The test-installer workflow on push to `dev` and manual dispatch,
     using a reusable build shared with `release.yml`.
   - `release.yml` and `release-please.yml` stay `main`-only.
5. Fast-forward `dev` to `main` (`dev` has no commits of its own yet), so
   `dev` carries the new CI, PR-key and test-build workflows before the
   rehearsal.
6. Maintainer, by hand in the repository settings:
   - Allow merge commits and squash merges; rebase merging off.
   - Ruleset `dev`: PR required, status checks required, allowed merge
     methods squash and merge (merge only for back-merge PRs), no
     linear-history rule (the back-merge needs a merge commit).
   - Ruleset `main`: PR required, status checks required, allowed merge
     methods merge and squash, no linear-history rule.
   - Delete or replace the existing ruleset `protect-main`: it targets
     `~DEFAULT_BRANCH` and requires linear history, so it must be replaced
     by the explicit rulesets above before the default branch changes.
   - Set the default branch to `dev`.
7. Contributors switch their PR base to `dev` (branch from `origin/dev`).
8. Rehearsal:
   - A docs-only PR into `dev`; the test build runs.
   - Promotion PR `dev` -> `main`, merged with a merge commit; confirm no
     release PR is created.
   - A back-merge is not needed (no release), but run it once to prove the
     checks pass.
9. QA runs the full Definition of Done on `dev`.
