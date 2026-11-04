# 🧹 Temp Cleaner

[![Version](https://img.shields.io/badge/version-1.0.0-blue)](https://github.com/Sajedur0/Temp-Cleaner/releases)
[![Platform](https://img.shields.io/badge/platform-Windows-0078D6?logo=windows&logoColor=white)](https://www.microsoft.com/windows)
[![Language](https://img.shields.io/badge/language-Rust-DEA584?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-Apache--2.0-green)](LICENSE)
[![Build](https://github.com/Sajedur0/Temp-Cleaner/actions/workflows/build.yml/badge.svg)](https://github.com/Sajedur0/Temp-Cleaner/actions/workflows/build.yml)

A fast, safe **temporary-file cleaner for Windows**, written in Rust.
Double-click it and it scans, shows exactly what will be deleted, cleans on
confirmation and reports the result in a toast — or drive it entirely from
the command line / Task Scheduler.

> **Quick note:** Double-click to scan, review what will be removed, and
> clean with a single confirmation — or use the command line instead
> (`TempCleaner.exe --dry-run`, `--yes`, `--silent`, and more).
> For safety, files modified within the last 24 hours are kept by default.

---

## 📑 Table of Contents

- [Features](#-features)
- [What It Cleans](#-what-it-cleans)
- [Download & Install](#-download--install)
- [Requirements](#-requirements)
- [Usage](#-usage)
- [Examples](#-examples)
- [Safety First](#-safety-first)
- [Scheduled Cleaning](#-scheduled-cleaning)
- [Logging](#-logging)
- [Exit Codes](#-exit-codes)
- [FAQ / Troubleshooting](#-faq--troubleshooting)
- [Building from Source](#-building-from-source)
- [Project Layout](#-project-layout)
- [Contributing](#-contributing)
- [License](#-license)

---

## ✨ Features

- **Scan before deleting** — a confirmation dialog lists every category,
  file count and size before anything is touched.
- **Safe by default** — files modified in the last **24 hours are kept**
  (running apps and installers often still need them). Configure with
  `--min-age`, or use `--min-age 0` to delete everything.
- **Locked files are skipped, never forced**; read-only files get one retry
  after clearing the read-only attribute.
- **Symlinks and junctions are never followed** — the link itself is
  removed, its target is untouched.
- **Protected locations are refused** by a built-in safety gate
  (`C:\Windows` itself, `C:\Windows\System32`, drive roots, user-profile
  roots, `Program Files`, …). A target folder itself
  (e.g. `C:\Windows\Temp`) is never deleted — only its contents.
- **Toast result summary** (bottom-right, auto-dismiss, click to close):
  files deleted, space freed, files skipped.
- **Full CLI** for scripting: dry-run mode, category filters, logging and
  meaningful exit codes.
- **Tiny native binary**, no runtime dependencies; asks for Administrator
  once via UAC (needed for `C:\Windows\Temp` and `Prefetch`).

## 🧹 What It Cleans

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

## ⬇️ Download & Install

### Option 1 — Prebuilt binary (recommended)

1. Download `TempCleaner.exe` from the
   [Releases page](https://github.com/Sajedur0/Temp-Cleaner/releases).
2. Put it anywhere you like (e.g. `C:\Tools\` or your Desktop).
3. Double-click it and approve the **Administrator (UAC)** prompt.

> Windows SmartScreen may warn about an unsigned `.exe` from the internet.
> Click **More info → Run anyway** if you trust the source — or build from
> source yourself (Option 2).

No installation is needed; the app is a single self-contained `.exe`.

### Option 2 — Build from source

See [Building from Source](#-building-from-source) below.

## 📋 Requirements

|                     | Requirement                                             |
| ------------------- | ------------------------------------------------------- |
| Operating system    | Windows 10 or Windows 11                                |
| Privileges          | Administrator (UAC prompt on launch)                    |
| Runtime dependencies| None — single native `.exe`                             |
| To build from source| A recent stable [Rust toolchain](https://rustup.rs/)    |

## 🚀 Usage

### GUI mode (no arguments)

```text
TempCleaner.exe
```

**Scan →** shows a per-category breakdown and asks **Yes/No**
(the default button is *No*) **→ Clean →** toast summary.
If there is nothing to clean, the toast says so.

### Console mode (any argument)

Console mode activates as soon as any argument is given. **Nothing is
deleted without `--yes` or `--silent`** — otherwise the run only reports
what *would* be freed.

| Option                 | Description                                                                 |
| ---------------------- | --------------------------------------------------------------------------- |
| `-h`, `--help`         | Show help                                                                   |
| `-V`, `--version`      | Show version                                                                |
| `--dry-run`            | Scan only; report what would be freed                                       |
| `-y`, `--yes`          | Actually delete (no prompt)                                                 |
| `-s`, `--silent`       | No GUI at all, implies `--yes` (for scheduled tasks); logs to `TempCleaner.log` next to the `.exe` by default |
| `-v`, `--verbose`      | Detailed output (every skipped file/folder)                                 |
| `--min-age <HOURS>`    | Only delete files not modified for `HOURS` (default: `24`; `0` = everything) |
| `--only <LIST>`        | Only clean these categories (comma separated)                               |
| `--skip <LIST>`        | Exclude these categories (comma separated)                                  |
| `--windows-update`     | Also clean the Windows Update download cache                                |
| `--no-recycle-bin`     | Do not empty the Recycle Bin                                                |
| `--log <PATH>`         | Append a timestamped log to `PATH`                                          |

**Categories:** `user-temp`, `system-temp`, `prefetch`, `thumbnails`,
`crash-dumps`, `recycle-bin`, `windows-update` (opt-in).

## 💡 Examples

```powershell
TempCleaner.exe --dry-run                        # see what would be freed
TempCleaner.exe --yes                            # clean everything (24h rule)
TempCleaner.exe --yes --min-age 0                # clean everything, any age
TempCleaner.exe --yes --only user-temp,recycle-bin
TempCleaner.exe --yes --skip prefetch,thumbnails
TempCleaner.exe --silent --skip prefetch --log C:\Logs\clean.log
```

## 🔒 Safety First

Temp Cleaner is deliberately conservative for a deletion tool:

1. **It always scans and asks first** in GUI mode; console mode deletes
   nothing without an explicit `--yes` / `--silent`.
2. **Recent files survive** — anything modified within the last 24 hours
   (default `--min-age`) is kept, so running apps and in-progress
   installers are not disturbed.
3. **In-use files are skipped**, never force-deleted.
4. **Links are never followed** — symlinks/junctions are removed as links
   only.
5. **Hard safety gate** — drive roots, `C:\Windows` itself, `System32`,
   user-profile roots and similar protected paths are refused even if an
   environment variable (e.g. a misconfigured `%TEMP%`) points at them.
6. **The target folder itself is never removed**, only its contents.

Even so, as with any cleaner: when in doubt, run `--dry-run` first and
read the scan results.

## 🕒 Scheduled Cleaning

Run it weekly, silently, with Task Scheduler:

```cmd
schtasks /Create /TN "Temp Cleaner" /SC WEEKLY /D SUN /ST 03:00 ^
  /TR "\"C:\Path\To\TempCleaner.exe\" --silent" /RL HIGHEST
```

## 📝 Logging

- `--log <PATH>` appends timestamped lines (scan results, deletions,
  skipped files, summary) to the given file.
- In `--silent` mode, if no `--log` is given, the log defaults to
  `TempCleaner.log` next to `TempCleaner.exe`.
- Combine with `-v` / `--verbose` for per-file detail on the console.

## 🔢 Exit Codes

| Code | Meaning                                   |
| ---- | ----------------------------------------- |
| `0`  | Success, or a dry run / nothing to do     |
| `1`  | Cleaned, but some files were skipped      |
| `2`  | Usage error (unknown option, bad value)   |
| `3`  | Nothing could be scanned                  |

## ❓ FAQ / Troubleshooting

- **Thumbnail cache files are skipped?** Explorer keeps `thumbcache_*.db`
  open while it runs; locked files are skipped by design. Sign out/in (or
  restart Explorer) and run again to clear them fully.
- **"Nothing to clean" right after installing something?** Files from the
  last 24 hours are kept by default. Use `--min-age 0` if you really want
  everything gone.
- **Prefetch cleaning** — Windows rebuilds prefetch data automatically,
  though apps may launch slightly slower once afterwards. Use
  `--skip prefetch` if you prefer to keep it.
- **Windows Update cache** is opt-in (`--windows-update`) because Windows
  Update may be mid-download. Only clean it when no update is in progress.
- **No Administrator prompt appeared / access denied?** The `.exe` must
  run elevated to clean `C:\Windows\Temp` and `Prefetch`; approve the UAC
  prompt, or right-click → *Run as administrator*.
- **Does it delete personal files or browser data?** No. Only the system
  and app temp/cache locations listed above are touched.

## 🛠️ Building from Source

Requires a stable Rust toolchain on Windows:

```powershell
git clone https://github.com/Sajedur0/Temp-Cleaner.git
cd Temp-Cleaner
cargo build --release      # produces target\release\TempCleaner.exe
cargo test                 # unit tests (engine, CLI parsing, formatting)
cargo clippy -- -D warnings
```

The release build embeds the icon, version info and an Administrator
manifest (`build.rs`), and produces a small optimised binary
(`opt-level = "z"`, LTO, stripped).

Releases are built automatically by GitHub Actions
(`.github/workflows/build.yml`): pushes and pull requests run the tests,
and pushing a `v*` tag publishes `TempCleaner.exe` as a GitHub Release.

## 📂 Project Layout

```text
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

## 🤝 Contributing

Contributions are welcome!

1. Fork the repository and create a feature branch.
2. Make your change, keeping the safety rules above intact.
3. Run `cargo test` and `cargo clippy -- -D warnings`.
4. Open a pull request describing what you changed and why.

Please report bugs and request features via
[GitHub Issues](https://github.com/Sajedur0/Temp-Cleaner/issues).

## 📜 License

Copyright 2026 Sajedur0

Licensed under the **Apache License, Version 2.0**. You may obtain a copy of
the License at <http://www.apache.org/licenses/LICENSE-2.0> — see the
[LICENSE](LICENSE) file for the full text.

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an **"AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND**, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.

See [CHANGELOG.md](CHANGELOG.md) for release history.
