//! The one-line text box used by the search and the category name: typing,
//! the caret and its keys, paste, and drawing it.

use crate::*;

/// Shows the caret now and restarts its blink.
pub fn caret_reset() {
    let s = st();
    s.caret_on = true;
    unsafe {
        SetTimer(s.hwnd, TIMER_CARET, GetCaretBlinkTime().clamp(200, 1200), None);
    }
    invalidate();
}

/// Changes the text and/or caret; the search follows the text.
fn edit(f: impl FnOnce(&mut String, &mut usize)) {
    let s = st();
    let before = s.query.clone();
    f(&mut s.query, &mut s.caret);
    s.caret = s.caret.min(s.query.len());
    caret_reset();
    if s.mode == Mode::Add && s.query != before {
        add::refill();
    }
}

/// Empties the box.
pub fn clear() {
    edit(|q, c| {
        q.clear();
        *c = 0;
    });
}

fn prev_char(q: &str, at: usize) -> usize {
    q[..at].char_indices().next_back().map(|(i, _)| i).unwrap_or(0)
}
fn next_char(q: &str, at: usize) -> usize {
    q[at..].chars().next().map(|c| at + c.len_utf8()).unwrap_or(at)
}

/// The most characters the box holds: far more than any name, and few enough that a
/// pasted page of text can't slow the search or the drawing down.
const MAX_CHARS: usize = 256;

/// What typing or pasting `t` adds when there is room for `room` more characters: its
/// first line, without control characters.
fn typed(t: &str, room: usize) -> String {
    t.lines().next().unwrap_or("").chars().filter(|c| !c.is_control()).take(room).collect()
}

/// Inserts the first line of `t` at the caret (control characters dropped).
fn type_text(t: &str) {
    let t = typed(t, MAX_CHARS.saturating_sub(st().query.chars().count()));
    if !t.is_empty() {
        edit(|q, c| {
            q.insert_str(*c, &t);
            *c += t.len();
        });
    }
}

/// WM_CHAR: one UTF-16 unit (pairs arrive as two messages).
pub fn on_char(unit: u16) {
    let s = st();
    if s.mode == Mode::Grid || std::mem::take(&mut s.eat_char) {
        return;
    }
    match unit {
        0x08 => edit(|q, c| {
            let p = prev_char(q, *c);
            q.replace_range(p..*c, "");
            *c = p;
        }),
        // Ctrl+Backspace: the word before the caret
        0x7F => edit(|q, c| {
            let head = q[..*c].trim_end();
            let p = head.rfind(' ').map(|i| i + 1).unwrap_or(0);
            q.replace_range(p..*c, "");
            *c = p;
        }),
        // Ctrl+V
        0x16 => {
            if let Some(t) = shell::clipboard_text(s.hwnd) {
                type_text(&t);
            }
        }
        0xD800..=0xDBFF => s.high_surrogate = Some(unit),
        0xDC00..=0xDFFF => {
            if let Some(h) = s.high_surrogate.take() {
                type_text(&String::from_utf16_lossy(&[h, unit]));
            }
        }
        u if u >= 0x20 => type_text(&String::from_utf16_lossy(&[u])),
        _ => {}
    }
}

/// Caret keys. True = used.
pub fn caret_key(vk: VIRTUAL_KEY) -> bool {
    match vk {
        VK_LEFT => edit(|q, c| *c = prev_char(q, *c)),
        VK_RIGHT => edit(|q, c| *c = next_char(q, *c)),
        VK_HOME => edit(|_, c| *c = 0),
        VK_END => edit(|q, c| *c = q.len()),
        VK_DELETE => edit(|q, c| {
            let e = next_char(q, *c);
            q.replace_range(*c..e, "");
        }),
        _ => return false,
    }
    true
}

/// A text box: rounded box, accent focus line, optional glyph, the text
/// (scrolled so the caret stays in view) or a grey hint, and the caret.
#[allow(clippy::too_many_arguments)]
pub fn draw(g: &Gfx, th: &Theme, f: Rect, text: &str, caret: usize, caret_on: bool, hint: &str, glyph: Option<&str>) {
    g.fill(f, 6.0, th.field);
    g.outline(f, 6.0, th.stroke);
    // the focus line: the box's own rounded shape, cut to its bottom 2 DIPs
    g.clip(Rect::new(f.x, f.bottom() - 2.0, f.w, 2.0));
    g.fill(f, 6.0, th.accent);
    g.unclip();
    let lead = match glyph {
        Some(gl) => {
            g.text(gl, Rect::new(f.x + 6.0, f.y, 30.0, f.h), &g.f.icon16, th.text2);
            38.0
        }
        None => 12.0,
    };
    let tx = f.x + lead;
    let tw = f.w - lead - 12.0;
    let area = Rect::new(tx, f.y, tw, f.h);
    let caret_at = |x: f32| g.fill(Rect::new(g.snap(x), f.y + 11.0, g.hairline().max(1.0), 18.0), 0.0, th.text);
    if text.is_empty() {
        g.text(hint, area, &g.f.body, th.text3);
        if caret_on {
            caret_at(tx - 1.5);
        }
    } else if let Some(l) = g.layout(text, &g.f.body, 4000.0, f.h) {
        let pos = text[..caret.min(text.len())].encode_utf16().count() as u32;
        let (mut cx, mut cy) = (0.0f32, 0.0f32);
        let mut m = DWRITE_HIT_TEST_METRICS::default();
        unsafe {
            let _ = l.HitTestTextPosition(pos, false, &mut cx, &mut cy, &mut m);
        }
        let shift = (cx - tw + 2.0).max(0.0);
        g.clip(area);
        g.draw_layout(&l, tx - shift, f.y, th.text);
        if caret_on {
            caret_at(tx + cx - shift);
        }
        g.unclip();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typing_takes_one_line_without_control_characters() {
        assert_eq!(typed("pa\tint\r\nsecond line", 100), "paint");
        assert_eq!(typed("", 100), "");
    }

    #[test]
    fn typing_stops_when_the_box_is_full() {
        assert_eq!(typed(&"x".repeat(10_000), 3), "xxx");
        assert_eq!(typed("more", 0), "");
        // whole characters, never half of one
        assert_eq!(typed("\u{1F9C5}\u{1F9C5}", 1), "\u{1F9C5}");
    }
}
