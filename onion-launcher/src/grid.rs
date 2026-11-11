//! The main box: apps without a category first, then one section per category
//! (a heading and as many rows of tiles as it needs). Tiles can be dragged to
//! a new spot or into another section; the right-click menu moves an app to a
//! category (or a new one), and a heading's menu renames or deletes it.

use crate::*;

/// Height of a category heading, in DIPs.
pub const HEAD_H: f32 = 30.0;
/// Height of the "name a category" screen.
pub const NAME_H: f32 = PAD + 22.0 + 6.0 + FIELD_H + 10.0 + 16.0 + PAD;
/// glow key of the "+" tile (app glows are keyed by App::key)
pub const ADD_KEY: &str = "\u{1}+";

/// Where a dragged app would land: (category or None for the top section, slot).
pub type Place = (Option<usize>, usize);

pub struct TileDrag {
    pub app: usize,
    /// where in the tile it was grabbed
    pub grab: (f32, f32),
    pub at: (f32, f32),
    origin: (f32, f32),
    /// moved far enough to count as a drag (else it's a click)
    pub started: bool,
    home: Place,
    target: Place,
    /// the mouse is outside the box: dropping there puts it back, or removes
    /// the app once it has been held out there for HOLD_TO_REMOVE seconds
    outside: bool,
    out_since: Option<Instant>,
}

/// Seconds a dragged app is held outside the box before letting go removes it.
pub const HOLD_TO_REMOVE: f32 = 2.0;
/// how long the "Release to remove" panel takes to fade in
const ARM_FADE: f32 = 0.15;

impl TileDrag {
    /// Seconds held outside the box (0 inside it).
    fn held_out(&self) -> f32 {
        self.out_since.map(|t| t.elapsed().as_secs_f32()).unwrap_or(0.0)
    }
    /// Letting go now removes the app.
    pub fn armed(&self) -> bool {
        self.outside && self.held_out() >= HOLD_TO_REMOVE
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Naming {
    New(usize),
    Rename(usize),
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Item {
    App(usize),
    Add,
    /// the empty spot a dragged app would drop into
    Gap,
    /// an empty category while dragging (so it can still be dropped into)
    Empty,
}

pub struct Section {
    pub cat: Option<usize>,
    /// heading top (or tiles top for the top section)
    pub top: f32,
    pub tiles_top: f32,
    /// apps in it, not counting the one being dragged
    pub others: usize,
}

pub struct Layout {
    pub items: Vec<(Item, Rect)>,
    pub heads: Vec<(usize, Rect)>,
    pub sections: Vec<Section>,
    pub h: f32,
}

fn layout_with(s: &State, drag: Option<&TileDrag>) -> Layout {
    let skip = drag.map(|d| d.app);
    let target = drag.map(|d| if d.outside { d.home } else { d.target });
    let apps_in = |name: &str| -> Vec<Item> {
        s.cfg.apps.iter().enumerate().filter(|(i, a)| Some(*i) != skip && a.category == name).map(|(i, _)| Item::App(i)).collect()
    };
    let with_gap = |mut v: Vec<Item>, cat: Option<usize>| {
        let others = v.len();
        if let Some((tc, ti)) = target {
            if tc == cat {
                v.insert(ti.min(others), Item::Gap);
            }
        }
        (v, others)
    };
    let mut secs = vec![];
    let (mut top, others) = with_gap(apps_in(""), None);
    top.push(Item::Add);
    secs.push((None, top, others));
    for (c, name) in s.cfg.categories.iter().enumerate() {
        let (mut v, others) = with_gap(apps_in(name), Some(c));
        if v.is_empty() {
            if drag.is_none() {
                continue;
            }
            v.push(Item::Empty);
        }
        secs.push((Some(c), v, others));
    }
    let mut lay = Layout { items: vec![], heads: vec![], sections: vec![], h: 0.0 };
    let mut y = PAD;
    for (cat, items, others) in secs {
        let top = y;
        if let Some(c) = cat {
            lay.heads.push((c, Rect::new(PAD, y, WIDTH - 2.0 * PAD, HEAD_H)));
            y += HEAD_H;
        }
        for (i, it) in items.iter().enumerate() {
            lay.items.push((*it, Rect::new(PAD + (i % COLS) as f32 * TILE_W, y + (i / COLS) as f32 * TILE_H, TILE_W, TILE_H)));
        }
        lay.sections.push(Section { cat, top, tiles_top: y, others });
        y += items.len().div_ceil(COLS).max(1) as f32 * TILE_H;
    }
    lay.h = y + PAD;
    lay
}

/// The box as it looks right now (with the drag gap while dragging).
pub fn layout(s: &State) -> Layout {
    layout_with(s, s.tile_drag.as_ref().filter(|d| d.started))
}

/// Apps in the order they're shown (what the number keys count).
pub fn order(s: &State) -> Vec<usize> {
    layout_with(s, None).items.iter().filter_map(|(it, _)| if let Item::App(i) = it { Some(*i) } else { None }).collect()
}

pub fn item_rect(s: &State, item: Item) -> Option<Rect> {
    layout(s).items.into_iter().find(|(it, _)| *it == item).map(|(_, r)| r)
}

pub fn hit(s: &State, x: f32, y: f32) -> Option<Hit> {
    let lay = layout(s);
    if let Some((c, _)) = lay.heads.iter().find(|(_, r)| r.contains(x, y)) {
        return Some(Hit::Heading(*c));
    }
    lay.items.iter().find(|(_, r)| r.contains(x, y)).and_then(|(it, _)| match it {
        Item::App(i) => Some(Hit::Tile(*i)),
        Item::Add => Some(Hit::AddTile),
        _ => None,
    })
}

/// The section and slot under the mouse, or None outside the box.
fn drop_target(lay: &Layout, x: f32, y: f32) -> Option<Place> {
    if x < 0.0 || x > WIDTH || y < 0.0 || y > lay.h {
        return None;
    }
    let sec = lay.sections.iter().rev().find(|s| y >= s.top).unwrap_or(&lay.sections[0]);
    let col = (((x - PAD) / TILE_W).floor().max(0.0) as usize).min(COLS - 1);
    let row = ((y - sec.tiles_top) / TILE_H).floor().max(0.0) as usize;
    Some((sec.cat, (row * COLS + col).min(sec.others)))
}

/// An app's section and position in it.
fn place_of(s: &State, app: usize) -> Place {
    let cat = &s.cfg.apps[app].category;
    let c = s.cfg.categories.iter().position(|n| n == cat);
    let idx = s.cfg.apps[..app].iter().filter(|a| &a.category == cat).count();
    (c, idx)
}

// ---------- changing the list ----------

/// Moves an app into a category (None = the top section) at `idx` among the
/// apps already there (past the end = last).
pub fn move_app(app: usize, cat: Option<usize>, idx: usize) {
    let s = st();
    let name = cat.and_then(|c| s.cfg.categories.get(c).cloned()).unwrap_or_default();
    let mut a = s.cfg.apps.remove(app);
    a.category = name;
    let same: Vec<usize> = s.cfg.apps.iter().enumerate().filter(|(_, b)| b.category == a.category).map(|(i, _)| i).collect();
    let at = match same.get(idx) {
        Some(&i) => i,
        None => same.last().map(|&i| i + 1).unwrap_or(s.cfg.apps.len()),
    };
    log(&format!("move '{}' to '{}' slot {idx}", a.name, a.category));
    s.cfg.apps.insert(at, a);
    s.cfg.tidy();
    save();
    resize_in_place();
    kick();
}

pub fn delete_category(c: usize) {
    let s = st();
    let name = s.cfg.categories.remove(c);
    // its apps join the end of the top section, in their order
    let (mut moved, rest): (Vec<App>, Vec<App>) = std::mem::take(&mut s.cfg.apps).into_iter().partition(|a| a.category == name);
    s.cfg.apps = rest;
    moved.iter_mut().for_each(|a| a.category.clear());
    let at = s.cfg.apps.iter().rposition(|a| a.category.is_empty()).map(|i| i + 1).unwrap_or(0);
    s.cfg.apps.splice(at..at, moved);
    log(&format!("delete category '{name}'"));
    save();
    resize_in_place();
    kick();
}

pub fn start_naming(n: Naming) {
    let s = st();
    s.naming = Some(n);
    s.query = match n {
        Naming::New(_) => String::new(),
        Naming::Rename(c) => s.cfg.categories[c].clone(),
    };
    s.caret = s.query.len();
    set_mode(Mode::Name);
}

fn commit_name() {
    let s = st();
    let name = s.query.trim().to_string();
    let Some(n) = s.naming else { return };
    if name.is_empty() {
        return;
    }
    let existing = |s: &State, skip: Option<usize>| s.cfg.categories.iter().enumerate().position(|(i, c)| Some(i) != skip && c.eq_ignore_ascii_case(&name));
    match n {
        Naming::New(app) => {
            let c = existing(s, None).unwrap_or_else(|| {
                s.cfg.categories.push(name.clone());
                s.cfg.categories.len() - 1
            });
            s.naming = None;
            set_mode(Mode::Grid);
            move_app(app, Some(c), usize::MAX);
        }
        Naming::Rename(c) => {
            let old = s.cfg.categories[c].clone();
            // renamed to another category's name: the two merge
            let to = existing(s, Some(c)).map(|e| s.cfg.categories[e].clone()).unwrap_or(name.clone());
            for a in s.cfg.apps.iter_mut().filter(|a| a.category == old) {
                a.category = to.clone();
            }
            s.cfg.categories[c] = to;
            s.cfg.tidy();
            log(&format!("rename category '{old}' -> '{name}'"));
            save();
            s.naming = None;
            set_mode(Mode::Grid);
        }
    }
}

pub fn on_name_key(vk: VIRTUAL_KEY) {
    match vk {
        VK_RETURN => commit_name(),
        VK_ESCAPE => {
            st().naming = None;
            set_mode(Mode::Grid);
        }
        _ => {
            field::caret_key(vk);
        }
    }
}

// ---------- menus ----------

pub fn tile_menu(i: usize) {
    let s = st();
    let cmd = unsafe {
        let (Ok(m), Ok(sub)) = (CreatePopupMenu(), CreatePopupMenu()) else { return };
        let cur = &s.cfg.apps[i].category;
        win::menu_item(sub, 99, "No category", cur.is_empty());
        for (c, name) in s.cfg.categories.iter().enumerate() {
            win::menu_item(sub, 100 + c, name, name == cur);
        }
        let _ = AppendMenuW(sub, MF_SEPARATOR, 0, PCWSTR::null());
        win::menu_item(sub, 98, "New category\u{2026}", false);
        win::menu_item(m, 1, "Open", false);
        let t = shell::wide("Move to category");
        let _ = AppendMenuW(m, MF_POPUP, sub.0 as usize, PCWSTR(t.as_ptr()));
        let _ = AppendMenuW(m, MF_SEPARATOR, 0, PCWSTR::null());
        win::menu_item(m, 2, "Remove from Onion Launcher", false);
        win::track_menu(m)
    };
    match cmd {
        1 => launch(i),
        2 => remove_app(i),
        98 => start_naming(Naming::New(i)),
        99 => move_app(i, None, usize::MAX),
        c if c >= 100 => move_app(i, Some(c - 100), usize::MAX),
        _ => {}
    }
}

pub fn head_menu(c: usize) {
    match win::popup_menu(&[(1, "Rename category"), (0, ""), (2, "Delete category")]) {
        1 => start_naming(Naming::Rename(c)),
        2 => delete_category(c),
        _ => {}
    }
}

// ---------- dragging ----------

pub fn mouse_down(x: f32, y: f32) {
    let s = st();
    if let Some(Hit::Tile(i)) = s.pressed {
        if let Some(r) = item_rect(s, Item::App(i)) {
            let home = place_of(s, i);
            s.tile_drag = Some(TileDrag { app: i, grab: (x - r.x, y - r.y), at: (x, y), origin: (x, y), started: false, home, target: home, outside: false, out_since: None });
        }
    }
}

/// True while a drag is going on (the move is used up).
pub fn mouse_move(x: f32, y: f32) -> bool {
    let s = st();
    let Some(d) = s.tile_drag.as_mut() else { return false };
    d.at = (x, y);
    if !d.started {
        let (dx, dy) = (x - d.origin.0, y - d.origin.1);
        if dx * dx + dy * dy < 36.0 {
            return false;
        }
        d.started = true;
        s.pressed = None;
        s.hover = None;
    }
    let lay = layout(s);
    let t = drop_target(&lay, x, y);
    let d = s.tile_drag.as_mut().unwrap();
    let (outside, target) = match t {
        Some(t) => (false, t),
        None => (true, d.target),
    };
    if outside != d.outside || target != d.target {
        if outside != d.outside {
            // the 2 s count starts again each time it leaves the box
            d.out_since = outside.then(Instant::now);
        }
        d.outside = outside;
        d.target = target;
        resize_in_place();
    }
    kick();
    invalidate();
    true
}

/// True if this ended a drag (so it isn't also a click).
pub fn mouse_up() -> bool {
    let s = st();
    let Some(d) = s.tile_drag.take() else { return false };
    if !d.started {
        return false;
    }
    let key = s.cfg.apps[d.app].key();
    if d.armed() {
        s.pos.remove(&key);
        remove_app(d.app);
        return true;
    }
    // the tile glides from where it was let go into its new spot
    s.pos.insert(key, (d.at.0 - d.grab.0, d.at.1 - d.grab.1));
    if d.outside || d.target == d.home {
        log("drag: put back");
        resize_in_place();
        kick();
    } else {
        move_app(d.app, d.target.0, d.target.1);
    }
    true
}

pub fn cancel_drag() {
    let s = st();
    if s.tile_drag.take().is_some_and(|d| d.started) {
        resize_in_place();
        kick();
    }
}

/// Tiles glide to new spots when the order changes. True while moving.
pub fn animate(s: &mut State, dt: f32) -> bool {
    let lay = layout(s);
    let k = 1.0 - (-dt / 0.055).exp();
    // keep the frames coming while counting down to (and fading in) "Release to remove"
    let mut busy = s.tile_drag.as_ref().is_some_and(|d| d.started && d.outside && d.held_out() < HOLD_TO_REMOVE + ARM_FADE);
    for (it, r) in &lay.items {
        let Item::App(i) = it else { continue };
        let p = s.pos.entry(s.cfg.apps[*i].key()).or_insert((r.x, r.y));
        let (dx, dy) = (r.x - p.0, r.y - p.1);
        if dx.abs() + dy.abs() > 0.3 {
            p.0 += dx * k;
            p.1 += dy * k;
            busy = true;
        } else {
            *p = (r.x, r.y);
        }
    }
    busy
}

// ---------- painting ----------

/// A tile's card (hover, pressed, lifted): it ends as far below the name (one line or
/// two) as it starts above the icon, not at the bottom of the slot, which leaves room
/// for a second line.
fn card_rect(r: Rect, label: Option<&IDWriteTextLayout>) -> Rect {
    let label_h = label.map_or(0.0, |l| {
        let mut m = DWRITE_TEXT_METRICS::default();
        unsafe {
            let _ = l.GetMetrics(&mut m);
        }
        m.height
    });
    let inner = r.inset(2.0);
    Rect::new(inner.x, inner.y, inner.w, (54.0 + label_h + 10.0 - 2.0).min(inner.h))
}

#[allow(clippy::too_many_arguments)]
fn draw_tile(g: &Gfx, th: &Theme, icons: &mut Icons, names: &mut HashMap<String, IDWriteTextLayout>, app: &App, r: Rect, glow: f32, down: bool, badge: Option<usize>, ipx: u32) {
    let label = name_layout(names, g, &app.name, r.w - 8.0).cloned();
    let inner = card_rect(r, label.as_ref());
    if glow > 0.0 {
        g.fill(inner, 8.0, if down { th.pressed } else { th.hover.fade(glow) });
        g.outline(inner, 8.0, th.stroke.fade(glow * 0.6));
    }
    let cx = r.x + r.w / 2.0;
    let isz = g.snap(GRID_ICON * if down { 0.9 } else { 1.0 });
    let iy = r.y + 12.0 + (GRID_ICON - isz) / 2.0;
    let ir = Rect::new(g.snap(cx - isz / 2.0), g.snap(iy), isz, isz);
    match icons.get(app.icon_source(), ipx, true) {
        Some(b) => g.image(b, ir),
        None => placeholder(g, th, ir, &app.name),
    }
    if let Some(l) = &label {
        g.draw_layout(l, r.x + 4.0, r.y + 54.0, th.text);
    }
    if let Some(k) = badge {
        g.text(&k.to_string(), Rect::new(r.x + 6.0, r.y + 5.0, 14.0, 14.0), &g.f.badge, if glow > 0.5 { th.text2 } else { th.text3 });
    }
}

pub fn paint_grid(s: &mut State, th: &Theme) {
    let lay = layout(s);
    let t = s.shown_at.elapsed().as_secs_f32() * 1000.0;
    let ipx = icon_px(s, GRID_ICON);
    let drag = s.tile_drag.as_ref().filter(|d| d.started).map(|d| (d.app, d.at, d.grab));
    // 0..1: how far the "Release to remove" panel has faded in
    let arm = s.tile_drag.as_ref().filter(|d| d.started && d.outside).map(|d| ease((d.held_out() - HOLD_TO_REMOVE) / ARM_FADE)).unwrap_or(0.0);
    let (g, icons, names, cfg, glow, pressed, pos) = (&s.gfx, &mut s.icons, &mut s.names, &s.cfg, &s.glow, s.pressed, &s.pos);

    for (c, r) in &lay.heads {
        let name = &cfg.categories[*c];
        let tr = Rect::new(r.x + 6.0, r.y + 8.0, r.w - 12.0, 18.0);
        let mut width = 0.0;
        if let Some(l) = g.layout(name, &g.f.head, tr.w - 40.0, tr.h) {
            let mut m = DWRITE_TEXT_METRICS::default();
            unsafe {
                let _ = l.GetMetrics(&mut m);
            }
            width = m.widthIncludingTrailingWhitespace;
            g.draw_layout(&l, tr.x, tr.y, th.text2);
        }
        // a thin line from the name to the edge
        let lx = tr.x + width + 10.0;
        g.fill(Rect::new(lx, g.snap(tr.y + tr.h / 2.0 + 1.0), r.right() - 6.0 - lx, g.hairline()), 0.0, th.divider);
    }

    let mut shown_apps = 0;
    for (k, (item, r)) in lay.items.iter().enumerate() {
        // tiles drift up and fade in, one after another
        let p = ease((t - (k as f32 * 12.0).min(72.0)) / 170.0);
        g.set_alpha(p);
        g.translate(0.0, (1.0 - p) * 6.0);
        match *item {
            Item::App(i) => {
                let app = &cfg.apps[i];
                let r = pos.get(&app.key()).map(|&(x, y)| Rect::new(x, y, r.w, r.h)).unwrap_or(*r);
                let gl = glow.get(&app.key()).copied().unwrap_or(0.0);
                let badge = (shown_apps < 10).then_some((shown_apps + 1) % 10);
                draw_tile(g, th, icons, names, app, r, gl, pressed == Some(Hit::Tile(i)), badge, ipx);
                shown_apps += 1;
            }
            Item::Add => {
                let gl = glow.get(ADD_KEY).copied().unwrap_or(0.0);
                let down = pressed == Some(Hit::AddTile);
                let inner = r.inset(2.0);
                if gl > 0.0 {
                    g.fill(inner, 8.0, if down { th.pressed } else { th.hover.fade(gl) });
                    g.outline(inner, 8.0, th.stroke.fade(gl * 0.6));
                }
                let cx = r.x + r.w / 2.0;
                let rad = GRID_ICON / 2.0 * if down { 0.9 } else { 1.0 };
                let cy = r.y + 12.0 + GRID_ICON / 2.0;
                g.circle(cx, cy, rad, th.accent_soft);
                g.text(G_ADD, Rect::new(cx - 12.0, cy - 12.0, 24.0, 24.0), &g.f.icon16, th.accent);
                g.text("Add", Rect::new(r.x + 4.0, r.y + 54.0, r.w - 8.0, 16.0), &g.f.label, th.text2);
                if cfg.apps.is_empty() {
                    let x = r.right() + 8.0;
                    let w = WIDTH - PAD - x;
                    g.text("No apps yet", Rect::new(x, r.y + 26.0, w, 20.0), &g.f.strong, th.text);
                    g.text("Click + to add some, or drop files here", Rect::new(x, r.y + 46.0, w, 18.0), &g.f.small, th.text2);
                }
            }
            Item::Gap => {
                // the spot the dragged tile would drop into: as tall as its card
                let rr = match drag {
                    Some((i, ..)) => card_rect(*r, name_layout(names, g, &cfg.apps[i].name, r.w - 8.0)).inset(2.0),
                    None => r.inset(4.0),
                };
                g.fill(rr, 8.0, th.accent_soft);
                g.outline(rr, 8.0, th.accent.fade(0.55));
            }
            Item::Empty => {
                let rr = r.inset(4.0);
                g.outline(rr, 8.0, th.stroke);
                g.text("Drop here", rr, &g.f.small_c, th.text3);
            }
        }
    }
    g.set_alpha(1.0);
    g.translate(0.0, 0.0);

    // held outside the box long enough: letting go removes it
    if let (true, Some((i, ..))) = (arm > 0.0, drag) {
        let full = Rect::new(0.0, 0.0, WIDTH, lay.h);
        g.fill(full, 0.0, th.surface.fade(0.9 * arm));
        g.fill(full, 0.0, th.danger_soft.fade(arm));
        // a card: a trash can in a soft red circle, "Release to remove", the app's name
        let card = Rect::new(g.snap(WIDTH / 2.0 - 116.0), g.snap(lay.h / 2.0 - 50.0), 232.0, 100.0);
        g.fill(card.inset(-1.0), 11.0, hex(0x00000014).fade(arm));
        g.fill(card, 10.0, th.surface.fade(arm));
        g.fill(card, 10.0, th.danger_soft.fade(arm * 0.6));
        g.outline(card, 10.0, th.danger.fade(0.45 * arm));
        let cx = WIDTH / 2.0;
        g.circle(cx, card.y + 30.0, 18.0, th.danger_soft.fade(arm * 1.6));
        g.text(G_DELETE, Rect::new(cx - 12.0, card.y + 18.0, 24.0, 24.0), &g.f.icon16, th.danger.fade(arm));
        g.text("Release to remove", Rect::new(card.x + 10.0, card.y + 54.0, card.w - 20.0, 20.0), &g.f.strong_c, th.text.fade(arm));
        g.text(&cfg.apps[i].name, Rect::new(card.x + 12.0, card.y + 74.0, card.w - 24.0, 16.0), &g.f.small_c, th.text2.fade(arm));
    }

    // the dragged tile, lifted above the rest
    if let Some((i, at, grab)) = drag {
        let r = Rect::new(at.0 - grab.0, at.1 - grab.1, TILE_W, TILE_H);
        let card = card_rect(r, name_layout(names, g, &cfg.apps[i].name, r.w - 8.0));
        g.fill(card.inset(-1.0), 9.0, hex(0x00000022));
        g.fill(card, 8.0, th.surface);
        g.fill(card, 8.0, th.selected);
        g.outline(card, 8.0, if arm > 0.0 { th.danger.fade(arm) } else { th.stroke });
        draw_tile(g, th, icons, names, &cfg.apps[i], r, 0.0, false, None, ipx);
    }
}

/// "New category" / "Rename": a title, a name field and a hint.
pub fn paint_name(s: &mut State, th: &Theme) {
    let p = ease(s.shown_at.elapsed().as_secs_f32() * 1000.0 / 160.0);
    let g = &s.gfx;
    g.set_alpha(p);
    let title = match s.naming {
        Some(Naming::New(app)) => format!("New category for \u{201C}{}\u{201D}", s.cfg.apps.get(app).map(|a| a.name.as_str()).unwrap_or("")),
        Some(Naming::Rename(c)) => format!("Rename \u{201C}{}\u{201D}", s.cfg.categories.get(c).map(String::as_str).unwrap_or("")),
        None => String::new(),
    };
    g.text(&title, Rect::new(PAD + 2.0, PAD, WIDTH - 2.0 * PAD - 4.0, 22.0), &g.f.strong, th.text);
    let f = Rect::new(PAD, PAD + 28.0, WIDTH - 2.0 * PAD, FIELD_H);
    field::draw(g, th, f, &s.query, s.caret, s.caret_on, "Category name, like Games", None);
    g.text("Enter to save  \u{00B7}  Esc to cancel", Rect::new(PAD + 2.0, f.bottom() + 10.0, f.w, 16.0), &g.f.small, th.text3);
    g.set_alpha(1.0);
}
