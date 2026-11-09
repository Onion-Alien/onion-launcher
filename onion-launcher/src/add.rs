//! The "+" screen: a search box over every app on the PC, a scrolling list
//! (click or Enter adds one), and Back / Browse files at the bottom.

use crate::*;

/// Height of the whole screen, in DIPs.
pub const ADD_H: f32 = 464.0;
const ROW_H: f32 = 40.0;
const FOOT_H: f32 = 52.0;
const LIST_ICON: f32 = 24.0;
/// space on the right of the list for the scrollbar, so it never touches a row
const GUTTER: f32 = 14.0;

/// An installed app with its name lower-cased once (searching is per key press).
pub struct Entry {
    pub app: App,
    lower: String,
}

impl Entry {
    pub fn new(app: App) -> Entry {
        Entry { lower: fold(&app.name), app }
    }
}

// ---------- layout ----------

pub fn field_rect() -> Rect {
    Rect::new(PAD, PAD, WIDTH - 2.0 * PAD, FIELD_H)
}
pub fn list_rect() -> Rect {
    let top = PAD + FIELD_H + 12.0;
    Rect::new(PAD, top, WIDTH - 2.0 * PAD, ADD_H - FOOT_H - top - 10.0)
}
fn back_rect() -> Rect {
    Rect::new(PAD, ADD_H - FOOT_H + 10.0, 84.0, 32.0)
}
fn browse_rect() -> Rect {
    Rect::new(WIDTH - PAD - 142.0, ADD_H - FOOT_H + 10.0, 142.0, 32.0)
}
/// The middle of list row `row` as it is scrolled now (for the self-test).
pub fn row_center(s: &State, row: usize) -> (f32, f32) {
    let l = list_rect();
    (l.x + 100.0, l.y + row as f32 * ROW_H + ROW_H / 2.0 - s.scroll_to)
}
fn max_scroll(s: &State) -> f32 {
    (s.shown.len() as f32 * ROW_H - list_rect().h).max(0.0)
}
/// Scrollbar thumb: (top, height), or None when everything fits.
fn thumb(s: &State) -> Option<(f32, f32)> {
    let l = list_rect();
    let max = max_scroll(s);
    if max <= 0.0 {
        return None;
    }
    let h = (l.h * l.h / (s.shown.len() as f32 * ROW_H)).max(28.0);
    Some((l.y + (l.h - h) * (s.scroll / max).clamp(0.0, 1.0), h))
}

pub fn hit(s: &State, x: f32, y: f32) -> Option<Hit> {
    let l = list_rect();
    if field_rect().contains(x, y) {
        Some(Hit::Field)
    } else if back_rect().contains(x, y) {
        Some(Hit::Back)
    } else if browse_rect().contains(x, y) {
        Some(Hit::Browse)
    } else if l.contains(x, y) {
        if thumb(s).is_some() && x >= l.right() - GUTTER {
            return Some(Hit::Scrollbar);
        }
        let row = ((y - l.y + s.scroll) / ROW_H) as usize;
        (row < s.shown.len()).then_some(Hit::Row(row))
    } else {
        None
    }
}

// ---------- searching ----------

/// Lower-cases char by char (same char count), so match positions map back
/// onto the real name for highlighting.
fn fold(s: &str) -> String {
    s.chars().map(|c| c.to_lowercase().next().unwrap_or(c)).collect()
}

/// Search: names starting with the text first, then a word starting with it,
/// then anywhere in the name; alphabetical within each group.
pub fn refill() {
    let s = st();
    let q = fold(s.query.trim());
    let mut hits: Vec<(u8, usize, Option<(u32, u32)>)> = vec![];
    for (i, e) in s.installed.iter().enumerate() {
        if q.is_empty() {
            hits.push((0, i, None));
            continue;
        }
        let Some(first) = e.lower.find(&q) else { continue };
        let word_start = |p: usize| p == 0 || e.lower[..p].ends_with([' ', '-', '_', '.', '(']);
        // highlight the first word-start match if there is one
        let (rank, at) = match e.lower.match_indices(&q).map(|(p, _)| p).find(|&p| word_start(p)) {
            Some(0) => (0, 0),
            Some(p) => (1, p),
            None => (2, first),
        };
        let u16_at = |byte: usize| e.app.name.chars().take(e.lower[..byte].chars().count()).map(char::len_utf16).sum::<usize>() as u32;
        let start = u16_at(at);
        let end = u16_at(at + q.len());
        hits.push((rank, i, Some((start, end - start))));
    }
    hits.sort_by_key(|h| (h.0, h.1));
    s.shown = hits.into_iter().map(|(_, i, m)| (i, m)).collect();
    s.sel = 0;
    s.scroll = 0.0;
    s.scroll_to = 0.0;
    invalidate();
}

/// Highlights a row and scrolls it into view.
fn select(row: usize) {
    let s = st();
    if s.shown.is_empty() {
        return;
    }
    s.sel = row.min(s.shown.len() - 1);
    let l = list_rect();
    let top = s.sel as f32 * ROW_H;
    if top < s.scroll_to {
        s.scroll_to = top;
    } else if top + ROW_H > s.scroll_to + l.h {
        s.scroll_to = top + ROW_H - l.h;
    }
    s.scroll_to = s.scroll_to.clamp(0.0, max_scroll(s));
    kick();
    invalidate();
}

pub fn add_row(row: usize) {
    let s = st();
    let Some(&(i, _)) = s.shown.get(row) else { return };
    add_apps(vec![s.installed[i].app.clone()]);
    set_mode(Mode::Grid);
}

pub fn browse() {
    let s = st();
    s.busy = true;
    let picked = shell::pick_files(s.hwnd);
    s.busy = false;
    let apps: Vec<App> = picked.iter().map(|p| App::from_path(p)).collect();
    if !apps.is_empty() && add_apps(apps) > 0 {
        set_mode(Mode::Grid);
    }
    unsafe {
        let _ = SetForegroundWindow(s.hwnd);
    }
}

/// The first time the screen opens, every icon in the list is queued (in
/// list order, behind the ones on screen) so scrolling never waits.
pub fn queue_icons() {
    let s = st();
    if s.icons_queued || !s.installed_ready {
        return;
    }
    s.icons_queued = true;
    let px = icon_px(s, LIST_ICON);
    for e in &s.installed {
        s.icons.get(e.app.icon_source(), px, false);
    }
}

// ---------- input ----------

/// Keys on this screen. True = used.
pub fn on_key(vk: VIRTUAL_KEY) -> bool {
    let s = st();
    let n = s.shown.len();
    match vk {
        VK_RETURN => add_row(s.sel),
        VK_ESCAPE if s.query.is_empty() => set_mode(Mode::Grid),
        VK_ESCAPE => field::clear(),
        VK_DOWN if n > 0 => select(s.sel + 1),
        VK_UP if n > 0 => select(s.sel.saturating_sub(1)),
        VK_NEXT if n > 0 => select(s.sel + 8),
        VK_PRIOR if n > 0 => select(s.sel.saturating_sub(8)),
        _ => return field::caret_key(vk),
    }
    true
}

pub fn wheel(delta: f32) {
    let s = st();
    s.scroll_to = (s.scroll_to - delta * ROW_H * 3.0).clamp(0.0, max_scroll(s));
    kick();
}

/// Pressing on the scrollbar grabs the thumb where clicked, or jumps it under the mouse.
pub fn grab_thumb(y: f32) {
    let s = st();
    if let Some((ty, th)) = thumb(s) {
        let grab = if y >= ty && y < ty + th { y - ty } else { th / 2.0 };
        s.thumb_drag = Some(grab);
        drag_thumb(y);
    }
}

pub fn drag_thumb(y: f32) {
    let s = st();
    let (Some(grab), Some((_, th))) = (s.thumb_drag, thumb(s)) else { return };
    let l = list_rect();
    let frac = ((y - grab - l.y) / (l.h - th)).clamp(0.0, 1.0);
    s.scroll_to = frac * max_scroll(s);
    s.scroll = s.scroll_to;
    invalidate();
}

/// Smooth scrolling, one frame. True while moving.
pub fn animate(s: &mut State, dt: f32) -> bool {
    let d = s.scroll_to - s.scroll;
    let moving = d.abs() > 0.3;
    s.scroll = if moving { s.scroll + d * (1.0 - (-dt / 0.045).exp()) } else { s.scroll_to };
    // the row under a still mouse changes as the list moves in or scrolls
    if let Some((x, y)) = s.mouse {
        s.hover = hit(s, x, y);
    }
    moving
}

// ---------- painting ----------

pub fn paint(s: &mut State, th: &Theme) {
    let p = ease(s.shown_at.elapsed().as_secs_f32() * 1000.0 / 160.0);
    let lpx = icon_px(s, LIST_ICON);
    let thumb_at = thumb(s);
    let (g, icons) = (&s.gfx, &mut s.icons);
    g.set_alpha(p);
    g.translate(0.0, (1.0 - p) * 4.0);

    field::draw(g, th, field_rect(), &s.query, s.caret, s.caret_on, "Search apps on this PC", Some(G_SEARCH));

    // results: only the rows in view
    let l = list_rect();
    let row_w = if thumb_at.is_some() { l.w - GUTTER } else { l.w };
    g.clip(l);
    let first = (s.scroll / ROW_H) as usize;
    let rows = (l.h / ROW_H) as usize + 2;
    for row in first..(first + rows).min(s.shown.len()) {
        let (i, m) = s.shown[row];
        let e = &s.installed[i];
        let rr = Rect::new(l.x, l.y + row as f32 * ROW_H - s.scroll + 2.0, row_w, ROW_H - 4.0);
        if row == s.sel {
            g.fill(rr, 5.0, th.selected);
            g.fill(Rect::new(rr.x, rr.y + rr.h / 2.0 - 8.0, 3.0, 16.0), 1.5, th.accent);
        } else if s.hover == Some(Hit::Row(row)) {
            g.fill(rr, 5.0, th.hover);
        }
        let ir = Rect::new(g.snap(rr.x + 12.0), g.snap(rr.y + (rr.h - LIST_ICON) / 2.0), g.snap(LIST_ICON), g.snap(LIST_ICON));
        match icons.get(e.app.icon_source(), lpx, true) {
            Some(b) => g.image(b, ir),
            None => placeholder(g, th, ir, &e.app.name),
        }
        let added = s.cfg.has(&e.app.path);
        let nw = rr.w - 48.0 - if added { 74.0 } else { 10.0 };
        if let Some(lay) = g.layout(&e.app.name, &g.f.body, nw, rr.h) {
            if let Some((start, length)) = m {
                unsafe {
                    let _ = lay.SetFontWeight(DWRITE_FONT_WEIGHT_BOLD, DWRITE_TEXT_RANGE { startPosition: start, length });
                }
            }
            g.draw_layout(&lay, rr.x + 48.0, rr.y, th.text);
        }
        if added {
            // icon fonts sit higher than text: nudge the tick down onto the word's middle
            g.text(G_CHECK, Rect::new(rr.right() - 70.0, rr.y + 1.5, 14.0, rr.h), &g.f.icon12, th.text3);
            g.text("Added", Rect::new(rr.right() - 54.0, rr.y, 44.0, rr.h), &g.f.small_right, th.text3);
        }
    }
    g.unclip();
    if let Some((ty, h)) = thumb_at {
        let wide = s.hover == Some(Hit::Scrollbar) || s.thumb_drag.is_some();
        let w = if wide { 6.0 } else { 3.0 };
        g.fill(Rect::new(l.right() - GUTTER / 2.0 - w / 2.0, ty + 2.0, w, h - 4.0), w / 2.0, if wide { th.text2 } else { th.text3 });
    }
    let note = if !s.installed_ready {
        Some(("Finding apps on this PC\u{2026}".to_string(), ""))
    } else if s.shown.is_empty() {
        Some((format!("No apps match \u{201C}{}\u{201D}", s.query.trim()), "Try Browse to pick a file instead"))
    } else {
        None
    };
    if let Some((a, b)) = note {
        let cy = l.y + l.h * 0.4;
        g.text(&a, Rect::new(l.x + 12.0, cy - 12.0, l.w - 24.0, 22.0), &g.f.strong_c, th.text2);
        if !b.is_empty() {
            g.text(b, Rect::new(l.x + 12.0, cy + 10.0, l.w - 24.0, 18.0), &g.f.small_c, th.text3);
        }
    }

    // footer
    g.fill(Rect::new(0.0, ADD_H - FOOT_H, WIDTH, g.hairline()), 0.0, th.divider);
    let b = back_rect();
    let (hb, pb) = (s.hover == Some(Hit::Back), s.pressed == Some(Hit::Back));
    g.fill(b, 4.0, if pb { th.pressed } else if hb { th.hover } else { hex(0) });
    g.text(G_BACK, Rect::new(b.x + 8.0, b.y + 1.5, 18.0, b.h), &g.f.icon12, th.text);
    g.text("Back", Rect::new(b.x + 30.0, b.y, b.w - 34.0, b.h), &g.f.body, th.text);
    let o = browse_rect();
    let (ho, po) = (s.hover == Some(Hit::Browse), s.pressed == Some(Hit::Browse));
    g.fill(o, 4.0, if po { th.pressed } else if ho { th.control_hover } else { th.control });
    g.outline(o, 4.0, th.stroke);
    g.text(G_FOLDER, Rect::new(o.x + 10.0, o.y, 18.0, o.h), &g.f.icon16, th.text);
    g.text("Browse files", Rect::new(o.x + 34.0, o.y, o.w - 40.0, o.h), &g.f.body, th.text);
    g.set_alpha(1.0);
    g.translate(0.0, 0.0);
}
