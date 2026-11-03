# Release checklist

Run it twice per release: first on the installer from the `dev` test-build
artifact, before the promotion PR, then on the draft release from `main`,
before publishing ([RELEASE-FLOW.md](RELEASE-FLOW.md), "Test build" and
"Draft test").

Manual test of an installed **draft** build, run before the draft is
published ([RELEASE-FLOW.md](RELEASE-FLOW.md)). Work through the list once
on macOS and once on Windows. Every item must pass on both platforms; a
failure keeps the draft unpublished and becomes a `bug` issue.

Copy the list into a comment on the merged release PR, tick per platform,
and note the version under test. Expected behaviour comes from
[DESIGN.md](DESIGN.md) and [ARCHITECTURE.md](ARCHITECTURE.md); when this
list and those documents disagree, the documents win and this list is fixed.

## Before you start

- [ ] The draft release `v<version>` lists the four installers named in
      DECISIONS.md ("Release ownership"): `.dmg`, `.app.tar.gz`,
      `-setup.exe`, `.msi`. No `.sig` files, no `latest.json`.
- [ ] Downloaded from the draft (not built locally), so the OS treats it
      like a user download.
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
- [ ] **macOS:** open the `.dmg`, move the app to Applications. The first
      open is refused because the app is not signed or notarized.
- [ ] **macOS:** after closing the warning, **System Settings > Privacy &
      Security > Security** lists Tokenmonk with **Open Anyway**; confirming
      with the login password opens the app.
- [ ] **macOS (Apple silicon):** the app is not reported as "damaged". If
      it is, the gate fails (the README asks users to open an issue).

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

- [ ] Tray icon: ring track plus centre dot in the mode accent.
- [ ] With limit data: the arc shows 5-hour percent left and drains as the
      window is used.
- [ ] Without limit data: dotted track, no arc.
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
- [ ] Version in the note matches the release under test.
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

- [ ] All items pass on macOS.
- [ ] All items pass on Windows.
- [ ] Release notes on the draft reviewed. Then publish.
