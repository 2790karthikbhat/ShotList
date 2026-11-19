// src/core/xmp.rs
// XMP sidecar read/write — the universal interchange layer between ShotList and
// Affinity Photo, Adobe Bridge, Lightroom, and any other DAM tool.
//
// Sidecar path: same directory and stem as the source file, `.xmp` extension.
//   DSC00001.ARW  →  DSC00001.xmp
//
// Fields written
// ─────────────
//   xmp:Rating      0–5 star rating
//   lr:pickStatus   1=pick, -1=reject, 0=unflagged   (Lightroom / Affinity convention)
//   xmp:Label       colour label string ("Red", "Orange", …) or absent
//   sl:cullState    full ShotList state ("pick","reject","flagged","unreviewed")
//                   — private namespace for lossless round-trip of Flagged vs Unreviewed
//
// Fields read (from files written by any tool)
// ─────────────────────────────────────────────
//   lr:pickStatus   primary source for cull state
//   xmp:Rating      star rating
//   xmp:Label       colour label
//   sl:cullState    overrides lr:pickStatus when present (ShotList wrote this file)
//   culld:cullState legacy alias — sidecars from earlier builds are still recognised

use crate::core::caption::Caption;
use crate::core::image::{ColorLabel, CullState, ImageEntry};
use crate::core::session::Session;
use std::path::{Path, PathBuf};
use tracing::{debug, warn};

// ── Path helper ───────────────────────────────────────────────────────────────

/// Return the sidecar path for a given image path.
pub fn sidecar_path(image_path: &Path) -> PathBuf {
    image_path.with_extension("xmp")
}

// ── Parsed sidecar data ───────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct SidecarData {
    pub cull_state: CullState,
    pub rating: u8,
    pub color_label: Option<ColorLabel>,
}

// ── Write ─────────────────────────────────────────────────────────────────────

/// Write (or overwrite) the XMP sidecar for a single image entry.
pub fn write_sidecar(entry: &ImageEntry) -> anyhow::Result<()> {
    let path = sidecar_path(&entry.path);
    let xml = render_xmp(entry, None);
    std::fs::write(&path, xml.as_bytes())?;
    debug!("XMP written → {}", path.display());
    Ok(())
}

/// Write XMP sidecar including editorial caption fields.
/// Overwrites any existing sidecar.
pub fn write_sidecar_with_caption(entry: &ImageEntry, caption: &Caption) -> anyhow::Result<()> {
    let path = sidecar_path(&entry.path);
    let xml = render_xmp(entry, Some(caption));
    std::fs::write(&path, xml.as_bytes())?;
    debug!("XMP+caption written → {}", path.display());
    Ok(())
}

/// Write XMP sidecars for every image in the session.
/// Used for the "Write all XMP" batch button.
pub fn write_all(session: &Session) {
    let mut written = 0usize;
    let mut failed = 0usize;
    for id in &session.image_ids {
        let entry = &session.images[id];
        match write_sidecar(entry) {
            Ok(_) => written += 1,
            Err(e) => {
                warn!("XMP write failed for {:?}: {e}", entry.path);
                failed += 1;
            }
        }
    }
    tracing::info!("Write all XMP: {written} written, {failed} failed");
}

fn render_xmp(entry: &ImageEntry, caption: Option<&Caption>) -> String {
    let pick_status = match entry.cull_state {
        CullState::Pick => "1",
        CullState::Reject => "-1",
        CullState::Flagged => "0",
        CullState::Unreviewed => "0",
    };

    let label_attr = entry
        .color_label
        .as_ref()
        .map(|l| format!("\n      xmp:Label=\"{}\"", color_label_str(l)))
        .unwrap_or_default();

    // Build caption XML blocks when caption data is present.
    let caption_xml = caption
        .filter(|c| !c.title.is_empty() || !c.description.is_empty() || !c.keywords.is_empty())
        .map(|c| {
            let mut parts = String::new();

            if !c.title.is_empty() {
                parts.push_str(&format!(
                    "      <dc:title><rdf:Alt><rdf:li xml:lang=\"x-default\">{}</rdf:li></rdf:Alt></dc:title>\n",
                    xml_escape(&c.title)
                ));
            }
            if !c.description.is_empty() {
                parts.push_str(&format!(
                    "      <dc:description><rdf:Alt><rdf:li xml:lang=\"x-default\">{}</rdf:li></rdf:Alt></dc:description>\n",
                    xml_escape(&c.description)
                ));
            }
            if !c.keywords.is_empty() {
                let items: String = c.keywords.iter()
                    .map(|k| format!("        <rdf:li>{}</rdf:li>\n", xml_escape(k)))
                    .collect();
                parts.push_str(&format!(
                    "      <dc:subject><rdf:Bag>\n{items}      </rdf:Bag></dc:subject>\n"
                ));
            }
            if !c.location.is_empty() {
                parts.push_str(&format!(
                    "      <Iptc4xmpCore:Location>{}</Iptc4xmpCore:Location>\n",
                    xml_escape(&c.location)
                ));
            }
            if !c.credit.is_empty() {
                parts.push_str(&format!(
                    "      <dc:creator><rdf:Seq><rdf:li>{}</rdf:li></rdf:Seq></dc:creator>\n",
                    xml_escape(&c.credit)
                ));
            }
            parts
        })
        .unwrap_or_default();

    let has_caption = !caption_xml.is_empty();

    // culld:cullState preserves Flagged vs Unreviewed across round-trips
    // (lr:pickStatus maps both to "0" so we can't distinguish them otherwise).
    if has_caption {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" x:xmptk="ShotList 0.1">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description rdf:about=""
      xmlns:xmp="http://ns.adobe.com/xap/1.0/"
      xmlns:lr="http://ns.adobe.com/lightroom/1.0/"
      xmlns:sl="https://shotlist.app/xmp/1.0/"
      xmlns:dc="http://purl.org/dc/elements/1.1/"
      xmlns:Iptc4xmpCore="http://iptc.org/std/Iptc4xmpCore/1.0/xmlns/"
      xmp:Rating="{rating}"
      lr:pickStatus="{pick}"
      sl:cullState="{state}"{label}>
{caption_xml}    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>
"#,
            rating = entry.rating,
            pick = pick_status,
            state = entry.cull_state.label(),
            label = label_attr,
            caption_xml = caption_xml,
        )
    } else {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" x:xmptk="ShotList 0.1">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description rdf:about=""
      xmlns:xmp="http://ns.adobe.com/xap/1.0/"
      xmlns:lr="http://ns.adobe.com/lightroom/1.0/"
      xmlns:sl="https://shotlist.app/xmp/1.0/"
      xmp:Rating="{rating}"
      lr:pickStatus="{pick}"
      sl:cullState="{state}"{label}/>
  </rdf:RDF>
</x:xmpmeta>
"#,
            rating = entry.rating,
            pick = pick_status,
            state = entry.cull_state.label(),
            label = label_attr,
        )
    }
}

/// Escape the five predefined XML entities.
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn color_label_str(label: &ColorLabel) -> &'static str {
    match label {
        ColorLabel::Red => "Red",
        ColorLabel::Orange => "Orange",
        ColorLabel::Yellow => "Yellow",
        ColorLabel::Green => "Green",
        ColorLabel::Blue => "Blue",
        ColorLabel::Purple => "Purple",
    }
}

// ── Read ──────────────────────────────────────────────────────────────────────

/// Try to read a pre-existing XMP sidecar.  Returns `None` on any error or if
/// the file does not exist.  Used at import time to seed decisions from
/// Affinity / Bridge / Lightroom.
pub fn read_sidecar(xmp_path: &Path) -> Option<SidecarData> {
    let xml = std::fs::read_to_string(xmp_path).ok()?;
    let mut data = SidecarData::default();

    // sl:cullState — full fidelity when ShotList wrote the file.
    // Also accept the legacy culld:cullState written by earlier builds.
    let sl_state = attr_value(&xml, "sl:cullState").or_else(|| attr_value(&xml, "culld:cullState"));
    if let Some(s) = sl_state {
        data.cull_state = CullState::from_label(s);
    } else {
        // Fall back to the standard lr:pickStatus.
        match attr_value(&xml, "lr:pickStatus").and_then(|v| v.parse::<i32>().ok()) {
            Some(1) => data.cull_state = CullState::Pick,
            Some(-1) => data.cull_state = CullState::Reject,
            _ => data.cull_state = CullState::Unreviewed,
        }
    }

    // xmp:Rating
    data.rating = attr_value(&xml, "xmp:Rating")
        .and_then(|v| v.parse::<u8>().ok())
        .unwrap_or(0)
        .min(5);

    // xmp:Label
    data.color_label = attr_value(&xml, "xmp:Label").and_then(parse_color_label);

    Some(data)
}

/// Extract the value of `attr="..."` from an XMP attribute string.
///
/// The search is anchored to a whitespace character immediately before the
/// attribute name, preventing false matches where an attribute *value* contains
/// text that looks like `lr:pickStatus="1"`.
fn attr_value<'a>(xml: &'a str, attr: &str) -> Option<&'a str> {
    for quote in ['"', '\''] {
        // XMP attributes are always preceded by at least one whitespace character
        // (space, newline, or tab).  Requiring that prefix rules out matches
        // inside attribute values.
        for ws in [' ', '\n', '\t', '\r'] {
            let key = format!("{ws}{attr}={quote}");
            if let Some(pos) = xml.find(&key) {
                let rest = &xml[pos + key.len()..];
                if let Some(end) = rest.find(quote) {
                    return Some(&rest[..end]);
                }
            }
        }
    }
    None
}

fn parse_color_label(s: &str) -> Option<ColorLabel> {
    match s {
        "Red" => Some(ColorLabel::Red),
        "Orange" => Some(ColorLabel::Orange),
        "Yellow" => Some(ColorLabel::Yellow),
        "Green" => Some(ColorLabel::Green),
        "Blue" => Some(ColorLabel::Blue),
        "Purple" => Some(ColorLabel::Purple),
        _ => None,
    }
}

// ── Open in Affinity (macOS only) ─────────────────────────────────────────────

/// Open `paths` in Affinity Photo 2 (or Affinity Photo as a fallback).
/// Returns an error string if Affinity is not installed — the caller shows it
/// as a status message rather than panicking.
pub fn open_in_affinity(paths: &[PathBuf]) -> anyhow::Result<()> {
    if paths.is_empty() {
        anyhow::bail!("No files to open");
    }
    open_in_affinity_impl(paths)
}

#[cfg(target_os = "macos")]
fn open_in_affinity_impl(paths: &[PathBuf]) -> anyhow::Result<()> {
    for app in &["Affinity Photo 2", "Affinity"] {
        let ok = std::process::Command::new("open")
            .arg("-a")
            .arg(app)
            .args(paths)
            .status()
            .is_ok_and(|s| s.success());
        if ok {
            tracing::info!("Opened {} file(s) in {app}", paths.len());
            return Ok(());
        }
    }
    anyhow::bail!(
        "Affinity Photo not found — install it from the Mac App Store \
         (https://affinity.serif.com)"
    )
}

#[cfg(not(target_os = "macos"))]
fn open_in_affinity_impl(_paths: &[PathBuf]) -> anyhow::Result<()> {
    anyhow::bail!("Open in Affinity is currently macOS-only")
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::image::{ImageEntry, ImageFormat};
    use chrono::Utc;
    use tempfile::tempdir;

    fn make_entry(cull_state: CullState, rating: u8) -> ImageEntry {
        let dir = tempdir().unwrap();
        let path = dir.path().join("DSC00001.ARW");
        std::fs::write(&path, b"fake").unwrap();
        let mut e = ImageEntry::new(path, ImageFormat::Arw, 4, Utc::now());
        e.cull_state = cull_state;
        e.rating = rating;
        // Keep dir alive by leaking it for the test duration.
        std::mem::forget(dir);
        e
    }

    #[test]
    fn round_trip_pick() {
        let entry = make_entry(CullState::Pick, 4);
        write_sidecar(&entry).unwrap();
        let data = read_sidecar(&sidecar_path(&entry.path)).unwrap();
        assert_eq!(data.cull_state, CullState::Pick);
        assert_eq!(data.rating, 4);
    }

    #[test]
    fn round_trip_reject() {
        let entry = make_entry(CullState::Reject, 0);
        write_sidecar(&entry).unwrap();
        let data = read_sidecar(&sidecar_path(&entry.path)).unwrap();
        assert_eq!(data.cull_state, CullState::Reject);
    }

    #[test]
    fn round_trip_flagged_preserves_state() {
        // Flagged maps to lr:pickStatus="0" just like Unreviewed; only
        // culld:cullState distinguishes them on read-back.
        let entry = make_entry(CullState::Flagged, 0);
        write_sidecar(&entry).unwrap();
        let data = read_sidecar(&sidecar_path(&entry.path)).unwrap();
        assert_eq!(data.cull_state, CullState::Flagged);
    }

    #[test]
    fn read_external_xmp_pick() {
        // Simulate a file written by Lightroom / Affinity (no culld: namespace).
        let dir = tempdir().unwrap();
        let xmp = dir.path().join("DSC00002.xmp");
        std::fs::write(
            &xmp,
            r#"<?xml version="1.0"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description rdf:about=""
      xmlns:xmp="http://ns.adobe.com/xap/1.0/"
      xmlns:lr="http://ns.adobe.com/lightroom/1.0/"
      xmp:Rating="3"
      lr:pickStatus="1"/>
  </rdf:RDF>
</x:xmpmeta>"#,
        )
        .unwrap();
        let data = read_sidecar(&xmp).unwrap();
        assert_eq!(data.cull_state, CullState::Pick);
        assert_eq!(data.rating, 3);
    }

    #[test]
    fn missing_sidecar_returns_none() {
        assert!(read_sidecar(Path::new("/tmp/no_such_file.xmp")).is_none());
    }
}
