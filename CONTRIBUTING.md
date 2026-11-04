<div align="center">

# 🤝 Contributing to Temp Cleaner

Thanks for taking the time to help! This is a small Windows utility written in
Rust, and contributions of every size are welcome — from a typo fix to a new
cleaning category.

</div>

---

## Before You Start

- **Bugs & feature requests:** open an
  [Issue](https://github.com/Sajedur0/Temp-Cleaner/issues) first so we can
  agree on the approach before you write code.
- **Security issues:** do **not** open a public issue — follow
  [SECURITY.md](SECURITY.md) instead.
- **Anything that deletes files** is reviewed extra carefully. When in doubt,
  ask in an issue first.

## Getting Set Up

You need a stable Rust toolchain on Windows (the app uses the Windows API):

```powershell
git clone https://github.com/Sajedur0/Temp-Cleaner.git
cd Temp-Cleaner
cargo build          # debug build
cargo test           # unit tests (engine, CLI parsing, formatting)
```

Tests must pass on any OS — `src/platform.rs` is stubbed on non-Windows so CI
and local Linux/macOS runs work. Keep new logic pure and unit-tested wherever
possible, and keep Windows-specific calls behind `platform.rs`.

## The Golden Rules

1. **Never delete anything the user did not ask to delete.**
   New targets must be system/app cache or temp locations only — no personal
   files, no browser profiles, no downloads or documents.
2. **Keep the safety gate intact.** Protected paths, drive roots, symlinks and
   junctions stay off-limits. See `src/cleaner.rs`.
3. **The target folder itself is never removed**, only its contents.
4. **Fail safe.** If something is locked, ambiguous or unexpected, skip it and
   report it — do not force it.
5. **Anything new is opt-out-able.** New cleaning categories should respect
   `--only` / `--skip`, and anything risky should be opt-in like
   `--windows-update`.

## Making a Change

1. Fork the repo and create a branch: `git switch -c feat/my-change`.
2. Write the code **and** the tests.
3. Run the checks:

   ```powershell
   cargo fmt --all
   cargo clippy -- -D warnings
   cargo test
   ```

4. Update the docs when behaviour changes:
   - `README.md` — features, options table, category table, FAQ
   - `CHANGELOG.md` — add a line under **Unreleased**
5. Commit with a [conventional](https://www.conventionalcommits.org/) style
   message, e.g. `feat: add Edge cache category` or `fix: skip locked WER files`.
6. Open a pull request describing **what** changed, **why**, and **how you
   tested it** (Windows version, whether you ran elevated, before/after disk
   usage).

## Adding a Cleaning Category

A new category touches several files — please update all of them:

| File | Change |
| --- | --- |
| `src/cli.rs` | New `Category` variant, a `label()`, and friendly aliases in `parse()`; update `HELP` |
| `src/targets.rs` | Build the target path(s) from environment variables (never hard-code `C:\` or a user name) |
| `src/cleaner.rs` | Pick the right `CleanMode` (`AllContents` / `Matching` / `SingleFile`) |
| `src/main.rs` | Wire it into the GUI/console flow if it needs special handling |
| `README.md`, `CHANGELOG.md` | Document the category and its CLI name |

## Code Style

- Plain `std` + `winapi`; no extra runtime dependencies without discussion.
- Keep functions small and documented with `///` comments explaining *why*.
- Match the surrounding style — `cargo fmt` handles the rest.

## Code of Conduct

Be kind and constructive. Harassment, personal attacks and bad-faith
behaviour are not welcome anywhere in this project.

---

📄 See the [README](README.md) for usage and the
[Changelog](CHANGELOG.md) for release history.
