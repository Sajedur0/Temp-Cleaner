//! OS integration: console attachment, message boxes, the toast popup and
//! the Recycle Bin. The real implementation is Windows-only; on other
//! platforms these become harmless stubs so the crate compiles and its
//! unit tests run anywhere.

#[cfg(windows)]
use std::path::Path;
#[cfg(windows)]
use std::ptr::null_mut;
#[cfg(windows)]
use std::sync::OnceLock;

#[cfg(windows)]
use winapi::shared::minwindef::{LPARAM, LRESULT, UINT, WPARAM};
#[cfg(windows)]
use winapi::shared::windef::{HWND, RECT};
#[cfg(windows)]
use winapi::um::fileapi::{CreateFileW, GetLogicalDrives, OPEN_EXISTING};
#[cfg(windows)]
use winapi::um::handleapi::INVALID_HANDLE_VALUE;
#[cfg(windows)]
use winapi::um::libloaderapi::GetModuleHandleW;
#[cfg(windows)]
use winapi::um::processenv::{GetStdHandle, SetStdHandle};
#[cfg(windows)]
use winapi::um::shellapi::{
    SHEmptyRecycleBinW, SHERB_NOCONFIRMATION, SHERB_NOPROGRESSUI, SHERB_NOSOUND,
};
#[cfg(windows)]
use winapi::um::winbase::{STD_ERROR_HANDLE, STD_OUTPUT_HANDLE};
#[cfg(windows)]
use winapi::um::wincon::{AttachConsole, ATTACH_PARENT_PROCESS};
#[cfg(windows)]
use winapi::um::wingdi::*;
#[cfg(windows)]
use winapi::um::winnt::{FILE_SHARE_WRITE, GENERIC_WRITE};
#[cfg(windows)]
use winapi::um::winuser::*;

#[cfg(windows)]
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

// ------------------------------------------------------------ console I/O

/// Attach to the parent process console (needed because this is a
/// `windows_subsystem = "windows"` binary: it starts with no console and
/// invalid std handles). Returns true when printing to stdout will work.
/// Must be called before the first `println!`.
#[cfg(windows)]
pub fn attach_console() -> bool {
    unsafe {
        if AttachConsole(ATTACH_PARENT_PROCESS) == 0 {
            // Maybe a console is already attached (launched from a shell
            // that gave us valid handles).
            let h = GetStdHandle(STD_OUTPUT_HANDLE);
            return !h.is_null() && h != INVALID_HANDLE_VALUE;
        }
        // Point Rust's stdout/stderr at the console we just attached to.
        let conout = CreateFileW(
            wide("CONOUT$").as_ptr(),
            GENERIC_WRITE,
            FILE_SHARE_WRITE,
            null_mut(),
            OPEN_EXISTING,
            0,
            null_mut(),
        );
        if !conout.is_null() && conout != INVALID_HANDLE_VALUE {
            SetStdHandle(STD_OUTPUT_HANDLE, conout);
            SetStdHandle(STD_ERROR_HANDLE, conout);
        }
        true
    }
}

#[cfg(not(windows))]
pub fn attach_console() -> bool {
    true
}

// ------------------------------------------------------------- dialog box

/// Yes/No question box. Default button is "No" — deleting must be a
/// deliberate choice.
#[cfg(windows)]
pub fn confirm_box(title: &str, text: &str) -> bool {
    unsafe {
        MessageBoxW(
            null_mut(),
            wide(text).as_ptr(),
            wide(title).as_ptr(),
            MB_YESNO | MB_ICONQUESTION | MB_DEFBUTTON2,
        ) == IDYES
    }
}

#[cfg(not(windows))]
pub fn confirm_box(_title: &str, text: &str) -> bool {
    println!("{}", text);
    false
}

/// Simple OK information box.
#[cfg(windows)]
pub fn info_box(title: &str, text: &str) {
    unsafe {
        MessageBoxW(
            null_mut(),
            wide(text).as_ptr(),
            wide(title).as_ptr(),
            MB_OK | MB_ICONINFORMATION,
        );
    }
}

#[cfg(not(windows))]
pub fn info_box(_title: &str, text: &str) {
    println!("{}", text);
}

// ------------------------------------------------------------------ toast

#[cfg(windows)]
const TOAST_DURATION_MS: u32 = 4500;
#[cfg(windows)]
const TOAST_W: i32 = 340;
#[cfg(windows)]
const TOAST_H: i32 = 112;
#[cfg(windows)]
const MARGIN: i32 = 12;

#[cfg(windows)]
static TOAST_STATE: OnceLock<(String, bool)> = OnceLock::new();

#[cfg(windows)]
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

            let (text, success) = TOAST_STATE
                .get()
                .map(|(t, s)| (t.as_str(), *s))
                .unwrap_or(("", true));

            // Accent bar on the left: green = cleaned, blue = nothing to do
            let (r, g, b) = if success { (76, 175, 80) } else { (66, 133, 244) };
            let accent = CreateSolidBrush(RGB(r, g, b));
            let bar = RECT {
                left: 0,
                top: 0,
                right: 5,
                bottom: rc.bottom,
            };
            FillRect(hdc, &bar, accent);
            DeleteObject(accent as _);

            // Text
            let font = CreateFontW(
                -17,
                0,
                0,
                0,
                FW_NORMAL as i32,
                0,
                0,
                0,
                DEFAULT_CHARSET as u32,
                OUT_DEFAULT_PRECIS as u32,
                CLIP_DEFAULT_PRECIS as u32,
                CLEARTYPE_QUALITY as u32,
                DEFAULT_PITCH as u32,
                wide("Segoe UI").as_ptr(),
            );
            let old = SelectObject(hdc, font as _);
            SetBkMode(hdc, TRANSPARENT as i32);
            SetTextColor(hdc, RGB(255, 255, 255));

            let text = wide(text);
            let mut tr = RECT {
                left: 20,
                top: 10,
                right: rc.right - 10,
                bottom: rc.bottom - 10,
            };
            DrawTextW(
                hdc,
                text.as_ptr(),
                -1,
                &mut tr,
                DT_LEFT | DT_VCENTER | DT_WORDBREAK,
            );

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
        WM_LBUTTONDOWN | WM_RBUTTONDOWN => {
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

/// Show the auto-dismissing toast in the bottom-right corner.
/// `success = false` renders a blue "nothing to clean" variant.
/// Clicking the toast dismisses it immediately.
#[cfg(windows)]
pub fn show_toast(text: &str, success: bool) {
    let _ = TOAST_STATE.set((text.to_string(), success));

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
            x,
            y,
            TOAST_W,
            TOAST_H,
            null_mut(),
            null_mut(),
            hinst,
            null_mut(),
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

#[cfg(not(windows))]
pub fn show_toast(text: &str, _success: bool) {
    println!("{}", text);
}

// ------------------------------------------------------------ recycle bin

/// Best-effort size estimate of all Recycle Bins on fixed drives
/// (sums the real files under `<drive>:\$Recycle.Bin`).
#[cfg(windows)]
pub fn recycle_bin_size() -> u64 {
    fn dir_size(path: &Path) -> u64 {
        let mut total = 0u64;
        let mut stack = vec![path.to_path_buf()];
        while let Some(dir) = stack.pop() {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    if let Ok(meta) = std::fs::symlink_metadata(entry.path()) {
                        if meta.is_dir() && !meta.file_type().is_symlink() {
                            stack.push(entry.path());
                        } else {
                            total += meta.len();
                        }
                    }
                }
            }
        }
        total
    }

    let mut total = 0u64;
    let mask = unsafe { GetLogicalDrives() };
    for i in 0..26u32 {
        if mask & (1u32 << i) == 0 {
            continue;
        }
        let drive = format!("{}:\\", (b'A' + i as u8) as char);
        total += dir_size(&Path::new(&drive).join("$Recycle.Bin"));
    }
    total
}

#[cfg(not(windows))]
pub fn recycle_bin_size() -> u64 {
    0
}

/// Empty the Recycle Bin on all drives, without confirmation or progress UI.
#[cfg(windows)]
pub fn empty_recycle_bin() -> bool {
    unsafe {
        SHEmptyRecycleBinW(
            null_mut(),
            null_mut(),
            SHERB_NOCONFIRMATION | SHERB_NOPROGRESSUI | SHERB_NOSOUND,
        ) >= 0
    }
}

#[cfg(not(windows))]
pub fn empty_recycle_bin() -> bool {
    false
}
