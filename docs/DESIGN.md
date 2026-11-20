# Design

The design system for the Tokenmonk popup, tray icon and README.
[`src/styles/tokens.css`](../src/styles/tokens.css) is the single source of
truth for every value; this document describes the rules and names the
tokens (`--name`, or the bare name in tables). Components use the
variables, never literal values. Brand files are in
[`assets/brand/`](../assets/brand/). One monk does every job: the app icon
for the OS, the figure inside the popup and the centre of the tray icon
(see "Icon system").

Source: redesign round two, row E (TKM-288), signed off by the maintainer
on the round-two mockups for every screen in light and dark: dashboard
with limits, low limit, no limits yet, level-up, achievement toast,
Settings, About (the onboarding mockup stays on the canvas only; there is
no onboarding screen, see "Product line"). It replaces the prayer-bead
round (TKM-269) everywhere except the tray icon and the app icon, which
stay as they are.
Rules and copy the mockups do not show are derived from the shown parts
and the earlier rules; they are marked "derived" and listed under "Not
covered by the design / out of scope". Where an older rule and row E
disagree, row E wins. Decision log:
[DECISIONS.md](DECISIONS.md), "Redesign round two, row E (2026-11-20,
TKM-288)".

## Product line

Maintainer decision (TKM-287). Where a section below disagrees, this
section wins; superseded parts are marked "Removed by TKM-287". Decision
log: [DECISIONS.md](DECISIONS.md), "Product line (2026-11-16, TKM-287)".

**Core question**: "How much do I have left?"

**Popup, top to bottom**:

1. Header with level and title, always visible (look: "Dashboard",
   TKM-288).
2. Both limit windows, 5-hour and 7-day, at equal size: percent left plus
   reset time.

**Without limits** (not connected, before the first reply, or a plan
without `rate_limits`): tokens today is the full view. No warning, no
estimate, plus one calm line.

**Limit states and their line**:

| State | Popup |
|---|---|
| Connected, limits in | Both windows: percent left and reset time |
| Connected, no limits yet | Tokens today, "Limits after your next reply." |
| Not connected | Tokens today, "Connect limits in Settings." |
| Outdated, no limits | Tokens today, "Reconnect limits in Settings." |
| Plan without `rate_limits` | As "no limits yet" until it can be told apart (open point, derived) |

Not connected covers `LimitsConnection.state` `notConnected` and
`blocked`: the gauge status "Connect limits in Settings." is a text link
to Settings that scrolls to "Limits". `outdated` (the own entry is in
the call form of another shell) with a current reading shows the limits
as usual; without one, "Reconnect limits in Settings." is the same kind
of text link.

**Limit source**: the Claude Code status line (`rate_limits`) only. No
undocumented endpoints. "Connect limits" in Settings: after an explicit
click the app writes the `statusLine` entry into `settings.json` in the
effective Claude folder (TKM-211), with a backup. An existing status line
is wrapped: Tokenmonk reads the limits, then runs the old command with its
output unchanged. "Disconnect" restores the previous entry.

**No estimates anywhere**: no budget, no forecast, no ETA.

**One mode**: Max. Saffron is the only accent.

**Roles**:

- **Tray**: the beads always follow the 5-hour window.
- **Game**: level and title in the popup header; XP and levels as before.
- **Settings**: only Limits (connect, disconnect), Claude folder,
  autostart and updates, notifications, sound, keep popup open.
- **Notifications**: once per window and reset cycle at 10 % left, dry
  tone ("10% left. Resets 16:40."). Level-ups as system notifications,
  with a toggle.

**Wording**: "Limits" everywhere in the popup, Settings, tray and
notifications. "Status line" only where the setup is explained.

**Fed by** (contract TKM-300; rules in ARCHITECTURE.md, Limits, "Connect
limits" and "Notifications"): `LimitsConnection` (`get_limits_connection`,
`connect_limits`, `disconnect_limits`, `limits-connection-changed`) for
the Limits row and the popup's calm line, together with
`DashboardSnapshot.limits`; `Settings.notifyLimits` and
`Settings.notifyLevelUps` for the two notification switches. Accent token:
`--saffron` via `--accent`; there is no `--jade` and no `data-mode`.

Out of scope: usage per project or model, history, API cost, any
estimated budget or forecast.

## Personality and voice

The monk is calm, a little smug and secretly competitive. He never panics,
even with 1 % left.

- **Short and dry.** "Getting thin. Slow down.", not "Warning! You are
  about to run out!"
- **Gamer wink, not cringe.** Titles and achievement names (*Limit
  Chaser*, *Rate-Limit Rambo*) carry the jokes; the rest stays plain.
- **Mode-aware.** Max it out talks like a coach ("Make it count."), Do it
  yourself like a friend with a walking habit ("Go for a walk.").
  Removed by TKM-287: one mode only.
- **Number first.** The dashboard leads with one big number: percent left.
  TKM-287: two numbers of equal size, one per limit window ("Product
  line").
- English UI copy, real typographic quotes and apostrophes (’), no em
  dashes, no emoji. Separators in running copy are commas ("Level 7, 340
  of 920 XP"), not dots.
- Numbers: in running text percent is written with a space (`72 %`) and
  thousands with a comma (`184,230`). Stat values are compact (see
  "Number formats").

## Modes

Removed by TKM-287: Max is the only mode. "Do it yourself", the mode
picker, the onboarding mode choice, the daily budget and the jade accent
go; existing Do it yourself users are switched to Max silently, XP and
levels stay. The rules below are design history only: the app, the
contracts (`Mode` keeps `diy` only for legacy XP rows) and the tokens
(`--jade`, `data-mode`) no longer have them.

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
and previews only). Warm paper and dark wood, one saffron accent.

| Token | Light | Dark | Use |
|---|---|---|---|
| `page` | #CBCAC4 | #131715 | Backdrop outside the popup (previews only) |
| `pop` | #F3EBDD | #1C1814 | Popup background; text on the toast and the primary button |
| `surface` | #EADFCC | #26201B | Header band (dashboard, About); switch track when off |
| `ink` | #241A12 | #F3EBDD | Text and numbers; toast and primary button background; bead and robe outline |
| `ink-2` | #5C4F42 | #BBAE9D | Labels, meta, units, captions, text links |
| `line` | #DCCFBA | #3A322A | Rules and hairlines, level bar track, ground shadow on `surface` |
| `window-ring` | #D6C8B2 | #3A322A | 1 px ring around the popup |
| `saffron` | #EE8A12 | #EE8A12 | The only accent, brand |
| `low-band` | `saffron` 18 % in `pop` | `saffron` 20 % in `pop` | Band behind the limit window that runs out (`color-mix(in srgb, ...)`) |
| `bead-string` | #8C7A66 | #8A7A68 | Bead string line, gauge ring, empty bead outline, switch outline when off |
| `bead-empty-fill` | #FBF6EE | #2A241E | Empty bead and the empty part of a partial bead |
| `monk-face` | #FBF6EE | = `ink` | Monk head fill |
| `monk-line` | = `ink` | = `pop` | Sash, hands, eyes and mouth strokes |
| `monk-head-stroke` | = `ink` | none | Head outline |
| `monk-ground` | #E2D5BF | #2E2620 | Ground shadow under the large monk on `pop` |
| `switch-knob-on` | #FBF6EE | = `pop` | Switch knob, on |
| `switch-knob-off` | #FBF6EE | = `line` | Switch knob, off |
| `toast-ink-2` | #CDBFAE | #5C4F42 | Line under the name on the toast |
| `toast-string` | #8C7A66 | #8C7A66 | String through the toast bead |
| `scrim` | rgba(36,26,18,.45) | rgba(0,0,0,.55) | Dims the popup behind the toast |

- `accent` is always `saffron`. Accents never carry text: they fill
  beads, the robe, the level bar, switch tracks and the low band.
- `bead-empty-stroke` is `bead-string`; the name stays for components
  that already read it.
- Primary button: `button-fill` (= `ink`) with `button-text` (= `pop`),
  so it is dark on light and light on dark.
- Stale limit values: the accent part of every bead takes `--accent` at
  `--stale-opacity` (45 %). One opacity token, no extra colour.
- Contrast (WCAG ratio): `ink` on `pop` 14.4 light / 14.9 dark; `ink-2`
  on `pop` 6.7 / 8.1 and on `surface` 6.0 / 7.4; `ink` on `low-band`
  12.6 / 10.6; `toast-ink-2` on the toast 9.5 / 6.7. Body text, meta and
  captions pass AA in both themes. Empty bead outlines are 3.5 on `pop`
  (3.8 on their own `bead-empty-fill`) light / 4.3 dark; the
  beads repeat the hero number, they never carry information alone.

`shadow-popup`: a 1 px `window-ring` plus `0 24px 48px -28px`
(light: rgba(60,40,20,.4); dark: rgba(0,0,0,.6)). `shadow-toast`:
`0 18px 36px -18px` black at 60 % light / 80 % dark. No gradients, no
glow, no danger colour: beads never turn red, the low band is the
loudest signal, the copy carries the urgency.

## Typography

Two open-licensed families (SIL OFL 1.1), bundled with the app; no
runtime Google Fonts, no CDN, works offline. Decision and licence:
[DECISIONS.md](DECISIONS.md), "Redesign round two, row E".

- `--font-sans` Atkinson Hyperlegible Next, weights 400, 500, 700: all UI
  text.
- `--font-mono` Atkinson Hyperlegible Mono, weights 400, 500: every
  number that is data (percent, token counts, XP, levels, durations,
  times, versions, paths) with `font-variant-numeric: tabular-nums`.
- Line height 1.4 everywhere (`--text-body-line`) unless a style below
  says otherwise. No italics anywhere: not for titles, achievement names
  or emphasis. Emphasis is weight, never slant.

**Loading**: the
fonts come from the npm packages `@fontsource/atkinson-hyperlegible-next`
and `@fontsource/atkinson-hyperlegible-mono`, the same mechanism as the
fonts they replace (`@fontsource/schibsted-grotesk`,
`@fontsource/martian-mono`). `src/main.tsx` imports only the Latin
subsets of the weights above
(`@fontsource/atkinson-hyperlegible-next/latin-400.css`, `-500`, `-700`;
`@fontsource/atkinson-hyperlegible-mono/latin-400.css`, `-500`). Vite
copies the woff2 files into the app bundle, so the font files live in
`node_modules` at build time and in the built `dist/` assets at run
time; nothing is fetched over the network. Packages and imports landed
in TKM-346. The OFL text ships inside each package
(`LICENSE`).

| Style | Family, weight | Size / tracking / line | Use |
|---|---|---|---|
| hero | mono 500 | 64, -0.04em, 1.4 | Percent left per window ("73"), tokens today ("184.2") |
| hero-unit | mono 400 | 28, 0 | "%", "k", "M" inside the hero run |
| hero-label | sans 400, `ink-2` | 16 | "left", "tokens today" |
| levelup-title | sans 700 | 40, line 1.2 | "Level 8" (number mono 500) |
| title | sans 700 | 26 | "Settings", achievements title |
| rank | sans 700 | 22 | Title in the dashboard header ("Quiet Novice") |
| levelup-line | sans 500 | 20 | "Now a Quiet Adept." |
| status | sans 500 | 18 | Closing line on the dashboard |
| toast-title | sans 700 | 18 | Achievement name on the toast |
| brand | sans 700 | 17, -0.01em | "Tokenmonk" wordmark |
| body / row | sans 400 | 16 | Body copy, switch rows, About rows; 700 for button text and Settings row labels |
| section | sans 700 | 15 | Window label ("5 hours", "7 days"), "Achievements" heading |
| link | sans 400, `ink-2` | 15 | Text links in top rows and rows |
| small | sans 400 | 14 | Toast line, About caption |
| meta | mono 400, `ink-2` | 13 | Reset times, "Level 7, 340 of 920 XP", step count |
| caption / note | sans 400, `ink-2` | 13 | Row sub-lines, captions, footer notes |
| stat | mono 500 | 44, -0.04em, line 1.2 | Version on About |

**Text links**: underlined, `text-underline-offset` 3 px
(`--link-underline-offset`), no other decoration, `ink-2`, link size 15,
padding 4 px vertical. Inside About rows and the Settings footer they
take `ink` and the row's size. Hover: `ink` (derived; the mockups show no
hover).

### Number formats

- Percent left: integer, `max(0, round(100 - utilization))`.
- Bead fill (popup, five beads per window, 20 % each): bead i (1 to 5)
  fills `clamp((p - 20 (i - 1)) / 20, 0, 1)` of its height, p the integer
  percent left. 73 % is three full beads, the fourth at 65 %, the fifth
  empty; 8 % is the first bead at 40 %. The tray keeps its own rule
  ("Tray icon").
- Compact token values (tokens today): below 1,000 as is; below
  1,000,000 one decimal plus `k` ("184.2k"); above that one decimal plus
  `M` ("1.2M"). In the hero the suffix is the hero-unit run.
- Reset durations: under 24 h "Hh Mm" ("2h 14m", "0h 41m"); from 24 h
  "Dd Hh" ("3d 4h"). Prefixed "resets in ".
- Week reset: English short weekday of the local reset time ("resets
  Thu"); under 24 h away the duration as above ("resets in 5h 12m",
  derived).
- XP: full numbers with thousands commas ("Level 7, 340 of 920 XP").
- Times: local 24 h ("14:05").

## Spacing, radius, motion

- Spacing: `space-1` 4, `space-2` 8, `space-3` 12, `space-4` 16, `space-5`
  20, `space-6` 24. Screens have `--popup-padding` 24 left and right; top
  and bottom are given per screen. Dashboard sections sit in a column
  with `--popup-gap` 28. Other values (2, 10, 14) are named in the
  screen sections and live in the component CSS as sums of tokens or as
  tokens where they repeat.
- Radius: `radius-popup` 18 (window), `radius-button` 24 (primary
  button, 48 tall: a pill), `radius-toast` 14, `radius-switch` 13,
  `radius-bar` 2, `radius-focus` 4.
- No cards. Blocks are separated by space, the header band (`surface`)
  or 1 px `line` hairlines; the only filled block in the body is the low
  band. No uniform card stacks, no filler icons, no emoji.
- Level bar: radius 2, track `line`, fill `--accent`, `--bar-height` 4.
- Primary button: full width, `--button-height` 48, `radius-button` 24,
  `button-fill` with `button-text`, body size, weight 700, no border. One
  per screen at most (level-up).
- Focus: `:focus-visible` outline 2 px `--accent`, offset 3, radius 4.
- Motion, allowed and nothing else (derived from the earlier rules; the
  mockups are still):
  - Bead fill: the first time the dashboard renders after start, the
    beads of each window fill left to right, `--motion-bead-stagger` 45 ms
    apart, each in `--motion-bead` 260 ms, `--ease-bead`
    `cubic-bezier(.2,.8,.2,1)`, from empty to their value.
  - Bead change: later values move the fill level of the affected beads
    in 260 ms, same easing. A fresh window jumps to full.
  - Toast slide: up from below the popup edge (`translateY(140%)` to 0)
    in 280 ms, `cubic-bezier(.2,.8,.2,1)`, and back down when it leaves;
    the scrim fades in and out in the same 280 ms.
  - Switch knob: slides 18 px in 150 ms.
  - With `prefers-reduced-motion: reduce` all of them are off: beads show
    their final state, toast, scrim and knob appear in place.

Sound: one short synthesized tone per unlock or level-up (generated in the
UI, no audio file). It plays only when `Settings.sound` is on, once per
event, whether the popup is open or hidden.

## Icon system

One monk everywhere: closed smiling eyes, a sash across the robe, hands
resting at the hem. The monk and the beads are the only drawings in the
app; no other icons.

### The monk drawing

All parts share one coordinate system: the 280 x 280 gauge viewBox
(centre 140, 140), so the monk sits inside the bead gauge without a
transform. The corner mark crops it with `viewBox="76 76 128 146"`; the
large monk with its ground shadow uses `viewBox="60 76 160 160"`. Lines
have round caps; the robe outline has a round join.

| Part, in drawing order | Path | Fill | Stroke |
|---|---|---|---|
| Ground (large monk only) | ellipse cx 140, cy 220, rx 74, ry 13 | `--monk-ground` on `pop`, `line` on `surface` | none |
| Robe | `M82 216c2-46 26-74 58-74s56 28 58 74z` | `--accent` | `--ink` |
| Sash | `M112 152c10 22 30 40 62 46` | none | `--monk-line` |
| Hands | `M116 216c6-10 16-14 24-14s18 4 24 14` | none | `--monk-line` |
| Head | circle cx 140, cy 106, r 24 | `--monk-face` | `--monk-head-stroke` (light `ink`, dark none) |
| Eyes | `M128 108c2 3 6 3 8 0M144 108c2 3 6 3 8 0` | none | `--monk-line` |
| Mouth | `M135 118c3 2 7 2 10 0` | none | `--monk-line` |

`--monk-line` is `ink` in light and `pop` in dark, so the sash and the
face stay dark on the saffron robe and the light dark-theme head.

Stroke widths are viewBox units, chosen so every size draws about 1 to
1.5 px lines: corner mark `--mark-stroke` 5, large monk and level-up
gauge `--monk-stroke` / `--gauge-bead-stroke` 2, About gauge
`--gauge-sm-stroke` 3.

### Corner rule

The top-left corner names the app on every screen:

- **Monk mark plus wordmark** on the dashboard (all states), the toast
  (it sits over the dashboard) and Settings (and Achievements, derived):
  the monk drawing at `--mark-width` 26 x `--mark-height` 30, inline SVG
  with token colours, then "Tokenmonk" in brand style, gap 10
  (`--mark-gap`), items centred. On sub-screens "Back" sits where
  "Settings" sits on the dashboard.
- **Wordmark alone** where the large monk is on screen: level-up and
  About.

The mark is drawn from the parts above, never from the app icon files; no
tile behind it.

### Roles and files

| Role | Source | Colour | Where | Size |
|---|---|---|---|---|
| A. App icon | `tokenmonk-icon.svg`, `tokenmonk-icon-windows.svg` | Fixed in the files: dark tile, saffron robe | Only where the OS shows the app: taskbar, dock, Start menu, installer, notifications, the generated icon set. Never inside the popup. | Platform sizes from `node scripts/icons.mjs` |
| B. Popup monk | The parts table above, drawn inline | Tokens: follows theme | Corner mark; large monk on the dashboard without limits; inside the five-bead gauge on level-up and About | Mark 26 x 30; large 120 (`--monk-lg-size`); gauge 200 (`--gauge-size-lg`), About 112 (`--gauge-size-sm`) |
| C. Tray monk | Rendered in Rust from robe and head | Robe `--tray-accent`, head `--tray-track` | Inside the tray beads | 11.5 x 13 units in a 32-unit box |
| Brand files | `tokenmonk-monk-light.svg`, `tokenmonk-monk-dark.svg`, `tokenmonk-monk-mark.svg` | Fixed in the files | README, docs, release pages | Any |

Files in `assets/brand/` (unchanged by TKM-288; they keep the earlier
palette, like the app icon and the tray), all with
`viewBox="76 76 128 146"`:

- `tokenmonk-monk-light.svg`: colour on light. Robe #E39A0B stroked
  #181A1B; sash, hands, eyes, mouth #181A1B; head #F4F3EF stroked #181A1B.
- `tokenmonk-monk-dark.svg`: colour on dark. Robe #E39A0B stroked #E8E7E0;
  sash, hands, eyes, mouth #1D2220; head #E8E7E0, no stroke.
- `tokenmonk-monk-mark.svg`: single-colour mark. Robe and head #181A1B,
  sash knocked out in #E4E3DD at stroke 3 (`--mark-sash-stroke`), no
  hands and no face details.
- `tokenmonk-mark-mono.svg`: the old monk, legacy, until a cleanup removes
  it. Do not use it for anything new.

- **A. App icon**: the monk on the dark ink tile. Details in "App icon".
- **B. Popup monk**: the parts above with token colours, no tile and no
  background (the ground shadow is part of the large monk).
- **C. Tray monk**: robe and head only; sash, hands and face are below
  pixel size there. Details in "Tray icon".

## Popup layout (360 x 680)

The popup is 360 x 680 px, radius 18, background `pop`, `shadow-popup`,
overflow hidden. Each screen is a column, `--popup-padding` 24 left and
right; top and bottom per screen. Actions are text links, at most one
primary button, or switches. The only drawings are the monk and the
beads. Nothing on the dashboard scrolls.

### Shared parts

- **Top row**: space-between, items centred, min height 32
  (`--top-row-height`). Left: the corner (see "Icon system", "Corner
  rule"). Right: one text link at link size, `ink-2` ("Settings" on the
  dashboard, "Back" on sub-screens; Esc does what "Back" does). On
  level-up the right side stays empty.
- **Window section**: one per limit window, a column:
  1. Label row, space-between, baseline: label in section style ("5
     hours", "7 days") left, reset time in meta style right ("resets in
     2h 14m", "resets Thu"; empty while the reset time is `null`).
  2. Hero row, margin top 4, baseline, gap 10: the hero run (number in
     hero style, `%` in hero-unit style, one mono run, `tabular-nums`),
     then "left" (hero-label).
  3. Bead string, margin top 8 (see below).
- **Bead string**: inline SVG `--bead-string-width` 312 x
  `--bead-string-height` 48, `viewBox="0 0 312 48"`, full content width.
  - String: `M2 24H310`, stroke `--bead-string`, width 1.5
    (`--bead-stroke`).
  - Five beads (`--bead-count`), r 17 (`--bead-radius`), centres at
    x 31, 93.5, 156, 218.5, 281 (`--bead-pitch` 62.5), y 24. Bead 1 is
    on the left, each bead is 20 % of the window.
  - Full bead: fill `--accent`, stroke `--ink` 1.5.
  - Partial bead: base circle in `--bead-empty-fill`, then the accent
    part as a circular segment filling the bead from the bottom to the
    bead's fill fraction f ("Number formats"): the chord sits at
    `y = 24 + 17 - 34 f`, path `M x1 y A 17 17 0 L 0 x2 y Z` with
    `x1, x2 = cx -/+ sqrt(17² - (y - 24)²)` and the large-arc flag L = 1
    when f > 0.5, else 0 (sweep 0: the arc runs through the bottom).
    Then the outline: stroke `--ink` 1.5, no fill.
  - Empty bead: fill `--bead-empty-fill`, stroke `--bead-empty-stroke`
    (`bead-string`) 1.5.
  - The string drains from the right: bead 5 goes first, bead 1 last.
  - Stale values: the accent fills at `--stale-opacity`.
- **Five-bead gauge** (level-up, About): inline SVG,
  `viewBox="0 0 280 280"` (`--gauge-viewbox`), centred. Ring: circle at
  (140, 140), r 118 (`--gauge-radius`), stroke `--bead-string`, no fill.
  Five beads r 17 at angle `-90° + (i - 1) * 72°` (`--bead-start-angle`,
  `--bead-step-angle`): (140, 22), (252.2, 103.5), (209.4, 235.5),
  (70.6, 235.5), (27.8, 103.5); all full (fill `--accent`, stroke
  `--ink`). The monk inside, same coordinates, no ground. Strokes:
  level-up ring 1.5 (`--gauge-track-stroke`), beads and monk 2
  (`--gauge-bead-stroke`); About ring 2.5 (`--gauge-sm-track-stroke`),
  beads and monk 3 (`--gauge-sm-stroke`). The tray icon uses the same
  five-bead idea at its own geometry ("Tray icon").
- **Level bar**: track `line`, fill `--accent`, radius 2, height 4
  (`--bar-height`), full content width, width = `GameState.progress`.
- **Hairline list**: rows full width with a 1 px `line` rule on top; the
  last row also has one at the bottom. No fill, no radius.
- **Primary button**: see "Spacing, radius, motion".

### Dashboard

`View = dashboard`. Two parts: the header band and the body.

**Header** (background `surface`, padding 16 24 20, column, gap 14):

1. Top row: monk mark plus wordmark left, the text link "Settings"
   right.
2. Rank block (column, gap 2): `GameState.title` in rank style ("Quiet
   Novice"), then "Level 7, 340 of 920 XP" in meta style (`level`,
   `xpIntoLevel`, `xpForLevel`). History replay
   (`GameState.retroDone = false`): the meta line reads "Counting old XP.
   Hold still." (derived). The rank block is the button that opens
   Achievements (derived: the level card that did this is gone); no
   underline, focus ring as everywhere.

The header shows in every dashboard state. No level bar in the header.

**Body** (padding 24, column, gap 28 `--popup-gap`), with limits:

1. Window section "5 hours" (`limits.fiveHour`).
2. Window section "7 days" (`limits.weekly`).
3. Spacer (`flex: 1`).
4. Closing line (status style), first match:
   1. Week runs out first: weekly percent left < 5-hour percent left and
      weekly percent left < 20 (both integers as shown, both windows
      set): "The week runs out first."
   2. `fiveHour` is `null`, `weekly` set: "Fresh window. Numbers on your
      next reply." (derived).
   3. Otherwise by 5-hour percent left:

      | Percent left | Line |
      |---|---|
      | 60 and more | "Barely touched. Use it." |
      | 30 to 59 | "Good pace." |
      | 10 to 29 | "Nearly there. As planned." |
      | 1 to 9 | "Almost gone. Make it count." |
      | exactly 0 | "Limit reached. Breathe." |
5. Note line (derived, see "Note line"), margin top 8 after the closing
   line (0 in the tight state, see "Low limit"). With no note the line
   is not rendered at all; the spacer takes the freed height.

Both windows have equal size; neither is the hero over the other. A
window section shows while its value is set; with only one set, the
other is left out (derived). With neither set the no-limits layout
applies.

### Low limit

When a window's percent left is under 10 (0 to 9; derived threshold, the
same 10 % as the limit notification), its section becomes the low band:
background `--low-band`, bleeding to the popup edges (margin 0 -24,
padding 16 24). Inside the band the label reads "5 hours, nearly out" /
"7 days, nearly out" (1 to 9 %; at exactly 0 the plain label stays,
derived: the closing line says "Limit reached. Breathe."), and the meta
and "left" take `ink` instead of `ink-2`. Beads and numbers as usual.
Nothing else changes: no red, no icon, no motion. Both windows can be in
the band at once (derived).

**Tight state.** When both windows are in the low band and a note line
shows, the body does not fit 680 px with the normal spacing. Only in
that state the dashboard tightens:

- Body gap 28 (`--popup-gap`) becomes 16 (`--space-4`).
- The note line's margin top 8 becomes 0.

Height budget for that state (from the tokens):

| Part | Normal | Tight |
|---|---|---|
| Header band | 133 | 133 |
| Body (padding, two low bands, closing line, note line, gaps) | 588.6 | 544.6 |
| Total | 721.6 (41.6 over 680) | 677.6 (2.4 spare) |

The saving is 44 px: 3 body gaps x 12 = 36, plus the note margin 8.
Everything else stays as is: band padding, hero number size, gaps in all
other states. Nothing scrolls. With both windows low but no note line,
the normal spacing fits and stays.

### Dashboard without limits

Covers every state in "Product line" without limit values: not
connected, connected before the first reply, outdated without a reading,
plan without `rate_limits`, and `limits.status = unavailable`.

Header as above. Body (padding 24, column, centred, text centred):

1. Spacer (`flex: 1`).
2. Large monk, 120 x 120 (`--monk-lg-size`), with its ground in
   `--monk-ground`, strokes 2.
3. Tokens today, margin top 24: `today.total` compact in the hero run
   ("184.2" hero, "k" hero-unit; below 1,000 no suffix).
4. "tokens today" (hero-label).
5. A short rule: 48 px wide, 1 px `line`, margin 24 0.
6. Closing line (status style), by the "Product line" table: "Limits
   after your next reply." (no limits yet, plan without `rate_limits`,
   `unavailable`), "Connect limits in Settings." (not connected,
   blocked), "Reconnect limits in Settings." (outdated). The last two
   are text links in the same style (underlined, offset 3) that open
   Settings scrolled to Limits.
7. Spacer (`flex: 1.3`), so the group sits a little above centre.
8. Note line (derived), centred.

### Dashboard states

- **Stale** (`limits.status = stale`, derived): values as usual; the hero
  numbers in `ink-2`; bead accent fills at `--stale-opacity`; the reset
  meta of the 5-hour section reads "as of 14:05" (`fetchedAt`, local
  24 h time).
- **No logs** (`data.state = noLogs`): token numbers are 0; the windows
  follow the limits as usual; the note line explains it.
- **History replay**: see the header.

All numbers and copy in examples are samples; real values come from
`DashboardSnapshot` and `GameState`.

### Note line

Not in the row E mockups; kept from the earlier design so status and
update messages keep a place (derived, open point). Caption style
(13, `ink-2`), one line, min height 18, under the closing line. First
match wins; when nothing matches the line is left out: no empty
paragraph, no reserved 18 px, no margin (the note is `null`; the spacer
absorbs the height):

| Condition | Text |
|---|---|
| `data.state = importing` | "Reading logs, 24 of 38 files." (numbers mono) |
| `data.state = noLogs` | "No Claude Code logs yet. Start a session." |
| `data.state = error` | "Can’t read the logs. Retrying in the background." |
| `UpdateStatus.kind` is `available`, `downloading`, `installing` or `installFailed` | the update line, see "Update line" |

The limit note ("Limit % includes claude.ai chats. Token stats are
Claude Code only.") is not shown on the dashboard any more (derived:
row E has no room for a permanent footnote); it stays in the "Limits"
row captions in Settings.

### Update line

The dashboard's only trace of updates, shown in the note line. Note
style; the version is mono (`tabular-nums`), version first; actions are
text links in the same line (gap 8). No icon, no badge, no toast.

| `UpdateStatus.kind` | Text | Action |
|---|---|---|
| `available` | "0.4.1 is out. Update" | "Update" calls `install_update` |
| `downloading`, `percent` set | "Downloading 0.4.1, 42%." | none |
| `downloading`, `percent` null | "Downloading 0.4.1." | none |
| `installing` | "Installing 0.4.1. Back in a moment." | none |
| `installFailed` | "0.4.1 didn’t install. Try again" | "Try again" calls `install_update` |

All other kinds (`idle`, `checking`, `upToDate`, `unavailable`,
`checkFailed`) show nothing here. "Couldn’t check" lives in Settings
only. While `install_update` runs the link is disabled.

Download progress (TKM-212): after "Update" the line goes `downloading`
(percent null, then 0 to 100), then `installing`. The percent is a live
number: mono, `tabular-nums`, the "%" inside the same mono span, last in
the line so nothing after it shifts as it grows. It is the whole percent
from Rust, shown as is: no decimals, no animation between values, no
progress bar, no spinner. Percent null drops the number and the comma.
Rust emits only on whole-percent changes, so the UI renders every event
as it comes. Windows: the installer's own small progress window appears
and Tokenmonk quits, then starts again. macOS: the app restarts by
itself. If `install_update` rejects (`CommandError`), re-read
`get_update_status`; no copy for it.

Version numbers come from `UpdateStatus.version` ("0.4.1" is a sample).

### Settings

`View = settings`, opened by "Settings" on the dashboard or by the tray
menu (`navigate`). Back: the "Back" link or Esc. Scrolls vertically when
taller than the popup; the top row stays. Opened from the dashboard's
limits line, it scrolls to "Limits". No header band: background `pop`,
padding 16 24 20.

1. Top row: monk mark plus wordmark left, "Back" right.
2. Title "Settings" (title style, margin 16 0 16).
3. **Rows** (hairline list; rows padding 12 0, space-between, items
   centred, gap 12). Left a column (gap 2): label (row size, weight 700)
   over a sub-line (caption, `ink-2`); right one or two text links (gap
   12).
   - **Limits**. Sub-line and links by `LimitsConnection.state`. Every
     state has one link except `outdated`, which has two, in the order
     listed, in one line (gap 12):

     | State | Sub-line | Links |
     |---|---|---|
     | `notConnected` | "Not connected." | "Connect limits" (`connect_limits`) |
     | `connected`, `limits.status` not `unavailable` and `fetchedAt` set | "Connected, last reading 14:05" (`fetchedAt`, local 24 h, mono `tabular-nums`) | "Disconnect" (`disconnect_limits`) |
     | `connected`, otherwise | "Connected. Limits after your next reply." | "Disconnect" |
     | `outdated` | "Connected, but set up for another shell. Reconnect to fix it." | "Reconnect" (`connect_limits`), "Disconnect" (`disconnect_limits`) |
     | `blocked`, `problem = noClaudeFolder` | "Can’t connect: the Claude folder isn’t there. Pick it below." | "Try again" (`connect_limits`) |
     | `blocked`, `problem = settingsUnreadable` | "Can’t connect: Claude Code’s settings.json can’t be read." | "Try again" |
     | `blocked`, `problem = settingsInvalid` | "Can’t connect: settings.json isn’t valid JSON. Fix it, then try again." | "Try again" |
     | `blocked`, `problem` null | "Can’t connect right now." | "Try again" |

     Captions (derived: not in the mockup; caption style, full row width
     under the label row, margin top 8, gap 4), `LimitsConnection.detail`
     never shows:
     - `notConnected` only: "Limits come from Claude Code’s status line.
       Connect adds it to settings.json, with a backup." The file name is
       `LimitsConnection.settingsPath` (mono, `ink`), "settings.json"
       while it is `null`. If `previousStatusLine`, it goes on: " Your
       current status line keeps working, output unchanged."
     - `connected` with `previousStatusLine`: "Your own status line runs
       on, output unchanged. Disconnect restores it."
     - `outdated` with `previousStatusLine`: "Your own status line stays
       wrapped; Reconnect keeps it."
     - Always: "Needs a Pro or Max plan. Numbers show up after the first
       reply; the desktop app may not run the status line." (Hedged on
       purpose: see DECISIONS.md, "Popup with log-only data".) Then "Limit
       % includes claude.ai chats. Token stats are Claude Code only."

     While a call runs the links are disabled. Connect and Disconnect only
     ever run on that click (ARCHITECTURE.md, "Connect limits"). On a
     `CommandError` the caption "Couldn’t change that." appears under
     the row until the next try; the state follows
     `limits-connection-changed`.
   - **Claude folder**. Sub-line: the path (meta style, mono 13,
     `ink-2`, one line): longer than 38 characters loses its middle to
     "…", the full path is the tooltip; "…" while `get_claude_folder`
     has not answered, "Unknown" if `ClaudeFolder.path` is `null`. Link
     "Change" opens the native folder picker (ARCHITECTURE.md, "Claude
     folder") and sends `update_settings { patch: { claudeFolder } }`;
     cancel does nothing. Derived (not in the mockup): when
     `ClaudeFolder.source` is `setting` a second link "Reset" sends
     `claudeFolder: null`; the source shows as a caption under the row
     ("Set here", "From CLAUDE_CONFIG_DIR", "Default"); if
     `ClaudeFolder.projectsFound` is false, the caption "No logs here
     yet." follows (a hint, never an error). On a rejected change:
     "Couldn’t save that. Try again." under the row.
4. **Switches** (hairline list, margin top 20; rows padding 9 0,
   space-between, items centred, label in row style left, switch right,
   `role="switch"`), in this order:
   - "Start at login"
   - "Check for updates" (`Settings.updateCheck`, default on). Off: no
     check at start; "Check now" still works. Turning it on does not
     check at once.
   - "Keep popup open" (`Settings.keepOpen`, default off). Off: the popup
     hides when it loses focus. On: it stays visible and hides only via
     the tray icon or the title bar "Close". Applies at once.
   - "Warn at 10 % left" (`Settings.notifyLimits`; "10 %" mono): once per
     window and reset cycle (ARCHITECTURE.md, "Notifications").
   - "Level-up notifications" (`Settings.notifyLevelUps`).
   - "Sound"

   No captions under the switches. Switch: 44 x 26 (`--switch-width`,
   `--switch-height`), radius 13, 1.5 outline (`--switch-stroke`); knob
   17 round (`--switch-knob`), inset 4.5, 1.5 outline. On: track
   `--accent` outlined `ink`, knob `--switch-knob-on` outlined `ink`, on
   the right. Off: track `surface` outlined `bead-string`, knob
   `--switch-knob-off` outlined `bead-string`, on the left. The knob
   moves 18 px.
5. Spacer (`flex: 1`, min height 16).
6. **Update row** (derived, not in the mockup; note style, margin bottom
   4), directly above the footer: status text, then text links (gap 12).
   Versions and times mono (`tabular-nums`), times local 24 h. By
   `UpdateStatus.kind`:

   | Kind | Text | Links |
   |---|---|---|
   | `idle` | none | "Check now" |
   | `checking` | "Checking for updates." | none |
   | `upToDate` | "Up to date, checked at 14:05." (`checkedAt`) | "Check now" |
   | `available` | "0.4.1 is out." | "Update", "Release notes" |
   | `downloading`, `percent` set | "Downloading 0.4.1, 42%." | none |
   | `downloading`, `percent` null | "Downloading 0.4.1." | none |
   | `installing` | "Installing 0.4.1. Back in a moment." | none |
   | `installFailed` | "0.4.1 didn’t install." | "Try again", "Release notes" |
   | `checkFailed` | "Couldn’t check." | "Check now" |
   | `unavailable` | "Updates aren’t set up in this build." | none |

   "Check now" calls `check_for_updates`, "Update" and "Try again" call
   `install_update`, "Release notes" opens `UpdateStatus.url` in the
   browser. `downloading` follows the dashboard's update line rules. On
   a `CommandError` from `check_for_updates` the row reads "Couldn’t
   check.". The tray item "Check for updates" lands here and fills the
   row the same way.
7. **Footer** (note style, row, baseline, gap 12): "Tokenmonk 0.6.0,
   MIT." (`AppInfo.version` mono `tabular-nums`; while empty "Tokenmonk,
   MIT.") then the text link "About" in `ink`, which opens About.

Every switch applies at once through `update_settings { patch }`, no save
button. While a call runs the control stays in its new position; on
`CommandError` it flips back and "Couldn’t save that. Try again."
(caption) appears under the switch list (derived).

Fed by: `LimitsConnection` (`get_limits_connection`, `connect_limits`,
`disconnect_limits`, `limits-connection-changed`; `state`,
`settingsPath`, `previousStatusLine`, `problem`),
`DashboardSnapshot.limits` (`status`, `fetchedAt`), `ClaudeFolder`
(`get_claude_folder`, read again on `settings-changed` and
`app-info-changed`), `Settings`, `update_settings`, `settings-changed`,
`AppInfo.version`, `NavigateTo`, `UpdateStatus` (`get_update_status`,
`update-status-changed`), `check_for_updates`, `install_update`.

### About

`View = about` (TKM-125), opened by the "About" link in the Settings
footer. No other entry point. Back: the "Back" link or Esc, both return
to Settings (opened at the top). A `navigate` event while About is open
switches views like on any other screen.

**Header band** (background `surface`, padding 16 24 24, column, items
centred, text centred):

1. Top row (full width): wordmark alone left, "Back" right.
2. Five-bead gauge at 112 px (`--gauge-size-sm`), margin top 12, all
   beads full, the monk inside.
3. Version, the screen's number: `AppInfo.version` as is ("0.6.0" is a
   sample), stat style, margin top 16. While it is still empty: "n/a" in
   the same style (derived).
4. "Installed version. Updates live in Settings." (small, `ink-2`,
   margin top 4).

**Body** (padding 20 24, column):

1. Hairline list, rows space-between, items centred, row size:

   | Label | Value | Row padding |
   |---|---|---|
   | "Licence" | "MIT" (`ink-2`, plain text) | 12 0 |
   | "Source" | text link "GitHub" (`ink`), opens `REPO_URL` | 8 0 |
   | "Ideas and bugs" | text link "GitHub issues" (`ink`), opens `ISSUES_URL` | 8 0 |

   The links open in the browser through the opener plugin; a failed
   open shows nothing.
2. Spacer (`flex: 1`).
3. Note: "Made by Tokenmonk contributors. No account, no telemetry."

Nothing else: no author, no email, no website, no credits list, no
changelog, no update controls. Only the two GitHub links leave the app.

Fed by: `AppInfo.version` (`get_app_info`, `app-info-changed`),
`REPO_URL`, `ISSUES_URL`.

### Achievements

Not in the row E mockups; derived from Settings (open point).
`View = achievements`, opened by the rank block in the dashboard header.
Back: the "Back" link or Esc. Scrolls vertically when needed. No header
band: background `pop`, padding 16 24 20.

1. Top row: monk mark plus wordmark left, "Back" right.
2. `GameState.title` (title style, margin top 16).
3. "Level 7, 340 of 920 XP" (meta, margin top 2).
4. Level bar, margin 12 0 24.
5. "Achievements, 2 of 5" (section style, margin bottom 8): unlocked
   count of the listed rows, then the listed count.
6. Hairline list, one row per listed achievement in display order
   (`GameState.achievements`): padding 12 0, space-between, gap 12.
   Left: name (row size, weight 700) over description (caption). Right:
   "Unlocked" or "Locked" (caption, no wrap). Locked rows show name and
   description at `--locked-opacity` 50 %.

Listed: every achievement except secret ones that are still locked. Boo
("Halloween 2026") is a retired legacy badge: secret and no longer
earnable, so it is listed only for players who already unlocked it.

Fed by: `GameState` (`title`, `level`, `xpIntoLevel`, `xpForLevel`,
`progress`, `achievements`).

### Achievement toast

Shown for every `achievement-unlocked` with `retroactive = false`, inside
the popup on any screen (the mockup shows it over the dashboard).

- **Scrim**: absolute, inset 0, `--scrim`, over the screen below the
  toast.
- **Toast**: a button, absolute, inset 16 (`--toast-inset`) from left,
  right and bottom; background `ink`, text `pop`, radius 14, padding
  16 18 16 16, `shadow-toast`; row, items centred, gap 14, text left.
  1. One bead on a string: SVG 24 x 48 (`viewBox="0 0 24 48"`), string
     `M12 0V48` in `--toast-string` 1.5, bead at (12, 24) r 10
     (`--toast-bead-radius`), fill `--accent`, stroke `pop` 1.5.
  2. Text column (`flex: 1`, gap 2): the name (toast-title) over one line
     (small, `--toast-ink-2`): `detail`, else the achievement's
     `description`.
  3. "+50 XP" (mono 14, `tabular-nums`, `pop`).

Rules:

- One toast at a time, visible 3.4 s (`--toast-visible`), a click on the
  toast or the scrim dismisses (scrim: derived), pointer hover pauses the
  timer. It slides in and out (see Motion).
- Queue: events that arrive while the popup is hidden are kept and shown
  on the next open, oldest first, in addition to the OS notification.
  Three or more waiting collapse into one toast: "3 achievements
  unlocked", line "Newest: Night Owl.", right "+150 XP" (summed XP;
  derived layout).
- Boo can no longer be unlocked, so it never gets a toast.
- Priority: level-up screen, then toasts. A toast waits while the
  level-up screen is open.
- Sound: the synthesized tone, only with `Settings.sound` on.

While hidden: achievement unlocks also go out as OS notifications
through the notification plugin; the in-popup queue still shows on the
next open. No OS notification for `retroactive = true`. The notification
is sent without its own sound where the plugin allows it, so the tone is
not doubled.

| Notification | When | Title | Body |
|---|---|---|---|
| Unlock | `achievement-unlocked`, not retroactive | "Night Owl unlocked, +50 XP" | detail, else description |

Limit and level-up notifications: ARCHITECTURE.md, "Notifications"; copy
and tone in "Product line".

Fed by: `AchievementUnlocked`, `GameState.achievements`, `Settings.sound`;
the limit notification from `DashboardSnapshot.limits` and
`Settings.notifyLimits` (ARCHITECTURE.md, "Notifications").

### Level-up screen

Shown for every `level-up` with `retroactive = false`, over the current
screen, queued like toasts. Background `pop`, padding 20 24 24, column,
items centred, text centred.

1. Wordmark alone, top left (`align-self: flex-start`).
2. Spacer (`flex: 1`).
3. Five-bead gauge at 200 px (`--gauge-size-lg`), all beads full, the
   monk inside. No bead motion.
4. "Level 8" (levelup-title, margin top 24; "Level" sans 700, the number
   mono 500 `tabular-nums`).
5. Several levels at once (`level - previousLevel > 1`): "+2 levels"
   (meta, margin top 4; derived, not in the mockup).
6. Title line (levelup-line, margin top 8) from `LevelUp.title`:
   `titleChanged = true` "Now a <title>." ("Now a Quiet Adept."),
   `titleChanged = false` "Still a <title>.". The article is "an"
   before a vowel sound ("Now an Abbot of Restraint.").
7. "Keep burning." (body, `ink-2`, margin top 2).
8. Spacer (`flex: 1`).
9. "Level 8, 0 of 1,040 XP" (meta, full width, left aligned).
10. Level bar, full width, margin top 8.
11. Primary button "Back to work", full width, margin top 20.

Esc and the button close it. Tone only with `Settings.sound` on.

Retro summary: when `retroDone` turns from false to true in this run, the
screen shows once with "Level 6", the line "Straight from your history."
and "4 achievements, 2,640 XP" (unlocked count, `xpTotal`) in the place
of "Keep burning.". Skipped at level 1 with no unlocks. No toasts for
retroactive unlocks.

Fed by: `LevelUp`, `GameState`, `Settings.sound`.

## Tray icon

Rendered in Rust (resvg) at runtime; `tokens.css` mirrors its colours and
opacities. A 32 x 32 unit box (`viewBox="0 0 32 32"`):

- **Beads**: 5, one per 20 %, r 3.2, centres in this order: (16, 4),
  (27.4, 12.3), (23.1, 25.7), (8.9, 25.7), (4.6, 12.3). Bead 1 is at
  12 o'clock, the rest follow clockwise. Count from the 5-hour percent
  left: 0 only at exactly 0 %, otherwise `ceil(percent / 20)`. Beads 1 to
  N are full, the rest empty: the string drains from the end, as in the
  popup. The tray keeps whole beads; the popup fills partial beads.
  - Full: fill `--tray-accent` #E39A0B, no stroke (derived).
  - Empty: same circle, fill `--tray-track` #8A8D8B at 35 %
    (`--tray-bead-empty-opacity`), no scale (derived).
- **Monk**: robe and head of the monk drawing ("Icon system"), nested as
  `<svg x="10.25" y="9.5" width="11.5" height="13" viewBox="76 76 128 146">`.
  Robe fills `--tray-accent`, head fills `--tray-track`
  at full opacity; no strokes, no sash, hands or face (derived: below
  pixel size).
- **No limit data** (no 5-hour value): all beads empty and the whole monk
  at opacity 0.45 (`--tray-monk-nodata-opacity`).
- **macOS**: a template image in black (alpha carries the shape; the menu
  bar tints it). Full beads and robe black; empty beads at 30 %
  (`--tray-bead-empty-opacity-template`); head at 40 %
  (`--tray-head-opacity-template`). The no-data fade (0.45) applies to the
  monk here too.

Value: 5-hour percent left. The tray keeps the earlier saffron #E39A0B
(`--tray-accent`, unchanged by TKM-288); the popup uses `--saffron`
#EE8A12. The tray icon is not the app icon (role C in "Icon system"): it
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

Out of scope (TKM-287): usage per project or model, history, API cost,
any estimated budget or forecast. Out of scope (TKM-288): the app icon,
the tray icon and the brand files; they keep the earlier palette
(saffron #E39A0B).

Open points from the product line (TKM-287):

- How to tell a plan without `rate_limits` (API, gateway) reliably from
  "not yet", so the hint line can disappear for those users.
- Whether the desktop app and IDE extensions run the status line
  (verified in TKM-242).

Open points from row E (TKM-288). Each rule below is marked "derived"
above and stands until the maintainer decides otherwise:

- Note line and update line on the dashboard: not in the mockups; kept
  under the closing line. The permanent limit note moved to Settings.
- Low band: threshold under 10 % and the plain label at exactly 0 %.
- Only one window set (fresh 5-hour window): the other section is left
  out and the closing line reads "Fresh window. Numbers on your next
  reply.".
- Stale values: hero in `ink-2`, "as of 14:05" in the 5-hour meta.
- The rank block in the header opens Achievements; the Achievements
  screen layout follows Settings.
- Settings: captions under Limits and Claude folder, "Reset", the update
  row and the error captions are not in the mockup; they keep their
  earlier content in row E styles. The switch captions are gone, as in
  the mockup.
- Toast: scrim click dismisses; the collapsed toast layout.
- Level-up: the "+2 levels" line.
- Motion and hover states: the mockups are still; the rules in
  "Spacing, radius, motion" carry over from the earlier design.
