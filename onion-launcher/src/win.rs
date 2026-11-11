//! Small Windows helpers for the box: the tray icon, menus, light/dark frame
//! and menus, cloaking, monitors, and keeping the Start menu shut.

use crate::*;

const TRAY_ID: u32 = 1;

/// The tray icon, its tooltip naming the hotkey: "Onion Launcher (Win+`)".
pub fn add_tray(hwnd: HWND, icon: HICON, keys: &str) {
    let mut nid = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: TRAY_ID,
        uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
        uCallbackMessage: WM_TRAY,
        hIcon: icon,
        ..Default::default()
    };
    set_tip(&mut nid, keys);
    unsafe {
        let _ = Shell_NotifyIconW(NIM_ADD, &nid);
    }
}

/// A new hotkey: the tray tooltip says so.
pub fn set_tray_tip(hwnd: HWND, keys: &str) {
    let mut nid = NOTIFYICONDATAW { cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32, hWnd: hwnd, uID: TRAY_ID, uFlags: NIF_TIP, ..Default::default() };
    set_tip(&mut nid, keys);
    unsafe {
        let _ = Shell_NotifyIconW(NIM_MODIFY, &nid);
    }
}

fn set_tip(nid: &mut NOTIFYICONDATAW, keys: &str) {
    let text = format!("Onion Launcher ({keys})");
    // at most 127 characters, then the closing 0
    for (d, c) in nid.szTip.iter_mut().take(127).zip(text.encode_utf16()) {
        *d = c;
    }
}

pub fn remove_tray(hwnd: HWND) {
    let nid = NOTIFYICONDATAW { cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32, hWnd: hwnd, uID: TRAY_ID, ..Default::default() };
    unsafe {
        let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
    }
}

/// A plain menu at the mouse: (id, text), id 0 = separator. Returns the picked id (0 = none).
pub fn popup_menu(items: &[(usize, &str)]) -> usize {
    unsafe {
        let Ok(m) = CreatePopupMenu() else { return 0 };
        for &(id, t) in items {
            if id == 0 {
                let _ = AppendMenuW(m, MF_SEPARATOR, 0, PCWSTR::null());
            } else {
                menu_item(m, id, t, false);
            }
        }
        track_menu(m)
    }
}

pub fn menu_item(m: HMENU, id: usize, text: &str, checked: bool) {
    let t = shell::wide(text);
    let flags = if checked { MF_STRING | MF_CHECKED } else { MF_STRING };
    unsafe {
        let _ = AppendMenuW(m, flags, id, PCWSTR(t.as_ptr()));
    }
}

/// Shows a menu at the mouse and returns the picked id (0 = none). Frees it.
pub fn track_menu(m: HMENU) -> usize {
    let s = st();
    unsafe {
        let mut pt = POINT::default();
        let _ = GetCursorPos(&mut pt);
        s.busy = true; // losing focus to the menu mustn't hide the box
        let _ = SetForegroundWindow(s.hwnd);
        // from the tray, the mouse is on the taskbar: open the menu just above it, or the
        // taskbar can cover its last item
        let wa = work_area(MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST));
        let flags = if pt.y >= wa.bottom {
            pt.y = wa.bottom;
            TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_BOTTOMALIGN
        } else {
            TPM_RETURNCMD | TPM_RIGHTBUTTON
        };
        let cmd = TrackPopupMenu(m, flags, pt.x, pt.y, 0, s.hwnd, None);
        s.busy = false;
        let _ = DestroyMenu(m);
        cmd.0 as usize
    }
}

/// Dark or light window frame, and menus to match (uxtheme's unnamed
/// SetPreferredAppMode / FlushMenuThemes, the calls Explorer and Notepad use;
/// skipped if missing).
pub fn apply_theme(hwnd: HWND, dark: bool) {
    let v = BOOL::from(dark);
    unsafe {
        let _ = DwmSetWindowAttribute(hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, &v as *const _ as _, 4);
        let Ok(ux) = LoadLibraryW(w!("uxtheme.dll")) else { return };
        if let Some(f) = GetProcAddress(ux, PCSTR(135 as *const u8)) {
            let set: extern "system" fn(i32) -> i32 = std::mem::transmute(f);
            set(if dark { 2 } else { 3 });
        }
        if let Some(f) = GetProcAddress(ux, PCSTR(136 as *const u8)) {
            let flush: extern "system" fn() = std::mem::transmute(f);
            flush();
        }
    }
}

/// Hides the window from view while it still paints (no empty first frame).
pub fn cloak(hwnd: HWND, on: bool) {
    let v = BOOL::from(on);
    unsafe {
        let _ = DwmSetWindowAttribute(hwnd, DWMWA_CLOAK, &v as *const _ as _, 4);
    }
}

/// The monitor's area without the taskbar.
pub fn work_area(mon: HMONITOR) -> RECT {
    let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
    unsafe {
        let _ = GetMonitorInfoW(mon, &mut mi);
    }
    mi.rcWork
}

/// The monitor's display scale (1.0 = 100%).
pub fn monitor_scale(mon: HMONITOR) -> f32 {
    let (mut x, mut y) = (96u32, 96u32);
    unsafe {
        let _ = GetDpiForMonitor(mon, MDT_EFFECTIVE_DPI, &mut x, &mut y);
    }
    x as f32 / 96.0
}

/// The unassigned key code swallow_win_key presses.
pub const SWALLOW_KEY: u16 = 0xE8;

/// While Windows is held for the hotkey, tell Windows another key was pressed
/// too (an unassigned key code that does nothing), so letting go of Windows
/// doesn't also open the Start menu, which would steal focus and close the box.
/// PowerToys does the same.
pub fn swallow_win_key() {
    unsafe {
        if GetAsyncKeyState(VK_LWIN.0 as i32) >= 0 && GetAsyncKeyState(VK_RWIN.0 as i32) >= 0 {
            return;
        }
        let key = |flags: KEYBD_EVENT_FLAGS| INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: VIRTUAL_KEY(SWALLOW_KEY), wScan: 0, dwFlags: flags, time: 0, dwExtraInfo: 0 } },
        };
        let inputs = [key(KEYBD_EVENT_FLAGS(0)), key(KEYEVENTF_KEYUP)];
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

/// The files dropped on the box.
pub fn dropped_files(hdrop: HDROP) -> Vec<String> {
    unsafe {
        let n = DragQueryFileW(hdrop, u32::MAX, None);
        let files = (0..n)
            .map(|i| {
                let mut buf = vec![0u16; DragQueryFileW(hdrop, i, None) as usize + 1];
                let len = DragQueryFileW(hdrop, i, Some(&mut buf)) as usize;
                String::from_utf16_lossy(&buf[..len])
            })
            .collect();
        DragFinish(hdrop);
        files
    }
}
