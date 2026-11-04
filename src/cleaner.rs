//! The scanning / deleting engine. Pure `std` code: all policy (which files
//! are old enough, which folders are safe to touch) lives here so it can be
//! unit-tested on any OS with real files in a sandbox directory.

use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::cli::Category;

/// How a target location should be treated.
#[derive(Clone, Debug)]
pub enum CleanMode {
    /// Delete everything inside the folder (but never the folder itself).
    AllContents,
    /// Only delete files whose name matches (used for the Explorer
    /// thumbnail/icon cache folder, which contains unrelated files).
    Matching {
        prefixes: Vec<String>,
        suffix: String,
    },
    /// The target path *is* a single file (e.g. `C:\Windows\MEMORY.DMP`).
    SingleFile,
}

#[derive(Clone, Debug)]
pub struct Target {
    pub label: String,
    pub category: Category,
    pub path: PathBuf,
    pub mode: CleanMode,
}

impl Target {
    pub fn new(
        label: &str,
        category: Category,
        path: PathBuf,
        mode: CleanMode,
    ) -> Target {
        Target {
            label: label.to_string(),
            category,
            path,
            mode,
        }
    }
}

/// Why a file was kept instead of deleted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkipKind {
    /// Newer than `--min-age`: kept on purpose, not a failure.
    Recent,
    /// Another process holds the file open.
    InUse,
    /// Permissions refused the delete (usually: not running elevated).
    Denied,
    /// Anything else.
    Other,
}

// Windows error codes worth naming; `raw_os_error()` returns errno elsewhere,
// where these numbers simply never match and the `ErrorKind` fallback below
// takes over.
const ERROR_ACCESS_DENIED: i32 = 5;
const ERROR_WRITE_PROTECT: i32 = 19;
const ERROR_SHARING_VIOLATION: i32 = 32;
const ERROR_LOCK_VIOLATION: i32 = 33;

/// Classify a failed delete so the summary can say *why* files were kept.
pub fn classify_error(err: &std::io::Error) -> SkipKind {
    match err.raw_os_error() {
        Some(ERROR_SHARING_VIOLATION) | Some(ERROR_LOCK_VIOLATION) => SkipKind::InUse,
        Some(ERROR_ACCESS_DENIED) | Some(ERROR_WRITE_PROTECT) => SkipKind::Denied,
        _ => match err.kind() {
            std::io::ErrorKind::PermissionDenied => SkipKind::Denied,
            _ => SkipKind::Other,
        },
    }
}

/// A short, human reason for a skipped file (logs and the final summary).
pub fn describe_error(err: &std::io::Error) -> String {
    match classify_error(err) {
        SkipKind::InUse => "in use by another process".to_string(),
        SkipKind::Denied => "access denied (not running as Administrator?)".to_string(),
        SkipKind::Recent | SkipKind::Other => err.to_string(),
    }
}

fn is_transient(err: &std::io::Error) -> bool {
    classify_error(err) == SkipKind::InUse
}

fn note_skip(report: &mut Report, kind: SkipKind) {
    match kind {
        SkipKind::Recent => report.skipped_recent += 1,
        SkipKind::InUse => report.skipped_in_use += 1,
        SkipKind::Denied => report.skipped_denied += 1,
        SkipKind::Other => report.skipped_other += 1,
    }
}

/// Result of scanning (and optionally cleaning) one target.
#[derive(Clone, Debug, Default)]
pub struct Report {
    pub label: String,
    pub category: Option<Category>,
    /// False when the folder does not exist or failed the safety check.
    pub exists: bool,
    pub files_found: u64,
    pub bytes_found: u64,
    pub files_deleted: u64,
    pub bytes_freed: u64,
    /// Files that matched but were kept (too recent or locked / in use).
    pub files_skipped: u64,
    /// `files_skipped` broken down by reason.
    pub skipped_recent: u64,
    pub skipped_in_use: u64,
    pub skipped_denied: u64,
    pub skipped_other: u64,
    /// Folders whose contents could not even be listed (access denied).
    pub dirs_unreadable: u64,
    pub dirs_removed: u64,
}

#[derive(Clone, Debug, Default)]
pub struct Totals {
    pub files_found: u64,
    pub bytes_found: u64,
    pub files_deleted: u64,
    pub bytes_freed: u64,
    pub files_skipped: u64,
    pub skipped_recent: u64,
    pub skipped_in_use: u64,
    pub skipped_denied: u64,
    pub skipped_other: u64,
    pub dirs_unreadable: u64,
    pub dirs_removed: u64,
}

impl Totals {
    pub fn add(&mut self, r: &Report) {
        self.files_found += r.files_found;
        self.bytes_found += r.bytes_found;
        self.files_deleted += r.files_deleted;
        self.bytes_freed += r.bytes_freed;
        self.files_skipped += r.files_skipped;
        self.skipped_recent += r.skipped_recent;
        self.skipped_in_use += r.skipped_in_use;
        self.skipped_denied += r.skipped_denied;
        self.skipped_other += r.skipped_other;
        self.dirs_unreadable += r.dirs_unreadable;
        self.dirs_removed += r.dirs_removed;
    }
}

/// Safety gate: refuse anything that is not clearly a disposable location.
///
/// * must be absolute with at least one real directory component
///   (so a drive root like `C:\` is rejected);
/// * a *single*-component path may not be a protected system folder
///   (`C:\Windows`, `C:\Users`, `C:\ProgramData`, …). Deeper paths such as
///   `C:\Windows\Temp` are fine, and `D:\Temp` is fine.
pub fn is_safe_target(path: &Path) -> bool {
    if !path.is_absolute() {
        return false;
    }
    let normals: Vec<String> = path
        .components()
        .filter_map(|c| match c {
            Component::Normal(s) => Some(s.to_string_lossy().to_lowercase()),
            _ => None,
        })
        .collect();
    if normals.is_empty() {
        return false; // drive root / UNC root
    }
    const DENIED_SINGLE: &[&str] = &[
        "windows",
        "users",
        "program files",
        "program files (x86)",
        "programdata",
        "system volume information",
        "$recycle.bin",
        "recovery",
        "documents and settings",
        "perflogs",
    ];
    if normals.len() == 1 && DENIED_SINGLE.contains(&normals[0].as_str()) {
        return false;
    }
    // Nothing inside these roots is ever a disposable location.
    const DENIED_FIRST: &[&str] = &[
        "program files",
        "program files (x86)",
        "system volume information",
        "$recycle.bin",
        "recovery",
        "documents and settings",
    ];
    if DENIED_FIRST.contains(&normals[0].as_str()) {
        return false;
    }
    match normals[0].as_str() {
        // Inside the Windows folder only the known disposable locations
        // may be touched (so a stray %TEMP% pointing at System32 is refused).
        "windows" => {
            const ALLOWED: &[&str] = &[
                "temp",
                "prefetch",
                "minidump",
                "softwaredistribution",
                "memory.dmp",
            ];
            normals.len() >= 2 && ALLOWED.contains(&normals[1].as_str())
        }
        // A user profile root (C:\Users\<name>) or anything shallower is
        // off-limits; real targets live deep under AppData.
        "users" => normals.len() >= 4,
        // %ProgramData% itself and its first level are off-limits.
        "programdata" => normals.len() >= 3,
        _ => true,
    }
}

/// True when the (possibly junction-resolved) location is still safe.
/// Falls back to `true` when the path cannot be canonicalised, because
/// `is_safe_target` has already been applied to the unresolved path and a
/// non-existent path is skipped later anyway.
fn canonical_is_safe(path: &Path) -> bool {
    match fs::canonicalize(path) {
        Ok(real) => is_safe_target(&real),
        Err(_) => true,
    }
}

/// A file may only be deleted when it has not been modified for `min_age`.
/// Files with a modification time in the future are kept (clock skew /
/// freshly written by an installer that set a future timestamp).
pub fn is_old_enough(modified: Option<SystemTime>, min_age: Duration, now: SystemTime) -> bool {
    if min_age.is_zero() {
        return true;
    }
    match modified {
        Some(m) => match now.duration_since(m) {
            Ok(elapsed) => elapsed >= min_age,
            Err(_) => false,
        },
        None => true, // timestamp unreadable: size is known, allow cleanup
    }
}

fn name_matches(file_name: &str, mode: &CleanMode) -> bool {
    match mode {
        CleanMode::Matching { prefixes, suffix } => {
            let name = file_name.to_lowercase();
            name.ends_with(&suffix.to_lowercase())
                && prefixes
                    .iter()
                    .any(|p| name.starts_with(&p.to_lowercase()))
        }
        _ => true,
    }
}

/// Drop the read-only attribute (Windows: via the file attributes API,
/// elsewhere via `std`) so a delete can succeed.
fn clear_readonly(path: &Path) -> bool {
    if crate::platform::clear_readonly_attribute(path) {
        return true;
    }
    if let Ok(meta) = fs::metadata(path) {
        let mut perm = meta.permissions();
        if perm.readonly() {
            #[allow(clippy::permissions_set_readonly_false)]
            perm.set_readonly(false);
            return fs::set_permissions(path, perm).is_ok();
        }
    }
    false
}

/// Try hard to delete one file: normal delete, then clear the read-only
/// attribute and try again, then retry briefly while the file is merely held
/// open. Returns the last error when every attempt failed.
pub fn remove_file_force(path: &Path) -> std::io::Result<()> {
    let first = match fs::remove_file(path) {
        Ok(()) => return Ok(()),
        Err(e) => e,
    };

    // A read-only file can never be deleted; drop the attribute and retry.
    if clear_readonly(path) && fs::remove_file(path).is_ok() {
        return Ok(());
    }

    // Windows rewrites some of these files continuously (prefetch records,
    // the search index, virus scanners…), so a sharing violation is often
    // gone a moment later. Retry briefly instead of giving up instantly —
    // this is what makes stubborn system folders such as Prefetch clear.
    let mut last = first;
    if !is_transient(&last) {
        return Err(last);
    }
    for _ in 0..3 {
        std::thread::sleep(Duration::from_millis(120));
        match fs::remove_file(path) {
            Ok(()) => return Ok(()),
            Err(e) => {
                last = e;
                if !is_transient(&last) {
                    break;
                }
            }
        }
    }
    Err(last)
}

/// Delete a symlink / junction *link itself* without ever following it.
fn remove_link(path: &Path, meta: &fs::Metadata) -> std::io::Result<()> {
    // A link pointing at a directory needs remove_dir on Windows.
    if meta.file_type().is_symlink() {
        return fs::remove_file(path).or_else(|e| match fs::remove_dir(path) {
            Ok(()) => Ok(()),
            Err(_) => Err(e),
        });
    }
    remove_file_force(path)
}

struct Walker {
    dirs_preorder: Vec<PathBuf>,
    /// Folders we could not list, with the reason. Previously these were
    /// skipped silently, which made an unreadable folder look like an empty
    /// one.
    unreadable: Vec<(PathBuf, String)>,
}

impl Walker {
    fn new() -> Walker {
        Walker {
            dirs_preorder: Vec::new(),
            unreadable: Vec::new(),
        }
    }

    /// Walk `root`, calling `on_file` for every file / symlink (never
    /// descending into symlinks). Records directories in pre-order so the
    /// caller can remove them in reverse (children before parents).
    fn run<F: FnMut(&Path, &fs::Metadata)>(&mut self, root: &Path, on_file: &mut F) {
        self.dirs_preorder.push(root.to_path_buf());
        let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];
        while let Some(dir) = stack.pop() {
            let entries = match fs::read_dir(&dir) {
                Ok(e) => e,
                Err(e) => {
                    self.unreadable.push((dir.clone(), describe_error(&e)));
                    continue;
                }
            };
            for entry in entries.flatten() {
                let path = entry.path();
                let meta = match fs::symlink_metadata(&path) {
                    Ok(m) => m,
                    Err(_) => continue,
                };
                let ft = meta.file_type();
                if ft.is_dir() && !ft.is_symlink() {
                    self.dirs_preorder.push(path.clone());
                    stack.push(path);
                } else {
                    on_file(&path, &meta);
                }
            }
        }
    }
}

fn base_report(target: &Target) -> Report {
    Report {
        label: target.label.clone(),
        category: Some(target.category),
        ..Report::default()
    }
}

/// Count what could be deleted (read-only operation).
pub fn scan_target(target: &Target, min_age: Duration, now: SystemTime) -> Report {
    let mut report = base_report(target);
    if !is_safe_target(&target.path) || !canonical_is_safe(&target.path) {
        return report;
    }

    match &target.mode {
        CleanMode::SingleFile => {
            if let Ok(meta) = fs::symlink_metadata(&target.path) {
                report.exists = true;
                if !meta.is_dir()
                    && is_old_enough(meta.modified().ok(), min_age, now)
                {
                    report.files_found = 1;
                    report.bytes_found = meta.len();
                }
            }
        }
        _ => {
            if !target.path.is_dir() {
                return report;
            }
            report.exists = true;
            let mut walker = Walker::new();
            walker.run(&target.path, &mut |path, meta| {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                if name_matches(&name, &target.mode)
                    && is_old_enough(meta.modified().ok(), min_age, now)
                {
                    report.files_found += 1;
                    report.bytes_found += meta.len();
                }
            });
            report.dirs_unreadable = walker.unreadable.len() as u64;
        }
    }
    report
}

/// Delete everything `scan_target` would count. `log` receives one line per
/// file that could not be removed (locked, access denied, …).
pub fn clean_target(
    target: &Target,
    min_age: Duration,
    now: SystemTime,
    log: &mut dyn FnMut(&str),
) -> Report {
    let mut report = base_report(target);
    if !is_safe_target(&target.path) || !canonical_is_safe(&target.path) {
        log(&format!("Refused unsafe target: {}", target.path.display()));
        return report;
    }

    match &target.mode {
        CleanMode::SingleFile => {
            if let Ok(meta) = fs::symlink_metadata(&target.path) {
                report.exists = true;
                if !meta.is_dir() {
                    if is_old_enough(meta.modified().ok(), min_age, now) {
                        report.files_found = 1;
                        report.bytes_found = meta.len();
                        match remove_link(&target.path, &meta) {
                            Ok(()) => {
                                report.files_deleted = 1;
                                report.bytes_freed = meta.len();
                            }
                            Err(e) => {
                                report.files_skipped = 1;
                                note_skip(&mut report, classify_error(&e));
                                log(&format!(
                                    "Could not delete {} ({})",
                                    target.path.display(),
                                    describe_error(&e)
                                ));
                            }
                        }
                    } else {
                        report.files_skipped = 1;
                        note_skip(&mut report, SkipKind::Recent);
                    }
                }
            }
        }
        CleanMode::AllContents | CleanMode::Matching { .. } => {
            if !target.path.is_dir() {
                return report;
            }
            report.exists = true;
            let remove_empty_dirs = matches!(target.mode, CleanMode::AllContents);
            let mut walker = Walker::new();
            let mode = target.mode.clone();
            walker.run(&target.path, &mut |path, meta| {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                if !name_matches(&name, &mode) {
                    return;
                }
                if !is_old_enough(meta.modified().ok(), min_age, now) {
                    report.files_skipped += 1;
                    note_skip(&mut report, SkipKind::Recent);
                    return;
                }
                report.files_found += 1;
                report.bytes_found += meta.len();
                match remove_link(path, meta) {
                    Ok(()) => {
                        report.files_deleted += 1;
                        report.bytes_freed += meta.len();
                    }
                    Err(e) => {
                        report.files_skipped += 1;
                        note_skip(&mut report, classify_error(&e));
                        log(&format!(
                            "Could not delete {} ({})",
                            path.display(),
                            describe_error(&e)
                        ));
                    }
                }
            });

            for (dir, why) in &walker.unreadable {
                report.dirs_unreadable += 1;
                log(&format!("Could not read folder {} ({})", dir.display(), why));
            }

            if remove_empty_dirs {
                // Reverse pre-order: children before parents. The target
                // folder itself (first entry) is always kept.
                for dir in walker.dirs_preorder.iter().rev() {
                    if *dir == target.path {
                        continue;
                    }
                    if fs::remove_dir(dir).is_ok() {
                        report.dirs_removed += 1;
                    }
                }
            }
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn sandbox(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "temp_cleaner_test_{}_{}",
            name,
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_file(path: &Path, contents: &[u8]) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut f = fs::File::create(path).unwrap();
        f.write_all(contents).unwrap();
    }

    fn target_for(path: PathBuf) -> Target {
        Target::new("test", Category::UserTemp, path, CleanMode::AllContents)
    }

    #[test]
    fn scan_and_clean_real_files() {
        let root = sandbox("clean");
        write_file(&root.join("a.tmp"), b"12345");
        write_file(&root.join("sub").join("b.tmp"), b"1234567");
        let target = target_for(root.clone());

        let scan = scan_target(&target, Duration::ZERO, SystemTime::now());
        assert!(scan.exists);
        assert_eq!(scan.files_found, 2);
        assert_eq!(scan.bytes_found, 12);

        let mut lines = Vec::new();
        let report = clean_target(&target, Duration::ZERO, SystemTime::now(), &mut |l| {
            lines.push(l.to_string())
        });
        assert_eq!(report.files_deleted, 2);
        assert_eq!(report.bytes_freed, 12);
        assert_eq!(report.dirs_removed, 1); // "sub", but never the root itself
        assert!(root.exists());
        assert!(!root.join("a.tmp").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn recent_files_are_kept_when_min_age_set() {
        let root = sandbox("age");
        write_file(&root.join("new.tmp"), b"data");
        let target = target_for(root.clone());

        let report = clean_target(
            &target,
            Duration::from_secs(365 * 24 * 3600), // a year: everything is "recent"
            SystemTime::now(),
            &mut |_| {},
        );
        assert_eq!(report.files_deleted, 0);
        assert_eq!(report.files_skipped, 1);
        assert!(root.join("new.tmp").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn matching_mode_only_touches_matching_names() {
        let root = sandbox("matching");
        write_file(&root.join("thumbcache_32.db"), b"xx");
        write_file(&root.join("explorer.exe"), b"keep me");
        let target = Target::new(
            "thumbs",
            Category::Thumbnails,
            root.clone(),
            CleanMode::Matching {
                prefixes: vec!["thumbcache_".into(), "iconcache_".into()],
                suffix: ".db".into(),
            },
        );
        let report = clean_target(&target, Duration::ZERO, SystemTime::now(), &mut |_| {});
        assert_eq!(report.files_deleted, 1);
        assert!(root.join("explorer.exe").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn symlinks_are_never_followed() {
        let root = sandbox("links");
        let outside = sandbox("links_outside");
        write_file(&outside.join("precious.txt"), b"do not touch");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, root.join("linkdir")).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(&outside, root.join("linkdir")).unwrap();

        let target = target_for(root.clone());
        let report = clean_target(&target, Duration::ZERO, SystemTime::now(), &mut |_| {});
        // The link itself is removed, the target folder is untouched.
        assert!(outside.join("precious.txt").exists());
        assert_eq!(report.files_deleted, 1);
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&outside);
    }

    #[cfg(windows)]
    #[test]
    fn safety_gate() {
        assert!(!is_safe_target(Path::new(r"C:\")));
        assert!(!is_safe_target(Path::new(r"C:\Windows")));
        assert!(!is_safe_target(Path::new(r"C:\Users")));
        assert!(!is_safe_target(Path::new("relative\\Temp")));
        assert!(is_safe_target(Path::new(r"C:\Windows\Temp")));
        assert!(is_safe_target(Path::new(r"C:\Windows\Prefetch")));
        assert!(is_safe_target(Path::new(r"D:\Temp")));
        assert!(is_safe_target(Path::new(
            r"C:\Users\me\AppData\Local\Temp"
        )));
        // Hardened cases:
        assert!(!is_safe_target(Path::new(r"C:\Windows\System32")));
        assert!(!is_safe_target(Path::new(r"C:\Users\me")));
        assert!(!is_safe_target(Path::new(r"C:\Program Files\App")));
        assert!(!is_safe_target(Path::new(r"C:\$Recycle.Bin\S-1-5-21")));
        assert!(is_safe_target(Path::new(
            r"C:\Windows\SoftwareDistribution\Download"
        )));
        assert!(is_safe_target(Path::new(r"C:\Windows\MEMORY.DMP")));
        assert!(is_safe_target(Path::new(
            r"C:\ProgramData\Microsoft\Windows\WER\ReportArchive"
        )));
    }
}
