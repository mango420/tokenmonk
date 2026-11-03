//! Tray icon artwork: SVG generation (pure) and rasterisation.
//!
//! Ring, arc and centre dot are drawn from the geometry in docs/DESIGN.md,
//! "Tray icon". Colours mirror `src/styles/tokens.css`.

use std::f64::consts::PI;

use resvg::{tiny_skia, usvg};

use crate::contracts::Mode;

/// Ring radius in the 24 x 24 view box.
const RING_RADIUS: f64 = 8.5;
/// `--saffron`: accent of Max and before onboarding.
const SAFFRON: &str = "#e39a0b";
/// `--jade`: accent of Do it yourself.
const JADE: &str = "#1f8a64";
/// `--tray-track` and `--tray-track-opacity`.
const TRACK: &str = "#8a8d8b";
const TRACK_OPACITY: &str = "0.6";

/// Edge length of the rendered icon in pixels (square). Large enough for
/// 2x displays; the OS scales it down to 16 to 22 points.
pub const ICON_SIZE: u32 = 64;

/// Which artwork to draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    /// Ring and dot in the accent of the mode, track in grey.
    Colour,
    /// Black ink only (alpha carries the shape), for template use.
    Mono,
}

/// Complete look of the tray icon for one platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IconStyle {
    pub mark: Mark,
    /// Hand the image to the OS as a template (macOS menu bar tints it).
    pub template: bool,
}

impl IconStyle {
    /// Picks the style.
    ///
    /// - macOS: mono mark as template image.
    /// - Other platforms: accent colour mark.
    pub fn pick(macos: bool) -> Self {
        if macos {
            Self {
                mark: Mark::Mono,
                template: true,
            }
        } else {
            Self {
                mark: Mark::Colour,
                template: false,
            }
        }
    }

    /// Style for the platform this build runs on.
    pub fn for_current_platform() -> Self {
        Self::pick(cfg!(target_os = "macos"))
    }
}

/// Accent colour of `mode` (saffron before onboarding).
pub fn accent(mode: Mode) -> &'static str {
    match mode {
        Mode::Diy => JADE,
        Mode::Max | Mode::NoneYet => SAFFRON,
    }
}

/// Length of the visible arc in the `stroke-dasharray` unit for `percent`
/// left.
///
/// `None`, NaN and values at or below zero give `0.0` (no arc); values
/// above 100 are drawn as a full ring.
pub fn arc_length(percent: Option<f64>) -> f64 {
    let circumference = 2.0 * PI * RING_RADIUS;
    match percent {
        Some(p) if p.is_finite() && p > 0.0 => circumference * p.min(100.0) / 100.0,
        _ => 0.0,
    }
}

/// SVG text for the tray icon with the arc at `percent` left (clockwise
/// from 12 o'clock) in the accent of `mode`. `None` draws the dotted track
/// and no arc.
pub fn halo_svg(style: IconStyle, percent: Option<f64>, mode: Mode) -> String {
    let (accent, track, track_opacity) = match style.mark {
        Mark::Colour => (accent(mode), TRACK, TRACK_OPACITY),
        Mark::Mono => ("#000000", "#000000", "0.4"),
    };
    let r = RING_RADIUS;
    let circumference = 2.0 * PI * r;
    let has_data = percent.is_some_and(f64::is_finite);
    let track_dash = if has_data {
        ""
    } else {
        r#" stroke-dasharray="1.5 3""#
    };
    let arc = arc_length(percent);
    let arc_svg = if arc > 0.0 {
        format!(
            r#"<circle cx="12" cy="12" r="{r}" fill="none" stroke="{accent}" stroke-width="2.6" stroke-linecap="round" stroke-dasharray="{arc:.2} {circumference:.2}" transform="rotate(-90 12 12)"/>"#
        )
    } else {
        String::new()
    };
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="24" height="24"><circle cx="12" cy="12" r="{r}" fill="none" stroke="{track}" stroke-opacity="{track_opacity}" stroke-width="2"{track_dash}/>{arc_svg}<circle cx="12" cy="12" r="2.2" fill="{accent}"/></svg>"#
    )
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

/// Renders the tray icon for `style`, `percent` left and `mode` at
/// [`ICON_SIZE`].
pub fn render_icon(style: IconStyle, percent: Option<f64>, mode: Mode) -> Result<Vec<u8>, String> {
    render_rgba(&halo_svg(style, percent, mode), ICON_SIZE)
}

#[cfg(test)]
mod tests {
    use super::*;

    const COLOUR: IconStyle = IconStyle {
        mark: Mark::Colour,
        template: false,
    };
    const SAFFRON_RGB: [u8; 3] = [0xE3, 0x9A, 0x0B];
    const JADE_RGB: [u8; 3] = [0x1F, 0x8A, 0x64];

    fn pixel(rgba: &[u8], x: u32, y: u32) -> [u8; 4] {
        let i = ((y * ICON_SIZE + x) * 4) as usize;
        [rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]]
    }

    fn near(px: [u8; 4], rgb: [u8; 3]) -> bool {
        (0..3).all(|i| px[i].abs_diff(rgb[i]) <= 8) && px[3] > 240
    }

    fn ring_px() -> u32 {
        (f64::from(ICON_SIZE) / 24.0 * RING_RADIUS).round() as u32
    }

    #[test]
    fn accent_follows_the_mode() {
        assert_eq!(accent(Mode::Max), "#e39a0b");
        assert_eq!(
            accent(Mode::NoneYet),
            "#e39a0b",
            "saffron before onboarding"
        );
        assert_eq!(accent(Mode::Diy), "#1f8a64");
    }

    #[test]
    fn arc_length_follows_percent() {
        let full = 2.0 * PI * 8.5;
        assert_eq!(arc_length(None), 0.0);
        assert_eq!(arc_length(Some(0.0)), 0.0);
        assert_eq!(arc_length(Some(-5.0)), 0.0);
        assert_eq!(arc_length(Some(f64::NAN)), 0.0);
        assert!((arc_length(Some(50.0)) - full / 2.0).abs() < 1e-9);
        assert!((arc_length(Some(100.0)) - full).abs() < 1e-9);
        assert!((arc_length(Some(250.0)) - full).abs() < 1e-9);
    }

    #[test]
    fn svg_carries_arc_accent_and_solid_track() {
        let svg = halo_svg(COLOUR, Some(50.0), Mode::Max);
        assert!(svg.contains(r#"stroke-dasharray="26.70 53.41""#), "{svg}");
        assert!(svg.contains(r##"stroke="#e39a0b""##));
        assert!(svg.contains(r##"fill="#e39a0b""##));
        assert!(!svg.contains(r#"stroke-dasharray="1.5 3""#), "solid track");
        let diy = halo_svg(COLOUR, Some(50.0), Mode::Diy);
        assert!(diy.contains(r##"stroke="#1f8a64""##));
        assert!(diy.contains(r##"fill="#1f8a64""##));
    }

    #[test]
    fn no_data_gives_dotted_track_and_no_arc() {
        for p in [None, Some(f64::NAN)] {
            let svg = halo_svg(COLOUR, p, Mode::Max);
            assert!(svg.contains(r#"stroke-dasharray="1.5 3""#), "{svg}");
            assert!(!svg.contains(r#"stroke-width="2.6""#), "no arc: {svg}");
        }
    }

    #[test]
    fn zero_left_has_solid_track_and_no_arc() {
        let svg = halo_svg(COLOUR, Some(0.0), Mode::Max);
        assert!(!svg.contains(r#"stroke-dasharray="1.5 3""#));
        assert!(!svg.contains(r#"stroke-width="2.6""#));
    }

    #[test]
    fn style_per_platform() {
        let mac = IconStyle::pick(true);
        assert_eq!((mac.mark, mac.template), (Mark::Mono, true));
        let win = IconStyle::pick(false);
        assert_eq!((win.mark, win.template), (Mark::Colour, false));
    }

    #[test]
    fn arc_runs_clockwise_from_twelve_in_the_accent() {
        // 50 % left: the right half (3 o'clock) is arc, the left half (9 o'clock) is track.
        let rgba = render_icon(COLOUR, Some(50.0), Mode::Max).unwrap();
        assert_eq!(rgba.len(), (ICON_SIZE * ICON_SIZE * 4) as usize);
        let mid = ICON_SIZE / 2;
        assert!(near(pixel(&rgba, mid + ring_px(), mid), SAFFRON_RGB));
        let left = pixel(&rgba, mid - ring_px(), mid);
        assert!(left[3] > 100 && left[3] < 200, "track at 60 %: {left:?}");
        assert!(left[0].abs_diff(0x8A) <= 8 && left[1].abs_diff(0x8D) <= 8);
        // The centre dot carries the accent too.
        assert!(near(pixel(&rgba, mid, mid), SAFFRON_RGB));
    }

    #[test]
    fn diy_draws_jade() {
        let rgba = render_icon(COLOUR, Some(50.0), Mode::Diy).unwrap();
        let mid = ICON_SIZE / 2;
        assert!(near(pixel(&rgba, mid + ring_px(), mid), JADE_RGB));
        assert!(near(pixel(&rgba, mid, mid), JADE_RGB));
    }

    #[test]
    fn no_data_renders_no_arc() {
        let rgba = render_icon(COLOUR, None, Mode::Max).unwrap();
        let accent_px = rgba
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| near(**p, SAFFRON_RGB))
            .count();
        // Only the centre dot (r 2.2 at 64 px is about 100 px) is in the accent.
        assert!(accent_px > 40 && accent_px < 200, "{accent_px}");
    }

    #[test]
    fn mono_icon_is_black_with_alpha_only() {
        let rgba = render_icon(IconStyle::pick(true), Some(60.0), Mode::Max).unwrap();
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
