//! Tray icon artwork: SVG generation (pure) and rasterisation.
//!
//! Ring, arc and the monk in the centre are drawn from the geometry in docs/DESIGN.md,
//! "Tray icon". Colours mirror `src/styles/tokens.css`.

use std::f64::consts::PI;

use resvg::{tiny_skia, usvg};

use crate::contracts::Mode;

/// Ring radius in the 24 x 24 view box.
const RING_RADIUS: f64 = 10.0;
/// Stroke width of the track.
const TRACK_WIDTH: f64 = 2.0;
/// Stroke width of the arc.
const ARC_WIDTH: f64 = 2.4;
/// Places the monk (1024 unit art, spanning y 270 to 885) so it is centred at
/// (12, 12) in the ring: about x 6.6 to 17.4, y 5.4 to 18.6.
const MONK_TRANSFORM: &str = "translate(0.992 -0.416) scale(0.0215)";
/// Path data of `assets/brand/tokenmonk-mark-mono.svg`: head with ears and
/// eyes, robe, sleeve.
const MONK_PATHS: [&str; 3] = [
    "M362.0 420.0 A150.0 150.0 0 1 1 662.0 420.0 A150.0 150.0 0 1 1 362.0 420.0 ZM332.0 432.0 A36.0 36.0 0 1 1 404.0 432.0 A36.0 36.0 0 1 1 332.0 432.0 ZM620.0 432.0 A36.0 36.0 0 1 1 692.0 432.0 A36.0 36.0 0 1 1 620.0 432.0 ZM416.0 427.5 A36.0 36.0 0 0 0 488.0 427.5 ZM536.0 427.5 A36.0 36.0 0 0 0 608.0 427.5 Z",
    "M262.0 742 C262.0 634.0 367.0 592 512 592 C555.5 592.0 595.4 595.8 629.9 604.0 L697.0 818.4 A290 290 0 0 1 262.0 742 Z",
    "M646.8 608.5 C717.6 629.5 762.0 671.8 762.0 742 A290 290 0 0 1 719.0 798.1 Z",
];
/// `--saffron`: accent of Max and before onboarding.
const SAFFRON: &str = "#e39a0b";
/// `--jade`: accent of Do it yourself.
const JADE: &str = "#1f8a64";
/// `--tray-track` and `--tray-track-opacity`.
const TRACK: &str = "#8a8d8b";
const TRACK_OPACITY: &str = "0.6";
/// Opacity of the monk while there is no limit data. Mirrors
/// `--tray-monk-nodata-opacity` in `src/styles/tokens.css`.
const MONK_NODATA_OPACITY: &str = "0.45";

/// Edge length of the rendered icon in pixels (square). Large enough for
/// 2x displays; the OS scales it down to 16 to 22 points.
pub const ICON_SIZE: u32 = 64;

/// Which artwork to draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    /// Grey head, sleeve and track; robe and arc in the accent of the mode.
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
    // Fills of head, robe and sleeve (in `MONK_PATHS` order).
    let (accent, monk_fills, track, track_opacity) = match style.mark {
        Mark::Colour => {
            let a = accent(mode);
            (a, [TRACK, a, TRACK], TRACK, TRACK_OPACITY)
        }
        Mark::Mono => ("#000000", ["#000000"; 3], "#000000", "0.4"),
    };
    let r = RING_RADIUS;
    let circumference = 2.0 * PI * r;
    let has_data = percent.is_some_and(f64::is_finite);
    let track_dash = if has_data {
        ""
    } else {
        r#" stroke-dasharray="1.5 3""#
    };
    let monk_opacity = if has_data {
        String::new()
    } else {
        format!(r#" opacity="{MONK_NODATA_OPACITY}""#)
    };
    let arc = arc_length(percent);
    let arc_svg = if arc > 0.0 {
        format!(
            r#"<circle cx="12" cy="12" r="{r}" fill="none" stroke="{accent}" stroke-width="{ARC_WIDTH}" stroke-linecap="round" stroke-dasharray="{arc:.2} {circumference:.2}" transform="rotate(-90 12 12)"/>"#
        )
    } else {
        String::new()
    };
    let monk: String = MONK_PATHS
        .iter()
        .zip(monk_fills)
        .map(|(d, fill)| format!(r#"<path d="{d}" fill="{fill}"/>"#))
        .collect();
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="24" height="24"><circle cx="12" cy="12" r="{r}" fill="none" stroke="{track}" stroke-opacity="{track_opacity}" stroke-width="{TRACK_WIDTH}"{track_dash}/>{arc_svg}<g transform="{MONK_TRANSFORM}" {monk_opacity}>{monk}</g></svg>"#
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
    const GREY_RGB: [u8; 3] = [0x8A, 0x8D, 0x8B];

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
        let full = 2.0 * PI * RING_RADIUS;
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
        assert!(svg.contains(r#"stroke-dasharray="31.42 62.83""#), "{svg}");
        assert!(svg.contains(r##"stroke="#e39a0b""##));
        assert!(svg.contains(&format!(
            r##"<path d="{}" fill="#e39a0b"/>"##,
            MONK_PATHS[1]
        )));
        for i in [0, 2] {
            assert!(svg.contains(&format!(
                r##"<path d="{}" fill="#8a8d8b"/>"##,
                MONK_PATHS[i]
            )));
        }
        assert!(!svg.contains("opacity=\"0.45\""), "full opacity with data");
        assert!(svg.contains(MONK_TRANSFORM));
        assert_eq!(svg.matches("<path").count(), 3, "three monk paths");
        assert!(!svg.contains(r#"stroke-dasharray="1.5 3""#), "solid track");
        let diy = halo_svg(COLOUR, Some(50.0), Mode::Diy);
        assert!(diy.contains(r##"stroke="#1f8a64""##));
        assert!(diy.contains(&format!(
            r##"<path d="{}" fill="#1f8a64"/>"##,
            MONK_PATHS[1]
        )));
    }

    #[test]
    fn monk_paths_match_brand_mark() {
        let svg = include_str!("../../../assets/brand/tokenmonk-mark-mono.svg");
        let ds: Vec<&str> = svg
            .split(" d=\"")
            .skip(1)
            .map(|rest| rest.split('"').next().unwrap())
            .collect();
        assert_eq!(ds, MONK_PATHS.to_vec());
    }

    #[test]
    fn no_data_gives_dotted_track_and_no_arc() {
        for p in [None, Some(f64::NAN)] {
            let svg = halo_svg(COLOUR, p, Mode::Max);
            assert!(svg.contains(r#"stroke-dasharray="1.5 3""#), "{svg}");
            assert!(!svg.contains(r#"stroke-width="2.4""#), "no arc: {svg}");
            assert!(svg.contains(r#"opacity="0.45""#), "dim monk: {svg}");
            let mono = halo_svg(IconStyle::pick(true), p, Mode::Max);
            assert!(mono.contains(r#"opacity="0.45""#), "dim mono monk");
        }
    }

    #[test]
    fn zero_left_has_solid_track_and_no_arc() {
        let svg = halo_svg(COLOUR, Some(0.0), Mode::Max);
        assert!(!svg.contains(r#"stroke-dasharray="1.5 3""#));
        assert!(!svg.contains(r#"stroke-width="2.4""#));
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
        // The robe (below the centre) carries the accent.
        assert!(near(pixel(&rgba, mid, mid + 8), SAFFRON_RGB));
    }

    #[test]
    fn diy_draws_jade() {
        let rgba = render_icon(COLOUR, Some(50.0), Mode::Diy).unwrap();
        let mid = ICON_SIZE / 2;
        assert!(near(pixel(&rgba, mid + ring_px(), mid), JADE_RGB));
        assert!(near(pixel(&rgba, mid, mid + 8), JADE_RGB));
    }

    #[test]
    fn no_data_renders_no_arc() {
        let rgba = render_icon(COLOUR, None, Mode::Max).unwrap();
        // The monk is dimmed to about 45 % alpha and there is no opaque arc.
        let mid = ICON_SIZE / 2;
        let robe = pixel(&rgba, mid, mid + 8);
        assert!(robe[3] > 90 && robe[3] < 140, "{robe:?}");
        assert!(!rgba
            .as_chunks::<4>()
            .0
            .iter()
            .any(|p| near(*p, SAFFRON_RGB)));
    }

    #[test]
    fn monk_stays_inside_the_ring_and_is_centred() {
        let limit = 8.8 * f64::from(ICON_SIZE) / 24.0;
        for p in [Some(0.0)] {
            let rgba = render_icon(COLOUR, p, Mode::Max).unwrap();
            let (mut min_y, mut max_y) = (u32::MAX, 0);
            for y in 0..ICON_SIZE {
                for x in 0..ICON_SIZE {
                    let px = pixel(&rgba, x, y);
                    if !near(px, GREY_RGB) && !near(px, SAFFRON_RGB) {
                        continue;
                    }
                    let dx = f64::from(x) + 0.5 - f64::from(ICON_SIZE) / 2.0;
                    let dy = f64::from(y) + 0.5 - f64::from(ICON_SIZE) / 2.0;
                    assert!(dx.hypot(dy) <= limit, "monk pixel outside at {x},{y}");
                    min_y = min_y.min(y);
                    max_y = max_y.max(y);
                }
            }
            let mid_y = f64::from(min_y + max_y + 1) / 2.0;
            assert!(
                (mid_y - f64::from(ICON_SIZE) / 2.0).abs() <= 1.5,
                "monk vertical middle {mid_y}"
            );
        }
    }

    #[test]
    fn mono_icon_is_black_with_alpha_only() {
        let rgba = render_icon(IconStyle::pick(true), Some(60.0), Mode::Max).unwrap();
        let mid = ICON_SIZE / 2;
        let robe = pixel(&rgba, mid, mid + 8);
        assert!(robe[3] > 240 && robe[..3] == [0, 0, 0], "{robe:?}");
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
