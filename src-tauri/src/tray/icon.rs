//! Tray icon artwork: SVG generation (pure) and rasterisation.
//!
//! The icon is built at runtime from the brand marks in `assets/brand`. Only
//! the halo arc (and, in the Halloween season, its colour) is changed; no
//! other shape is touched (docs/DESIGN.md, "Logo and halo").

use std::f64::consts::PI;

use resvg::{tiny_skia, usvg};

use crate::contracts::app::Season;

const MARK_COLOUR: &str = include_str!("../../../assets/brand/tokenmonk-mark.svg");
const MARK_MONO: &str = include_str!("../../../assets/brand/tokenmonk-mark-mono.svg");

/// The halo circle of the brand SVGs: radius 54 in a 128 x 128 view box.
const HALO_RADIUS: f64 = 54.0;
/// Halo placeholder in the brand files (a 72 % sample arc) with its caps.
const HALO_SAMPLE: &str = r#"stroke-linecap="round" stroke-dasharray="244 340""#;
/// Saffron halo stroke in the colour mark, and the pumpkin that replaces it.
const SAFFRON_STROKE: &str = r##"stroke="#F5A142""##;
const PUMPKIN_STROKE: &str = r##"stroke="#FF8A2B""##;

/// Edge length of the rendered icon in pixels (square). Large enough for
/// 2x displays; the OS scales it down to 16 to 22 points.
pub const ICON_SIZE: u32 = 64;

/// Which artwork to draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    /// Full colour mark on its own disc (`tokenmonk-mark.svg`).
    Colour,
    /// Single black ink mark (`tokenmonk-mark-mono.svg`), for template use.
    Mono,
}

/// Complete look of the tray icon for one platform and season.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IconStyle {
    pub mark: Mark,
    /// Halo drawn in pumpkin instead of saffron (colour mark only).
    pub pumpkin: bool,
    /// Hand the image to the OS as a template (macOS menu bar tints it).
    pub template: bool,
}

impl IconStyle {
    /// Picks the style.
    ///
    /// - macOS, regular season: mono mark as template image.
    /// - Halloween: colour mark with a pumpkin halo. A template image cannot
    ///   carry colour, so on macOS the template flag is dropped for the season.
    /// - Other platforms: colour mark (its own disc keeps it readable on any
    ///   taskbar theme).
    pub fn pick(macos: bool, season: Season) -> Self {
        match (macos, season) {
            (true, Season::Regular) => Self {
                mark: Mark::Mono,
                pumpkin: false,
                template: true,
            },
            (_, Season::Halloween) => Self {
                mark: Mark::Colour,
                pumpkin: true,
                template: false,
            },
            (false, Season::Regular) => Self {
                mark: Mark::Colour,
                pumpkin: false,
                template: false,
            },
        }
    }

    /// Style for the platform this build runs on.
    pub fn for_current_platform(season: Season) -> Self {
        Self::pick(cfg!(target_os = "macos"), season)
    }
}

/// Length of the visible arc in the `stroke-dasharray` unit for `percent`.
///
/// `None`, NaN and values at or below zero give `0.0` (track only); values
/// above 100 are drawn as a full ring.
pub fn arc_length(percent: Option<f64>) -> f64 {
    let circumference = 2.0 * PI * HALO_RADIUS;
    match percent {
        Some(p) if p.is_finite() && p > 0.0 => circumference * p.min(100.0) / 100.0,
        _ => 0.0,
    }
}

/// SVG text for the tray icon with the halo filled to `percent`
/// (clockwise from 12 o'clock). `None` draws the track only.
pub fn halo_svg(style: IconStyle, percent: Option<f64>) -> String {
    let base = match style.mark {
        Mark::Colour => MARK_COLOUR,
        Mark::Mono => MARK_MONO,
    };
    let arc = arc_length(percent);
    let halo = if arc > 0.0 {
        format!(r#"stroke-linecap="round" stroke-dasharray="{arc:.2} 340""#)
    } else {
        // Zero length with round caps would still paint a dot.
        r#"stroke-linecap="butt" stroke-dasharray="0 340""#.to_string()
    };
    let svg = base.replace(HALO_SAMPLE, &halo);
    if style.pumpkin && style.mark == Mark::Colour {
        svg.replace(SAFFRON_STROKE, PUMPKIN_STROKE)
    } else {
        svg
    }
}

/// Rasterises `svg` to straight (non-premultiplied) RGBA, `size` x `size`.
pub fn render_rgba(svg: &str, size: u32) -> Result<Vec<u8>, String> {
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).map_err(|e| e.to_string())?;
    let mut pixmap =
        tiny_skia::Pixmap::new(size, size).ok_or_else(|| "empty pixmap".to_string())?;
    let tree_size = tree.size();
    let transform = tiny_skia::Transform::from_scale(
        size as f32 / tree_size.width(),
        size as f32 / tree_size.height(),
    );
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    let mut data = pixmap.take();
    for px in data.as_chunks_mut::<4>().0 {
        let a = u32::from(px[3]);
        if a != 0 && a != 255 {
            for c in &mut px[..3] {
                *c = ((u32::from(*c) * 255 + a / 2) / a).min(255) as u8;
            }
        }
    }
    Ok(data)
}

/// Renders the tray icon for `style` and `percent` at [`ICON_SIZE`].
pub fn render_icon(style: IconStyle, percent: Option<f64>) -> Result<Vec<u8>, String> {
    render_rgba(&halo_svg(style, percent), ICON_SIZE)
}

#[cfg(test)]
mod tests {
    use super::*;

    const REGULAR_COLOUR: IconStyle = IconStyle {
        mark: Mark::Colour,
        pumpkin: false,
        template: false,
    };

    fn pixel(rgba: &[u8], x: u32, y: u32) -> [u8; 4] {
        let i = ((y * ICON_SIZE + x) * 4) as usize;
        [rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]]
    }

    fn near(px: [u8; 4], rgb: [u8; 3]) -> bool {
        (0..3).all(|i| px[i].abs_diff(rgb[i]) <= 8) && px[3] > 240
    }

    #[test]
    fn brand_files_contain_the_markers_we_replace() {
        assert!(MARK_COLOUR.contains(HALO_SAMPLE));
        assert!(MARK_MONO.contains(HALO_SAMPLE));
        assert!(MARK_COLOUR.contains(SAFFRON_STROKE));
    }

    #[test]
    fn arc_length_follows_percent() {
        let full = 2.0 * PI * 54.0;
        assert_eq!(arc_length(None), 0.0);
        assert_eq!(arc_length(Some(0.0)), 0.0);
        assert_eq!(arc_length(Some(-5.0)), 0.0);
        assert_eq!(arc_length(Some(f64::NAN)), 0.0);
        assert!((arc_length(Some(50.0)) - full / 2.0).abs() < 1e-9);
        assert!((arc_length(Some(100.0)) - full).abs() < 1e-9);
        assert!((arc_length(Some(250.0)) - full).abs() < 1e-9);
    }

    #[test]
    fn svg_carries_the_requested_arc() {
        let svg = halo_svg(REGULAR_COLOUR, Some(72.0));
        assert!(svg.contains(r#"stroke-dasharray="244.29 340""#), "{svg}");
        assert!(!svg.contains(r#"stroke-dasharray="244 340""#));
        let mono = halo_svg(IconStyle::pick(true, Season::Regular), Some(10.0));
        assert!(mono.contains(r#"stroke-dasharray="33.93 340""#), "{mono}");
    }

    #[test]
    fn empty_halo_has_no_round_cap_dot() {
        for p in [None, Some(0.0)] {
            let svg = halo_svg(REGULAR_COLOUR, p);
            assert!(svg.contains(r#"stroke-linecap="butt" stroke-dasharray="0 340""#));
        }
    }

    #[test]
    fn halloween_turns_only_the_halo_pumpkin() {
        let svg = halo_svg(IconStyle::pick(false, Season::Halloween), Some(50.0));
        assert!(svg.contains(PUMPKIN_STROKE));
        assert!(!svg.contains(SAFFRON_STROKE));
        // The monk's robe keeps its saffron fill.
        assert!(svg.contains(r##"fill="#F5A142""##));
    }

    #[test]
    fn style_per_platform_and_season() {
        let mac = IconStyle::pick(true, Season::Regular);
        assert_eq!(
            (mac.mark, mac.template, mac.pumpkin),
            (Mark::Mono, true, false)
        );
        let win = IconStyle::pick(false, Season::Regular);
        assert_eq!(
            (win.mark, win.template, win.pumpkin),
            (Mark::Colour, false, false)
        );
        for macos in [true, false] {
            let s = IconStyle::pick(macos, Season::Halloween);
            assert_eq!((s.mark, s.template, s.pumpkin), (Mark::Colour, false, true));
        }
    }

    #[test]
    fn halo_fills_clockwise_from_twelve() {
        // 50 %: the right half (3 o'clock) is filled, the left half (9 o'clock) is track.
        let rgba = render_icon(REGULAR_COLOUR, Some(50.0)).unwrap();
        assert_eq!(rgba.len(), (ICON_SIZE * ICON_SIZE * 4) as usize);
        let mid = ICON_SIZE / 2;
        let ring = (f64::from(ICON_SIZE) / 128.0 * 54.0).round() as u32;
        assert!(near(pixel(&rgba, mid + ring, mid), [0xF5, 0xA1, 0x42]));
        assert!(near(pixel(&rgba, mid - ring, mid), [0x2A, 0x26, 0x31]));
    }

    #[test]
    fn empty_halo_renders_track_only() {
        let rgba = render_icon(REGULAR_COLOUR, None).unwrap();
        let mid = ICON_SIZE / 2;
        let ring = (f64::from(ICON_SIZE) / 128.0 * 54.0).round() as u32;
        assert!(near(pixel(&rgba, mid + ring, mid), [0x2A, 0x26, 0x31]));
        // Twelve o'clock start has no round-cap dot either.
        assert!(near(pixel(&rgba, mid, mid - ring), [0x2A, 0x26, 0x31]));
    }

    #[test]
    fn mono_icon_is_black_with_alpha_only() {
        let rgba = render_icon(IconStyle::pick(true, Season::Regular), Some(60.0)).unwrap();
        assert!(rgba.as_chunks::<4>().0.iter().any(|p| p[3] > 0));
        assert!(rgba
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[3] > 0)
            .all(|p| p[..3] == [0, 0, 0]));
    }

    #[test]
    fn invalid_svg_is_an_error() {
        assert!(render_rgba("not svg", 16).is_err());
    }
}
