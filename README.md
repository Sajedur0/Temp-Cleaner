# Temp Cleaner

A fast, safe **temporary-file cleaner for Windows**, written in Rust.
Double-click it and it scans, shows exactly what will be deleted, cleans on
confirmation and reports the result in a toast — or drive it entirely from
the command line / Task Scheduler.

> **বাংলা:** ডাবল-ক্লিক করলেই স্ক্যান করে, কী মুছবে তা দেখিয়ে অনুমতি নিয়ে
> ক্লিন করে দেয়। চাইলে কমান্ড লাইন থেকেও চালানো যায়
> (`TempCleaner.exe --dry-run`, `--yes`, `--silent` ইত্যাদি)।

## Features

- **Scan before deleting** — a confirmation dialog lists every category,
  file count and size before anything is touched.
- **Safe by default**
  - Files modified in the last **24 hours are kept** (running apps and
    installers often still need them). Configure with `--min-age`, or use
    `--min-age 0` to delete everything.
  - Locked / in-use files are skipped, never forced.
  - Symlinks and junctions are removed as links only — never followed.
  - Protected locations (`C:\Windows` itself, drive roots, user-profile
    roots, …) are refused by a built-in safety gate. A target folder itself
    (e.g. `C:\Windows\Temp`) is never deleted, only its contents.
- **Toast result summary** (bottom-right, auto-dismiss, click to close):
  files deleted, space freed, files skipped.
- **Full CLI** for scripting, with dry-run mode, category filters, logging
  and meaningful exit codes.
- **Tiny native binary**, no runtime dependencies; asks for Administrator
  once via UAC (needed for `C:\Windows\Temp` and `Prefetch`).

## What it cleans

| Category          | Locations                                                                 |
| ----------------- | ------------------------------------------------------------------------- |
| User temp         | `%TEMP%` / `%TMP%`                                                        |
| Windows temp      | `%WINDIR%\Temp`                                                           |
| Prefetch          | `%WINDIR%\Prefetch`                                                       |
| Thumbnail & icons | `%LOCALAPPDATA%\Microsoft\Windows\Explorer\thumbcache_*.db`, `iconcache_*.db`, `%LOCALAPPDATA%\IconCache.db` |
| Crash dumps       | `%LOCALAPPDATA%\CrashDumps`, `%WINDIR%\Minidump`, `%WINDIR%\MEMORY.DMP`, Windows Error Reporting archives (`%ProgramData%\Microsoft\Windows\WER\…`) |
| Recycle Bin       | All drives (via the Shell API)                                            |
| Windows Update    | `%WINDIR%\SoftwareDistribution\Download` — **opt-in** (`--windows-update`) |

Browser caches and personal files are intentionally **not** touched.

## Usage

### GUI (no arguments)

```
TempCleaner.exe
```

Scans → shows a breakdown and asks **Yes/No** (default is *No*) → cleans →
toast summary. If there is nothing to clean, the toast says so.

### Command line

Console mode activates as soon as any argument is given. **Nothing is
deleted without `--yes` or `--silent`** — otherwise the run only reports
what *would* be freed.

```
Options:
    -h, --help               Show help
    -V, --version            Show version
        --dry-run            Scan only; report what would be freed
    -y, --yes                Actually delete (no prompt)
    -s, --silent             No GUI at all, implies --yes (scheduled tasks);
                             logs to TempCleaner.log next to the .exe
    -v, --verbose            Detailed output (every skipped file/folder)
        --min-age <HOURS>    Only delete files not modified for HOURS
                             (default: 24; 0 = delete everything)
        --only <LIST>        Only clean these categories (comma separated)
        --skip <LIST>        Exclude these categories
        --windows-update     Also clean the Windows Update download cache
        --no-recycle-bin     Do not empty the Recycle Bin
        --log <PATH>         Append a timestamped log to PATH
```

Categories: `user-temp`, `system-temp`, `prefetch`, `thumbnails`,
`crash-dumps`, `recycle-bin`, `windows-update`.

### Examples

```powershell
TempCleaner.exe --dry-run                        # see what would be freed
TempCleaner.exe --yes                            # clean everything (24h rule)
TempCleaner.exe --yes --min-age 0                # clean everything, any age
TempCleaner.exe --yes --only user-temp,recycle-bin
TempCleaner.exe --silent --skip prefetch --log C:\Logs\clean.log
```

### Exit codes

| Code | Meaning                                        |
| ---- | ---------------------------------------------- |
| 0    | Success, or a dry run / nothing to do          |
| 1    | Cleaned, but some files were skipped           |
| 2    | Usage error (unknown option, bad value)        |
| 3    | Nothing could be scanned                       |

### Run weekly with Task Scheduler

```cmd
schtasks /Create /TN "Temp Cleaner" /SC WEEKLY /D SUN /ST 03:00 ^
  /TR "\"C:\Path\To\TempCleaner.exe\" --silent" /RL HIGHEST
```

## Building from source

Requires a stable Rust toolchain on Windows:

```powershell
cargo build --release      # produces target\release\TempCleaner.exe
cargo test                 # unit tests (engine, CLI parsing, formatting)
cargo clippy -- -D warnings
```

The release build embeds the icon, version info and an Administrator
manifest (`build.rs`), and produces a small optimised binary
(`opt-level = "z"`, LTO, stripped).

Releases are built automatically by GitHub Actions
(`.github/workflows/build.yml`): every push runs the tests, and pushing a
`v*` tag publishes `TempCleaner.exe` as a GitHub Release.

## Project layout

```
src/
  main.rs      entry point: GUI flow vs console flow, exit codes
  cli.rs       argument & category parsing (unit-tested)
  targets.rs   builds the location list from environment variables
  cleaner.rs   scan/delete engine + safety gate (unit-tested)
  platform.rs  Windows toast, dialogs, console attach, Recycle Bin
               (stubbed on other OSes so tests run anywhere)
  report.rs    size/number/date formatting + log file writer
build.rs       UAC manifest, icon, version info
assets/icon.ico
```

## Notes & FAQ

- **Thumbnail cache files are skipped?** Explorer keeps `thumbcache_*.db`
  open while it runs; locked files are skipped by design. Sign out/in (or
  restart Explorer) and run again to clear them fully.
- **Prefetch cleaning** is included because that was this tool's original
  behaviour. Windows rebuilds prefetch data automatically; use
  `--skip prefetch` if you prefer to keep it.
- **Windows Update cache** is opt-in because Windows Update may be
  mid-download. Only clean it when no update is in progress.

## License

Apache License 2.0 — see [LICENSE](LICENSE).
