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
use winapi::shared::minwindef::{DWORD, LPARAM, LRESULT, UINT, WPARAM};
#[cfg(windows)]
use winapi::shared::ntdef::LUID;
#[cfg(windows)]
use winapi::shared::windef::{HWND, RECT};
#[cfg(windows)]
use winapi::um::fileapi::{
    CreateFileW, GetDriveTypeW, GetFileAttributesW, GetLogicalDrives, SetFileAttributesW,
    INVALID_FILE_ATTRIBUTES, OPEN_EXISTING,
};
#[cfg(windows)]
use winapi::um::handleapi::{CloseHandle, INVALID_HANDLE_VALUE};
#[cfg(windows)]
use winapi::um::libloaderapi::GetModuleHandleW;
#[cfg(windows)]
use winapi::um::processenv::{GetStdHandle, SetStdHandle};
#[cfg(windows)]
use winapi::um::processthreadsapi::{GetCurrentProcess, OpenProcessToken};
#[cfg(windows)]
use winapi::um::securitybaseapi::{AdjustTokenPrivileges, GetTokenInformation};
#[cfg(windows)]
use winapi::um::shellapi::{
    SHEmptyRecycleBinW, SHQueryRecycleBinW, SHERB_NOCONFIRMATION, SHERB_NOPROGRESSUI,
    SHERB_NOSOUND, SHQUERYRBINFO,
};
#[cfg(windows)]
use winapi::um::winbase::{
    LookupPrivilegeValueW, MoveFileExW, DRIVE_FIXED, MOVEFILE_DELAY_UNTIL_REBOOT,
    STD_ERROR_HANDLE, STD_OUTPUT_HANDLE,
};
#[cfg(windows)]
use winapi::um::wincon::{AttachConsole, ATTACH_PARENT_PROCESS};
#[cfg(windows)]
use winapi::um::wingdi::*;
#[cfg(windows)]
use winapi::um::winnt::{
    FILE_ATTRIBUTE_READONLY, FILE_SHARE_WRITE, GENERIC_WRITE, HANDLE, LUID_AND_ATTRIBUTES,
    SE_PRIVILEGE_ENABLED, TOKEN_ADJUST_PRIVILEGES, TOKEN_ELEVATION, TOKEN_ELEVATION_TYPE,
    TOKEN_PRIVILEGES, TOKEN_QUERY, TokenElevation, TokenElevationType, TokenElevationTypeDefault,
};
#[cfg(windows)]
use winapi::um::winuser::*;

#[cfg(windows)]
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

// ------------------------------------------------------- privileges / admin

/// Privileges that let an elevated process delete files whose ACL would
/// otherwise refuse it (classic backup/cleanup technique). They are only
/// granted if the token already holds them — this never escalates rights, it
/// only *enables* ones the user already has.
#[cfg(windows)]
const CLEANUP_PRIVILEGES: [&str; 3] = [
    "SeBackupPrivilege",
    "SeRestorePrivilege",
    "SeTakeOwnershipPrivilege",
];

/// Enable the privileges above for this process. Best effort: silently does
/// nothing when the process is not elevated (a limited token does not hold
/// them at all) or when the API calls fail.
#[cfg(windows)]
pub fn enable_cleanup_privileges() {
    unsafe {
        let mut token: HANDLE = null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_ADJUST_PRIVILEGES, &mut token) == 0 {
            return;
        }
        for name in CLEANUP_PRIVILEGES {
            let mut luid: LUID = std::mem::zeroed();
            if LookupPrivilegeValueW(std::ptr::null(), wide(name).as_ptr(), &mut luid) == 0 {
                continue;
            }
            let mut tp: TOKEN_PRIVILEGES = std::mem::zeroed();
            tp.PrivilegeCount = 1;
            tp.Privileges[0] = LUID_AND_ATTRIBUTES {
                Luid: luid,
                Attributes: SE_PRIVILEGE_ENABLED,
            };
            AdjustTokenPrivileges(
                token,
                0,
                &mut tp,
                std::mem::size_of::<TOKEN_PRIVILEGES>() as DWORD,
                null_mut(),
                null_mut(),
            );
        }
        CloseHandle(token);
    }
}

#[cfg(not(windows))]
pub fn enable_cleanup_privileges() {}

/// Is this process running with a full administrator token? Used only to warn
/// the user: without elevation `C:\Windows\Temp` and `C:\Windows\Prefetch`
/// cannot be cleaned and every file there is reported as "access denied".
#[cfg(windows)]
pub fn is_elevated() -> bool {
    unsafe {
        let mut token: HANDLE = null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }
        // TokenIsElevated is 0 when UAC is off (there is no split token to
        // elevate), so a "default" elevation type counts as running with full
        // rights too.
        let mut elevation: TOKEN_ELEVATION = std::mem::zeroed();
        let mut returned: DWORD = 0;
        let queried = GetTokenInformation(
            token,
            TokenElevation,
            &mut elevation as *mut _ as *mut _,
            std::mem::size_of::<TOKEN_ELEVATION>() as DWORD,
            &mut returned,
        );
        let mut kind: TOKEN_ELEVATION_TYPE = 0;
        let mut kind_len: DWORD = 0;
        let kind_queried = GetTokenInformation(
            token,
            TokenElevationType,
            &mut kind as *mut _ as *mut _,
            std::mem::size_of::<TOKEN_ELEVATION_TYPE>() as DWORD,
            &mut kind_len,
        );
        CloseHandle(token);
        (queried != 0 && elevation.TokenIsElevated != 0)
            || (kind_queried != 0 && kind == TokenElevationTypeDefault)
    }
}

#[cfg(not(windows))]
pub fn is_elevated() -> bool {
    true
}

/// Clear the read-only attribute so a file can be deleted. Returns true when
/// an attribute was actually changed.
#[cfg(windows)]
pub fn clear_readonly_attribute(path: &Path) -> bool {
    unsafe {
        let wide_path = wide(&path.to_string_lossy());
        let attrs = GetFileAttributesW(wide_path.as_ptr());
        if attrs == INVALID_FILE_ATTRIBUTES || attrs & FILE_ATTRIBUTE_READONLY == 0 {
            return false;
        }
        SetFileAttributesW(wide_path.as_ptr(), attrs & !FILE_ATTRIBUTE_READONLY) != 0
    }
}

#[cfg(not(windows))]
pub fn clear_readonly_attribute(_path: &std::path::Path) -> bool {
    false
}

/// Queue a file for deletion at the next system start.
///
/// This is the **only** way to remove a file another process holds open
/// without delete sharing — which is exactly the case for Windows' own
/// Prefetch records, some thumbnail databases and locked service caches.
/// `MoveFileEx` never opens the file itself: it only writes the name into
/// `HKLM\...\Session Manager\PendingFileRenameOperations`, so a sharing
/// violation cannot stop it. That also means it needs Administrator rights,
/// and nothing is removed until the machine restarts.
///
/// Returns true when the deletion was queued.
#[cfg(windows)]
pub fn schedule_delete_on_reboot(path: &Path) -> bool {
    // MoveFileEx refuses to queue a read-only file, so clear the attribute
    // first (harmless: we are about to delete it anyway).
    clear_readonly_attribute(path);
    unsafe {
        let wide_path = wide(&path.to_string_lossy());
        MoveFileExW(
            wide_path.as_ptr(),
            std::ptr::null(),
            MOVEFILE_DELAY_UNTIL_REBOOT,
        ) != 0
    }
}

#[cfg(not(windows))]
pub fn schedule_delete_on_reboot(_path: &std::path::Path) -> bool {
    false
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
const TOAST_W: i32 = 360;
#[cfg(windows)]
const TOAST_H: i32 = 142;
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

/// Root paths (`C:\`) of every *fixed* drive on this machine.
///
/// Only fixed drives take part in the Recycle Bin work. Counting removable
/// drives and empty card readers was the old behaviour and it produced a
/// phantom "Recycle Bin" size that could never be emptied (a CD-ROM drive has
/// no bin at all, and `SHEmptyRecycleBin` cannot clear one).
#[cfg(windows)]
fn fixed_drive_roots() -> Vec<String> {
    let mut roots = Vec::new();
    let mask = unsafe { GetLogicalDrives() };
    for i in 0..26u32 {
        if mask & (1u32 << i) == 0 {
            continue;
        }
        let root = format!("{}:\\", (b'A' + i as u8) as char);
        let wide_root = wide(&root);
        let kind = unsafe { GetDriveTypeW(wide_root.as_ptr()) };
        if kind == DRIVE_FIXED {
            roots.push(root);
        }
    }
    roots
}

/// Size of one drive's Recycle Bin, as reported by the Shell itself — the
/// same number Explorer shows. Returns `None` when the Shell call fails.
#[cfg(windows)]
fn query_recycle_bin(root: &str) -> Option<u64> {
    unsafe {
        let mut info: SHQUERYRBINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<SHQUERYRBINFO>() as DWORD;
        if SHQueryRecycleBinW(wide(root).as_ptr(), &mut info) < 0 {
            return None;
        }
        Some(if info.i64Size > 0 { info.i64Size as u64 } else { 0 })
    }
}

/// Fallback: sum the real files under `<drive>:\$Recycle.Bin`, used only when
/// the Shell refuses to answer (it needs a logged-on interactive session).
#[cfg(windows)]
fn recycle_bin_size_by_walking() -> u64 {
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

    fixed_drive_roots()
        .iter()
        .map(|root| dir_size(&Path::new(root).join("$Recycle.Bin")))
        .sum()
}

/// How much space the Recycle Bin of every fixed drive is using.
///
/// Querying the Shell (instead of walking `$Recycle.Bin` ourselves) matters:
/// the walk sees *every* user's bin plus orphaned SID folders left behind by
/// deleted accounts, while `SHEmptyRecycleBin` only removes what the Shell
/// considers recyclable. That mismatch is why the bin used to keep showing a
/// size no matter how often it was cleaned.
#[cfg(windows)]
pub fn recycle_bin_size() -> u64 {
    let mut total = 0u64;
    let mut queried_any = false;
    for root in fixed_drive_roots() {
        if let Some(bytes) = query_recycle_bin(&root) {
            queried_any = true;
            total += bytes;
        }
    }
    if queried_any {
        total
    } else {
        recycle_bin_size_by_walking()
    }
}

#[cfg(not(windows))]
pub fn recycle_bin_size() -> u64 {
    0
}

/// Combined flags for `SHEmptyRecycleBinW`: no prompt, no progress, no sound.
#[cfg(windows)]
const EMPTY_FLAGS: DWORD = SHERB_NOCONFIRMATION | SHERB_NOPROGRESSUI | SHERB_NOSOUND;

/// Empty the Recycle Bin, without confirmation or progress UI.
///
/// Returns the size `(before, after)` so the caller can report what was
/// genuinely freed. The size is re-queried afterwards because
/// `SHEmptyRecycleBinW` happily returns success while leaving items behind
/// (files in use, or a bin owned by another user), and because it reports
/// failure when the bin was already empty.
///
/// Drives are emptied one at a time in addition to the "all drives" call: on
/// some systems the `NULL` root silently skips drives.
#[cfg(windows)]
pub fn empty_recycle_bin() -> (u64, u64) {
    let before = recycle_bin_size();
    unsafe {
        SHEmptyRecycleBinW(null_mut(), std::ptr::null(), EMPTY_FLAGS);
        for root in fixed_drive_roots() {
            SHEmptyRecycleBinW(null_mut(), wide(&root).as_ptr(), EMPTY_FLAGS);
        }
    }
    let after = recycle_bin_size();
    (before, after)
}

#[cfg(not(windows))]
pub fn empty_recycle_bin() -> (u64, u64) {
    (0, 0)
}
