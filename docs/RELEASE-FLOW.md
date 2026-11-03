# Release flow

How a change gets from a pull request to a published release, and where the
release gate sits. Related: [RELEASE-CHECKLIST.md](RELEASE-CHECKLIST.md)
(the manual test), [DECISIONS.md](DECISIONS.md) ("Automatic versioning with
release-please", "Release ownership", "v0.1 without updater artifacts").

## Principles

- `main` is the only long-lived branch. There is no dev or staging branch.
- The gate is the draft release: release-please creates every release as a
  draft, and a draft is never published before its installers have passed
  [RELEASE-CHECKLIST.md](RELEASE-CHECKLIST.md) on macOS and Windows.
- Fix forward: a broken release is fixed by a new patch release. Tags are
  never moved, deleted or recreated, and a published release is never
  replaced.

## Flow

1. **Feature PR -> `main`.** Every change lands on `main` through a PR with
   green CI and a review (merge queue). Conventional Commit types decide
   the next version (`feat` minor, `fix` patch before 1.0; `chore`, `ci`,
   `docs`, `refactor`, `test` alone release nothing).
2. **Release PR.** After every push to `main`, release-please updates one
   open release PR (`chore(main): release x.y.z`) with the version bump in
   all version files and the new `CHANGELOG.md` section. Nobody edits it by
   hand; release-please rebuilds it on every push.
3. **Merge the release PR.** release-please creates the tag `v<version>`
   and a **draft** GitHub release with the changelog as notes
   (`draft: true`, `force-tag-creation: true`).
4. **Build.** `release.yml` runs on the tag (directly, or through the
   `dispatch-release` job when no `RELEASE_PLEASE_TOKEN` is set), builds
   macOS universal and Windows, and uploads the installers to that draft.
   Wait until both build jobs have finished: publishing earlier makes
   `tauri-action` create a second release.
5. **Gate.** The maintainer downloads the draft assets (the four installers
   listed in DECISIONS.md, "Release ownership"), installs them on a Mac and
   a Windows machine and works through
   [RELEASE-CHECKLIST.md](RELEASE-CHECKLIST.md). Notes are reviewed and,
   if needed, edited on the draft.
6. **Publish.** Only when every item passes on both platforms does the
   maintainer publish the draft. It then becomes "latest", which is where
   the README download link points.

If the gate fails, the draft stays unpublished. The fix goes through a
normal `fix` PR, which produces the next patch release PR. The failed draft
may be deleted as a release; its tag stays (fix forward, see below).

## Fix-forward policy

- A defect found in a draft or a published release is fixed on `main` and
  shipped as a new patch release (`x.y.z+1`). There is no rollback.
- Tags are never moved, deleted or recreated. A version number that was
  tagged is spent, even if its draft is never published.
- An unpublished draft whose build or gate failed may be deleted as a
  GitHub release; its tag stays. A published release stays as it is; its
  notes may point to the fix release.
- Rebuilding an existing tag (for example after a CI-only failure) uses
  `release.yml` started from `main` with the `tag` input, never a new tag
  on the same version (DECISIONS.md, "Release ownership").

## Hotfix path

1. Branch `hotfix/TKM-<n>-<short>` from `main`, commit as `fix(...)`.
2. PR -> `main` through the merge queue as usual.
3. Merge the release PR that release-please now proposes (patch bump).
4. Install the draft, run the checklist (at least the affected items plus first start and tray).
5. Publish.

## Updater

Auto-update is off. Releases carry no `.sig` files and no `latest.json`;
`UPDATER_ARTIFACTS` in `release.yml` is `'false'` (DECISIONS.md, "v0.1
without updater artifacts"). Users update by installing the newer release
by hand. The in-app update check is tracked in #41.

## Release PRs and CI

While the release PR is opened and updated with the built-in
`GITHUB_TOKEN`, it gets no CI checks: events created by `GITHUB_TOKEN` do
not start new workflow runs. The release-please-action README says a
`token` must be specified "for your workflows to run on Release Please's
releases and PRs" and recommends a personal access token stored as a
secret. The workflow already reads `secrets.RELEASE_PLEASE_TOKEN` and
falls back to `GITHUB_TOKEN`; the secret is currently not set.

Optional fix: create a fine-grained token with access to this repository
only and store it as the Actions secret `RELEASE_PLEASE_TOKEN`. Then the
release PR runs CI and the tag push starts `release.yml` itself (the
`dispatch-release` job is skipped). No workflow change is needed. Until
then the release PR's gate is a reviewer check of the version and
changelog diff, and the draft gate above covers the built app.

## Options evaluated

Evaluated for TKM-111, not implemented.

**(a) Beta prereleases from `main`** (`prerelease: true`,
`prerelease-type: "beta"`, `versioning: "prerelease"`, graduation via a
`Release-As: x.y.z` commit footer). Verdict: not worth it.

- A dry run of `release-please release-pr --dry-run` against a scratch
  branch with that config proposed `0.3.0 -> 0.4.0-beta`; adding a
  `Release-As: 0.4.0` footer switched it to `0.4.0`.
- Tauri's MSI bundler accepts only a numeric pre-release identifier
  (`x.y.z-1`, at most 65535), so `0.4.0-beta` breaks the Windows `.msi`.
- `prerelease` is a config switch on `main`: every graduation needs a
  config PR plus a `Release-As` commit, and each beta spends a tag.
- Without an updater nobody receives betas automatically; the draft gate
  already gives a tested point before publishing.

**(b) Installer artifacts for a labelled PR** (`pull_request` with
`types: [labeled]`, `tauri-action` without `tagName`/`releaseId`, which
its README documents as build-only, then `actions/upload-artifact`).
Verdict: worth it, filed as idea #115.

- Lets the maintainer test risky platform changes (tray, autostart,
  notifications, window) before they land on `main` and spend a version.
- Opt-in by label, so normal PRs cost nothing; only users with triage
  access can add labels. The build needs no secrets (binaries are unsigned).
- Cost: one workflow that mirrors the `release.yml` build matrix (keep the
  two in step), macOS runner minutes per labelled push, and artifact
  storage (retention 1 to 90 days, set it low).
