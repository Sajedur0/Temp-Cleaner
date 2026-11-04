//! Builds the list of locations to clean from the environment.
//! Everything is derived from well-known Windows environment variables, so
//! the app works on any Windows install regardless of drive letter or
//! user-name. Locations that fail the safety gate are dropped later by the
//! cleaner engine itself (defence in depth).

use std::collections::HashSet;
use std::env;
use std::path::PathBuf;

use crate::cleaner::{CleanMode, Target};
use crate::cli::{Category, Config};

fn env_path(var: &str) -> Option<PathBuf> {
    env::var_os(var)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

fn windows_dir() -> PathBuf {
    env_path("WINDIR")
        .or_else(|| env_path("SystemRoot"))
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows"))
}

fn local_app_data() -> Option<PathBuf> {
    env_path("LOCALAPPDATA").or_else(|| {
        env_path("USERPROFILE").map(|p| p.join("AppData").join("Local"))
    })
}

fn program_data() -> PathBuf {
    env_path("ProgramData").unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"))
}

fn user_temp() -> Option<PathBuf> {
    env_path("TEMP")
        .or_else(|| env_path("TMP"))
        .or_else(|| local_app_data().map(|p| p.join("Temp")))
}

/// All candidate targets for the enabled categories. Missing folders are
/// kept in the list; the engine reports them as non-existent and moves on.
pub fn build_targets(cfg: &Config) -> Vec<Target> {
    let mut targets: Vec<Target> = Vec::new();
    let windir = windows_dir();
    let local = local_app_data();
    let progdata = program_data();

    if cfg.category_enabled(Category::UserTemp) {
        if let Some(t) = user_temp() {
            targets.push(Target::new(
                "User temp (%TEMP%)",
                Category::UserTemp,
                t,
                CleanMode::AllContents,
            ));
        }
    }

    if cfg.category_enabled(Category::SystemTemp) {
        targets.push(Target::new(
            "Windows temp",
            Category::SystemTemp,
            windir.join("Temp"),
            CleanMode::AllContents,
        ));
    }

    if cfg.category_enabled(Category::Prefetch) {
        targets.push(Target::new(
            "Prefetch",
            Category::Prefetch,
            windir.join("Prefetch"),
            CleanMode::AllContents,
        ));
    }

    if cfg.category_enabled(Category::Thumbnails) {
        if let Some(local) = &local {
            targets.push(Target::new(
                "Thumbnail & icon cache",
                Category::Thumbnails,
                local.join("Microsoft").join("Windows").join("Explorer"),
                CleanMode::Matching {
                    prefixes: vec!["thumbcache_".to_string(), "iconcache_".to_string()],
                    suffix: ".db".to_string(),
                },
            ));
            targets.push(Target::new(
                "Legacy icon cache",
                Category::Thumbnails,
                local.join("IconCache.db"),
                CleanMode::SingleFile,
            ));
        }
    }

    if cfg.category_enabled(Category::CrashDumps) {
        if let Some(local) = &local {
            targets.push(Target::new(
                "Per-app crash dumps",
                Category::CrashDumps,
                local.join("CrashDumps"),
                CleanMode::AllContents,
            ));
        }
        targets.push(Target::new(
            "Minidumps",
            Category::CrashDumps,
            windir.join("Minidump"),
            CleanMode::AllContents,
        ));
        targets.push(Target::new(
            "Full memory dump",
            Category::CrashDumps,
            windir.join("MEMORY.DMP"),
            CleanMode::SingleFile,
        ));
        let wer = progdata.join("Microsoft").join("Windows").join("WER");
        for sub in ["ReportArchive", "ReportQueue", "Temp"] {
            targets.push(Target::new(
                &format!("Error reports ({sub})"),
                Category::CrashDumps,
                wer.join(sub),
                CleanMode::AllContents,
            ));
        }
    }

    if cfg.category_enabled(Category::WindowsUpdate) {
        targets.push(Target::new(
            "Windows Update downloads",
            Category::WindowsUpdate,
            windir.join("SoftwareDistribution").join("Download"),
            CleanMode::AllContents,
        ));
    }

    dedupe(targets)
}

/// `%TEMP%` and `%TMP%` usually point at the same folder — drop exact
/// duplicates (case-insensitive, as Windows paths are).
fn dedupe(targets: Vec<Target>) -> Vec<Target> {
    let mut seen: HashSet<String> = HashSet::new();
    targets
        .into_iter()
        .filter(|t| {
            seen.insert(
                t.path
                    .to_string_lossy()
                    .trim_end_matches(['\\', '/'])
                    .to_lowercase(),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedupe_drops_case_insensitive_duplicates() {
        let mk = |p: &str| {
            Target::new("t", Category::UserTemp, PathBuf::from(p), CleanMode::AllContents)
        };
        let out = dedupe(vec![
            mk(r"C:\Users\a\AppData\Local\Temp"),
            mk(r"c:\users\a\appdata\local\temp\"),
            mk(r"C:\Windows\Temp"),
        ]);
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn category_filtering_drives_targets() {
        let cfg = Config {
            only: Some(vec![Category::Prefetch]),
            ..Config::default()
        };
        let targets = build_targets(&cfg);
        assert!(!targets.is_empty());
        assert!(targets.iter().all(|t| t.category == Category::Prefetch));
    }
}
