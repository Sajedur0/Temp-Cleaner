# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Legend: **Added** · **Changed** · **Fixed** · **Safety** · **Removed**

---

## [Unreleased]

Nothing yet — ideas and bug reports are welcome in
[Issues](https://github.com/Sajedur0/Temp-Cleaner/issues).

---

## [1.0.0] — 2026-11-04

First complete release: a safe GUI cleaner with a full scripting CLI.

### Added

- Fixed project layout (`src/main.rs`) so `cargo build` works out of the box.
- **Scan → confirm → clean GUI flow**: a dialog shows the per-category
  breakdown (files + size) before anything is deleted, and the result toast
  reports files deleted, space freed and files skipped.
- **Full CLI**: `--dry-run`, `--yes`, `--silent`, `--verbose`, `--min-age`,
  `--only`, `--skip`, `--windows-update`, `--no-recycle-bin`, `--log`,
  `--help`, `--version`, plus meaningful exit codes for scripting.
- **New cleaning categories**: thumbnail & icon cache, crash dumps / WER
  reports / `MEMORY.DMP`, Recycle Bin (all drives) and an opt-in Windows
  Update download cache — on top of user temp, Windows temp and Prefetch.
- Timestamped log file support (`TempCleaner.log` next to the `.exe` by
  default in silent mode).
- Application icon, version info, DPI-aware and long-path-aware manifest.
- GitHub Actions workflow: tests on every push, release binary on `v*` tags.
- Unit tests for the cleaning engine, CLI parsing and formatting helpers.

### Safety

- Files modified within the last 24 hours are kept by default
  (`--min-age 0` restores delete-everything behaviour).
- Locked files are skipped; read-only files get one retry after the read-only
  attribute is cleared.
- Symlinks and junctions are never followed.
- Protected system locations and drive roots are refused by a hard safety
  gate; a target folder itself is never removed, only its contents.

---

## [0.1.0]

Initial prototype.

### Added

- Cleaned `%TEMP%`, `C:\Windows\Temp` and Prefetch, then showed a 2-second
  toast with the totals.

---

[Unreleased]: https://github.com/Sajedur0/Temp-Cleaner/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/Sajedur0/Temp-Cleaner/releases/tag/v1.0.0
[0.1.0]: https://github.com/Sajedur0/Temp-Cleaner/commits/a572f77dbf0cf05b1c456af943c44ab637a40a37
