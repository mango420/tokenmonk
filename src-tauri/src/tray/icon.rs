//! Tray icon artwork: SVG generation (pure) and rasterisation.
//!
//! Five beads around the monk, drawn from the geometry in docs/DESIGN.md,
//! "Tray icon". Colours mirror `src/styles/tokens.css`.

use resvg::{tiny_skia, usvg};

use crate::contracts::Mode;

/// Radius of a bead in the 32 x 32 view box.
const BEAD_RADIUS: f64 = 3.2;
/// Bead centres, clockwise from 12 o'clock.
const BEADS: [(f64, f64); 5] = [
    (16.0, 4.0),
    (27.4, 12.3),
    (23.1, 25.7),
    (8.9, 25.7),
    (4.6, 12.3),
];
/// Percent of the 5-hour limit one bead stands for.
const PERCENT_PER_BEAD: f64 = 20.0;
/// Placement of the monk drawing in the view box.
const MONK_BOX: &str = r#"x="10.25" y="9.5" width="11.5" height="13" viewBox="76 76 128 146""#;
/// Robe path of `assets/brand/tokenmonk-monk-mark.svg`.
const ROBE_PATH: &str = "M82 216c2-46 26-74 58-74s56 28 58 74z";
/// Head circle (cx, cy, r) of `assets/brand/tokenmonk-monk-mark.svg`.
const HEAD_CIRCLE: (u32, u32, u32) = (140, 106, 24);
/// `--saffron`: accent of Max and before onboarding.
const SAFFRON: &str = "#e39a0b";
/// `--jade`: accent of Do it yourself.
const JADE: &str = "#1f8a64";
/// `--tray-track`: monk head and empty beads.
const TRACK: &str = "#8a8d8b";
/// `--tray-bead-empty-opacity`.
const BEAD_EMPTY_OPACITY: &str = "0.35";
/// `--tray-bead-empty-opacity-template` (macOS).
const BEAD_EMPTY_OPACITY_TEMPLATE: &str = "0.3";
/// `--tray-head-opacity-template` (macOS).
const HEAD_OPACITY_TEMPLATE: &str = "0.4";
/// Opacity of the monk while there is no limit data. Mirrors
/// `--tray-monk-nodata-opacity` in `src/styles/tokens.css`.
const MONK_NODATA_OPACITY: &str = "0.45";

/// Edge length of the rendered icon in pixels (square). Large enough for
/// 2x displays; the OS scales it down to 16 to 22 points.
pub const ICON_SIZE: u32 = 64;

/// Which artwork to draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    /// Grey head and empty beads; robe and full beads in the accent of the mode.
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

/// Number of full beads for `percent` left: `0` for `None`, NaN and values at
/// or below zero, otherwise `ceil(percent / 20)` capped at 5.
pub fn bead_count(percent: Option<f64>) -> usize {
    match percent {
        Some(p) if p.is_finite() && p > 0.0 => {
            ((p / PERCENT_PER_BEAD).ceil() as usize).min(BEADS.len())
        }
        _ => 0,
    }
}

/// SVG text for the tray icon with `percent` left shown as beads in the
/// accent of `mode`. `None` draws all beads empty and a dimmed monk.
pub fn halo_svg(style: IconStyle, percent: Option<f64>, mode: Mode) -> String {
    let (accent, head, empty_fill, empty_opacity, head_opacity) = match style.mark {
        Mark::Colour => (accent(mode), TRACK, TRACK, BEAD_EMPTY_OPACITY, "1"),
        Mark::Mono => (
            "#000000",
            "#000000",
            "#000000",
            BEAD_EMPTY_OPACITY_TEMPLATE,
            HEAD_OPACITY_TEMPLATE,
        ),
    };
    let has_data = percent.is_some_and(f64::is_finite);
    let full = bead_count(percent);
    let beads: String = BEADS
        .iter()
        .enumerate()
        .map(|(i, (x, y))| {
            if i < full {
                format!(r#"<circle cx="{x}" cy="{y}" r="{BEAD_RADIUS}" fill="{accent}"/>"#)
            } else {
                format!(
                    r#"<circle cx="{x}" cy="{y}" r="{BEAD_RADIUS}" fill="{empty_fill}" fill-opacity="{empty_opacity}"/>"#
                )
            }
        })
        .collect();
    let monk_opacity = if has_data {
        String::new()
    } else {
        format!(r#" opacity="{MONK_NODATA_OPACITY}""#)
    };
    let (cx, cy, r) = HEAD_CIRCLE;
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32" width="32" height="32">{beads}<svg {MONK_BOX}{monk_opacity}><path d="{ROBE_PATH}" fill="{accent}"/><circle cx="{cx}" cy="{cy}" r="{r}" fill="{head}" fill-opacity="{head_opacity}"/></svg></svg>"#
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
    /// Pixels per view box unit.
    const SCALE: f64 = ICON_SIZE as f64 / 32.0;

    fn pixel(rgba: &[u8], x: u32, y: u32) -> [u8; 4] {
        let i = ((y * ICON_SIZE + x) * 4) as usize;
        [rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]]
    }

    fn near(px: [u8; 4], rgb: [u8; 3]) -> bool {
        (0..3).all(|i| px[i].abs_diff(rgb[i]) <= 8) && px[3] > 240
    }

    fn bead_px(rgba: &[u8], i: usize) -> [u8; 4] {
        let (x, y) = BEADS[i];
        pixel(rgba, (x * SCALE) as u32, (y * SCALE) as u32)
    }

    /// Robe pixel below the head.
    fn robe_px(rgba: &[u8]) -> [u8; 4] {
        pixel(rgba, ICON_SIZE / 2, 39)
    }

    fn head_px(rgba: &[u8]) -> [u8; 4] {
        pixel(rgba, ICON_SIZE / 2, 24)
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
    fn bead_count_follows_percent() {
        assert_eq!(bead_count(None), 0);
        assert_eq!(bead_count(Some(f64::NAN)), 0);
        assert_eq!(bead_count(Some(-5.0)), 0);
        assert_eq!(bead_count(Some(0.0)), 0);
        assert_eq!(bead_count(Some(0.1)), 1);
        assert_eq!(bead_count(Some(1.0)), 1);
        assert_eq!(bead_count(Some(20.0)), 1);
        assert_eq!(bead_count(Some(21.0)), 2);
        assert_eq!(bead_count(Some(99.0)), 5);
        assert_eq!(bead_count(Some(100.0)), 5);
        assert_eq!(bead_count(Some(250.0)), 5);
    }

    #[test]
    fn svg_has_five_beads_and_monk_in_the_accent() {
        let svg = halo_svg(COLOUR, Some(50.0), Mode::Max);
        assert_eq!(svg.matches(r#"r="3.2""#).count(), 5, "{svg}");
        assert_eq!(
            svg.matches(r##"fill="#e39a0b"/>"##).count(),
            4,
            "3 full beads + robe"
        );
        assert_eq!(svg.matches(r#"fill-opacity="0.35""#).count(), 2);
        assert!(svg.contains(MONK_BOX));
        assert!(
            !svg.contains(r#" opacity="0.45""#),
            "full opacity with data"
        );
        let diy = halo_svg(COLOUR, Some(50.0), Mode::Diy);
        assert!(diy.contains(r##"fill="#1f8a64""##));
    }

    #[test]
    fn monk_paths_match_brand_mark() {
        let svg = include_str!("../../../assets/brand/tokenmonk-monk-mark.svg");
        assert!(svg.contains(&format!(r#"d="{ROBE_PATH}""#)));
        let (cx, cy, r) = HEAD_CIRCLE;
        assert!(svg.contains(&format!(r#"cx="{cx}" cy="{cy}" r="{r}""#)));
        assert!(svg.contains(r#"viewBox="76 76 128 146""#));
    }

    #[test]
    fn no_data_gives_all_empty_beads_and_dim_monk() {
        for p in [None, Some(f64::NAN)] {
            let svg = halo_svg(COLOUR, p, Mode::Max);
            assert_eq!(svg.matches(r#"fill-opacity="0.35""#).count(), 5, "{svg}");
            assert!(svg.contains(r#" opacity="0.45""#), "dim monk: {svg}");
            let mono = halo_svg(IconStyle::pick(true), p, Mode::Max);
            assert!(mono.contains(r#" opacity="0.45""#), "dim mono monk");
            assert_eq!(mono.matches(r#"fill-opacity="0.3""#).count(), 5);
        }
    }

    #[test]
    fn zero_left_has_all_empty_beads_and_full_monk() {
        let svg = halo_svg(COLOUR, Some(0.0), Mode::Max);
        assert_eq!(svg.matches(r#"fill-opacity="0.35""#).count(), 5);
        assert!(!svg.contains(r#" opacity="0.45""#));
    }

    #[test]
    fn style_per_platform() {
        let mac = IconStyle::pick(true);
        assert_eq!((mac.mark, mac.template), (Mark::Mono, true));
        let win = IconStyle::pick(false);
        assert_eq!((win.mark, win.template), (Mark::Colour, false));
    }

    #[test]
    fn beads_drain_from_the_end_in_the_accent() {
        // 50 % left: beads 1 to 3 full, 4 and 5 empty.
        let rgba = render_icon(COLOUR, Some(50.0), Mode::Max).unwrap();
        assert_eq!(rgba.len(), (ICON_SIZE * ICON_SIZE * 4) as usize);
        for i in 0..3 {
            assert!(near(bead_px(&rgba, i), SAFFRON_RGB), "bead {i}");
        }
        for i in 3..5 {
            let px = bead_px(&rgba, i);
            assert!(px[3] > 70 && px[3] < 110, "empty bead {i}: {px:?}");
            assert!(px[0].abs_diff(0x8A) <= 8 && px[1].abs_diff(0x8D) <= 8);
        }
        assert!(near(robe_px(&rgba), SAFFRON_RGB));
        assert!(near(head_px(&rgba), GREY_RGB));
    }

    #[test]
    fn diy_draws_jade() {
        let rgba = render_icon(COLOUR, Some(50.0), Mode::Diy).unwrap();
        assert!(near(bead_px(&rgba, 0), JADE_RGB));
        assert!(near(robe_px(&rgba), JADE_RGB));
    }

    #[test]
    fn no_data_renders_no_full_bead() {
        let rgba = render_icon(COLOUR, None, Mode::Max).unwrap();
        let robe = robe_px(&rgba);
        assert!(robe[3] > 90 && robe[3] < 140, "{robe:?}");
        assert!(!rgba
            .as_chunks::<4>()
            .0
            .iter()
            .any(|p| near(*p, SAFFRON_RGB)));
    }

    #[test]
    fn mono_icon_is_black_with_alpha_only() {
        let rgba = render_icon(IconStyle::pick(true), Some(60.0), Mode::Max).unwrap();
        let robe = robe_px(&rgba);
        assert!(robe[3] > 240 && robe[..3] == [0, 0, 0], "{robe:?}");
        let head = head_px(&rgba);
        assert!(head[3] > 90 && head[3] < 115, "head at 40 %: {head:?}");
        let empty = bead_px(&rgba, 4);
        assert!(
            empty[3] > 65 && empty[3] < 90,
            "empty bead at 30 %: {empty:?}"
        );
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
