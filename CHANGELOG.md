# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] - Unreleased

First version.

### Added

- Tray (Windows) and menu bar (macOS) icon with a live halo gauge, and a
  popup that opens from it.
- Dashboard: tokens today, current session, burn rate, 5-hour and weekly
  limit, daily budget and level progress.
- Reads Claude Code's local logs read-only and incrementally, follows new
  lines as they are written and keeps usage numbers in a local database.
- Limit data through Claude Code's status line: a `--statusline` bridge
  mode that keeps only the rate limits, plus a copyable setup snippet in
  Settings.
- Onboarding with two modes, Max it out and Do it yourself; the popup
  opens by itself until a mode is picked.
- XP, levels, titles and achievements, including retroactive levelling
  from existing logs, achievement toasts and a level-up screen.
- OS notifications for unlocks and limit warnings while the popup is
  hidden.
- Settings: mode, unlock sound, start at login and limit data setup.
- Halloween season look and a ghost easter egg.

### Changed

- Limit history stores a reading only when it is newer than the last one,
  so idle polling no longer adds rows.

### Fixed

- Limit windows that have already reset are no longer shown with their old
  numbers.
