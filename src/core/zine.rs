// src/core/zine.rs
// Zine data model: pages, templates, and paper sizes for the PDF export stage.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum PaperSize {
    A4,
    #[default]
    A5,
    Square,
    HalfLetter,
}

impl PaperSize {
    /// Returns (width_mm, height_mm) in portrait orientation.
    pub fn dims_mm(&self) -> (f64, f64) {
        match self {
            PaperSize::A4 => (210.0, 297.0),
            PaperSize::A5 => (148.0, 210.0),
            PaperSize::Square => (200.0, 200.0),
            PaperSize::HalfLetter => (139.7, 215.9),
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            PaperSize::A4 => "a4",
            PaperSize::A5 => "a5",
            PaperSize::Square => "square",
            PaperSize::HalfLetter => "half-letter",
        }
    }

    pub fn from_label(s: &str) -> Self {
        match s {
            "a4" => PaperSize::A4,
            "square" => PaperSize::Square,
            "half-letter" => PaperSize::HalfLetter,
            _ => PaperSize::A5,
        }
    }
}

/// How images and text are arranged on a page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum TemplateId {
    /// Image fills the full page; small caption panel overlaid at the bottom.
    FullBleed,
    /// Image in top 75 %, caption text panel in bottom 25 %.
    #[default]
    Top75,
    /// Image left 50 %, caption text right 50 %.
    Left50,
    /// Image right 50 %, caption text left 50 %.
    Right50,
    /// Two images in a 2×1 grid, no caption.
    GridTwo,
    /// Text-only title / section-break page.
    TitlePage,
    /// Full-bleed image; large italic title + desc overlaid at the bottom on a dark panel.
    PullQuote,
    /// Image occupies top 60 %; large bold headline + description fill the white area below.
    ImageLead,
    /// Image letterbox-fitted to full width, centred on page; caption text below.
    Panoramic,
    /// Image left 65 %, editorial text column right 35 %.
    Editorial,
    /// Three equal vertical panels, each with its own image.
    Triptych,
    /// 2×2 grid of four images.
    Grid4x,
    /// Large hero image top 70 %, two smaller images side by side below.
    FeatureSpread,
    /// One large image left + 4 smaller images in 2×2 grid on the right (5 total).
    PhotoEssay,
    /// Dense 3×3 grid of up to 9 images — classic contact sheet.
    ContactSheet,
}

impl TemplateId {
    pub fn label(&self) -> &'static str {
        match self {
            TemplateId::FullBleed => "fullBleed",
            TemplateId::Top75 => "top75",
            TemplateId::Left50 => "left50",
            TemplateId::Right50 => "right50",
            TemplateId::GridTwo => "gridTwo",
            TemplateId::TitlePage => "titlePage",
            TemplateId::PullQuote => "pullQuote",
            TemplateId::ImageLead => "imageLead",
            TemplateId::Panoramic => "panoramic",
            TemplateId::Editorial => "editorial",
            TemplateId::Triptych => "triptych",
            TemplateId::Grid4x => "grid4x",
            TemplateId::FeatureSpread => "featureSpread",
            TemplateId::PhotoEssay => "photoEssay",
            TemplateId::ContactSheet => "contactSheet",
        }
    }

    pub fn from_label(s: &str) -> Self {
        match s {
            "fullBleed" => TemplateId::FullBleed,
            "left50" => TemplateId::Left50,
            "right50" => TemplateId::Right50,
            "gridTwo" => TemplateId::GridTwo,
            "titlePage" => TemplateId::TitlePage,
            "pullQuote" => TemplateId::PullQuote,
            "imageLead" => TemplateId::ImageLead,
            "panoramic" => TemplateId::Panoramic,
            "editorial" => TemplateId::Editorial,
            "triptych" => TemplateId::Triptych,
            "grid4x" => TemplateId::Grid4x,
            "featureSpread" => TemplateId::FeatureSpread,
            "photoEssay" => TemplateId::PhotoEssay,
            "contactSheet" => TemplateId::ContactSheet,
            _ => TemplateId::Top75,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ZinePage {
    pub id: String,
    pub zine_id: String,
    pub position: i64,
    pub image_id: Option<String>,
    pub image_id_2: Option<String>,
    pub image_id_3: Option<String>,
    pub image_id_4: Option<String>,
    pub image_id_5: Option<String>,
    pub image_id_6: Option<String>,
    pub image_id_7: Option<String>,
    pub image_id_8: Option<String>,
    pub image_id_9: Option<String>,
    pub template: TemplateId,
    pub caption_override: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Zine {
    pub id: String,
    pub session_id: String,
    pub paper_size: PaperSize,
    pub pages: Vec<ZinePage>,
}
