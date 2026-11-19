// src/core/develop.rs
// Non-destructive parametric develop pipeline.
//
// `DevelopParams` describes the adjustments; `apply_develop` bakes them onto
// a decoded image.  The same function is used for both the live preview
// (applied to the 400 px thumbnail) and the final export (applied to the
// full-resolution decoded image), so the look is always identical.
//
// Pipeline order (standard photography practice):
//   White Balance → Exposure → Highlights/Shadows → Whites/Blacks
//   → Gamma-encode → Contrast → Clarity → Vibrance/Saturation
//   → Vignette → Grain → B&W

use image::{imageops, DynamicImage, RgbImage};
use serde::{Deserialize, Serialize};

// ── Geometry ──────────────────────────────────────────────────────────────────

/// Normalized crop rectangle (0..1), relative to the rotated frame.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CropRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

// ── Parameter struct ──────────────────────────────────────────────────────────

/// All adjustable develop parameters for a single image.
/// Default is the identity — applying default params returns the original.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DevelopParams {
    // ── Geometry ──────────────────────────────────────────────────────────────
    /// Clockwise rotation in degrees (0, 90, 180, 270).
    pub rotation: i16,
    /// Crop rectangle, normalized to the rotated frame. `None` = full frame.
    pub crop: Option<CropRect>,

    // ── Basic ─────────────────────────────────────────────────────────────────
    /// Exposure in EV stops (-5.0 … +5.0).
    pub exposure: f32,
    /// Global contrast S-curve (-100 … +100).
    pub contrast: f32,
    /// Highlight recovery/boost (-100 … +100; negative = recover).
    pub highlights: f32,
    /// Shadow lift/crush (-100 … +100; positive = lift).
    pub shadows: f32,
    /// White-point adjustment (-100 … +100).
    pub whites: f32,
    /// Black-point adjustment (-100 … +100; positive = lift blacks).
    pub blacks: f32,

    // ── Presence ──────────────────────────────────────────────────────────────
    /// Mid-frequency local contrast (unsharp-mask style) (-100 … +100).
    pub clarity: f32,
    /// Smart saturation — boosts under-saturated hues first (-100 … +100).
    pub vibrance: f32,
    /// Global HSL saturation (-100 … +100).
    pub saturation: f32,

    // ── White Balance ─────────────────────────────────────────────────────────
    /// Colour-temperature shift (-100 = cooler, +100 = warmer).
    pub temperature: f32,
    /// Green/Magenta tint (-100 = magenta, +100 = green).
    pub tint: f32,

    // ── Effects ───────────────────────────────────────────────────────────────
    /// Radial edge darkening (-100 = strong dark vignette, +100 = bright).
    pub vignette: f32,
    /// Film-grain amount (0 … 100).
    pub grain: f32,

    // ── B&W ───────────────────────────────────────────────────────────────────
    pub bw_enabled: bool,
    /// Per-channel luminosity weights for B&W mix (-100 … +100).
    pub bw_red: f32,
    pub bw_orange: f32,
    pub bw_yellow: f32,
    pub bw_green: f32,
    pub bw_aqua: f32,
    pub bw_blue: f32,
}

impl Default for DevelopParams {
    fn default() -> Self {
        Self {
            rotation: 0,
            crop: None,
            exposure: 0.0,
            contrast: 0.0,
            highlights: 0.0,
            shadows: 0.0,
            whites: 0.0,
            blacks: 0.0,
            clarity: 0.0,
            vibrance: 0.0,
            saturation: 0.0,
            temperature: 0.0,
            tint: 0.0,
            vignette: 0.0,
            grain: 0.0,
            bw_enabled: false,
            bw_red: 0.0,
            bw_orange: 0.0,
            bw_yellow: 0.0,
            bw_green: 0.0,
            bw_aqua: 0.0,
            bw_blue: 0.0,
        }
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

/// Apply all develop adjustments to `img` and return the processed image.
pub fn apply_develop(img: &DynamicImage, p: &DevelopParams) -> DynamicImage {
    let geo = apply_geometry(img, p.rotation, p.crop);
    let src = geo.to_rgb8();
    let (w, h) = src.dimensions();

    // Pre-compute the Gaussian blur for Clarity in one pass (skip if unused).
    let clarity_blur: Option<RgbImage> = if p.clarity.abs() > 0.5 {
        let sigma = 18.0_f32; // large radius → mid-frequency contrast
        Some(image::imageops::blur(&src, sigma))
    } else {
        None
    };

    let mut dst = RgbImage::new(w, h);

    for (x, y, px) in src.enumerate_pixels() {
        let [r8, g8, b8] = px.0;

        // ── Step 1: Decode sRGB → linear float ──────────────────────────────
        let mut r = srgb_to_linear(r8 as f32 / 255.0);
        let mut g = srgb_to_linear(g8 as f32 / 255.0);
        let mut b = srgb_to_linear(b8 as f32 / 255.0);

        // ── Step 2: White Balance (in linear space) ──────────────────────────
        if p.temperature.abs() > 0.5 || p.tint.abs() > 0.5 {
            let t = p.temperature / 100.0;
            let ti = p.tint / 100.0;
            r *= 1.0 + t * 0.30;
            g *= 1.0 + ti * 0.12;
            b *= 1.0 - t * 0.30;
        }

        // ── Step 3: Exposure (EV stops) ──────────────────────────────────────
        if p.exposure.abs() > 0.01 {
            let factor = 2.0_f32.powf(p.exposure);
            r *= factor;
            g *= factor;
            b *= factor;
        }

        // ── Step 4: Highlights / Shadows (tone range) ────────────────────────
        if p.highlights.abs() > 0.5 || p.shadows.abs() > 0.5 {
            let luma = 0.2126 * r + 0.7152 * g + 0.0722 * b;
            let hl = p.highlights / 100.0;
            let sh = p.shadows / 100.0;

            // Highlight curve: smooth bump centred at upper range.
            let ht = ((luma - 0.5) / 0.5).clamp(0.0, 1.0);
            let hl_bump = hl * ht * ht * 0.55;

            // Shadow curve: smooth bump centred at lower range.
            let st = ((0.5 - luma) / 0.5).clamp(0.0, 1.0);
            let sh_bump = sh * st * st * 0.55;

            r = (r + hl_bump + sh_bump).max(0.0);
            g = (g + hl_bump + sh_bump).max(0.0);
            b = (b + hl_bump + sh_bump).max(0.0);
        }

        // ── Step 5: Whites / Blacks ──────────────────────────────────────────
        if p.whites.abs() > 0.5 {
            let wf = 1.0 + p.whites / 100.0 * 0.45;
            r *= wf;
            g *= wf;
            b *= wf;
        }
        if p.blacks.abs() > 0.5 {
            let bl = p.blacks / 100.0 * 0.18;
            let lift = |v: f32| v + bl * (1.0 - v).max(0.0);
            r = lift(r);
            g = lift(g);
            b = lift(b);
        }

        // ── Step 6: Clamp, encode back to sRGB gamma ─────────────────────────
        r = linear_to_srgb(r.clamp(0.0, 1.0));
        g = linear_to_srgb(g.clamp(0.0, 1.0));
        b = linear_to_srgb(b.clamp(0.0, 1.0));

        // ── Step 7: Contrast (S-curve in gamma space) ────────────────────────
        if p.contrast.abs() > 0.5 {
            let ct = p.contrast / 100.0;
            r = contrast_curve(r, ct);
            g = contrast_curve(g, ct);
            b = contrast_curve(b, ct);
        }

        // ── Step 8: Clarity (local contrast via unsharp mask) ────────────────
        if let Some(ref blur) = clarity_blur {
            let [br, bg, bb] = blur.get_pixel(x, y).0;
            let cl = p.clarity / 100.0;
            let scale = 0.65;
            r = (r + cl * (r - br as f32 / 255.0) * scale).clamp(0.0, 1.0);
            g = (g + cl * (g - bg as f32 / 255.0) * scale).clamp(0.0, 1.0);
            b = (b + cl * (b - bb as f32 / 255.0) * scale).clamp(0.0, 1.0);
        }

        // ── Step 9: Vibrance + Saturation (HSL space) ────────────────────────
        if p.vibrance.abs() > 0.5 || p.saturation.abs() > 0.5 {
            let (h_hsl, s_hsl, l_hsl) = rgb_to_hsl(r, g, b);
            let vib = p.vibrance / 100.0 * (1.0 - s_hsl); // smart: less effect on already-saturated
            let sat = p.saturation / 100.0;
            let s_new = (s_hsl + vib + sat * s_hsl).clamp(0.0, 1.0);
            (r, g, b) = hsl_to_rgb(h_hsl, s_new, l_hsl);
        }

        // ── Step 10: Vignette (radial edge darkening) ────────────────────────
        if p.vignette.abs() > 0.5 {
            let vf = vignette_factor(x, y, w, h, p.vignette / 100.0);
            r = (r * vf).clamp(0.0, 1.0);
            g = (g * vf).clamp(0.0, 1.0);
            b = (b * vf).clamp(0.0, 1.0);
        }

        // ── Step 11: Grain (film-like noise) ─────────────────────────────────
        if p.grain > 0.5 {
            let noise = grain_at(x, y, p.grain / 100.0);
            r = (r + noise).clamp(0.0, 1.0);
            g = (g + noise).clamp(0.0, 1.0);
            b = (b + noise).clamp(0.0, 1.0);
        }

        // ── Step 12: B&W conversion (last) ───────────────────────────────────
        if p.bw_enabled {
            let (h_hsl, s_hsl, _) = rgb_to_hsl(r, g, b);
            let lum = bw_luma(r, g, b, h_hsl, s_hsl, p);
            r = lum;
            g = lum;
            b = lum;
        }

        dst.put_pixel(
            x,
            y,
            image::Rgb([
                (r * 255.0).round().clamp(0.0, 255.0) as u8,
                (g * 255.0).round().clamp(0.0, 255.0) as u8,
                (b * 255.0).round().clamp(0.0, 255.0) as u8,
            ]),
        );
    }

    DynamicImage::ImageRgb8(dst)
}

// ── Geometry (rotate + crop) ──────────────────────────────────────────────────

/// Rotate `img` by `rotation` degrees (0/90/180/270, clockwise) and crop to
/// `crop` (normalized to the rotated frame). Runs before the color pipeline
/// so effects like vignette are centered on the final, cropped frame.
///
/// Public so callers outside the color pipeline (the developed-cache
/// regenerator in `commands.rs`) can apply geometry alone.
pub fn apply_geometry(img: &DynamicImage, rotation: i16, crop: Option<CropRect>) -> DynamicImage {
    let rotated = match rotation.rem_euclid(360) {
        90 => img.rotate90(),
        180 => img.rotate180(),
        270 => img.rotate270(),
        _ => img.clone(),
    };

    let Some(rect) = crop else {
        return rotated;
    };

    let rgb = rotated.to_rgb8();
    let (w, h) = rgb.dimensions();

    let x = ((rect.x.clamp(0.0, 1.0) * w as f32).round() as u32).min(w.saturating_sub(1));
    let y = ((rect.y.clamp(0.0, 1.0) * h as f32).round() as u32).min(h.saturating_sub(1));
    let cw = ((rect.width.max(0.0) * w as f32).round() as u32)
        .max(1)
        .min(w - x);
    let ch = ((rect.height.max(0.0) * h as f32).round() as u32)
        .max(1)
        .min(h - y);

    DynamicImage::ImageRgb8(imageops::crop_imm(&rgb, x, y, cw, ch).to_image())
}

// ── Colour-space helpers ──────────────────────────────────────────────────────

#[inline]
fn srgb_to_linear(v: f32) -> f32 {
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

#[inline]
fn linear_to_srgb(v: f32) -> f32 {
    if v <= 0.0031308 {
        v * 12.92
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    }
}

/// S-curve contrast: `amount` in [-1, +1].
fn contrast_curve(v: f32, amount: f32) -> f32 {
    // Scale distance from mid-gray; negative amount compresses.
    let mid = 0.5_f32;
    let delta = v - mid;
    let scale = 1.0 + amount * 0.65;
    (mid + delta * scale).clamp(0.0, 1.0)
}

fn rgb_to_hsl(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let l = (max + min) * 0.5;
    let s = if d < 1e-6 {
        0.0
    } else {
        d / (1.0 - (2.0 * l - 1.0).abs())
    };
    let h = if d < 1e-6 {
        0.0
    } else if max == r {
        let seg = (g - b) / d;
        60.0 * (seg.rem_euclid(6.0))
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    (h, s, l)
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    if s < 1e-6 {
        return (l, l, l);
    }
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0).rem_euclid(2.0) - 1.0).abs());
    let m = l - c * 0.5;
    let (r, g, b) = match h as u32 / 60 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    (
        (r + m).clamp(0.0, 1.0),
        (g + m).clamp(0.0, 1.0),
        (b + m).clamp(0.0, 1.0),
    )
}

// ── Effect helpers ────────────────────────────────────────────────────────────

/// Radial vignette multiplier at pixel (x, y).
/// `amount > 0` = dark edges; `amount < 0` = bright edges.
fn vignette_factor(x: u32, y: u32, w: u32, h: u32, amount: f32) -> f32 {
    let cx = w as f32 * 0.5;
    let cy = h as f32 * 0.5;
    let dx = (x as f32 - cx) / cx;
    let dy = (y as f32 - cy) / cy;
    // Normalise so the corner (1,1) maps to distance 1.
    let dist = (dx * dx + dy * dy).sqrt() / std::f32::consts::SQRT_2;
    (1.0 - amount * dist * dist).clamp(0.0, 1.5)
}

/// Deterministic per-pixel noise derived from coordinates.
fn grain_at(x: u32, y: u32, amount: f32) -> f32 {
    // LCG-style hash of the pixel coordinates.
    let h = x
        .wrapping_mul(2_246_822_519_u32)
        .wrapping_add(y.wrapping_mul(2_654_435_761_u32));
    // Map u32 → [-0.5, +0.5]
    let norm = h as f32 / u32::MAX as f32 - 0.5;
    norm * amount * 0.12
}

// ── B&W channel mixer ─────────────────────────────────────────────────────────

/// Convert RGB to grayscale using hue-aware channel weights.
fn bw_luma(r: f32, g: f32, b: f32, hue_deg: f32, sat: f32, p: &DevelopParams) -> f32 {
    // Base Rec.709 luminance.
    let base = 0.2126 * r + 0.7152 * g + 0.0722 * b;

    // Determine which hue band(s) this pixel falls into and blend weights.
    // The mixer shifts the effective luminance of each hue range.
    let hue_adj = hue_adjust(hue_deg, p);

    // Blend: fully saturated pixels use hue adjustment; grays use base luma.
    (base + sat * hue_adj * 0.30).clamp(0.0, 1.0)
}

/// Returns a luminance bias for pixels of the given hue (from the B&W mixer sliders).
fn hue_adjust(h: f32, p: &DevelopParams) -> f32 {
    // 6 primary hue bands, each ±60° wide with cosine blending at edges.
    let zones: &[(f32, f32)] = &[
        (0.0, p.bw_red / 100.0),     // Red
        (30.0, p.bw_orange / 100.0), // Orange
        (60.0, p.bw_yellow / 100.0), // Yellow
        (120.0, p.bw_green / 100.0), // Green
        (180.0, p.bw_aqua / 100.0),  // Aqua
        (240.0, p.bw_blue / 100.0),  // Blue
    ];

    zones.iter().fold(0.0_f32, |acc, &(center, weight)| {
        let diff = angular_diff(h, center);
        // Cosine falloff: full influence within 30°, zero at 90°.
        let influence = ((1.0 - diff / 90.0) * std::f32::consts::PI * 0.5)
            .cos()
            .max(0.0);
        acc + weight * influence
    })
}

#[inline]
fn angular_diff(a: f32, b: f32) -> f32 {
    let d = (a - b).rem_euclid(360.0);
    if d > 180.0 {
        360.0 - d
    } else {
        d
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn test_image(w: u32, h: u32) -> DynamicImage {
        DynamicImage::ImageRgb8(RgbImage::from_fn(w, h, |x, y| {
            image::Rgb([(x % 256) as u8, (y % 256) as u8, 128])
        }))
    }

    #[test]
    fn identity_params_preserve_dimensions() {
        let img = test_image(40, 20);
        let out = apply_develop(&img, &DevelopParams::default());
        assert_eq!((out.width(), out.height()), (40, 20));
    }

    #[test]
    fn rotate_90_swaps_dimensions() {
        let img = test_image(40, 20);
        let params = DevelopParams {
            rotation: 90,
            ..Default::default()
        };
        let out = apply_develop(&img, &params);
        assert_eq!((out.width(), out.height()), (20, 40));
    }

    #[test]
    fn rotate_180_preserves_dimensions() {
        let img = test_image(40, 20);
        let params = DevelopParams {
            rotation: 180,
            ..Default::default()
        };
        let out = apply_develop(&img, &params);
        assert_eq!((out.width(), out.height()), (40, 20));
    }

    #[test]
    fn crop_produces_expected_pixel_size() {
        let img = test_image(100, 50);
        let params = DevelopParams {
            crop: Some(CropRect {
                x: 0.25,
                y: 0.2,
                width: 0.5,
                height: 0.6,
            }),
            ..Default::default()
        };
        let out = apply_develop(&img, &params);
        assert_eq!((out.width(), out.height()), (50, 30));
    }

    #[test]
    fn rotate_then_crop_uses_rotated_frame() {
        // Rotate 90° (100x50 -> 50x100), then crop half the (rotated) width.
        let img = test_image(100, 50);
        let params = DevelopParams {
            rotation: 90,
            crop: Some(CropRect {
                x: 0.0,
                y: 0.0,
                width: 0.5,
                height: 1.0,
            }),
            ..Default::default()
        };
        let out = apply_develop(&img, &params);
        assert_eq!((out.width(), out.height()), (25, 100));
    }
}
