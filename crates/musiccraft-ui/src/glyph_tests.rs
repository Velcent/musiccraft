//! Every character the interface writes must be in its fonts, or it shows as
//! an empty box. (egui's own `has_glyph` reports false for characters that
//! live in the face holding its replacement glyph, so this reads the fonts'
//! character maps directly.)

use std::path::Path;

/// Non-ASCII characters inside string literals of the `.rs` files under
/// `dir`, with where they are, skipping comment lines.
fn literal_chars(dir: &Path, out: &mut Vec<(char, String)>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            literal_chars(&path, out);
            continue;
        }
        if path.extension().is_none_or(|e| e != "rs") || path.ends_with("glyph_tests.rs") {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        for (n, line) in text.lines().enumerate() {
            let code = line.trim_start();
            if code.starts_with("//") || code.starts_with("#[") {
                continue;
            }
            for (i, part) in line.split('"').enumerate() {
                if i % 2 == 1 {
                    for c in part.chars().filter(|c| !c.is_ascii()) {
                        out.push((c, format!("{}:{}", path.display(), n + 1)));
                    }
                }
            }
        }
    }
}

fn u16_at(data: &[u8], at: usize) -> u32 {
    data.get(at..at + 2)
        .map_or(0, |b| u32::from(u16::from_be_bytes([b[0], b[1]])))
}

fn u32_at(data: &[u8], at: usize) -> u32 {
    data.get(at..at + 4)
        .map_or(0, |b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
}

/// Whether a TrueType/OpenType font's character map has `c` (cmap formats 4
/// and 12, which cover the bundled fonts).
fn font_has(data: &[u8], c: char) -> bool {
    let code = u32::from(c);
    let tables = u16_at(data, 4) as usize;
    let Some(cmap) = (0..tables)
        .map(|i| 12 + i * 16)
        .find(|&rec| data.get(rec..rec + 4) == Some(b"cmap"))
        .map(|rec| u32_at(data, rec + 8) as usize)
    else {
        return false;
    };
    let subtables = u16_at(data, cmap + 2) as usize;
    (0..subtables).any(|i| {
        let sub = cmap + u32_at(data, cmap + 4 + i * 8 + 4) as usize;
        match u16_at(data, sub) {
            4 => {
                let segments = u16_at(data, sub + 6) as usize / 2;
                let ends = sub + 14;
                let starts = ends + segments * 2 + 2;
                let deltas = starts + segments * 2;
                let offsets = deltas + segments * 2;
                (0..segments).any(|s| {
                    let (start, end) = (u16_at(data, starts + s * 2), u16_at(data, ends + s * 2));
                    if code < start || code > end || code == 0xFFFF {
                        return false;
                    }
                    let offset = u16_at(data, offsets + s * 2) as usize;
                    if offset == 0 {
                        return true;
                    }
                    let at = offsets + s * 2 + offset + (code - start) as usize * 2;
                    u16_at(data, at) != 0
                })
            }
            12 => {
                let groups = u32_at(data, sub + 12) as usize;
                (0..groups).any(|g| {
                    let at = sub + 16 + g * 12;
                    (u32_at(data, at)..=u32_at(data, at + 4)).contains(&code)
                })
            }
            _ => false,
        }
    })
}

#[test]
fn every_interface_character_has_a_glyph() {
    let fonts = crate::theme::fonts();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut chars = Vec::new();
    literal_chars(&root.join("src"), &mut chars);
    literal_chars(&root.join("../musiccraft-app/src"), &mut chars);
    assert!(
        chars.iter().any(|(c, _)| *c == '…'),
        "the sources were read"
    );
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        let faces: Vec<&[u8]> = fonts.families[&family]
            .iter()
            .filter_map(|name| fonts.font_data.get(name))
            .map(|data| data.font.as_ref())
            .collect();
        let missing: Vec<String> = chars
            .iter()
            .filter(|(c, _)| !faces.iter().any(|face| font_has(face, *c)))
            .map(|(c, at)| format!("{c} (U+{:04X}) at {at}", u32::from(*c)))
            .collect();
        assert!(
            missing.is_empty(),
            "{family:?} has no glyph for:\n{}",
            missing.join("\n")
        );
    }
    let regular = fonts.families[&egui::FontFamily::Proportional]
        .iter()
        .filter_map(|name| fonts.font_data.get(name))
        .map(|data| data.font.as_ref())
        .collect::<Vec<_>>();
    assert!(
        !regular.iter().any(|f| font_has(f, '✕')),
        "the checker can say no"
    );
    assert!(regular.iter().any(|f| font_has(f, 'A')));
}
