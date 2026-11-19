// src/core/pdf.rs
// Zine PDF renderer using printpdf 0.9.1.
//
// Coordinate system: PDF origin is at bottom-left; Y increases upward.
// All dimensions are in millimetres; converted to Pt (1 pt = 1/72 inch,
// 1 mm = 72/25.4 pt) when calling the printpdf API.

use anyhow::Result;
use printpdf::{
    BuiltinFont, Color, Greyscale, Mm, Op, PdfDocument, PdfFontHandle, PdfPage, PdfSaveOptions, Pt,
    RawImage, TextItem, TextMatrix, XObjectTransform,
};
use std::collections::HashMap;

use crate::core::caption::Caption;
use crate::core::zine::{PaperSize, TemplateId, Zine, ZinePage};

const DPI: f32 = 300.0;
const MARGIN_MM: f64 = 6.0;

fn mm_to_pt(mm: f64) -> f64 {
    mm * 72.0 / 25.4
}

/// Letterbox-fit XObjectTransform: scale to fit inside the area, centred (no cropping).
/// Use this when you intentionally want white margins (Panoramic template).
fn letterbox_transform(
    raw: &RawImage,
    area_x: f64,
    area_y: f64,
    area_w: f64,
    area_h: f64,
) -> XObjectTransform {
    let nat_w_pt = raw.width as f64 * 72.0 / DPI as f64;
    let nat_h_pt = raw.height as f64 * 72.0 / DPI as f64;
    let area_w_pt = mm_to_pt(area_w);
    let area_h_pt = mm_to_pt(area_h);
    let scale = f64::min(area_w_pt / nat_w_pt, area_h_pt / nat_h_pt);
    let x_off = (area_w_pt - nat_w_pt * scale) / 2.0;
    let y_off = (area_h_pt - nat_h_pt * scale) / 2.0;
    XObjectTransform {
        translate_x: Some(Pt((mm_to_pt(area_x) + x_off) as f32)),
        translate_y: Some(Pt((mm_to_pt(area_y) + y_off) as f32)),
        scale_x: Some(scale as f32),
        scale_y: Some(scale as f32),
        dpi: Some(DPI),
        rotate: None,
    }
}

/// Cover-fit XObjectTransform: scale so the image FILLS the area (like CSS object-fit:cover).
/// The image is centred; any overflow will be hidden by the caller's clip rectangle.
fn cover_transform(
    raw: &RawImage,
    area_x: f64,
    area_y: f64,
    area_w: f64,
    area_h: f64,
) -> XObjectTransform {
    let px_w = raw.width as f64;
    let px_h = raw.height as f64;

    let nat_w_pt = px_w * 72.0 / DPI as f64;
    let nat_h_pt = px_h * 72.0 / DPI as f64;

    let area_w_pt = mm_to_pt(area_w);
    let area_h_pt = mm_to_pt(area_h);

    // Cover scale: use MAX so the image fills the entire area (may overflow)
    let scale = f64::max(area_w_pt / nat_w_pt, area_h_pt / nat_h_pt);

    let actual_w = nat_w_pt * scale;
    let actual_h = nat_h_pt * scale;

    // Center (negative offset means we start outside the area — that's the crop)
    let x_off = (area_w_pt - actual_w) / 2.0;
    let y_off = (area_h_pt - actual_h) / 2.0;

    let tx = mm_to_pt(area_x) + x_off;
    let ty = mm_to_pt(area_y) + y_off;

    XObjectTransform {
        translate_x: Some(Pt(tx as f32)),
        translate_y: Some(Pt(ty as f32)),
        scale_x: Some(scale as f32),
        scale_y: Some(scale as f32),
        dpi: Some(DPI),
        rotate: None,
    }
}

/// Place an image with cover fit, clipping to the designated rectangle.
fn place_image_cover(
    doc: &mut PdfDocument,
    ops: &mut Vec<Op>,
    raw: &RawImage,
    area_x: f64,
    area_y: f64,
    area_w: f64,
    area_h: f64,
) {
    use printpdf::graphics::{LinePoint, PaintMode, Polygon, PolygonRing, WindingOrder};
    use printpdf::Point as Pt2;

    let x1 = Mm(area_x as f32);
    let y1 = Mm(area_y as f32);
    let x2 = Mm((area_x + area_w) as f32);
    let y2 = Mm((area_y + area_h) as f32);

    ops.push(Op::SaveGraphicsState);

    // Clip to the image area so cover-overflow is hidden
    ops.push(Op::DrawPolygon {
        polygon: Polygon {
            rings: vec![PolygonRing {
                points: vec![
                    LinePoint {
                        p: Pt2::new(x1, y1),
                        bezier: false,
                    },
                    LinePoint {
                        p: Pt2::new(x2, y1),
                        bezier: false,
                    },
                    LinePoint {
                        p: Pt2::new(x2, y2),
                        bezier: false,
                    },
                    LinePoint {
                        p: Pt2::new(x1, y2),
                        bezier: false,
                    },
                ],
            }],
            mode: PaintMode::Clip,
            winding_order: WindingOrder::NonZero,
        },
    });

    let id = doc.add_image(raw);
    ops.push(Op::UseXobject {
        id,
        transform: cover_transform(raw, area_x, area_y, area_w, area_h),
    });

    ops.push(Op::RestoreGraphicsState);
}

/// Simple word-wrap.  Returns lines no longer than `max_chars` characters.
fn wrap_text(text: &str, max_chars: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.is_empty() {
            current.push_str(word);
        } else if current.len() + 1 + word.len() <= max_chars {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current);
            current = word.to_string();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

/// Append text ops to render a block of wrapped text.
///
/// Returns the number of lines written (so callers can advance the cursor).
// 25 call sites across this file; a params struct isn't worth the mechanical
// rewrite risk for a style-only lint on a private helper.
#[allow(clippy::too_many_arguments)]
fn push_text_block(
    ops: &mut Vec<Op>,
    text: &str,
    x_mm: f64,
    y_mm: f64,
    font: PdfFontHandle,
    pt_size: f32,
    line_gap: f64,
    max_chars: usize,
    max_lines: usize,
) -> usize {
    if text.is_empty() {
        return 0;
    }
    let lines = wrap_text(text, max_chars);
    let limited: Vec<&str> = lines.iter().take(max_lines).map(|s| s.as_str()).collect();
    let count = limited.len();

    ops.push(Op::StartTextSection);
    ops.push(Op::SetFont {
        font,
        size: Pt(pt_size),
    });
    ops.push(Op::SetFillColor {
        col: Color::Greyscale(Greyscale {
            percent: 0.0,
            icc_profile: None,
        }),
    });

    for (i, line) in limited.iter().enumerate() {
        let y = y_mm - (i as f64 * line_gap);
        // Use SetTextMatrix (Tm) for absolute positioning — SetTextCursor maps to
        // Td which is relative to the text line matrix and accumulates incorrectly.
        ops.push(Op::SetTextMatrix {
            matrix: TextMatrix::Translate(Pt(mm_to_pt(x_mm) as f32), Pt(mm_to_pt(y) as f32)),
        });
        ops.push(Op::ShowText {
            items: vec![TextItem::Text(line.to_string())],
        });
    }

    ops.push(Op::EndTextSection);
    count
}

/// Render a filled grey rectangle (for caption panel background).
fn push_rect_fill(ops: &mut Vec<Op>, x_mm: f64, y_mm: f64, w_mm: f64, h_mm: f64, grey: f32) {
    use printpdf::{
        graphics::{PaintMode, Polygon, PolygonRing, WindingOrder},
        Point as Pt2,
    };
    let x = Mm(x_mm as f32);
    let y = Mm(y_mm as f32);

    // Build a rectangle polygon
    let bl = Pt2::new(x, y);
    let br = Pt2::new(Mm((x_mm + w_mm) as f32), y);
    let tr = Pt2::new(Mm((x_mm + w_mm) as f32), Mm((y_mm + h_mm) as f32));
    let tl = Pt2::new(x, Mm((y_mm + h_mm) as f32));

    ops.push(Op::SetFillColor {
        col: Color::Greyscale(Greyscale {
            percent: grey,
            icc_profile: None,
        }),
    });
    ops.push(Op::DrawPolygon {
        polygon: Polygon {
            rings: vec![PolygonRing {
                points: vec![
                    printpdf::graphics::LinePoint {
                        p: bl,
                        bezier: false,
                    },
                    printpdf::graphics::LinePoint {
                        p: br,
                        bezier: false,
                    },
                    printpdf::graphics::LinePoint {
                        p: tr,
                        bezier: false,
                    },
                    printpdf::graphics::LinePoint {
                        p: tl,
                        bezier: false,
                    },
                ],
            }],
            mode: PaintMode::Fill,
            winding_order: WindingOrder::NonZero,
        },
    });
}

/// Render one zine page to a `PdfPage`.
fn render_page(
    doc: &mut PdfDocument,
    page: &ZinePage,
    paper: &PaperSize,
    images: &HashMap<String, RawImage>,
    captions: &HashMap<String, Caption>,
) -> PdfPage {
    let (w, h) = paper.dims_mm();
    let mut ops: Vec<Op> = Vec::new();

    // caption_override or plain-text fallback — no emoji (Helvetica is Latin-1 only).
    let caption_text: Option<String> = page.caption_override.clone().or_else(|| {
        page.image_id
            .as_ref()
            .and_then(|id| captions.get(id))
            .map(|c| {
                let mut parts = Vec::new();
                if !c.title.is_empty() {
                    parts.push(c.title.clone());
                }
                if !c.description.is_empty() {
                    parts.push(c.description.clone());
                }
                if !c.location.is_empty() {
                    parts.push(c.location.clone());
                }
                parts.join(" — ")
            })
    });

    let raw: Option<&RawImage> = page.image_id.as_ref().and_then(|id| images.get(id));

    match page.template {
        TemplateId::FullBleed => {
            if let Some(raw) = raw {
                place_image_cover(doc, &mut ops, raw, 0.0, 0.0, w, h);
            }

            let usable_w = w - 2.0 * MARGIN_MM;
            let max_chars = (usable_w / 1.9) as usize;

            if let Some(cap) = page.image_id.as_ref().and_then(|id| captions.get(id)) {
                if !cap.title.is_empty() || !cap.description.is_empty() {
                    // Pre-count lines so the grey panel is exactly as tall as the text.
                    let title_n = if cap.title.is_empty() {
                        0
                    } else {
                        wrap_text(&cap.title, max_chars).len().min(2)
                    };
                    let desc_n = if cap.description.is_empty() {
                        0
                    } else {
                        wrap_text(&cap.description, max_chars).len().min(2)
                    };
                    let sep = if title_n > 0 && desc_n > 0 { 1.5 } else { 0.0 };
                    let panel_h =
                        (title_n as f64 * 3.8) + (desc_n as f64 * 3.2) + sep + 2.0 * MARGIN_MM;

                    push_rect_fill(&mut ops, 0.0, 0.0, w, panel_h, 0.93);
                    let mut y = panel_h - MARGIN_MM;

                    if title_n > 0 {
                        let rendered = push_text_block(
                            &mut ops,
                            &cap.title,
                            MARGIN_MM,
                            y,
                            PdfFontHandle::Builtin(BuiltinFont::HelveticaBold),
                            8.5,
                            3.8,
                            max_chars,
                            2,
                        );
                        y -= rendered as f64 * 3.8 + sep;
                    }
                    if desc_n > 0 && y > MARGIN_MM {
                        push_text_block(
                            &mut ops,
                            &cap.description,
                            MARGIN_MM,
                            y,
                            PdfFontHandle::Builtin(BuiltinFont::Helvetica),
                            7.0,
                            3.2,
                            max_chars,
                            2,
                        );
                    }
                }
            } else if let Some(text) = &caption_text {
                let line_n = wrap_text(text, max_chars).len().min(3);
                let panel_h = (line_n as f64 * 3.5) + 2.0 * MARGIN_MM;
                push_rect_fill(&mut ops, 0.0, 0.0, w, panel_h, 0.93);
                push_text_block(
                    &mut ops,
                    text,
                    MARGIN_MM,
                    panel_h - MARGIN_MM,
                    PdfFontHandle::Builtin(BuiltinFont::Helvetica),
                    8.0,
                    3.5,
                    max_chars,
                    3,
                );
            }
        }

        TemplateId::Top75 => {
            let img_h = h * 0.75;
            let panel_h = h - img_h;

            // White caption panel background
            push_rect_fill(&mut ops, 0.0, 0.0, w, panel_h, 1.0);

            if let Some(raw) = raw {
                place_image_cover(doc, &mut ops, raw, 0.0, panel_h, w, img_h);
            }

            // Caption: title bold, then description
            let usable_w = w - 2.0 * MARGIN_MM;
            let max_chars = (usable_w / 1.8) as usize;
            let mut y_cursor = panel_h - MARGIN_MM;

            if let (Some(image_id), Some(cap)) = (
                &page.image_id,
                page.image_id.as_ref().and_then(|id| captions.get(id)),
            ) {
                let _ = image_id;
                if !cap.title.is_empty() {
                    let lines = push_text_block(
                        &mut ops,
                        &cap.title,
                        MARGIN_MM,
                        y_cursor,
                        PdfFontHandle::Builtin(BuiltinFont::HelveticaBold),
                        9.0,
                        3.8,
                        max_chars,
                        2,
                    );
                    y_cursor -= lines as f64 * 3.8 + 1.5;
                }
                if !cap.description.is_empty() && y_cursor > MARGIN_MM {
                    push_text_block(
                        &mut ops,
                        &cap.description,
                        MARGIN_MM,
                        y_cursor,
                        PdfFontHandle::Builtin(BuiltinFont::Helvetica),
                        7.5,
                        3.2,
                        max_chars,
                        3,
                    );
                }
            } else if let Some(text) = &caption_text {
                push_text_block(
                    &mut ops,
                    text,
                    MARGIN_MM,
                    y_cursor,
                    PdfFontHandle::Builtin(BuiltinFont::Helvetica),
                    8.0,
                    3.5,
                    max_chars,
                    3,
                );
            }
        }

        TemplateId::Left50 | TemplateId::Right50 => {
            let img_w = w * 0.5;
            let text_w = w - img_w;
            let (img_x, text_x) = if matches!(page.template, TemplateId::Left50) {
                (0.0, img_w)
            } else {
                (text_w, 0.0)
            };

            push_rect_fill(&mut ops, text_x, 0.0, text_w, h, 1.0);

            if let Some(raw) = raw {
                place_image_cover(doc, &mut ops, raw, img_x, 0.0, img_w, h);
            }

            let usable_w = text_w - 2.0 * MARGIN_MM;
            let max_chars = (usable_w / 2.0) as usize;
            let top_y = h - MARGIN_MM;

            if let Some(cap) = page.image_id.as_ref().and_then(|id| captions.get(id)) {
                let mut y = top_y;
                if !cap.title.is_empty() {
                    let lines = push_text_block(
                        &mut ops,
                        &cap.title,
                        text_x + MARGIN_MM,
                        y,
                        PdfFontHandle::Builtin(BuiltinFont::HelveticaBold),
                        10.0,
                        4.5,
                        max_chars,
                        3,
                    );
                    y -= lines as f64 * 4.5 + 3.0;
                }
                if !cap.description.is_empty() {
                    let lines = push_text_block(
                        &mut ops,
                        &cap.description,
                        text_x + MARGIN_MM,
                        y,
                        PdfFontHandle::Builtin(BuiltinFont::Helvetica),
                        8.5,
                        3.8,
                        max_chars,
                        8,
                    );
                    y -= lines as f64 * 3.8 + 2.0;
                }
                if !cap.location.is_empty() {
                    push_text_block(
                        &mut ops,
                        &cap.location,
                        text_x + MARGIN_MM,
                        y,
                        PdfFontHandle::Builtin(BuiltinFont::HelveticaOblique),
                        8.0,
                        3.5,
                        max_chars,
                        1,
                    );
                }
            } else if let Some(text) = &caption_text {
                push_text_block(
                    &mut ops,
                    text,
                    text_x + MARGIN_MM,
                    top_y,
                    PdfFontHandle::Builtin(BuiltinFont::Helvetica),
                    8.5,
                    3.8,
                    max_chars,
                    6,
                );
            }
        }

        TemplateId::GridTwo => {
            // Two equal side-by-side panels.
            let half_w = w / 2.0;
            let ids = [(&page.image_id, 0.0), (&page.image_id_2, half_w)];
            for (id_opt, x) in &ids {
                if let Some(raw) = id_opt.as_ref().and_then(|id| images.get(id.as_str())) {
                    place_image_cover(doc, &mut ops, raw, *x, 0.0, half_w, h);
                } else {
                    push_rect_fill(&mut ops, *x, 0.0, half_w, h, 0.15);
                }
            }
            // 1pt white divider
            push_rect_fill(&mut ops, half_w - 0.5, 0.0, 1.0, h, 1.0);
        }

        // ── Phase 1 new templates ─────────────────────────────────────────────
        TemplateId::PullQuote => {
            // Full-bleed image (cover fit) + dark translucent panel + large italic title.
            if let Some(raw) = raw {
                place_image_cover(doc, &mut ops, raw, 0.0, 0.0, w, h);
            }

            let usable_w = w - 2.0 * MARGIN_MM;
            let max_chars = (usable_w / 2.2) as usize;

            if let Some(cap) = page.image_id.as_ref().and_then(|id| captions.get(id)) {
                if !cap.title.is_empty() || !cap.description.is_empty() {
                    let title_n = if cap.title.is_empty() {
                        0
                    } else {
                        wrap_text(&cap.title, max_chars).len().min(2)
                    };
                    let desc_n = if cap.description.is_empty() {
                        0
                    } else {
                        wrap_text(&cap.description, max_chars).len().min(2)
                    };
                    let sep = if title_n > 0 && desc_n > 0 { 2.0 } else { 0.0 };
                    let panel_h =
                        (title_n as f64 * 5.2) + (desc_n as f64 * 3.4) + sep + 2.0 * MARGIN_MM;

                    // Dark translucent-looking panel (10% grey = dark charcoal)
                    push_rect_fill(&mut ops, 0.0, 0.0, w, panel_h, 0.12);

                    let mut y = panel_h - MARGIN_MM;

                    // Title as large italic quote (white text on dark panel)
                    if title_n > 0 {
                        ops.push(Op::StartTextSection);
                        ops.push(Op::SetFont {
                            font: PdfFontHandle::Builtin(BuiltinFont::HelveticaBoldOblique),
                            size: Pt(13.0),
                        });
                        ops.push(Op::SetFillColor {
                            col: Color::Greyscale(Greyscale {
                                percent: 1.0,
                                icc_profile: None,
                            }),
                        });
                        for (i, line) in wrap_text(&cap.title, max_chars).iter().take(2).enumerate()
                        {
                            let ly = y - i as f64 * 5.2;
                            ops.push(Op::SetTextMatrix {
                                matrix: TextMatrix::Translate(
                                    Pt(mm_to_pt(MARGIN_MM) as f32),
                                    Pt(mm_to_pt(ly) as f32),
                                ),
                            });
                            ops.push(Op::ShowText {
                                items: vec![TextItem::Text(line.clone())],
                            });
                        }
                        ops.push(Op::EndTextSection);
                        y -= title_n as f64 * 5.2 + sep;
                    }

                    // Description in lighter white
                    if desc_n > 0 && y > MARGIN_MM {
                        ops.push(Op::StartTextSection);
                        ops.push(Op::SetFont {
                            font: PdfFontHandle::Builtin(BuiltinFont::Helvetica),
                            size: Pt(7.5),
                        });
                        ops.push(Op::SetFillColor {
                            col: Color::Greyscale(Greyscale {
                                percent: 0.82,
                                icc_profile: None,
                            }),
                        });
                        for (i, line) in wrap_text(&cap.description, max_chars)
                            .iter()
                            .take(2)
                            .enumerate()
                        {
                            let ly = y - i as f64 * 3.4;
                            ops.push(Op::SetTextMatrix {
                                matrix: TextMatrix::Translate(
                                    Pt(mm_to_pt(MARGIN_MM) as f32),
                                    Pt(mm_to_pt(ly) as f32),
                                ),
                            });
                            ops.push(Op::ShowText {
                                items: vec![TextItem::Text(line.clone())],
                            });
                        }
                        ops.push(Op::EndTextSection);
                    }
                }
            } else if let Some(text) = &caption_text {
                let line_n = wrap_text(text, max_chars).len().min(3);
                let panel_h = (line_n as f64 * 4.8) + 2.0 * MARGIN_MM;
                push_rect_fill(&mut ops, 0.0, 0.0, w, panel_h, 0.12);
                ops.push(Op::StartTextSection);
                ops.push(Op::SetFont {
                    font: PdfFontHandle::Builtin(BuiltinFont::HelveticaBoldOblique),
                    size: Pt(11.0),
                });
                ops.push(Op::SetFillColor {
                    col: Color::Greyscale(Greyscale {
                        percent: 1.0,
                        icc_profile: None,
                    }),
                });
                for (i, line) in wrap_text(text, max_chars).iter().take(3).enumerate() {
                    let y = panel_h - MARGIN_MM - i as f64 * 4.8;
                    ops.push(Op::SetTextMatrix {
                        matrix: TextMatrix::Translate(
                            Pt(mm_to_pt(MARGIN_MM) as f32),
                            Pt(mm_to_pt(y) as f32),
                        ),
                    });
                    ops.push(Op::ShowText {
                        items: vec![TextItem::Text(line.clone())],
                    });
                }
                ops.push(Op::EndTextSection);
            }
        }

        TemplateId::ImageLead => {
            // Image top 60 %; white panel bottom 40 % with large headline.
            let img_h = h * 0.60;
            let panel_h = h - img_h;

            push_rect_fill(&mut ops, 0.0, 0.0, w, panel_h, 1.0);

            if let Some(raw) = raw {
                place_image_cover(doc, &mut ops, raw, 0.0, panel_h, w, img_h);
            }

            let usable_w = w - 2.0 * MARGIN_MM;
            let max_chars = (usable_w / 1.9) as usize;
            let mut y = panel_h - MARGIN_MM;

            if let Some(cap) = page.image_id.as_ref().and_then(|id| captions.get(id)) {
                if !cap.title.is_empty() {
                    let lines = push_text_block(
                        &mut ops,
                        &cap.title,
                        MARGIN_MM,
                        y,
                        PdfFontHandle::Builtin(BuiltinFont::HelveticaBold),
                        13.0,
                        5.5,
                        max_chars,
                        2,
                    );
                    y -= lines as f64 * 5.5 + 2.0;
                }
                if !cap.description.is_empty() && y > MARGIN_MM {
                    push_text_block(
                        &mut ops,
                        &cap.description,
                        MARGIN_MM,
                        y,
                        PdfFontHandle::Builtin(BuiltinFont::Helvetica),
                        7.5,
                        3.2,
                        max_chars,
                        4,
                    );
                }
            } else if let Some(text) = &caption_text {
                push_text_block(
                    &mut ops,
                    text,
                    MARGIN_MM,
                    y,
                    PdfFontHandle::Builtin(BuiltinFont::HelveticaBold),
                    12.0,
                    5.0,
                    max_chars,
                    3,
                );
            }
        }

        TemplateId::Panoramic => {
            // White page; image letterbox-fitted across full width; caption text below.
            push_rect_fill(&mut ops, 0.0, 0.0, w, h, 1.0);

            let cap_area_h = 22.0_f64; // mm reserved at bottom for caption
            let img_area_y = cap_area_h; // image sits above caption area
            let img_area_h = h - cap_area_h - MARGIN_MM; // top margin too

            let usable_w = w - 2.0 * MARGIN_MM;
            let max_chars = (usable_w / 1.9) as usize;

            if let Some(raw) = raw {
                // Letterbox fit (intentional — shows the full panoramic frame)
                let id = doc.add_image(raw);
                ops.push(Op::SaveGraphicsState);
                ops.push(Op::UseXobject {
                    id,
                    transform: letterbox_transform(raw, 0.0, img_area_y, w, img_area_h),
                });
                ops.push(Op::RestoreGraphicsState);
            }

            // Caption in the reserved bottom area
            let mut y = cap_area_h - MARGIN_MM;

            if let Some(cap) = page.image_id.as_ref().and_then(|id| captions.get(id)) {
                if !cap.title.is_empty() {
                    let lines = push_text_block(
                        &mut ops,
                        &cap.title,
                        MARGIN_MM,
                        y,
                        PdfFontHandle::Builtin(BuiltinFont::HelveticaBold),
                        8.5,
                        3.6,
                        max_chars,
                        1,
                    );
                    y -= lines as f64 * 3.6 + 1.0;
                }
                if !cap.description.is_empty() && y > MARGIN_MM {
                    push_text_block(
                        &mut ops,
                        &cap.description,
                        MARGIN_MM,
                        y,
                        PdfFontHandle::Builtin(BuiltinFont::Helvetica),
                        7.0,
                        3.0,
                        max_chars,
                        2,
                    );
                }
            } else if let Some(text) = &caption_text {
                push_text_block(
                    &mut ops,
                    text,
                    MARGIN_MM,
                    y,
                    PdfFontHandle::Builtin(BuiltinFont::Helvetica),
                    7.5,
                    3.2,
                    max_chars,
                    2,
                );
            }
        }

        TemplateId::Editorial => {
            // Image fills left 65%, editorial text column right 35%.
            let img_w = w * 0.65;
            let text_x = img_w + MARGIN_MM;
            let text_w = w - img_w - 2.0 * MARGIN_MM;
            let max_chars = (text_w / 1.8) as usize;

            push_rect_fill(&mut ops, 0.0, 0.0, w, h, 1.0);
            if let Some(raw) = page.image_id.as_ref().and_then(|id| images.get(id)) {
                place_image_cover(doc, &mut ops, raw, 0.0, 0.0, img_w, h);
            }
            if let Some(cap) = page.image_id.as_ref().and_then(|id| captions.get(id)) {
                push_text_block(
                    &mut ops,
                    &cap.title,
                    text_x,
                    h * 0.72,
                    PdfFontHandle::Builtin(BuiltinFont::HelveticaBold),
                    11.0,
                    5.5,
                    max_chars,
                    3,
                );
                push_text_block(
                    &mut ops,
                    &cap.description,
                    text_x,
                    h * 0.72 - 22.0,
                    PdfFontHandle::Builtin(BuiltinFont::Helvetica),
                    7.5,
                    3.8,
                    max_chars,
                    8,
                );
                if !cap.location.is_empty() {
                    push_text_block(
                        &mut ops,
                        &cap.location,
                        text_x,
                        MARGIN_MM * 2.0,
                        PdfFontHandle::Builtin(BuiltinFont::HelveticaOblique),
                        7.0,
                        3.5,
                        max_chars,
                        2,
                    );
                }
            }
        }

        TemplateId::Triptych => {
            // Three equal vertical panels.
            let panel_w = w / 3.0;
            let ids = [&page.image_id, &page.image_id_2, &page.image_id_3];
            for (i, id_opt) in ids.iter().enumerate() {
                let x = i as f64 * panel_w;
                if let Some(raw) = id_opt.as_ref().and_then(|id| images.get(id.as_str())) {
                    place_image_cover(doc, &mut ops, raw, x, 0.0, panel_w, h);
                } else {
                    push_rect_fill(&mut ops, x, 0.0, panel_w, h, 0.15);
                }
                // 1px white divider (except after last panel)
                if i < 2 {
                    push_rect_fill(&mut ops, x + panel_w - 0.5, 0.0, 1.0, h, 1.0);
                }
            }
        }

        TemplateId::Grid4x => {
            // 2×2 grid of 4 images.
            let half_w = w / 2.0;
            let half_h = h / 2.0;
            let ids = [
                (&page.image_id, 0.0, half_h),
                (&page.image_id_2, half_w, half_h),
                (&page.image_id_3, 0.0, 0.0),
                (&page.image_id_4, half_w, 0.0),
            ];
            for (id_opt, x, y) in &ids {
                if let Some(raw) = id_opt.as_ref().and_then(|id| images.get(id.as_str())) {
                    place_image_cover(doc, &mut ops, raw, *x, *y, half_w, half_h);
                } else {
                    push_rect_fill(&mut ops, *x, *y, half_w, half_h, 0.15);
                }
            }
            // 1pt white gutters
            push_rect_fill(&mut ops, half_w - 0.5, 0.0, 1.0, h, 1.0);
            push_rect_fill(&mut ops, 0.0, half_h - 0.5, w, 1.0, 1.0);
        }

        TemplateId::FeatureSpread => {
            // Hero image top 70%, two smaller images below side by side.
            let hero_h = h * 0.70;
            let bottom_h = h - hero_h;
            let half_w = w / 2.0;

            if let Some(raw) = page.image_id.as_ref().and_then(|id| images.get(id)) {
                place_image_cover(doc, &mut ops, raw, 0.0, bottom_h, w, hero_h);
            } else {
                push_rect_fill(&mut ops, 0.0, bottom_h, w, hero_h, 0.15);
            }
            for (id_opt, x) in [(&page.image_id_2, 0.0), (&page.image_id_3, half_w)] {
                if let Some(raw) = id_opt.as_ref().and_then(|id| images.get(id.as_str())) {
                    place_image_cover(doc, &mut ops, raw, x, 0.0, half_w, bottom_h);
                } else {
                    push_rect_fill(&mut ops, x, 0.0, half_w, bottom_h, 0.12);
                }
            }
            // 1pt white gutters
            push_rect_fill(&mut ops, 0.0, bottom_h - 0.5, w, 1.0, 1.0);
            push_rect_fill(&mut ops, half_w - 0.5, 0.0, 1.0, bottom_h, 1.0);

            // Caption overlay on hero
            if let Some(cap) = page.image_id.as_ref().and_then(|id| captions.get(id)) {
                let max_chars = ((w - 2.0 * MARGIN_MM) / 2.2) as usize;
                push_text_block(
                    &mut ops,
                    &cap.title,
                    MARGIN_MM,
                    bottom_h + MARGIN_MM,
                    PdfFontHandle::Builtin(BuiltinFont::HelveticaBold),
                    13.0,
                    6.0,
                    max_chars,
                    2,
                );
            }
        }

        TemplateId::PhotoEssay => {
            // Large image left 50%, 2×2 grid of 4 smaller images on the right.
            let half_w = w / 2.0;
            let half_h = h / 2.0;

            if let Some(raw) = page.image_id.as_ref().and_then(|id| images.get(id)) {
                place_image_cover(doc, &mut ops, raw, 0.0, 0.0, half_w, h);
            } else {
                push_rect_fill(&mut ops, 0.0, 0.0, half_w, h, 0.15);
            }

            let small_ids = [
                &page.image_id_2,
                &page.image_id_3,
                &page.image_id_4,
                &page.image_id_5,
            ];
            for (i, id_opt) in small_ids.iter().enumerate() {
                let col = i % 2;
                let row = i / 2;
                let x = half_w + col as f64 * half_w / 2.0;
                let y = row as f64 * half_h;
                if let Some(raw) = id_opt.as_ref().and_then(|id| images.get(id.as_str())) {
                    place_image_cover(doc, &mut ops, raw, x, y, half_w / 2.0, half_h);
                } else {
                    push_rect_fill(&mut ops, x, y, half_w / 2.0, half_h, 0.12);
                }
            }
            // Gutters
            push_rect_fill(&mut ops, half_w - 0.5, 0.0, 1.0, h, 1.0);
            push_rect_fill(&mut ops, half_w, half_h - 0.5, half_w, 1.0, 1.0);
            push_rect_fill(&mut ops, half_w + half_w / 2.0 - 0.5, 0.0, 1.0, h, 1.0);

            // Caption on left image
            if let Some(cap) = page.image_id.as_ref().and_then(|id| captions.get(id)) {
                let max_chars = ((half_w - 2.0 * MARGIN_MM) / 2.0) as usize;
                push_text_block(
                    &mut ops,
                    &cap.title,
                    MARGIN_MM,
                    MARGIN_MM * 2.0,
                    PdfFontHandle::Builtin(BuiltinFont::HelveticaBold),
                    11.0,
                    5.5,
                    max_chars,
                    2,
                );
            }
        }

        TemplateId::ContactSheet => {
            // Classic 3×3 film contact sheet — 9 equal cells with thin white gutters.
            push_rect_fill(&mut ops, 0.0, 0.0, w, h, 0.95);
            let cols = 3_usize;
            let rows = 3_usize;
            let gutter = 1.5_f64;
            let cell_w = (w - gutter * (cols as f64 + 1.0)) / cols as f64;
            let cell_h = (h - gutter * (rows as f64 + 1.0)) / rows as f64;

            let ids: [&Option<String>; 9] = [
                &page.image_id,
                &page.image_id_2,
                &page.image_id_3,
                &page.image_id_4,
                &page.image_id_5,
                &page.image_id_6,
                &page.image_id_7,
                &page.image_id_8,
                &page.image_id_9,
            ];

            for (i, id_opt) in ids.iter().enumerate() {
                let col = i % cols;
                let row = rows - 1 - i / cols; // top row = highest y in PDF coords
                let x = gutter + col as f64 * (cell_w + gutter);
                let y = gutter + row as f64 * (cell_h + gutter);

                if let Some(raw) = id_opt.as_ref().and_then(|id| images.get(id.as_str())) {
                    place_image_cover(doc, &mut ops, raw, x, y, cell_w, cell_h);
                } else {
                    push_rect_fill(&mut ops, x, y, cell_w, cell_h, 0.80);
                }
            }
        }

        TemplateId::TitlePage => {
            push_rect_fill(&mut ops, 0.0, 0.0, w, h, 1.0);

            let cx = MARGIN_MM * 2.0;
            let mid_y = h / 2.0;
            let max_chars = ((w - 4.0 * MARGIN_MM) / 2.0) as usize;

            if let Some(cap) = page.image_id.as_ref().and_then(|id| captions.get(id)) {
                push_text_block(
                    &mut ops,
                    &cap.title,
                    cx,
                    mid_y + 10.0,
                    PdfFontHandle::Builtin(BuiltinFont::HelveticaBold),
                    18.0,
                    8.0,
                    max_chars,
                    3,
                );
                push_text_block(
                    &mut ops,
                    &cap.description,
                    cx,
                    mid_y - 5.0,
                    PdfFontHandle::Builtin(BuiltinFont::Helvetica),
                    10.0,
                    4.5,
                    max_chars,
                    4,
                );
            } else if let Some(text) = &caption_text {
                push_text_block(
                    &mut ops,
                    text,
                    cx,
                    mid_y,
                    PdfFontHandle::Builtin(BuiltinFont::HelveticaBold),
                    16.0,
                    7.0,
                    max_chars,
                    4,
                );
            }
        }
    }

    PdfPage::new(Mm(w as f32), Mm(h as f32), ops)
}

/// Render the full zine to PDF bytes.
pub fn render_pdf(
    zine: &Zine,
    images: &HashMap<String, RawImage>,
    captions: &HashMap<String, Caption>,
) -> Result<Vec<u8>> {
    let mut warnings = Vec::new();
    let mut doc = PdfDocument::new("ShotList Zine");
    let mut pages: Vec<PdfPage> = Vec::new();

    for page in &zine.pages {
        pages.push(render_page(
            &mut doc,
            page,
            &zine.paper_size,
            images,
            captions,
        ));
    }

    let bytes = doc
        .with_pages(pages)
        .save(&PdfSaveOptions::default(), &mut warnings);

    for w in &warnings {
        tracing::warn!("PDF render warning: {:?}", w);
    }

    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tiny synthetic JPEG, decoded the same way `commands.rs` decodes real
    /// developed images before handing them to printpdf.
    fn test_raw_image() -> RawImage {
        let img = image::RgbImage::from_pixel(4, 4, image::Rgb([128, 128, 128]));
        let mut jpeg_bytes: Vec<u8> = Vec::new();
        use image::ImageEncoder as _;
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg_bytes, 90)
            .write_image(img.as_raw(), 4, 4, image::ExtendedColorType::Rgb8)
            .unwrap();
        let mut warnings = Vec::new();
        RawImage::decode_from_bytes(&jpeg_bytes, &mut warnings).unwrap()
    }

    /// Build a page for `template` with `slot_count` distinct image ids filled
    /// in (image_id, image_id_2, ... in order) and the rest left empty.
    fn test_page(template: TemplateId, slot_count: usize) -> ZinePage {
        let slot = |i: usize| (i < slot_count).then(|| format!("img-{i}"));
        ZinePage {
            id: "page-1".to_string(),
            zine_id: "zine-1".to_string(),
            position: 0,
            image_id: slot(0),
            image_id_2: slot(1),
            image_id_3: slot(2),
            image_id_4: slot(3),
            image_id_5: slot(4),
            image_id_6: slot(5),
            image_id_7: slot(6),
            image_id_8: slot(7),
            image_id_9: slot(8),
            template,
            caption_override: None,
        }
    }

    /// Render `page` against `slot_count` synthetic images and assert exactly
    /// that many `Op::UseXobject` placements landed in the page — i.e. every
    /// filled image slot actually got drawn, not silently dropped.
    fn assert_renders_n_images(template: TemplateId, slot_count: usize) {
        let page = test_page(template.clone(), slot_count);
        let mut images: HashMap<String, RawImage> = HashMap::new();
        for i in 0..slot_count {
            images.insert(format!("img-{i}"), test_raw_image());
        }
        let captions: HashMap<String, Caption> = HashMap::new();

        let mut doc = PdfDocument::new("test");
        let rendered = render_page(&mut doc, &page, &PaperSize::A5, &images, &captions);

        assert_eq!(
            rendered.get_xobject_ids().len(),
            slot_count,
            "{template:?}: expected {slot_count} image placement(s)"
        );
    }

    #[test]
    fn full_bleed_renders_one_image() {
        assert_renders_n_images(TemplateId::FullBleed, 1);
    }

    #[test]
    fn top75_renders_one_image() {
        assert_renders_n_images(TemplateId::Top75, 1);
    }

    #[test]
    fn left50_renders_one_image() {
        assert_renders_n_images(TemplateId::Left50, 1);
    }

    #[test]
    fn right50_renders_one_image() {
        assert_renders_n_images(TemplateId::Right50, 1);
    }

    #[test]
    fn grid_two_renders_both_images() {
        // Regression test for the bug where GridTwo only ever placed the
        // primary image and silently dropped image_id_2.
        assert_renders_n_images(TemplateId::GridTwo, 2);
    }

    #[test]
    fn title_page_renders_no_images() {
        assert_renders_n_images(TemplateId::TitlePage, 0);
    }

    #[test]
    fn pull_quote_renders_one_image() {
        assert_renders_n_images(TemplateId::PullQuote, 1);
    }

    #[test]
    fn image_lead_renders_one_image() {
        assert_renders_n_images(TemplateId::ImageLead, 1);
    }

    #[test]
    fn panoramic_renders_one_image() {
        assert_renders_n_images(TemplateId::Panoramic, 1);
    }

    #[test]
    fn editorial_renders_one_image() {
        assert_renders_n_images(TemplateId::Editorial, 1);
    }

    #[test]
    fn triptych_renders_three_images() {
        assert_renders_n_images(TemplateId::Triptych, 3);
    }

    #[test]
    fn grid4x_renders_four_images() {
        assert_renders_n_images(TemplateId::Grid4x, 4);
    }

    #[test]
    fn feature_spread_renders_three_images() {
        assert_renders_n_images(TemplateId::FeatureSpread, 3);
    }

    #[test]
    fn photo_essay_renders_five_images() {
        assert_renders_n_images(TemplateId::PhotoEssay, 5);
    }

    #[test]
    fn contact_sheet_renders_nine_images() {
        assert_renders_n_images(TemplateId::ContactSheet, 9);
    }

    #[test]
    fn grid_two_with_only_left_slot_renders_one_image() {
        // Empty slots fall back to a placeholder rect, not a missing/blank draw.
        assert_renders_n_images(TemplateId::GridTwo, 1);
    }

    #[test]
    fn crop_and_rotate_persist_through_zine_render() {
        // Reproduces the exact pipeline commands.rs uses to embed a developed
        // image into a zine PDF: apply_develop -> JPEG encode -> RawImage -> render_page.
        // Proves saved geometry (rotation/crop) survives all the way to the
        // exported page, not just to apply_develop's own output.
        use crate::core::develop::{apply_develop, CropRect, DevelopParams};

        let src = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
            200,
            100,
            image::Rgb([100, 150, 200]),
        ));

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

        // Rotate 90° (200x100 -> 100x200), then crop the left half of the
        // now-rotated width (100x200 -> 50x200).
        let processed = apply_develop(&src, &params);
        assert_eq!((processed.width(), processed.height()), (50, 200));

        let rgb = processed.to_rgb8();
        let mut jpeg_bytes: Vec<u8> = Vec::new();
        use image::ImageEncoder as _;
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg_bytes, 90)
            .write_image(
                rgb.as_raw(),
                rgb.width(),
                rgb.height(),
                image::ExtendedColorType::Rgb8,
            )
            .unwrap();
        let mut warnings = Vec::new();
        let raw = RawImage::decode_from_bytes(&jpeg_bytes, &mut warnings).unwrap();

        // Geometry survived the JPEG round-trip commands.rs relies on.
        assert_eq!((raw.width, raw.height), (50, 200));

        // And the page render actually embeds *this* geometry-adjusted
        // image — not silently falling back to some other source.
        // (test_page names its first slot "img-0".)
        let mut images = HashMap::new();
        images.insert("img-0".to_string(), raw);
        let page = test_page(TemplateId::FullBleed, 1);
        let mut doc = PdfDocument::new("test");
        let rendered = render_page(&mut doc, &page, &PaperSize::A5, &images, &HashMap::new());
        assert_eq!(rendered.get_xobject_ids().len(), 1);
    }
}
