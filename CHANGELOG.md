# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Legend: **Added** · **Changed** · **Fixed** · **Safety** · **Removed**

---

## [Unreleased]

### Fixed

- **Recycle Bin kept reporting a size no matter how often it was emptied.**
  The size was measured by walking `<drive>:\$Recycle.Bin`, which also counts
  other accounts' bins and orphaned SID folders the Shell can never empty —
  and it included removable drives and empty card readers that have no bin at
  all. It now asks the Shell itself (`SHQueryRecycleBinW`) for the size of
  each *fixed* drive, empties every drive explicitly in addition to the
  all-drives call, then re-measures and reports what was genuinely freed.
- **The summary claimed Recycle Bin space that was never freed.** Emptying
  was reported as success on nothing but the API's return value; the result
  is now verified (`before` vs `after`) and a bin that is still full is
  reported as such (and yields exit code `1`).
- **Prefetch (and other protected folders) could not be cleaned.** The
  process never enabled the backup/restore rights its elevated token already
  holds, so any file whose ACL refused the delete failed instantly. Those
  privileges are now enabled at startup, blocking attributes are cleared, and
  a sharing violation is retried a few times instead of giving up at once.

### Added

- **More Disk Cleanup locations**, all cleaned in one pass:
  - `inet-cache` — Temporary Internet Files (`INetCache`, the legacy
    IE/Edge cache Disk Cleanup clears);
  - `shader-cache` — DirectX shader cache (`%LOCALAPPDATA%\D3DSCache`);
  - `service-cache` — Windows service caches: Delivery Optimization,
    font cache and BranchCache;
  - `previous-windows` — `C:\Windows.old`, `$WINDOWS.~BT`, `$WINDOWS.~WS`
    (**opt-in**, irreversible: you lose the rollback to the old Windows).
- **`--all`** — enable every category, including the two opt-in ones, for a
  one-shot "clean everything" run.
- **Guaranteed Administrator rights.** The binary still embeds a
  `requireAdministrator` manifest, and it now also enables the backup,
  restore and take-ownership privileges of the elevated token. On top of
  that, if a double-clicked run somehow starts un-elevated (manifest not
  applied, UAC off, standard account), the app restarts itself through a UAC
  prompt instead of silently skipping half the job. Console and `--silent`
  runs are never relaunched — a scheduled task must not wait for a prompt.
- The safety gate now allows the per-service cache folders under
  `%WINDIR%\ServiceProfiles` (font cache, Delivery Optimization, BranchCache)
  while the rest of the Windows tree stays off-limits.
- **`--reboot-delete`** — files Windows holds open (Prefetch records, locked
  caches) are queued for deletion at the next boot through
  `MoveFileEx(…, MOVEFILE_DELAY_UNTIL_REBOOT)` instead of being skipped. A
  file opened without delete-sharing cannot be deleted by anything while the
  system is running, so this is the only way to actually remove them. Needs
  Administrator rights; the space is freed on restart and those files are
  counted separately (never reported as "freed" before they are gone).
- **Prefetch is exempt from the default `--min-age` rule.** Windows rewrites
  `.pf` records on every app launch and during automatic maintenance, so the
  default "keep the last 24 hours" rule could leave the whole folder behind.
  An explicit `--min-age` is still honoured, and `--skip prefetch` is
  unchanged.
- A hint in the console summary (and the toast) pointing at
  `--reboot-delete` when files are skipped as "in use".
- Skipped files are now counted **by reason** — too recent, in use, or access
  denied — and shown in the console summary and the result toast.
- Folders that cannot even be listed are reported ("could not be read —
  access denied") instead of silently looking empty.
- A warning in the confirmation dialog and the console when Temp Cleaner is
  not running elevated, since that is the usual reason system locations are
  reported as access denied.
- Log lines now carry the real OS reason for every file that could not be
  deleted instead of a generic "in use?".
- The result toast grew a little to fit the extra detail.

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
