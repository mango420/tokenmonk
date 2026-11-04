# Design

The design system for the Tokenmonk popup, tray icon and README. All values
live as CSS variables in [`src/styles/tokens.css`](../src/styles/tokens.css);
components use the variables, never literal values. Brand files are in
[`assets/brand/`](../assets/brand/). The app icon is the monk without a
ring on an ink rounded-square body (see "App icon"). It is the only brand
mark; the ring appears only in the tray gauge, around the mono monk.

Source: the refreshed popup prototype (onboarding, dashboard, settings,
achievements, toast, tray icon). Where an older rule and the prototype
disagree, the prototype wins. States the prototype does not show are
derived below from its components and tokens; such copy is marked
"derived".

## Personality and voice

The monk is calm, a little smug and secretly competitive. He never panics,
even with 1 % left.

- **Short and dry.** "Getting thin. Slow down.", not "Warning! You are
  about to run out!"
- **Gamer wink, not cringe.** Titles and achievement names (*Limit
  Chaser*, *Rate-Limit Rambo*) carry the jokes; the rest stays plain.
- **Mode-aware.** Max it out talks like a coach ("Make it count."), Do it
  yourself like a friend with a walking habit ("Go for a walk.").
- **Number first.** The dashboard leads with one big number: percent left.
- English UI copy, real typographic quotes and apostrophes (’), no em
  dashes, no emoji. Separators in running copy are commas ("Level 7, 340
  of 500 XP"), not dots.
- Numbers: in running text percent is written with a space (`72 %`) and
  thousands with a comma (`184,230`). Stat values are compact (see
  "Number formats").

## Modes

| Mode | Accent | Name | Description |
|---|---|---|---|
| Max it out | `--saffron` #E39A0B | "Max it out" | "Use the whole limit, every window." |
| Do it yourself | `--jade` #1F8A64 | "Do it yourself" | "Use less. Keep it calm." |

`--accent` is switched by `data-mode` on `<html>` (`max` or `diy`). Before
onboarding there is no `data-mode` and the accent is saffron (brand). The
accent is never the only signal: the mode always comes with its name,
title or copy. The accent is used for fills and rules only (ring arcs,
level bar, selected segment underline, selected settings row, switch on,
toast edge, focus ring), never for text.

## Colour

Light and dark both ship. The theme follows the OS
(`prefers-color-scheme`); `data-theme="light"` or `"dark"` on `<html>`
forces one.

| Token | Light | Dark | Use |
|---|---|---|---|
| `page` | #CBCAC4 | #131715 | Backdrop outside the popup card |
| `pop` | #E4E3DD | #1D2220 | Popup background; toast text; switch knob |
| `ink` | #181A1B | #E8E7E0 | Primary text and numbers; toast background; ring track (at 22 %) |
| `ink-2` | #5A5E5C | #98A09B | Labels, captions, units, links, secondary numbers |
| `line` | #BAB9B1 | #363E3A | Hairlines, bar and switch tracks, popup outline |
| `saffron` | #E39A0B | #E39A0B | Max it out accent, brand |
| `jade` | #1F8A64 | #1F8A64 | Do it yourself accent |

`shadow-popup`: a 1 px `line` outline plus `0 28px 60px -24px` black at
40 %. No gradients, no neon glow, no purple, no danger colour: the ring
never turns red; the copy carries the urgency.

## Typography

Fonts are bundled from `@fontsource/schibsted-grotesk` (weights 400, 500,
700) and `@fontsource/martian-mono` (weights 300, 400, 500); no CDN, works
offline.

- `--font-sans` Schibsted Grotesk: all UI text. Body 14 px, line height
  1.4, weight 400; emphasis is weight 500.
- `--font-mono` Martian Mono: every number that is data (percent, token
  counts, XP, durations, times) with `font-variant-numeric: tabular-nums`.

| Style | Family, weight | Size / tracking | Use |
|---|---|---|---|
| hero | mono 300 | 70, -0.05em, line 1 | Ring centre: percent left |
| hero-sm | mono 300 | 46, -0.05em, line 1 | Ring centre without limit data: tokens today |
| hero-unit | mono 300 | 22, `ink-2`, 2 px left gap | The `%` after the hero |
| onboard-title | sans 500 | 32, -0.025em, line 1.08 | "How do you want to vibe?" |
| ach-title | sans 500 | 28, -0.02em | Title on the achievements screen |
| title | sans 500 | 22, -0.02em | "Settings" |
| choice-lg | sans 500 | 21, -0.015em | Onboarding choice name |
| status | sans 500 | 17, -0.01em | Status line under the ring |
| stat | mono 400 | 26, -0.03em, line 1.2 | Stat values |
| choice | sans 500 | 15.5 | Settings mode row name |
| brand | sans 500 | 15, -0.01em | "Tokenmonk" wordmark |
| row | sans 500 | 15 | Level title, achievement name |
| toast-title | sans 500 | 14.5 | Toast name |
| caption | sans 400 | 13, `ink-2` | Ring caption, stat labels, descriptions, section labels |
| mono-sm | mono 400 | 12.5, `ink-2` | Reset line, level line on the achievements screen |
| small | sans or mono 400 | 12, `ink-2` | Stat units, "Level 7", XP figure (mono), achievement state |
| note | sans 400 | 11.5, `ink-2` | Footer notes |

### Number formats

- Percent left: integer, `max(0, round(100 - utilization))`.
- Compact token values (stats, hero-sm): below 1,000 as is; below
  1,000,000 one decimal plus `k` ("4.2k"); above that one decimal plus `M`
  ("1.2M").
- Reset durations: under 24 h "Hh Mm" ("2h 14m", "0h 41m"); from 24 h
  "Dd Hh" ("3d 4h"). Prefixed "Resets in ".
- XP: full numbers with thousands commas ("1,240 / 1,850 XP").

## Spacing, radius, motion

- Spacing: `space-1` 4, `space-2` 8, `space-3` 12, `space-4` 16, `space-5`
  20, `space-6` 24. Odd values from the prototype (14, 18, 22, 30) are
  written as named layout values in this document and live in the
  component CSS as sums of tokens or as tokens where they repeat.
- Radius: `radius-popup` 18, `radius-switch` 12, `radius-toast` 8,
  `radius-focus` 4, `radius-bar` 2. Rows and choices have no radius and no
  fill; they are separated by 1 px `line` hairlines on top.
- Focus: `:focus-visible` outline 2 px `--accent`, offset 3, radius 4.
- Motion, allowed and nothing else:
  - Ring draw: the arcs draw from 0 to their length in 1.1 s,
    `cubic-bezier(.3,.7,.2,1)`, the first time the dashboard renders after
    start. Later value changes jump.
  - Toast slide: up from below the popup edge (`translateY(140%)` to 0)
    in 280 ms, `cubic-bezier(.2,.8,.2,1)`, and back down when it leaves.
  - Switch knob: slides 18 px in 150 ms.
  - With `prefers-reduced-motion: reduce` all three are off: the ring
    shows its final length, the toast and knob appear in place.

Sound: one short synthesized tone per unlock or level-up (generated in the
UI, no audio file). It plays only when `Settings.sound` is on, once per
event, whether the popup is open or hidden.

## Popup layout (360 x 680)

The popup is 360 x 680 px, radius 18, background `pop`, `shadow-popup`,
overflow hidden. Each screen is a column with padding 20 (top), 24 (sides),
18 (bottom). There are no cards and no icons; actions are text links
(`ink-2`, hover `ink`, padding 6 px vertical) or full-width rows.

### Shared parts

- **Top row**: min height 32, items centred, space-between. Brand on the
  left: the app icon (`assets/brand/tokenmonk-icon.svg`, unchanged file,
  body included, its own colours in both themes and both modes) at 26 px,
  gap 10, "Tokenmonk" (brand style). On sub-screens the top row holds only
  the text link "Back" (Esc does the same).
- **Onboarding mark** (onboarding mode choice): the app icon
  (`assets/brand/tokenmonk-icon.svg`, same file as the top row, body
  included, its own colours) at 52 px. The former inline ring brand mark
  is gone.
- **Monk mark** (level-up screen): `assets/brand/tokenmonk-mark-mono.svg`
  at 52 px, filled in `--accent` of the current mode, no body, no ring.
- **Level bar**: track `line`, fill `--accent`, radius 2, height 3
  (dashboard) or 4 (achievements screen), width = `GameState.progress`.
- **Hairline row**: full width, 1 px `line` on top, no fill.
- **Note**: note style, min height 16.

### Dashboard

Top to bottom:

1. **Top row**: brand left; text link "Settings" right.
2. **Segment** (margin top 16, gap 20): two text buttons "5 hours" and
   "Week", weight 500, padding 4 0, 2 px bottom rule. Selected: `ink` text,
   `--accent` rule (`aria-pressed`). Unselected: `ink-2`, no rule. The
   selection is local UI state, starts at "5 hours" and is not saved.
   Without limit data the segment is hidden and its 30 px height kept.
3. **Ring** (248 x 248, centred, margin top 8):
   - Track: circle r 104, stroke `ink` at 22 %, width 2, round caps,
     dotted (`stroke-dasharray 1.5 7`).
   - Main arc: r 104, `--accent`, width 14, round caps, from 12 o'clock
     clockwise, length = percent left of the selected window. It drains as
     the window is used.
   - Inner arc: r 96, `--accent` at 35 %, width 4, round caps, length =
     0.93 x the main arc.
   - Both arcs share a hand-drawn wobble: an SVG filter (region -10 %,
     120 %) of `feTurbulence` `fractalNoise`, base frequency 0.045, 2
     octaves, seed 4, into `feDisplacementMap` scale 5. The track is not
     filtered.
   - Centre: hero number plus `%` (hero-unit), caption "left" (margin top
     6) below.
4. **Status line** (status style, centred, margin top 8), from percent
   left of the selected window and the mode:

   | Percent left | Max it out | Do it yourself |
   |---|---|---|
   | 60 and more | "Barely touched. Use it." | "Light day. Nicely done." |
   | 30 to 59 | "Good pace." | "Mid-range. Breathe." |
   | 10 to 29 | "Nearly there. As planned." | "Getting thin. Slow down." |
   | below 10 | "Almost gone. Make it count." | "Nearly out. Go for a walk." |

5. **Reset line** (mono-sm, centred, margin top 4, min height 18): "Resets
   in 2h 14m" from the selected window's `resetsAt`; empty when it is
   `null`.
6. **Stats** (two equal columns, gap 24, margin top 24). Each: label
   (caption), value (stat style, margin top 2), unit (small).
   - "Burn rate": `burnRate.tokensPerMinute`, compact; unit "tokens per
     minute".
   - "Daily budget", by `DailyBudget.basis`:
     - `weeklyLimit`: `tokensLeftPerDay`, compact; unit "tokens left per
       day". If it is `null` (no pace yet): value "n/a", unit "no pace yet".
     - `tokenHistory` (Do it yourself mode without limit data; `allowance`
       is the average active day of the past week in tokens): value
       `allowance - usedToday` floored at 0, compact; unit "tokens left
       today".
     - `dailyBudget` is `null` (no budget at all): value "n/a", unit "needs
       limit data".
7. **Level block**, pushed to the bottom (margin top auto): a full-width
   button, 1 px `line` on top, padding 14 0 6. Row (space-between,
   baseline): `GameState.title` (row style) and "340 / 500 XP" (small,
   mono, `xpIntoLevel / xpForLevel`); below "Level 7" (small); then the
   level bar (3 px, margin top 10). Opens the achievements screen.
8. **Note** (margin top 10): see "Note line" below.

### Note line

One line, first match wins. Status notes take precedence, then the update
line; the limit note is the fallback and shows with and without limit
data, so the line is never empty:

| Condition | Text |
|---|---|
| `data.state = importing` | "Reading logs, 24 of 38 files." (derived; numbers mono) |
| `data.state = noLogs` | "No Claude Code logs yet. Start a session." (derived) |
| `data.state = error` | "Can’t read the logs. Retrying in the background." (derived) |
| `UpdateStatus.kind` is `available`, `installing` or `installFailed` | the update line, see "Update line" below |
| `limits.reason = noSource` | "No limit data yet. Setup is in Settings." as a text link to Settings, scrolled to "Limit data" (derived, see DECISIONS.md) |
| otherwise (limit note) | "Limit % includes claude.ai chats. Token stats are Claude Code only." |

### Update line

The dashboard's only trace of updates, shown in the note line (above).
Derived from the maintainer decisions in TKM-41; the design prototype has
no update UI. Note style; the version is mono (`tabular-nums`), version
first; actions are text links in the same line, styled like the
`noSource` note link (gap 8 before the link). No icon, no badge, no
popup, no toast.

| `UpdateStatus.kind` | Text | Action |
|---|---|---|
| `available` | "0.4.1 is out. Update" | "Update" calls `install_update` |
| `installing` | "Installing 0.4.1. Back in a moment." | none |
| `installFailed` | "0.4.1 didn’t install. Try again" | "Try again" calls `install_update` |

All other kinds (`idle`, `checking`, `upToDate`, `unavailable`,
`checkFailed`) show nothing here; the note line falls through to the next
row. "Couldn’t check" lives in Settings only. While `install_update` runs
the link is disabled. Windows: the installer's own small progress window
appears and Tokenmonk quits, then starts again. macOS: the app restarts by
itself. If `install_update` rejects (`CommandError`, for example a stale
status), re-read `get_update_status`; no copy for it.

Version numbers come from `UpdateStatus.version` ("0.4.1" is a sample).

### Dashboard states

- **No limit data** (`limits.status = unavailable`): segment hidden; ring
  track only; centre hero-sm with `today.total` (compact) and caption
  "tokens today"; status line "No limit data. Showing tokens only."; reset
  line empty; daily budget stat on its `tokenHistory` basis in Do it
  yourself mode, or "n/a" / "needs limit data" when there is no budget.
- **Selected window missing** (`limits` usable but the selected window is
  `null`, for example a fresh 5-hour window before the next reply): ring
  track only; centre as in "No limit data"; status line "Fresh window.
  Numbers on your next reply." (derived); reset line empty. The other tab
  still works.
- **Stale** (`limits.status = stale`): values as usual, hero in `ink-2`;
  the reset line reads "As of 14:05. Updates on your next reply."
  (`fetchedAt`, local 24 h time, derived).
- **No logs** (`data.state = noLogs`): token numbers are 0; ring and
  status line follow the limits as usual; the note line explains it.
- **History replay** (`GameState.retroDone = false`): the level block
  shows "Counting old XP. Hold still." instead of "Level N" and a bar
  track only (derived).

All numbers and copy in examples are samples; real values come from
`DashboardSnapshot` and `GameState`.

### Onboarding

Shown while `Settings.mode = noneYet` (`View = onboarding`); the popup
opens by itself on every start while it is pending. Accent: saffron.

1. Top row: brand only.
2. Title "How do you want to vibe?" (onboard-title, margin 30 0 8).
3. "Pick one. You can switch later." (`ink-2`, margin bottom 22).
4. Two choice rows, full width, each filling half of the remaining height
   (`flex: 1`), 1 px `line` on top, padding 22 0, gap 16, items centred:
   the onboarding mark (app icon) at 52 px, the same in both rows, then
   the name (choice-lg) over the description (14 px,
   `ink-2`, margin top 2). Hover underlines the name (offset 4). Names
   and descriptions as in "Modes".
5. Note (margin top 18): "Mindful about your tokens. Or not."

A click chooses at once: `complete_onboarding { mode }`; the following
`settings-changed` switches to the dashboard. While the call runs both
rows are disabled. On `CommandError` the note reads "Couldn’t save that.
Try again." (derived) and the rows are enabled again.

Fed by: `Settings`, `complete_onboarding`.

### Settings

`View = settings`, opened by "Settings" on the dashboard or by the tray
menu (`navigate`). Back: the "Back" link or Esc. The screen scrolls
vertically when taller than the popup; the top row stays.

1. Top row: "Back".
2. Title "Settings" (title style, margin 16 0 6).
3. Label "Mode" (`ink-2`, margin bottom 10), then a radio group of two
   rows (`role="radio"`), 1 px `line` on top, padding 14 0: name (choice)
   over description (caption). The checked row has a 3 px `--accent` bar
   flush with the popup's left edge, full row height.
4. Gap 18, then switch rows (space-between, 1 px `line` on top, padding
   14 0): label left, switch right (`role="switch"`):
   - "Sound"
   - "Start at login"
   - "Check for updates automatically" (`Settings.updateCheck`, default
     on). Off: no check at start; "Check now" still works. Turning it on
     does not check at once.
   - Switch: 42 x 24, radius 12, track `line`; knob 18, round, `pop`,
     inset 3. On: track `--accent`, knob moved 18 px.
   A final empty hairline row closes the list.
5. **Limit data** (below; not in the prototype, kept because it is the
   only place to set limits up).
6. **Update row** and note, pushed to the bottom together:
   - Update row (note style, min height 16, margin bottom 4), directly
     above the version note: status text, then text links styled like
     the `noSource` note link (gap 12). Versions and times mono
     (`tabular-nums`), times in local 24 h format. By
     `UpdateStatus.kind`:

     | Kind | Text | Links |
     |---|---|---|
     | `idle` | none | "Check now" |
     | `checking` | "Checking for updates." | none |
     | `upToDate` | "Up to date, checked at 14:05." (`checkedAt`) | "Check now" |
     | `available` | "0.4.1 is out." | "Update", "Release notes" |
     | `installing` | "Installing 0.4.1. Back in a moment." | none |
     | `installFailed` | "0.4.1 didn’t install." | "Try again", "Release notes" |
     | `checkFailed` | "Couldn’t check." | "Check now" |
     | `unavailable` | "Updates aren’t set up in this build." | none |

     "Check now" calls `check_for_updates`, "Update" and "Try again" call
     `install_update`, "Release notes" opens `UpdateStatus.url` (the
     version's GitHub release page) in the browser. No release notes in
     the app. On a `CommandError` from `check_for_updates` the row reads
     "Couldn’t check." (derived). The tray item "Check for updates"
     (Tray menu) lands here and fills the row the same way.
   - Note: "Tokenmonk 0.3.1, MIT. Ideas and bugs go to GitHub issues."
     The version is `AppInfo.version` ("0.3.1" is a sample); while it is
     still empty: "Tokenmonk, MIT. Ideas and bugs go to GitHub issues."

Every change applies at once through `update_settings { patch }`, no save
button. While a call runs the control stays in its new position; on
`CommandError` it flips back and the note "Couldn’t save that. Try
again." appears under the section (derived). Switching mode changes the
accent after `settings-changed`.

Fed by: `Settings`, `update_settings`, `settings-changed`,
`AppInfo.version`, `NavigateTo`, `LimitSetup`, `DashboardSnapshot.limits`,
`UpdateStatus` (`get_update_status`, `update-status-changed`),
`check_for_updates`, `install_update`.

#### Limit data

Derived from the settings rows; same tokens. Tokenmonk never edits Claude
Code settings; this section only shows what to paste.

1. Label "Limit data" (`ink-2`, margin top 18, margin bottom 10).
2. Status (weight 500), from `DashboardSnapshot.limits`:

   | Limits | Text |
   |---|---|
   | `ok` or `stale` | "Connected. Last reading 14:05." (`fetchedAt`, local 24 h time, mono for the time); "Connected." if `fetchedAt` is `null` |
   | `unavailable`, `noSource` or no `reason`, `statuslineFile.receivedAt` set | "Set up. No current limits." |
   | `unavailable`, `noSource` or no `reason`, otherwise | "Not set up yet." |
   | `unavailable`, other reasons | "Can’t read the limit file. Next reply fixes it." |

   `statuslineFile` comes from `LimitSetup`; while `get_limit_setup` runs
   or if it failed, the "otherwise" row applies.

   Setup check line (caption, directly below the status, margin top 4),
   from `LimitSetup.statuslineFile`. Shown only if `limits.status` is
   `unavailable` and `get_limit_setup` succeeded; for `ok` and `stale` the
   status already carries the time. Time first, mono, `tabular-nums`;
   `<time>` is local 24 h time ("14:05") on the same local day, otherwise
   day, English short month and time ("3 Nov 14:05"). First match:

   | `statuslineFile` | Text |
   |---|---|
   | `exists = false` | "Nothing from the status line yet. Paste the block below, then send Claude Code one message." |
   | `receivedAt` set | "<receivedAt> · last status line. No current window in it: the next reply brings one on Pro or Max." |
   | `modifiedAt` set | "<modifiedAt> · something arrived, but it can’t be read. Send another message; if it sticks, update Tokenmonk." |
   | otherwise | "Something arrived, but it can’t be read. Send another message; if it sticks, update Tokenmonk." |

   Text link "Check again" (note link style) at the end of the line: calls
   `get_limit_setup` again and updates the check line and the status; the
   line keeps its old text while the call runs; if it fails, the old text
   stays.

3. Setup copy (caption): "Limits come from Claude Code’s status line.
   Local only, no login. Open <path>, create it if missing, and add this
   as your status line:"
   (the path in mono). `<path>` is `%USERPROFILE%\.claude\settings.json`
   if `AppInfo.platform = windows`, otherwise `~/.claude/settings.json`.
4. Snippet block: 1 px `line` border, radius 8, padding 12, mono 12,
   `ink`, selectable, wraps anywhere (`white-space: pre-wrap`,
   `overflow-wrap: anywhere`). Content is
   `JSON.stringify({ statusLine: { type: "command", command } }, null, 2)`
   with `command = LimitSetup.statuslineCommand`, for example:

   ```json
   {
     "statusLine": {
       "type": "command",
       "command": "/Applications/Tokenmonk.app/Contents/MacOS/tokenmonk --statusline"
     }
   }
   ```

   The block is the whole status line setting: its `command` is the
   entire status line command, not a line for a script. Merge hint
   (caption, directly below the block): "File already has settings? Put
   "statusLine" at the top level next to them and end the key before it
   with a comma. Already has a "statusLine"? This replaces it." followed,
   if `AppInfo.platform` is not `windows`, by " To keep your own script,
   skip this block and use the line below." The key names in mono.

   Text link "Copy" at the block's top right. After a successful copy the
   caption under the block reads "Copied. Paste it, then send Claude Code
   one message." for 2 s. If copying fails: "Couldn’t copy. Select it and
   copy by hand." If `statuslineCommand` is `null`, the block is replaced by
   "Couldn’t find the Tokenmonk executable. Reinstall, then look again.";
   the copy link, the PowerShell alternative and the chain hint are not
   shown. If `get_limit_setup` fails, the same parts are replaced by
   "Couldn’t look up the setup line. Close Settings and open it again."
   While the call runs none of them is shown yet.

   Windows, path with a space (`AppInfo.platform = windows` and
   `statuslineCommand` starts with `"`): below the block, the caption "No
   Git Bash? Claude Code then uses PowerShell. Use this one instead:" and a
   second block (same style, own copy link and captions) with
   `command = "& " + statuslineCommand`. Otherwise not shown.
5. Chain hint. The line is Bash, so it depends on the platform:
   - `AppInfo.platform` is not `windows`: caption "Already have a status
     line script? Leave settings.json as it is. Add this line inside the
     script, after it reads the input:" (settings.json in mono) and a
     block (own copy link) with
     `printf '%s' "$input" | <command> >/dev/null`, where `<command>` is
     `statuslineCommand`.
   - `windows`: caption only, no block and no copy link: "Already have a
     status line? Tokenmonk can’t share it on Windows yet. Swap yours for
     the block above, or keep it and go without limits."
6. Plan note (caption): "Needs a Pro or Max plan. Numbers show up after
   the first reply."

Fed by: `LimitSetup` (`get_limit_setup`, called once when the view opens
and again on "Check again"; `statuslineCommand`, `statuslineFile`),
`DashboardSnapshot.limits` (`status`, `reason`, `fetchedAt`),
`AppInfo.platform`.

### Achievements

`View = achievements`, opened by the dashboard's level block. Back: the
"Back" link or Esc. Scrolls vertically when needed.

1. Top row: "Back".
2. `GameState.title` (ach-title, margin top 16, margin bottom 0).
3. "Level 7, 340 of 500 XP" (mono-sm, margin top 2).
4. Level bar, 4 px, margin 12 0 24.
5. "Achievements, 2 of 5" (`ink-2`, margin bottom 6): unlocked count of
   the listed rows, then the listed count.
6. One row per listed achievement, in display order
   (`GameState.achievements`): 1 px `line` on top, padding 11 0,
   space-between, gap 12. Left: name (row style) over description
   (caption). Right: "Unlocked" or "Locked" (small, no wrap, padding top
   3). Locked rows show name and description at 50 % opacity.

Listed: every achievement except secret ones that are still locked. Boo
("Halloween 2026") is a retired legacy badge: secret and no longer
earnable, so it is listed only for players who already unlocked it.

Fed by: `GameState` (`title`, `level`, `xpIntoLevel`, `xpForLevel`,
`progress`, `achievements`).

### Achievement toast

Shown for every `achievement-unlocked` with `retroactive = false`, inside
the popup on any screen.

Layout: absolute, inset 16 from left, right and bottom; background `ink`,
text `pop`, radius 8, padding 12 16 12 18. A 3 px `--accent` bar on the
left edge, inset 8 top and bottom, radius 2. Content: the name
(toast-title) over one line (12.5, 75 % opacity): `detail`, else the
achievement's `description`, followed by " +N XP" (for example "Coding
after midnight. +50 XP"). It slides in and out (see Motion).

Rules:

- One toast at a time, visible 3.4 s, click dismisses, pointer hover
  pauses the timer.
- Queue: events that arrive while the popup is hidden are kept and shown
  on the next open, oldest first, in addition to the OS notification.
  Three or more waiting collapse into one toast: "3 achievements
  unlocked", line "Newest: Night Owl. +150 XP" (summed XP).
- Boo can no longer be unlocked, so it never gets a toast.
- Priority: level-up screen, then toasts. A toast waits while the level-up
  screen is open.
- Sound: the synthesized tone, only with `Settings.sound` on.

While hidden: achievement unlocks and limit warnings also go out as OS
notifications through the notification plugin. The in-popup queue still
shows on the next open. No OS notification for `retroactive = true`. The
notification is sent without its own sound where the plugin allows it, so
the tone is not doubled.

| Notification | When | Title | Body |
|---|---|---|---|
| Unlock | `achievement-unlocked`, not retroactive | "Night Owl unlocked, +50 XP" | detail, else description |
| Max it out, 90 % | `limits.fiveHour.utilization` crosses 90 | "5h limit at 90 %" | "Limit in ~25 min at this pace." |
| Max it out, 100 % | crosses 100 | "5h limit at 100 %" | "Limit reached. Breathe." |
| Do it yourself, over budget | `dailyBudget.usedPercent` crosses 100 | "Today’s budget at 100 %" | "Over budget. The AI wins today." |

Each limit warning fires once per upward crossing: once per 5-hour window
(`fiveHour.resetsAt`) and once per day for the budget. None while
`limits.status` is not `ok` (Max it out), none for stale values, none while
`fiveHour = null`. The 90 % body uses `burnRate.limitEtaMinutes` (N rounded
to 5, at least 5); without an ETA the body is omitted. Level-ups get no OS
notification; the level-up screen waits for the next open.

Fed by: `AchievementUnlocked`, `GameState.achievements`, `Settings.sound`;
limit warnings from `DashboardSnapshot.limits`, `burnRate`, `dailyBudget`.

### Level-up screen

Not in the prototype; derived from the achievements screen. Shown for
every `level-up` with `retroactive = false`, over the current screen
(background `pop`, same padding), queued like toasts.

Top to bottom: the monk mark at 52 px in the mode accent; "Level 8" (onboard-title); the
mode line (status style): Max it out "Keep burning.", Do it yourself
"Well earned. Breathe."; "Level 8, 0 of 2,100 XP" (mono-sm); the level bar
(4 px); the text link "Back to work". Several levels at once (`level -
previousLevel > 1`): "+2 levels" (mono-sm) under the title. Esc and the
link close it. Tone only with `Settings.sound` on.

Retro summary: when `retroDone` turns from false to true in this run, the
screen shows once with "Level 6", the line "Straight from your history."
and "4 achievements, 3,420 XP" (unlocked count, `xpTotal`). Skipped at
level 1 with no unlocks. No toasts for retroactive unlocks.

Fed by: `LevelUp`, `GameState`, `Settings.sound`.

## Tray icon

Rendered in Rust (resvg) at runtime; `tokens.css` mirrors its values.
viewBox 24 x 24:

- Track: circle r 10, stroke `--tray-track` #8A8D8B at 60 %, width 2.
  Without limit data (no 5-hour value) the track is dotted
  (`stroke-dasharray 1.5 3`) and there is no arc.
- Arc: r 10, stroke `--accent` of the current mode, width 2.4, round
  caps, from 12 o'clock clockwise, length = 5-hour percent left. It drains
  as the window is used.
- Monk: the paths of `assets/brand/tokenmonk-mark-mono.svg` inside the
  ring, filled in `--accent` of the current mode (black on the macOS
  template icon), placed with `transform="translate(0.992 -0.416)
  scale(0.0215)"`: about 13 units tall (y 5.4 to 18.6), centred at (12, 12). There is no
  centre dot any more.

Value: 5-hour percent left in both modes. Before onboarding the accent is
saffron. The tray gauge is not the app icon: it carries the same monk as
the app icon, in one colour and without the ink body, and is drawn from
the mono mark, so it follows changes to that file, not to the app icon
files.

## Tray menu

Opened by a right click on the tray icon (a left click toggles the
popup). Native menu, system font, no tokens. Items, top to bottom:

| Item | Does |
|---|---|
| "Open Tokenmonk" | Shows the popup. |
| "Settings" | Shows the popup and opens Settings. |
| "Check for updates" | Shows the popup, opens Settings and starts the same check as "Check now". |
| separator | |
| "Quit" | Ends the app. |

"Check for updates" has no result of its own: no dialog, no
notification, no menu text change. The answer appears on the Settings
update row like any other check ("Checking for updates.", then "Up to
date, checked at 14:05.", "0.4.1 is out." or "Couldn’t check."; see
Settings, "Update row"). It works whatever "Check for updates
automatically" says. While a check or an install runs it only opens
Settings and the row keeps showing that state.

## App icon

The monk without a ring on an ink rounded-square body. It replaces the
monk-in-ring brand mark as the app icon. Files in `assets/brand/`:

- `tokenmonk-icon.svg`: the macOS grid and the main source. Canvas
  1024 x 1024; body 824 x 824 at 100,100, rx 185. Also the popup top row
  mark at 26 px and the onboarding mark at 52 px.
- `tokenmonk-icon-windows.svg`: the same artwork with viewBox
  `84 84 856 856` (tighter crop, so the body fills more of the small
  Windows sizes). Source of `icon.ico` only.
- `tokenmonk-mark-mono.svg`: the monk alone, one colour on a transparent
  background, eyes cut out by nonzero winding. Recolour it by `fill`; the
  level-up screen fills it with `--accent`.

Colours (fixed in the files, not tokens, same in both themes):

| Part | Colour |
|---|---|
| Body | #181A1B |
| Inner edge (16 px stroke) | #363E3A |
| Skin | #E4E3DD |
| Robe | #E39A0B |
| Eyes | #181A1B |

Platform icons are generated from these files by `node scripts/icons.mjs`
(desktop sets only).

## Not covered by the design / out of scope

Out of scope for v0.1: the about screen (no `View` in the contracts).

Open points:

- Copy links in "Limit data" use the web Clipboard API
  (`navigator.clipboard.writeText`); whether it works in the Tauri webviews
  on both platforms is unverified. On failure the fallback copy applies
  and the snippet stays selectable.
- The prototype lists all achievements regardless of mode; whether
  mode-only achievements need a hint for the other mode is open.
- Update UI (update line, Settings update row and switch) is not in the
  prototype; it is derived from the TKM-41 decisions using existing
  styles only. Download progress is not shown.
