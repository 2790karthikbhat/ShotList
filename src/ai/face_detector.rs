// src/ai/face_detector.rs
// Lightweight face detection using the UltraFace RFB-320 ONNX model and
// the pure-Rust `tract-onnx` inference engine.
//
// Model spec (version-RFB-320.onnx, ONNX Model Zoo, MIT licence):
//   Input  "input"       : [1, 3, 240, 320] f32  normalised to [-1, 1]
//   Output "scores"      : [1, 4420, 2]     f32  [background, face] probability
//   Output "boxes"       : [1, 4420, 4]     f32  [x1, y1, x2, y2] in [0, 1]
//
// Lifecycle
//   init_detector()      — download model if absent, load it, warm the cache.
//                          Call once from a background thread at app start.
//   detect_faces(img)    — non-blocking; returns empty vec if not yet ready.
//
// Model is cached at  {data_local_dir}/culld/models/ultraface_rfb_320.onnx
//   macOS: ~/Library/Application Support/culld/models/
//   Linux: ~/.local/share/culld/models/

use crate::core::image::FaceRegion;
use image::{imageops::FilterType, DynamicImage};
use std::{
    io::Read,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
};
use tracing::{info, warn};
use tract_onnx::prelude::*;

// ── Detector status (observable from the settings UI) ────────────────────────

/// Current state of the face-detector lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetectorStatus {
    /// init_detector() has not yet been called.
    NotStarted,
    /// Download or load is currently in progress — do not start a second attempt.
    Pending,
    /// Model is loaded and ready for inference.
    Ready,
    /// Network error during model download — safe to retry.
    DownloadFailed,
    /// Model file exists but failed to load (corrupt?).  Retry unlikely to help.
    LoadFailed,
}

// ── Constants ─────────────────────────────────────────────────────────────────

const MODEL_URL: &str = "https://github.com/onnx/models/raw/main/validated/\
                         vision/body_analysis/ultraface/models/version-RFB-320.onnx";
const MODEL_NAME: &str = "ultraface_rfb_320.onnx";

const INPUT_W: u32 = 320;
const INPUT_H: u32 = 240;

/// Minimum confidence to keep a detection.
const SCORE_THRESHOLD: f32 = 0.65;

/// IoU threshold for non-maximum suppression.
const IOU_THRESHOLD: f32 = 0.45;

/// Maximum faces returned per image (guards against degenerate inputs).
const MAX_FACES: usize = 32;

// ── Detector ──────────────────────────────────────────────────────────────────

/// Concrete tract model plan type.
type TractPlan = SimplePlan<TypedFact, Box<dyn TypedOp>, Graph<TypedFact, Box<dyn TypedOp>>>;

/// Thread-safe wrapper around the compiled ONNX inference plan.
///
/// Wrapped in `Mutex` because `SimplePlan::run()` requires `&mut self` in
/// some tract versions; the mutex overhead is negligible vs. inference time.
pub struct FaceDetector {
    model: Mutex<TractPlan>,
}

impl FaceDetector {
    fn load(path: &Path) -> anyhow::Result<Self> {
        info!("Loading face detector from {}", path.display());
        let model = tract_onnx::onnx()
            .model_for_path(path)?
            .into_optimized()?
            .into_runnable()?;
        info!("Face detector ready");
        Ok(Self {
            model: Mutex::new(model),
        })
    }

    /// Run inference; returns normalised face regions.
    pub fn run(&self, img: &DynamicImage) -> anyhow::Result<Vec<FaceRegion>> {
        // ── Preprocess ────────────────────────────────────────────────────────
        let resized = img.resize_exact(INPUT_W, INPUT_H, FilterType::Triangle);
        let rgb = resized.to_rgb8();

        // Build a 1×3×H×W f32 tensor normalised to [-1, 1].
        let input = tract_ndarray::Array4::from_shape_fn(
            (1, 3, INPUT_H as usize, INPUT_W as usize),
            |(_, c, y, x)| {
                let p = rgb.get_pixel(x as u32, y as u32);
                (p[c] as f32 - 127.0) / 128.0
            },
        );

        // ── Inference ─────────────────────────────────────────────────────────
        // Array4<f32> → Tensor → TValue (two-step conversion required by tract).
        let tensor: Tensor = input.into();
        let outputs = self.model.lock().unwrap().run(tvec![tensor.into()])?;

        // ── Postprocess ───────────────────────────────────────────────────────
        // Flatten both output tensors into contiguous f32 slices.
        let scores_view = outputs[0].to_array_view::<f32>()?;
        let boxes_view = outputs[1].to_array_view::<f32>()?;

        // scores: [1, N, 2] — iterate as flat [bg0, face0, bg1, face1, ...]
        let scores_flat: Vec<f32> = scores_view.iter().copied().collect();
        let boxes_flat: Vec<f32> = boxes_view.iter().copied().collect();

        let num_anchors = scores_flat.len() / 2;

        let mut candidates: Vec<(f32, [f32; 4])> = Vec::new();
        for i in 0..num_anchors {
            let face_score = scores_flat[i * 2 + 1];
            if face_score < SCORE_THRESHOLD {
                continue;
            }
            let x1 = boxes_flat[i * 4].clamp(0.0, 1.0);
            let y1 = boxes_flat[i * 4 + 1].clamp(0.0, 1.0);
            let x2 = boxes_flat[i * 4 + 2].clamp(0.0, 1.0);
            let y2 = boxes_flat[i * 4 + 3].clamp(0.0, 1.0);

            // Skip degenerate boxes.
            if x2 <= x1 || y2 <= y1 {
                continue;
            }
            candidates.push((face_score, [x1, y1, x2, y2]));
        }

        // Sort by confidence descending before NMS.
        candidates.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        let kept = nms(&candidates, IOU_THRESHOLD);

        let faces = kept
            .into_iter()
            .take(MAX_FACES)
            .map(|(_, [x1, y1, x2, y2])| FaceRegion {
                x: x1,
                y: y1,
                width: (x2 - x1).max(0.0),
                height: (y2 - y1).max(0.0),
                eyes_open_prob: None,
                in_focus_prob: None,
            })
            .collect();

        Ok(faces)
    }
}

// ── Global retryable state cell ───────────────────────────────────────────────

struct DetectorCell {
    status: DetectorStatus,
    detector: Option<Arc<FaceDetector>>,
}

static CELL: OnceLock<Mutex<DetectorCell>> = OnceLock::new();

fn cell() -> &'static Mutex<DetectorCell> {
    CELL.get_or_init(|| {
        Mutex::new(DetectorCell {
            status: DetectorStatus::NotStarted,
            detector: None,
        })
    })
}

/// Current status — cheap to call every frame for the settings UI.
pub fn status() -> DetectorStatus {
    cell().lock().unwrap().status.clone()
}

/// Returns the loaded detector if ready, without blocking.
pub fn get_loaded_detector() -> Option<Arc<FaceDetector>> {
    cell().lock().unwrap().detector.clone()
}

/// Download the model if absent, load it, and store the result.
///
/// - No-op if already `Ready`, `LoadFailed`, or `Pending` (in progress).
/// - Retryable after `DownloadFailed` (call again after reconnecting).
/// - Intended to be called from a background thread so the UI never blocks.
///
/// The check and the `Pending` stamp happen **inside a single lock acquisition**
/// so two concurrent callers (startup thread + Retry button) cannot both pass
/// the guard — the second one will see `Pending` and return immediately.
pub fn init_detector() {
    // Atomic check-and-stamp: acquire once, inspect, stamp if appropriate,
    // then release before doing any slow I/O.
    {
        let mut c = cell().lock().unwrap();
        match c.status {
            // Already done or in progress — nothing to do.
            DetectorStatus::Ready | DetectorStatus::LoadFailed | DetectorStatus::Pending => return,
            // First call or explicit retry after a network failure.
            DetectorStatus::NotStarted | DetectorStatus::DownloadFailed => {
                c.status = DetectorStatus::Pending; // stamp while holding the lock
            }
        }
    } // lock released — slow I/O happens below, outside the lock

    let path = match model_path() {
        Ok(p) => p,
        Err(e) => {
            warn!("Cannot determine model cache path: {e}");
            cell().lock().unwrap().status = DetectorStatus::LoadFailed;
            return;
        }
    };

    if let Err(e) = ensure_model(&path) {
        warn!("Face model download failed: {e}");
        cell().lock().unwrap().status = DetectorStatus::DownloadFailed;
        return;
    }

    match FaceDetector::load(&path) {
        Ok(d) => {
            let mut c = cell().lock().unwrap();
            c.detector = Some(Arc::new(d));
            c.status = DetectorStatus::Ready;
            info!("Face detector ready");
        }
        Err(e) => {
            warn!("Face detector load failed: {e}");
            cell().lock().unwrap().status = DetectorStatus::LoadFailed;
        }
    }
}

/// Run face detection. Returns empty vec when the detector is not yet ready.
/// Never panics, never blocks.
pub fn detect_faces(img: &DynamicImage) -> Vec<FaceRegion> {
    get_loaded_detector()
        .and_then(|d| d.run(img).ok())
        .unwrap_or_default()
}

// ── Model management ──────────────────────────────────────────────────────────

fn model_path() -> anyhow::Result<PathBuf> {
    let base = dirs::data_local_dir()
        .ok_or_else(|| anyhow::anyhow!("Cannot locate local data directory"))?;
    Ok(base.join("shotlist").join("models").join(MODEL_NAME))
}

fn ensure_model(path: &Path) -> anyhow::Result<()> {
    if path.exists() {
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    info!("Downloading UltraFace model (~1.6 MB)…");
    let response = ureq::get(MODEL_URL)
        .call()
        .map_err(|e| anyhow::anyhow!("HTTP download failed: {e}"))?;

    let mut bytes = Vec::new();
    response.into_reader().read_to_end(&mut bytes)?;

    std::fs::write(path, &bytes)?;
    info!("Model saved → {}", path.display());
    Ok(())
}

// ── Non-maximum suppression ───────────────────────────────────────────────────

fn nms(candidates: &[(f32, [f32; 4])], iou_threshold: f32) -> Vec<(f32, [f32; 4])> {
    let mut suppressed = vec![false; candidates.len()];
    let mut kept = Vec::new();

    for i in 0..candidates.len() {
        if suppressed[i] {
            continue;
        }
        kept.push(candidates[i]);
        for j in (i + 1)..candidates.len() {
            if !suppressed[j] && box_iou(candidates[i].1, candidates[j].1) > iou_threshold {
                suppressed[j] = true;
            }
        }
    }
    kept
}

fn box_iou(a: [f32; 4], b: [f32; 4]) -> f32 {
    let ix1 = a[0].max(b[0]);
    let iy1 = a[1].max(b[1]);
    let ix2 = a[2].min(b[2]);
    let iy2 = a[3].min(b[3]);

    if ix2 <= ix1 || iy2 <= iy1 {
        return 0.0;
    }

    let inter = (ix2 - ix1) * (iy2 - iy1);
    let area_a = (a[2] - a[0]) * (a[3] - a[1]);
    let area_b = (b[2] - b[0]) * (b[3] - b[1]);
    let union = area_a + area_b - inter;

    if union <= 0.0 {
        0.0
    } else {
        inter / union
    }
}
