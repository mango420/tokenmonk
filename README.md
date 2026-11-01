# Tokenmonk

Mindful about your tokens. Or not.

A small tray app for macOS and Windows that shows your live Claude Code
token usage and how much of your subscription limit is left, with a bit of
gamification on top.

Status: version 0.1.0 is in development. No release has been published yet.

## What it does

- Lives in the tray (Windows) or the menu bar (macOS). The icon carries a
  halo that fills up: with the 5-hour limit in Max it out, with the daily
  budget in Do it yourself.
- Click it for a small popup: tokens today, current session, burn rate,
  the 5-hour and weekly limit, and a daily budget.
- Two modes, picked on first start and switchable in Settings:
  - **Max it out**: use every token you pay for. XP for burning the limit.
  - **Do it yourself**: lean on Claude Code as little as possible. XP for
    staying under budget.
- XP, levels, titles and achievements. Your existing history counts: on
  first start Tokenmonk levels you up from the logs already on disk.
- OS notifications for unlocks and limit warnings while the popup is
  hidden, an optional unlock sound and optional start at login.

## Screenshots

Screenshots will be added with the first release.

## Privacy

Everything stays on your machine. No account, no login, no telemetry.

- **Usage data** comes from Claude Code's local logs in
  `~/.claude/projects/`. Tokenmonk only reads these files and never changes
  them. Message text is never stored.
- **Local database**: a SQLite file in the app's data directory holds
  per-message usage (token counts, model, timestamps, message, request,
  session and subagent IDs, a sidechain flag, the project folder path),
  the paths of the log files read so far with their read offsets, your
  settings, XP and achievements, internal app state and limit readings.
- **Limit data** comes from Claude Code's status line, if you set it up
  (below). Claude Code passes a JSON object to the status-line command; the
  bridge keeps only the rate-limit part (percent used and reset time for the
  5-hour and weekly window) and writes it to a small file in the app's data
  directory. Nothing else from that input is kept, stored or logged.
- Tokenmonk makes no network requests for usage or limit data, and it never
  edits your Claude Code settings.

## Install

Installers for macOS (universal: Apple silicon and Intel) and Windows will
be attached to the GitHub Releases page of this repository once the first
release is published.

The binaries are **not code-signed** with an Apple or Windows developer
certificate, so both systems warn you the first time you open the app.

### Windows

Microsoft Defender SmartScreen shows a blue dialog titled
"Windows protected your PC". Click **More info** if it is shown, then **Run anyway**.

If Smart App Control is turned on, Windows may block unsigned apps without
offering that option.

### macOS

macOS refuses to open the app the first time because it is not signed or
notarized by Apple. If the warning offers no way to open it, try this:

1. Try to open Tokenmonk once and close the warning.
2. Open **System Settings > Privacy & Security** and scroll down to
   **Security**.
3. If Tokenmonk is listed there, click **Open Anyway**. The button is
   available for about an hour after you tried to open the app.
4. Confirm with your login password.

On Apple silicon Macs, an unsigned app downloaded from the internet may
instead be reported as "damaged" with no **Open Anyway** button. The
current builds are not signed at all, so this message can appear. If you
see it, please open an issue.

## Set up limit data

Token usage works right away. The limit numbers (5-hour and weekly) need
one line in Claude Code's settings, because Claude Code hands them only to
its status-line command.

1. Open Tokenmonk, then **Settings > Limit data**. It shows the exact
   snippet for your installation, with the path to the Tokenmonk
   executable filled in, and a copy button.
2. Paste it into `~/.claude/settings.json`. It has this shape:

   ```json
   {
     "statusLine": {
       "type": "command",
       "command": "<path to the Tokenmonk executable> --statusline"
     }
   }
   ```

3. Send Claude Code one message. The numbers show up after the first reply.

Notes:

- Limit data needs a Claude Pro or Max plan.
- Your Claude Code status line then shows a short summary such as
  `5h 24% | 7d 41%`.
- Already have a status line script? Keep it. Settings shows one line to
  add to your script after it reads its input, which passes the same input
  on to Tokenmonk.
- On Windows, if the path contains a space and Claude Code runs without Git
  Bash, Settings also shows a PowerShell variant of the snippet.

## Contact

Questions, ideas and bug reports: please open a GitHub issue in this
repository. That is the only contact channel.

## License

MIT, see [LICENSE](LICENSE).
