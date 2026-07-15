//! CANVAS DRAW — the Sheet stage's thin, DOM-facing Canvas2D draw layer (S3.5).
//!
//! This is the ONLY module in the crate that touches `web_sys` /
//! `CanvasRenderingContext2d`: it takes an already-built, already-tested
//! [`RenderPlan`] (S3.3/S3.4, [`crate::render_plan`]) and paints it onto a 2D
//! context, plus resolves the Strand `--dna-*` palette off a live element. It
//! does **no geometry math** beyond reading the plan's rects verbatim and **no
//! engine truth**: what the plan says is exactly what is drawn (Foundation
//! doctrine — the pixels are an honest function of the plan, never a fabricated
//! or re-derived grid).
//!
//! **Why this layer is not unit-tested.** A `CanvasRenderingContext2d` only
//! exists inside a browser, so the draw routine is build-verified here and
//! browser-tested at S3.11 (against the debug readout the mount publishes),
//! never screenshot-asserted. The parts that *are* pure — [`Palette::fallback`]
//! and [`looks_numeric`] — carry their own native unit tests.
//!
//! **Full redraw per change.** [`draw_render_plan`] repaints the whole viewport
//! every call; there is no tile cache or dirty-region tracking. That is correct
//! at the bounded-sheet scale this stage targets — real tiling and
//! interest-window narrowing (`SetGridInterest`) are the G4 / S3.9 concern, out
//! of scope here.
//!
//! **Native vs wasm.** `web_sys` compiles on both native and `wasm32` (its
//! imported functions are link-time stubs on native that panic only if called),
//! so this module builds under the crate's native `ssr` test build without any
//! `cfg` gate. The draw routine is simply never *invoked* natively — the mount
//! wires it into a Leptos `Effect`, which does not run under `ssr`.
//!
//! **S3.7 (this bead)** adds [`draw_overlays`]: the read-only overlay pass
//! (structured tables, spilled-array regions, merged cells) drawn from a
//! [`dnacalc_skin_ir::GridOverlayBundle`] — over the cells, honoring the
//! window's `clipped_*` edge flags with a dashed "continues beyond the
//! window" affordance rather than a fabricated hard border. Its pixel geometry
//! ([`overlay_pixel_rect`]) and its edge-style decision
//! ([`overlay_edge_style`]) are pure, DOM-free helpers with their own native
//! unit tests, matching [`looks_numeric`]/[`Palette::fallback`] above; the
//! `ctx`-touching draw calls themselves are build-verified only, per this
//! module's doctrine.

use web_sys::{CanvasRenderingContext2d, Element};

use dnacalc_skin_ir::{
    GridMergedOverlayDescriptor, GridOverlayBundle, GridOverlayRect, GridSpillOverlayDescriptor,
    GridTableOverlayDescriptor,
};

use crate::geometry::{CellRect, GridMetrics, Viewport, cell_rect};
use crate::render_plan::RenderPlan;

/// The monospace stack the Sheet stage paints cell values in, at data size.
/// Fonts are not a `--dna-*` token (Strand tokenizes color/spacing, not the
/// sheet's fixed grid type), so the stack is a constant, mirroring the mono
/// stack `SHEET_CSS` already declares for the crate.
const SHEET_CELL_FONT: &str = "12px 'Recursive Mono','Cascadia Code',Consolas,ui-monospace,monospace";
/// The header-strip label font — the same stack a hair smaller than the cell
/// font, so A1 column letters / row numbers read as chrome, not data.
const SHEET_HEADER_FONT: &str = "11px 'Recursive Mono','Cascadia Code',Consolas,ui-monospace,monospace";

/// Horizontal padding inside a cell before its text, in CSS px.
const CELL_TEXT_PAD: f64 = 4.0;

/// The resolved Strand colors one draw pass paints with — plain CSS color
/// strings (`#RRGGBB`), read once per redraw off the live element via
/// [`resolve_palette`] so a theme flip repaints in the new theme.
///
/// The `*_font` fields are constants (fonts are not tokenized, see
/// [`SHEET_CELL_FONT`]); they live here so the draw routine reads every paint
/// input from one struct.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Palette {
    /// Grid background — `--dna-paper`.
    pub paper: String,
    /// Header-strip background — `--dna-paper-2`.
    pub paper_2: String,
    /// Cell text (non-numeric) — `--dna-ink`.
    pub ink: String,
    /// Header label ink — `--dna-ink-2`.
    pub ink_2: String,
    /// Gridline stroke — `--dna-line`.
    pub line: String,
    /// Numeric-value ink — `--dna-value-ink`.
    pub value_ink: String,
    /// Active-cell selection outline — `--dna-accent`. The 2px box
    /// [`draw_active_cell`] strokes around the selected cell (Excel's idiom),
    /// also reused by [`draw_overlays`] for a structured-table outline/tint.
    pub accent: String,
    /// Spill (live, non-blocked) veil fill + outline + origin-badge color —
    /// `--dna-signal-soft`, Strand's "external/volatile" channel (the same
    /// token the Bench x-ray uses for a `bound` expression chip, paired there
    /// with `--dna-prov-ext` ink).
    pub spill: String,
    /// Blocked-spill (`#SPILL!`) veil fill + outline + origin-badge color —
    /// `--dna-red-ink`, so a blocked spill reads as a distinct error treatment
    /// rather than the same color as a live one.
    pub spill_blocked: String,
    /// Cell value font (constant, see [`SHEET_CELL_FONT`]).
    pub cell_font: String,
    /// Header label font (constant, see [`SHEET_HEADER_FONT`]).
    pub header_font: String,
}

impl Palette {
    /// The last-resort palette: the cockpit-light hexes, used ONLY when a live
    /// `--dna-*` value cannot be read (a headless/SSR context with no computed
    /// style, or an empty custom property). These are a fallback so a
    /// misconfigured host still paints *something* legible — they are NOT the
    /// source of truth for the theme (that is Strand's `css_custom_properties`,
    /// resolved live in [`resolve_palette`]).
    #[must_use]
    pub fn fallback() -> Self {
        Self {
            paper: "#FFFFFF".to_string(),
            paper_2: "#EEF2F4".to_string(),
            ink: "#17282E".to_string(),
            ink_2: "#48606A".to_string(),
            line: "#D5DFE3".to_string(),
            value_ink: "#2E6E5B".to_string(),
            accent: "#318995".to_string(),
            spill: "#FFEDDB".to_string(),
            spill_blocked: "#D02A23".to_string(),
            cell_font: SHEET_CELL_FONT.to_string(),
            header_font: SHEET_HEADER_FONT.to_string(),
        }
    }
}

/// Resolve the live Strand palette off `el`'s computed style.
///
/// Reads each `--dna-*` custom property via `getComputedStyle(el)` (custom
/// properties inherit, so any element under `:root` resolves the theme's
/// values). A property that is missing or empty — or a headless context with no
/// `window`/computed style at all — falls back to the matching
/// [`Palette::fallback`] color, so the function is total and never panics.
///
/// JS-interop errors are swallowed to the fallback (`ok()`/`let ... else`),
/// never unwrapped: a hostile or half-initialized DOM degrades to a legible
/// paint rather than tearing down the stage.
#[must_use]
pub fn resolve_palette(el: &Element) -> Palette {
    let fallback = Palette::fallback();
    let Some(window) = web_sys::window() else {
        return fallback;
    };
    // `get_computed_style` returns `Result<Option<..>, JsValue>`: an error
    // (exception) or a `None` (detached element) both degrade to the fallback.
    let Ok(Some(style)) = window.get_computed_style(el) else {
        return fallback;
    };
    // Read one `--dna-*` value, trimming CSS whitespace; an empty/absent
    // property keeps the fallback color rather than painting with `""`.
    let read = |property: &str, fallback: &str| -> String {
        match style.get_property_value(property) {
            Ok(value) if !value.trim().is_empty() => value.trim().to_string(),
            _ => fallback.to_string(),
        }
    };
    Palette {
        paper: read("--dna-paper", &fallback.paper),
        paper_2: read("--dna-paper-2", &fallback.paper_2),
        ink: read("--dna-ink", &fallback.ink),
        ink_2: read("--dna-ink-2", &fallback.ink_2),
        line: read("--dna-line", &fallback.line),
        value_ink: read("--dna-value-ink", &fallback.value_ink),
        accent: read("--dna-accent", &fallback.accent),
        spill: read("--dna-signal-soft", &fallback.spill),
        spill_blocked: read("--dna-red-ink", &fallback.spill_blocked),
        cell_font: fallback.cell_font,
        header_font: fallback.header_font,
    }
}

/// Draw `plan` onto `ctx` for viewport `v` under metrics `m`, using `palette`.
///
/// The pass, in paint order (later layers overdraw earlier ones):
/// 1. clear the whole viewport to `--dna-paper`;
/// 2. fill the top + left header strips with `--dna-paper-2`;
/// 3. stroke the gridlines in `--dna-line` — derived from the plan's visible
///    header bands (the plan intentionally does not materialize gridlines,
///    [`crate::render_plan`]), one line per band edge plus the header/data
///    separators;
/// 4. draw each [`crate::render_plan::PlannedCell`]'s text, clipped to its own
///    rect so overflow never bleeds into the neighbor — numeric-looking values
///    right-aligned in `--dna-value-ink` (Excel's convention, [`looks_numeric`]),
///    everything else left-aligned in `--dna-ink`;
/// 5. draw each header band's label centered in its rect, in `--dna-ink-2`.
///
/// Every rect is taken verbatim from the plan; this routine computes no cell
/// geometry of its own. The `ctx` is assumed already scaled to CSS px (the
/// caller applies the device-pixel-ratio transform), so all coordinates here
/// are the plan's CSS-px rects.
pub fn draw_render_plan(
    ctx: &CanvasRenderingContext2d,
    plan: &RenderPlan,
    m: &GridMetrics,
    v: &Viewport,
    palette: &Palette,
) {
    // (1) Clear to paper. `fill_rect` over the whole viewport is the honest
    // "start from blank" for a full redraw.
    ctx.set_fill_style_str(&palette.paper);
    ctx.fill_rect(0.0, 0.0, v.width, v.height);

    // (2) Header strips: the top column-header band and the left row-header
    // band, both `paper-2`. The corner box is covered by both.
    ctx.set_fill_style_str(&palette.paper_2);
    ctx.fill_rect(0.0, 0.0, v.width, m.header_h);
    ctx.fill_rect(0.0, 0.0, m.header_w, v.height);

    // (3) Gridlines, derived from the visible bands. A vertical line at each
    // column band's left edge (plus the last band's right edge) and a
    // horizontal line at each row band's top edge (plus the last band's bottom
    // edge), each spanning the full viewport so the header strips are ruled
    // too. Finally the two header/data separators. Sub-pixel crispness snapping
    // is deferred (a cosmetic G4 refinement).
    ctx.set_stroke_style_str(&palette.line);
    ctx.set_line_width(1.0);
    ctx.begin_path();
    for header in &plan.col_headers {
        ctx.move_to(header.rect.x, 0.0);
        ctx.line_to(header.rect.x, v.height);
    }
    if let Some(last) = plan.col_headers.last() {
        let right = last.rect.x + last.rect.w;
        ctx.move_to(right, 0.0);
        ctx.line_to(right, v.height);
    }
    for header in &plan.row_headers {
        ctx.move_to(0.0, header.rect.y);
        ctx.line_to(v.width, header.rect.y);
    }
    if let Some(last) = plan.row_headers.last() {
        let bottom = last.rect.y + last.rect.h;
        ctx.move_to(0.0, bottom);
        ctx.line_to(v.width, bottom);
    }
    // Header/data separators.
    ctx.move_to(0.0, m.header_h);
    ctx.line_to(v.width, m.header_h);
    ctx.move_to(m.header_w, 0.0);
    ctx.line_to(m.header_w, v.height);
    ctx.stroke();

    // (4) Cell values, each clipped to its own rect. `save`/`clip`/`restore`
    // per cell keeps a wide value from spilling into the next column.
    ctx.set_text_baseline("middle");
    ctx.set_font(&palette.cell_font);
    for cell in &plan.cells {
        let rect = &cell.rect;
        ctx.save();
        ctx.begin_path();
        ctx.rect(rect.x, rect.y, rect.w, rect.h);
        ctx.clip();
        let mid_y = rect.y + rect.h / 2.0;
        if looks_numeric(&cell.text) {
            // Numbers right-align in value ink (Excel-strict convention).
            ctx.set_fill_style_str(&palette.value_ink);
            ctx.set_text_align("right");
            let _ = ctx.fill_text(&cell.text, rect.x + rect.w - CELL_TEXT_PAD, mid_y);
        } else {
            ctx.set_fill_style_str(&palette.ink);
            ctx.set_text_align("left");
            let _ = ctx.fill_text(&cell.text, rect.x + CELL_TEXT_PAD, mid_y);
        }
        ctx.restore();
    }

    // (5) Header labels: the plan already carries A1 letters (columns) and
    // numeric strings (rows) — drawn verbatim, centered, in header ink.
    ctx.set_fill_style_str(&palette.ink_2);
    ctx.set_text_align("center");
    ctx.set_font(&palette.header_font);
    for header in &plan.col_headers {
        draw_centered_label(ctx, &header.label, header.rect.x, header.rect.y, header.rect.w, header.rect.h);
    }
    for header in &plan.row_headers {
        draw_centered_label(ctx, &header.label, header.rect.x, header.rect.y, header.rect.w, header.rect.h);
    }
}

/// Stroke the active-cell selection outline over an already-drawn plan (S3.8).
///
/// A follow-on pass the redraw effect calls AFTER [`draw_render_plan`], so it
/// overdraws the gridlines with a 2px `--dna-accent` box around the selected cell
/// — the Excel idiom, and the ONLY selection chrome the stage renders (ranges,
/// fill grips and row/col bands are G3 degrades, so none are drawn). The stroke is
/// clipped to the data area (past the header strips) so a selection near the
/// origin never bleeds onto the headers, and the box is inset half the line width
/// so both edges land inside the cell's own rect rather than on the neighbor's
/// gridline. Outline-only (no fill) so the cell's value text stays legible under
/// the highlight.
///
/// `rect` is the selected cell's viewport-local [`crate::geometry::CellRect`]
/// (computed by the caller with the SAME metrics/viewport the plan drew with);
/// `m`/`v` supply the header offsets for the clip. Draws nothing — an honest
/// no-op — when the rect lies entirely under the header strips (fully
/// scrolled-off), never a clamped phantom box.
pub fn draw_active_cell(
    ctx: &CanvasRenderingContext2d,
    m: &GridMetrics,
    v: &Viewport,
    rect: &crate::geometry::CellRect,
    palette: &Palette,
) {
    // The data area past the two header strips — the region a selection may paint.
    let clip_w = (v.width - m.header_w).max(0.0);
    let clip_h = (v.height - m.header_h).max(0.0);
    if clip_w <= 0.0 || clip_h <= 0.0 {
        return; // No data area to draw into (viewport smaller than the headers).
    }
    // Nothing to draw if the cell is entirely under the header strips (scrolled
    // off the top-left) — honest absence, not a box clamped onto the headers.
    if rect.x + rect.w <= m.header_w || rect.y + rect.h <= m.header_h {
        return;
    }

    let line_w = 2.0;
    let inset = line_w / 2.0;
    ctx.save();
    // Clip to the data area so the outline cannot stroke onto the header strips.
    ctx.begin_path();
    ctx.rect(m.header_w, m.header_h, clip_w, clip_h);
    ctx.clip();
    ctx.set_stroke_style_str(&palette.accent);
    ctx.set_line_width(line_w);
    // Inset the box by half the line width so the 2px stroke sits inside the
    // cell's own rect (a stroke is centered on its path), not over the neighbor.
    ctx.stroke_rect(
        rect.x + inset,
        rect.y + inset,
        (rect.w - line_w).max(0.0),
        (rect.h - line_w).max(0.0),
    );
    ctx.restore();
}

// --- S3.7 overlays: tables / spills / merged --------------------------------

/// Stroke width for a merged-region outline — heavier than the 1px gridline
/// ([`draw_render_plan`]) so a merge reads as a real boundary, not another rule.
const MERGED_LINE_WIDTH: f64 = 2.0;
/// Stroke width for a structured-table range/band outline.
const TABLE_LINE_WIDTH: f64 = 1.5;
/// Stroke width for a spill extent's outline.
const SPILL_LINE_WIDTH: f64 = 1.5;
/// Translucency for a table header/totals band tint and a spill veil fill —
/// low enough that the cell values underneath stay legible through it.
const OVERLAY_TINT_ALPHA: f64 = 0.14;
/// Dash segment length (CSS px) for a "continues beyond the window" soft edge.
const SOFT_EDGE_DASH: f64 = 4.0;
/// Gap length (CSS px) between dash segments on a soft edge.
const SOFT_EDGE_GAP: f64 = 3.0;
/// Side length of the small filled square marking a spill's origin cell.
const SPILL_BADGE_SIZE: f64 = 6.0;

/// The viewport-local pixel rectangle an overlay descriptor's cell range
/// spans: the top-left corner of `cell_rect(top_row, left_col)` to the
/// bottom-right corner of `cell_rect(bottom_row, right_col)`.
///
/// A PURE function built entirely from [`cell_rect`] — no DOM, no engine
/// truth — so every overlay family (table/spill/merged) computes its pixel
/// rect through this one path and [`crate::geometry`]'s own invariants
/// (contiguous bands, exact scroll-translation) carry over for free. A
/// single-cell overlay rect (`top_row == bottom_row && left_col == right_col`)
/// is bit-for-bit `cell_rect(m, v, top_row, left_col)`.
#[must_use]
pub fn overlay_pixel_rect(m: &GridMetrics, v: &Viewport, r: &GridOverlayRect) -> CellRect {
    let top_left = cell_rect(m, v, r.top_row, r.left_col);
    let bottom_right = cell_rect(m, v, r.bottom_row, r.right_col);
    CellRect {
        x: top_left.x,
        y: top_left.y,
        w: (bottom_right.x + bottom_right.w) - top_left.x,
        h: (bottom_right.y + bottom_right.h) - top_left.y,
    }
}

/// Which edges of a window-clipped overlay rect are "hard" (a genuine
/// boundary, drawn solid) vs "soft" (the overlay continues beyond the viewed
/// window, drawn as a dashed affordance) — the direct translation of
/// [`GridOverlayRect`]'s `clipped_*` flags into a draw decision. Kept as its
/// own pure function (rather than inlined into the draw calls) so the
/// "which edge is which" mapping is unit-tested independently of any canvas
/// call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverlayEdgeStyle {
    pub top_soft: bool,
    pub right_soft: bool,
    pub bottom_soft: bool,
    pub left_soft: bool,
}

#[must_use]
pub fn overlay_edge_style(r: &GridOverlayRect) -> OverlayEdgeStyle {
    OverlayEdgeStyle {
        top_soft: r.clipped_top,
        right_soft: r.clipped_right,
        bottom_soft: r.clipped_bottom,
        left_soft: r.clipped_left,
    }
}

/// Stroke one straight, axis-aligned edge from `(x0, y0)` to `(x1, y1)` —
/// solid when `soft` is `false`, or as a manual dash (short segments with
/// gaps, [`SOFT_EDGE_DASH`]/[`SOFT_EDGE_GAP`]) when `soft` is `true` (the
/// "continues beyond the window" affordance for a `clipped_*` edge).
///
/// Drawn as hand-stepped segments rather than via
/// `CanvasRenderingContext2d::set_line_dash` (which takes a JS array), so this
/// module pulls in no `js_sys` array-marshalling surface for one cosmetic
/// line style. Assumes the caller has already set the stroke style + line
/// width; this owns only the path.
fn stroke_edge(ctx: &CanvasRenderingContext2d, x0: f64, y0: f64, x1: f64, y1: f64, soft: bool) {
    if !soft {
        ctx.begin_path();
        ctx.move_to(x0, y0);
        ctx.line_to(x1, y1);
        ctx.stroke();
        return;
    }
    let dx = x1 - x0;
    let dy = y1 - y0;
    let len = dx.hypot(dy);
    if len <= 0.0 {
        return; // A degenerate (zero-length) edge draws nothing.
    }
    let (ux, uy) = (dx / len, dy / len);
    let step = SOFT_EDGE_DASH + SOFT_EDGE_GAP;
    ctx.begin_path();
    let mut travelled = 0.0;
    while travelled < len {
        let seg_end = (travelled + SOFT_EDGE_DASH).min(len);
        ctx.move_to(x0 + ux * travelled, y0 + uy * travelled);
        ctx.line_to(x0 + ux * seg_end, y0 + uy * seg_end);
        travelled += step;
    }
    ctx.stroke();
}

/// Stroke all four edges of pixel rect `px`, per-edge solid/dashed per
/// `style` ([`overlay_edge_style`]), in `color` at `line_width`. Its own
/// `save`/`restore` so the stroke style/width set here never leaks into the
/// next overlay drawn.
fn stroke_overlay_rect(
    ctx: &CanvasRenderingContext2d,
    px: &CellRect,
    style: OverlayEdgeStyle,
    color: &str,
    line_width: f64,
) {
    ctx.save();
    ctx.set_stroke_style_str(color);
    ctx.set_line_width(line_width);
    let (left, top, right, bottom) = (px.x, px.y, px.x + px.w, px.y + px.h);
    stroke_edge(ctx, left, top, right, top, style.top_soft);
    stroke_edge(ctx, right, top, right, bottom, style.right_soft);
    stroke_edge(ctx, right, bottom, left, bottom, style.bottom_soft);
    stroke_edge(ctx, left, bottom, left, top, style.left_soft);
    ctx.restore();
}

/// Fill pixel rect `px` with `color` at `alpha` opacity — the shared
/// "veil"/tint primitive for a table header/totals band and a spill's
/// extent. Its own `save`/`restore` so `global_alpha` never leaks into a
/// later draw call.
fn fill_translucent(ctx: &CanvasRenderingContext2d, px: &CellRect, color: &str, alpha: f64) {
    ctx.save();
    ctx.set_global_alpha(alpha);
    ctx.set_fill_style_str(color);
    ctx.fill_rect(px.x, px.y, px.w, px.h);
    ctx.restore();
}

/// A merged-region overlay: a solid outline (heavier than a gridline) in
/// `--dna-line`, dashed on any window-clipped edge.
fn draw_merged_overlay(
    ctx: &CanvasRenderingContext2d,
    merged: &GridMergedOverlayDescriptor,
    m: &GridMetrics,
    v: &Viewport,
    palette: &Palette,
) {
    let px = overlay_pixel_rect(m, v, &merged.rect);
    let style = overlay_edge_style(&merged.rect);
    stroke_overlay_rect(ctx, &px, style, &palette.line, MERGED_LINE_WIDTH);
}

/// A structured-table overlay: an accent outline around the whole
/// `table_range`; the `header_rect`/`totals_rect` bands (if present) tinted
/// with a translucent accent fill plus an underline at the band/body seam;
/// and a light rule at each column band's boundary (skipping the boundary
/// that coincides with the range's own right edge, already stroked above).
/// Every sub-rect honors its OWN `clipped_*` flags — a table clipped at the
/// window's right edge draws a dashed right edge on the range AND on every
/// band/rule that reaches it.
fn draw_table_overlay(
    ctx: &CanvasRenderingContext2d,
    table: &GridTableOverlayDescriptor,
    m: &GridMetrics,
    v: &Viewport,
    palette: &Palette,
) {
    let range_px = overlay_pixel_rect(m, v, &table.table_range);
    let range_style = overlay_edge_style(&table.table_range);
    stroke_overlay_rect(ctx, &range_px, range_style, &palette.accent, TABLE_LINE_WIDTH);

    if let Some(header) = &table.header_rect {
        let px = overlay_pixel_rect(m, v, header);
        fill_translucent(ctx, &px, &palette.accent, OVERLAY_TINT_ALPHA);
        ctx.save();
        ctx.set_stroke_style_str(&palette.accent);
        ctx.set_line_width(TABLE_LINE_WIDTH);
        stroke_edge(ctx, px.x, px.y + px.h, px.x + px.w, px.y + px.h, header.clipped_bottom);
        ctx.restore();
    }
    if let Some(totals) = &table.totals_rect {
        let px = overlay_pixel_rect(m, v, totals);
        fill_translucent(ctx, &px, &palette.accent, OVERLAY_TINT_ALPHA);
        ctx.save();
        ctx.set_stroke_style_str(&palette.accent);
        ctx.set_line_width(TABLE_LINE_WIDTH);
        stroke_edge(ctx, px.x, px.y, px.x + px.w, px.y, totals.clipped_top);
        ctx.restore();
    }
    for column in &table.columns {
        if column.data_rect.right_col >= table.table_range.right_col {
            // Coincides with the range's own right edge — already stroked.
            continue;
        }
        let px = overlay_pixel_rect(m, v, &column.data_rect);
        ctx.save();
        ctx.set_stroke_style_str(&palette.line);
        ctx.set_line_width(1.0);
        stroke_edge(
            ctx,
            px.x + px.w,
            px.y,
            px.x + px.w,
            px.y + px.h,
            column.data_rect.clipped_right,
        );
        ctx.restore();
    }
}

/// A spilled-array overlay: `extent` drawn as a light veil (translucent fill)
/// with a dashed-on-clip outline in the spill (Strand "signal") color, plus a
/// small origin badge at the (unclipped) `(anchor_row, anchor_col)` cell. A
/// blocked spill (`#SPILL!`) swaps the whole treatment to the error ink so a
/// blocked spill reads as distinctly different from a live one, never the
/// same color.
fn draw_spill_overlay(
    ctx: &CanvasRenderingContext2d,
    spill: &GridSpillOverlayDescriptor,
    m: &GridMetrics,
    v: &Viewport,
    palette: &Palette,
) {
    let ink = if spill.blocked {
        &palette.spill_blocked
    } else {
        &palette.spill
    };
    let px = overlay_pixel_rect(m, v, &spill.extent);
    let style = overlay_edge_style(&spill.extent);

    fill_translucent(ctx, &px, ink, OVERLAY_TINT_ALPHA);
    stroke_overlay_rect(ctx, &px, style, ink, SPILL_LINE_WIDTH);

    // Origin badge, in the same ink — an unclipped address, so it is drawn
    // wherever it lands; the data-area clip `draw_overlays` established
    // naturally hides it if it falls outside the current window.
    let anchor_px = cell_rect(m, v, spill.anchor_row, spill.anchor_col);
    ctx.save();
    ctx.set_fill_style_str(ink);
    ctx.fill_rect(anchor_px.x, anchor_px.y, SPILL_BADGE_SIZE, SPILL_BADGE_SIZE);
    ctx.restore();
}

/// Draw `bundle`'s read-only overlays (S3.7) on top of an already-drawn
/// [`RenderPlan`] — structured tables, spilled-array regions, and merged
/// cells. Draws EXACTLY what `bundle` contains and nothing else: an empty
/// bundle (the demo workbook, which is cells-only) draws nothing, never a
/// fabricated overlay.
///
/// Paint order: merged, then tables, then spills — spills, the most dynamic /
/// attention-worthy family, are drawn last so one is never hidden under a
/// table/merge outline that happens to coincide.
///
/// Every rect is computed by [`overlay_pixel_rect`] over the SAME `m`/`v` the
/// plan was drawn with, so an overlay lines up with the cells underneath at
/// any scroll/zoom. The whole pass is clipped to the data area (past the two
/// header strips — the same clip [`draw_active_cell`] uses) so an overlay
/// scrolled toward the origin never bleeds onto the header chrome.
///
/// A `clipped_*` edge ([`GridOverlayRect`] — the window cut the overlay off)
/// draws as a dashed "continues beyond the window" affordance
/// ([`overlay_edge_style`]) rather than a hard solid border: the overlay
/// never claims a boundary the window did not actually see.
pub fn draw_overlays(
    ctx: &CanvasRenderingContext2d,
    bundle: &GridOverlayBundle,
    m: &GridMetrics,
    v: &Viewport,
    palette: &Palette,
) {
    if bundle.is_empty() {
        return; // Honest no-op: nothing fabricated for a cells-only grid.
    }
    let clip_w = (v.width - m.header_w).max(0.0);
    let clip_h = (v.height - m.header_h).max(0.0);
    if clip_w <= 0.0 || clip_h <= 0.0 {
        return; // No data area to draw into (viewport smaller than the headers).
    }

    ctx.save();
    ctx.begin_path();
    ctx.rect(m.header_w, m.header_h, clip_w, clip_h);
    ctx.clip();

    for merged in &bundle.merged {
        draw_merged_overlay(ctx, merged, m, v, palette);
    }
    for table in &bundle.tables {
        draw_table_overlay(ctx, table, m, v, palette);
    }
    for spill in &bundle.spills {
        draw_spill_overlay(ctx, spill, m, v, palette);
    }

    ctx.restore();
}

/// Draw `label` centered in the rect `(x, y, w, h)`, clipped to it. Assumes the
/// caller has already set `text_align("center")`, `text_baseline("middle")`,
/// the font, and the fill color — this only owns the clip + placement so the
/// two header loops share one implementation.
fn draw_centered_label(ctx: &CanvasRenderingContext2d, label: &str, x: f64, y: f64, w: f64, h: f64) {
    ctx.save();
    ctx.begin_path();
    ctx.rect(x, y, w, h);
    ctx.clip();
    let _ = ctx.fill_text(label, x + w / 2.0, y + h / 2.0);
    ctx.restore();
}

/// Whether `text` reads as a plain number, so the renderer right-aligns it in
/// value ink (Excel-strict: numbers right, text left). A pure display heuristic
/// over the *already-rendered* text — it never re-parses the engine value, so
/// it cannot fabricate or alter data, only choose an alignment. A parenthesized
/// state marker (`(empty)`, `(pending)`) or an error string is not numeric and
/// left-aligns.
#[must_use]
pub fn looks_numeric(text: &str) -> bool {
    let trimmed = text.trim();
    !trimmed.is_empty() && trimmed.parse::<f64>().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The fallback palette is total: every color slot is a non-empty string,
    /// so a headless/SSR paint (where no `--dna-*` resolves) still has a legible
    /// color for every layer rather than an empty `fillStyle`.
    #[test]
    fn fallback_palette_fills_every_slot() {
        let palette = Palette::fallback();
        for (slot, value) in [
            ("paper", &palette.paper),
            ("paper_2", &palette.paper_2),
            ("ink", &palette.ink),
            ("ink_2", &palette.ink_2),
            ("line", &palette.line),
            ("value_ink", &palette.value_ink),
            ("accent", &palette.accent),
            ("spill", &palette.spill),
            ("spill_blocked", &palette.spill_blocked),
            ("cell_font", &palette.cell_font),
            ("header_font", &palette.header_font),
        ] {
            assert!(!value.is_empty(), "fallback palette slot {slot} must not be empty");
        }
        // The color slots are the cockpit-light hexes Strand emits, so the
        // fallback matches the default theme rather than an arbitrary color.
        assert_eq!(palette.paper, "#FFFFFF");
        assert_eq!(palette.value_ink, "#2E6E5B");
        assert_eq!(palette.accent, "#318995");
        assert_eq!(palette.spill, "#FFEDDB");
        assert_eq!(palette.spill_blocked, "#D02A23");
    }

    /// `looks_numeric` is honest about what right-aligns: integers, decimals,
    /// signed and exponent forms are numeric; text, error strings, empty, and
    /// the parenthesized state markers `value_text` emits are not.
    #[test]
    fn looks_numeric_matches_only_plain_numbers() {
        for numeric in ["1", "10", "-3", "3.14", "0", "  42  ", "1e3", "-0.5"] {
            assert!(looks_numeric(numeric), "{numeric:?} should read as numeric");
        }
        for non_numeric in [
            "",
            "   ",
            "hello",
            "Sheet1!A1",
            "#DIV/0!",
            "(empty)",
            "(pending)",
            "2\u{00d7}3 array",
            "1,000",
        ] {
            assert!(
                !looks_numeric(non_numeric),
                "{non_numeric:?} should NOT read as numeric"
            );
        }
    }

    /// An overlay rect with no `clipped_*` flags set — the common fixture for
    /// [`overlay_pixel_rect`] geometry tests, which don't care about clipping.
    fn overlay_rect(top_row: u32, left_col: u32, bottom_row: u32, right_col: u32) -> GridOverlayRect {
        GridOverlayRect {
            top_row,
            left_col,
            bottom_row,
            right_col,
            clipped_top: false,
            clipped_left: false,
            clipped_bottom: false,
            clipped_right: false,
        }
    }

    /// A single-cell overlay rect (`top_row == bottom_row && left_col ==
    /// right_col`) is bit-for-bit `cell_rect` at that address — the base case
    /// every multi-cell span builds on.
    #[test]
    fn overlay_pixel_rect_of_a_single_cell_matches_cell_rect() {
        let m = GridMetrics::default();
        let v = Viewport {
            scroll_x: 0.0,
            scroll_y: 0.0,
            width: 800.0,
            height: 600.0,
        };
        let r = overlay_rect(4, 3, 4, 3);
        assert_eq!(overlay_pixel_rect(&m, &v, &r), cell_rect(&m, &v, 4, 3));
    }

    /// A multi-cell overlay rect spans exactly the cells it names: its
    /// top-left corner is the top-left cell's own top-left corner, and its
    /// size is the full row/column count times the uniform cell extents.
    #[test]
    fn overlay_pixel_rect_spans_a_multi_cell_range() {
        let m = GridMetrics::default();
        let v = Viewport {
            scroll_x: 0.0,
            scroll_y: 0.0,
            width: 800.0,
            height: 600.0,
        };
        // Rows 2..=4 (3 rows), cols 3..=5 (3 cols).
        let r = overlay_rect(2, 3, 4, 5);
        let top_left = cell_rect(&m, &v, 2, 3);
        let px = overlay_pixel_rect(&m, &v, &r);
        assert_eq!(px.x, top_left.x);
        assert_eq!(px.y, top_left.y);
        assert_eq!(px.w, m.col_width * 3.0);
        assert_eq!(px.h, m.row_height * 3.0);
    }

    /// A scrolled overlay rect is exactly the unscrolled rect translated by
    /// `-scroll` — the same pure-translation invariant [`cell_rect`] carries,
    /// inherited here for free since `overlay_pixel_rect` is built from it.
    #[test]
    fn overlay_pixel_rect_translates_under_scroll() {
        let m = GridMetrics::default();
        let base = Viewport {
            scroll_x: 0.0,
            scroll_y: 0.0,
            width: 800.0,
            height: 600.0,
        };
        let scrolled = Viewport {
            scroll_x: 50.0,
            scroll_y: 30.0,
            ..base
        };
        let r = overlay_rect(2, 3, 4, 5);
        let base_px = overlay_pixel_rect(&m, &base, &r);
        let scrolled_px = overlay_pixel_rect(&m, &scrolled, &r);
        assert_eq!(scrolled_px.x, base_px.x - 50.0);
        assert_eq!(scrolled_px.y, base_px.y - 30.0);
        assert_eq!(scrolled_px.w, base_px.w);
        assert_eq!(scrolled_px.h, base_px.h);
    }

    /// `overlay_edge_style` is a direct, honest mirror of the descriptor's
    /// `clipped_*` flags — no edge is inverted or dropped, and an all-hard
    /// rect (nothing clipped) maps to an all-solid style.
    #[test]
    fn overlay_edge_style_mirrors_the_clipped_flags() {
        let mixed = GridOverlayRect {
            top_row: 1,
            left_col: 1,
            bottom_row: 2,
            right_col: 2,
            clipped_top: true,
            clipped_left: false,
            clipped_bottom: true,
            clipped_right: false,
        };
        assert_eq!(
            overlay_edge_style(&mixed),
            OverlayEdgeStyle {
                top_soft: true,
                right_soft: false,
                bottom_soft: true,
                left_soft: false,
            }
        );

        let all_hard = GridOverlayRect {
            clipped_top: false,
            clipped_left: false,
            clipped_bottom: false,
            clipped_right: false,
            ..mixed
        };
        assert_eq!(
            overlay_edge_style(&all_hard),
            OverlayEdgeStyle {
                top_soft: false,
                right_soft: false,
                bottom_soft: false,
                left_soft: false,
            }
        );

        let all_soft = GridOverlayRect {
            clipped_top: true,
            clipped_left: true,
            clipped_bottom: true,
            clipped_right: true,
            ..mixed
        };
        assert_eq!(
            overlay_edge_style(&all_soft),
            OverlayEdgeStyle {
                top_soft: true,
                right_soft: true,
                bottom_soft: true,
                left_soft: true,
            }
        );
    }
}
