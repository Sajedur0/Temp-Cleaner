<div align="center">

# 🧹 Temp Cleaner

**A fast, safe temporary-file cleaner for Windows — written in Rust.**

[![Release](https://img.shields.io/github/v/release/Sajedur0/Temp-Cleaner?label=release&color=2f6feb)](https://github.com/Sajedur0/Temp-Cleaner/releases/latest)
[![Downloads](https://img.shields.io/github/downloads/Sajedur0/Temp-Cleaner/total?color=2f6feb)](https://github.com/Sajedur0/Temp-Cleaner/releases)
[![Platform](https://img.shields.io/badge/platform-Windows%2010%20%7C%2011-0078D6?logo=windows&logoColor=white)](https://www.microsoft.com/windows)
[![Built with Rust](https://img.shields.io/badge/built%20with-Rust-DEA584?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-Apache--2.0-green)](LICENSE)
[![Build](https://github.com/Sajedur0/Temp-Cleaner/actions/workflows/build.yml/badge.svg)](https://github.com/Sajedur0/Temp-Cleaner/actions/workflows/build.yml)

*Double-click → scan → review → confirm → toast summary.*
*Or drive it entirely from the command line / Task Scheduler.*

</div>

---

## ✨ At a glance

| | |
| --- | --- |
| 🎯 **What it does** | Frees disk space by deleting temp files, caches, crash dumps and the Recycle Bin |
| 🖱️ **Everyday use** | Double-click the `.exe`, approve UAC, press **Yes** — done |
| 🤖 **Automation** | Full CLI with `--dry-run`, `--yes`, `--silent`, `--log` and scriptable exit codes |
| 🛡️ **Safety** | Recent files kept, in-use files skipped, symlinks never followed, protected paths refused |
| 📦 **Footprint** | One small native binary, zero runtime dependencies, no installer |

> **New here?** Download → double-click → read the confirmation dialog → press **Yes**.
> Want to see it work without deleting anything first? Run `TempCleaner.exe --dry-run`.

---

## 📑 Table of Contents

- [Features](#-features)
- [What It Cleans](#-what-it-cleans)
- [Quick Start](#-quick-start)
- [Download & Install](#-download--install)
- [Requirements](#-requirements)
- [Usage](#-usage)
- [Options](#-options)
- [Examples](#-examples)
- [Safety First](#-safety-first)
- [Scheduled Cleaning](#-scheduled-cleaning)
- [Logging](#-logging)
- [Exit Codes](#-exit-codes)
- [FAQ / Troubleshooting](#-faq--troubleshooting)
- [How It Works](#-how-it-works)
- [Building from Source](#-building-from-source)
- [Project Layout](#-project-layout)
- [Contributing](#-contributing)
- [License](#-license)

---

## 🚀 Features

- **Scan before deleting** — a confirmation dialog lists every category, file
  count and total size *before* anything is touched. The default button is
  **No**.
- **Safe by default** — files modified in the last **24 hours are kept**,
  because running apps and in-progress installers often still need them.
  Tune it with `--min-age`, or use `--min-age 0` to delete everything.
- **Locked files are skipped, never forced**; read-only files get one retry
  after the read-only attribute is cleared.
- **Symlinks and junctions are never followed** — the link itself is removed,
  its target stays untouched.
- **Protected locations are refused** by a built-in safety gate
  (`C:\Windows` itself, `C:\Windows\System32`, drive roots, `C:\Users`,
  `C:\ProgramData`, `Program Files`, `System Volume Information`, …). Even a
  misconfigured `%TEMP%` pointing somewhere dangerous is rejected.
- **A target folder itself is never deleted** — only its contents
  (`C:\Windows\Temp` survives; what is inside it does not).
- **Toast result summary** (bottom-right, auto-dismiss, click to close):
  files deleted, space freed, files skipped.
- **Full CLI for scripting** — dry runs, category filters, logging and
  meaningful exit codes.
- **Tiny native binary**, no runtime dependencies. Asks for Administrator
  once via UAC (required for `C:\Windows\Temp` and `Prefetch`).

## 🧹 What It Cleans

| Category | Locations | CLI name |
| --- | --- | --- |
| User temp | `%TEMP%` / `%TMP%` | `user-temp` |
| Windows temp | `%WINDIR%\Temp` | `system-temp` |
| Prefetch | `%WINDIR%\Prefetch` | `prefetch` |
| Thumbnail & icon cache | `%LOCALAPPDATA%\Microsoft\Windows\Explorer\thumbcache_*.db` and `iconcache_*.db`, plus `%LOCALAPPDATA%\IconCache.db` | `thumbnails` |
| Crash dumps | `%LOCALAPPDATA%\CrashDumps`, `%WINDIR%\Minidump`, `%WINDIR%\MEMORY.DMP`, Windows Error Reporting archives (`%ProgramData%\Microsoft\Windows\WER\…`) | `crash-dumps` |
| Recycle Bin | All drives (via the Windows Shell API) | `recycle-bin` |
| Windows Update cache | `%WINDIR%\SoftwareDistribution\Download` — **opt-in** (`--windows-update`) | `windows-update` |

Browser caches, downloads, documents and any other personal files are
**intentionally never touched**.

## ⚡ Quick Start

```powershell
# 1. Look, don't touch — see exactly what would be freed
TempCleaner.exe --dry-run

# 2. Happy with the list? Clean it (keeps files younger than 24h)
TempCleaner.exe --yes

# 3. Or just double-click TempCleaner.exe and press Yes in the dialog
```

## ⬇️ Download & Install

### Option 1 — Prebuilt binary (recommended)

1. Download `TempCleaner.exe` from the
   [latest release](https://github.com/Sajedur0/Temp-Cleaner/releases/latest).
2. Put it wherever you like (e.g. `C:\Tools\` or your Desktop).
3. Double-click it and approve the **Administrator (UAC)** prompt.

No installer, no DLLs, no registry entries — it is a single self-contained
`.exe`. Uninstalling is deleting the file.

> **SmartScreen warning?** Windows may warn about an unsigned `.exe`
> downloaded from the internet. Click **More info → Run anyway** if you trust
> the source, or build it yourself from source (Option 2).

### Option 2 — Build from source

See [Building from Source](#-building-from-source).

## 📋 Requirements

| | Requirement |
| --- | --- |
| Operating system | Windows 10 or Windows 11 |
| Privileges | Administrator (UAC prompt on launch) |
| Runtime dependencies | None — one native `.exe` |
| To build from source | A recent stable [Rust toolchain](https://rustup.rs/) |

## 🖥️ Usage

### GUI mode (no arguments)

```text
TempCleaner.exe
```

**Scan →** a dialog shows the per-category breakdown and asks **Yes/No**
(default is *No*) → **Clean →** toast summary.
If there is nothing to clean, the toast says so.

### Console mode (any argument)

Console mode activates as soon as any argument is passed.
**Nothing is deleted without `--yes` or `--silent`** — otherwise the run only
reports what *would* be freed, so the CLI is safe to explore.

## 🔧 Options

| Option | Description |
| --- | --- |
| `-h`, `--help` | Show help |
| `-V`, `--version` | Show version |
| `--dry-run` | Scan only; report what would be freed |
| `-y`, `--yes` | Actually delete (no prompt) |
| `-s`, `--silent` | No GUI at all, implies `--yes` (for scheduled tasks); logs to `TempCleaner.log` next to the `.exe` by default |
| `-v`, `--verbose` | Detailed output (every skipped file/folder) |
| `--min-age <HOURS>` | Only delete files not modified for `HOURS` (default: `24`; `0` = everything) |
| `--only <LIST>` | Only clean these categories (comma separated) |
| `--skip <LIST>` | Exclude these categories (comma separated) |
| `--windows-update` | Also clean the Windows Update download cache |
| `--no-recycle-bin` | Do not empty the Recycle Bin |
| `--log <PATH>` | Append a timestamped log to `PATH` |

Every option also accepts the `--flag=value` form, e.g. `--min-age=48`.

**Categories:** `user-temp`, `system-temp`, `prefetch`, `thumbnails`,
`crash-dumps`, `recycle-bin`, `windows-update` (opt-in).

## 💡 Examples

```powershell
TempCleaner.exe --dry-run                          # see what would be freed
TempCleaner.exe --yes                              # clean everything (24h rule)
TempCleaner.exe --yes --min-age 0                  # clean everything, any age
TempCleaner.exe --yes --only user-temp,recycle-bin # just these two categories
TempCleaner.exe --yes --skip prefetch,thumbnails   # everything except these
TempCleaner.exe --silent --skip prefetch --log C:\Logs\clean.log
```

## 🔒 Safety First

Temp Cleaner is deliberately conservative for a tool that deletes files:

1. **It always scans and asks first** in GUI mode; console mode deletes
   nothing without an explicit `--yes` / `--silent`.
2. **Recent files survive** — anything modified within the last 24 hours
   (the default `--min-age`) is kept, so running apps and in-progress
   installers are not disturbed.
3. **In-use files are skipped**, never force-deleted.
4. **Links are never followed** — symlinks and junctions are removed as links
   only.
5. **Hard safety gate** — drive roots, `C:\Windows` itself, `System32`,
   `C:\Users`, `C:\ProgramData`, `Program Files` and similar protected paths
   are refused, even if an environment variable points at them.
6. **The target folder itself is never removed**, only its contents.

Even so — as with any cleaner — when in doubt run `--dry-run` first and read
the scan results.

## 🕒 Scheduled Cleaning

Run it weekly and silently with Task Scheduler:

```cmd
schtasks /Create /TN "Temp Cleaner" /SC WEEKLY /D SUN /ST 03:00 ^
  /TR "\"C:\Path\To\TempCleaner.exe\" --silent" /RL HIGHEST
```

Add `--log C:\Logs\TempCleaner.log` to keep a history of what was removed.

## 📝 Logging

- `--log <PATH>` appends timestamped lines (scan results, deletions, skipped
  files, summary) to the given file.
- In `--silent` mode, if no `--log` is given, the log defaults to
  `TempCleaner.log` next to `TempCleaner.exe`.
- Add `-v` / `--verbose` for per-file detail on the console.

## 🔢 Exit Codes

| Code | Meaning |
| --- | --- |
| `0` | Success — or a dry run, or nothing to do |
| `1` | Cleaned, but some files were skipped |
| `2` | Usage error (unknown option, bad value) |
| `3` | Nothing could be scanned |

## ❓ FAQ / Troubleshooting

- **Thumbnail cache files are skipped?**
  Explorer keeps `thumbcache_*.db` open while it runs, and locked files are
  skipped by design. Sign out and back in (or restart Explorer) and run again
  to clear them fully.
- **“Nothing to clean” right after installing something?**
  Files from the last 24 hours are kept by default. Use `--min-age 0` if you
  really want everything gone.
- **Prefetch cleaning** — Windows rebuilds prefetch data automatically,
  though apps may launch slightly slower once afterwards. Use
  `--skip prefetch` if you would rather keep it.
- **Windows Update cache is opt-in** (`--windows-update`) because Windows
  Update may be mid-download. Only clean it when no update is in progress.
- **No Administrator prompt, or “access denied”?**
  The `.exe` must run elevated to clean `C:\Windows\Temp` and `Prefetch`.
  Approve the UAC prompt, or right-click → *Run as administrator*.
- **Does it delete personal files or browser data?**
  No. Only the system and app temp/cache locations listed above are touched.
- **Does it phone home or collect telemetry?**
  No. There is no network code in the binary at all.

## ⚙️ How It Works

```text
┌──────────┐   ┌───────────────────────────────┐   ┌──────────┐   ┌─────────┐
│  targets │ → │ scan (min-age + safety gate)  │ → │ confirm  │ → │  clean  │
│  .rs     │   │            cleaner.rs         │   │ (GUI/CLI)│   │ + toast │
└──────────┘   └───────────────────────────────┘   └──────────┘   └─────────┘
   env vars        skip recent / locked / links        Yes or --yes   report.rs
                   refuse protected paths                            logging
```

## 🛠️ Building from Source

Requires a stable Rust toolchain on Windows:

```powershell
git clone https://github.com/Sajedur0/Temp-Cleaner.git
cd Temp-Cleaner
cargo build --release      # → target\release\TempCleaner.exe
cargo test                 # unit tests (engine, CLI parsing, formatting)
cargo clippy -- -D warnings
```

The release build embeds the icon, version info and an Administrator manifest
(via `build.rs`) and produces a small optimised binary: `opt-level = "z"`,
LTO, single codegen unit, stripped, `panic = "abort"`.

Releases are built automatically by GitHub Actions
(`.github/workflows/build.yml`): pushes and pull requests run `cargo test`
and Clippy, and pushing a `v*` tag publishes `TempCleaner.exe` as a GitHub
Release.

## 📂 Project Layout

```text
src/
  main.rs      entry point: GUI flow vs console flow, exit codes
  cli.rs       argument & category parsing (unit-tested)
  targets.rs   builds the location list from environment variables
  cleaner.rs   scan/delete engine + safety gate (unit-tested)
  platform.rs  Windows toast, dialogs, console attach, Recycle Bin
               (stubbed on other OSes so the tests run anywhere)
  report.rs    size/number/date formatting + log file writer
build.rs       UAC manifest, icon, version info
assets/icon.ico
```

## 🤝 Contributing

Contributions are welcome.

1. Fork the repository and create a feature branch.
2. Make your change, keeping the [safety rules](#-safety-first) intact.
3. Run `cargo test` and `cargo clippy -- -D warnings`.
4. Open a pull request describing **what** you changed and **why**.

A short guide lives in [CONTRIBUTING.md](CONTRIBUTING.md) — including how to
add a new cleaning category.

Please report bugs and request features via
[GitHub Issues](https://github.com/Sajedur0/Temp-Cleaner/issues).
See [SECURITY.md](SECURITY.md) for how to report a vulnerability.

## 📜 License

Copyright © 2026 Sajedur0

Licensed under the **Apache License, Version 2.0**. You may obtain a copy of
the License at <http://www.apache.org/licenses/LICENSE-2.0> — see the
[LICENSE](LICENSE) file for the full text.

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an **"AS IS" BASIS, WITHOUT
WARRANTIES OR CONDITIONS OF ANY KIND**, either express or implied. See the
License for the specific language governing permissions and limitations under
the License.

---

<div align="center">

📄 [Changelog](CHANGELOG.md) · 🔐 [Security Policy](SECURITY.md) ·
🐛 [Report a bug](https://github.com/Sajedur0/Temp-Cleaner/issues) ·
⬇️ [Releases](https://github.com/Sajedur0/Temp-Cleaner/releases)

If Temp Cleaner freed some space for you, consider giving the repo a ⭐

</div>
