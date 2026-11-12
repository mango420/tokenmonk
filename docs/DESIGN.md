# Design

The design system for the Tokenmonk popup, tray icon and README.
[`src/styles/tokens.css`](../src/styles/tokens.css) is the single source of
truth for every value; this document describes the rules and names the
tokens (`--name`, or the bare name in tables). Components use the
variables, never literal values. Brand files are in
[`assets/brand/`](../assets/brand/). One monk does every job: the app icon
for the OS, the figure inside the popup and the centre of the tray icon
(see "Icon system").

Source: the prayer-bead redesign (TKM-269, with TKM-270 dark mode and
TKM-271 monk), signed off by the maintainer, on top of the earlier popup
prototype for the screens it did not change. Where an older rule and the
redesign disagree, the redesign wins. States neither shows are derived
below from their components and tokens; such rules and copy are marked
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
  of 920 XP"), not dots.
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
title or copy. The accent is used for fills and rules only (full beads,
level and week bars, mode dot, selected settings row, switch on, toast
edge, focus ring, the robe of the monk, the full tray beads), never for
text.

## Colour

Light and dark both ship. The theme follows the OS
(`prefers-color-scheme`) live, without a restart and without a toggle in
the app; `data-theme="light"` or `"dark"` on `<html>` forces one (tests
and previews only).

| Token | Light | Dark | Use |
|---|---|---|---|
| `page` | #CBCAC4 | #131715 | Backdrop outside the popup card |
| `pop` | #E4E3DD | #1D2220 | Popup background; toast text; switch knob; empty bead fill |
| `ink` | #181A1B | #E8E7E0 | Primary text and numbers; toast background; full bead and robe outline |
| `ink-2` | #5A5E5C | #98A09B | Labels, captions, units, links, secondary numbers |
| `line` | #BAB9B1 | #363E3A | Hairlines, card outline, gauge track, bar and switch tracks, popup outline |
| `saffron` | #E39A0B | #E39A0B | Max it out accent, brand |
| `jade` | #1F8A64 | #2FA57A | Do it yourself accent |
| `monk-face` | #F4F3EF | = `ink` | Monk head fill |
| `monk-line` | = `ink` | = `pop` | Sash, hands, eyes and mouth strokes |
| `monk-head-stroke` | = `ink` | none | Head outline |
| `bead-empty-stroke` | #8A8D88 | #7B847F | Empty bead outline |
| `bead-empty-fill` | = `pop` | = `pop` | Empty bead fill |

- `accent` is `saffron` or `jade` by `data-mode` (saffron before
  onboarding). Accents never carry text: they fill beads, bars, the robe,
  the mode dot, switch tracks and accent rules.
- Stale limit values: full beads and the week bar fill take `--accent` at
  `--stale-opacity` (45 %). One opacity token, no extra colour.
- Contrast (WCAG ratio on `pop`): `ink` 13.6 light / 13.0 dark, `ink-2`
  5.1 / 6.0, so body text, captions and mono numbers pass AA in both
  themes. Empty bead strokes are 2.6 / 4.2; the beads repeat the hero
  number, they never carry information alone.

`shadow-popup`: a 1 px `line` outline plus `0 28px 60px -24px` black at
40 %. No gradients, no neon glow, no purple, no danger colour: the beads
never turn red; the copy carries the urgency.

## Typography

Fonts are bundled from `@fontsource/schibsted-grotesk` (weights 400, 500,
600, 700) and `@fontsource/martian-mono` (weights 300, 400, 500); no CDN,
works offline. Weight 600 (`--weight-semibold`) is new with the redesign
(brand, level card title); it ships in the same package, no new
dependency.

- `--font-sans` Schibsted Grotesk: all UI text. Body 14 px, line height
  1.4, weight 400; emphasis is weight 500.
- `--font-mono` Martian Mono: every number that is data (percent, token
  counts, XP, durations, times) with `font-variant-numeric: tabular-nums`.

| Style | Family, weight | Size / tracking | Use |
|---|---|---|---|
| hero | mono 500 | 52, -2px, line 1 | Under the gauge: percent left ("73%") |
| hero-sm | mono 500 | 46, -2px, line 1 | Under the gauge without limit data: tokens today |
| hero-label | sans 500 | 18 | "of your limit left" / "tokens today", beside the hero on its baseline |
| onboard-title | sans 500 | 32, -0.025em, line 1.08 | "How do you want to vibe?", "Level 8" (number mono) |
| ach-title | sans 500 | 28, -0.02em | Title on the achievements screen |
| title | sans 500 | 22, -0.02em | "Settings" |
| choice-lg | sans 500 | 21, -0.015em | Onboarding choice name |
| status | sans 500 | 17, -0.01em | Status line under the hero |
| stat | mono 400 | 26, -0.03em, line 1.2 | Version on About |
| choice | sans 500 | 15.5 | Settings mode row name |
| brand | sans 600 | 15, -0.01em | "Tokenmonk" wordmark |
| row | sans 500 | 15 | Achievement name, About rows |
| toast-title | sans 500 | 14.5 | Toast name |
| caption | sans 400 | 13, `ink-2` | Gauge caption, "This week", level-up title line, descriptions, section labels |
| card-title | sans 600 | 13 (caption size), `ink` | Level card title |
| mono-sm | mono 400 | 12.5, `ink-2` | Level line on the achievements screen |
| small | sans or mono 400 | 12, `ink-2` | Mode name, week-low line, level-up XP line (mono), achievement state |
| meta | mono 400 | 11.5 (`--text-meta-size`), `ink-2` | Meta row, XP on the level card, week figure |
| note | sans 400 | 11.5, `ink-2` | Footer notes |

### Number formats

- Percent left: integer, `max(0, round(100 - utilization))`.
- Bead count: `0` only at exactly 0 % left, otherwise `ceil(percent / 5)`
  of the integer percent left (1 % is one bead, 100 % all 20).
- Compact token values (meta row, hero-sm): below 1,000 as is; below
  1,000,000 one decimal plus `k` ("4.2k"); above that one decimal plus `M`
  ("1.2M").
- Reset durations: under 24 h "Hh Mm" ("2h 14m", "0h 41m"); from 24 h
  "Dd Hh" ("3d 4h"). In the meta row prefixed "resets in ".
- Week reset: English short weekday of the local reset time ("resets
  Thu"); under 24 h away the duration as above ("resets in 5h 12m",
  derived).
- Estimates carry `~` directly before the number, no space ("~84.2k",
  "~25 min"); the `~` uses the number's own style.
- Limit ETA: `burnRate.limitEtaMinutes` rounded to 5, at least 5, as
  "~N min" (same rule as the OS notification, "While hidden").
- XP: full numbers with thousands commas ("1,240 / 1,880 XP").

## Spacing, radius, motion

- Spacing: `space-1` 4, `space-2` 8, `space-3` 12, `space-4` 16, `space-5`
  20, `space-6` 24. Every screen has `--popup-padding` 20 on all four
  sides; dashboard blocks sit in a column with `--popup-gap` 14. Odd
  values from the prototype (18, 22, 30) are written as named layout
  values in this document and live in the component CSS as sums of
  tokens or as tokens where they repeat.
- Radius: `radius-popup` 18, `radius-card` 12, `radius-switch` 12,
  `radius-toast` 8, `radius-focus` 4, `radius-bar` 2. Rows and choices
  have no radius and no fill; they are separated by 1 px `line` hairlines
  on top.
- Card: exactly one style. 1 px `line` outline, `radius-card` 12, no fill,
  no shadow. Only for blocks that are clickable or highlighted: the level
  card and the week-low box on the dashboard. Nothing else is a card.
- Bars: radius 2, track `line`, fill `--accent`. Level bar
  `--bar-height` 4 everywhere; week bar `--bar-week-height` 6.
- Focus: `:focus-visible` outline 2 px `--accent`, offset 3, radius 4.
- Motion, allowed and nothing else:
  - Bead fill: the first time the dashboard renders after start, the full
    beads fill clockwise one after another from bead 1, `--motion-bead-stagger`
    45 ms apart, each in `--motion-bead` 260 ms, `--ease-bead`
    `cubic-bezier(.2,.8,.2,1)`, from the empty look (scale 0.6, `pop`
    fill) to full (scale 1, `--accent` fill).
  - Bead lost: when a later value has fewer beads, each lost bead shrinks
    to empty in 260 ms, same easing (scale and fill). Gained beads after
    the first render (a fresh window) jump to full (derived).
  - Toast slide: up from below the popup edge (`translateY(140%)` to 0)
    in 280 ms, `cubic-bezier(.2,.8,.2,1)`, and back down when it leaves.
  - Switch knob: slides 18 px in 150 ms.
  - With `prefers-reduced-motion: reduce` all of them are off: the beads
    show their final state, the toast and knob appear in place.

Sound: one short synthesized tone per unlock or level-up (generated in the
UI, no audio file). It plays only when `Settings.sound` is on, once per
event, whether the popup is open or hidden.

## Icon system

One monk everywhere: closed smiling eyes, a sash across the robe, hands
resting at the hem. Besides the monk, the only icon in the app is the 8 px
mode dot (see "Shared parts").

### The monk drawing

All parts share one coordinate system: the 280 x 280 gauge viewBox
(centre 140, 140), so the monk sits inside the bead gauge without a
transform. Standalone files crop it with `viewBox="76 76 128 146"`.
Strokes are `--monk-stroke` 1.5 (derived for sash, hands, eyes and mouth;
the redesign gives 1.5 for the robe), lines with round caps.

| Part, in drawing order | Path | Fill | Stroke |
|---|---|---|---|
| Robe | `M82 216c2-46 26-74 58-74s56 28 58 74z` | `--accent` | `--ink` 1.5 |
| Sash | `M112 152c10 22 30 40 62 46` | none | `--monk-line` |
| Hands | `M116 216c6-10 16-14 24-14s18 4 24 14` | none | `--monk-line` (derived: drawn on the robe like the sash) |
| Head | circle cx 140, cy 106, r 24 | `--monk-face` | `--monk-head-stroke` (light `ink`, dark none) |
| Eyes | `M128 108c2 3 6 3 8 0M144 108c2 3 6 3 8 0` | none | `--monk-line` |
| Mouth | `M135 118c3 2 7 2 10 0` | none | `--monk-line` |

`--monk-line` is `ink` in light and `pop` in dark, so the sash and the
face stay dark on the saffron robe and the light dark-theme head.

### Roles and files

| Role | Source | Colour | Where | Size |
|---|---|---|---|---|
| A. App icon | `tokenmonk-icon.svg`, `tokenmonk-icon-windows.svg` | Fixed in the files: dark tile, saffron robe | Only where the OS shows the app: taskbar, dock, Start menu, installer, notifications, the generated icon set. Never inside the popup. | Platform sizes from `node scripts/icons.mjs` |
| B. Popup monk | The parts table above, drawn inline | Tokens: follows theme and mode | Inside the bead gauge (dashboard, level-up), onboarding rows, About | Gauge 240 / 220 (week-low) / 200 px (same coordinates); 52 px (`--mark-lg-size`) cropped to the standalone viewBox |
| C. Tray monk | Rendered in Rust from robe and head | Robe `--accent`, head `--tray-track` | Inside the tray beads | 11.5 x 13 units in a 32-unit box |
| Brand files | `tokenmonk-monk-light.svg`, `tokenmonk-monk-dark.svg`, `tokenmonk-monk-mark.svg` | Fixed in the files | README, docs, release pages | Any |

Files in `assets/brand/`, all with `viewBox="76 76 128 146"`:

- `tokenmonk-monk-light.svg`: colour on light. Robe #E39A0B stroked
  #181A1B; sash, hands, eyes, mouth #181A1B; head #F4F3EF stroked #181A1B.
- `tokenmonk-monk-dark.svg`: colour on dark. Robe #E39A0B stroked #E8E7E0;
  sash, hands, eyes, mouth #1D2220; head #E8E7E0, no stroke.
- `tokenmonk-monk-mark.svg`: single-colour mark. Robe and head #181A1B,
  sash knocked out in pop #E4E3DD at stroke 3 (`--mark-sash-stroke`),
  no hands and no face details.
- `tokenmonk-mark-mono.svg`: the old monk, legacy. Still read by the
  current popup mark and tray code; deleted once TKM-273 and TKM-275 have
  moved to the new monk. Do not use it for anything new.

- **A. App icon**: the new monk on the dark ink tile. Details in "App
  icon".
- **B. Popup monk**: the parts above with token colours, no tile and no
  background. Where a row sets its own accent (the onboarding choice
  rows), the robe takes that accent.
- **C. Tray monk**: robe and head only; sash, hands and face are below
  pixel size there (derived). Details in "Tray icon".

## Popup layout (360 x 680)

The popup is 360 x 680 px, radius 18, background `pop`, `shadow-popup`,
overflow hidden. Each screen is a column with `--popup-padding` 20 on all
four sides. Actions are text links (`ink-2`, hover `ink`, padding 6 px
vertical), full-width rows or a card (one style, see "Spacing, radius,
motion"). The only icons are the monk and the mode dot.

### Shared parts

- **Top row**: min height 32, items centred, space-between. On sub-screens
  the top row holds only the text link "Back" (Esc does the same). The
  dashboard and onboarding versions are described with their screens.
- **Mode dot**: 8 px circle (`--mode-dot-size`), fill `--accent`, gap 6
  (`--gap-6`, derived) before the mode name. Only next to a mode name:
  the dashboard top row and the settings mode rows (there it takes the
  row's own mode accent).
- **Monk**: role B in "Icon system", the only figure in the popup. Inside
  the bead gauge on the dashboard (240 px; 220 px in the week-low state)
  and the level-up screen
  (200 px); alone at 52 px (`--mark-lg-size`) in the onboarding rows and
  on About. The app icon tile never appears in the popup.
- **Bead gauge**: `--gauge-size` 240 px (level-up `--gauge-size-lg`
  200 px; dashboard in the week-low state `--gauge-size-week-low` 220 px,
  TKM-282), `viewBox="0 0 280 280"` (`--gauge-viewbox`), centred. Only the
  rendered size changes; the viewBox, beads and monk keep their units.
  - Track: circle at (140, 140), r 112 (`--gauge-radius`), stroke `line`,
    width 1.5 (`--gauge-track-stroke`), no fill.
  - 20 beads (`--bead-count`) on the track. Bead i (1 to 20) sits at angle
    `-90° + (i - 1) * 18°` (`--bead-start-angle`, `--bead-step-angle`):
    `cx = 140 + 112 cos a`, `cy = 140 + 112 sin a`. Bead 1 is at
    12 o'clock, the rest follow clockwise.
  - Full bead: r 10 (`--bead-radius`), fill `--accent`, stroke `--ink`
    1.5 (`--bead-stroke`).
  - Empty bead: the same circle scaled to 0.6 (`--bead-empty-scale`)
    around its own centre, fill `--bead-empty-fill` (`pop`), stroke
    `--bead-empty-stroke`. The scale is a transform, so the motion can
    tween it (see "Spacing, radius, motion").
  - Beads 1 to N are full, the rest empty, N from "Number formats". The
    string drains from the end: bead 20 goes first, bead 1 last.
  - Stale values: full bead fill at `--stale-opacity`.
  - The monk sits inside, drawn in the same coordinates (see "Icon
    system").
- **Level bar**: track `line`, fill `--accent`, radius 2, height 4
  (`--bar-height`) everywhere, width = `GameState.progress`.
- **Week bar**: as the level bar, height 6 (`--bar-week-height`), width =
  weekly percent left.
- **Hairline row**: full width, 1 px `line` on top, no fill.
- **Note**: note style, min height 16.

### Dashboard

A column with gap 14 (`--popup-gap`) between the blocks below; spacing
inside a block is given per block. Top to bottom:

1. **Top row**: three items, space-between (derived): the brand
   "Tokenmonk" (brand style, text only, no mark); the mode dot and the
   mode name ("Max it out" or "Do it yourself", small style); the text
   link "Settings".
2. **Caption** (caption style, centred): "Your Claude Code limit, this
   5-hour window". Other states in "Dashboard states".
3. **Gauge**: the bead gauge with the monk inside (see "Shared parts").
   The 20 beads show the 5-hour window only, never the week.
4. **Hero and status** (one block, centred):
   - Hero row, items on a shared baseline, gap 8 (derived): hero "73%"
     (number and `%` in one mono run, hero style, `tabular-nums`) and
     hero-label "of your limit left". Percent left as in "Number
     formats". Stale: hero in `ink-2`.
   - Status line (status style, margin top 4, derived), first match:
     1. Week runs out first (see block 6): "The week runs out first."
     2. Otherwise by 5-hour percent left and mode:

        | Percent left | Max it out | Do it yourself |
        |---|---|---|
        | 60 and more | "Barely touched. Use it." | "Light day. Nicely done." |
        | 30 to 59 | "Good pace." | "Mid-range. Breathe." |
        | 10 to 29 | "Nearly there. As planned." | "Getting thin. Slow down." |
        | 1 to 9 | "Almost gone. Make it count." | "Nearly out. Go for a walk." |
        | exactly 0 | "Limit reached. Breathe." | "All out. Good time for a walk." |

     A fresh window and a reached limit both show all beads empty; the
     hero and the status line tell them apart ("Dashboard states").
5. **Meta row** (meta style: mono 11.5, `ink-2`, `tabular-nums`; 1 px
   `line` on top, padding top 12, space-between, one line):
   - Left: "resets in 2h 14m" from `fiveHour.resetsAt`; empty when it is
     `null`. With a limit ETA it reads "Limit in ~25 min" instead.
     - **Limit ETA** (TKM-228). Shown only if all hold:
       `limits.status = ok`, `limits.fiveHour` is set,
       `burnRate.limitEtaMinutes` is at least 1, and either
       `fiveHour.resetsAt` is `null` or the rounded N shown in the text
       (see "Number formats") is less than the whole minutes until
       `resetsAt`, as also shown in "resets in" (otherwise the window
       resets first and the ETA means nothing; 118 min with the reset in
       1h 59m rounds to ~120 and is hidden). `limitEtaMinutes = 0` shows
       no ETA (the beads already say 0 %). Never while stale.
   - Right: "1.2k tokens/min" (`burnRate.tokensPerMinute`, compact). In
     Do it yourself with a `dailyBudget` the budget replaces it:
     `weeklyLimit` basis "~84.2k left today" (`tokensLeftPerDay`,
     compact, `~` because it also counts claude.ai chats; when it is
     `null` the tokens/min figure stays, derived); `tokenHistory` basis
     "52.1k left today" (`allowance - usedToday`, floored at 0, compact,
     derived).
6. **Week block**, shown while `limits.weekly` is set (otherwise left
   out, derived):
   - Row, space-between, baseline (derived): "This week" (caption) left,
     "81% left · resets Thu" (meta) right: weekly percent left as in
     "Number formats", then the week reset; " · resets Thu" is dropped
     when `weekly.resetsAt` is `null` (derived).
   - Week bar, 6 px, margin top 8 (derived). Stale: fill at
     `--stale-opacity`.
   - **Week runs out first**: when weekly percent left < 5-hour percent
     left and weekly percent left < 20 (both integers as shown; needs
     `limits.fiveHour` set, derived), the block becomes a card (padding
     12, `--card-padding`, derived). The card holds the row and the bar
     only; no extra line (TKM-282). The status line reads "The week runs
     out first." In this state the bead gauge renders at 220 px
     (`--gauge-size-week-low`, see "Shared parts"); the size is
     otherwise unchanged.
7. **Spacer**: takes the free height (`flex: 1`).
8. **Level card**: a full-width button in the card style, padding 12
   (`--card-padding`). Row, space-between, baseline: "Quiet Spender ·
   Level 7" (card-title: `GameState.title`, " · Level ", `level` in
   mono) left, "340 / 920 XP" (meta, `xpIntoLevel / xpForLevel`) right;
   below the 4 px level bar (margin top 8, derived). Opens the
   achievements screen.
9. **Note** (see "Note line" below).

The stat tiles ("Burn rate", "Daily budget", "This session") and the
"5 hours / Week" segment are gone; their numbers live in the meta row and
the week block.

### Note line

One line, first match wins. Status notes take precedence, then the update
line; the limit note is the fallback and shows with and without limit
data, so the line is never empty:

| Condition | Text |
|---|---|
| `data.state = importing` | "Reading logs, 24 of 38 files." (derived; numbers mono) |
| `data.state = noLogs` | "No Claude Code logs yet. Start a session." (derived) |
| `data.state = error` | "Can’t read the logs. Retrying in the background." (derived) |
| `UpdateStatus.kind` is `available`, `downloading`, `installing` or `installFailed` | the update line, see "Update line" below |
| `limits.reason = noSource` | "No limit data. It comes from the status line, see Settings." as a text link to Settings, scrolled to "Limit data" (derived, TKM-228; see DECISIONS.md) |
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
| `downloading`, `percent` set | "Downloading 0.4.1, 42%." | none |
| `downloading`, `percent` null | "Downloading 0.4.1." | none |
| `installing` | "Installing 0.4.1. Back in a moment." | none |
| `installFailed` | "0.4.1 didn’t install. Try again" | "Try again" calls `install_update` |

All other kinds (`idle`, `checking`, `upToDate`, `unavailable`,
`checkFailed`) show nothing here; the note line falls through to the next
row. "Couldn’t check" lives in Settings only. While `install_update` runs
the link is disabled.

Download progress (TKM-212, derived; the prototype has no update UI): after
"Update" the line goes `downloading` (percent null, then 0 to 100), then
`installing`. The percent is a live number: mono, `tabular-nums`, the "%"
inside the same mono span, last in the line so nothing after it shifts as
it grows from 1 to 3 digits. It is the whole percent from Rust, shown as
is: no decimals, no rounding in the UI, no animation between values, no
progress bar, no spinner. Percent null (size not known yet, or the server
sends no length) drops the number and the comma: "Downloading 0.4.1."
stays until `installing`, no fake percent, no ellipsis animation. Rust
emits only on whole-percent changes (see `UpdateStatus` in the contracts),
so the UI renders every event as it comes, no extra throttle. Windows: the installer's own small progress window
appears and Tokenmonk quits, then starts again. macOS: the app restarts by
itself. If `install_update` rejects (`CommandError`, for example a stale
status), re-read `get_update_status`; no copy for it.

Version numbers come from `UpdateStatus.version` ("0.4.1" is a sample).

### Dashboard states

- **Limit reached** (5-hour percent left exactly 0): all beads empty;
  hero "0%" with "of your limit left"; status line from the "exactly 0"
  row.
- **Fresh window** (`limits` usable, `fiveHour = null`, for example a new
  5-hour window before the next reply): all beads empty; hero-sm with
  `today.total` (compact) and hero-label "tokens today"; status line
  "Fresh window. Numbers on your next reply." (derived); meta row left
  empty. The week block shows as usual.
- **No limit data, with a token budget** (`limits.status = unavailable`
  and `dailyBudget.basis = tokenHistory`; only Do it yourself gets this
  basis; derived, TKM-228):
  - Caption: "Your token budget, today" (derived).
  - Beads: budget left = `max(0, round(100 - usedPercent))`, bead count
    as in "Number formats" (derived: the beads drain over the day like
    the 5-hour window).
  - Hero-sm with `today.total` (compact), hero-label "tokens today".
  - Status line: budget left through the mode's column of the status
    table ("Light day. Nicely done." and so on). Above 100 %
    (`usedPercent > 100`): "Over budget. The AI wins today." (the OS
    notification body).
  - Meta row: left "average day 52.1k" (`allowance`, compact, derived),
    right the `tokenHistory` budget as in "Dashboard".
  - No week block.
- **No limit data, no budget** (`limits.status = unavailable`, any other
  case): caption "Your Claude Code tokens, today" (derived); all beads
  empty; hero-sm with `today.total` (compact) and "tokens today"; status
  line "No limit data. Showing tokens only."; meta row left empty, right
  tokens per minute; no week block.
- **Stale** (`limits.status = stale`): values as usual; hero in `ink-2`;
  full beads and the week bar fill at `--stale-opacity`; meta row left
  "as of 14:05" (`fetchedAt`, local 24 h time, derived).
- **No logs** (`data.state = noLogs`): token numbers are 0; beads and
  status line follow the limits as usual; the note line explains it.
- **History replay** (`GameState.retroDone = false`): the level card
  reads "Counting old XP. Hold still." on the left, no XP figure, bar
  track only (derived).

All numbers and copy in examples are samples; real values come from
`DashboardSnapshot` and `GameState`.

### Onboarding

Shown while `Settings.mode = noneYet` (`View = onboarding`); the popup
opens by itself on every start while it is pending. Accent: saffron.

Padding 20 on all sides like every screen.

1. Top row: the brand "Tokenmonk" only (brand style, text only).
2. Title "How do you want to vibe?" (onboard-title, margin 30 0 8).
3. "Pick one. You can switch later." (`ink-2`, margin bottom 22).
4. Two choice rows, full width, each filling half of the remaining height
   (`flex: 1`), 1 px `line` on top, padding 22 0, gap 16, items centred:
   the monk at 52 px (`--mark-lg-size`), its robe in the row's own mode
   accent (Max it out saffron, Do it yourself jade; every other part as
   in "Icon system"), then the name (choice-lg) over the description (14 px,
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

Padding 20 on all sides like every screen; no cards.

1. Top row: "Back".
2. Title "Settings" (title style, margin 16 0 6).
3. Label "Mode" (`ink-2`, margin bottom 10), then a radio group of two
   rows (`role="radio"`), 1 px `line` on top, padding 14 0: the mode dot
   in the row's own mode accent (saffron or jade) and the name (choice)
   on one line, over the description (caption). The checked row has a
   3 px `--accent` bar flush with the popup's left edge (offset by the
   20 px padding), full row height.
4. Gap 18, then switch rows (space-between, 1 px `line` on top, padding
   14 0): label left, switch right (`role="switch"`):
   - "Sound"
   - "Start at login"
   - "Check for updates automatically" (`Settings.updateCheck`, default
     on). Off: no check at start; "Check now" still works. Turning it on
     does not check at once.
   - "Keep popup open" (`Settings.keepOpen`, default off), with the
     caption "Stays put until the tray icon or Close." under the label.
     Off: the popup hides when it loses focus. On: it stays visible next
     to other windows and hides only via the tray icon or the title bar
     "Close". Applies at once.
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
     | `downloading`, `percent` set | "Downloading 0.4.1, 42%." (`percent` mono) | none |
     | `downloading`, `percent` null | "Downloading 0.4.1." | none |
     | `installing` | "Installing 0.4.1. Back in a moment." | none |
     | `installFailed` | "0.4.1 didn’t install." | "Try again", "Release notes" |
     | `checkFailed` | "Couldn’t check." | "Check now" |
     | `unavailable` | "Updates aren’t set up in this build." | none |

     "Check now" calls `check_for_updates`, "Update" and "Try again" call
     `install_update`, "Release notes" opens `UpdateStatus.url` (the
     version's GitHub release page) in the browser. No release notes in
     the app. `downloading` follows the dashboard's update line rules
     (Update line above): same copy, same null fallback, no bar. On a
     `CommandError` from `check_for_updates` the row reads
     "Couldn’t check." (derived). The tray item "Check for updates"
     (Tray menu) lands here and fills the row the same way.
   - Note: "Tokenmonk 0.3.1, MIT." followed by the text link "About"
     (styled like the `noSource` note link, gap 12), which opens About.
     The version is `AppInfo.version` ("0.3.1" is a sample, mono
     `tabular-nums`); while it is still empty: "Tokenmonk, MIT." and the
     same link. The issues hint moved to About (TKM-125).

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
   the first reply. Limit numbers come from Claude Code’s status line;
   sessions in the desktop app may not run it and then bring no numbers."
   (Hedged on purpose: the docs say nothing about the desktop app; that
   it did not run there was only observed once, not verified. See
   DECISIONS.md, "Popup with log-only data".)

Fed by: `LimitSetup` (`get_limit_setup`, called once when the view opens
and again on "Check again"; `statuslineCommand`, `statuslineFile`),
`DashboardSnapshot.limits` (`status`, `reason`, `fetchedAt`),
`AppInfo.platform`.

### About

Not in the prototype: derived from the Settings and Achievements parts,
so all of its copy is derived. `View = about` (TKM-125), opened by the "About" link in the Settings
footer note (Settings, item 6). No other entry point: no tray item, no
link on the dashboard. Back: the "Back" link or Esc, both return to
Settings (not the dashboard); Settings opens at the top as usual. A
`navigate` event while About is open switches views like on any other
screen. Scrolls vertically when taller than the popup; the top row stays.
Padding 20 on all sides; rows, no cards.

1. Top row: "Back".
2. Monk at 52 px (`--mark-lg-size`), margin top 16.
3. Title "Tokenmonk" (title style, margin 12 0 2).
4. Version, the screen's number: `AppInfo.version` as is, for example
   "0.4.0" (stat style, mono `tabular-nums`). While it is still empty:
   "n/a" in the same style (derived).
5. Caption under it (margin top 2, margin bottom 18): "Installed
   version. Updates live in Settings."
6. Hairline rows (space-between, 1 px `line` on top, padding 14 0):
   label left (row style), value right; a final empty hairline row closes
   the list.

   | Label | Value |
   |---|---|
   | "Licence" | "MIT" (`ink-2`, plain text, no link) |
   | "Source" | text link "GitHub", opens `REPO_URL` |
   | "Ideas and bugs" | text link "GitHub issues", opens `ISSUES_URL` |

   Text links as everywhere (`ink-2`, hover `ink`, padding 6 px
   vertical). They open in the browser through the opener plugin like
   "Release notes"; a failed open shows nothing.
7. Note, pushed to the bottom: "Made by Tokenmonk contributors. No
   account, no telemetry."

Nothing else: no author, no email, no website, no credits list, no
changelog, no update controls (those stay in Settings). Only the two
GitHub links above leave the app.

Tokens: shared ones only (`--mark-lg-size`, title, stat, row, caption
and note text tokens, `line`, `ink`, `ink-2`). No new token.

Fed by: `AppInfo.version` (`get_app_info`, `app-info-changed`),
`REPO_URL`, `ISSUES_URL`.

### Achievements

`View = achievements`, opened by the dashboard's level card. Back: the
"Back" link or Esc. Scrolls vertically when needed. Padding 20 on all
sides; rows, no cards.

1. Top row: "Back".
2. `GameState.title` (ach-title, margin top 16, margin bottom 0).
3. "Level 7, 340 of 920 XP" (mono-sm, margin top 2).
4. Level bar, 4 px, margin 12 0 24.
5. "Achievements, 2 of 5" (`ink-2`, margin bottom 6): unlocked count of
   the listed rows, then the listed count.
6. One row per listed achievement, in display order
   (`GameState.achievements`): 1 px `line` on top, padding 11 0,
   space-between, gap 12. Left: name (row style) over description
   (caption). Right: "Unlocked" or "Locked" (small, no wrap, padding top
   3). Locked rows show name and description at 50 % opacity.
7. Other-mode hint: a locked row whose achievement only counts in the
   other mode (`AchievementProgress.mode` is set and differs from
   `GameState.mode`) gets one more line under the description: "Max it
   out only. Not on this mode's menu." or "Do it yourself only. Not on
   this mode's menu." (the mode name from Modes). Style: small, `ink-2`,
   margin top 2, full opacity (not dimmed with the row), may wrap. The
   right side still reads "Locked". No accent colour, no icon.

Other-mode hint rules: no hint on unlocked rows (earned is earned, the XP
stays in the one pool), none for achievements with `mode = null`, none
while `GameState.mode` is `noneYet`. The hint follows `game-updated`, so
a mode switch in Settings updates it. It does not change the heading
count, the row order or the dimming.

Listed: every achievement except secret ones that are still locked. Boo
("Halloween 2026") is a retired legacy badge: secret and no longer
earnable, so it is listed only for players who already unlocked it.

Fed by: `GameState` (`mode`, `title`, `level`, `xpIntoLevel`,
`xpForLevel`, `progress`, `achievements`).

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

Shown for every `level-up` with `retroactive = false`, over the current
screen (background `pop`, padding 20), queued like toasts. Centred
column; spacing between the items as on the current screen (derived).

Top to bottom:

1. The bead gauge at 200 px (`--gauge-size-lg`) with all 20 beads full
   and the monk inside, robe in `--accent` of the current mode. No bead
   motion (derived).
2. "Level 8" (onboard-title; "Level" sans, the number mono
   `tabular-nums`).
3. Several levels at once (`level - previousLevel > 1`): "+2 levels"
   (mono-sm).
4. Title line (caption: 13 px, `ink-2`) from `LevelUp.title`:
   `titleChanged = false` "Still a <title>." ("Still a Quiet
   Spender."), `titleChanged = true` "Now a <title>.". The article is
   "an" before a vowel sound ("Now an Abbot of Restraint.").
5. Mode line (status style): Max it out "Keep burning.", Do it yourself
   "Well earned. Breathe.".
6. "Level 8, 0 of 1,040 XP" (mono 12, small style).
7. The level bar (4 px).
8. The text link "Back to work".

Esc and the link close it. Tone only with `Settings.sound` on.

Retro summary: when `retroDone` turns from false to true in this run, the
screen shows once with "Level 6", the line "Straight from your history."
and "4 achievements, 2,640 XP" (unlocked count, `xpTotal`). Skipped at
level 1 with no unlocks. No toasts for retroactive unlocks.

Fed by: `LevelUp`, `GameState`, `Settings.sound`.

## Tray icon

Rendered in Rust (resvg) at runtime; `tokens.css` mirrors its colours and
opacities. A 32 x 32 unit box (`viewBox="0 0 32 32"`):

- **Beads**: 5, one per 20 %, r 3.2, centres in this order: (16, 4),
  (27.4, 12.3), (23.1, 25.7), (8.9, 25.7), (4.6, 12.3). Bead 1 is at
  12 o'clock, the rest follow clockwise. Count from the 5-hour percent
  left: 0 only at exactly 0 %, otherwise `ceil(percent / 20)`. Beads 1 to
  N are full, the rest empty: the string drains from the end, as in the
  popup.
  - Full: fill `--accent` of the current mode, no stroke (derived).
  - Empty: same circle, fill `--tray-track` #8A8D8B at 35 %
    (`--tray-bead-empty-opacity`), no scale (derived).
- **Monk**: robe and head of the monk drawing ("Icon system"), nested as
  `<svg x="10.25" y="9.5" width="11.5" height="13" viewBox="76 76 128 146">`.
  Robe fills `--accent` of the current mode, head fills `--tray-track`
  at full opacity; no strokes, no sash, hands or face (derived: below
  pixel size).
- **No limit data** (no 5-hour value): all beads empty and the whole monk
  at opacity 0.45 (`--tray-monk-nodata-opacity`).
- **macOS**: a template image in black (alpha carries the shape; the menu
  bar tints it). Full beads and robe black; empty beads at 30 %
  (`--tray-bead-empty-opacity-template`); head at 40 %
  (`--tray-head-opacity-template`). The no-data fade (0.45) applies to the
  monk here too.

Value: 5-hour percent left in both modes. Before onboarding the accent is
saffron. The tray icon is not the app icon (role C in "Icon system"): it
is drawn from the monk paths, not from the app icon files.

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

The new monk on the dark ink rounded-square tile (role A in "Icon
system"). Used only where the OS shows the app (taskbar, dock, Start
menu, installer, notifications, the generated icon set), never inside
the popup. Files in `assets/brand/`:

- `tokenmonk-icon.svg`: the macOS grid and the main source. Canvas
  1024 x 1024; body 824 x 824 at 100,100, rx 185, with a 16 px inner
  edge. The monk is the colour-on-dark drawing placed with
  `transform="translate(-48 -84) scale(4)"`: centred on x 512, head top
  at y 244, hem at y 780, robe 464 wide (derived: about the size of the
  old monk on the tile).
- `tokenmonk-icon-windows.svg`: the same artwork with viewBox
  `84 84 856 856` (tighter crop, so the body fills more of the small
  Windows sizes). Source of `icon.ico` only.

Colours (fixed in the files, not tokens, same in both themes):

| Part | Colour |
|---|---|
| Body | #181A1B |
| Inner edge (16 px stroke) | #363E3A |
| Robe | #E39A0B |
| Robe outline, head | #E8E7E0 |
| Sash, hands, eyes, mouth | #1D2220 |

Platform icons are generated from these files by `node scripts/icons.mjs`
(desktop sets only); the generated set is refreshed by TKM-276.

## Not covered by the design / out of scope

Open points:

- Copy links in "Limit data" use the web Clipboard API
  (`navigator.clipboard.writeText`). Verified manually on Windows (WebView2,
  installed release build): the snippet lands on the clipboard. macOS
  (WKWebView) is still unverified. If `writeText` rejects there, the
  fallback applies: the caption "Couldn’t copy. Select it and copy by hand."
  shows and the snippet text is selected.
- The other-mode hint on the Achievements screen is not in the prototype
  (it lists all achievements regardless of mode); decided in TKM-126
  using existing styles only.
- Update UI (update line, Settings update row and switch) is not in the
  prototype; it is derived from the TKM-41 decisions using existing
  styles only. Download progress (TKM-212) is text in the same line, no
  bar.

Open points from the prayer-bead redesign (TKM-269). Each rule below is
marked "derived" above and stands until the maintainer decides otherwise:

- Text styles and spacing inside the week block ("This week" caption,
  figure in meta style, margins 8), the week-low card padding (12, as the
  level card) and the week reset copy under 24 h.
- Dashboard without limit data: captions, beads on the token budget,
  "average day" and "as of 14:05" in the meta row.
- Monk strokes: widths of sash, hands, eyes and mouth (1.5) and the
  colour of the hands (`--monk-line`).
- Height: the week-low state drops the extra card line and renders the
  bead gauge at 220 px (TKM-282, maintainer decision after the #281
  measurement at 680 px). TKM-273 checks the result with the real fonts.
  No scrolling on the dashboard.
