// No console window; the app runs silently and only shows a toast.
#![windows_subsystem = "windows"]

use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::ptr::null_mut;
use std::sync::OnceLock;

use winapi::shared::minwindef::{LPARAM, LRESULT, UINT, WPARAM};
use winapi::shared::windef::{HWND, RECT};
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::wingdi::*;
use winapi::um::winuser::*;

const TOAST_DURATION_MS: u32 = 2000;
const TOAST_W: i32 = 320;
const TOAST_H: i32 = 96;
const MARGIN: i32 = 12;

static TOAST_TEXT: OnceLock<String> = OnceLock::new();

#[derive(Default)]
struct Stats {
    files: u64,
    bytes: u64,
}

// ---------------------------------------------------------------- cleaning

fn clean_dir(dir: &Path, stats: &mut Stats) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let meta = match fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };

        if meta.is_dir() {
            clean_dir(&path, stats);
            let _ = fs::remove_dir(&path); // only succeeds if empty
        } else {
            let size = meta.len();
            if remove_file_force(&path) {
                stats.files += 1;
                stats.bytes += size;
            }
        }
    }
}

fn remove_file_force(path: &Path) -> bool {
    if fs::remove_file(path).is_ok() {
        return true;
    }
    // Retry after clearing the read-only attribute.
    if let Ok(meta) = fs::metadata(path) {
        let mut perm = meta.permissions();
        if perm.readonly() {
            #[allow(clippy::permissions_set_readonly_false)]
            perm.set_readonly(false);
            let _ = fs::set_permissions(path, perm);
            return fs::remove_file(path).is_ok();
        }
    }
    false // file is in use or access denied -> skip
}

fn target_folders() -> Vec<PathBuf> {
    let mut list: Vec<PathBuf> = Vec::new();

    // %TEMP% and %TMP% (user temp)
    for var in ["TEMP", "TMP"] {
        if let Ok(v) = env::var(var) {
            list.push(PathBuf::from(v));
        }
    }

    // C:\Windows\Temp and C:\Windows\Prefetch
    let windir = env::var("WINDIR")
        .or_else(|_| env::var("SystemRoot"))
        .unwrap_or_else(|_| String::from(r"C:\Windows"));
    list.push(PathBuf::from(&windir).join("Temp"));
    list.push(PathBuf::from(&windir).join("Prefetch"));

    // Remove duplicates (case-insensitive on Windows)
    let mut seen = HashSet::new();
    list.retain(|p| seen.insert(p.to_string_lossy().to_lowercase()));
    list
}

fn format_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let b = bytes as f64;
    if b >= GB {
        format!("{:.2} GB", b / GB)
    } else if b >= MB {
        format!("{:.2} MB", b / MB)
    } else if b >= KB {
        format!("{:.2} KB", b / KB)
    } else {
        format!("{} B", bytes)
    }
}

// ------------------------------------------------------------------- toast

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: UINT, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);

            let mut rc: RECT = std::mem::zeroed();
            GetClientRect(hwnd, &mut rc);

            // Background
            let bg = CreateSolidBrush(RGB(32, 33, 36));
            FillRect(hdc, &rc, bg);
            DeleteObject(bg as _);

            // Accent bar on the left
            let accent = CreateSolidBrush(RGB(76, 175, 80));
            let bar = RECT { left: 0, top: 0, right: 5, bottom: rc.bottom };
            FillRect(hdc, &bar, accent);
            DeleteObject(accent as _);

            // Text
            let font = CreateFontW(
                -18, 0, 0, 0, FW_NORMAL as i32, 0, 0, 0, DEFAULT_CHARSET as u32,
                OUT_DEFAULT_PRECIS as u32, CLIP_DEFAULT_PRECIS as u32,
                CLEARTYPE_QUALITY as u32, DEFAULT_PITCH as u32,
                wide("Segoe UI").as_ptr(),
            );
            let old = SelectObject(hdc, font as _);
            SetBkMode(hdc, TRANSPARENT as i32);
            SetTextColor(hdc, RGB(255, 255, 255));

            let text = wide(TOAST_TEXT.get().map(|s| s.as_str()).unwrap_or(""));
            let mut tr = RECT { left: 20, top: 10, right: rc.right - 10, bottom: rc.bottom - 10 };
            DrawTextW(hdc, text.as_ptr(), -1, &mut tr, DT_LEFT | DT_VCENTER | DT_WORDBREAK);

            SelectObject(hdc, old);
            DeleteObject(font as _);
            EndPaint(hwnd, &ps);
            0
        }
        WM_TIMER => {
            KillTimer(hwnd, 1);
            DestroyWindow(hwnd);
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

fn show_toast(text: String) {
    let _ = TOAST_TEXT.set(text);

    unsafe {
        let hinst = GetModuleHandleW(null_mut());
        let class_name = wide("TempCleanerToast");

        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinst,
            hIcon: null_mut(),
            hCursor: LoadCursorW(null_mut(), IDC_ARROW),
            hbrBackground: null_mut(),
            lpszMenuName: null_mut(),
            lpszClassName: class_name.as_ptr(),
        };
        RegisterClassW(&wc);

        // Bottom-right corner of the usable desktop area (above the taskbar)
        let mut work: RECT = std::mem::zeroed();
        SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut work as *mut _ as *mut _, 0);
        let x = work.right - TOAST_W - MARGIN;
        let y = work.bottom - TOAST_H - MARGIN;

        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            class_name.as_ptr(),
            wide("Temp Cleaner").as_ptr(),
            WS_POPUP,
            x, y, TOAST_W, TOAST_H,
            null_mut(), null_mut(), hinst, null_mut(),
        );
        if hwnd.is_null() {
            return;
        }

        // Rounded corners
        let rgn = CreateRoundRectRgn(0, 0, TOAST_W + 1, TOAST_H + 1, 14, 14);
        SetWindowRgn(hwnd, rgn, 1);

        ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        UpdateWindow(hwnd);
        SetTimer(hwnd, 1, TOAST_DURATION_MS, None);

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

// -------------------------------------------------------------------- main

fn main() {
    let mut stats = Stats::default();

    for folder in target_folders() {
        clean_dir(&folder, &mut stats);
    }

    let text = format!(
        "Temp Cleaner\nDeleted files: {}\nFreed space: {}",
        stats.files,
        format_size(stats.bytes)
    );
    show_toast(text);
}
