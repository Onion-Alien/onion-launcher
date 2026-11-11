//! Talking to Windows: launching, the list of installed apps, app icons as
//! raw pixels, the file picker, the clipboard and the light/dark setting.

use crate::config::App;
use std::collections::HashMap;
use std::ffi::c_void;
use std::path::{Path, PathBuf};

use windows::core::{w, PCWSTR, PWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::Storage::FileSystem::{FILE_ATTRIBUTE_NORMAL, FILE_FLAGS_AND_ATTRIBUTES};
use windows::Win32::System::Com::*;
use windows::Win32::System::DataExchange::*;
use windows::Win32::System::Memory::*;
use windows::Win32::UI::Controls::Dialogs::*;
use windows::Win32::UI::Shell::Common::ITEMIDLIST;
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::*;

pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// COM for the shell calls (ShellExecute on .lnk files, icons, the app list).
pub fn com_init() {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE);
    }
}

/// Opens the app the way Explorer would (.exe, .lnk, .url, Store apps, links).
pub fn launch(app: &App) -> bool {
    let file = wide(&app.path);
    let args = wide(&app.args);
    // start in the app's folder, but only for real files ("shell:AppsFolder\..." has none)
    let p = Path::new(&app.path);
    let dir = if p.is_file() { p.parent().map(|d| d.display().to_string()).unwrap_or_default() } else { String::new() };
    let dirw = wide(&dir);
    unsafe {
        let r = ShellExecuteW(
            HWND::default(),
            w!("open"),
            PCWSTR(file.as_ptr()),
            if app.args.is_empty() { PCWSTR::null() } else { PCWSTR(args.as_ptr()) },
            if dir.is_empty() { PCWSTR::null() } else { PCWSTR(dirw.as_ptr()) },
            SW_SHOWNORMAL,
        );
        r.0 as isize > 32
    }
}

/// Every app in Start's "All apps" list (desktop and Store apps), as
/// name + "shell:AppsFolder\<id>". Uninstallers and web links are skipped.
pub fn installed_apps() -> Vec<App> {
    let mut out = vec![];
    let link_icons = start_menu_link_icons();
    unsafe {
        let Ok(folder) = SHCreateItemFromParsingName::<_, _, IShellItem>(w!("shell:AppsFolder"), None) else { return out };
        let Ok(items) = folder.BindToHandler::<_, IEnumShellItems>(None, &BHID_EnumItems) else { return out };
        loop {
            let mut one = [None];
            let mut got = 0u32;
            if items.Next(&mut one, Some(&mut got)).is_err() || got == 0 {
                break;
            }
            let Some(item) = one[0].take() else { break };
            let get = |k: SIGDN| -> Option<String> {
                let p = item.GetDisplayName(k).ok()?;
                let s = p.to_string().ok();
                CoTaskMemFree(Some(p.0 as *const c_void));
                s
            };
            let (Some(name), Some(id)) = (get(SIGDN_NORMALDISPLAY), get(SIGDN_PARENTRELATIVEPARSING)) else { continue };
            let low = name.to_lowercase();
            if low.contains("uninstall") || id.starts_with("http") || id.to_lowercase().ends_with(".url") {
                continue;
            }
            if id.contains("://") {
                // a game-launcher link (uplay://, steam://...): open the link itself,
                // take the picture from the .url file's IconFile
                let icon = link_icons.get(&id.to_lowercase()).cloned().unwrap_or_default();
                out.push(App::new(name, id, icon));
            } else {
                out.push(App::new(name, format!(r"shell:AppsFolder\{id}"), String::new()));
            }
        }
    }
    out.sort_by_cached_key(|a| a.name.to_lowercase());
    out.dedup_by(|a, b| a.name.eq_ignore_ascii_case(&b.name));
    out
}

/// URL -> IconFile for every .url shortcut in the two Start menu folders.
fn start_menu_link_icons() -> HashMap<String, String> {
    let mut map = HashMap::new();
    let mut dirs: Vec<PathBuf> = ["APPDATA", "ProgramData"]
        .iter()
        .filter_map(std::env::var_os)
        .map(|b| PathBuf::from(b).join(r"Microsoft\Windows\Start Menu\Programs"))
        .collect();
    while let Some(d) = dirs.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if e.file_type().is_ok_and(|t| t.is_dir()) {
                dirs.push(p);
            } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("url")) {
                let Ok(text) = std::fs::read_to_string(&p) else { continue };
                let field = |k: &str| text.lines().find_map(|l| l.strip_prefix(k)).map(|v| v.trim().to_string());
                if let (Some(url), Some(icon)) = (field("URL="), field("IconFile=")) {
                    if !icon.is_empty() {
                        map.insert(url.to_lowercase(), icon.replace('/', "\\"));
                    }
                }
            }
        }
    }
    map
}

/// Windows "app mode" setting: true = dark.
pub fn system_dark() -> bool {
    use windows::Win32::System::Registry::*;
    let mut v = 1u32;
    let mut n = 4u32;
    unsafe {
        let _ = RegGetValueW(
            HKEY_CURRENT_USER,
            w!(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize"),
            w!("AppsUseLightTheme"),
            RRF_RT_REG_DWORD,
            None,
            Some(&mut v as *mut u32 as *mut c_void),
            Some(&mut n),
        );
    }
    v == 0
}

// ---------- icons ----------

/// A picture as premultiplied BGRA rows, top row first.
pub struct Pixels {
    pub w: u32,
    pub h: u32,
    pub bgra: Vec<u8>,
}

impl Pixels {
    fn blank(&self) -> bool {
        self.bgra.as_chunks::<4>().0.iter().all(|p| p[3] == 0)
    }
    /// Some sources hand back straight alpha: premultiply those.
    fn fix_alpha(&mut self) {
        let straight = self.bgra.as_chunks::<4>().0.iter().any(|p| p[0] > p[3] || p[1] > p[3] || p[2] > p[3]);
        if straight {
            for p in self.bgra.as_chunks_mut::<4>().0 {
                let a = p[3] as u32;
                for c in &mut p[..3] {
                    *c = ((*c as u32 * a + 127) / 255) as u8;
                }
            }
        }
    }
}

/// The app's icon at `size` px: a .ico file directly, else what Explorer shows
/// (Store apps and links too), else the shell's generic icon.
pub fn load_icon(src: &str, size: i32) -> Option<Pixels> {
    unsafe {
        if src.to_lowercase().ends_with(".ico") {
            let pw = wide(src);
            if let Ok(h) = LoadImageW(None, PCWSTR(pw.as_ptr()), IMAGE_ICON, size, size, LR_LOADFROMFILE) {
                let p = hicon_pixels(HICON(h.0), size);
                let _ = DestroyIcon(HICON(h.0));
                if p.is_some() {
                    return p;
                }
            }
        }
        let pw = wide(src);
        if let Ok(f) = SHCreateItemFromParsingName::<_, _, IShellItemImageFactory>(PCWSTR(pw.as_ptr()), None) {
            if let Ok(b) = f.GetImage(SIZE { cx: size, cy: size }, SIIGBF_ICONONLY) {
                let p = hbitmap_pixels(b);
                let _ = DeleteObject(b);
                // a blank picture means a busy icon cache: use the older lookup below
                if let Some(mut p) = p.filter(|p| !p.blank()) {
                    p.fix_alpha();
                    return Some(p);
                }
            }
        }
        let h = large_hicon(src);
        if h.is_invalid() {
            return None;
        }
        let p = hicon_pixels(h, size);
        let _ = DestroyIcon(h);
        p
    }
}

unsafe fn dib(w: i32, h: i32) -> Option<(HDC, HBITMAP, HGDIOBJ, *mut u8)> {
    let dc = CreateCompatibleDC(None);
    let info = bmi(w, h);
    let mut bits: *mut c_void = std::ptr::null_mut();
    let Ok(bmp) = CreateDIBSection(dc, &info, DIB_RGB_COLORS, &mut bits, None, 0) else {
        let _ = DeleteDC(dc);
        return None;
    };
    let old = SelectObject(dc, bmp);
    Some((dc, bmp, old, bits as *mut u8))
}

unsafe fn dib_free(dc: HDC, bmp: HBITMAP, old: HGDIOBJ) {
    SelectObject(dc, old);
    let _ = DeleteObject(bmp);
    let _ = DeleteDC(dc);
}

/// A top-down 32-bit BITMAPINFO.
pub fn bmi(w: i32, h: i32) -> BITMAPINFO {
    BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Any HBITMAP -> top-down BGRA (GetDIBits does the flipping and format work).
unsafe fn hbitmap_pixels(b: HBITMAP) -> Option<Pixels> {
    let mut bm = BITMAP::default();
    if GetObjectW(b, std::mem::size_of::<BITMAP>() as i32, Some(&mut bm as *mut _ as *mut c_void)) == 0 {
        return None;
    }
    let (w, h) = (bm.bmWidth, bm.bmHeight.abs());
    let mut info = bmi(w, h);
    let mut bgra = vec![0u8; (w * h * 4) as usize];
    let dc = CreateCompatibleDC(None);
    let n = GetDIBits(dc, b, 0, h as u32, Some(bgra.as_mut_ptr() as *mut c_void), &mut info, DIB_RGB_COLORS);
    let _ = DeleteDC(dc);
    (n == h).then_some(Pixels { w: w as u32, h: h as u32, bgra })
}

unsafe fn hicon_pixels(icon: HICON, size: i32) -> Option<Pixels> {
    let n = (size * size * 4) as usize;
    let grab = |flags: DI_FLAGS| -> Option<Vec<u8>> {
        let (dc, bmp, old, bits) = dib(size, size)?;
        let _ = DrawIconEx(dc, 0, 0, icon, size, size, 0, None, flags);
        let _ = GdiFlush();
        let px = std::slice::from_raw_parts(bits, n).to_vec();
        dib_free(dc, bmp, old);
        Some(px)
    };
    let mut px = grab(DI_NORMAL)?;
    if px.as_chunks::<4>().0.iter().all(|p| p[3] == 0) {
        // old icon without alpha: take transparency from its mask
        let mask = grab(DI_MASK)?;
        for (p, m) in px.as_chunks_mut::<4>().0.iter_mut().zip(mask.as_chunks::<4>().0) {
            if m[0] == 0 { p[3] = 255 } else { p.copy_from_slice(&[0, 0, 0, 0]) }
        }
    }
    Some(Pixels { w: size as u32, h: size as u32, bgra: px })
}

/// The shell's large icon for a path or "shell:..." entry; the generic program
/// icon for a missing file. Caller owns it.
fn large_hicon(path: &str) -> HICON {
    let pw = wide(path);
    let n = std::mem::size_of::<SHFILEINFOW>() as u32;
    let flags = SHGFI_ICON | SHGFI_LARGEICON;
    let mut sfi = SHFILEINFOW::default();
    unsafe {
        if path.to_ascii_lowercase().starts_with("shell:") {
            let mut pidl: *mut ITEMIDLIST = std::ptr::null_mut();
            if SHParseDisplayName(PCWSTR(pw.as_ptr()), None, &mut pidl, 0, None).is_ok() && !pidl.is_null() {
                SHGetFileInfoW(PCWSTR(pidl as *const u16), FILE_FLAGS_AND_ATTRIBUTES(0), Some(&mut sfi), n, flags | SHGFI_PIDL);
                CoTaskMemFree(Some(pidl as *const c_void));
            }
            return sfi.hIcon;
        }
        if SHGetFileInfoW(PCWSTR(pw.as_ptr()), FILE_FLAGS_AND_ATTRIBUTES(0), Some(&mut sfi), n, flags) != 0 {
            return sfi.hIcon;
        }
        let gen = wide("x.exe");
        SHGetFileInfoW(PCWSTR(gen.as_ptr()), FILE_ATTRIBUTE_NORMAL, Some(&mut sfi), n, flags | SHGFI_USEFILEATTRIBUTES);
        sfi.hIcon
    }
}

// ---------- dialogs and clipboard ----------

/// The Open dialog, multi-select, shortcuts kept as shortcuts.
pub fn pick_files(owner: HWND) -> Vec<String> {
    let mut buf = vec![0u16; 32 * 1024];
    let filter: Vec<u16> = "Programs and shortcuts\0*.exe;*.lnk;*.bat;*.cmd;*.url;*.appref-ms\0All files\0*.*\0\0".encode_utf16().collect();
    let mut ofn = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: owner,
        lpstrFilter: PCWSTR(filter.as_ptr()),
        lpstrFile: PWSTR(buf.as_mut_ptr()),
        nMaxFile: buf.len() as u32,
        lpstrTitle: w!("Add apps"),
        // NOCHANGEDIR: without it the folder picked from stays this app's working folder,
        // so it is "in use" (can't be renamed or deleted, a USB stick can't be ejected)
        // for as long as Onion Launcher runs
        Flags: OFN_EXPLORER | OFN_ALLOWMULTISELECT | OFN_FILEMUSTEXIST | OFN_NODEREFERENCELINKS | OFN_HIDEREADONLY | OFN_NOCHANGEDIR,
        ..Default::default()
    };
    unsafe {
        if !GetOpenFileNameW(&mut ofn).as_bool() {
            return vec![];
        }
    }
    // "dir\0a.exe\0b.lnk\0\0" or "C:\full\path.exe\0\0"
    let parts: Vec<String> = buf.split(|&c| c == 0).take_while(|s| !s.is_empty()).map(String::from_utf16_lossy).collect();
    match parts.len() {
        0 => vec![],
        1 => parts,
        _ => parts[1..].iter().map(|f| format!("{}\\{}", parts[0].trim_end_matches('\\'), f)).collect(),
    }
}

/// The start of the clipboard's text, if it holds any. The text box takes one short
/// line, so a whole copied file is never read in.
pub fn clipboard_text(owner: HWND) -> Option<String> {
    const CF_UNICODETEXT: u32 = 13;
    const MAX_UNITS: usize = 4096;
    unsafe {
        OpenClipboard(owner).ok()?;
        let mut out = None;
        if let Ok(h) = GetClipboardData(CF_UNICODETEXT) {
            let g = HGLOBAL(h.0);
            let p = GlobalLock(g) as *const u16;
            if !p.is_null() {
                // up to the first 0, never past the end of the block
                let max = (GlobalSize(g) / 2).min(MAX_UNITS);
                let n = (0..max).find(|&i| *p.add(i) == 0).unwrap_or(max);
                out = Some(String::from_utf16_lossy(std::slice::from_raw_parts(p, n)));
                let _ = GlobalUnlock(g);
            }
        }
        let _ = CloseClipboard();
        out
    }
}
