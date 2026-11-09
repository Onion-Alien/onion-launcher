//! Drawing: Direct2D + DirectWrite into a 32-bit memory bitmap with real
//! alpha, then one copy to the window. Smooth shapes, crisp text, and the
//! see-through parts let Windows' blurred backdrop show. The same bitmap is
//! what the self-test saves as a picture.
//!
//! It runs on the CPU (a software render target): the box is small, so a
//! frame costs about a millisecond and the app holds no graphics memory.

use crate::shell::Pixels;
use windows::core::{w, HSTRING};
use windows::Foundation::Numerics::Matrix3x2;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::DirectWrite::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::Win32::Graphics::Gdi::*;

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Color(pub f32, pub f32, pub f32, pub f32);

/// 0xRRGGBBAA
pub const fn hex(v: u32) -> Color {
    Color(((v >> 24) & 255) as f32 / 255.0, ((v >> 16) & 255) as f32 / 255.0, ((v >> 8) & 255) as f32 / 255.0, (v & 255) as f32 / 255.0)
}

impl Color {
    pub fn fade(self, a: f32) -> Color {
        Color(self.0, self.1, self.2, self.3 * a)
    }
}

/// A rectangle in DIPs (1/96 inch).
#[derive(Clone, Copy, Default, Debug)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect { x, y, w, h }
    }
    pub fn inset(self, d: f32) -> Rect {
        Rect::new(self.x + d, self.y + d, self.w - 2.0 * d, self.h - 2.0 * d)
    }
    pub fn right(&self) -> f32 {
        self.x + self.w
    }
    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.right() && y >= self.y && y < self.bottom()
    }
    fn d2d(&self) -> D2D_RECT_F {
        D2D_RECT_F { left: self.x, top: self.y, right: self.right(), bottom: self.bottom() }
    }
}

pub struct Fonts {
    /// tile names: 12, centred, wrapping
    pub name: IDWriteTextFormat,
    /// list rows and the search text: 14
    pub body: IDWriteTextFormat,
    /// headings: 14 semibold
    pub strong: IDWriteTextFormat,
    /// hints, "Added", key numbers: 12
    pub small: IDWriteTextFormat,
    pub small_right: IDWriteTextFormat,
    pub badge: IDWriteTextFormat,
    /// category headings: 12 semibold
    pub head: IDWriteTextFormat,
    /// centred one-liners: 12, 14 semibold, 12
    pub label: IDWriteTextFormat,
    pub strong_c: IDWriteTextFormat,
    pub small_c: IDWriteTextFormat,
    /// Segoe Fluent Icons glyphs
    pub icon16: IDWriteTextFormat,
    pub icon12: IDWriteTextFormat,
}

pub struct Gfx {
    pub dw: IDWriteFactory,
    rt: ID2D1DCRenderTarget,
    brush: ID2D1SolidColorBrush,
    dc: HDC,
    bmp: HBITMAP,
    old: HGDIOBJ,
    bits: *mut u8,
    pub w: i32,
    pub h: i32,
    scale: f32,
    /// multiplies every colour and picture (fade-ins)
    alpha: std::cell::Cell<f32>,
    pub f: Fonts,
}

fn has_family(dw: &IDWriteFactory, name: &str) -> bool {
    unsafe {
        let mut coll = None;
        if dw.GetSystemFontCollection(&mut coll, false).is_err() {
            return false;
        }
        let (mut i, mut found) = (0u32, BOOL(0));
        coll.is_some_and(|c| c.FindFamilyName(&HSTRING::from(name), &mut i, &mut found).is_ok() && found.as_bool())
    }
}

fn format(dw: &IDWriteFactory, family: &str, size: f32, weight: DWRITE_FONT_WEIGHT, align: DWRITE_TEXT_ALIGNMENT, wrap: bool) -> windows::core::Result<IDWriteTextFormat> {
    unsafe {
        let f = dw.CreateTextFormat(&HSTRING::from(family), None, weight, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_STRETCH_NORMAL, size, w!("en-us"))?;
        f.SetTextAlignment(align)?;
        f.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER)?;
        if wrap {
            f.SetWordWrapping(DWRITE_WORD_WRAPPING_WRAP)?;
        } else {
            f.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP)?;
            let sign = dw.CreateEllipsisTrimmingSign(&f)?;
            let trim = DWRITE_TRIMMING { granularity: DWRITE_TRIMMING_GRANULARITY_CHARACTER, delimiter: 0, delimiterCount: 0 };
            f.SetTrimming(&trim, &sign)?;
        }
        Ok(f)
    }
}

impl Gfx {
    pub fn new() -> windows::core::Result<Gfx> {
        unsafe {
            let d2d: ID2D1Factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
            let props = D2D1_RENDER_TARGET_PROPERTIES {
                r#type: D2D1_RENDER_TARGET_TYPE_SOFTWARE,
                pixelFormat: D2D1_PIXEL_FORMAT { format: DXGI_FORMAT_B8G8R8A8_UNORM, alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED },
                dpiX: 96.0,
                dpiY: 96.0,
                usage: D2D1_RENDER_TARGET_USAGE_NONE,
                minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
            };
            let rt = d2d.CreateDCRenderTarget(&props)?;
            // see-through pixels rule out ClearType; greyscale is what WinUI uses too
            rt.SetTextAntialiasMode(D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE);
            let brush = rt.CreateSolidColorBrush(&D2D1_COLOR_F { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }, None)?;
            let dw: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;
            let text = if has_family(&dw, "Segoe UI Variable Text") { "Segoe UI Variable Text" } else { "Segoe UI" };
            let glyphs = if has_family(&dw, "Segoe Fluent Icons") { "Segoe Fluent Icons" } else { "Segoe MDL2 Assets" };
            let (c, l, r) = (DWRITE_TEXT_ALIGNMENT_CENTER, DWRITE_TEXT_ALIGNMENT_LEADING, DWRITE_TEXT_ALIGNMENT_TRAILING);
            let f = Fonts {
                name: format(&dw, text, 12.0, DWRITE_FONT_WEIGHT_NORMAL, c, true)?,
                body: format(&dw, text, 14.0, DWRITE_FONT_WEIGHT_NORMAL, l, false)?,
                strong: format(&dw, text, 14.0, DWRITE_FONT_WEIGHT_SEMI_BOLD, l, false)?,
                small: format(&dw, text, 12.0, DWRITE_FONT_WEIGHT_NORMAL, l, false)?,
                small_right: format(&dw, text, 12.0, DWRITE_FONT_WEIGHT_NORMAL, r, false)?,
                badge: format(&dw, text, 11.0, DWRITE_FONT_WEIGHT_SEMI_BOLD, c, false)?,
                head: format(&dw, text, 12.0, DWRITE_FONT_WEIGHT_SEMI_BOLD, l, false)?,
                label: format(&dw, text, 12.0, DWRITE_FONT_WEIGHT_NORMAL, c, false)?,
                strong_c: format(&dw, text, 14.0, DWRITE_FONT_WEIGHT_SEMI_BOLD, c, false)?,
                small_c: format(&dw, text, 12.0, DWRITE_FONT_WEIGHT_NORMAL, c, false)?,
                icon16: format(&dw, glyphs, 16.0, DWRITE_FONT_WEIGHT_NORMAL, c, false)?,
                icon12: format(&dw, glyphs, 12.0, DWRITE_FONT_WEIGHT_NORMAL, c, false)?,
            };
            let dc = CreateCompatibleDC(None);
            Ok(Gfx { dw, rt, brush, dc, bmp: HBITMAP::default(), old: HGDIOBJ::default(), bits: std::ptr::null_mut(), w: 0, h: 0, scale: 1.0, alpha: std::cell::Cell::new(1.0), f })
        }
    }

    /// Starts a frame of `w` x `h` pixels, cleared to see-through.
    pub fn begin(&mut self, w: i32, h: i32, scale: f32) {
        unsafe {
            if w != self.w || h != self.h || self.bits.is_null() {
                if !self.bmp.is_invalid() {
                    SelectObject(self.dc, self.old);
                    let _ = DeleteObject(self.bmp);
                }
                let info = crate::shell::bmi(w.max(1), h.max(1));
                let mut bits = std::ptr::null_mut();
                self.bmp = CreateDIBSection(self.dc, &info, DIB_RGB_COLORS, &mut bits, None, 0).unwrap_or_default();
                self.old = SelectObject(self.dc, self.bmp);
                self.bits = bits as *mut u8;
                self.w = w;
                self.h = h;
            }
            self.scale = scale;
            let _ = self.rt.BindDC(self.dc, &RECT { left: 0, top: 0, right: w, bottom: h });
            self.rt.SetDpi(96.0 * scale, 96.0 * scale);
            self.rt.BeginDraw();
            self.rt.Clear(Some(&D2D1_COLOR_F { r: 0.0, g: 0.0, b: 0.0, a: 0.0 }));
            self.set_alpha(1.0);
            self.translate(0.0, 0.0);
        }
    }

    pub fn end(&mut self) {
        unsafe {
            let _ = self.rt.EndDraw(None, None);
            let _ = GdiFlush();
        }
    }

    pub fn blit(&self, hdc: HDC) {
        unsafe {
            let _ = BitBlt(hdc, 0, 0, self.w, self.h, self.dc, 0, 0, SRCCOPY);
        }
    }

    /// The last frame as straight RGBA (for the self-test pictures).
    pub fn rgba(&self) -> Vec<u8> {
        let n = (self.w * self.h * 4) as usize;
        let mut px = unsafe { std::slice::from_raw_parts(self.bits, n) }.to_vec();
        for p in px.chunks_exact_mut(4) {
            let a = p[3] as u32;
            if a > 0 && a < 255 {
                for c in &mut p[..3] {
                    *c = ((*c as u32 * 255 + a / 2) / a).min(255) as u8;
                }
            }
            p.swap(0, 2);
        }
        px
    }

    pub fn set_alpha(&self, a: f32) {
        self.alpha.set(a);
    }

    /// Rounds a length to whole pixels (sharp icons and lines).
    pub fn snap(&self, v: f32) -> f32 {
        (v * self.scale).round() / self.scale
    }

    /// One device pixel, in DIPs.
    pub fn hairline(&self) -> f32 {
        1.0 / self.scale
    }

    fn color(&self, c: Color) {
        unsafe { self.brush.SetColor(&D2D1_COLOR_F { r: c.0, g: c.1, b: c.2, a: c.3 * self.alpha.get() }) }
    }

    pub fn translate(&self, x: f32, y: f32) {
        unsafe { self.rt.SetTransform(&Matrix3x2::translation(x, y)) }
    }

    pub fn fill(&self, r: Rect, radius: f32, c: Color) {
        if c.3 * self.alpha.get() <= 0.0 {
            return;
        }
        self.color(c);
        unsafe {
            if radius > 0.0 {
                self.rt.FillRoundedRectangle(&D2D1_ROUNDED_RECT { rect: r.d2d(), radiusX: radius, radiusY: radius }, &self.brush);
            } else {
                self.rt.FillRectangle(&r.d2d(), &self.brush);
            }
        }
    }

    /// A 1-pixel outline drawn just inside `r`.
    pub fn outline(&self, r: Rect, radius: f32, c: Color) {
        if c.3 * self.alpha.get() <= 0.0 {
            return;
        }
        self.color(c);
        let t = self.hairline();
        let r = r.inset(t / 2.0);
        unsafe { self.rt.DrawRoundedRectangle(&D2D1_ROUNDED_RECT { rect: r.d2d(), radiusX: radius, radiusY: radius }, &self.brush, t, None) }
    }

    pub fn circle(&self, cx: f32, cy: f32, radius: f32, c: Color) {
        self.color(c);
        let e = D2D1_ELLIPSE { point: D2D_POINT_2F { x: cx, y: cy }, radiusX: radius, radiusY: radius };
        unsafe { self.rt.FillEllipse(&e, &self.brush) }
    }

    pub fn text(&self, s: &str, r: Rect, f: &IDWriteTextFormat, c: Color) {
        self.color(c);
        let t: Vec<u16> = s.encode_utf16().collect();
        unsafe { self.rt.DrawText(&t, f, &r.d2d(), &self.brush, D2D1_DRAW_TEXT_OPTIONS_CLIP, DWRITE_MEASURING_MODE_NATURAL) }
    }

    pub fn layout(&self, s: &str, f: &IDWriteTextFormat, w: f32, h: f32) -> Option<IDWriteTextLayout> {
        let t: Vec<u16> = s.encode_utf16().collect();
        unsafe { self.dw.CreateTextLayout(&t, f, w.max(1.0), h.max(1.0)).ok() }
    }

    pub fn draw_layout(&self, l: &IDWriteTextLayout, x: f32, y: f32, c: Color) {
        self.color(c);
        unsafe { self.rt.DrawTextLayout(D2D_POINT_2F { x, y }, l, &self.brush, D2D1_DRAW_TEXT_OPTIONS_NONE) }
    }

    pub fn clip(&self, r: Rect) {
        unsafe { self.rt.PushAxisAlignedClip(&r.d2d(), D2D1_ANTIALIAS_MODE_ALIASED) }
    }

    pub fn unclip(&self) {
        unsafe { self.rt.PopAxisAlignedClip() }
    }

    pub fn bitmap(&self, p: &Pixels) -> Option<ID2D1Bitmap> {
        let props = D2D1_BITMAP_PROPERTIES {
            pixelFormat: D2D1_PIXEL_FORMAT { format: DXGI_FORMAT_B8G8R8A8_UNORM, alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED },
            dpiX: 96.0,
            dpiY: 96.0,
        };
        unsafe { self.rt.CreateBitmap(D2D_SIZE_U { width: p.w, height: p.h }, Some(p.bgra.as_ptr() as *const _), p.w * 4, &props).ok() }
    }

    pub fn image(&self, b: &ID2D1Bitmap, r: Rect) {
        unsafe { self.rt.DrawBitmap(b, Some(&r.d2d()), self.alpha.get(), D2D1_BITMAP_INTERPOLATION_MODE_LINEAR, None) }
    }
}
