//! Command-line parsing. Pure `std` code so it can be unit-tested anywhere.
//!
//! Design notes:
//! * Running with **no arguments** starts the GUI flow (scan → confirm → toast).
//! * Running with **any argument** switches to console mode. In console mode
//!   nothing is deleted unless `--yes` (or `--silent`, which implies `--yes`)
//!   is given — otherwise the run behaves like a dry run. This makes the CLI
//!   safe to explore.

use std::path::PathBuf;
use std::time::Duration;

/// A group of locations that are cleaned together.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Category {
    UserTemp,
    SystemTemp,
    Prefetch,
    Thumbnails,
    CrashDumps,
    RecycleBin,
    /// Temporary Internet Files (the legacy IE / Edge cache that Windows
    /// Disk Cleanup clears).
    InetCache,
    /// DirectX / D3D shader cache.
    ShaderCache,
    /// Windows service caches: Delivery Optimization, font cache, BranchCache.
    ServiceCache,
    /// `C:\Windows.old` and leftover setup folders (opt-in: irreversible).
    PreviousWindows,
    WindowsUpdate,
}

impl Category {
    pub fn label(&self) -> &'static str {
        match self {
            Category::UserTemp => "User temp files",
            Category::SystemTemp => "Windows temp files",
            Category::Prefetch => "Prefetch files",
            Category::Thumbnails => "Thumbnail & icon cache",
            Category::CrashDumps => "Crash dumps & error reports",
            Category::RecycleBin => "Recycle Bin",
            Category::InetCache => "Temporary Internet Files",
            Category::ShaderCache => "DirectX shader cache",
            Category::ServiceCache => "Windows service caches",
            Category::PreviousWindows => "Previous Windows installation",
            Category::WindowsUpdate => "Windows Update cache",
        }
    }

    /// Parse a category name (several friendly aliases are accepted).
    pub fn parse(raw: &str) -> Option<Category> {
        match raw.trim().to_ascii_lowercase().replace('_', "-").as_str() {
            "temp" | "user-temp" | "usertemp" => Some(Category::UserTemp),
            "system-temp" | "systemtemp" | "windows-temp" => Some(Category::SystemTemp),
            "prefetch" => Some(Category::Prefetch),
            "thumbnails" | "thumbnail" | "icons" | "icon-cache" | "thumbcache" => {
                Some(Category::Thumbnails)
            }
            "crash-dumps" | "crashdumps" | "dumps" | "wer" => Some(Category::CrashDumps),
            "recycle-bin" | "recyclebin" | "recycle" | "bin" => Some(Category::RecycleBin),
            "windows-update" | "windowsupdate" | "update" | "wu" => {
                Some(Category::WindowsUpdate)
            }
            "inet-cache" | "inetcache" | "internet" | "internet-cache" | "iecache" | "cache" => {
                Some(Category::InetCache)
            }
            "shader-cache" | "shadercache" | "shaders" | "d3dscache" | "dxcache" => {
                Some(Category::ShaderCache)
            }
            "service-cache" | "servicecache" | "services" | "delivery" | "delivery-optimization"
            | "font-cache" | "fonts" | "branch-cache" | "branchcache" => {
                Some(Category::ServiceCache)
            }
            "previous-windows" | "previouswindows" | "windows-old" | "windowsold"
            | "old-windows" | "windows.old" => Some(Category::PreviousWindows),
            _ => None,
        }
    }

}

/// Default: files touched within the last day are kept. Temp files that are
/// still fresh often belong to installers / running apps.
pub const DEFAULT_MIN_AGE_HOURS: u64 = 24;

#[derive(Clone, Debug)]
pub struct Config {
    /// Scan only, never delete.
    pub dry_run: bool,
    /// Do not ask for confirmation (console mode requires this to delete).
    pub yes: bool,
    /// No GUI at all; implies `yes`. For Task Scheduler / scripts.
    pub silent: bool,
    /// Only clean these categories (None = default set).
    pub only: Option<Vec<Category>>,
    /// Categories to exclude.
    pub skip: Vec<Category>,
    /// Also clean the Windows Update download cache (off by default).
    pub include_windows_update: bool,
    /// Also delete `C:\Windows.old` and leftover setup folders (off by
    /// default: it removes the ability to roll back to the old Windows).
    pub include_previous_windows: bool,
    /// Empty the Recycle Bin as part of the run.
    pub empty_recycle_bin: bool,
    /// Only delete files whose last modification is at least this old.
    pub min_age_hours: u64,
    /// True when the user passed `--min-age` explicitly (rather than relying
    /// on the default). Some categories opt out of the *default* rule only.
    pub min_age_explicit: bool,
    /// Queue files that are locked by Windows for deletion at the next boot.
    pub reboot_delete: bool,
    /// Append a timestamped log to this file.
    pub log_path: Option<PathBuf>,
    /// Extra console output (every skipped file, every folder).
    pub verbose: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            dry_run: false,
            yes: false,
            silent: false,
            only: None,
            skip: Vec::new(),
            include_windows_update: false,
            include_previous_windows: false,
            empty_recycle_bin: true,
            min_age_hours: DEFAULT_MIN_AGE_HOURS,
            min_age_explicit: false,
            reboot_delete: false,
            log_path: None,
            verbose: false,
        }
    }
}

impl Config {
    pub fn min_age(&self) -> Duration {
        // min_age_hours is validated during parsing, so this cannot overflow.
        Duration::from_secs(self.min_age_hours.saturating_mul(3600))
    }

    /// Minimum age that applies to one category.
    ///
    /// Prefetch records are a pure cache: Windows rewrites `.pf` files
    /// constantly (every launch, plus during automatic maintenance), so the
    /// default "keep the last 24 hours" rule would very often leave the whole
    /// folder behind. Windows regenerates anything deleted, so the category
    /// ignores the *default* rule. An explicit `--min-age` is always honoured,
    /// and `--skip prefetch` keeps it untouched.
    pub fn min_age_for(&self, cat: Category) -> Duration {
        if cat == Category::Prefetch && !self.min_age_explicit {
            Duration::ZERO
        } else {
            self.min_age()
        }
    }

    /// Is this category part of the current run?
    pub fn category_enabled(&self, cat: Category) -> bool {
        // Destructive / risky categories are opt-in, even if `--only`
        // mentions others.
        let opt_in = match cat {
            Category::WindowsUpdate => !self.include_windows_update,
            Category::PreviousWindows => !self.include_previous_windows,
            _ => false,
        };
        if opt_in {
            // ...unless the user explicitly asked for *only* this category.
            let explicitly_only = matches!(&self.only, Some(list) if list.contains(&cat));
            if !explicitly_only {
                return false;
            }
        }
        if let Some(only) = &self.only {
            if !only.contains(&cat) {
                return false;
            }
        }
        !self.skip.contains(&cat)
    }
}

pub enum ParseOutcome {
    Run(Config),
    Help,
    Version,
    Error(String),
}

fn take_value(
    args: &[String],
    i: &mut usize,
    inline: Option<String>,
    flag: &str,
) -> Result<String, String> {
    if let Some(v) = inline {
        if v.is_empty() {
            return Err(format!("Option {flag} needs a value, e.g. {flag}=24"));
        }
        return Ok(v);
    }
    *i += 1;
    match args.get(*i) {
        Some(v) if !v.starts_with("--") => Ok(v.clone()),
        _ => Err(format!("Option {flag} needs a value, e.g. {flag} 24")),
    }
}

fn parse_category_list(value: &str, flag: &str) -> Result<Vec<Category>, String> {
    let mut out = Vec::new();
    for part in value.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        match Category::parse(part) {
            Some(c) => {
                if !out.contains(&c) {
                    out.push(c);
                }
            }
            None => {
                return Err(format!(
                    "Unknown category '{part}' in {flag}. Valid categories: \
                     user-temp, system-temp, prefetch, thumbnails, crash-dumps, \
                     inet-cache, shader-cache, service-cache, recycle-bin, \
                     windows-update, previous-windows"
                ));
            }
        }
    }
    if out.is_empty() {
        return Err(format!("{flag} needs at least one category"));
    }
    Ok(out)
}

pub fn parse_args(args: &[String]) -> ParseOutcome {
    let mut cfg = Config::default();
    let mut only: Vec<Category> = Vec::new();
    let mut i = 0usize;

    while i < args.len() {
        let raw = &args[i];
        // Support --flag=value as well as --flag value.
        let (flag, inline) = match raw.split_once('=') {
            Some((f, v)) if f.starts_with("--") => (f, Some(v.to_string())),
            _ => (raw.as_str(), None),
        };

        match flag {
            "--help" | "-h" | "/?" | "help" => return ParseOutcome::Help,
            "--version" | "-V" | "version" => return ParseOutcome::Version,
            "--dry-run" => cfg.dry_run = true,
            "--yes" | "-y" => cfg.yes = true,
            "--silent" | "-s" => cfg.silent = true,
            "--verbose" | "-v" => cfg.verbose = true,
            "--windows-update" => cfg.include_windows_update = true,
            "--previous-windows" => cfg.include_previous_windows = true,
            // One shot: everything, including the opt-in categories.
            "--all" => {
                cfg.include_windows_update = true;
                cfg.include_previous_windows = true;
            }
            "--no-recycle-bin" => cfg.empty_recycle_bin = false,
            "--min-age" => {
                let v = match take_value(args, &mut i, inline, "--min-age") {
                    Ok(v) => v,
                    Err(e) => return ParseOutcome::Error(e),
                };
                match v.parse::<u64>() {
                    Ok(h) if h <= 24 * 365 * 10 => {
                        cfg.min_age_hours = h;
                        cfg.min_age_explicit = true;
                    }
                    _ => {
                        return ParseOutcome::Error(format!(
                            "Invalid --min-age value '{v}' (expected hours, 0 = delete everything)"
                        ))
                    }
                }
            }
            "--reboot-delete" | "--delete-on-reboot" => cfg.reboot_delete = true,
            "--only" => {
                let v = match take_value(args, &mut i, inline, "--only") {
                    Ok(v) => v,
                    Err(e) => return ParseOutcome::Error(e),
                };
                match parse_category_list(&v, "--only") {
                    Ok(mut list) => only.append(&mut list),
                    Err(e) => return ParseOutcome::Error(e),
                }
            }
            "--skip" => {
                let v = match take_value(args, &mut i, inline, "--skip") {
                    Ok(v) => v,
                    Err(e) => return ParseOutcome::Error(e),
                };
                match parse_category_list(&v, "--skip") {
                    Ok(mut list) => cfg.skip.append(&mut list),
                    Err(e) => return ParseOutcome::Error(e),
                }
            }
            "--log" => {
                let v = match take_value(args, &mut i, inline, "--log") {
                    Ok(v) => v,
                    Err(e) => return ParseOutcome::Error(e),
                };
                cfg.log_path = Some(PathBuf::from(v));
            }
            other => {
                return ParseOutcome::Error(format!(
                    "Unknown option: {other}\nRun with --help to see usage."
                ));
            }
        }
        i += 1;
    }

    if !only.is_empty() {
        cfg.only = Some(only);
    }
    // Recycle Bin participates in --only/--skip like every other category.
    if !cfg.category_enabled(Category::RecycleBin) {
        cfg.empty_recycle_bin = false;
    }
    ParseOutcome::Run(cfg)
}

pub const HELP: &str = "\
Temp Cleaner — free up disk space by deleting temporary files (Windows)

USAGE:
    TempCleaner.exe                  GUI mode: scan, ask, clean, show a toast
    TempCleaner.exe [OPTIONS]        Console mode (see below)

OPTIONS:
    -h, --help               Show this help
    -V, --version            Show version
        --dry-run            Scan only; report what *would* be freed
    -y, --yes                Actually delete (console mode deletes nothing
                             without --yes or --silent)
    -s, --silent             No GUI at all, implies --yes. For scheduled tasks.
                             Writes TempCleaner.log next to the .exe by default.
    -v, --verbose            Detailed output (every skipped file / folder)
        --min-age <HOURS>    Only delete files not modified for HOURS
                             (default: 24; 0 = delete everything)
        --only <LIST>        Only clean these categories (comma separated)
        --skip <LIST>        Exclude these categories (comma separated)
        --windows-update     Also clean the Windows Update download cache
        --previous-windows   Also delete C:\\Windows.old and leftover setup
                             folders ($WINDOWS.~BT, $WINDOWS.~WS). Irreversible
                             (you can no longer roll back to the old Windows).
        --all                Enable every category, including the two opt-in
                             ones above. One-shot \"clean everything\" run.
        --no-recycle-bin     Do not empty the Recycle Bin
        --reboot-delete      Files Windows keeps open (Prefetch records, locked
                             caches) are queued for deletion at the next boot
                             instead of being skipped. Needs Administrator.
        --log <PATH>         Append a timestamped log to PATH

CATEGORIES:
    user-temp, system-temp, prefetch, thumbnails, crash-dumps,
    inet-cache, shader-cache, service-cache, recycle-bin,
    windows-update (opt-in), previous-windows (opt-in)

    Prefetch is a cache Windows rewrites constantly, so the default 24h
    \"keep recent files\" rule does not apply to it. Pass --min-age to
    override, or --skip prefetch to leave it alone.

EXAMPLES:
    TempCleaner.exe --dry-run
    TempCleaner.exe --yes --min-age 0
    TempCleaner.exe --silent --skip prefetch --log C:\\Logs\\clean.log
    TempCleaner.exe --yes --only user-temp,recycle-bin
    TempCleaner.exe --yes --only prefetch --reboot-delete

EXIT CODES:
    0 = success (or dry run)   1 = cleaned, but some files were skipped
    2 = usage error            3 = nothing could be scanned

SAFETY:
    Files in use, files newer than --min-age, symlinks / junctions and
    protected system locations are never followed or deleted. The root
    folder of every target (e.g. C:\\Windows\\Temp itself) is always kept.
";

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(v: &[&str]) -> ParseOutcome {
        parse_args(&v.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn empty_args_give_defaults() {
        match parse(&[]) {
            ParseOutcome::Run(c) => {
                assert_eq!(c.min_age_hours, DEFAULT_MIN_AGE_HOURS);
                assert!(c.empty_recycle_bin);
                assert!(!c.silent && !c.yes && !c.dry_run);
            }
            _ => panic!("expected Run"),
        }
    }

    #[test]
    fn flags_and_values_parse() {
        match parse(&["--yes", "--min-age", "0", "--skip", "prefetch,thumbnails"]) {
            ParseOutcome::Run(c) => {
                assert!(c.yes);
                assert_eq!(c.min_age_hours, 0);
                assert!(!c.category_enabled(Category::Prefetch));
                assert!(!c.category_enabled(Category::Thumbnails));
                assert!(c.category_enabled(Category::UserTemp));
            }
            _ => panic!("expected Run"),
        }
    }

    #[test]
    fn equals_form_and_only() {
        match parse(&["--min-age=48", "--only=user-temp,recycle"]) {
            ParseOutcome::Run(c) => {
                assert_eq!(c.min_age_hours, 48);
                assert!(c.category_enabled(Category::UserTemp));
                assert!(!c.category_enabled(Category::Prefetch));
                assert!(c.empty_recycle_bin);
            }
            _ => panic!("expected Run"),
        }
    }

    #[test]
    fn windows_update_is_opt_in() {
        match parse(&["--yes"]) {
            ParseOutcome::Run(c) => assert!(!c.category_enabled(Category::WindowsUpdate)),
            _ => panic!("expected Run"),
        }
        match parse(&["--yes", "--only", "windows-update"]) {
            ParseOutcome::Run(c) => assert!(c.category_enabled(Category::WindowsUpdate)),
            _ => panic!("expected Run"),
        }
    }

    #[test]
    fn previous_windows_is_opt_in() {
        match parse(&["--yes"]) {
            ParseOutcome::Run(c) => assert!(!c.category_enabled(Category::PreviousWindows)),
            _ => panic!("expected Run"),
        }
        match parse(&["--yes", "--only", "windows-old"]) {
            ParseOutcome::Run(c) => assert!(c.category_enabled(Category::PreviousWindows)),
            _ => panic!("expected Run"),
        }
    }

    #[test]
    fn all_enables_the_opt_in_categories() {
        match parse(&["--all"]) {
            ParseOutcome::Run(c) => {
                assert!(c.category_enabled(Category::WindowsUpdate));
                assert!(c.category_enabled(Category::PreviousWindows));
                assert!(c.category_enabled(Category::Prefetch));
                assert!(c.category_enabled(Category::ServiceCache));
            }
            _ => panic!("expected Run"),
        }
    }

    #[test]
    fn new_cache_categories_parse() {
        for name in [
            "inet-cache",
            "shader-cache",
            "d3dscache",
            "service-cache",
            "delivery-optimization",
            "font-cache",
            "branch-cache",
            "previous-windows",
            "windows.old",
        ] {
            assert!(Category::parse(name).is_some(), "{name} should parse");
        }
    }

    #[test]
    fn bad_input_is_an_error() {
        assert!(matches!(parse(&["--nope"]), ParseOutcome::Error(_)));
        assert!(matches!(parse(&["--min-age"]), ParseOutcome::Error(_)));
        assert!(matches!(parse(&["--min-age", "abc"]), ParseOutcome::Error(_)));
        assert!(matches!(parse(&["--only", "bogus"]), ParseOutcome::Error(_)));
    }
}
