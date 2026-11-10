//! Puts an app icon and a manifest into OnionLauncher.exe with no resource
//! compiler (the GNU toolchain has none here): a COFF object holding a
//! `.rsrc` section, which the linker takes as it is.
//!
//! The manifest makes the app per-monitor DPI aware (sharp on every screen)
//! and asks for the modern common-controls library (themed menus/dialogs).

use std::path::Path;

const RT_ICON: u32 = 3;
const RT_GROUP_ICON: u32 = 14;
const RT_MANIFEST: u32 = 24;
const LANG_EN_US: u32 = 0x409;

const MANIFEST: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <dependency><dependentAssembly>
    <assemblyIdentity type="win32" name="Microsoft.Windows.Common-Controls" version="6.0.0.0"
      processorArchitecture="*" publicKeyToken="6595b64144ccf1df" language="*"/>
  </dependentAssembly></dependency>
  <application xmlns="urn:schemas-microsoft-com:asm.v3"><windowsSettings>
    <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true/pm</dpiAware>
    <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2</dpiAwareness>
  </windowsSettings></application>
</assembly>
"#;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let sizes = [16u32, 24, 32, 48, 64];
    let images: Vec<Vec<u8>> = sizes.iter().map(|&s| ico_image(s)).collect();

    // group icon: ICONDIR header + one 14-byte GRPICONDIRENTRY per image
    let mut group = vec![];
    put16(&mut group, 0);
    put16(&mut group, 1);
    put16(&mut group, sizes.len() as u16);
    let mut items: Vec<(u32, u32, Vec<u8>)> = vec![];
    for (i, (&s, img)) in sizes.iter().zip(&images).enumerate() {
        group.push(s as u8);
        group.push(s as u8);
        group.push(0); // colours
        group.push(0);
        put16(&mut group, 1); // planes
        put16(&mut group, 32); // bits
        put32(&mut group, img.len() as u32);
        put16(&mut group, i as u16 + 1);
        items.push((RT_ICON, i as u32 + 1, img.clone()));
    }
    items.push((RT_GROUP_ICON, 1, group));
    items.push((RT_MANIFEST, 1, MANIFEST.as_bytes().to_vec()));

    let obj = Path::new(&std::env::var("OUT_DIR").unwrap()).join("res.o");
    std::fs::write(&obj, coff(&rsrc(&items))).unwrap();
    println!("cargo:rustc-link-arg-bins={}", obj.display());
}

/// Same picture as the tray icon: orange rounded square, 2x2 white tiles.
fn rgba(size: u32) -> Vec<u8> {
    let s = size as f32;
    let mut px = vec![0u8; (size * size * 4) as usize];
    let r = s * 0.22;
    for y in 0..size {
        for x in 0..size {
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            let dx = (r - fx).max(fx - (s - r)).max(0.0);
            let dy = (r - fy).max(fy - (s - r)).max(0.0);
            let a = (r - (dx * dx + dy * dy).sqrt() + 0.5).clamp(0.0, 1.0);
            if a <= 0.0 {
                continue;
            }
            let m = s * 0.2;
            let g = s * 0.08;
            let in_tile = |v: f32| (v > m && v < s / 2.0 - g / 2.0) || (v > s / 2.0 + g / 2.0 && v < s - m);
            let c = if in_tile(fx) && in_tile(fy) { [255, 255, 255] } else { [242, 140, 40] };
            let i = ((y * size + x) * 4) as usize;
            px[i..i + 4].copy_from_slice(&[c[0], c[1], c[2], (a * 255.0) as u8]);
        }
    }
    px
}

/// One icon image in .ico/.rsrc form: BITMAPINFOHEADER (double height),
/// bottom-up BGRA rows, then an all-zero AND mask.
fn ico_image(size: u32) -> Vec<u8> {
    let px = rgba(size);
    let mut v = vec![];
    put32(&mut v, 40);
    put32(&mut v, size);
    put32(&mut v, size * 2);
    put16(&mut v, 1);
    put16(&mut v, 32);
    put32(&mut v, 0);
    put32(&mut v, size * size * 4);
    v.extend_from_slice(&[0; 16]);
    for y in (0..size).rev() {
        for x in 0..size {
            let i = ((y * size + x) * 4) as usize;
            v.extend_from_slice(&[px[i + 2], px[i + 1], px[i], px[i + 3]]);
        }
    }
    let mask_row = (size.div_ceil(32) * 4) as usize;
    v.extend(std::iter::repeat_n(0, mask_row * size as usize));
    v
}

fn put16(v: &mut Vec<u8>, x: u16) {
    v.extend_from_slice(&x.to_le_bytes());
}

fn put32(v: &mut Vec<u8>, x: u32) {
    v.extend_from_slice(&x.to_le_bytes());
}

/// The resource section: a tree type -> id -> language -> data entry.
/// Returns the section and where its data addresses are.
fn rsrc(items: &[(u32, u32, Vec<u8>)]) -> (Vec<u8>, Vec<u32>) {
    let dir = |entries: usize| 16 + 8 * entries;
    let types = [RT_ICON, RT_GROUP_ICON, RT_MANIFEST]; // ascending, as Windows requires
    let of_type = |t: u32| items.iter().filter(move |it| it.0 == t).count();
    let mut at = dir(types.len());
    let mut take = |n: usize, align: usize| {
        at = at.next_multiple_of(align) + n;
        at - n
    };
    let type_dirs: Vec<usize> = types.iter().map(|&t| take(dir(of_type(t)), 1)).collect();
    let lang_dirs: Vec<usize> = items.iter().map(|_| take(dir(1), 1)).collect();
    let entries: Vec<usize> = items.iter().map(|_| take(16, 1)).collect();
    let datas: Vec<usize> = items.iter().map(|it| take(it.2.len(), 8)).collect();

    let mut s = vec![];
    let head = |s: &mut Vec<u8>, n: usize| {
        s.extend_from_slice(&[0; 12]);
        put16(s, 0);
        put16(s, n as u16);
    };
    let link = |s: &mut Vec<u8>, id: u32, to: usize, table: bool| {
        put32(s, id);
        put32(s, to as u32 | if table { 0x8000_0000 } else { 0 });
    };
    head(&mut s, types.len());
    for (&t, &d) in types.iter().zip(&type_dirs) {
        link(&mut s, t, d, true);
    }
    for &t in &types {
        head(&mut s, of_type(t));
        for (i, it) in items.iter().enumerate().filter(|(_, it)| it.0 == t) {
            link(&mut s, it.1, lang_dirs[i], true);
        }
    }
    for &e in &entries {
        head(&mut s, 1);
        link(&mut s, LANG_EN_US, e, false);
    }
    let mut fixups = vec![];
    for (it, &d) in items.iter().zip(&datas) {
        fixups.push(s.len() as u32);
        put32(&mut s, d as u32);
        put32(&mut s, it.2.len() as u32);
        put32(&mut s, 0);
        put32(&mut s, 0);
    }
    for (it, &d) in items.iter().zip(&datas) {
        s.resize(d, 0);
        s.extend_from_slice(&it.2);
    }
    (s, fixups)
}

/// A one-section COFF object (x64) whose fixups point at the section itself.
fn coff((data, fixups): &(Vec<u8>, Vec<u32>)) -> Vec<u8> {
    let relocs = 20 + 40 + data.len() as u32;
    let symbols = relocs + 10 * fixups.len() as u32;
    let mut o = vec![];
    put16(&mut o, 0x8664);
    put16(&mut o, 1);
    put32(&mut o, 0);
    put32(&mut o, symbols);
    put32(&mut o, 2);
    put16(&mut o, 0);
    put16(&mut o, 0);

    o.extend_from_slice(b".rsrc\0\0\0");
    put32(&mut o, 0);
    put32(&mut o, 0);
    put32(&mut o, data.len() as u32);
    put32(&mut o, 20 + 40);
    put32(&mut o, relocs);
    put32(&mut o, 0);
    put16(&mut o, fixups.len() as u16);
    put16(&mut o, 0);
    put32(&mut o, 0x4000_0040);

    o.extend_from_slice(data);
    for &f in fixups {
        put32(&mut o, f);
        put32(&mut o, 0);
        put16(&mut o, 3); // IMAGE_REL_AMD64_ADDR32NB
    }

    o.extend_from_slice(b".rsrc\0\0\0");
    put32(&mut o, 0);
    put16(&mut o, 1);
    put16(&mut o, 0);
    o.push(3);
    o.push(1);
    put32(&mut o, data.len() as u32);
    put16(&mut o, fixups.len() as u16);
    o.extend_from_slice(&[0; 12]);
    put32(&mut o, 4);
    o
}
