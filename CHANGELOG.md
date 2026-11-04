# Changelog

## 1.0.0 — 2026-11-04

First complete release.

### Added
- Fixed project layout (`src/main.rs`) so `cargo build` works out of the box.
- Scan → confirm → clean GUI flow: a dialog shows the per-category
  breakdown (files + size) before anything is deleted; the result toast
  reports files deleted, space freed and files skipped.
- Full CLI: `--dry-run`, `--yes`, `--silent`, `--verbose`, `--min-age`,
  `--only`, `--skip`, `--windows-update`, `--no-recycle-bin`, `--log`,
  `--help`, `--version`, plus exit codes for scripting.
- New cleaning categories: thumbnail & icon cache, crash dumps / WER
  reports / `MEMORY.DMP`, Recycle Bin (all drives), and opt-in Windows
  Update download cache — on top of user temp, Windows temp and Prefetch.
- Timestamped log file support (`TempCleaner.log` by default in silent mode).
- Application icon, version info, DPI-aware + long-path-aware manifest.
- GitHub Actions workflow: tests on every push, release binary on `v*` tags.
- Unit tests for the cleaning engine, CLI parsing and formatting helpers.

### Safety
- Files modified within the last 24 hours are kept by default
  (`--min-age 0` restores delete-everything behaviour).
- Locked files are skipped; read-only files get one retry after clearing
  the attribute.
- Symlinks/junctions are never followed; protected system locations and
  drive roots are refused; a target folder itself is never removed.

## 0.1.0

- Initial prototype: cleaned `%TEMP%`, `C:\Windows\Temp` and Prefetch,
  then showed a 2-second toast with the totals.
