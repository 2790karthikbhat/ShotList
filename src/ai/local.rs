// src/ai/local.rs
// Local, offline quality signals that run without any API key.
//
// 1. Focus score — Laplacian variance (pure math, no model needed).
// 2. Face detection — YOLOv8-face ONNX model (bundled or downloaded on first run).
// 3. Eye-open classifier — lightweight binary classifier on a face crop.
//
// The ONNX inference uses `ort` (ONNX Runtime Rust bindings).

use crate::core::image::{AiAssessment, ExposureInfo, FaceRegion};
use anyhow::Result;
use image::{DynamicImage, GrayImage, Luma};

// ── Focus scoring (Laplacian variance) ───────────────────────────────────────
// No model required — just pixel math.
// High variance = sharp edges = in-focus image.

/// Returns a normalised focus score in [0, 1].
/// Scores below ~0.05 are typically very blurry; above ~0.4 are sharp.
pub fn laplacian_focus_score(img: &DynamicImage) -> f32 {
    let gray: GrayImage = img.to_luma8();
    let (w, h) = (gray.width() as usize, gray.height() as usize);

    if w < 3 || h < 3 {
        return 0.0;
    }

    let pixels: Vec<f32> = gray.pixels().map(|Luma([v])| *v as f32).collect();

    // 3×3 Laplacian kernel:  0  1  0
    //                        1 -4  1
    //                        0  1  0
    let mut sum_sq = 0.0f64;
    let mut count = 0u64;

    for y in 1..(h - 1) {
        for x in 1..(w - 1) {
            let idx = y * w + x;
            let lap = -4.0 * pixels[idx]
                + pixels[idx - 1]
                + pixels[idx + 1]
                + pixels[idx - w]
                + pixels[idx + w];
            sum_sq += (lap * lap) as f64;
            count += 1;
        }
    }

    if count == 0 {
        return 0.0;
    }

    let variance = (sum_sq / count as f64).sqrt() as f32;

    // Normalise: empirically, sharp photos typically fall in [20, 80].
    // Clamp to [0, 1] with 60 as the "sharp" ceiling.
    (variance / 60.0).clamp(0.0, 1.0)
}

// ── Perceptual hash (dHash) ───────────────────────────────────────────────────
//
// Difference hash: compare each pixel to its right neighbour in a 9×8 grid.
// Produces a stable 64-bit fingerprint; Hamming distance ≤ 8 → near-duplicate.

/// Compute a 64-bit dHash of `img`.  Fast and allocation-light — runs on the
/// already-decoded thumbnail, costs nothing extra in the decode pipeline.
pub fn dhash(img: &DynamicImage) -> u64 {
    let small = img.resize_exact(9, 8, image::imageops::FilterType::Triangle);
    let gray = small.to_luma8();
    let mut hash: u64 = 0;
    for y in 0..8u32 {
        for x in 0..8u32 {
            let left = gray.get_pixel(x, y)[0] as i16;
            let right = gray.get_pixel(x + 1, y)[0] as i16;
            hash = (hash << 1) | ((left > right) as u64);
        }
    }
    hash
}

/// Hamming distance between two dHashes.
/// Values ≤ 4 = likely same shot; ≤ 8 = near-duplicate burst frame; > 10 = different.
pub fn dhash_distance(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}

// ── Exposure / histogram analysis ─────────────────────────────────────────────

/// Derive exposure signals from the thumbnail's luma histogram.
/// Takes ~100 µs on a 512-pixel thumbnail; free to run alongside focus score.
pub fn exposure_info(img: &DynamicImage) -> ExposureInfo {
    let gray: GrayImage = img.to_luma8();
    let pixels: Vec<u8> = gray.pixels().map(|Luma([v])| *v).collect();
    let total = pixels.len();
    if total == 0 {
        return ExposureInfo::default();
    }

    let sum: u64 = pixels.iter().map(|&p| p as u64).sum();
    let mean_luma = sum as f32 / (total as f32 * 255.0);
    let blown_pct = pixels.iter().filter(|&&p| p > 245).count() as f32 / total as f32;
    let crushed_pct = pixels.iter().filter(|&&p| p < 10).count() as f32 / total as f32;

    ExposureInfo {
        mean_luma,
        blown_pct,
        crushed_pct,
    }
}

// ── ONNX face detection ───────────────────────────────────────────────────────

/// Run UltraFace detection via tract-onnx.
///
/// Returns an empty vec (not an error) when the detector is not yet warm
/// (model still downloading / first-run init in progress).
pub fn detect_faces_onnx(img: &DynamicImage) -> Result<Vec<FaceRegion>> {
    Ok(crate::ai::face_detector::detect_faces(img))
}

// ── Combined local assessment ─────────────────────────────────────────────────

/// Run all local, offline assessments on a decoded image.
/// Safe to call from a rayon thread.
pub fn local_assess(img: &DynamicImage) -> AiAssessment {
    let focus_score = Some(laplacian_focus_score(img));
    let faces = detect_faces_onnx(img).unwrap_or_default();

    AiAssessment {
        focus_score,
        faces,
        scene_rank: None, // requires comparing within a scene — done at session level
        ai_label: None,
        provider: Some("local".into()),
    }
}
