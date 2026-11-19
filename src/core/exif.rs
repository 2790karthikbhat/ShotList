// src/core/exif.rs
// Shared EXIF extraction used by both the import scanner and the thumbnailer.
//
// Two entry points:
//   from_rawler_exif(&rawler::exif::Exif) -> ImageExif
//     — called from the thumbnailer where a rawler decoder is already open.
//   read_raw_exif(&Path) -> Option<ImageExif>
//     — called at import time; opens the file just to read metadata headers,
//       no pixel decode, ~1 ms per ARW file.

use chrono::{DateTime, TimeZone, Utc};
use std::path::Path;

// ── Value type ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct ImageExif {
    pub iso: Option<u32>,
    /// Human-readable shutter speed, e.g. "1/500" or "2\"".
    pub shutter_speed: Option<String>,
    /// F-number, e.g. 2.8.
    pub aperture: Option<f32>,
    /// Focal length in mm.
    pub focal_length_mm: Option<f32>,
    /// Lens model string from EXIF, e.g. "FE 24-70mm F2.8 GM II".
    pub lens_model: Option<String>,
    /// UTC capture time (from DateTimeOriginal + SubSecTimeOriginal).
    pub captured_at: Option<DateTime<Utc>>,
}

// ── Extraction from a rawler Exif struct ─────────────────────────────────────

pub fn from_rawler_exif(exif: &rawler::exif::Exif) -> ImageExif {
    // ISO — prefer iso_speed_ratings (widely supported), fall back to iso_speed.
    let iso = exif.iso_speed_ratings.map(|v| v as u32).or(exif.iso_speed);

    // Shutter speed: Rational{n, d} → "1/500" / "2\"" (long exposure).
    let shutter_speed = exif.exposure_time.map(|r| {
        if r.d == 0 {
            return String::from("?");
        }
        if r.d == 1 {
            return format!("{}\"", r.n);
        }
        if r.n == 1 {
            return format!("1/{}", r.d);
        }
        format!("{}/{}", r.n, r.d)
    });

    let aperture = exif
        .fnumber
        .filter(|r| r.d > 0)
        .map(|r| r.n as f32 / r.d as f32);

    let focal_length_mm = exif
        .focal_length
        .filter(|r| r.d > 0)
        .map(|r| r.n as f32 / r.d as f32);

    let captured_at = parse_datetime(
        exif.date_time_original.as_deref(),
        exif.sub_sec_time_original.as_deref(),
    );

    ImageExif {
        iso,
        shutter_speed,
        aperture,
        focal_length_mm,
        lens_model: exif.lens_model.clone(),
        captured_at,
    }
}

// ── Metadata-only read at import time ─────────────────────────────────────────

/// Open a RAW file, read only its metadata headers, and return EXIF fields.
/// Never decodes pixel data — cost is ~1 ms per ARW/NEF/CR2 file.
/// Returns `None` on any error (unsupported format, truncated file, etc.).
pub fn read_raw_exif(path: &Path) -> Option<ImageExif> {
    use rawler::{decoders::RawDecodeParams, rawsource::RawSource};

    let source = RawSource::new(path).ok()?;
    let decoder = rawler::get_decoder(&source).ok()?;
    let meta = decoder
        .raw_metadata(&source, &RawDecodeParams::default())
        .ok()?;
    Some(from_rawler_exif(&meta.exif))
}

// ── Date/time helpers ─────────────────────────────────────────────────────────

/// Parse EXIF datetime "YYYY:MM:DD HH:MM:SS" + optional sub-second digits.
pub fn parse_datetime(dt: Option<&str>, sub_sec: Option<&str>) -> Option<DateTime<Utc>> {
    let s = dt?;
    if s.len() < 19 {
        return None;
    }

    let year: i32 = s[0..4].parse().ok()?;
    let month: u32 = s[5..7].parse().ok()?;
    let day: u32 = s[8..10].parse().ok()?;
    let hour: u32 = s[11..13].parse().ok()?;
    let min: u32 = s[14..16].parse().ok()?;
    let sec: u32 = s[17..19].parse().ok()?;

    let base = Utc
        .with_ymd_and_hms(year, month, day, hour, min, sec)
        .single()?;

    // Sub-second: "70" → 0.70 s → 700_000 µs.  We store at microsecond granularity
    // which is enough for burst-grouping purposes.
    //
    // Some high-fps cameras (Sony A1, Nikon Z9) write 7+ digit SubSec strings.
    // Without clamping, a 7-digit value like "9999999" would add ~10 seconds to
    // the timestamp, breaking burst grouping.  We take only the first 6 digits.
    if let Some(ss) = sub_sec.filter(|s| !s.is_empty()) {
        let trimmed = ss.trim();
        let clamped = &trimmed[..trimmed.len().min(6)]; // ← cap at µs precision
        if let Ok(digits) = clamped.parse::<u32>() {
            let factor = 10u32.pow(6 - clamped.len() as u32);
            let micros = (digits * factor) as i64;
            return Some(base + chrono::Duration::microseconds(micros));
        }
    }

    Some(base)
}
