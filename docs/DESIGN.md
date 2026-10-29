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
or the mode name. Text on any fill (`saffron`, `jade`, `pumpkin`) is
`--on-accent`. Before onboarding the accent is saffron (brand colour).

Components use the mode-neutral variables, switched by `data-mode` on
`<html>`:

| Variable | Meaning |
|---|---|
| `--accent` | Fills: mode label, weekly bar, level title, badge ring, gauge |
| `--accent-text` | Accent used as text or thin icon |
| `--halo` | Halo of the header logo and the tray icon |
| `--robe` | Robe of the header logo |
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
| `pumpkin` | #FF8A2B | #E8700E | Halloween only |
| `pumpkin-text` | #FF8A2B | (#E8700E) | Pumpkin as text; the light value is a placeholder, see open points |
| `scrim` | surface-100 at 94 % | surface-100 at 94 % | Ghost overlay background |
| `shadow-popup` | 0 12px 32px rgba(0,0,0,.45) | 0 12px 32px rgba(34,28,22,.14) | Popup and toasts only |

No gradients, no neon glow, no purple.

## Typography

Fonts are bundled from `@fontsource/fraunces`, `@fontsource/manrope` and
`@fontsource/jetbrains-mono` (no CDN, works offline).

| Style | Family | Size / line | Weight | Use |
|---|---|---|---|---|
| display | Fraunces | 40 / 44 | 600 | Level-up screen, onboarding title, wordmark (large) |
| title | Fraunces | 24 / 30 | 600 | Window and section titles ("How do you want to vibe?") |
| heading | Manrope | 16 / 22 | 700 | Card headings, burn and mood line |
| body | Manrope | 14 / 20 | 400 | Default text |
| caption | Manrope | 12 / 16 | 500 | Labels, hints, timestamps |
| counter | JetBrains Mono | 32 / 36 | 600 | The big live number |
| mono | JetBrains Mono | 12 / 16 | 500 | Token counts in lists, XP lines, percentages |

Popup-specific sizes from the screens: wordmark Fraunces 600 18/22, level
Fraunces 600 20/26, stat value JetBrains Mono 600 20/26, ghost title
Fraunces 600 28/32, strong body labels Manrope 700 14/20. Fraunces is never
used for UI labels.

Every live number uses the mono family with `font-variant-numeric:
tabular-nums`.

## Spacing, radius, shadow

- Spacing on a 4 px grid: `space-1` 4, `space-2` 8, `space-3` 12,
  `space-4` 16, `space-6` 24, plus `space-8` 32 (ghost overlay padding).
- Radius: `radius-sm` 6 (inputs, chips, toggles), `radius-md` 12 (cards,
  buttons, popup), `radius-lg` 20 (large panels, achievement tiles,
  onboarding choices), `radius-pill` 999 (mode badges, progress bars).
- One shadow, `shadow-popup`, for the popup and toasts only.

## Popup layout (360 x 680)

The popup is 360 x 680 px, radius 12, background `surface-100`, padding 16,
a single column with a 12 px gap between blocks. Hit targets are at least
44 x 44. Top to bottom:

1. **Header** (height 44): logo button (44 x 44, mark drawn at 36),
   wordmark "Tokenmonk" (Fraunces 600 18/22) above the mode label (caption,
   `--accent-text`, 12 px mode icon at stroke 2, gap 4). During Halloween a
   pill "Spooky season" (caption, pumpkin text on `surface-300`, padding
   4 x 8, pill radius). Settings button 44 x 44, radius 12, 20 px gear icon
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

### Ghost overlay (easter egg)

Three logo clicks, each within 800 ms of the previous one, cover the whole
popup with the overlay: `--scrim`, padding 32, centred column, gap 16.
Ghost mark 128 px; "Hoho, you found me!" (Fraunces 600 28/32); "Secret
achievement unlocked: Boo · +100 XP" (body, pumpkin); button "Back to work"
(min height 44, padding 0 24, radius 12, pumpkin fill, `on-accent` text,
Manrope 700 14). The overlay uses pumpkin in every mode and season. It
works all year.

## Logo and halo

The mark is a monk inside a ring. The ring is the **halo gauge**: filled
clockwise from 12 o'clock with the current value, round line ends.

- `tokenmonk-mark.svg`: full colour on its own `surface-200` disc, halo
  saffron, track `surface-300`. Works on both themes.
- `tokenmonk-mark-mono.svg`: single black ink, track at 30 % opacity. For
  the macOS menu bar (template image) and Windows tray contrast variants.
  Recolour by tinting, never by editing shapes.
- `tokenmonk-ghost.svg`: Halloween and easter egg; ghost instead of monk,
  halo pumpkin.

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
| Halloween | halo, robe and all accents pumpkin | halo pumpkin, everything else stays jade |

The tray icon is rendered in Rust from the mark shapes with the live arc.

## Halloween (24 Oct to 2 Nov)

Season comes from Rust (`AppInfo.season`) and is set as
`data-season="halloween"` on `<html>`.

- Max it out: `pumpkin` replaces `saffron` everywhere (halo, robe, mode
  label, bars, title, badge ring, XP text, gauge).
- Do it yourself: only the halo turns pumpkin; gauge, bars and labels stay
  jade.
- The "Spooky season" pill appears in the header; the ghost mark replaces
  the monk on the about screen.
- Ember still overrides the gauge in danger.

## Iconography

Line icons, 1.75 px stroke (2 px at 12 px size), round caps and joins,
matching the halo's round ends. Sizes 20 (buttons), 16 (badge glyphs), 12
(inline with captions). The mode emojis 🔥 and 🛠️ may appear in UI text.
Achievement badges are circles (mini halos) with one glyph inside.

## Not covered by the design

The two popup screens do not show these; they are not designed yet and need
a design decision before implementation (the frontend should not invent
them):

- Onboarding ("How do you want to vibe?") and settings screens.
- Limit data unavailable or stale (Max it out gauge without a 5-hour value).
- Empty and import states (`noLogs`, `importing`, idle session).
- Toasts for achievements and level-ups, and the level-up screen.
- When the achievement row shows the latest unlock and when an achievement
  in progress.
- The ghost overlay text on repeat triggers, when Boo is already unlocked.
- The header logo in the screens is a simplified inline drawing (disc radius
  44, stroke 10, robe in the accent) rather than `tokenmonk-mark.svg`.
- Light theme `pumpkin-text` value.
