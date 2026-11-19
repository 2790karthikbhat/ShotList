# ShotList — Product Plan

> _"From SD card to finished zine — with an AI that talks to you the whole way."_

---

## Vision

ShotList is an end-to-end creative pipeline for street and documentary photographers.
You dump your RAW photos into a folder and walk out with a print-ready zine PDF —
with an AI acting as your darkroom collaborator throughout, explaining every decision
it makes in plain language so you understand your craft better with every session.

**The full arc:**

```
Import → AI-assisted cull → technical analysis → develop (mini Lightroom + AI explanation) → AI captions → zine layout → PDF
```

**The philosophy — AI as collaborator, not black box:**
Every time the AI makes a decision — picking a frame, adjusting exposure, writing a caption,
suggesting a page sequence — it explains why, in plain English, in the voice of an experienced
darkroom assistant. You stay in control. The tool educates as it assists.

---

## Target User (v1)

**Street and documentary photographers making personal zines.**

Key characteristics that shape every design decision:

- Shoots high-ISO, low-light, fast-paced — can't reshoot the decisive moment
- Values authenticity over polish — grain is texture, not noise
- Works in B&W as often as colour
- Shoots in bursts at key moments (not continuous spray-and-pray)
- Wants to tell a story, not just organise files
- Makes personal zines: small-run, self-published, gritty, typographic

Plans to expand to other photographer types in later versions.

---

## Why This Is Different

| Tool             | Culling | Editing | AI Collaborator          | Zine Builder |
| ---------------- | ------- | ------- | ------------------------ | ------------ |
| Lightroom        | ✅      | ✅      | ❌                       | ❌           |
| Narrative Select | ✅      | ❌      | ❌                       | ❌           |
| Luminar          | ❌      | ✅      | Partial (no explanation) | ❌           |
| Canva            | ❌      | ❌      | ❌                       | ✅ (generic) |
| Darkroom (iOS)   | ❌      | ✅      | ❌                       | ❌           |
| **ShotList**     | **✅**  | **✅**  | **✅**                   | **✅**       |

No tool currently does this full arc. This is the gap.

---

## Framework Decision

### Tauri 2.0

- **Frontend:** React + TypeScript + Tailwind CSS
- **Component library:** shadcn/ui (Radix primitives, fully unstyled/customizable)
- **Icons:** Lucide React
- **Drag-and-drop:** @dnd-kit (filmstrip reorder, zine page sequencing)
- **Backend:** Rust — all existing `core/`, `ai/`, `db/` code unchanged
- **PDF export:** Rust (`printpdf` crate)
- **Image processing:** Rust (existing pipeline + new develop engine)

### Why Tauri over alternatives

|                    | egui (current)          | Slint        | Tauri                      |
| ------------------ | ----------------------- | ------------ | -------------------------- |
| Design ceiling     | Low — always looks egui | High         | Unlimited                  |
| Commercial license | MIT — free              | GPL or paid  | MIT — free                 |
| Histogram UI       | Hand-painted            | Limited      | Chart.js / D3              |
| Tone curve editor  | Very painful            | Limited      | Canvas — natural           |
| Zine drag-and-drop | Very painful            | Very painful | @dnd-kit — trivial         |
| AI streaming text  | Manual repaint          | Limited      | EventSource — trivial      |
| Image performance  | In-process              | In-process   | WKWebView + convertFileSrc |

**egui ceiling:** Can't achieve the target aesthetic without years of custom painting work.  
**Slint:** GPL means must open-source, or pay for a commercial license — adds recurring cost.  
**Tauri:** MIT, can sell as proprietary closed-source app, no royalties, no licensing friction.

### What stays in Rust (untouched)

```
src/core/    — image model, session, import, export, thumbnailer, burst grouping, watcher
src/ai/      — Claude Vision, OpenAI, local ONNX, autocull, face detection, scoring
src/db/      — SQLite migrations, queries
```

### What's new in Rust

```
src/develop/ — DevelopParams, parametric apply pipeline, Auto algorithm, JPEG export
src/analyse/ — histogram compute, zone distribution, sharpness grid, colour temperature
src/zine/    — Zine/ZinePage structs, PDF generation via printpdf
```

### What replaces src/ui/

```
frontend/    — React app (Tauri frontend)
  components/
    stages/      — Ingest, Cull, Analyse, Develop, Caption, Zine views
    filmstrip/   — Thumbnail strip (shared across stages)
    viewer/      — Main image viewer
    analyser/    — Histogram, zones, sharpness heatmap panels
    develop/     — Sliders, tone curve, AI explanation panel
    caption/     — Caption editor
    zine/        — Page sequencer, template picker, PDF export
  lib/
    ipc.ts       — All Tauri invoke() calls, typed
    store.ts     — Zustand global state
```

---

## The 6 Stages

### Stage 1 — Ingest ✅ (mostly done)

Scan folder, extract RAW previews, read EXIF, build thumbnails, SQLite persistence,
face detection, burst grouping. Resume decisions on re-import.

**Remaining work:**

- [ ] Full-quality RAW → JPEG demosaicing (for develop export, beyond embedded preview)
- [ ] Drag-and-drop folder import zone in Tauri UI (replaces rfd picker, keep as fallback)
- [ ] Multi-folder / multi-session support (later)

**Supported RAW formats:** CR2/CR3, NEF, ARW, RAF, RW2, ORF, DNG (via `rawler`)

---

### Stage 2 — Culling ✅ (mostly done, wire AI)

Keyboard-first pick/reject/flag, filmstrip navigation, burst compare, auto-cull, persistent decisions.

**Remaining work:**

- [ ] Wire Claude Vision for aesthetic/compositional scoring (IPC scaffolded, not wired to UI)
- [ ] Scene grouping: cluster images by subject/location, pick best per scene
- [ ] Duplicate detection: pHash clustering (pHash already computed per image)
- [ ] Redesigned filmstrip in React (same data, new UI — see Design section)

**AI collaborator moment in Cull:**

> _"Frame 3 of 7: sharpest in the burst (focus score 84%, eye in focus). Frame 5: slightly softer (71%) but both subjects visible and better street context in the background. Recommending frame 5 as pick, frame 3 as flag."_

**Keyboard shortcuts:** unchanged — P/X/U/1-5/arrows/C stay identical.

---

### Stage 3 — Technical Analyser 🆕

Per-image technical breakdown in a collapsible right sidebar.
All computed in Rust on full-res decode, cached in SQLite.

**Analyser components:**

| Component          | Description                                    | Rendered by              |
| ------------------ | ---------------------------------------------- | ------------------------ |
| Histogram          | RGB channels + luminosity, 256 bins each       | Chart.js canvas          |
| Exposure Zones     | Ansel Adams zone 0–X, tone distribution        | React custom component   |
| Sharpness Heatmap  | 16×16 grid Laplacian per cell, heatmap overlay | Canvas overlay on viewer |
| Colour Temperature | Estimated K value + warm/cool indicator        | React                    |
| Clipping stats     | Highlight clip %, shadow crush %               | React                    |

**Rust IPC data structure:**

```rust
#[derive(Serialize)]
pub struct ImageAnalysis {
    pub histogram: HistogramData,       // [u32; 256] × 4 channels (R,G,B,L)
    pub zone_distribution: [f32; 11],   // Ansel Adams zones 0–X, 0.0–1.0 each
    pub sharpness_grid: Vec<f32>,       // 16×16 = 256 values, 0.0–1.0 each
    pub colour_temp_k: Option<u32>,     // e.g. 4200
    pub highlight_clip_pct: f32,        // % pixels > 250
    pub shadow_crush_pct: f32,          // % pixels < 5
}
```

**Ansel Adams Zone System reference:**

```
Zone 0  — Pure black, no detail
Zone I  — Near black, slight texture
Zone II — Darkest tone with texture (dark shadows in street scenes)
Zone V  — Middle grey (18% grey card, skin in open shade)
Zone VIII — Lightest tone with detail (bright sky, white fabric)
Zone X  — Pure white, no detail
```

The zone display teaches photographers to read light. This is unique among photo tools.

---

### Stage 4 — Develop 🆕 (the heart of the product)

Non-destructive parametric editing with an AI collaborator that explains every decision.
Decisions stored as JSON in SQLite — original RAW/JPEG is never modified.

**Edit parameters:**

```rust
#[derive(Serialize, Deserialize, Default)]
pub struct DevelopParams {
    // Light
    pub exposure: f32,           // -5.0 to +5.0 EV
    pub highlights: f32,         // -100 to +100
    pub shadows: f32,            // -100 to +100
    pub whites: f32,             // -100 to +100
    pub blacks: f32,             // -100 to +100
    pub contrast: f32,           // -100 to +100
    pub clarity: f32,            // -100 to +100 (local/midtone contrast)

    // Colour
    pub temp: f32,               // 2000–12000K delta from shot WB
    pub tint: f32,               // -150 to +150 (green–magenta)
    pub vibrance: f32,           // -100 to +100
    pub saturation: f32,         // -100 to +100

    // Detail
    pub sharpening: f32,         // 0 to 150
    pub noise_reduction: f32,    // 0 to 100

    // Effects (critical for street/doc aesthetic)
    pub grain_amount: f32,       // 0 to 100
    pub grain_size: f32,         // 0 to 100
    pub vignette: f32,           // -100 to +100

    // B&W
    pub bw_enabled: bool,
    pub bw_channel_mixer: BwChannelMixer,  // per-colour-range luminance weights

    // Tone curve (overrides contrast if set)
    pub curve_points: Vec<(f32, f32)>,     // input/output pairs, 0.0–1.0
}
```

#### The Auto Develop Algorithm (Rust, no AI required)

```
1. Compute histogram on full-res image
2. Highlight recovery:
   pct_clipped = pixels > 250 / total_pixels
   highlights = -(pct_clipped * 100).clamp(0, 90)
3. Shadow lift:
   pct_crushed = pixels < 5 / total_pixels
   shadows = (pct_crushed * 80).clamp(0, 60)  ← capped at 60, street mood preserved
4. Exposure:
   shift midtone centre (median luminosity) toward zone V (128/255)
   exposure_ev = log2(128.0 / current_median)
5. Whites/Blacks:
   whites = headroom before clipping starts × 0.8
   blacks = pull blacks down to set a true black point
6. Contrast:
   derived from histogram spread (std dev of luminosity)
7. Street/documentary bias applied throughout:
   - noise_reduction = 0 by default (grain is authentic)
   - shadows capped at +60 (preserve dramatic shadow areas)
   - saturation unchanged (don't pump colours)
   - grain_amount mirrors original ISO: ISO3200 → ~40 grain by default
```

#### The AI Explanation (Claude — streams live into the panel)

Rust computes params → assembles a structured prompt → Claude streams back the explanation:

**System prompt:**

```
You are a darkroom assistant for a street and documentary photographer.
For each adjustment, write exactly one sentence explaining what it does
to this specific image and why it matters for the documentary aesthetic.
Speak in first person ("I pulled...", "I lifted...").
Never over-explain. Preserve the grit.
```

**User prompt (assembled by Rust):**

```
Image metadata: [ISO, shutter, aperture, focal length]
Technical analysis: [histogram summary, clip %, crush %, colour temp]
Proposed adjustments: [key: value list of non-zero params]

Explain each adjustment for this street photograph.
```

**Example output (streams into the panel):**

> _I pulled the highlights down to -55 to recover the blown street sign in your upper-right — that detail was there in the RAW, just clipped. I lifted shadows +28 to let you read the face on the left without losing the darkness that makes the alley feel real; I stopped before it got muddy. The +0.4 exposure brings your midtones up from zone IV where the meter landed to a more readable zone V. I left the grain alone — at ISO 3200 it's doing work._

This is the differentiating feature. No other tool does this.

#### JPEG Export from Develop

- Rust applies all `DevelopParams` to full-res RAW decode (or source JPEG)
- Pipeline: decode → exposure → tone curve → HSL → clarity → B&W → grain → vignette → sharpen → NR → encode JPEG
- EXIF + IPTC metadata preserved and extended (caption, develop note)
- Written to a `Developed/` subfolder alongside source images
- Original never touched

---

### Stage 5 — Caption 🆕

Claude Vision looks at each developed image → drafts a documentary caption → user edits and approves.

**Claude system prompt (Magnum style):**

```
You are writing captions for a street and documentary photography project.
Style: Magnum Photos caption style.
Rules:
  - Present tense
  - Factual, not interpretive
  - Maximum two sentences
  - No adjectives like "striking", "powerful", "beautiful"
  - Describe what is literally happening in the frame
  - Include location and year if available in EXIF
```

**UI flow:**

1. Developed image displayed (or preview if not yet developed)
2. AI draft appears in a large text editor below
3. User edits inline
4. Tab → advances to next uncaptioned image
5. "Regenerate" button → resends with optional style note (e.g. "more sparse", "include the weather")
6. Approve → stored to SQLite, embedded into JPEG IPTC `Caption-Abstract` on next export

**Keyboard:** Tab to advance, Shift-Tab to go back, Cmd-Enter to approve and advance.

---

### Stage 6 — Zine Builder 🆕

Sequence picked+developed images into a page layout, assign templates, export PDF.

#### Page Templates (v1)

| ID           | Name          | Layout                                 | Best for                 |
| ------------ | ------------- | -------------------------------------- | ------------------------ |
| `full_bleed` | Full Bleed    | Image fills page, edge to edge         | Strong single frames     |
| `centered`   | Centered      | Image centred, margin, caption below   | Portraits, quiet moments |
| `diptych`    | Diptych       | Two images side by side                | Contrast / tension       |
| `sequence`   | Sequence      | 3–4 small frames in a horizontal strip | Motion, time             |
| `spread`     | Double Spread | One image across both pages            | Wide establishing shots  |
| `text_photo` | Text + Photo  | Caption block left, image right        | Context/intro pages      |
| `contact`    | Contact Sheet | Grid of 9–12 small frames              | Documentary overflow     |

#### Zine Data Structures

```rust
pub struct Zine {
    pub id:          Uuid,
    pub title:       String,
    pub subtitle:    Option<String>,
    pub paper_size:  PaperSize,
    pub created_at:  DateTime<Utc>,
    pub pages:       Vec<ZinePage>,
}

pub struct ZinePage {
    pub order:            u16,
    pub template:         TemplateId,
    pub image_ids:        Vec<ImageId>,       // 1 or 2 depending on template
    pub caption_override: Option<String>,     // per-page caption override
}

pub enum PaperSize {
    A5,         // 148 × 210 mm — classic zine
    HalfLetter, // 139.7 × 215.9 mm — North American zine
    Square6,    // 152 × 152 mm — photo-book square
    A4,         // 210 × 297 mm — full page
}
```

#### AI Sequence Suggestion

Claude reads all approved captions → suggests a narrative page order with chapter groupings:

```
Prompt: "You are sequencing a street photography zine. Here are the captions for
24 photographs in import order. Suggest a compelling narrative sequence and identify
3–4 natural chapter breaks. Explain your logic briefly."
```

Output: numbered sequence with brief chapter labels → user accepts/adjusts.

#### PDF Export (Rust, `printpdf`)

- Full-res developed JPEGs embedded at print resolution
- Correct paper dimensions with 3mm bleed for full-bleed templates
- Caption text embedded (system font for v1, bundled font for v2)
- Optional: crop marks for print-shop output
- Print-on-demand target: A5 output compatible with Newspaper Club, Blurb, Artifact Uprising

---

## UI/UX Design Direction

### Aesthetic

- **Dark theme** — darkroom-inspired, very dark grey (`#111` background, `#1a1a1a` panels)
- **Not pure black** — pure black makes it hard to judge image blacks correctly
- **Warm amber accent** — keep `#DCA032` — it's distinctive and photographic
- **Green/red/amber** for pick/reject/flag — keep the existing semantic colours
- **Minimal chrome** — 80%+ of screen is the image at all times
- **Editorial typography** — Inter for UI labels, `font-mono` for all metadata values
- **Subtle film grain texture** on panel backgrounds (`noise.png` at 3% opacity)
- **Not over-designed** — functional, honest, gritty. Not Instagram. Not a social app.

### Stage Navigation

Left sidebar — icon-only activity bar (VS Code pattern), collapses to icons:

```
│ ⬇ │  Ingest     [badge: total images]
│ ◈ │  Cull        [badge: unreviewed count]
│ ◑ │  Analyse     [badge: picks ready]
│ ◐ │  Develop     [badge: undeveloped picks]
│ ✎ │  Caption     [badge: uncaptioned]
│ ⧉ │  Zine        [badge: active zines]
│   │
│ ⚙ │  Settings
```

Each badge count drives urgency — you can see at a glance where work remains.

### Cull Stage

```
┌─ Top bar (44px) ──────────────────────────────────────────────────────────┐
│  [ShotList icon]  SessionName        ✓ 23  ✗ 8  ○ 91  [▰▰▰▱▱ 34%]      │
├───────────────────────────────────────────────────────────────────────────┤
│                                                                           │
│                         MAIN VIEWER                                      │
│                  (image fills all available space)                       │
│                                                                           │
│  ┌─ overlay: top-left ─┐    ┌─ overlay: top-right ──────────────────┐   │
│  │  PICK / REJECT / …  │    │  4 / 114   IMG_4821.CR3   22 MB      │   │
│  └─────────────────────┘    └───────────────────────────────────────┘   │
│                                                                           │
│  ┌─ overlay: bottom ─────────────────────────────────────────────────┐  │
│  │  ISO 3200  1/500s  f/2.8  35mm  |  ⊙ 78%  |  ★★★☆☆  |  Burst 3/7│  │
│  └───────────────────────────────────────────────────────────────────┘  │
├─ Filter bar (32px) ───────────────────────────────────────────────────────┤
│  All (114)  ○ Unreviewed (83)  ✓ Picks (23)  ✗ Rejects (8)  ⚑ Flag (5) │
├─ Filmstrip (108px) ───────────────────────────────────────────────────────┤
│  [  ][  ][  ][▶▶▶SELECTED▶▶▶][  ][  ][  ][  ][  ][  ][  ][  ][  ][  ] │
└───────────────────────────────────────────────────────────────────────────┘
```

Metadata overlaid on the image (not in a separate bar) — maximises the image area.
Overlay text uses a semi-transparent pill background so it's readable on any image.

### Filmstrip Improvements (React redesign)

- **Fix badge overlap:** burst badge → bottom-right, face count → top-right, cull state → top-left, exposure → bottom-left (no more collisions)
- **Variable height:** drag handle on the filmstrip divider to resize (80px → 160px)
- **Hover tooltip:** filename + focus score + face count appears on hover
- **Smooth scroll:** CSS scroll-behavior: smooth + scroll into view on cursor change

### Develop Stage

```
┌─ Top bar ─────────────────────────────────────────────────────────────────┐
│  ◐ Develop  |  IMG_4821.CR3  |  [Reset]  [⚡ Auto]  [Export JPEG]        │
├─────────────────────────────────────────────────┬─────────────────────────┤
│                                                 │  LIGHT                  │
│                                                 │  Exposure    ──●──      │
│               VIEWER                           │  Highlights  ────●─     │
│     (live preview updates as sliders move)     │  Shadows     ─●────     │
│                                                 │  Whites      ────●─     │
│                                                 │  Blacks      ──●──      │
│                                                 │  Contrast    ───●─      │
│                                                 │  Clarity     ──●──      │
│                                                 ├─────────────────────────┤
│                                                 │  COLOUR                 │
│                                                 │  Temp   ────●─  4200K   │
│                                                 │  Tint   ──●────         │
│                                                 │  Vibrance ──●──         │
│                                                 ├─────────────────────────┤
│                                                 │  EFFECTS                │
│                                                 │  Grain  ─────●  40/25   │
│                                                 │  Vignette ──●──         │
│                                                 │  [B&W mode toggle]      │
│                                                 ├─────────────────────────┤
│                                                 │  AI EXPLANATION         │
│                                                 │  ┌───────────────────┐  │
│                                                 │  │ "I pulled the     │  │
│                                                 │  │  highlights down  │  │
│                                                 │  │  to -55 to        │  │
│                                                 │  │  recover the…"   │  │
│                                                 │  └───────────────────┘  │
├─────────────────────────────────────────────────┴─────────────────────────┤
│  Filmstrip                                                                │
└───────────────────────────────────────────────────────────────────────────┘
```

### Caption Stage

```
┌─ Top bar ─────────────────────────────────────────────────────────────────┐
│  ✎ Caption  |  23 picks  |  5 captioned  |  18 remaining                  │
├─────────────────────────────────────────────────┬─────────────────────────┤
│                                                 │                         │
│                                                 │  CAPTION                │
│               VIEWER                           │  ┌───────────────────┐  │
│                                                 │  │ A man in a grey   │  │
│                                                 │  │ coat crosses the  │  │
│                                                 │  │ intersection at   │  │
│                                                 │  │ Shinjuku station  │  │
│                                                 │  │ in the rain.      │  │
│                                                 │  └───────────────────┘  │
│                                                 │                         │
│                                                 │  [⚡ Generate]          │
│                                                 │  [Regenerate with note] │
│                                                 │  [✓ Approve → next]     │
│                                                 │                         │
│                                                 │  Style note (optional): │
│                                                 │  [____________]         │
├─────────────────────────────────────────────────┴─────────────────────────┤
│  Filmstrip (approved captions show ✓ badge)                               │
└───────────────────────────────────────────────────────────────────────────┘
```

### Zine Builder Stage

```
┌─ Top bar ─────────────────────────────────────────────────────────────────┐
│  ⧉ Zine  |  "Tokyo, March 2025"  |  A5  |  [⚡ AI sequence]  [Export PDF] │
├─ Page strip (scrollable horizontal, ~120px) ──────────────────────────────┤
│  [Cover][p2][p3][p4][p5][p6]...  ← drag to reorder, click to select       │
├───────────────────────────────────────────────────────────────────────────┤
│                                                                           │
│                    PAGE PREVIEW (large, centred)                         │
│                                                                           │
│             ┌──────────────────────────────────┐                         │
│             │                                  │                         │
│             │    [full bleed photograph]        │                         │
│             │                                  │                         │
│             │  caption text at bottom          │                         │
│             └──────────────────────────────────┘                         │
│                                                                           │
├─ Templates ───────────────────────────────────────────────────────────────┤
│  [Full Bleed] [Centered] [Diptych] [Sequence] [Spread] [Text+Photo] ...  │
└───────────────────────────────────────────────────────────────────────────┘
```

---

## Database Schema (new additions)

```sql
-- Non-destructive develop parameters
CREATE TABLE develop_params (
    image_path       TEXT PRIMARY KEY,
    session_id       TEXT NOT NULL,
    params_json      TEXT NOT NULL,          -- DevelopParams as JSON
    auto_applied_at  DATETIME,
    user_modified_at DATETIME,
    exported_path    TEXT                    -- path to last rendered JPEG
);

-- Per-image captions
ALTER TABLE images ADD COLUMN caption_ai_draft TEXT;
ALTER TABLE images ADD COLUMN caption_final    TEXT;
ALTER TABLE images ADD COLUMN caption_approved INTEGER DEFAULT 0;

-- Analysis cache
CREATE TABLE image_analysis (
    image_path             TEXT PRIMARY KEY,
    histogram_json         TEXT NOT NULL,
    zone_distribution_json TEXT NOT NULL,
    sharpness_grid_json    TEXT NOT NULL,
    colour_temp_k          INTEGER,
    highlight_clip_pct     REAL,
    shadow_crush_pct       REAL,
    computed_at            DATETIME NOT NULL
);

-- Zine sessions
CREATE TABLE zines (
    id          TEXT PRIMARY KEY,
    session_id  TEXT NOT NULL,
    title       TEXT NOT NULL,
    paper_size  TEXT NOT NULL,           -- 'A5' | 'HalfLetter' | 'Square6' | 'A4'
    created_at  DATETIME NOT NULL,
    updated_at  DATETIME NOT NULL
);

CREATE TABLE zine_pages (
    id               TEXT PRIMARY KEY,
    zine_id          TEXT NOT NULL REFERENCES zines(id),
    page_order       INTEGER NOT NULL,
    template         TEXT NOT NULL,
    image_ids_json   TEXT NOT NULL,      -- JSON array of ImageIds
    caption_override TEXT,
    UNIQUE(zine_id, page_order)
);
```

---

## Build Phases

### Phase 0 — Tauri Migration ✅ COMPLETE

Objective: Current culling workflow running inside Tauri, feature-for-feature parity.

- [x] Set up Tauri 2.0 project, React + TypeScript + Tailwind
- [x] Expose existing Rust backend as Tauri IPC commands (data URL approach, no asset protocol)
- [x] Build React skeleton: stage nav, filmstrip, viewer, cull controls
- [x] Thumbnail pipeline via data: URLs (base64 encoded, avoids asset protocol issues)
- [x] Filter bar, burst grouping, settings window
- [x] Toast system in React (Zustand store)
- [x] Auto-cull fixed (AppState writeback bug resolved — signals now visible to run_auto_cull)

**Exit criteria met:** Import a folder, cull images with keyboard shortcuts, export picks — all working.

### Phase 1 — Technical Analyser ✅ COMPLETE

Objective: Per-image technical breakdown.

- [x] Rust: `compute_analysis()` → `ImageAnalysis` struct
- [x] IPC command: `get_image_analysis(imageId)`
- [x] RGB + Luminance histogram (256-bucket canvas, filled areas + lum stroke line)
- [x] Ansel Adams Zone System (11-zone bar chart, Zone V/Highlights/Shadows callouts)
- [x] Spatial focus heatmap (8×5 Laplacian energy grid at correct aspect ratio)
- [x] Colour temperature estimation (R/B ratio from neutral pixels, K value + gradient strip)
- [x] Exposure stats: mean luma bar, blown %, crushed %

**Exit criteria met:** Full technical analysis visible for any image that has been scrolled in the filmstrip.

### Phase 2 — Develop ✅ COMPLETE

Objective: Non-destructive parametric editing with AI streaming suggestions.

- [x] Rust: `DevelopParams` struct (13 parameters + B&W 6-channel mixer)
- [x] Rust: `apply_develop()` — 12-step pipeline (WB → Exposure → H/S → W/B → gamma → Contrast → Clarity → Vibrance/Sat → Vignette → Grain → B&W)
- [x] IPC commands: `get_develop_params`, `save_develop_params`, `get_develop_preview`, `auto_develop`
- [x] DB: V2 migration (`develop_params` table)
- [x] React: accordion slider panels (Basic / Presence / WB / Effects / B&W)
- [x] Live preview with 180ms debounce; Before/After toggle
- [x] Claude SSE streaming — auto-develop suggestions stream as NDJSON, emitted as `develop-suggestion` events
- [x] AI feed UI: cards with param, value, reason; Apply / Apply All

**Exit criteria met:** Image → auto-develop suggestions from Claude → tweak sliders → live preview.

### Phase 3 — Caption ✅ COMPLETE

Objective: Editorial captions via Claude Vision, embedded into XMP sidecars.

- [x] Rust: `Caption` struct (title, description, keywords, location, credit)
- [x] DB: V3 migration (`captions` table with `generated_at` timestamp)
- [x] DB: `save_caption`, `save_caption_generated`, `load_caption`, `load_all_captions`
- [x] Claude Vision streaming: outputs 5 NDJSON lines, emits `caption-field` events per field
- [x] IPC commands: `get_caption`, `save_caption`, `generate_caption`, `embed_captions`
- [x] XMP extended: `write_sidecar_with_caption` adds `dc:title/description/subject`, `Iptc4xmpCore:Location`, `dc:creator`
- [x] React: caption panel with title/description/keywords chip editor/location/credit fields
- [x] Live field-by-field fill as Claude streams; Save only when dirty; Embed XMP batch button

**Exit criteria met:** Claude generates captions field-by-field live; saved to DB; embedded into XMP sidecars readable by Lightroom/Capture One.

### Phase 4 — Zine Builder ✅ COMPLETE

Objective: Sequence → template → PDF. The final creative output.

- [x] Rust: `Zine` / `ZinePage` / `PaperSize` / `TemplateId` structs (`src/core/zine.rs`)
- [x] SQLite V4 migration — `zines` + `zine_pages` tables with cascade delete
- [x] IPC commands: `create_or_get_zine`, `get_zine`, `update_page_order`, `set_page_template`, `set_page_caption_override`, `set_zine_paper_size`, `export_pdf`, `ai_sequence`
- [x] Rust: `printpdf 0.9.1` PDF generation — 6 templates (FullBleed, Top75, Left50, Right50, GridTwo, TitlePage), word-wrap text, letterbox image fit (`src/core/pdf.rs`)
- [x] AI: Claude streaming narrative sequence suggestion (`src/ai/sequence.rs`) — emits `sequence-suggestion` + `sequence-summary` Tauri events; auto-applies order to DB
- [x] React: horizontal page strip with @dnd-kit drag-to-reorder
- [x] React: CSS-based page preview with all 6 template layouts
- [x] React: right-panel inspector — template picker, caption override editor, auto-caption preview, AI suggestion cards
- [x] Paper size selector (A4, A5, Square, Half-Letter)
- [x] Export progress via `pdf-export-progress` events

**Exit criteria:** picks + captions → drag to sequence → assign templates → export A5 PDF ✅

---

## Licensing

| Dependency      | License          | Commercial use                 |
| --------------- | ---------------- | ------------------------------ |
| Tauri 2.0       | MIT + Apache 2.0 | ✅ Free, closed-source OK      |
| React           | MIT              | ✅ Free                        |
| Tailwind CSS    | MIT              | ✅ Free                        |
| shadcn/ui       | MIT              | ✅ Free                        |
| All Rust crates | MIT / Apache 2.0 | ✅ Free                        |
| Claude API      | Usage-based      | Users supply their own API key |

**Result:** ShotList can be sold as a proprietary commercial desktop app with zero framework licensing fees.
Claude API costs are borne by the user via their own Anthropic API key — no per-seat Claude cost for you.

---

## Open Questions (to resolve before or during build)

1. **App name:** "ShotList" (UI) / "shotlist" (repo) — neither captures the full scope. Worth renaming before public launch.
   Answer: ShotList
2. **RAW full demosaicing:** `rawler` supports full demosaicing but it's complex. Evaluate: shell out to `dcraw` or `rawtherapee-cli` for JPEG export in Phase 2 as a simpler v1 approach, then bring in-process later.
   Answer: Accepted the change
3. **GPU acceleration:** Tone mapping on 50MP+ RAW files may be slow in pure CPU Rust. Evaluate `wgpu` compute shaders as a Phase 3+ optimisation. Not needed for v1.
   Answer: Yes, agreed
4. **Offline graceful fallback:** Develop Auto algorithm works fully offline (pure Rust). AI explanation and caption require internet. When offline: Auto button works, explanation panel shows "Connect to use AI explanation", caption shows a placeholder.
   Answer: Yes, agreed
5. **Print-on-demand integration:** Future Phase 5 — direct upload to Newspaper Club, Blurb, or Artifact Uprising from the Zine stage. Not v1.
   Answer: Yea, this is not required for now
6. **iOS companion:** Long-term — read-only view of zines on iPhone. Not scoped.
   Answer: Yes, agreed

---

## Key Reference Points

- **Decisive moment (Cartier-Bresson):** The decisive moment philosophy shapes the cull UX — one keyboard press, immediate advance, no friction.
- **Magnum captions:** The caption prompt is modelled on Magnum Photos editorial caption style.
- **Ansel Adams Zone System:** The exposure zone display in the Analyser teaches the zone system implicitly as you work.
- **Narrative Select:** The culling UX inspiration. We meet or exceed it.
- **Lightroom Auto:** The target for the develop Auto button. We explain what Lightroom doesn't.

---

_Created: 2026-06-08_
_Updated: 2026-06-09_
_Status: Phases 0–4 complete. Full pipeline from RAW dump to zine PDF is working._
