// src/core/analysis.rs
// Per-image technical analysis: RGB histograms, Ansel Adams Zone System,
// spatial focus heatmap, and colour-temperature estimate.
//
// All signals are computed from the cached thumbnail JPEG (already on disk
// from the culling decode), so the call is fast (<30 ms on a typical 400 px
// thumbnail) and does not re-open the original RAW file.

use image::DynamicImage;
use serde::Serialize;

// ── Output DTO ────────────────────────────────────────────────────────────────

/// All technical signals for a single image.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageAnalysis {
    // ── Histograms ────────────────────────────────────────────────────────────
    /// Per-channel histograms, 256 buckets each.
    /// Values are normalised to [0, 1] relative to the global peak bucket so
    /// all four channels share the same vertical scale.
    pub hist_r: Vec<f32>,
    pub hist_g: Vec<f32>,
    pub hist_b: Vec<f32>,
    pub hist_lum: Vec<f32>,

    // ── Zone System ───────────────────────────────────────────────────────────
    /// Fraction of pixels falling in each Ansel Adams zone (Zone 0 … Zone X).
    /// `zone_pcts[0]` = Zone 0 (pure black), `zone_pcts[10]` = Zone X (pure white).
    /// The 256-level range is divided into 11 equal bands.
    pub zone_pcts: Vec<f32>,

    // ── Focus heatmap ─────────────────────────────────────────────────────────
    /// Flat (row-major) grid of normalised Laplacian energies.
    /// 0.0 = softest cell, 1.0 = sharpest cell in the image.
    pub focus_map: Vec<f32>,
    pub focus_map_cols: u32,
    pub focus_map_rows: u32,

    // ── Exposure ──────────────────────────────────────────────────────────────
    /// Fraction of pixels with any channel > 250 (blown highlights).
    pub blown_pct: f32,
    /// Fraction of pixels with all channels < 5 (crushed shadows).
    pub crushed_pct: f32,
    /// Mean Rec.709 luminance in [0, 1].
    pub mean_luma: f32,

    // ── Colour temperature ────────────────────────────────────────────────────
    /// Rough colour-temperature estimate in Kelvin, derived from the R/B ratio
    /// of neutral (low-saturation) pixels.  `None` when fewer than 100 neutral
    /// pixels were found — e.g. highly saturated or monochromatic scenes.
    pub color_temp_k: Option<u32>,
}

// ── Main entry point ──────────────────────────────────────────────────────────

/// Compute all technical signals from a decoded image.
/// Pass the cached thumbnail; full-resolution re-decode is not required.
pub fn compute_analysis(img: &DynamicImage) -> ImageAnalysis {
    let rgb = img.to_rgb8();
    let (w, h) = rgb.dimensions();
    let total = (w * h) as u64;

    if total == 0 {
        return empty_analysis();
    }

    // ── Single-pass pixel loop ────────────────────────────────────────────────
    let mut hist_r_raw = [0u64; 256];
    let mut hist_g_raw = [0u64; 256];
    let mut hist_b_raw = [0u64; 256];
    let mut hist_lum_raw = [0u64; 256];
    let mut zone_raw = [0u64; 11];
    let mut blown = 0u64;
    let mut crushed = 0u64;
    let mut luma_sum = 0.0f64;

    // Neutral-pixel accumulators for colour-temp estimation.
    // "Neutral" = mid-tone pixel whose R, G, B are within 25 % of each other.
    let mut neutral_r = 0.0f64;
    let mut neutral_b = 0.0f64;
    let mut neutral_n = 0u64;

    for pixel in rgb.pixels() {
        let [r, g, b] = pixel.0;

        hist_r_raw[r as usize] += 1;
        hist_g_raw[g as usize] += 1;
        hist_b_raw[b as usize] += 1;

        // Rec.709 luma coefficient.
        let luma = (0.2126 * r as f64 + 0.7152 * g as f64 + 0.0722 * b as f64)
            .round()
            .min(255.0) as u8;
        hist_lum_raw[luma as usize] += 1;
        luma_sum += luma as f64;

        // Zone 0–X: 11 equal luminance bands across [0, 255].
        let zone = ((luma as usize * 11) / 256).min(10);
        zone_raw[zone] += 1;

        if r > 250 || g > 250 || b > 250 {
            blown += 1;
        }
        if r < 5 && g < 5 && b < 5 {
            crushed += 1;
        }

        // Neutral-pixel filter: mid-tone, low saturation.
        let avg = (r as f64 + g as f64 + b as f64) / 3.0;
        if avg > 30.0 && avg < 220.0 {
            let dev = |v: u8| ((v as f64 - avg) / avg).abs();
            if dev(r) < 0.25 && dev(g) < 0.25 && dev(b) < 0.25 {
                neutral_r += r as f64;
                neutral_b += b as f64;
                neutral_n += 1;
            }
        }
    }

    // ── Normalise histograms ──────────────────────────────────────────────────
    // All four channels share a single peak so their relative heights compare.
    let global_peak = hist_r_raw
        .iter()
        .chain(hist_g_raw.iter())
        .chain(hist_b_raw.iter())
        .chain(hist_lum_raw.iter())
        .copied()
        .max()
        .unwrap_or(1)
        .max(1) as f32;

    let norm = |h: &[u64; 256]| -> Vec<f32> { h.iter().map(|&v| v as f32 / global_peak).collect() };

    // ── Zone system ───────────────────────────────────────────────────────────
    let zone_pcts: Vec<f32> = zone_raw.iter().map(|&c| c as f32 / total as f32).collect();

    // ── Scalar exposure signals ───────────────────────────────────────────────
    let mean_luma = (luma_sum / (total as f64 * 255.0)) as f32;
    let blown_pct = blown as f32 / total as f32;
    let crushed_pct = crushed as f32 / total as f32;

    // ── Colour temperature ────────────────────────────────────────────────────
    // Rough approximation: at ~5 500 K (D55 daylight) a neutral target has R ≈ B.
    // Warmer light raises R relative to B; cooler light raises B.
    let color_temp_k = if neutral_n >= 100 && neutral_b > 0.0 {
        let ratio = neutral_r / neutral_b; // > 1 = warm, < 1 = cool
        let k = (5_500.0 / ratio).round() as u32;
        Some(k.clamp(1_800, 12_000))
    } else {
        None
    };

    // ── Spatial focus heatmap ─────────────────────────────────────────────────
    // Use 8 × 5 cells for landscape, 5 × 8 for portrait.
    let (cols, rows) = if h > w { (5u32, 8u32) } else { (8u32, 5u32) };
    let cell_w = (w / cols).max(1);
    let cell_h = (h / rows).max(1);

    // Pre-compute a flat grayscale image for the Laplacian.
    let gray: Vec<f32> = rgb
        .pixels()
        .map(|p| {
            let [r, g, b] = p.0;
            0.2126 * r as f32 / 255.0 + 0.7152 * g as f32 / 255.0 + 0.0722 * b as f32 / 255.0
        })
        .collect();

    let mut focus_map = vec![0.0f32; (cols * rows) as usize];

    for row in 0..rows {
        for col in 0..cols {
            let x0 = (col * cell_w) as usize;
            let y0 = (row * cell_h) as usize;
            let x1 = (x0 + cell_w as usize).min(w as usize);
            let y1 = (y0 + cell_h as usize).min(h as usize);
            let cw = x1 - x0;
            let ch = y1 - y0;

            // Copy cell pixels into a contiguous buffer.
            let mut cell = vec![0.0f32; cw * ch];
            for cy in 0..ch {
                let src = &gray[(y0 + cy) * w as usize + x0..(y0 + cy) * w as usize + x1];
                cell[cy * cw..cy * cw + cw].copy_from_slice(src);
            }

            focus_map[(row * cols + col) as usize] = laplacian_energy(&cell, cw);
        }
    }

    // Normalise to [0, 1]; apply √ to spread low values apart.
    let max_focus = focus_map.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    if max_focus > 1e-8 {
        for v in focus_map.iter_mut() {
            *v = (*v / max_focus).sqrt();
        }
    }

    ImageAnalysis {
        hist_r: norm(&hist_r_raw),
        hist_g: norm(&hist_g_raw),
        hist_b: norm(&hist_b_raw),
        hist_lum: norm(&hist_lum_raw),
        zone_pcts,
        focus_map,
        focus_map_cols: cols,
        focus_map_rows: rows,
        blown_pct,
        crushed_pct,
        mean_luma,
        color_temp_k,
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Mean squared Laplacian energy — high in sharp regions, low in blurry ones.
fn laplacian_energy(luma: &[f32], width: usize) -> f32 {
    let height = luma.len() / width;
    if width < 3 || height < 3 {
        return 0.0;
    }

    let mut sum = 0.0f32;
    let mut count = 0u32;

    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let i = y * width + x;
            let lap =
                -4.0 * luma[i] + luma[i - 1] + luma[i + 1] + luma[i - width] + luma[i + width];
            sum += lap * lap;
            count += 1;
        }
    }

    if count > 0 {
        sum / count as f32
    } else {
        0.0
    }
}

fn empty_analysis() -> ImageAnalysis {
    ImageAnalysis {
        hist_r: vec![0.0; 256],
        hist_g: vec![0.0; 256],
        hist_b: vec![0.0; 256],
        hist_lum: vec![0.0; 256],
        zone_pcts: vec![0.0; 11],
        focus_map: vec![0.0; 40],
        focus_map_cols: 8,
        focus_map_rows: 5,
        blown_pct: 0.0,
        crushed_pct: 0.0,
        mean_luma: 0.0,
        color_temp_k: None,
    }
}
