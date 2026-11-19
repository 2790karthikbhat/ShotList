// src/ai/scoring.rs
// Scene-level ranking: group images into scenes, then rank within each scene.
//
// A "scene" is a run of images captured close together in time (configurable gap).
// Within a scene, images are ranked by a weighted composite of local + AI scores.

use crate::core::image::{AiAssessment, ImageEntry};
use std::time::Duration;

/// Maximum gap between consecutive images to be considered the same scene.
pub const DEFAULT_SCENE_GAP: Duration = Duration::from_secs(10);

/// Weights for the composite quality score.
#[derive(Debug, Clone)]
pub struct ScoringWeights {
    pub focus: f32,    // Laplacian focus score
    pub eyes: f32,     // average eyes-open probability across faces
    pub ai_scene: f32, // AI provider's scene_rank
}

impl Default for ScoringWeights {
    fn default() -> Self {
        Self {
            focus: 0.5,
            eyes: 0.3,
            ai_scene: 0.2,
        }
    }
}

/// Composite quality score in [0, 1].
pub fn composite_score(assessment: &AiAssessment, weights: &ScoringWeights) -> f32 {
    let focus = assessment.focus_score.unwrap_or(0.5);

    let eyes = if assessment.faces.is_empty() {
        0.5 // neutral if no faces
    } else {
        let sum: f32 = assessment
            .faces
            .iter()
            .filter_map(|f| f.eyes_open_prob)
            .sum();
        sum / assessment.faces.len() as f32
    };

    let scene = assessment.scene_rank.unwrap_or(0.5);

    // Normalise weights
    let total = weights.focus + weights.eyes + weights.ai_scene;
    (weights.focus * focus + weights.eyes * eyes + weights.ai_scene * scene) / total
}

/// Group a sorted-by-time slice of images into scenes.
/// Returns Vec<Vec<usize>> — each inner Vec is indices into `entries`.
pub fn group_into_scenes(entries: &[ImageEntry], gap: Duration) -> Vec<Vec<usize>> {
    let mut scenes: Vec<Vec<usize>> = Vec::new();
    let mut current_scene: Vec<usize> = Vec::new();

    for (i, entry) in entries.iter().enumerate() {
        if current_scene.is_empty() {
            current_scene.push(i);
            continue;
        }

        let prev_idx = *current_scene.last().unwrap();
        let prev_time = entries[prev_idx].captured_at;
        let curr_time = entry.captured_at;

        let should_split = match (prev_time, curr_time) {
            (Some(prev), Some(curr)) => {
                let diff = (curr - prev).to_std().unwrap_or(Duration::ZERO);
                diff > gap
            }
            _ => false,
        };

        if should_split {
            scenes.push(std::mem::take(&mut current_scene));
        }
        current_scene.push(i);
    }

    if !current_scene.is_empty() {
        scenes.push(current_scene);
    }

    scenes
}
