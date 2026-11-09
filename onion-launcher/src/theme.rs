//! Colours (Windows 11 "Fluent" tokens, light and dark) and icon glyphs.

use crate::gfx::{hex, Color};

pub struct Theme {
    pub surface: Color,
    pub text: Color,
    pub text2: Color,
    pub text3: Color,
    pub hover: Color,
    pub pressed: Color,
    pub selected: Color,
    pub control: Color,
    pub control_hover: Color,
    pub stroke: Color,
    pub field: Color,
    pub divider: Color,
    pub accent: Color,
    pub accent_soft: Color,
    /// "Release to remove"
    pub danger: Color,
    pub danger_soft: Color,
}

pub const DARK: Theme = Theme {
    surface: hex(0x2B2B2BFF),
    text: hex(0xFFFFFFFF),
    text2: hex(0xFFFFFFC8),
    text3: hex(0xFFFFFF8F),
    hover: hex(0xFFFFFF10),
    pressed: hex(0xFFFFFF0A),
    selected: hex(0xFFFFFF17),
    control: hex(0xFFFFFF0F),
    control_hover: hex(0xFFFFFF17),
    stroke: hex(0xFFFFFF1C),
    field: hex(0x1E1E1EB3),
    divider: hex(0xFFFFFF14),
    accent: hex(0xF28C28FF),
    accent_soft: hex(0xF28C2829),
    danger: hex(0xFF99A4FF),
    danger_soft: hex(0xFF99A42E),
};

pub const LIGHT: Theme = Theme {
    surface: hex(0xF9F9F9FF),
    text: hex(0x000000E4),
    text2: hex(0x0000009E),
    text3: hex(0x0000008F),
    hover: hex(0x0000000A),
    pressed: hex(0x00000006),
    selected: hex(0x00000010),
    control: hex(0xFFFFFFB3),
    control_hover: hex(0xF9F9F980),
    stroke: hex(0x00000024),
    field: hex(0xFFFFFFFF),
    divider: hex(0x00000012),
    accent: hex(0xC8640AFF),
    accent_soft: hex(0xC8640A1F),
    danger: hex(0xC42B1CFF),
    danger_soft: hex(0xC42B1C1F),
};

pub fn get(dark: bool) -> &'static Theme {
    if dark { &DARK } else { &LIGHT }
}

// Segoe Fluent Icons
pub const G_ADD: &str = "\u{E710}";
pub const G_SEARCH: &str = "\u{E721}";
pub const G_BACK: &str = "\u{E72B}";
pub const G_FOLDER: &str = "\u{E838}";
pub const G_CHECK: &str = "\u{E73E}";
pub const G_DELETE: &str = "\u{E74D}";
