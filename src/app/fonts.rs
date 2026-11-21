//! Hanzi: egui's own fonts have none, so Chinese text shows as tofu. The
//! first time the text has some, a Chinese font joins egui's as their
//! fallback: the system's on desktop, Noto Sans SC from jsDelivr on the web.

use eframe::egui;

/// Noto Sans SC, OpenType (egui reads no WOFF2), 8 MB.
#[cfg(target_arch = "wasm32")]
const NOTO_SANS_SC: &str = "https://cdn.jsdelivr.net/gh/notofonts/noto-cjk@main/Sans/SubsetOTF/SC/NotoSansSC-Regular.otf";

/// Simplified Chinese fonts the systems ship, with the face to use in a
/// collection (Noto Sans CJK: JP, KR, SC, …).
#[cfg(not(target_arch = "wasm32"))]
const SYSTEM: [(&str, u32); 8] = [
    ("/System/Library/Fonts/Hiragino Sans GB.ttc", 0),
    ("/System/Library/Fonts/STHeiti Light.ttc", 0),
    ("C:\\Windows\\Fonts\\msyh.ttc", 0),
    ("C:\\Windows\\Fonts\\simhei.ttf", 0),
    ("/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc", 2),
    ("/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc", 2),
    ("/usr/share/fonts/google-noto-cjk/NotoSansCJK-Regular.ttc", 2),
    ("/usr/share/fonts/truetype/wqy/wqy-microhei.ttc", 0),
];

#[derive(Default)]
pub struct Cjk {
    asked: bool,
    #[cfg(target_arch = "wasm32")]
    arrived: std::rc::Rc<std::cell::RefCell<Option<Vec<u8>>>>,
}

impl Cjk {
    /// Each frame, with the text being spoken: on its first hanzi, fetch the
    /// font; once it is here, give it to egui.
    pub fn ensure(&mut self, ctx: &egui::Context, text: &str) {
        #[cfg(target_arch = "wasm32")]
        if let Some(font) = self.arrived.borrow_mut().take() {
            install(ctx, font, 0);
        }
        if self.asked || !text.chars().any(is_hanzi) {
            return;
        }
        self.asked = true;
        #[cfg(not(target_arch = "wasm32"))]
        if let Some((font, index)) = SYSTEM.iter().find_map(|(p, i)| Some((std::fs::read(p).ok()?, *i))) {
            install(ctx, font, index);
        }
        #[cfg(target_arch = "wasm32")]
        {
            let (arrived, ctx) = (self.arrived.clone(), ctx.clone());
            wasm_bindgen_futures::spawn_local(async move {
                if let Some(font) = fetch(NOTO_SANS_SC).await {
                    *arrived.borrow_mut() = Some(font);
                    ctx.request_repaint();
                }
            });
        }
    }
}

fn is_hanzi(c: char) -> bool {
    matches!(c, '\u{3400}'..='\u{4dbf}' | '\u{4e00}'..='\u{9fff}' | '\u{f900}'..='\u{faff}' | '\u{20000}'..='\u{2ffff}')
}

/// The font after egui's own in both families, so Latin text keeps its look.
/// egui sets each font's glyphs from the row's top by that font's ascent,
/// centring the difference in line heights, so a font whose metrics differ
/// from Ubuntu's sits off its baseline: shifted back by that difference.
fn install(ctx: &egui::Context, font: Vec<u8>, index: u32) {
    use ab_glyph::Font;
    let mut fonts = egui::FontDefinitions::default();
    // ascent and line height as fractions of the size egui scales by
    let metrics = |f: &[u8], i| {
        let f = ab_glyph::FontRef::try_from_slice_and_index(f, i).ok()?;
        let h = f.height_unscaled();
        Some((f.ascent_unscaled() / h, (h + f.line_gap_unscaled()) / h))
    };
    let Some((ascent, height)) = metrics(&font, index) else { return };
    let latin = &fonts.font_data["Ubuntu-Light"];
    let (latin_ascent, latin_height) = metrics(&latin.font, latin.index).expect("egui's own font");
    let tweak = egui::FontTweak { y_offset_factor: latin_ascent - ascent - 0.5 * (latin_height - height), ..Default::default() };
    fonts.font_data.insert("cjk".into(), egui::FontData { index, tweak, ..egui::FontData::from_owned(font) });
    for family in fonts.families.values_mut() {
        family.push("cjk".into());
    }
    ctx.set_fonts(fonts);
}

#[cfg(target_arch = "wasm32")]
async fn fetch(url: &str) -> Option<Vec<u8>> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    let resp: web_sys::Response = JsFuture::from(web_sys::window()?.fetch_with_str(url)).await.ok()?.unchecked_into();
    if !resp.ok() {
        return None;
    }
    let buf = JsFuture::from(resp.array_buffer().ok()?).await.ok()?;
    Some(js_sys::Uint8Array::new(&buf).to_vec())
}
