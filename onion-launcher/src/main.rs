//! Onion Launcher: Win+` (or the tray icon) pops a small box next to the mouse
//! with your apps. Click one (or press its number) to open it. The "+" tile
//! turns the box into a search of every app on the PC; click one to add it.
//! Apps can be dragged around, sorted into categories, and removed (right-click,
//! or hold one outside the box). Files dropped on the box are added too.
//!
//! Files: main.rs (state, window, start-up), grid.rs (the app box), add.rs
//! (the "+" screen), field.rs (the text box), gfx.rs (drawing), theme.rs
//! (colours), win.rs / shell.rs (Windows calls), worker.rs (background
//! loading), config.rs (the saved list), selftest.rs (the automatic test).
#![windows_subsystem = "windows"]
#![allow(static_mut_refs)]

mod add;
mod config;
mod dialog;
mod dib;
mod field;
mod gfx;
mod grid;
mod install;
mod selftest;
mod shell;
mod theme;
mod win;
mod worker;

use config::{App, Config};
use gfx::{hex, Gfx, Rect};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::Instant;
use theme::{Theme, G_ADD, G_BACK, G_CHECK, G_DELETE, G_FOLDER, G_SEARCH};

use windows::core::{w, HSTRING, PCSTR, PCWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Direct2D::ID2D1Bitmap;
use windows::Win32::Graphics::DirectWrite::*;
use windows::Win32::Graphics::Dwm::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress, LoadLibraryW};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::Controls::MARGINS;
use windows::Win32::UI::HiDpi::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::*;

const WM_TRAY: u32 = WM_APP + 1;
const WM_LOADED: u32 = WM_APP + 2;
const WM_LAUNCHED: u32 = WM_APP + 3;
/// Sent by the installer (`install::quit_running`) before it replaces or removes the exe.
pub const WM_QUIT_APP: u32 = WM_APP + 4;
/// The main window's class; the installer finds the running copy by it.
pub const CLASS: PCWSTR = w!("OnionLauncher");
const WM_MOUSELEAVE: u32 = 0x02A3;
const WM_NCUAHDRAWCAPTION: u32 = 0x00AE;
const WM_NCUAHDRAWFRAME: u32 = 0x00AF;
const HOTKEY_ID: i32 = 1;
const TIMER_ANIM: usize = 1;
const TIMER_CARET: usize = 2;
const TIMER_TEST: usize = 3;

// ---------- layout, in DIPs (1/96 inch) ----------

const COLS: usize = 4;
const PAD: f32 = 12.0;
const TILE_W: f32 = 84.0;
const TILE_H: f32 = 92.0;
const GRID_ICON: f32 = 36.0;
const FIELD_H: f32 = 40.0;
const WIDTH: f32 = PAD * 2.0 + COLS as f32 * TILE_W;

// ---------- state ----------

#[derive(PartialEq, Clone, Copy)]
enum Mode {
    Grid,
    Add,
    /// naming a new category, or renaming one
    Name,
}

/// What is under the mouse.
#[derive(PartialEq, Clone, Copy, Debug)]
enum Hit {
    /// an app, by its index in the saved list
    Tile(usize),
    AddTile,
    Heading(usize),
    Field,
    Row(usize),
    Scrollbar,
    Back,
    Browse,
}

/// App icons as drawable bitmaps, per source and pixel size, loaded by the worker.
#[derive(Default)]
struct Icons {
    /// source -> [(px, bitmap)]; None = loading failed
    have: HashMap<String, Vec<(u32, Option<ID2D1Bitmap>)>>,
    asked: HashSet<(String, u32)>,
    /// asked again at the front of the queue because they are on screen now
    rushed: HashSet<(String, u32)>,
}

impl Icons {
    /// The best bitmap for `src` at `px`; asks the worker for the exact size if needed.
    fn get(&mut self, src: &str, px: u32, urgent: bool) -> Option<&ID2D1Bitmap> {
        let exact = self.have.get(src).is_some_and(|v| v.iter().any(|(p, _)| *p == px));
        if !exact {
            let key = (src.to_string(), px);
            if urgent && self.rushed.insert(key.clone()) {
                worker::push(worker::Job::Icon(src.to_string(), px), true);
                self.asked.insert(key);
            } else if !urgent && self.asked.insert(key) {
                worker::push(worker::Job::Icon(src.to_string(), px), false);
            }
        }
        let v = self.have.get(src)?;
        v.iter().find(|(p, _)| *p == px).or_else(|| v.iter().max_by_key(|(p, _)| *p)).and_then(|(_, b)| b.as_ref())
    }

    fn arrived(&mut self, g: &Gfx, src: String, px: u32, pic: Option<shell::Pixels>) {
        let b = pic.and_then(|p| g.bitmap(&p));
        let v = self.have.entry(src).or_default();
        v.retain(|(p, _)| *p != px);
        v.push((px, b));
    }
}

struct State {
    hwnd: HWND,
    mode: Mode,
    cfg: Config,
    cfg_path: PathBuf,
    /// every app on the PC (for "+"), once the background scan is done
    installed: Vec<add::Entry>,
    installed_ready: bool,
    /// the list icons have been queued (done the first time "+" opens)
    icons_queued: bool,
    /// list row -> (index into installed, matched chars start..end in UTF-16)
    shown: Vec<(usize, Option<(u32, u32)>)>,
    query: String,
    /// caret position, in bytes into query
    caret: usize,
    caret_on: bool,
    sel: usize,
    scroll: f32,
    scroll_to: f32,
    /// grab offset while dragging the scrollbar
    thumb_drag: Option<f32>,
    /// an app tile being dragged
    tile_drag: Option<grid::TileDrag>,
    /// where each tile is drawn (by App::key); eases toward its slot
    pos: HashMap<String, (f32, f32)>,
    naming: Option<grid::Naming>,
    mouse: Option<(f32, f32)>,
    hover: Option<Hit>,
    pressed: Option<Hit>,
    /// hover glow per tile (by App::key, or grid::ADD_KEY), 0..1
    glow: HashMap<String, f32>,
    shown_at: Instant,
    last_tick: Instant,
    animating: bool,
    /// the next WM_CHAR belongs to a key that already did something
    eat_char: bool,
    high_surrogate: Option<u16>,
    scale: f32,
    fixed_scale: bool,
    dark: bool,
    fixed_theme: bool,
    glass: bool,
    gfx: Gfx,
    icons: Icons,
    /// tile name -> laid-out text (fitted to two lines once)
    names: HashMap<String, IDWriteTextLayout>,
    tray_icon: HICON,
    taskbar_created: u32,
    /// our own menu/dialog is up: don't hide on focus loss
    busy: bool,
    test: Option<selftest::Test>,
}

// Everything runs on the one window thread, so the state is a plain global.
static mut ST: Option<State> = None;
fn st() -> &'static mut State {
    unsafe { ST.as_mut().unwrap() }
}
fn log(msg: &str) {
    if let Some(t) = st().test.as_mut() {
        t.log(msg);
    }
}
/// DIPs to whole device pixels.
fn px(s: &State, v: f32) -> i32 {
    (v * s.scale).round() as i32
}
fn icon_px(s: &State, dip: f32) -> u32 {
    (dip * s.scale).round() as u32
}
/// Ease-out over 0..1.
fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

fn size_dip(s: &State) -> (f32, f32) {
    match s.mode {
        Mode::Grid => (WIDTH, grid::layout(s).h),
        Mode::Add => (WIDTH, add::ADD_H),
        Mode::Name => (WIDTH, grid::NAME_H),
    }
}

fn hit(s: &State, x: f32, y: f32) -> Option<Hit> {
    match s.mode {
        Mode::Grid => grid::hit(s, x, y),
        Mode::Add => add::hit(s, x, y),
        Mode::Name => None,
    }
}

// ---------- placement ----------

/// Opens the box just to the right of the mouse (to the left if there's no
/// room), with its first row of icons level with the mouse, kept on screen.
fn place_at_cursor() {
    let s = st();
    unsafe {
        let mut pt = POINT::default();
        let _ = GetCursorPos(&mut pt);
        let mon = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
        if !s.fixed_scale {
            s.scale = win::monitor_scale(mon);
        }
        let (wd, hd) = size_dip(s);
        let (w, h) = (px(s, wd), px(s, hd));
        let wa = win::work_area(mon);
        let gap = px(s, 24.0);
        let mut x = pt.x + gap;
        if x + w > wa.right {
            x = pt.x - gap - w;
        }
        let x = x.min(wa.right - w).max(wa.left);
        let y = (pt.y - px(s, PAD + TILE_H / 2.0)).min(wa.bottom - h).max(wa.top);
        log(&format!("place: mouse=({},{}) screen={}..{} box x={x}..{} y={y} scale={}", pt.x, pt.y, wa.left, wa.right, x + w, s.scale));
        let _ = SetWindowPos(s.hwnd, HWND_TOPMOST, x, y, w, h, SWP_NOACTIVATE);
    }
}

/// Resize after a change, keeping the box on screen.
fn resize_in_place() {
    let s = st();
    let (wd, hd) = size_dip(s);
    let (w, h) = (px(s, wd), px(s, hd));
    unsafe {
        let mut r = RECT::default();
        let _ = GetWindowRect(s.hwnd, &mut r);
        let wa = win::work_area(MonitorFromWindow(s.hwnd, MONITOR_DEFAULTTONEAREST));
        let x = r.left.min(wa.right - w).max(wa.left);
        let y = r.top.min(wa.bottom - h).max(wa.top);
        let _ = SetWindowPos(s.hwnd, HWND_TOPMOST, x, y, w, h, SWP_NOACTIVATE);
    }
    invalidate();
}

// ---------- show / hide / launch ----------

fn show_popup() {
    let s = st();
    set_mode(Mode::Grid);
    s.hover = None;
    s.pressed = None;
    s.mouse = None;
    s.tile_drag = None;
    s.glow.clear();
    s.pos.clear();
    place_at_cursor();
    // paint the first frame while hidden from view, so it never flashes empty
    win::cloak(s.hwnd, true);
    s.shown_at = Instant::now();
    unsafe {
        let _ = ShowWindow(s.hwnd, SW_SHOW);
        let _ = SetForegroundWindow(s.hwnd);
        let _ = SetFocus(s.hwnd);
        let _ = RedrawWindow(s.hwnd, None, None, RDW_INVALIDATE | RDW_UPDATENOW);
    }
    win::cloak(s.hwnd, false);
    kick();
}

fn hide_popup() {
    let s = st();
    unsafe {
        // a drag in progress ends here, not when the box next opens
        if GetCapture() == s.hwnd {
            let _ = ReleaseCapture();
        }
        let _ = ShowWindow(s.hwnd, SW_HIDE);
    }
    set_mode(Mode::Grid);
}

fn visible() -> bool {
    unsafe { IsWindowVisible(st().hwnd).as_bool() }
}

/// Hides at once, then opens the app on a helper thread (some apps take a
/// moment to start, and the box shouldn't wait for them).
fn launch(i: usize) {
    let s = st();
    let Some(app) = s.cfg.apps.get(i).cloned() else { return };
    log(&format!("launch '{}'", app.name));
    let hwnd = s.hwnd.0 as isize;
    unsafe {
        // we are in front right now: let the app we start come to the front too
        let _ = AllowSetForegroundWindow(ASFW_ANY);
    }
    hide_popup();
    std::thread::spawn(move || {
        shell::com_init();
        let ok = shell::launch(&app);
        unsafe {
            let _ = PostMessageW(HWND(hwnd as _), WM_LAUNCHED, WPARAM(ok as usize), LPARAM(0));
        }
    });
}

// ---------- changing the list ----------

fn save() {
    let s = st();
    s.cfg.save(&s.cfg_path);
}

/// Adds the apps not already in the list. Returns how many were new.
fn add_apps(apps: Vec<App>) -> usize {
    let s = st();
    let mut n = 0;
    for a in apps {
        if !s.cfg.has(&a.path) {
            s.cfg.apps.push(a);
            n += 1;
        }
    }
    if n > 0 {
        save();
    }
    log(&format!("added {n}, now {}", s.cfg.apps.len()));
    n
}

fn remove_app(i: usize) {
    let s = st();
    log(&format!("remove '{}'", s.cfg.apps[i].name));
    s.cfg.apps.remove(i);
    s.cfg.tidy();
    save();
    s.hover = None;
    s.pressed = None;
    resize_in_place();
    kick(); // the tiles after it slide up
}

fn set_mode(m: Mode) {
    let s = st();
    if s.mode == m {
        return;
    }
    s.mode = m;
    s.hover = None;
    s.pressed = None;
    s.thumb_drag = None;
    s.tile_drag = None;
    s.shown_at = Instant::now();
    match m {
        Mode::Add => {
            s.query.clear();
            s.caret = 0;
            add::refill();
            add::queue_icons();
            field::caret_reset();
            log(&format!("+ opened, app list ready={}", s.installed_ready));
        }
        Mode::Name => field::caret_reset(),
        Mode::Grid => unsafe {
            let _ = KillTimer(s.hwnd, TIMER_CARET);
        },
    }
    resize_in_place();
    kick();
}

// ---------- animation ----------

fn invalidate() {
    unsafe {
        let _ = InvalidateRect(st().hwnd, None, false);
    }
}

/// Starts the frame timer if it isn't running (it stops itself when idle).
fn kick() {
    let s = st();
    if !s.animating {
        s.animating = true;
        s.last_tick = Instant::now();
        unsafe {
            SetTimer(s.hwnd, TIMER_ANIM, 10, None);
        }
    }
}

/// One animation step: the opening fade, hover glows, tiles sliding, scrolling.
fn tick() {
    let s = st();
    let now = Instant::now();
    let dt = now.duration_since(s.last_tick).as_secs_f32().min(0.05);
    s.last_tick = now;
    let mut busy = now.duration_since(s.shown_at).as_millis() < 260;
    let hovered = match s.hover {
        Some(Hit::Tile(i)) => s.cfg.apps.get(i).map(App::key),
        Some(Hit::AddTile) => Some(grid::ADD_KEY.to_string()),
        _ => None,
    };
    if let Some(h) = &hovered {
        s.glow.entry(h.clone()).or_insert(0.0);
    }
    let step = dt / 0.09;
    s.glow.retain(|k, g| {
        let target = if hovered.as_ref() == Some(k) { 1.0 } else { 0.0 };
        *g = if *g < target { (*g + step).min(target) } else { (*g - step).max(target) };
        busy |= *g != target;
        *g > 0.0 || target > 0.0
    });
    busy |= match s.mode {
        Mode::Grid => grid::animate(s, dt),
        Mode::Add => add::animate(s, dt),
        Mode::Name => false,
    };
    invalidate();
    if !busy {
        s.animating = false;
        unsafe {
            let _ = KillTimer(s.hwnd, TIMER_ANIM);
        }
    }
}

// ---------- painting ----------

fn render() {
    let s = st();
    let mut cr = RECT::default();
    unsafe {
        let _ = GetClientRect(s.hwnd, &mut cr);
    }
    let th = theme::get(s.dark);
    s.gfx.begin(cr.right, cr.bottom, s.scale);
    if !s.glass {
        s.gfx.fill(Rect::new(0.0, 0.0, cr.right as f32 / s.scale, cr.bottom as f32 / s.scale), 0.0, th.surface);
    }
    match s.mode {
        Mode::Grid => grid::paint_grid(s, th),
        Mode::Add => add::paint(s, th),
        Mode::Name => grid::paint_name(s, th),
    }
    s.gfx.end();
}

/// A tile name on up to two lines, with "…" if it still doesn't fit (once per name).
fn name_layout<'a>(names: &'a mut HashMap<String, IDWriteTextLayout>, g: &Gfx, name: &str, w: f32) -> Option<&'a IDWriteTextLayout> {
    if !names.contains_key(name) {
        let fits = |t: &str| {
            let l = g.layout(t, &g.f.name, w, 1000.0)?;
            let mut m = DWRITE_TEXT_METRICS::default();
            unsafe { l.GetMetrics(&mut m).ok()? };
            Some((l, m.lineCount <= 2))
        };
        let (mut l, ok) = fits(name)?;
        if !ok {
            // the longest start of the name that fits with "…" (binary search)
            let chars: Vec<char> = name.chars().collect();
            let cut = |n: usize| chars[..n].iter().collect::<String>().trim_end().to_string() + "\u{2026}";
            let (mut lo, mut hi) = (1, chars.len());
            while lo < hi {
                let mid = (lo + hi).div_ceil(2);
                if fits(&cut(mid)).is_some_and(|f| f.1) { lo = mid } else { hi = mid - 1 }
            }
            l = fits(&cut(lo))?.0;
        }
        unsafe {
            let _ = l.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_NEAR);
        }
        names.insert(name.to_string(), l);
    }
    names.get(name)
}

/// No icon: a soft rounded square with the app's first letter.
fn placeholder(g: &Gfx, th: &Theme, r: Rect, name: &str) {
    g.fill(r, r.w * 0.22, th.control_hover);
    g.outline(r, r.w * 0.22, th.stroke);
    let letter: String = name.chars().find(|c| c.is_alphanumeric()).map(|c| c.to_uppercase().collect()).unwrap_or_default();
    g.text(&letter, r, &g.f.strong_c, th.text2);
}

// ---------- input ----------

/// Mouse position in DIPs (negative outside the box while the mouse is captured).
fn lp_dip(s: &State, lp: LPARAM) -> (f32, f32) {
    let x = (lp.0 & 0xffff) as i16 as f32;
    let y = ((lp.0 >> 16) & 0xffff) as i16 as f32;
    (x / s.scale, y / s.scale)
}

/// A click (pressed and released on the same thing).
fn act(h: Hit) {
    let s = st();
    match h {
        Hit::Tile(i) if i < s.cfg.apps.len() => launch(i),
        Hit::AddTile => set_mode(Mode::Add),
        Hit::Row(r) => add::add_row(r),
        Hit::Back => set_mode(Mode::Grid),
        Hit::Browse => add::browse(),
        Hit::Tile(_) | Hit::Heading(_) | Hit::Field | Hit::Scrollbar => {}
    }
}

fn on_mouse_move(hwnd: HWND, x: f32, y: f32) {
    let s = st();
    s.mouse = Some((x, y));
    if s.thumb_drag.is_some() {
        add::drag_thumb(y);
    }
    if grid::mouse_move(x, y) {
        return;
    }
    let h = hit(s, x, y);
    if h != s.hover {
        s.hover = h;
        kick();
        invalidate();
    }
    // (the hidden test desktop has no real mouse, so it would "leave" at once)
    if s.test.is_none() {
        let mut tme = TRACKMOUSEEVENT { cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32, dwFlags: TME_LEAVE, hwndTrack: hwnd, dwHoverTime: 0 };
        unsafe {
            let _ = TrackMouseEvent(&mut tme);
        }
    }
}

fn on_left_down(hwnd: HWND, x: f32, y: f32) {
    let s = st();
    s.pressed = hit(s, x, y);
    if s.pressed == Some(Hit::Scrollbar) {
        add::grab_thumb(y);
    }
    if s.mode == Mode::Grid {
        grid::mouse_down(x, y);
    }
    unsafe {
        SetCapture(hwnd);
    }
    invalidate();
}

fn on_left_up(x: f32, y: f32) {
    let s = st();
    let was = s.pressed.take();
    s.thumb_drag = None;
    let dragged = grid::mouse_up();
    unsafe {
        let _ = ReleaseCapture();
    }
    invalidate();
    if !dragged {
        if let Some(h) = hit(s, x, y).filter(|&h| Some(h) == was) {
            act(h);
        }
    }
}

fn on_key(vk: VIRTUAL_KEY) {
    let s = st();
    match s.mode {
        Mode::Add => {
            add::on_key(vk);
        }
        Mode::Name => grid::on_name_key(vk),
        // Esc while dragging a tile cancels the drag (via WM_CAPTURECHANGED)
        Mode::Grid if s.tile_drag.as_ref().is_some_and(|d| d.started) => {
            if vk == VK_ESCAPE {
                unsafe {
                    let _ = ReleaseCapture();
                }
            }
        }
        Mode::Grid => {
            // 1..9 and 0 open the apps in the order shown
            let digit = match vk.0 {
                k @ 0x31..=0x39 => Some(k - 0x31),
                k if (VK_NUMPAD1.0..=VK_NUMPAD9.0).contains(&k) => Some(k - VK_NUMPAD1.0),
                k if k == 0x30 || k == VK_NUMPAD0.0 => Some(9),
                _ => None,
            };
            if let Some(d) = digit {
                if let Some(&i) = grid::order(s).get(d as usize) {
                    launch(i);
                }
            } else if vk == VK_ESCAPE {
                hide_popup();
            } else if vk == VK_ADD || vk == VK_OEM_PLUS {
                s.eat_char = true; // its "+" / "=" mustn't land in the search box
                set_mode(Mode::Add);
            }
        }
    }
}

/// The worker finished some icons and/or the installed-app scan.
fn on_loaded() {
    let s = st();
    let mut repaint = false;
    for d in worker::take() {
        match d {
            worker::Done::Icon(src, px, pic) => s.icons.arrived(&s.gfx, src, px, pic),
            worker::Done::Installed(apps) => {
                s.installed = apps.into_iter().map(add::Entry::new).collect();
                s.installed_ready = true;
                log(&format!("app list arrived: {} apps", s.installed.len()));
                if s.mode == Mode::Add {
                    add::refill();
                    add::queue_icons();
                }
            }
        }
        repaint = true;
    }
    if repaint && visible() {
        invalidate();
    }
}

extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe {
        if ST.is_none() {
            return DefWindowProcW(hwnd, msg, wp, lp);
        }
        let s = st();
        match msg {
            // the whole window is client area (no frame drawn), and no resize edges
            WM_NCCALCSIZE if wp.0 != 0 => return LRESULT(0),
            WM_NCHITTEST => return LRESULT(HTCLIENT as isize),
            // Focus changes make Windows draw its old window frame over the box
            // (a light rectangle round the edge, easy to see when the hotkey is
            // pressed fast). lParam -1 keeps the focus change but skips that
            // drawing; the two undocumented themed-frame messages are skipped too.
            WM_NCACTIVATE => {
                invalidate();
                return DefWindowProcW(hwnd, msg, wp, LPARAM(-1));
            }
            WM_NCUAHDRAWCAPTION | WM_NCUAHDRAWFRAME => return LRESULT(0),
            WM_PAINT => {
                let mut ps = PAINTSTRUCT::default();
                let hdc = BeginPaint(hwnd, &mut ps);
                render();
                s.gfx.blit(hdc);
                let _ = EndPaint(hwnd, &ps);
            }
            WM_ERASEBKGND => return LRESULT(1),
            WM_MOUSEMOVE => {
                let (x, y) = lp_dip(s, lp);
                on_mouse_move(hwnd, x, y);
            }
            WM_MOUSELEAVE => {
                s.mouse = None;
                s.hover = None;
                kick();
                invalidate();
            }
            WM_LBUTTONDOWN => {
                let (x, y) = lp_dip(s, lp);
                on_left_down(hwnd, x, y);
            }
            WM_LBUTTONUP => {
                let (x, y) = lp_dip(s, lp);
                on_left_up(x, y);
            }
            WM_CAPTURECHANGED => {
                s.pressed = None;
                s.thumb_drag = None;
                grid::cancel_drag();
            }
            WM_MOUSEWHEEL if s.mode == Mode::Add => add::wheel((wp.0 >> 16) as i16 as f32 / 120.0),
            WM_RBUTTONUP if s.mode == Mode::Grid => {
                let (x, y) = lp_dip(s, lp);
                match hit(s, x, y) {
                    Some(Hit::Tile(i)) if i < s.cfg.apps.len() => grid::tile_menu(i),
                    Some(Hit::Heading(c)) => grid::head_menu(c),
                    _ => {}
                }
            }
            WM_KEYDOWN => on_key(VIRTUAL_KEY(wp.0 as u16)),
            WM_CHAR => field::on_char(wp.0 as u16),
            WM_ACTIVATE => {
                if (wp.0 & 0xffff) as u32 == WA_INACTIVE && !s.busy && s.test.is_none() {
                    hide_popup();
                }
            }
            WM_HOTKEY => {
                if s.test.is_none() {
                    win::swallow_win_key();
                }
                if visible() && GetForegroundWindow() == hwnd {
                    hide_popup();
                } else {
                    show_popup();
                }
            }
            WM_TRAY => match lp.0 as u32 {
                WM_LBUTTONUP => show_popup(),
                WM_RBUTTONUP => match win::popup_menu(&[(1, "Open  (Win+`)"), (0, ""), (2, "Quit Onion Launcher")]) {
                    1 => show_popup(),
                    2 => {
                        let _ = DestroyWindow(hwnd);
                    }
                    _ => {}
                },
                _ => {}
            },
            WM_DROPFILES => {
                let files = win::dropped_files(HDROP(wp.0 as _));
                add_apps(files.iter().map(|f| App::from_path(f)).collect());
                set_mode(Mode::Grid);
                resize_in_place();
            }
            WM_TIMER => match wp.0 {
                TIMER_ANIM => tick(),
                TIMER_CARET => {
                    s.caret_on = !s.caret_on || s.test.is_some(); // steady in test pictures
                    invalidate();
                }
                TIMER_TEST => selftest::step(),
                _ => {}
            },
            WM_LOADED => on_loaded(),
            WM_LAUNCHED => log(&format!("launch ok={}", wp.0 != 0)),
            WM_DPICHANGED if !s.fixed_scale => {
                s.scale = (wp.0 & 0xffff) as f32 / 96.0;
                resize_in_place();
            }
            WM_SETTINGCHANGE if !s.fixed_theme => {
                let dark = shell::system_dark();
                if dark != s.dark {
                    s.dark = dark;
                    win::apply_theme(hwnd, dark);
                    invalidate();
                }
            }
            WM_CLOSE => hide_popup(),
            WM_QUIT_APP => {
                let _ = DestroyWindow(hwnd);
            }
            WM_DESTROY => {
                win::remove_tray(hwnd);
                PostQuitMessage(0);
            }
            // Explorer restarted: put the tray icon back
            m if m == s.taskbar_created && m != 0 => win::add_tray(hwnd, s.tray_icon),
            _ => return DefWindowProcW(hwnd, msg, wp, lp),
        }
        LRESULT(0)
    }
}

// ---------- start-up ----------

/// The window: a thick frame gets Windows 11's shadow, rounded corners and
/// backdrop; WM_NCCALCSIZE above hides the frame itself. Returns it and the
/// tray-size icon.
unsafe fn create_window() -> (HWND, HICON) {
    let module = GetModuleHandleW(None).unwrap();
    // the app icon from the exe's resources, at the tray's size and full size
    let cx = GetSystemMetrics(SM_CXSMICON);
    // MAKEINTRESOURCE(1): the icon group build.rs puts in as resource 1
    let icon_id = PCWSTR(std::ptr::without_provenance(1));
    let icon = |w: i32, flags| LoadImageW(module, icon_id, IMAGE_ICON, w, w, flags).map(|h| HICON(h.0)).unwrap_or_default();
    let tray_icon = icon(cx, LR_DEFAULTCOLOR);
    RegisterClassW(&WNDCLASSW {
        lpfnWndProc: Some(wndproc),
        hInstance: module.into(),
        lpszClassName: CLASS,
        hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
        hIcon: icon(0, LR_DEFAULTSIZE),
        ..Default::default()
    });
    let hwnd = CreateWindowExW(
        WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_ACCEPTFILES,
        w!("OnionLauncher"),
        w!("Onion Launcher"),
        WS_POPUP | WS_THICKFRAME,
        0,
        0,
        100,
        100,
        HWND::default(),
        HMENU::default(),
        module,
        None,
    )
    .expect("create window");
    let corner = DWMWCP_ROUND;
    let _ = DwmSetWindowAttribute(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, &corner as *const _ as _, 4);
    (hwnd, tray_icon)
}

/// Windows 11's blurred "acrylic" backdrop behind the see-through parts. False if unsupported.
unsafe fn enable_glass(hwnd: HWND) -> bool {
    let backdrop = DWMSBT_TRANSIENTWINDOW;
    DwmSetWindowAttribute(hwnd, DWMWA_SYSTEMBACKDROP_TYPE, &backdrop as *const _ as _, 4).is_ok()
        && DwmExtendFrameIntoClientArea(hwnd, &MARGINS { cxLeftWidth: -1, cxRightWidth: -1, cyTopHeight: -1, cyBottomHeight: -1 }).is_ok()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    shell::com_init();
    // Started with no arguments from anywhere but its installed place (Downloads, say):
    // this exe is its own installer, like Onion Peel's.
    match args.get(1).map(String::as_str) {
        Some("--uninstall") => {
            if dialog::run(dialog::Page::uninstall()) == Some(dialog::Kind::Uninstalled) {
                install::remove_leftovers();
            }
            return;
        }
        Some("--quit") => return install::quit_running(),
        None if !install::is_installed_copy() => {
            dialog::run(dialog::Page::install());
            return;
        }
        _ => {}
    }
    if config::arg_value(&args, "--tag").as_deref() == Some("setup") {
        if let Some(dir) = config::arg_value(&args, "--selftest") {
            return selftest::setup_pictures(std::path::Path::new(&dir));
        }
    }
    let test = selftest::Test::from_args(&args);
    // one copy at a time (a second start just quits); the handle stays open until exit
    if test.is_none() {
        let m = unsafe { CreateMutexW(None, true, w!("Local\\OnionLauncher.single")) };
        if m.is_ok() && unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            return;
        }
    }
    let forced_theme = config::arg_value(&args, "--theme");
    let dark = match forced_theme.as_deref() {
        Some("dark") => true,
        Some("light") => false,
        _ => shell::system_dark(),
    };
    let forced_scale = config::arg_value(&args, "--scale").and_then(|v| v.parse::<f32>().ok());
    let cfg_path = config::config_path(&args);
    let (cfg, broken) = Config::load(&cfg_path);
    let no_save = cfg.no_save;
    let testing = test.is_some();

    unsafe {
        let (hwnd, tray_icon) = create_window();
        win::apply_theme(hwnd, dark);
        // the self-test draws a solid background instead, so its pictures are stable
        let glass = !testing && enable_glass(hwnd);
        let scale = forced_scale.unwrap_or(GetDpiForSystem() as f32 / 96.0);
        ST = Some(State {
            hwnd,
            mode: Mode::Grid,
            cfg,
            cfg_path,
            installed: vec![],
            installed_ready: false,
            icons_queued: false,
            shown: vec![],
            query: String::new(),
            caret: 0,
            caret_on: true,
            sel: 0,
            scroll: 0.0,
            scroll_to: 0.0,
            thumb_drag: None,
            tile_drag: None,
            pos: HashMap::new(),
            naming: None,
            mouse: None,
            hover: None,
            pressed: None,
            glow: HashMap::new(),
            shown_at: Instant::now(),
            last_tick: Instant::now(),
            animating: false,
            eat_char: false,
            high_surrogate: None,
            scale,
            fixed_scale: forced_scale.is_some(),
            dark,
            fixed_theme: forced_theme.is_some(),
            glass,
            gfx: Gfx::new().expect("Direct2D"),
            icons: Icons::default(),
            names: HashMap::new(),
            tray_icon,
            taskbar_created: RegisterWindowMessageW(w!("TaskbarCreated")),
            busy: false,
            test,
        });
        log(&format!("glass={glass} scale={scale}"));
        if let Some(b) = &broken {
            log(&format!("app list unreadable, kept as {}", b.display()));
        }
        resize_in_place();

        // icons for the saved apps first, then the list of everything installed
        worker::start(hwnd, WM_LOADED);
        let s = st();
        let gpx = icon_px(s, GRID_ICON);
        for a in &s.cfg.apps {
            s.icons.get(a.icon_source(), gpx, false);
        }
        worker::push(worker::Job::Installed, false);

        if testing {
            SetTimer(hwnd, TIMER_TEST, 500, None);
        } else {
            win::add_tray(hwnd, tray_icon);
            // the key left of 1 on this keyboard (` on US layouts, ^ on German, ' on UK...)
            let key = match MapVirtualKeyW(0x29, MAPVK_VSC_TO_VK) {
                0 => VK_OEM_3.0 as u32,
                vk => vk,
            };
            if RegisterHotKey(hwnd, HOTKEY_ID, MOD_WIN | MOD_NOREPEAT, key).is_err() {
                MessageBoxW(hwnd, w!("Win + the key left of 1 is already used by another app. The tray icon still works."), w!("Onion Launcher"), MB_ICONWARNING);
            }
            if no_save {
                let text = format!(
                    "Your list of apps couldn't be read (another program may have it open), so Onion Launcher started with an empty one.

Your list was left as it is, and nothing will be saved until Onion Launcher is restarted:
{}",
                    st().cfg_path.display()
                );
                MessageBoxW(hwnd, &HSTRING::from(text), w!("Onion Launcher"), MB_ICONWARNING);
            }
            if let Some(b) = &broken {
                let text = format!(
                    "Your list of apps couldn't be read, so Onion Launcher started with an empty one.

The old list was kept here:
{}",
                    b.display()
                );
                MessageBoxW(hwnd, &HSTRING::from(text), w!("Onion Launcher"), MB_ICONWARNING);
            }
        }

        let mut m = MSG::default();
        while GetMessageW(&mut m, HWND::default(), 0, 0).as_bool() {
            let _ = TranslateMessage(&m);
            DispatchMessageW(&m);
        }
    }
}
