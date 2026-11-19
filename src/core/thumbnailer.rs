// src/core/thumbnailer.rs
// Off-thread thumbnail generation pipeline.
//
// Flow:
//   1. Caller sends ImageEntry paths via the `request_tx` channel.
//   2. A rayon thread pool decodes the embedded JPEG preview from each RAW
//      (or the JPEG itself), resizes it to THUMB_SIZE, and sends the result
//      back via `result_tx`.
//   3. The UI polls `result_rx` each frame and uploads ready thumbnails to
//      egui as TextureHandles.

use crate::core::exif::ImageExif;
use crate::core::image::{ExposureInfo, ImageFormat, ImageId};
use anyhow::Result;
use crossbeam::channel::{self, Receiver, Sender};
use image::DynamicImage;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tracing::warn;

/// Target longest edge for filmstrip thumbnails (pixels).
pub const THUMB_SIZE: u32 = 512;

/// Target longest edge for the high-resolution viewer image.
/// 2048 px fills a 1440p display without upscaling.
pub const VIEWER_SIZE: u32 = 2048;

// ── Request / response types ──────────────────────────────────────────────────

#[derive(Debug)]
pub struct ThumbRequest {
    pub id: ImageId,
    pub path: PathBuf,
    pub format: ImageFormat,
    /// When `true`, decode at `VIEWER_SIZE` and skip signal computation
    /// (focus score, faces, EXIF) — those are already known from the
    /// filmstrip-quality decode.
    pub high_res: bool,
}

#[derive(Debug)]
pub struct ThumbResult {
    pub id: ImageId,
    /// RGB8 pixels in row-major order.
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
    /// Mirrors the `high_res` flag on the originating request.
    pub high_res: bool,
    /// Local Laplacian focus score in [0,1]. Always `0.0` for high-res results.
    pub focus_score: f32,
    /// Face regions detected by the local UltraFace ONNX model.
    pub faces: Vec<crate::core::image::FaceRegion>,
    /// 64-bit dHash for near-duplicate detection.
    pub p_hash: u64,
    /// Histogram-derived exposure signals.
    pub exposure: ExposureInfo,
    /// EXIF camera metadata (None for formats where it isn't available).
    pub exif: Option<ImageExif>,
}

// ── Thumbnailer ───────────────────────────────────────────────────────────────

pub struct Thumbnailer {
    pub request_tx: Sender<ThumbRequest>,
    pub result_rx: Receiver<ThumbResult>,
}

impl Thumbnailer {
    /// Spawn the background worker pool and return a handle.
    pub fn spawn() -> Self {
        let (request_tx, request_rx) = channel::bounded::<ThumbRequest>(256);
        let (result_tx, result_rx) = channel::bounded::<ThumbResult>(256);

        // Clone into the thread so we can move it.
        std::thread::spawn(move || {
            // Use rayon for parallel decode without blocking the async runtime.
            use rayon::prelude::*;

            // Drain requests and process them in parallel batches.
            let request_rx = Arc::new(request_rx);
            loop {
                // Collect a small batch before dispatching.
                let batch: Vec<ThumbRequest> = {
                    let mut b = Vec::with_capacity(16);
                    // Block on the first item.
                    match request_rx.recv() {
                        Ok(req) => b.push(req),
                        Err(_) => break, // channel closed
                    }
                    // Drain any already-queued items (non-blocking).
                    while let Ok(req) = request_rx.try_recv() {
                        b.push(req);
                        if b.len() >= 16 {
                            break;
                        }
                    }
                    b
                };

                batch.into_par_iter().for_each(|req| {
                    tracing::trace!(
                        "decoding {:?} hi_res={}",
                        req.path.file_name(),
                        req.high_res
                    );
                    match decode_thumbnail_with_exif(&req) {
                        Ok((img, exif)) => {
                            let rgb = img.to_rgb8();
                            let (w, h) = (rgb.width(), rgb.height());

                            // High-res viewer requests skip signal computation
                            // (already done during the filmstrip-quality decode).
                            let (focus_score, faces, p_hash, exposure) = if req.high_res {
                                (0.0, vec![], 0u64, ExposureInfo::default())
                            } else {
                                (
                                    crate::ai::local::laplacian_focus_score(&img),
                                    crate::ai::face_detector::detect_faces(&img),
                                    crate::ai::local::dhash(&img),
                                    crate::ai::local::exposure_info(&img),
                                )
                            };

                            let _ = result_tx.send(ThumbResult {
                                id: req.id,
                                pixels: rgb.into_raw(),
                                width: w,
                                height: h,
                                high_res: req.high_res,
                                focus_score,
                                faces,
                                p_hash,
                                exposure,
                                exif,
                            });
                        }
                        Err(e) => {
                            warn!("Thumbnail decode failed for {:?}: {e}", req.path);
                        }
                    }
                });
            }
        });

        Self {
            request_tx,
            result_rx,
        }
    }

    pub fn enqueue(&self, req: ThumbRequest) {
        // Non-blocking — drop if the queue is full (backpressure handled by UI).
        let _ = self.request_tx.try_send(req);
    }

    /// Drain all ready results without blocking.
    pub fn drain_ready(&self) -> impl Iterator<Item = ThumbResult> + '_ {
        std::iter::from_fn(|| self.result_rx.try_recv().ok())
    }
}

// ── Decode logic ──────────────────────────────────────────────────────────────

/// Decode a thumbnail (or HD viewer image) and extract EXIF metadata.
/// The longest edge is capped at `VIEWER_SIZE` for high-res requests and at
/// `THUMB_SIZE` for filmstrip requests.
pub fn decode_thumbnail_with_exif(req: &ThumbRequest) -> Result<(DynamicImage, Option<ImageExif>)> {
    let size = if req.high_res {
        VIEWER_SIZE
    } else {
        THUMB_SIZE
    };
    if req.format.is_raw() {
        decode_raw_thumbnail(&req.path, size)
    } else {
        Ok((decode_jpeg_thumbnail(&req.path, size)?, None))
    }
}

/// Extract a viewable preview from a RAW file using the pure-Rust `rawler`
/// crate.  Also pulls EXIF camera metadata in the same metadata read.
///
/// Cameras embed a fully-rendered JPEG preview inside every RAW file. Pulling
/// that out is far faster than demosaicing the sensor data, and it carries the
/// camera's own colour rendering. We try the medium preview first, fall back to
/// the full-resolution embedded image, then the small thumbnail.
fn decode_raw_thumbnail(path: &Path, max_size: u32) -> Result<(DynamicImage, Option<ImageExif>)> {
    use rawler::decoders::RawDecodeParams;
    use rawler::rawsource::RawSource;

    let source = RawSource::new(path)?;
    let decoder = rawler::get_decoder(&source)?;
    let params = RawDecodeParams::default();

    let preview = decoder
        .preview_image(&source, &params)
        .ok()
        .flatten()
        .or_else(|| decoder.full_image(&source, &params).ok().flatten())
        .or_else(|| decoder.thumbnail_image(&source, &params).ok().flatten());

    let Some(preview) = preview else {
        anyhow::bail!("RAW file contains no embedded preview image");
    };

    // One metadata call gives us both orientation and all EXIF fields.
    let exif_result = decoder.raw_metadata(&source, &params);
    let orientation = exif_result
        .as_ref()
        .ok()
        .and_then(|m| m.exif.orientation)
        .unwrap_or(1);

    let thumb_exif = exif_result
        .ok()
        .map(|m| crate::core::exif::from_rawler_exif(&m.exif));

    Ok((
        fit_to_size(orient_image(preview, orientation), max_size),
        thumb_exif,
    ))
}

/// Apply an EXIF orientation (1–8) to an image.
///
/// A sanity check guards against double-rotation: if a 90°/270° orientation is
/// requested but the preview is already taller than it is wide, the camera has
/// already rotated it for us, so the image is returned unchanged.
fn orient_image(img: DynamicImage, orientation: u16) -> DynamicImage {
    let quarter_turn = matches!(orientation, 5..=8);
    if quarter_turn && img.height() > img.width() {
        return img; // already in display orientation
    }
    match orientation {
        2 => img.fliph(),
        3 => img.rotate180(),
        4 => img.flipv(),
        5 => img.rotate90().fliph(),
        6 => img.rotate90(),
        7 => img.rotate270().fliph(),
        8 => img.rotate270(),
        _ => img, // 1 (normal) or unknown
    }
}

/// Decode a JPEG/TIFF directly using the `image` crate.
fn decode_jpeg_thumbnail(path: &PathBuf, max_size: u32) -> Result<DynamicImage> {
    Ok(fit_to_size(image::open(path)?, max_size))
}

/// Resize an image so its longest edge equals `max_size`.
pub fn fit_to_size(img: DynamicImage, max_size: u32) -> DynamicImage {
    let (w, h) = (img.width(), img.height());
    if w == 0 || h == 0 {
        return img;
    }
    let (nw, nh) = if w >= h {
        (max_size, (h as f32 * max_size as f32 / w as f32) as u32)
    } else {
        ((w as f32 * max_size as f32 / h as f32) as u32, max_size)
    };
    img.resize(nw, nh, image::imageops::FilterType::Lanczos3)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use image::RgbImage;

    fn solid(w: u32, h: u32) -> DynamicImage {
        DynamicImage::ImageRgb8(RgbImage::new(w, h))
    }

    #[test]
    fn orientation_normal_is_identity() {
        let out = orient_image(solid(200, 100), 1);
        assert_eq!((out.width(), out.height()), (200, 100));
    }

    #[test]
    fn orientation_rotate180_keeps_dimensions() {
        let out = orient_image(solid(200, 100), 3);
        assert_eq!((out.width(), out.height()), (200, 100));
    }

    #[test]
    fn orientation_rotates_landscape_preview_to_portrait() {
        // A landscape (sensor-orientation) preview tagged "rotate 90" must
        // come out portrait.
        for tag in [5, 6, 7, 8] {
            let out = orient_image(solid(200, 100), tag);
            assert!(
                out.height() > out.width(),
                "orientation {tag} should yield a portrait image"
            );
        }
    }

    #[test]
    fn orientation_does_not_double_rotate_already_portrait_preview() {
        // The camera already rotated the preview — applying the tag again
        // would turn it back to landscape, which is wrong.
        for tag in [5, 6, 7, 8] {
            let out = orient_image(solid(100, 200), tag);
            assert_eq!(
                (out.width(), out.height()),
                (100, 200),
                "orientation {tag} double-rotated an already-portrait preview"
            );
        }
    }

    /// Decode every RAW file in the directory named by `CULLD_TEST_RAW_DIR`.
    /// Skipped when the variable is unset, so a checkout without sample RAW
    /// files (e.g. CI) still passes `cargo test`.
    ///
    ///   CULLD_TEST_RAW_DIR=/path/to/raws cargo test decode_real_raw_files -- --nocapture
    #[test]
    fn decode_real_raw_files() {
        let Ok(dir) = std::env::var("CULLD_TEST_RAW_DIR") else {
            eprintln!("CULLD_TEST_RAW_DIR not set — skipping real RAW decode test");
            return;
        };

        let mut tested = 0;
        for entry in std::fs::read_dir(&dir).expect("read CULLD_TEST_RAW_DIR") {
            let path = entry.unwrap().path();
            let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
                continue;
            };
            let is_raw = ImageFormat::from_extension(ext).is_some_and(|f| f.is_raw());
            if !is_raw {
                continue;
            }

            let (img, exif) = decode_raw_thumbnail(&path, THUMB_SIZE)
                .unwrap_or_else(|e| panic!("failed to decode {path:?}: {e}"));
            assert!(
                img.width() > 0 && img.height() > 0,
                "{path:?} decoded to an empty image"
            );
            assert!(
                img.width() <= THUMB_SIZE && img.height() <= THUMB_SIZE,
                "{path:?} was not resized down to THUMB_SIZE"
            );
            let focus = crate::ai::local::laplacian_focus_score(&img);
            let hash = crate::ai::local::dhash(&img);
            let exp = crate::ai::local::exposure_info(&img);
            eprintln!(
                "decoded {} -> {}x{}  focus={focus:.3}  hash={hash:#018x}  \
                 luma={:.2}  blown={:.1}%  crushed={:.1}%  \
                 iso={:?}  shutter={:?}  aperture={:?}  lens={:?}",
                path.display(),
                img.width(),
                img.height(),
                exp.mean_luma,
                exp.blown_pct * 100.0,
                exp.crushed_pct * 100.0,
                exif.as_ref().and_then(|e| e.iso),
                exif.as_ref().and_then(|e| e.shutter_speed.as_deref()),
                exif.as_ref().and_then(|e| e.aperture),
                exif.as_ref().and_then(|e| e.lens_model.as_deref()),
            );

            // Optionally dump the decoded thumbnail for visual inspection.
            if let Ok(out_dir) = std::env::var("CULLD_TEST_RAW_OUT") {
                let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("raw");
                let out = std::path::Path::new(&out_dir).join(format!("{stem}.png"));
                img.save(&out).expect("save decoded thumbnail");
                eprintln!("  wrote {}", out.display());
            }

            tested += 1;
        }
        assert!(tested > 0, "no RAW files found in {dir}");
    }
}
