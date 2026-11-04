// No console window for GUI use; console mode attaches to the parent
// console on demand (see `platform::attach_console`).
#![windows_subsystem = "windows"]

mod cleaner;
mod cli;
mod platform;
mod report;
mod targets;

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::SystemTime;

use cleaner::{clean_target, scan_target, Report, Totals};
use cli::{Category, Config, ParseOutcome, HELP};
use report::{format_number, format_size, Logger};

const EXIT_OK: u8 = 0;
const EXIT_PARTIAL: u8 = 1;
const EXIT_USAGE: u8 = 2;
const EXIT_NOTHING_SCANNED: u8 = 3;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|a| a.to_string_lossy().into_owned())
        .collect();

    match cli::parse_args(&args) {
        ParseOutcome::Help => {
            if platform::attach_console() {
                print!("{}", HELP);
            } else {
                platform::info_box("Temp Cleaner — Help", HELP);
            }
            ExitCode::from(EXIT_OK)
        }
        ParseOutcome::Version => {
            if platform::attach_console() {
                println!("Temp Cleaner {}", env!("CARGO_PKG_VERSION"));
            } else {
                platform::info_box(
                    "Temp Cleaner",
                    &format!("Temp Cleaner version {}", env!("CARGO_PKG_VERSION")),
                );
            }
            ExitCode::from(EXIT_OK)
        }
        ParseOutcome::Error(msg) => {
            if platform::attach_console() {
                eprintln!("Error: {}", msg);
            } else {
                platform::info_box("Temp Cleaner — Error", &msg);
            }
            ExitCode::from(EXIT_USAGE)
        }
        ParseOutcome::Run(cfg) => {
            // No arguments at all = double-clicked: friendly GUI flow.
            // Any argument = console mode.
            let code = if args.is_empty() {
                gui_mode(&cfg)
            } else {
                console_mode(&cfg)
            };
            ExitCode::from(code)
        }
    }
}

// ---------------------------------------------------------------- helpers

/// Resolve where the log file goes: an explicit `--log`, or (in silent
/// mode) `TempCleaner.log` next to the executable.
fn log_path_for(cfg: &Config) -> Option<PathBuf> {
    if let Some(p) = &cfg.log_path {
        return Some(p.clone());
    }
    if cfg.silent {
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                return Some(dir.join("TempCleaner.log"));
            }
        }
    }
    None
}

fn scan_all(cfg: &Config, logger: &mut Logger) -> (Vec<Report>, Totals, u64) {
    let targets = targets::build_targets(cfg);
    let now = SystemTime::now();
    let min_age = cfg.min_age();

    let mut reports = Vec::new();
    let mut totals = Totals::default();
    for t in &targets {
        let r = scan_target(t, min_age, now);
        if r.exists {
            logger.detail(&format!(
                "Found {} ({}) in {}",
                format_number(r.files_found),
                format_size(r.bytes_found),
                t.label
            ));
            totals.add(&r);
        } else {
            logger.detail(&format!("Skipped (not found / unsafe): {}", t.path.display()));
        }
        reports.push(r);
    }

    let recycle_bytes = if cfg.empty_recycle_bin {
        let b = platform::recycle_bin_size();
        if b > 0 {
            logger.detail(&format!("Recycle Bin holds about {}", format_size(b)));
        }
        b
    } else {
        0
    };

    (reports, totals, recycle_bytes)
}

fn clean_all(cfg: &Config, logger: &mut Logger) -> Totals {
    let targets = targets::build_targets(cfg);
    let now = SystemTime::now();
    let min_age = cfg.min_age();

    let mut totals = Totals::default();
    for t in &targets {
        let r = {
            let mut on_log = |line: &str| logger.detail(line);
            clean_target(t, min_age, now, &mut on_log)
        };
        if r.exists {
            logger.detail(&format!(
                "{}: deleted {} files, freed {} ({} skipped)",
                r.label,
                format_number(r.files_deleted),
                format_size(r.bytes_freed),
                format_number(r.files_skipped)
            ));
        }
        totals.add(&r);
    }
    totals
}

/// Group per-target reports into per-category totals (order preserved).
fn by_category(reports: &[Report]) -> Vec<(String, u64, u64)> {
    let mut out: Vec<(Category, u64, u64)> = Vec::new();
    for r in reports {
        if r.files_found == 0 {
            continue;
        }
        if let Some(cat) = r.category {
            if let Some(e) = out.iter_mut().find(|(c, _, _)| *c == cat) {
                e.1 += r.files_found;
                e.2 += r.bytes_found;
            } else {
                out.push((cat, r.files_found, r.bytes_found));
            }
        }
    }
    out.into_iter()
        .map(|(c, f, b)| (c.label().to_string(), f, b))
        .collect()
}

fn age_note(cfg: &Config) -> String {
    if cfg.min_age_hours == 0 {
        "Files of any age will be deleted.".to_string()
    } else {
        format!(
            "Files modified in the last {}h are kept (use --min-age 0 to remove everything).",
            cfg.min_age_hours
        )
    }
}

// -------------------------------------------------------------- GUI mode

fn gui_mode(cfg: &Config) -> u8 {
    let mut logger = Logger::new(log_path_for(cfg).as_deref(), false, false);
    logger.info("Temp Cleaner started (GUI mode)");

    // 1) Scan
    let (reports, totals, recycle_bytes) = scan_all(cfg, &mut logger);
    let scanned_any = reports.iter().any(|r| r.exists);

    if totals.files_found == 0 && recycle_bytes == 0 {
        let msg = if scanned_any {
            "Temp Cleaner\nNothing to clean — your temp folders are already tidy."
        } else {
            "Temp Cleaner\nNo cleanable locations were found."
        };
        logger.info(msg);
        platform::show_toast(msg, false);
        return EXIT_OK;
    }

    // 2) Confirm with a breakdown
    let mut text = String::from("The following junk was found:\n");
    for (label, files, bytes) in by_category(&reports) {
        text.push_str(&format!(
            "\n• {}: {} files — {}",
            label,
            format_number(files),
            format_size(bytes)
        ));
    }
    if recycle_bytes > 0 {
        text.push_str(&format!(
            "\n• Recycle Bin: about {}",
            format_size(recycle_bytes)
        ));
    }
    text.push_str(&format!(
        "\n\nTotal: {} files — {}\n\nDelete these files now?\n{}",
        format_number(totals.files_found),
        format_size(totals.bytes_found + recycle_bytes),
        age_note(cfg)
    ));

    if !platform::confirm_box("Temp Cleaner", &text) {
        logger.info("User cancelled at the confirmation dialog.");
        return EXIT_OK;
    }

    // 3) Clean
    let done = clean_all(cfg, &mut logger);
    let mut freed = done.bytes_freed;
    if cfg.empty_recycle_bin && platform::empty_recycle_bin() {
        freed += recycle_bytes;
        logger.info("Recycle Bin emptied.");
    }

    let mut summary = format!(
        "Temp Cleaner ✓\nDeleted {} files\nFreed {}",
        format_number(done.files_deleted),
        format_size(freed)
    );
    if done.files_skipped > 0 {
        summary.push_str(&format!(
            "\n({} in use or recent — skipped)",
            format_number(done.files_skipped)
        ));
    }
    logger.info(&summary.replace('\n', " | "));
    platform::show_toast(&summary, true);
    EXIT_OK
}

// ---------------------------------------------------------- console mode

fn console_mode(cfg: &Config) -> u8 {
    let has_console = platform::attach_console();
    // Printing without a console (e.g. --silent under Task Scheduler)
    // would panic on the invalid stdout handle, so gate every print.
    macro_rules! outln {
        () => {
            if has_console {
                println!()
            }
        };
        ($($arg:tt)*) => {
            if has_console {
                println!($($arg)*)
            }
        };
    }
    let mut logger = Logger::new(log_path_for(cfg).as_deref(), has_console, cfg.verbose);

    logger.info(&format!(
        "Temp Cleaner {} — console mode",
        env!("CARGO_PKG_VERSION")
    ));
    logger.info(&age_note(cfg));

    // 1) Scan and show the plan
    let (reports, totals, recycle_bytes) = scan_all(cfg, &mut logger);
    let scanned_any = reports.iter().any(|r| r.exists) || cfg.empty_recycle_bin;
    if !scanned_any && recycle_bytes == 0 {
        logger.info("Nothing could be scanned (no target folders exist).");
        return EXIT_NOTHING_SCANNED;
    }

    outln!();
    outln!("Scan results:");
    for (label, files, bytes) in by_category(&reports) {
        outln!(
            "  {:<32} {:>10} files  {:>12}",
            label,
            format_number(files),
            format_size(bytes)
        );
    }
    if cfg.empty_recycle_bin {
        outln!(
            "  {:<32} {:>10}        {:>12}",
            Category::RecycleBin.label(),
            "—",
            if recycle_bytes > 0 {
                format_size(recycle_bytes)
            } else {
                "(empty)".to_string()
            }
        );
    }
    outln!(
        "  {:<32} {:>10} files  {:>12}",
        "TOTAL",
        format_number(totals.files_found),
        format_size(totals.bytes_found + recycle_bytes)
    );

    // 2) Decide whether we may delete
    let may_delete = !cfg.dry_run && (cfg.yes || cfg.silent);
    if !may_delete {
        outln!();
        if cfg.dry_run {
            outln!("Dry run: nothing was deleted.");
        } else {
            outln!("Nothing was deleted. Re-run with --yes to clean,");
            outln!("or use --dry-run to only scan.");
        }
        logger.info("Finished without deleting (dry run / no --yes).");
        return EXIT_OK;
    }

    if totals.files_found == 0 && recycle_bytes == 0 && !cfg.empty_recycle_bin {
        outln!("\nNothing to delete.");
        logger.info("Nothing to delete.");
        return EXIT_OK;
    }

    // 3) Clean
    outln!("\nCleaning...");
    let done = clean_all(cfg, &mut logger);

    let mut freed = done.bytes_freed;
    if cfg.empty_recycle_bin {
        // Always call it: the size estimate can read 0 when $Recycle.Bin
        // is not readable, yet the Shell may still empty it.
        if platform::empty_recycle_bin() {
            freed += recycle_bytes;
            logger.info(&format!(
                "Recycle Bin emptied (about {}).",
                format_size(recycle_bytes)
            ));
        } else {
            logger.info("Could not empty the Recycle Bin.");
        }
    }

    outln!();
    outln!("Done:");
    outln!("  Files deleted : {}", format_number(done.files_deleted));
    outln!("  Space freed   : {}", format_size(freed));
    outln!("  Files skipped : {} (in use or too recent)", format_number(done.files_skipped));
    outln!("  Folders removed: {}", format_number(done.dirs_removed));
    logger.info(&format!(
        "Finished: deleted {} files, freed {}, skipped {}.",
        done.files_deleted, freed, done.files_skipped
    ));

    if done.files_skipped > 0 {
        EXIT_PARTIAL
    } else {
        EXIT_OK
    }
}
