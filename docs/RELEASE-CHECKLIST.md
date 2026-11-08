# Release checklist

Manual test of an installed build. The maintainer runs it once per release,
on Windows, on the installer from the `dev` test-build artifact, before the
promotion PR ([RELEASE-FLOW.md](RELEASE-FLOW.md), "Test build"). That run is
the release gate: every Windows item must pass; a failure blocks the
promotion and becomes a `bug` issue. The draft release gets no install
test ([RELEASE-FLOW.md](RELEASE-FLOW.md), "Draft check").

macOS is not part of the gate and ships untested by the maintainer. The
community can use this list on macOS after publishing: the items without a
platform plus those marked "macOS (community)". Those marked items are not
part of the maintainer gate. macOS reports become `bug` issues and are fixed
forward.

Copy the list into a comment on the promotion PR, tick the Windows items,
and note the `dev` head commit under test; the promotion must merge that
commit. Expected behaviour comes from [DESIGN.md](DESIGN.md) and
[ARCHITECTURE.md](ARCHITECTURE.md); when this list and those documents
disagree, the documents win and this list is fixed.

## Before you start

- [ ] The Windows installer comes from the artifacts of the test-build run
      for the `dev` head commit under test (not built locally), so the OS
      treats it like a user download.
- [ ] Test machine has Claude Code logs with some history, and no earlier
      Tokenmonk data (fresh user account, or the previous install and its
      app data removed), so first start and retroactive XP are covered.

## Install and unsigned-binary warnings

As documented in the README, "Install":

- [ ] **Windows:** Microsoft Defender SmartScreen shows the blue dialog
      "Windows protected your PC". **More info** (if shown), then **Run
      anyway** starts the installer. Install with the `-setup.exe`; the
      `.msi` installs as well (one of them per test run is enough, alternate
      between releases).
- [ ] **Windows:** if Smart App Control is on and blocks the app without
      that option, note it; the README already says so.
- [ ] **macOS (community):** open the `.dmg`, move the app to
      Applications. The first open is refused because the app is not signed
      or notarized.
- [ ] **macOS (community):** after closing the warning, **System Settings >
      Privacy & Security > Security** lists Tokenmonk with **Open Anyway**;
      confirming with the login password opens the app.
- [ ] **macOS (community, Apple silicon):** the app is not reported as
      "damaged". If it is, open a `bug` issue (the README asks users to do
      so).

## First start and onboarding

- [ ] The tray icon appears and the popup opens by itself with onboarding
      ("How do you want to vibe?"), accent saffron.
- [ ] Quitting and restarting before choosing a mode opens onboarding again.
- [ ] Choosing a mode switches to the dashboard; the accent matches the
      mode (saffron "Max it out", jade "Do it yourself").
- [ ] The first import shows its importing state, then real numbers.

## Retroactive XP

- [ ] After onboarding, the stored history is replayed once: level, XP and
      past achievements appear without a toast storm (at most one summary).
- [ ] No OS notification for retroactive unlocks.
- [ ] A restart does not replay again: XP and unlocks are unchanged.

## Tray and halo

- [ ] Tray icon (Windows): grey ring track, monk head and sleeve grey; arc
      and robe in the mode accent.
- [ ] Tray icon, macOS (community): black template image tinted by the
      menu bar; track at 40 %, arc and monk solid.
- [ ] With limit data: the arc shows 5-hour percent left and drains as the
      window is used.
- [ ] Without limit data: dotted track, no arc, the monk faded.
- [ ] Right click opens the tray menu: "Open Tokenmonk" opens the popup,
      "Settings" opens Settings, "Quit" ends the app.
- [ ] Icon is legible in light and dark system themes.

## Popup

- [ ] Left click on the tray icon toggles the popup; it can be dragged.
- [ ] Dashboard numbers update live while Claude Code is in use; numbers
      are monospace and do not jitter.
- [ ] "5 hours" / "Week" segment switches the ring.
- [ ] With limit data: footer note says "Limit % includes claude.ai chats.
      Token stats are Claude Code only."
- [ ] Light and dark theme follow the OS.

## Limit fallback

- [ ] Without the status-line bridge set up: the app runs on token data
      alone, explains that limit data is missing, and the dashboard note
      links to Settings > "Limit data". No error state.
- [ ] Settings > "Limit data" shows the setup snippet for this platform.
- [ ] After pasting the snippet into Claude Code settings and using Claude
      Code, limit percentages appear and the tray arc is drawn.
- [ ] Removing the bridge again: once its windows reset, the app falls back
      to the no-limit view without crashing.

## Settings

- [ ] Opens from the dashboard and from the tray menu; "Back" and Esc
      return.
- [ ] Switching mode applies at once and changes the accent; no save
      button.
- [ ] "Sound" on: a tone plays on the next unlock or level-up; off: silent.
- [ ] Version in the note matches `package.json` of the commit under test
      (test builds keep the version of the last release).
- [ ] Settings survive a restart.

## Achievements

- [ ] The achievements view opens from the dashboard level block and lists
      the five achievements; the retired "Halloween 2026" badge shows only
      if it was already unlocked.
- [ ] A live unlock (for example First Blood on a fresh profile, if not
      already unlocked retroactively) shows one toast and adds its XP.
- [ ] A level-up shows the level-up screen; toasts wait for it.

## Autostart

- [ ] "Start at login" on: after logging out and in (or a reboot) Tokenmonk
      is running in the tray.
- [ ] "Start at login" off: after logging out and in it does not start.

## Notifications

- [ ] With the popup hidden, a live unlock sends an OS notification
      ("<name> unlocked, +<xp> XP") and the toast still shows on the next
      open.
- [ ] Limit or budget warnings, if the conditions occur during the test:
      one notification per upward crossing, none for stale values.
- [ ] Level-ups send no OS notification.

## Done

- [ ] All Windows items pass on the test build of the `dev` head under
      test. This is the gate; then promote ([RELEASE-FLOW.md](RELEASE-FLOW.md),
      "Promotion PR").

Not part of the gate: macOS is checked by the community after publishing;
reports become `bug` issues and are fixed forward. Draft check and
publishing follow [RELEASE-FLOW.md](RELEASE-FLOW.md), "Draft check" and
"Publish".
