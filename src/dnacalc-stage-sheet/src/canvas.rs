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

use web_sys::{CanvasRenderingContext2d, Element};

use crate::geometry::{GridMetrics, Viewport};
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
            ("cell_font", &palette.cell_font),
            ("header_font", &palette.header_font),
        ] {
            assert!(!value.is_empty(), "fallback palette slot {slot} must not be empty");
        }
        // The color slots are the cockpit-light hexes Strand emits, so the
        // fallback matches the default theme rather than an arbitrary color.
        assert_eq!(palette.paper, "#FFFFFF");
        assert_eq!(palette.value_ink, "#2E6E5B");
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
}
