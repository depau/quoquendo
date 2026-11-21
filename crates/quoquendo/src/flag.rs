//! Language flags as shapes in a [`W`]×[`H`] box, so every front end paints
//! them at its own size: egui on the desktop and the web, Compose on Android.

/// One shape of a flag, painted in order; colours are `0xRRGGBB`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shape {
    /// x, y, width, height
    Rect(f32, f32, f32, f32, u32),
    /// a convex polygon
    Poly(&'static [(f32, f32)], u32),
    /// centre x, centre y, radius
    Circle(f32, f32, f32, u32),
    /// a five-pointed star: centre x, centre y, outer radius
    Star(f32, f32, f32, u32),
}

pub const W: f32 = 30.0;
pub const H: f32 = 21.0;

/// A star's inner radius, as a share of its outer one.
pub const STAR_INNER: f32 = 0.382;

/// The flag for a catalog language (`US English`) or a voice's MotherTongue
/// (`EnglishGb`, `Italian`); None for one no flag stands for (`SpanishAm`).
pub fn of(language: &str) -> Option<&'static [Shape]> {
    let key: String = language.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_lowercase();
    Some(match key.as_str() {
        "italian" => IT,
        "usenglish" | "englishus" => US,
        "ukenglish" | "englishgb" => GB,
        "australianenglish" | "englishau" => AU,
        "german" => DE,
        "french" => FR,
        "europeanspanish" | "spanishes" => ES,
        "colombiaspanish" => CO,
        "mexicanspanish" => MX,
        "argentinespanish" => AR,
        "chileanspanish" => CL,
        "europeanportuguese" | "portuguesept" => PT,
        "brazilianportuguese" | "portuguesebr" => BR,
        "catalan" | "catalance" => CAT,
        "valenciancatalan" | "catalanva" => VALENCIA,
        "galician" => GL,
        "dutch" => NL,
        "swedish" => SE,
        "danish" => DK,
        "polish" => PL,
        "russian" => RU,
        "greek" => GR,
        "chinese" => CN,
        _ => return None,
    })
}

use Shape::*;

const WHITE: u32 = 0xffffff;

const IT: &[Shape] = &[Rect(0.0, 0.0, 10.0, 21.0, 0x009246), Rect(10.0, 0.0, 10.0, 21.0, 0xf1f2f1), Rect(20.0, 0.0, 10.0, 21.0, 0xce2b37)];
const FR: &[Shape] = &[Rect(0.0, 0.0, 10.0, 21.0, 0x0055a4), Rect(10.0, 0.0, 10.0, 21.0, WHITE), Rect(20.0, 0.0, 10.0, 21.0, 0xef4135)];
const MX: &[Shape] = &[
    Rect(0.0, 0.0, 10.0, 21.0, 0x006847),
    Rect(10.0, 0.0, 10.0, 21.0, WHITE),
    Rect(20.0, 0.0, 10.0, 21.0, 0xce1126),
    Circle(15.0, 10.5, 2.6, 0x8c5a2b),
];
const DE: &[Shape] = &[Rect(0.0, 0.0, 30.0, 7.0, 0x000000), Rect(0.0, 7.0, 30.0, 7.0, 0xdd0000), Rect(0.0, 14.0, 30.0, 7.0, 0xffce00)];
const NL: &[Shape] = &[Rect(0.0, 0.0, 30.0, 7.0, 0xae1c28), Rect(0.0, 7.0, 30.0, 7.0, WHITE), Rect(0.0, 14.0, 30.0, 7.0, 0x21468b)];
const RU: &[Shape] = &[Rect(0.0, 0.0, 30.0, 7.0, WHITE), Rect(0.0, 7.0, 30.0, 7.0, 0x0039a6), Rect(0.0, 14.0, 30.0, 7.0, 0xd52b1e)];
const PL: &[Shape] = &[Rect(0.0, 0.0, 30.0, 10.5, WHITE), Rect(0.0, 10.5, 30.0, 10.5, 0xdc143c)];
const CO: &[Shape] = &[Rect(0.0, 0.0, 30.0, 10.5, 0xffd100), Rect(0.0, 10.5, 30.0, 5.25, 0x003893), Rect(0.0, 15.75, 30.0, 5.25, 0xce1126)];
const ES: &[Shape] = &[Rect(0.0, 0.0, 30.0, 21.0, 0xaa1518), Rect(0.0, 5.25, 30.0, 10.5, 0xf1bf00), Rect(6.0, 8.2, 4.6, 4.6, 0xc8102e)];
const AR: &[Shape] = &[Rect(0.0, 0.0, 30.0, 21.0, 0x74acdf), Rect(0.0, 7.0, 30.0, 7.0, WHITE), Circle(15.0, 10.5, 2.2, 0xf6b40e)];
const CL: &[Shape] = &[
    Rect(0.0, 0.0, 30.0, 10.5, WHITE),
    Rect(0.0, 10.5, 30.0, 10.5, 0xd52b1e),
    Rect(0.0, 0.0, 10.5, 10.5, 0x0039a6),
    Star(5.25, 5.4, 2.8, WHITE),
];
const PT: &[Shape] = &[
    Rect(0.0, 0.0, 30.0, 21.0, 0xff0000),
    Rect(0.0, 0.0, 12.0, 21.0, 0x006600),
    Circle(12.0, 10.5, 3.6, 0xffe000),
    Circle(12.0, 10.5, 2.2, 0xff0000),
    Circle(12.0, 10.5, 1.1, WHITE),
];
const BR: &[Shape] = &[
    Rect(0.0, 0.0, 30.0, 21.0, 0x009c3b),
    Poly(&[(15.0, 2.2), (27.6, 10.5), (15.0, 18.8), (2.4, 10.5)], 0xffdf00),
    Circle(15.0, 10.5, 4.2, 0x002776),
];
/// the Senyera: nine stripes, yellow first
const CAT: &[Shape] = &[
    Rect(0.0, 0.0, 30.0, 21.0, 0xfcdd09),
    Rect(0.0, 2.333, 30.0, 2.333, 0xda121a),
    Rect(0.0, 7.0, 30.0, 2.333, 0xda121a),
    Rect(0.0, 11.667, 30.0, 2.333, 0xda121a),
    Rect(0.0, 16.333, 30.0, 2.333, 0xda121a),
];
/// the Senyera with the blue band at the hoist (without its crown)
const VALENCIA: &[Shape] = &[
    Rect(0.0, 0.0, 30.0, 21.0, 0xfcdd09),
    Rect(0.0, 2.333, 30.0, 2.333, 0xda121a),
    Rect(0.0, 7.0, 30.0, 2.333, 0xda121a),
    Rect(0.0, 11.667, 30.0, 2.333, 0xda121a),
    Rect(0.0, 16.333, 30.0, 2.333, 0xda121a),
    Rect(0.0, 0.0, 7.0, 21.0, 0x0072bc),
];
const GL: &[Shape] = &[Rect(0.0, 0.0, 30.0, 21.0, 0xf1f5f9), Poly(&[(0.0, 0.0), (3.7, 0.0), (30.0, 18.4), (30.0, 21.0), (26.3, 21.0), (0.0, 2.6)], 0x0090d4)];
const SE: &[Shape] = &[Rect(0.0, 0.0, 30.0, 21.0, 0x006aa7), Rect(9.4, 0.0, 3.75, 21.0, 0xfecc00), Rect(0.0, 8.4, 30.0, 4.2, 0xfecc00)];
const DK: &[Shape] = &[Rect(0.0, 0.0, 30.0, 21.0, 0xc8102e), Rect(9.7, 0.0, 3.25, 21.0, WHITE), Rect(0.0, 9.0, 30.0, 3.0, WHITE)];
const GR: &[Shape] = &[
    Rect(0.0, 0.0, 30.0, 21.0, 0x0d5eaf),
    Rect(0.0, 2.333, 30.0, 2.333, WHITE),
    Rect(0.0, 7.0, 30.0, 2.333, WHITE),
    Rect(0.0, 11.667, 30.0, 2.333, WHITE),
    Rect(0.0, 16.333, 30.0, 2.333, WHITE),
    Rect(0.0, 0.0, 11.667, 11.667, 0x0d5eaf),
    Rect(4.667, 0.0, 2.333, 11.667, WHITE),
    Rect(0.0, 4.667, 11.667, 2.333, WHITE),
];
const CN: &[Shape] = &[Rect(0.0, 0.0, 30.0, 21.0, 0xde2910), Star(5.0, 5.25, 3.2, 0xffde00)];

const UK_BLUE: u32 = 0x012169;
const UK_RED: u32 = 0xc8102e;
/// the Union Flag; its diagonals are polygons clipped to the flag
const GB: &[Shape] = &[
    Rect(0.0, 0.0, 30.0, 21.0, UK_BLUE),
    Poly(&[(0.0, 0.0), (4.36, 0.0), (30.0, 17.95), (30.0, 21.0), (25.64, 21.0), (0.0, 3.05)], WHITE),
    Poly(&[(30.0, 0.0), (30.0, 3.05), (4.36, 21.0), (0.0, 21.0), (0.0, 17.95), (25.64, 0.0)], WHITE),
    Poly(&[(0.0, 0.0), (2.18, 0.0), (30.0, 19.47), (30.0, 21.0), (27.82, 21.0), (0.0, 1.53)], UK_RED),
    Poly(&[(30.0, 0.0), (30.0, 1.53), (2.18, 21.0), (0.0, 21.0), (0.0, 19.47), (27.82, 0.0)], UK_RED),
    Rect(11.0, 0.0, 8.0, 21.0, WHITE),
    Rect(0.0, 6.5, 30.0, 8.0, WHITE),
    Rect(12.5, 0.0, 5.0, 21.0, UK_RED),
    Rect(0.0, 8.0, 30.0, 5.0, UK_RED),
];
/// the Union Flag in the canton, the Commonwealth Star, the Southern Cross
const AU: &[Shape] = &[
    Rect(0.0, 0.0, 30.0, 21.0, 0x00247d),
    Poly(&[(0.0, 0.0), (2.18, 0.0), (15.0, 8.97), (15.0, 10.5), (12.82, 10.5), (0.0, 1.53)], WHITE),
    Poly(&[(15.0, 0.0), (15.0, 1.53), (2.18, 10.5), (0.0, 10.5), (0.0, 8.97), (12.82, 0.0)], WHITE),
    Poly(&[(0.0, 0.0), (1.09, 0.0), (15.0, 9.74), (15.0, 10.5), (13.91, 10.5), (0.0, 0.76)], UK_RED),
    Poly(&[(15.0, 0.0), (15.0, 0.76), (1.09, 10.5), (0.0, 10.5), (0.0, 9.74), (13.91, 0.0)], UK_RED),
    Rect(5.5, 0.0, 4.0, 10.5, WHITE),
    Rect(0.0, 3.25, 15.0, 4.0, WHITE),
    Rect(6.25, 0.0, 2.5, 10.5, UK_RED),
    Rect(0.0, 4.0, 15.0, 2.5, UK_RED),
    Star(7.5, 15.75, 2.6, WHITE),
    Star(22.5, 4.0, 1.4, WHITE),
    Star(26.5, 9.0, 1.4, WHITE),
    Star(18.8, 10.5, 1.4, WHITE),
    Star(22.5, 17.0, 1.4, WHITE),
];
const US_RED: u32 = 0xb22234;
const US: &[Shape] = &[
    Rect(0.0, 0.0, 30.0, 21.0, WHITE),
    Rect(0.0, 0.0, 30.0, 1.615, US_RED),
    Rect(0.0, 3.231, 30.0, 1.615, US_RED),
    Rect(0.0, 6.462, 30.0, 1.615, US_RED),
    Rect(0.0, 9.692, 30.0, 1.615, US_RED),
    Rect(0.0, 12.923, 30.0, 1.615, US_RED),
    Rect(0.0, 16.154, 30.0, 1.615, US_RED),
    Rect(0.0, 19.385, 30.0, 1.615, US_RED),
    Rect(0.0, 0.0, 13.0, 11.308, 0x3c3b6e),
    Circle(2.6, 2.4, 0.8, WHITE),
    Circle(6.5, 2.4, 0.8, WHITE),
    Circle(10.4, 2.4, 0.8, WHITE),
    Circle(4.55, 5.65, 0.8, WHITE),
    Circle(8.45, 5.65, 0.8, WHITE),
    Circle(2.6, 8.9, 0.8, WHITE),
    Circle(6.5, 8.9, 0.8, WHITE),
    Circle(10.4, 8.9, 0.8, WHITE),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_catalog_language_has_a_flag() {
        for i in crate::catalog::INSTALLERS {
            assert!(of(i.language).is_some(), "no flag for {}", i.language);
        }
        assert_eq!(of("EnglishGb"), Some(GB));
        assert_eq!(of("SpanishAm"), None);
    }
}
