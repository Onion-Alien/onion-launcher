//! `--selftest <dir> --tag <name>`: drives the real message paths (the hotkey
//! message, mouse moves and clicks, the wheel, typed characters, Enter/Esc)
//! one step every half second, saves a picture of each state and logs what
//! happened. Run it on a hidden desktop (tools/run_hidden.ps1): no tray icon,
//! no hotkey, and nothing reaches the real screen.

use crate::*;
use std::io::Write;

pub struct Test {
    dir: PathBuf,
    tag: String,
    step: u32,
    log: std::fs::File,
}

impl Test {
    pub fn from_args(args: &[String]) -> Option<Test> {
        let dir = PathBuf::from(config::arg_value(args, "--selftest")?);
        let tag = config::arg_value(args, "--tag").unwrap_or_else(|| "run".into());
        let log = std::fs::File::create(dir.join(format!("q-{tag}-log.txt"))).ok()?;
        Some(Test { dir, tag, step: 0, log })
    }
    pub fn log(&mut self, s: &str) {
        let _ = writeln!(self.log, "{s}");
    }
}

fn snap(name: &str) {
    render();
    let s = st();
    let t = s.test.as_ref().unwrap();
    let out = t.dir.join(format!("q-{}-{name}.png", t.tag));
    let ok = save_png(&out, &s.gfx.rgba(), s.gfx.w as u32, s.gfx.h as u32);
    log(&format!("snap {name} ok={ok}"));
}

fn save_png(out: &std::path::Path, rgba: &[u8], w: u32, h: u32) -> bool {
    let Ok(f) = std::fs::File::create(out) else { return false };
    let mut enc = png::Encoder::new(std::io::BufWriter::new(f), w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().and_then(|mut wr| wr.write_image_data(rgba)).is_ok()
}

fn lp_at(x: f32, y: f32) -> LPARAM {
    let s = st();
    let (x, y) = ((x * s.scale) as isize, (y * s.scale) as isize);
    LPARAM((y << 16) | (x & 0xffff))
}
fn center(r: Rect) -> (f32, f32) {
    (r.x + r.w / 2.0, r.y + r.h / 2.0)
}
/// The k-th app as shown (the "+" tile if there is none).
fn app_at(k: usize) -> (f32, f32) {
    let s = st();
    let item = grid::order(s).get(k).map(|&i| grid::Item::App(i)).unwrap_or(grid::Item::Add);
    center(grid::item_rect(s, item).unwrap_or_default())
}
fn add_at() -> (f32, f32) {
    center(grid::item_rect(st(), grid::Item::Add).unwrap_or_default())
}
fn lp((x, y): (f32, f32)) -> LPARAM {
    lp_at(x, y)
}
fn app_index(name: &str) -> usize {
    st().cfg.apps.iter().position(|a| a.name == name).unwrap_or(0)
}
fn cat_index(name: &str) -> usize {
    st().cfg.categories.iter().position(|c| c == name).unwrap_or(0)
}
/// "Notepad, Marker | Tools: Paint, ..." in the order shown.
fn describe() -> String {
    let s = st();
    let mut out = String::new();
    let mut cur = String::new();
    for i in grid::order(s) {
        let a = &s.cfg.apps[i];
        if a.category != cur {
            cur = a.category.clone();
            out += &format!(" | {cur}:");
        }
        out += &format!(" {},", a.name);
    }
    out
}
fn row_lp(row: usize) -> LPARAM {
    lp(add::row_center(st(), row))
}
fn post(m: u32, w: usize, l: LPARAM) {
    unsafe {
        let _ = PostMessageW(st().hwnd, m, WPARAM(w), l);
    }
}
fn click(l: LPARAM) {
    post(WM_LBUTTONDOWN, 1, l);
    post(WM_LBUTTONUP, 0, l);
}
fn type_str(t: &str) {
    for u in t.encode_utf16() {
        post(WM_CHAR, u as usize, LPARAM(0));
    }
}
fn quit() {
    unsafe {
        let _ = DestroyWindow(st().hwnd);
    }
}

pub fn step() {
    let s = st();
    let t = s.test.as_mut().unwrap();
    if t.step == 8 && !s.installed_ready {
        return; // wait for the background app scan before scrolling/searching
    }
    let step = t.step;
    t.step += 1;
    if t.tag == "cats" {
        return step_cats(step);
    }
    if t.tag == "frame" {
        return step_frame(step);
    }
    let short = t.tag != "full";
    let n = s.cfg.apps.len();
    match step {
        0 => post(WM_HOTKEY, HOTKEY_ID as usize, LPARAM(0)),
        1 => {
            log(&format!("after hotkey visible={}", visible()));
            snap("grid");
        }
        2 => post(WM_MOUSEMOVE, 0, lp(app_at(if n == 0 { 0 } else { 1 }))),
        3 => snap("hover"),
        4 => post(WM_LBUTTONDOWN, 1, lp(app_at(0))),
        5 => {
            snap("pressed");
            post(WM_LBUTTONUP, 0, lp_at(1.0, 1.0)); // let go outside any tile: nothing opens
        }
        6 => {
            log(&format!("released outside: visible={} add={}", visible(), s.mode == Mode::Add));
            click(lp(add_at()));
        }
        7 => {
            log(&format!("after + click mode_add={}", s.mode == Mode::Add));
            snap("add");
        }
        8 => {
            post(WM_MOUSEWHEEL, ((-360i32 as u32 as usize) & 0xffff) << 16, LPARAM(0));
            post(WM_MOUSEMOVE, 0, row_lp(3));
        }
        9 => {
            log(&format!("after wheel scroll={:.0} hover={:?}", s.scroll, s.hover));
            snap("add-scrolled");
        }
        10 => type_str(if s.dark { "a" } else { "snip" }),
        11 => {
            log(&format!("typed '{}' shown={}", s.query, s.shown.len()));
            snap("add-search");
            if short {
                quit();
            }
        }
        12 => post(WM_KEYDOWN, VK_RETURN.0 as usize, LPARAM(0)),
        13 => {
            log(&format!("after Enter mode_grid={} apps={}", s.mode == Mode::Grid, s.cfg.apps.len()));
            snap("grid-after-add");
        }
        14 => post(WM_KEYDOWN, b'2' as usize, LPARAM(0)), // Marker
        15 => log(&format!("after key 2 visible={}", visible())),
        16 => post(WM_HOTKEY, HOTKEY_ID as usize, LPARAM(0)),
        17 => {
            log(&format!("after 2nd hotkey visible={}", visible()));
            post(WM_KEYDOWN, VK_OEM_PLUS.0 as usize, LPARAM(0)); // the message loop adds its "=" character
        }
        18 => {
            log(&format!("+ key: add={} query='{}'", s.mode == Mode::Add, s.query));
            type_str("zzzz");
        }
        19 => snap("add-nomatch"),
        20 => type_str(" and a really long search that runs past the edge"),
        21 => {
            post(WM_KEYDOWN, VK_LEFT.0 as usize, LPARAM(0));
            post(WM_CHAR, 0x7F, LPARAM(0)); // Ctrl+Backspace
        }
        22 => {
            log(&format!("long query='{}' caret={}", s.query, s.caret));
            snap("add-longquery");
            post(WM_KEYDOWN, VK_ESCAPE.0 as usize, LPARAM(0));
        }
        23 => {
            log(&format!("Esc#1 query='{}' add={}", s.query, s.mode == Mode::Add));
            post(WM_KEYDOWN, VK_ESCAPE.0 as usize, LPARAM(0));
        }
        24 => {
            log(&format!("Esc#2 grid={}", s.mode == Mode::Grid));
            post(WM_KEYDOWN, VK_ESCAPE.0 as usize, LPARAM(0));
        }
        25 => {
            log(&format!("Esc#3 visible={}", visible()));
            quit();
        }
        _ => {}
    }
}

/// What is really on the window right now (read back from its own DC, so
/// anything Windows drew over our picture shows up), as RGBA.
fn window_pixels() -> (Vec<u8>, u32, u32) {
    let s = st();
    unsafe {
        let mut r = RECT::default();
        let _ = GetWindowRect(s.hwnd, &mut r);
        let (w, h) = (r.right - r.left, r.bottom - r.top);
        let wdc = GetWindowDC(s.hwnd);
        let dc = CreateCompatibleDC(wdc);
        let info = shell::bmi(w, h);
        let mut bits = std::ptr::null_mut();
        let bmp = CreateDIBSection(dc, &info, DIB_RGB_COLORS, &mut bits, None, 0).unwrap_or_default();
        let old = SelectObject(dc, bmp);
        let _ = BitBlt(dc, 0, 0, w, h, wdc, 0, 0, SRCCOPY);
        let _ = GdiFlush();
        let mut px = std::slice::from_raw_parts(bits as *const u8, (w * h * 4) as usize).to_vec();
        SelectObject(dc, old);
        let _ = DeleteObject(bmp);
        let _ = DeleteDC(dc);
        ReleaseDC(s.hwnd, wdc);
        for p in px.chunks_exact_mut(4) {
            p.swap(0, 2);
            p[3] = 255;
        }
        (px, w as u32, h as u32)
    }
}

/// Saves what is on the window and logs how many pixels differ from our own picture.
fn snap_window(name: &str) {
    render();
    let ours = st().gfx.rgba();
    let (px, w, h) = window_pixels();
    let differ = px.chunks_exact(4).zip(ours.chunks_exact(4)).filter(|(a, b)| (0..3).any(|c| a[c].abs_diff(b[c]) > 24)).count();
    let t = st().test.as_ref().unwrap();
    let out = t.dir.join(format!("q-{}-{name}.png", t.tag));
    save_png(&out, &px, w, h);
    log(&format!("window {name}: {w}x{h}, {differ} px differ from our picture"));
}

/// Pressing the hotkey fast flips the box's focus on and off: Windows must
/// not draw its old window frame over the box when that happens.
fn step_frame(step: u32) {
    let s = st();
    match step {
        0 => post(WM_HOTKEY, HOTKEY_ID as usize, LPARAM(0)),
        1 => snap_window("shown"),
        2 => {
            for k in 0..7 {
                unsafe { SendMessageW(s.hwnd, WM_NCACTIVATE, WPARAM(k % 2), LPARAM(0)) };
            }
            // before any repaint could cover it up
            snap_window("right-after-flips");
        }
        3 => snap_window("after-focus-flips"),
        _ => quit(),
    }
}

/// Drags a tile from `from` to `to` in small mouse moves, as a hand would.
fn drag_path(from: (f32, f32), to: (f32, f32)) {
    post(WM_LBUTTONDOWN, 1, lp(from));
    for k in 1..=8 {
        let f = k as f32 / 8.0;
        post(WM_MOUSEMOVE, 1, lp((from.0 + (to.0 - from.0) * f, from.1 + (to.1 - from.1) * f)));
    }
}

/// Just past the right edge of the box, level with `p`.
fn out_right(p: (f32, f32)) -> (f32, f32) {
    (WIDTH + 30.0, p.1)
}
fn armed() -> bool {
    st().tile_drag.as_ref().is_some_and(|d| d.armed())
}

/// Categories and dragging. The right-click menu itself can't be clicked on
/// the hidden desktop, so these steps call what its items call.
fn step_cats(step: u32) {
    let s = st();
    match step {
        0 => post(WM_HOTKEY, HOTKEY_ID as usize, LPARAM(0)),
        1 => {
            log(&format!("start:{}", describe()));
            snap("grid");
        }
        // Notepad (top section) dragged onto the 2nd tile of "Tools", held there
        2 => {
            let to = center(grid::item_rect(s, grid::Item::App(app_index("Task Manager"))).unwrap());
            drag_path(app_at(0), to);
        }
        3 => snap("dragging"),
        4 => post(WM_LBUTTONUP, 0, lp(s.mouse.unwrap_or_default())),
        5 => {
            log(&format!("after drop:{}", describe()));
            snap("dropped");
        }
        // reorder inside the top section: 1st tile to the 3rd spot
        6 => {
            let to = app_at(2);
            drag_path(app_at(0), (to.0 + 10.0, to.1));
            post(WM_LBUTTONUP, 0, lp((to.0 + 10.0, to.1)));
        }
        7 => log(&format!("after reorder:{}", describe())),
        // a drag let go outside the box puts the tile back
        8 => {
            let from = app_at(0);
            drag_path(from, (from.0, -40.0));
            post(WM_LBUTTONUP, 0, lp((from.0, -40.0)));
        }
        9 => log(&format!("after drop outside:{}", describe())),
        // Move to category > New category... for Calculator
        10 => grid::start_naming(grid::Naming::New(app_index("Calculator"))),
        11 => type_str("Media"),
        12 => snap("naming"),
        13 => post(WM_KEYDOWN, VK_RETURN.0 as usize, LPARAM(0)),
        14 => {
            log(&format!("after new category:{}", describe()));
            snap("new-category");
        }
        // heading menu > Rename: "Games" -> "Play"
        15 => {
            grid::start_naming(grid::Naming::Rename(cat_index("Games")));
            post(WM_CHAR, 0x7F, LPARAM(0)); // Ctrl+Backspace clears the old name
            type_str("Play");
        }
        16 => snap("rename"),
        17 => post(WM_KEYDOWN, VK_RETURN.0 as usize, LPARAM(0)),
        // the only app in "Media" dragged back to the top: "Media" goes away
        18 => {
            log(&format!("after rename:{} cats={:?}", describe(), s.cfg.categories));
            let from = center(grid::item_rect(s, grid::Item::App(app_index("Calculator"))).unwrap());
            drag_path(from, app_at(0));
        }
        19 => snap("dragging-out-of-last"),
        20 => post(WM_LBUTTONUP, 0, lp(s.mouse.unwrap_or_default())),
        21 => {
            log(&format!("after emptying Media:{} cats={:?}", describe(), s.cfg.categories));
            grid::delete_category(cat_index("Tools"));
        }
        22 => {
            log(&format!("after deleting Tools:{} cats={:?}", describe(), s.cfg.categories));
            snap("after-delete");
            log(&format!("apps={}", s.cfg.apps.len()));
        }
        // held outside the box: nothing for 2 s, then "Release to remove"
        23 => {
            // the longest name, to see it cut off neatly
            let from = center(grid::item_rect(s, grid::Item::App(app_index("A Really Long Application Name That Keeps Going"))).unwrap());
            drag_path(from, out_right(from));
        }
        24 => {
            log(&format!("held out 0.5 s: armed={}", armed()));
            snap("hold-early");
        }
        28 => {
            log(&format!("held out 2.5 s: armed={}", armed()));
            snap("release-to-remove");
            post(WM_MOUSEMOVE, 1, lp(app_at(1))); // back inside: cancelled
        }
        29 => {
            log(&format!("back inside: armed={}", armed()));
            snap("back-inside");
            post(WM_MOUSEMOVE, 1, lp(out_right(app_at(1)))); // out again: the count restarts
        }
        30 => {
            log(&format!("out again 0.5 s: armed={}", armed()));
            post(WM_LBUTTONUP, 0, lp(s.mouse.unwrap_or_default()));
        }
        31 => {
            log(&format!("let go early:{} apps={}", describe(), s.cfg.apps.len()));
            let from = app_at(0);
            drag_path(from, out_right(from));
        }
        36 => {
            log(&format!("held out 2.5 s: armed={}", armed()));
            post(WM_LBUTTONUP, 0, lp(s.mouse.unwrap_or_default()));
        }
        37 => {
            log(&format!("let go armed:{} apps={}", describe(), s.cfg.apps.len()));
            snap("after-remove");
            // right-click > Remove from Onion Launcher: the rest slide up on their own
            remove_app(app_index("Explorer"));
            log(&format!("menu remove: tiles sliding={}", s.animating));
        }
        38 => {
            log(&format!("after menu remove:{} animating={}", describe(), s.animating));
            snap("after-menu-remove");
            // the box hidden mid-drag lets go of the mouse
            let from = app_at(0);
            drag_path(from, (from.0 + 40.0, from.1));
        }
        39 => {
            log(&format!("mid-drag: captured={}", unsafe { GetCapture() } == s.hwnd));
            hide_popup();
        }
        40 => {
            log(&format!("hidden mid-drag: visible={} captured={} dragging={}", visible(), unsafe { GetCapture() } == s.hwnd, s.tile_drag.is_some()));
            quit();
        }
        _ => {}
    }
}
