# Design

The design system for the Tokenmonk app, tray icon and README. All values
live as CSS variables in [`src/styles/tokens.css`](../src/styles/tokens.css);
components use the variables, never literal values. Brand files are in
[`assets/brand/`](../assets/brand/).

Where the brand guide and the popup screens disagree, the screens win.

## Personality and voice

The monk is calm, a little smug and secretly competitive. He never panics,
even at 99 %.

- **Short and dry.** "Limit in ~40 min at this pace", not "Warning! You are
  about to run out!"
- **Gamer wink, not cringe.** Titles such as *Rate-Limit Rambo* or *Token
  Monk* are the jokes; the rest of the UI stays plain.
- **Mode-aware.** Max it out talks like a coach ("Almost there 🔥"), Do it
  yourself like a teacher ("Breathe. You still have this.").
- **Numbers first.** Every screen leads with one number in the `counter`
  style.
- English UI copy, real typographic quotes and apostrophes (’), no em
  dashes. Percent is written with a space: `72 %`. Thousands use a comma:
  `184,230`.

## Modes

| Mode | Fill | As text | Icon |
|---|---|---|---|
| 🔥 Max it out | `--saffron` | `--saffron-text` | flame line icon |
| 🛠️ Do it yourself | `--jade` | `--jade-text` | wrench line icon |

A mode colour never stands alone as a signal; it always comes with the icon
or the mode name. Text on any fill (`saffron`, `jade`) is
`--on-accent`. Before onboarding the accent is saffron (brand colour).

Components use the mode-neutral variables, switched by `data-mode` on
`<html>`:

| Variable | Meaning |
|---|---|
| `--accent` | Fills: mode label, weekly bar, level title, badge ring, gauge |
| `--accent-text` | Accent used as text or thin icon |
| `--halo` | Halo of the header logo and the tray icon |
| `--gauge` | Main gauge fill (= `--accent`) |
| `--gauge-danger` | Gauge fill in danger (= `--ember`) |
| `--gauge-track` | Unfilled gauge and bar tracks (= `--surface-300`) |

## Colour

Dark is the shipped theme in v0.1 (`:root`). Light values exist under
`:root[data-theme="light"]` for later.

| Token | Dark | Light | Use |
|---|---|---|---|
| `surface-100` | #15131A | #FBF6EE | App and popup background |
| `surface-200` | #1F1C25 | #F2EADC | Cards, logo disc |
| `surface-300` | #2A2631 | #E7DCC8 | Hover, pressed, gauge and bar tracks, badge pill background |
| `line` | #3A3440 | #CDBFA6 | Hairlines only, never the sole border of a control |
| `ink` | #F4EEE4 | #221C16 | Primary text and numbers, XP bar fill |
| `ink-muted` | #A89F94 | #665C52 | Labels, captions, timestamps, settings icon |
| `saffron` | #F5A142 | #F5A142 | Max it out fill, brand |
| `saffron-text` | #F5A142 | #8F4808 | Saffron as text |
| `jade` | #9BE8C4 | #3FAF7E | Do it yourself fill |
| `jade-text` | #9BE8C4 | #266849 | Jade as text |
| `on-accent` | #1A1206 | #1A1206 | Text and icons on accent fills |
| `ember` | #FF6B57 | #B3261E | Limit danger and errors only |
| `scrim` | surface-100 at 94 % | surface-100 at 94 % | Overlay background (level-up screen) |
| `shadow-popup` | 0 12px 32px rgba(0,0,0,.45) | 0 12px 32px rgba(34,28,22,.14) | Popup and toasts only |

No gradients, no neon glow, no purple.

## Typography

Fonts are bundled from `@fontsource/fraunces`, `@fontsource/manrope` and
`@fontsource/jetbrains-mono` (no CDN, works offline).

| Style | Family | Size / line | Weight | Use |
|---|---|---|---|---|
| display | Fraunces | 40 / 44 | 600 | Level-up screen, wordmark (large) |
| title | Fraunces | 24 / 30 | 600 | Window and section titles, onboarding title ("How do you want to vibe?") |
| heading | Manrope | 16 / 22 | 700 | Card headings, burn and mood line |
| body | Manrope | 14 / 20 | 400 | Default text |
| caption | Manrope | 12 / 16 | 500 | Labels, hints, timestamps |
| counter | JetBrains Mono | 32 / 36 | 600 | The big live number |
| mono | JetBrains Mono | 12 / 16 | 500 | Token counts in lists, XP lines, percentages |

Popup-specific sizes from the screens: wordmark Fraunces 600 18/22, level
Fraunces 600 20/26, stat value JetBrains Mono 600 20/26, strong body
labels Manrope 700 14/20. Fraunces is never
used for UI labels.

Every live number uses the mono family with `font-variant-numeric:
tabular-nums`.

## Spacing, radius, shadow

- Spacing on a 4 px grid: `space-1` 4, `space-2` 8, `space-3` 12,
  `space-4` 16, `space-6` 24, plus `space-8` 32 (overlay padding).
- Radius: `radius-sm` 6 (inputs, chips, toggles), `radius-md` 12 (cards,
  buttons, popup), `radius-lg` 20 (large panels, achievement tiles,
  onboarding choices), `radius-pill` 999 (mode badges, progress bars).
- One shadow, `shadow-popup`, for the popup and toasts only.

## Popup layout (360 x 680)

The popup is 360 x 680 px, radius 12, background `surface-100`, padding 16,
a single column with a 12 px gap between blocks. Hit targets are at least
44 x 44. Top to bottom:

1. **Header** (height 44): logo button (44 x 44) showing
   `assets/brand/tokenmonk-mark.svg` at 36 px with a live halo (decided:
   the inline drawing in the screens is a simplification of this mark),
   wordmark "Tokenmonk" (Fraunces 600 18/22) above the mode label (caption,
   `--accent-text`, 12 px mode icon at stroke 2, gap 4). Settings button 44 x 44, radius 12, 20 px gear icon
   in `ink-muted`, stroke 1.75.
2. **Halo gauge card**: `surface-200`, radius 12, padding 16, gap 16. Gauge
   132 x 132, circle radius 58, stroke 10, round caps, track
   `--gauge-track`, fill clockwise from 12 o'clock. Centre: counter value
   plus caption.
   - Max it out: value = 5-hour limit %; caption "5h limit"; right column
     (gap 8): burn line (heading) and "Resets in 2 h 14 min" (caption,
     `ink-muted`). Fill is `--gauge`, `--gauge-danger` at 90 % and above.
     Burn line: "Limit in ~N min at this pace" (N rounded to 5, at least 5);
     at 100 % "Limit reached. Breathe."
   - Do it yourself: value = share of today's budget used (may exceed
     100 %, the ring is capped at full); caption "of today’s budget";
     right column: mood line (heading) and XP line (mono, `--accent-text`,
     ember when over budget). Fill is `--gauge`, `--gauge-danger` above
     100 %. Mood lines: 0 % "No AI yet today. Pure so far."; below 75 %
     "Calm. N % of today’s budget left."; 75 to 100 % "Careful. The AI is
     creeping in."; above 100 % "Over budget. The AI wins today." XP line:
     "+N XP if you stop now", over budget "+0 XP today".
3. **Stat tiles**: two equal columns, gap 12; each `surface-200`, radius 12,
   padding 12, gap 4: caption label (`ink-muted`) over a stat value. Labels
   "Tokens today" and "This session".
4. **Weekly card** (full width): `surface-200`, radius 12, padding 12, gap 8.
   Row: label (Manrope 700 14/20) and value (mono) with space-between; bar
   8 px, pill, track `--gauge-track`, fill `--accent`; caption below.
   - Max it out: "Weekly limit", caption "Resets Thursday, 09:00".
   - Do it yourself: "Weekly limit used", caption "Lower is better here.
     Resets Thursday, 09:00".
5. **Level card**: same card style. Row: "Level 7" (Fraunces 600 20/26) and
   the title (caption, `--accent-text`); XP bar 8 px with `ink` fill;
   caption in mono "1,240 / 1,850 XP to level 8" (`ink-muted`).
6. **Achievement row**: card, padding 12, gap 12, items centred. Badge 36 px
   circle, 3 px `--accent` ring, 16 px glyph in `ink` (mini halo). Text
   column: name (Manrope 700 14/20) and a caption.
   - Unlocked variant (Max screen): "Night Owl unlocked", detail "Tokens
     burned at 02:47", trailing "+50 XP" (mono, `--accent-text`).
   - Progress variant (DIY screen): name "Token Monk", a 6 px bar (fill
     `--accent`), caption "4 of 7 days under 20 %".
7. **Footer note**, pushed to the bottom: caption size, regular weight,
   `ink-muted`, centred: "Limit % includes claude.ai chats · token stats from
   Claude Code only".

All numbers and copy in the screens are samples; real values come from
`DashboardSnapshot` and `GameState`.

## Logo and halo

The mark is a monk inside a ring. The ring is the **halo gauge**: filled
clockwise from 12 o'clock with the current value, round line ends.

- `tokenmonk-mark.svg`: full colour on its own `surface-200` disc, halo
  saffron, track `surface-300`. Works on both themes.
- `tokenmonk-mark-mono.svg`: single black ink, track at 30 % opacity. For
  the macOS menu bar (template image) and Windows tray contrast variants.
  Recolour by tinting, never by editing shapes.

The files are drawn at 128 x 128 with a sample 72 % halo; in the app the
arc length is live (`stroke-dasharray` on the halo circle, radius 54).

Rules: never on a saffron fill, never rotated, no second ring, the monk
stays inside the halo, no gradient in the halo. Minimum size 16 px (tray);
below 32 px the face may lose its eyes.

Halo colour and value:

| Where | Max it out | Do it yourself |
|---|---|---|
| Header logo halo | `--halo`, value = 5-hour % | `--halo`, value = budget used (capped at 100) |
| Tray icon halo | same as header | same as header |

The tray icon is rendered in Rust from the mark shapes with the live arc.

## Iconography

Line icons, 1.75 px stroke (2 px at 12 px size), round caps and joins,
matching the halo's round ends. Sizes 20 (buttons), 16 (badge glyphs), 12
(inline with captions). The mode emojis 🔥 and 🛠️ may appear in UI text.
Achievement badges are circles (mini halos) with one glyph inside.

## Derived screens

The mockups show the two dashboards and one overlay only; the overlay
layout is reused below. Everything below is derived from them: same tokens, same components, no new colours,
fonts or sizes. Where a size is not a named token it is built from the
spacing scale (for example `space-8 + space-2` = 40). Copy in quotes is
final; numbers in it are samples.

Shared building blocks used below:

- **Card**: `surface-200`, `radius-md`, padding `card-padding` (12).
- **Primary button**: min height `hit-target`, padding 0 `space-6`,
  `radius-md`, `--accent` fill, `on-accent` text, Manrope 700 14/20.
  Disabled: `surface-300` fill, `ink-muted` text. Full width where stated.
- **Icon button**: `hit-target` square, `radius-md`, `icon-size` line icon in
  `ink-muted`, hover `surface-300` (as the settings button).
- **Choice tile**: `surface-200`, `radius-lg`, padding `space-4`, gap
  `space-2`, min height `hit-target`. Hover `surface-300`. Selected: 2 px
  inset ring in the tile's mode fill (`--saffron` or `--jade`) plus an
  `icon-size` check icon in the mode's text colour, so selection never relies on colour
  alone.
- **Toggle** (not in the mockups, derived): track `space-8 + space-2` x
  `space-6` (40 x 24), `radius-sm`; knob `space-4` (16), `radius-sm`, inset
  `space-1`. Off: track `surface-300`, knob `ink-muted`. On: track
  `--accent`, knob `on-accent`. The whole settings row is the hit target.
- **Overlay**: covers the whole popup, `--scrim`, padding `space-8`,
  centred column, gap `space-4`.

Motion: toasts and overlays fade in and out on opacity only, duration from
one named UI constant (`FADE_MS = 150`), not a design token. Nothing else
animates. With `prefers-reduced-motion: reduce` they appear and disappear
without the fade.

Sound: one short synthesized tone per unlock or level-up (generated in the
UI, no audio file). It plays only when `Settings.sound` is on, once per
event, whether the popup is open or hidden.

### Onboarding

Shown while `Settings.mode = noneYet` (`View = onboarding`). The accent is
saffron (brand) until a mode is chosen. On every start while onboarding is
pending the popup opens by itself, so the choice is not hidden behind the
tray icon.

Layout (360 x 680, padding `popup-padding`, column, gap `space-6`):

1. Header: logo button and wordmark as on the dashboard, no mode label, no
   settings button. The logo halo shows the import progress (below).
2. Title "How do you want to vibe?" (title style, `ink`).
3. Two choice tiles, full width, stacked, gap `space-3`. Each: row with the
   `icon-size` mode icon in the mode text colour (`--saffron-text`,
   `--jade-text`) and the name (heading, `ink`), then the description
   (body, `ink-muted`).
   - "🔥 Max it out": "Use every token you pay for. XP for burning the
     limit."
   - "🛠️ Do it yourself": "Give the AI as little as possible. XP for staying
     under budget."
4. Caption (`ink-muted`): "Both paths level up. Switch any time in
   settings."
5. Pushed to the bottom: primary button, full width. Nothing selected:
   disabled, "Pick one". Selected: fill in the selected mode's fill,
   "Start with Max it out" or "Start with Do it yourself".
6. Below the button, the import line (mono, `ink-muted`, centred):
   "Reading logs · 24 / 38 files" while `data.state = importing`,
   "184,230 tokens found today" when ready, "No Claude Code logs yet" for
   `noLogs`. This is the screen's number.

States: nothing selected, one selected, saving (button disabled, label
unchanged), error (caption in `ember` above the button: "Couldn’t save
that. Try again."). Choosing calls `complete_onboarding { mode }`; the
following `settings-changed` switches to the dashboard.

Fed by: `Settings`, `complete_onboarding`, `DashboardSnapshot.data`,
`DashboardSnapshot.today.total`.

### Settings

`View = settings`, opened by the header settings button or by the tray menu
(`navigate`). Back to the dashboard: the back button or Esc.

Layout (360 x 680, padding `popup-padding`, column, gap `space-6`):

1. Header row (height `header-height`): icon button with a chevron-left icon
   (aria-label "Back to dashboard"), title "Settings" (title style).
2. Number block: counter "12,400" (`GameState.xpTotal`) over the caption
   "XP total. Kept when you switch modes." (`ink-muted`).
3. Section "Mode" (caption label, `ink-muted`): two choice tiles side by
   side, gap `space-3`, padding `space-3`; icon and name only ("🔥 Max it
   out", "🛠️ Do it yourself"); the current mode is selected. Caption below:
   "Titles follow the mode. XP stays."
4. Hairline (`line`), then the toggle rows, each min height `hit-target`, label
   (Manrope 700 14/20) over a caption (`ink-muted`), toggle at the right:
   - "Sound", caption "On unlocks and level-ups".
   - "Start at login", caption "Lives in the tray, not in your way".
5. Hairline, then section "Limit data" (see below).
6. Footer, pushed to the bottom (caption, `ink-muted`, centred):
   "Tokenmonk 0.1.0" from `AppInfo.version`.

The settings view scrolls vertically when its content is taller than the
popup; the header row stays in place.

#### Limit data

Column, gap `space-3`. Tokenmonk never edits Claude Code settings; this
section only shows what to paste.

1. Caption label "Limit data" (`ink-muted`).
2. Status line (Manrope 700 14/20), from `DashboardSnapshot.limits`:

   | Limits | Text |
   |---|---|
   | `ok` or `stale` | "Connected. Last reading 14:05." (`fetchedAt`, local 24 h time, mono for the time); "Connected." if `fetchedAt` is `null` |
   | `unavailable`, `noSource` or no `reason` | "Not set up yet." |
   | `unavailable`, other reasons | "Can’t read the limit file. Next reply fixes it." |

3. Setup copy (caption, `ink-muted`): "Limits come from Claude Code’s
   status line. Local only, no login. Add this to
   ~/.claude/settings.json:" (the path in mono).
4. Snippet card: `surface-200`, `radius-sm`, padding `space-3`, mono style
   (`--text-mono-*`), `ink`, text selectable, wraps long lines anywhere
   (`white-space: pre-wrap`, `overflow-wrap: anywhere`). Content is
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

   Icon button (copy icon, aria-label "Copy snippet") in the card's top
   right corner. After a successful copy the caption under the card reads
   "Copied. Paste it, then send Claude Code one message." for 2 s. If
   copying fails, the caption reads "Couldn’t copy. Select it and copy by
   hand." If `statuslineCommand` is `null`, the card is replaced by the
   caption "Couldn’t find the Tokenmonk executable. Reinstall, then look
   again."; the copy button, the PowerShell alternative and the chain hint
   (item 5) are not shown. If `get_limit_setup` fails, the same parts are
   replaced by the caption "Couldn’t look up the setup line. Close Settings
   and open it again." (`ink-muted`). While the call runs, none of them is
   shown yet.

   Windows, path with a space (`AppInfo.platform = windows` and
   `statuslineCommand` starts with `"`): below the card, the caption "No
   Git Bash? Claude Code then uses PowerShell. Use this one instead:"
   (`ink-muted`) and a second snippet card (same style, own copy button,
   same feedback captions) built the same way with `command = "& " +
   statuslineCommand`. Otherwise this alternative is not shown.
5. Chain hint (caption, `ink-muted`): "Already have a status line? Keep it.
   Add this line to your script after it reads the input:" followed by a
   second snippet card (same style, own copy button) with
   `printf '%s' "$input" | <command> >/dev/null`, where `<command>` is
   `statuslineCommand`.
6. Plan note (caption, `ink-muted`): "Needs a Pro or Max plan. Numbers show
   up after the first reply."

Fed by: `LimitSetup` (`get_limit_setup`, called once when the view opens),
`DashboardSnapshot.limits` (`status`, `reason`, `fetchedAt`).

States: every change applies at once through `update_settings { patch }`,
no save button. While a call runs the control stays in its new position;
on `CommandError` it flips back and the caption "Couldn’t save that. Try
again." appears under the section in `ember`. Switching mode changes the
accent immediately after `settings-changed`.

Fed by: `Settings`, `update_settings`, `settings-changed`,
`GameState.xpTotal`, `AppInfo.version`, `NavigateTo`, `LimitSetup`,
`DashboardSnapshot.limits`.

### Achievement toast

Shown for every `achievement-unlocked` with `retroactive = false`, inside
the popup.

Layout: a card floating over the top of the popup, inset `popup-padding`
from top, left and right (width `popup-width` minus twice
`popup-padding`), `shadow-popup`, padding `card-padding`, gap `space-3`,
items centred. Content is the unlocked variant of the achievement row:
badge (`badge-size`, `--accent` ring, `icon-size-sm` glyph in `ink`), name "Night Owl
unlocked" (Manrope 700 14/20) over the detail (caption, `ink-muted`),
trailing "+50 XP" (mono, `--accent-text`). Detail is
`AchievementUnlocked.detail`, else the achievement's `description`.

States and rules:

- One toast at a time, visible 4 s, click dismisses, pointer hover pauses
  the timer.
- Queue: events that arrive while the popup is hidden are kept and shown
  on the next open, oldest first, in addition to the OS notification
  (see while hidden). Three or more waiting collapse into one
  toast: "3 achievements unlocked", detail "Newest: Night Owl", trailing
  the summed "+150 XP".
- Boo is a retired badge and can no longer be unlocked, so it never gets
  a toast.
- Priority: level-up screen, then toasts. A toast waits while an overlay
  is open.
- Sound: the synthesized tone (see Sound above), only with
  `Settings.sound` on.

While hidden: when the popup is hidden, achievement unlocks and limit
warnings also go out as OS notifications through the notification plugin.
The in-popup queue above still shows on the next open; the OS
notification does not replace it. No OS notification for `retroactive =
true`. The tone
follows `Settings.sound`; the notification itself is sent without its own
sound where the plugin allows it, so the tone is not doubled.

| Notification | When | Title | Body |
|---|---|---|---|
| Unlock | `achievement-unlocked`, not retroactive | "Night Owl unlocked · +50 XP" | detail, else description |
| Max it out, 90 % | `limits.fiveHour.utilization` crosses 90 | "5h limit at 90 %" | "Limit in ~25 min at this pace." |
| Max it out, 100 % | crosses 100 | "5h limit at 100 %" | "Limit reached. Breathe." |
| Do it yourself, over budget | `dailyBudget.usedPercent` crosses 100 | "Today’s budget at 100 %" | "Over budget. The AI wins today." |

Each limit warning fires once per crossing upwards: once per 5-hour window
(`fiveHour.resetsAt`) and once per day for the budget. None while
`limits.status` is not `ok` (Max it out), none for stale values, and none
while `fiveHour = null` (fresh window, see "Limits unavailable and
stale"). The
body of the 90 % warning uses the burn line rule (N from
`burnRate.limitEtaMinutes`); without an ETA the body is omitted. Level-ups
get no OS notification; the level-up screen waits for the next open.

Fed by: `AchievementUnlocked` (`achievement-unlocked`),
`GameState.achievements` for the description, `Settings.sound`; limit
warnings from `DashboardSnapshot.limits`, `burnRate` and `dailyBudget`.

### Level-up screen

Shown for every `level-up` with `retroactive = false`, as an overlay. Same
queueing as toasts (shown on the next open if the popup was hidden).

Layout: overlay. Top to bottom: the logo mark at `logo-size-lg` (128) with
the halo full in `--halo`; "Level 8" (display, `ink`); the mode line
(heading, `--accent-text`): Max it out "Keep burning 🔥", Do it yourself
"Well earned. Breathe."; the XP line (mono, `ink-muted`) "0 / 2,100 XP to
level 9" from the following `GameState`; primary button "Back to work".

Variants:

- Title changed (`titleChanged`): a line before the mode line, caption
  `ink-muted` "New title" over the title (heading, `--accent-text`):
  "Rate-Limit Rambo". The mode line is then omitted.
- Several levels at once (`level - previousLevel > 1`): display "Level 9",
  caption under it "+2 levels" (mono, `ink-muted`).
- Retro summary: see first-start import.

Esc and the button close it. The synthesized tone plays only with
`Settings.sound` on.

Fed by: `LevelUp` (`level-up`), `GameState.xpIntoLevel`,
`GameState.xpForLevel`, `GameState.level`, `Settings.sound`.

### First-start import (retroactive levelling)

Two phases: the log import (`data.state = importing`, can start before
onboarding) and the XP replay after onboarding (`GameState.retroDone =
false`). The dashboard stays visible; only these blocks change.

- **Halo gauge card, import variant** (both modes, replaces the mode
  variant while `data.state = importing`): counter "62 %" =
  `filesDone / filesTotal` (0 % while `filesTotal = 0`), caption "logs
  read"; ring fill `--accent`, never danger. Right column: heading
  "Reading your history." over the caption (mono, `ink-muted`) "24 / 38
  files". The header halo shows the same value.
- **Stat tiles and weekly card**: live as normal; numbers grow during the
  import.
- **Level card while `retroDone = false`**: "Level 1" and title as
  delivered; XP bar track only; caption "Counting old XP. Hold still."
  (caption, `ink-muted`). There is no replay progress in the contracts, so
  this state has no number.
- **Achievement row while `retroDone = false`**: hidden.
- **Retro summary**: when `retroDone` turns from false to true in this
  run, show the level-up screen once in a summary variant: display
  "Level 6", heading "Straight from your history." (`--accent-text`), mono
  line "4 achievements · 3,420 XP" (unlocked count from
  `GameState.achievements`, `xpTotal`), button "Back to work". Skipped when
  the result is level 1 with no unlocks. No toasts for retroactive
  unlocks.

Fed by: `DataStatus` (`state`, `filesTotal`, `filesDone`),
`GameState.retroDone`, `level`, `title`, `xpTotal`, `achievements`.

### Empty state (no logs yet)

`data.state = noLogs` (`AppInfo.logsFound = false`).

- **Halo gauge card**: ring track only; counter "0", caption "tokens
  today". Right column: heading "No logs yet." over the caption
  (`ink-muted`) "Start a Claude Code session. I pick it up by myself."
- **Stat tiles**: "0" and "Idle" (see idle session).
- **Weekly card**: limits as delivered (usually unavailable, below).
- **Level card**: as delivered (Level 1, 0 XP).
- **Achievement row**: progress variant per the row rule (First Blood, bar
  empty).

Related states on the same card:

- **Idle session** (`session = null`, any data state): the "This session"
  tile shows "Idle" in the stat value style, `ink-muted`.
- **Log folder unreadable** (`data.state = error`): ring track only,
  counter keeps today's total, heading "Can’t read the logs." (`ember`),
  caption "Retrying in the background." `detail` is not shown in v0.1.

Fed by: `DataStatus.state`, `AppInfo.logsFound`, `DashboardSnapshot.today`,
`DashboardSnapshot.session`.

### Limits unavailable and stale

v0.1 gets limits from Claude Code’s status line (ARCHITECTURE.md,
"Limits"). Until that is set up, or when no window is current,
`limits.status = unavailable` with `reason = noSource`. The app then runs
on token data and says so.

**Unavailable**, Max it out gauge card: ring track only, header halo track
only; counter = `burnRate.tokensPerMinute`, caption "tokens / min". Values
of 10,000 and more are shortened inside the gauge to one decimal with `k`
("12.4k") so they fit the ring; everywhere else full numbers. Right column:
heading "No limit data. Running on tokens." over the reason caption
(`ink-muted`):

| `reason` | Caption |
|---|---|
| `noSource` | "Hook up Claude Code’s status line in Settings." |
| `notSignedIn` | "Not signed in to Claude." |
| `network` | "Offline. Trying again in 5 min." |
| `invalidResponse`, `other` | "The limit file talked nonsense. Next reply fixes it." |

With `reason = noSource` the caption is a text button (`accent-text`,
underlined, min height `hit-target`) that opens Settings, scrolled to
"Limit data". The setup itself lives only in Settings.

**Unavailable**, Do it yourself: the gauge runs on `dailyBudget` with
`basis = tokenHistory` as usual. If `dailyBudget = null` (not enough
history): ring track only, counter = today's total (shortened as above),
caption "tokens today", heading "Learning your normal." over the caption
"A budget needs a few days of history."

**Unavailable**, weekly card (both modes, `limits.weekly = null`): label as
usual, value "n/a" (mono, `ink-muted`), bar track only, caption "No limit
data. Token stats still count." With `reason = noSource` the caption is
"No limit data yet. Setup is in Settings." (same text button style as
above, so Do it yourself shows the hint too).

**Unavailable**, footer note: "No limit data · token stats from Claude Code
only".

**Stale** (`status = stale`, last good reading kept): gauge, weekly card
and halo show the last values as usual; the counter turns `ink-muted`; in
the right column the reset caption is replaced by "As of 14:05. Updates
on your next reply."
(`fetchedAt`, local 24 h time). The footer stays as usual. Ember danger
still applies to stale values.

**Fresh 5-hour window** (`fiveHour = null`, `weekly` set, `status = ok` or
`stale`): the 5-hour window has reset and no reading has arrived since
(ARCHITECTURE.md, Aggregates, "Limit status").

- Max it out gauge card: ring track only; counter =
  `burnRate.tokensPerMinute` (shortened as above), caption "tokens / min";
  right column heading "Fresh window." over the caption "Numbers on your
  next reply." (`ink-muted`). Burn line: no ETA.
- Header halo and tray halo (Max it out): track only, as in the
  unavailable state. Do it yourself halos keep showing the budget.
- Weekly card, Do it yourself gauge and `dailyBudget` (`weeklyLimit`
  basis): unchanged, from `weekly`. Stale styling applies to them as
  usual.
- Footer: as usual.
- 90 % and 100 % notifications: none, there is no 5-hour value to cross.
  The first reading of the new window (new `fiveHour.resetsAt`) starts a
  new crossing count; if it already is at or above 90 and `status = ok`,
  the 90 % warning fires once.

`source` and `detail` are not shown in v0.1.

Fed by: `LimitsView` (`status`, `reason`, `fiveHour`, `weekly`,
`fetchedAt`), `BurnRate.tokensPerMinute`, `DailyBudget`,
`DashboardSnapshot.today.total`.

### Achievement row rule

The row shows one achievement, picked in this order:

1. **Latest unlock**: `recentUnlocks[0]` if its `unlockedAt` is within the
   last 24 h. Unlocked variant (name "… unlocked", detail, "+N XP").
2. **In progress**: otherwise, from `achievements` in display order, the
   ones that are locked, not `secret`, with `mode` null or the current
   mode, and with `current` and `target` set; pick the highest
   `current / target`, ties by display order. Progress variant: name, 6 px
   bar (`bar-height-sm`, `--accent`), caption `progressLabel`, else mono
   "4 / 7".
3. **Next up**: if none has progress, the first eligible locked one in
   display order, bar empty, caption its `description`.
4. **All done**: the latest unlock regardless of age.

Boo ("Halloween 2026") is a retired legacy badge that can no longer be
earned. It is `secret`, so it never shows as in progress or next up; an
existing unlock shows like any other.

Retroactive unlocks count like any other; their historical `unlockedAt`
usually makes them older than 24 h. The row is hidden while
`retroDone = false`.

Fed by: `GameState.recentUnlocks`, `GameState.achievements`,
`GameState.mode`.

## Not covered by the design / out of scope

Out of scope for v0.1:

- The about screen. It has no `View` in the contracts.

Open points:

- Copy buttons in "Limit data" use the web Clipboard API
  (`navigator.clipboard.writeText`); whether it works in the Tauri webviews
  on both platforms is unverified. No clipboard plugin in v0.1; on failure
  the fallback copy applies and the snippet stays selectable.
- "Limit data" has no mockup; it is derived from the settings rows and
  card building blocks only.
