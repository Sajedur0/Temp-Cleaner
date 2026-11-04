<div align="center">

# 🔐 Security Policy

**Temp Cleaner** is a small, dependency-light Windows utility written in Rust.
This page explains which versions receive fixes and how to report a problem.

</div>

---

## Supported Versions

Only the latest released version of Temp Cleaner receives security fixes.

| Version | Status | Notes |
| --- | --- | --- |
| `1.0.x` | ✅ Supported | Current release — fixes land here first |
| `< 1.0` | ❌ Not supported | Prototype only; please upgrade to `1.0.0` or newer |

There are no backport branches: fixes are released as a new patch version
(`1.0.1`, `1.1.0`, …) as soon as they are ready.

## Reporting a Vulnerability

**Please do not open a public issue for security problems.**

Report privately using either of these channels:

1. **GitHub private vulnerability reporting** (preferred) —
   *Security → Report a vulnerability* on
   [the repository](https://github.com/Sajedur0/Temp-Cleaner/security/advisories),
   or
2. **A GitHub Security Advisory draft** opened through the same page.

### What to include

| Detail | Why it helps |
| --- | --- |
| Affected version / commit | Reproducing against the right build |
| Steps to reproduce | A minimal, concrete sequence |
| Expected vs actual behaviour | Makes the impact clear |
| Sample paths or environment variables | Deletion logic depends on `%TEMP%`, `%WINDIR%`, `%LOCALAPPDATA%` |
| Suggested fix (optional) | Always welcome |

### What to expect

| Stage | Target |
| --- | --- |
| Acknowledgement | Within **7 days** |
| Initial assessment / triage | Within **14 days** |
| Fix or mitigation plan | As soon as practical, depending on severity |
| Public advisory | After a fixed release is published |

If the report is **accepted**, we will credit you in the release notes and
security advisory unless you ask to stay anonymous. If it is **declined**
(e.g. it depends on a misconfiguration outside the tool's control), we will
explain why and, where possible, suggest a mitigation.

## Security Model

Temp Cleaner is built to be auditable and boring on purpose:

- **No network code.** The binary never connects to the internet, so there is
  no update channel or telemetry to attack.
- **Deletion is the highest-risk operation**, so it is guarded by a hard
  safety gate in `src/cleaner.rs`: drive roots, `C:\Windows` itself,
  `System32`, `C:\Users`, `C:\ProgramData`, `Program Files`,
  `System Volume Information` and `$Recycle.Bin` are always refused, even if
  an environment variable such as `%TEMP%` is misconfigured to point at them.
- **Links are never followed.** Symlinks and junctions are removed as links
  only; their targets are untouched.
- **Target folders are never deleted**, only their contents.
- **Locked files are skipped**, never force-deleted.
- **Elevation is requested once** via a UAC manifest (needed for
  `C:\Windows\Temp` and `Prefetch`), not silently re-launched.

A bug that causes **deleting something the user did not ask to delete** is
treated as a security issue, not a cosmetic one — please report it.

## Out of Scope

- Issues that require the attacker to already run code as the target user.
- Warnings from antivirus engines or SmartScreen about the unsigned binary
  (a known consequence of distributing a free `.exe`; verify the checksum of
  the file you downloaded from
  [Releases](https://github.com/Sajedur0/Temp-Cleaner/releases)).
- Theoretical issues with no demonstrated impact on a supported Windows
  version.

## Verifying Your Download

Prefer binaries from the
[Releases page](https://github.com/Sajedur0/Temp-Cleaner/releases) or, better
still, [build from source](README.md#-building-from-source) and compare the
result.

```powershell
Get-FileHash .\TempCleaner.exe -Algorithm SHA256
```

---

📄 See the [README](README.md) for usage, and the
[Changelog](CHANGELOG.md) for release history.
