//! The hotkey that opens the box: Win + the key left of 1, unless another one was
//! picked (tray menu > Change hotkey..., saved in apps.json). This file names keys,
//! registers the hotkey with Windows, and runs the "press the new keys" screen.

use crate::*;
use config::Hotkey;
use gfx::Color;

/// Height of the "Change hotkey" screen (the same as the naming screen).
pub const PANEL_H: f32 = grid::NAME_H;
/// How long "Saved" stays up before the box closes, in ms.
const SAVED_MS: u32 = 1400;

/// What the line under the keys says.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Note {
    Hint,
    NeedsModifier,
    Taken,
    Reserved,
    Saved,
}

/// The "Change hotkey" screen's state.
pub struct Capture {
    pub note: Note,
    /// modifiers held right now (shown while the rest is still to come)
    pub held: u32,
    /// the last combination pressed (shown in the box, red if refused)
    pub tried: Option<Hotkey>,
}

impl Capture {
    pub fn new() -> Capture {
        Capture { note: Note::Hint, held: 0, tried: None }
    }
}

/// Win + the key left of 1 on this keyboard (` on US layouts, ^ on German, ' on UK...).
pub fn default_key() -> Hotkey {
    let key = match unsafe { MapVirtualKeyW(0x29, MAPVK_VSC_TO_VK) } {
        0 => VK_OEM_3.0 as u32,
        vk => vk,
    };
    Hotkey { mods: MOD_WIN.0, key }
}

pub fn current() -> Hotkey {
    st().cfg.hotkey.unwrap_or_else(default_key)
}

/// The key's name on this keyboard: "A", "Space", "F5", "`".
pub fn key_name(vk: u32) -> String {
    unsafe {
        let sc = MapVirtualKeyW(vk, MAPVK_VK_TO_VSC);
        if sc != 0 {
            let mut lp = ((sc & 0xff) as i32) << 16;
            // these share their scan code with the number pad: without this, Page Down is "Num 3"
            let nav = [VK_PRIOR, VK_NEXT, VK_END, VK_HOME, VK_LEFT, VK_UP, VK_RIGHT, VK_DOWN, VK_INSERT, VK_DELETE, VK_DIVIDE, VK_NUMLOCK];
            if nav.iter().any(|k| u32::from(k.0) == vk) {
                lp |= 1 << 24;
            }
            let mut buf = [0u16; 64];
            let n = GetKeyNameTextW(lp, &mut buf);
            if n > 0 {
                return String::from_utf16_lossy(&buf[..n as usize]);
            }
        }
    }
    format!("Key {vk}")
}

/// "Win", "Ctrl", "Alt", "Shift" for the modifiers in `mods`, in that order.
fn mod_names(mods: u32) -> Vec<&'static str> {
    [(MOD_WIN, "Win"), (MOD_CONTROL, "Ctrl"), (MOD_ALT, "Alt"), (MOD_SHIFT, "Shift")].iter().filter(|(m, _)| mods & m.0 != 0).map(|&(_, n)| n).collect()
}

/// Each key of a combination, in order: ["Ctrl", "Shift", "Space"].
fn parts(h: Hotkey) -> Vec<String> {
    let mut v: Vec<String> = mod_names(h.mods).into_iter().map(String::from).collect();
    v.push(key_name(h.key));
    v
}

/// "Ctrl+Shift+Space".
pub fn label(h: Hotkey) -> String {
    parts(h).join("+")
}

/// Takes the hotkey for this window. False if another app already has it.
fn register(h: Hotkey) -> bool {
    let s = st();
    let ok = if s.test.is_some() {
        // the test never takes a real hotkey; Win+L stands in for a taken one (Windows keeps it)
        !(h.mods == MOD_WIN.0 && h.key == u32::from(b'L'))
    } else {
        unsafe { RegisterHotKey(s.hwnd, HOTKEY_ID, HOT_KEY_MODIFIERS(h.mods) | MOD_NOREPEAT, h.key).is_ok() }
    };
    s.hotkey_on = ok;
    ok
}

fn unregister() {
    let s = st();
    if s.hotkey_on && s.test.is_none() {
        unsafe {
            let _ = UnregisterHotKey(s.hwnd, HOTKEY_ID);
        }
    }
    s.hotkey_on = false;
}

/// Takes the saved hotkey (at start, and after the screen closes). False if it's taken.
pub fn register_current() -> bool {
    st().hotkey_on || register(current())
}

/// Tray menu > Change hotkey...: the box opens on the "press the new keys" screen.
pub fn start() {
    show_popup_in(Mode::Hotkey);
}

/// Entering the screen: let go of the hotkey, so pressing it (or anything) lands here.
pub fn entered() {
    let s = st();
    s.capture = Capture::new();
    unregister();
    log(&format!("change hotkey: now {}", label(current())));
}

/// Leaving the screen without a new one: take the old one back.
pub fn left() {
    unsafe {
        let _ = KillTimer(st().hwnd, TIMER_SAVED);
    }
    register_current();
}

fn held_mods() -> u32 {
    let down = |vk: VIRTUAL_KEY| unsafe { GetKeyState(vk.0 as i32) } < 0;
    let mut m = 0;
    if down(VK_LWIN) || down(VK_RWIN) {
        m |= MOD_WIN.0;
    }
    if down(VK_CONTROL) {
        m |= MOD_CONTROL.0;
    }
    if down(VK_MENU) {
        m |= MOD_ALT.0;
    }
    if down(VK_SHIFT) {
        m |= MOD_SHIFT.0;
    }
    m
}

fn is_modifier(vk: VIRTUAL_KEY) -> bool {
    matches!(vk, VK_LWIN | VK_RWIN | VK_CONTROL | VK_LCONTROL | VK_RCONTROL | VK_MENU | VK_LMENU | VK_RMENU | VK_SHIFT | VK_LSHIFT | VK_RSHIFT)
}

/// A key went down (or up: modifiers only) on the screen.
pub fn on_key(vk: VIRTUAL_KEY, down: bool) {
    let s = st();
    // closing; or the do-nothing key win::swallow_win_key presses while Win is held
    if s.capture.note == Note::Saved || vk.0 == win::SWALLOW_KEY {
        return;
    }
    if is_modifier(vk) {
        // Win let go on its own would open the Start menu and close the box
        if down && (vk == VK_LWIN || vk == VK_RWIN) && s.test.is_none() {
            win::swallow_win_key();
        }
        hold(held_mods());
        return;
    }
    s.capture.held = held_mods();
    if !down {
        return;
    }
    if vk == VK_ESCAPE {
        log("change hotkey: cancelled");
        hide_popup();
        return;
    }
    try_set(s.capture.held, vk.0 as u32);
}

/// Modifiers pressed or let go: a new try starts, so an old "refused" goes away.
pub fn hold(mods: u32) {
    let c = &mut st().capture;
    c.held = mods;
    if mods != 0 && c.note != Note::Hint {
        c.note = Note::Hint;
        c.tried = None;
    }
    invalidate();
}

/// Checks a pressed combination and, if Windows lets us have it, saves it.
pub fn try_set(mods: u32, key: u32) {
    let s = st();
    let h = Hotkey { mods, key };
    s.capture.tried = Some(h);
    s.capture.note = if mods & (MOD_CONTROL.0 | MOD_ALT.0 | MOD_WIN.0) == 0 {
        Note::NeedsModifier
    } else if mods == MOD_ALT.0 && key == VK_F4.0 as u32 {
        Note::Reserved
    } else if register(h) {
        // the usual one is kept as "none", so it follows the keyboard layout
        s.cfg.hotkey = (h != default_key()).then_some(h);
        save();
        win::set_tray_tip(s.hwnd, &label(h));
        unsafe {
            SetTimer(s.hwnd, TIMER_SAVED, SAVED_MS, None);
        }
        Note::Saved
    } else {
        Note::Taken
    };
    log(&format!("change hotkey: {} -> {:?}", label(h), s.capture.note));
    invalidate();
}

/// "Saved" has been up long enough.
pub fn saved_timer() {
    unsafe {
        let _ = KillTimer(st().hwnd, TIMER_SAVED);
    }
    if st().mode == Mode::Hotkey {
        hide_popup();
    }
}

/// Keys drawn as little rounded caps with "+" between, centred in `r`.
fn draw_caps(g: &Gfx, th: &Theme, r: Rect, keys: &[String], more: bool, color: Color) {
    let (cap_h, pad, gap) = (26.0, 9.0, 16.0);
    let mut caps: Vec<(IDWriteTextLayout, f32)> = vec![];
    for k in keys {
        let Some(l) = g.layout(k, &g.f.label, 400.0, cap_h) else { continue };
        let mut m = DWRITE_TEXT_METRICS::default();
        unsafe {
            let _ = l.GetMetrics(&mut m);
        }
        caps.push((l, m.widthIncludingTrailingWhitespace.ceil()));
    }
    let n = caps.len() + more as usize;
    let total: f32 = caps.iter().map(|(_, w)| w + 2.0 * pad).sum::<f32>() + gap * n.saturating_sub(1) as f32 + if more { 14.0 } else { 0.0 };
    let mut x = (r.x + (r.w - total) / 2.0).round();
    let y = (r.y + (r.h - cap_h) / 2.0).round();
    for (i, (l, w)) in caps.iter().enumerate() {
        if i > 0 {
            g.text("+", Rect::new(x - gap, y, gap, cap_h), &g.f.small_c, th.text3);
        }
        let cap = Rect::new(x, y, w + 2.0 * pad, cap_h);
        g.fill(cap, 5.0, th.control_hover);
        g.outline(cap, 5.0, th.stroke);
        unsafe {
            let _ = l.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER);
            let _ = l.SetMaxWidth(cap.w);
        }
        g.draw_layout(l, cap.x, cap.y, color);
        x += cap.w + gap;
    }
    if more {
        g.text("+", Rect::new(x - gap, y, gap, cap_h), &g.f.small_c, th.text3);
        g.text("\u{2026}", Rect::new(x, y, 14.0, cap_h), &g.f.small_c, th.text3);
    }
}

/// "Change hotkey": a title, the keys pressed (or a prompt), and a line under them.
pub fn paint(s: &mut State, th: &Theme) {
    let p = ease(s.shown_at.elapsed().as_secs_f32() * 1000.0 / 160.0);
    let g = &s.gfx;
    let c = &s.capture;
    g.set_alpha(p);
    g.text("Change hotkey", Rect::new(PAD + 2.0, PAD, WIDTH - 2.0 * PAD - 4.0, 22.0), &g.f.strong, th.text);
    let f = Rect::new(PAD, PAD + 28.0, WIDTH - 2.0 * PAD, FIELD_H);
    let refused = matches!(c.note, Note::NeedsModifier | Note::Taken | Note::Reserved);
    g.fill(f, 6.0, th.field);
    g.outline(f, 6.0, if refused { th.danger } else { th.stroke });
    g.clip(Rect::new(f.x, f.bottom() - 2.0, f.w, 2.0));
    g.fill(f, 6.0, if refused { th.danger } else { th.accent });
    g.unclip();
    if c.held != 0 && c.note != Note::Saved {
        // modifiers held, the last key still to come
        let keys: Vec<String> = mod_names(c.held).into_iter().map(String::from).collect();
        draw_caps(g, th, f, &keys, true, th.text);
    } else if let Some(h) = c.tried {
        draw_caps(g, th, f, &parts(h), false, th.text);
    } else {
        g.text("Press the new keys", f, &g.f.label, th.text3);
    }
    let now = label(current());
    let (line, color) = match c.note {
        Note::Hint => (format!("Now {now}  \u{00B7}  Esc to cancel"), th.text3),
        Note::NeedsModifier => ("Hold Ctrl, Alt or Win with it".to_string(), th.danger),
        Note::Taken => ("Another app already uses that one".to_string(), th.danger),
        Note::Reserved => ("That one closes windows, try another".to_string(), th.danger),
        Note::Saved => ("Saved. Press it any time to open the box".to_string(), th.text2),
    };
    let hint = Rect::new(PAD + 2.0, f.bottom() + 10.0, f.w - 4.0, 16.0);
    if c.note == Note::Saved {
        g.text(G_CHECK, Rect::new(hint.x, hint.y, 14.0, hint.h), &g.f.icon12, th.accent);
        g.text(&line, Rect::new(hint.x + 20.0, hint.y, hint.w - 20.0, hint.h), &g.f.small, color);
    } else {
        g.text(&line, hint, &g.f.small, color);
    }
    g.set_alpha(1.0);
}
