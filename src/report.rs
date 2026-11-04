//! Small formatting helpers and the run logger. Pure `std`.

use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Human-readable size: `512 B`, `12.30 KB`, `1.25 GB`, …
pub fn format_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    const TB: f64 = GB * 1024.0;
    let b = bytes as f64;
    if b >= TB {
        format!("{:.2} TB", b / TB)
    } else if b >= GB {
        format!("{:.2} GB", b / GB)
    } else if b >= MB {
        format!("{:.2} MB", b / MB)
    } else if b >= KB {
        format!("{:.2} KB", b / KB)
    } else {
        format!("{} B", bytes)
    }
}

/// `1234567` -> `1,234,567` for display.
pub fn format_number(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// Format a timestamp as `YYYY-MM-DD HH:MM:SS UTC` without any date crate.
/// (Civil-from-days algorithm by Howard Hinnant.)
pub fn format_timestamp(t: SystemTime) -> String {
    let secs = t
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (hh, mm, ss) = (rem / 3600, (rem % 3600) / 60, rem % 60);

    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097); // day of era, [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };

    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC",
        year, m, d, hh, mm, ss
    )
}

/// Writes timestamped lines to an optional log file, and (if enabled) to the
/// console. `detail` lines only reach the console in verbose mode, so a
/// normal run stays short while the log file keeps the full picture.
pub struct Logger {
    file: Option<File>,
    console: bool,
    verbose_console: bool,
}

impl Logger {
    pub fn new(log_path: Option<&Path>, console: bool, verbose_console: bool) -> Logger {
        let file = log_path.and_then(|p| {
            if let Some(parent) = p.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            match std::fs::OpenOptions::new().create(true).append(true).open(p) {
                Ok(f) => Some(f),
                Err(e) => {
                    if console {
                        println!("Warning: could not open log file {}: {}", p.display(), e);
                    }
                    None
                }
            }
        });
        Logger {
            file,
            console,
            verbose_console,
        }
    }

    pub fn info(&mut self, msg: &str) {
        if self.console {
            println!("{}", msg);
        }
        self.write_file(msg);
    }

    pub fn detail(&mut self, msg: &str) {
        if self.console && self.verbose_console {
            println!("{}", msg);
        }
        self.write_file(msg);
    }

    fn write_file(&mut self, msg: &str) {
        if let Some(f) = &mut self.file {
            let _ = writeln!(f, "[{}] {}", format_timestamp(SystemTime::now()), msg);
            let _ = f.flush();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn sizes() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(1536), "1.50 KB");
        assert_eq!(format_size(5 * 1024 * 1024), "5.00 MB");
        assert_eq!(format_size(3 * 1024 * 1024 * 1024), "3.00 GB");
    }

    #[test]
    fn numbers() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(999), "999");
        assert_eq!(format_number(1000), "1,000");
        assert_eq!(format_number(1_234_567), "1,234,567");
    }

    #[test]
    fn timestamps() {
        assert_eq!(
            format_timestamp(UNIX_EPOCH),
            "1970-01-01 00:00:00 UTC"
        );
        assert_eq!(
            format_timestamp(UNIX_EPOCH + Duration::from_secs(1_700_000_000)),
            "2023-11-14 22:13:20 UTC"
        );
        assert_eq!(
            format_timestamp(UNIX_EPOCH + Duration::from_secs(951_782_400)),
            "2000-02-29 00:00:00 UTC" // leap day
        );
    }
}
