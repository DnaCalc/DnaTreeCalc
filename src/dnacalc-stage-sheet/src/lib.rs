//! TIER: TP (Presentation) — T0 skin-ir + Leptos + TP crates only; no Ox* ever (P-gate).
//!
//! The Sheet stage (S3): the Excel-strict profile's home stage — a Canvas2D
//! grid rendered from a windowed [`dnacalc_skin_ir::GridProjection`]. The plan
//! (SHEET_SPEC) is a canvas tile renderer + exactly one DOM overlay editor, both
//! driven by a **RenderPlan IR** whose geometry and hit-test are pure functions
//! with unit-tested invariants (Foundation doctrine — no screenshot assertions).
//! Honest-degrade v1: the stage works at bounded-sheet scale until G4 makes
//! `SetGridInterest` real, single-cell selection until G3, and the Detail zoom
//! tier until the labeled-block/district tiers land.
//!
//! **S3.1 (the scaffold)** stood up the [`dnacalc_shell::StageSurface`] under
//! [`dnacalc_shell::StageId::Sheet`] and rendered honestly from host truth: an
//! explicit honest-empty card when the workspace carries no grid, else a
//! placeholder readout of the active grid's real extent + windowed cell count.
//! It is never rendered blank.
//!
//! **S3.5 (this bead)** replaces that placeholder with a real `<canvas>` that
//! draws the [`RenderPlan`] via [`crate::canvas::draw_render_plan`]: the honest
//! pixels of the windowed cells, header bands, gridlines, and value text. The
//! new code is the thin DOM-facing draw layer ([`crate::canvas`]) plus the
//! reactive redraw wiring in [`SheetStage::mount`] — device-pixel-ratio sizing
//! for crisp text, live `--dna-*` palette resolution, and a full redraw on each
//! workspace change (real tiling / `SetGridInterest` narrowing is G4 / S3.9).
//! The geometry and the plan themselves ([`crate::geometry`],
//! [`crate::render_plan`]) are pure + unit-tested and merely *consumed* here.
//! The honest-empty card still stands in for a workspace with no grid.
//!
//! A visually-hidden debug readout (`data-testid="sheet-render-plan"`) mirrors
//! the drawn plan's cell count + extent, so the S3.11 browser test can assert
//! the demo grid rendered without pixel-reading the canvas.
//!
//! **S3.6 (this bead)** adds the core grid interaction: a click on the canvas
//! hit-tests to a cell ([`hit_test`]), which becomes the single selected/active
//! cell; exactly ONE DOM overlay editor ([`dnacalc_bridge::FormulaBridgeDegrade`])
//! then floats over the canvas at that cell's rect ([`cell_rect`]), seeded from
//! the cell's own authored text. Enter commits through the universal entry verb
//! ([`edit::enter_cell_intent`]) and reads the host's three-way outcome back
//! ([`edit::interpret_receipt`]); a successful commit closes the overlay and the
//! workbook dispatcher's re-projection repaints the canvas automatically (no
//! manual refresh), while a rejection keeps the editor open with the typed
//! diagnostics underlined; Esc cancels. The click-mapping and editor positioning
//! use the SAME [`GridMetrics::default`] + origin [`Viewport`] the redraw effect
//! draws with, so a click lands on — and the editor sits over — the drawn cell.
//! Single-cell selection only: arrow-key navigation and range/fill are S3.8.
//!
//! **S3.7 (this bead)** draws the grid's READ-ONLY overlays — structured
//! tables, spilled-array regions, merged cells — from the windowed
//! [`dnacalc_skin_ir::GridOverlayBundle`] the projection now carries
//! (`grid.overlays`), via [`crate::canvas::draw_overlays`]. The redraw effect
//! calls it right after [`draw_render_plan`] (on top of the cells) and BEFORE
//! the S3.8 active-cell highlight below (so a selection box always stays the
//! topmost chrome). A window-clipped overlay edge
//! (`GridOverlayRect::clipped_*`) draws as a dashed "continues beyond the
//! window" affordance rather than a fabricated hard border. Honest-empty v1:
//! the demo workbook is cells-only, so its `overlays` bundle is empty and this
//! pass draws nothing there — the value is the correct draw path plus the
//! unit-tested pure geometry ([`crate::canvas::overlay_pixel_rect`],
//! [`crate::canvas::overlay_edge_style`]) over a hand-built projection.
//!
//! **S3.8 (this bead)** splits S3.6's conflated "selected == editing" into the
//! proper Excel-strict SELECT-vs-EDIT model (SHEET_SPEC §4). SELECT is the base
//! state: a single click OR an arrow key moves the ACTIVE cell
//! ([`active_cell`](SheetStage)), drawn as a 2px `--dna-accent` highlight ON THE
//! CANVAS ([`draw_active_cell`]) by the same redraw effect (now subscribed to the
//! active cell) — no editor. EDIT opens the ONE overlay editor at the active cell
//! and is entered three ways (all reusing the S3.6 machinery): a printable key
//! (type-to-replace — seed the buffer with that character), **F2** or a
//! double-click (edit-in-place — seed with the cell's existing text). In SELECT
//! mode **Enter** moves the active cell DOWN (Excel-classic), it does NOT open the
//! editor. **Esc** returns to SELECT; an accepted commit (**Enter** WHILE editing)
//! writes through [`edit::enter_cell_intent`] and advances the active cell one row
//! down ([`next_active`]), staying in SELECT. The pure nav grammar and the edge-clamped
//! movement live in the unit-tested [`crate::selection`] module (an
//! atlas-taggable keymap table, mirroring the Notebook stage's `keyboard.rs`).
//!
//! **G3 honest degrades.** Everything the interaction pack (G3) owns — range
//! select (Shift+click / Shift+Arrow), whole row/column select (header click),
//! select-all (corner click), fill grips, and the grid clipboard — renders as a
//! visible disabled-with-reason affordance citing **G3** and performs ZERO
//! selection change beyond the single active cell and ZERO mutation. A reactive
//! `data-testid="sheet-g3-degrade"` note surfaces the reason when the user
//! attempts a gated GESTURE; a static strip of `disabled` chips
//! (`data-testid="sheet-g3-affordances"`) stands in for the gated TOOLS (fill,
//! clipboard). No fake fill handle is ever drawn (the highlight is the only
//! selection chrome).
//!
//! **S3.9 (this bead)** makes the grid SCROLLABLE and replaces the hardcoded
//! origin [`Viewport`] the earlier beads drew/clicked/edited with, with a single
//! reactive `viewport: RwSignal<Viewport>` — the ONE source of truth every
//! consumer agrees on. A `on:wheel` handler accumulates deltas into a
//! [`crate::viewport::ScrollCoalescer`] and a `requestAnimationFrame` callback
//! applies at most ONE clamped `viewport.scroll` update per frame (so a wheel
//! storm coalesces to one repaint — the estate's interest-coalescer pattern, the
//! same frame boundary G4 will emit real `SetGridInterest` from). The redraw
//! effect subscribes to the viewport, so a scroll repaints; the click / nav /
//! editor read the CURRENT viewport, so a click while scrolled lands on — and the
//! editor sits over — the right cell. Because `SetGridInterest` is a workbook
//! no-op today (G4 unbuilt), the stage works HONESTLY at bounded-sheet scale: it
//! renders the host's already-windowed cells as-is (no fabricated client-side
//! virtualization) and surfaces an honest `data-testid="sheet-interest-degrade"`
//! note that windowing / prefetch are deferred to G4. Ctrl+arrow edge-jump
//! degrades to a window-local jump ([`crate::viewport::window_edge_jump`]) with
//! the same honest note.
//!
//! **S3.10 (this bead)** adds SEMANTIC ZOOM: a `zoom: RwSignal<f64>` and a
//! `+`/`−`/reset control ([`crate::viewport::clamp_zoom`]) that scales the drawn
//! [`GridMetrics`] live via [`GridMetrics::scaled`]. The Detail tier (≥ 60%,
//! [`crate::viewport::zoom_tier`]) renders full cells/gridlines/text at the
//! scaled metrics. The Structure (15–60%) and District (< 15%) tiers
//! HONEST-DEGRADE v1: the labeled-block / district renderers are NOT built, so
//! the stage holds the Detail grid at the legibility floor
//! ([`crate::viewport::legible_factor`] — text never shrinks below readable) and
//! renders a `data-testid="sheet-zoom-degrade"` disabled-with-reason note that
//! the semantic tier is deferred, rather than half-building it. Default
//! `zoom = 1.0` + `scroll = 0` reproduce the earlier beads' exact geometry
//! (`GridMetrics::scaled(1.0)` is the identity, `Viewport::default()` is the
//! origin), so the S3.11 click test's cell math is unchanged.
//!
//! **dtc-j7n8.26 (editor focus ownership).** The overlay editor OWNS keyboard
//! focus from the moment it mounts until commit / revert: the Editable overlay
//! focuses the bridge's `<textarea>` (caret at the end, so type-to-replace keeps
//! typing after its seed character) through a mount effect; the section's
//! keydown pipeline routes any key that still reaches the section while EDITING
//! back into the editor ([`section_key_route`]) instead of re-running the SELECT
//! grammar over it; and an accepted commit / Esc revert hands focus back to the
//! `<section>` so the next arrow / F2 / typed key lands in the grammar again.
//! Before this the bridge textarea was never focused: after F2 or a typed key
//! every further keystroke hit the section (re-seeding the editor with THAT
//! character; Enter = Move(Down)) until the textarea itself was clicked.
//!
//! **dtc-j7n8.25 (Ctrl+S from inside the stage).** The shell's universal chords
//! (Ctrl+S / Ctrl+O / Ctrl+K / F9) must reach the shell's `.dna-shell` keydown
//! pipeline from any focus inside this stage: the degrade bridge's textarea no
//! longer stops every keydown (it stops only what it consumes), and a shell
//! chord the sheet grammar does not bind that reaches the section WHILE EDITING
//! is [`SectionKeyRoute::ShellOwns`] — left untouched so it bubbles — rather
//! than being consumed by the editor refocus.

use std::sync::Arc;

use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use leptos::wasm_bindgen::closure::Closure;
use web_sys::CanvasRenderingContext2d;

use dnacalc_bridge::{BridgeEvent, CommitAdvance, FormulaBridgeDegrade};
use dnacalc_shell::{ProfileTag, StageContext, StageHandle, StageId, StageSurface};
use dnacalc_skin_ir::intent::Dispatcher;
use dnacalc_skin_ir::{GridEntryDiagnosticProjection, GridProjection, NodeId, WorkspaceState};

pub mod canvas;
pub mod edit;
pub mod geometry;
pub mod gestures;
pub mod render_plan;
pub mod selection;
pub mod viewport;

pub use canvas::{
    OverlayEdgeStyle, Palette, draw_active_cell, draw_overlays, draw_render_plan, looks_numeric,
    overlay_edge_style, overlay_pixel_rect, resolve_palette,
};
pub use edit::{CellOutcome, enter_cell_intent, interpret_receipt};
pub use geometry::{
    CellRect, GridMetrics, HitTarget, Viewport, cell_rect, hit_test, visible_col_range,
    visible_row_range,
};
pub use gestures::{PanTapTracker, pinch_scale, point_distance};
pub use render_plan::{
    PlannedCell, PlannedColHeader, PlannedRowHeader, RenderPlan, build_render_plan, col_label,
    value_text,
};
pub use selection::{
    ActionAvailability, GridExtent, KeyBinding, KeyChord, NavAction, SectionKeyRoute, SheetAction,
    action_availability, is_shell_chord, next_active, resolve_action, section_key_route,
    sheet_keymap, typed_char,
};
pub use viewport::{
    BOUNDED_SCALE_INTEREST_NOTE, EDGE_JUMP_DEGRADE_REASON, EdgeDir, ScrollCoalescer, Tier,
    clamp_scroll, clamp_zoom, edge_dir_from_key, legible_factor, reveal_scroll,
    tier_degrade_reason, window_edge_jump, zoom_tier,
};

/// The Sheet stage surface.
#[derive(Debug, Clone, Copy, Default)]
pub struct SheetStage;

impl SheetStage {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

/// Monotonic milliseconds for gesture timing (`performance.now()`); 0.0 when
/// no window — native builds compile this but never invoke the handlers that
/// call it (no effects / event handlers under `ssr`).
fn performance_now_ms() -> f64 {
    web_sys::window()
        .and_then(|window| window.performance())
        .map(|performance| performance.now())
        .unwrap_or(0.0)
}

impl StageSurface for SheetStage {
    fn id(&self) -> StageId {
        StageId::Sheet
    }

    fn title(&self) -> &'static str {
        "Sheet"
    }

    fn supports(&self, profile: &ProfileTag) -> bool {
        matches!(profile, ProfileTag::ExcelStrict)
    }

    fn mount(&self, ctx: StageContext) -> StageHandle {
        // The one host-truth read the stage makes. `ReadSignal` is `Copy`, so
        // every reactive read below re-derives from live host truth — the grid
        // view is never a copy that can drift.
        let workspace = ctx.workspace;

        // The canvas node, and the two signals the redraw effect publishes for
        // the debug readout. All are created ONCE here in `mount` (the stable
        // owner), so a workspace re-projection re-runs only the inner reactive
        // fragments, never remints the canvas or its effect.
        let canvas_ref = NodeRef::<leptos::html::Canvas>::new();
        let plan_cell_count = RwSignal::new(0usize);
        let plan_extent = RwSignal::new(String::new());

        // Whether the workspace carries a grid to draw. A `Memo` fires only when
        // this boolean flips, so the `<canvas>` is built once (not remade on
        // every workspace change) — the redraw effect owns the per-projection
        // repaint, not a view rebuild.
        let has_grid = Memo::new(move |_| workspace.with(|ws| active_grid(ws).is_some()));

        // --- S3.8 select/edit state ------------------------------------------
        // The one dispatcher a commit crosses (SHELL_SPEC §6). Held in a
        // `StoredValue` (Copy) so the overlay closure — embedded inside the
        // re-runnable `has_grid` branch — stays `Copy`. The workbook dispatcher
        // re-projects on every accepted dispatch, so a committed edit repaints the
        // canvas through the redraw effect with no refresh of our own.
        let dispatch: StoredValue<Arc<dyn Dispatcher>> = StoredValue::new(ctx.dispatch.clone());
        // SELECT vs EDIT, split from S3.6's conflated `selected_cell`. `active_cell`
        // is the single selected cell — always drawn as the canvas highlight, and
        // it SURVIVES a repaint (created once here in the stable owner). `editing`
        // is the orthogonal mode bit: the overlay editor renders only when EDITING
        // at the active cell. A single click sets `active_cell` (SELECT); F2 /
        // a printable key / a double-click flip `editing` (EDIT); Enter moves down.
        let active_cell: RwSignal<Option<(u32, u32)>> = RwSignal::new(None);
        let editing = RwSignal::new(false);
        // The transient honest-degrade note (aria-live): set to a G3 reason when the
        // user attempts a range / row-col / select-all gesture, cleared on the next
        // real select/nav. Never gates a mutation — it only explains an absence.
        let g3_note: RwSignal<Option<String>> = RwSignal::new(None);
        // The one editor's live buffer, its last commit's typed rejections, and a
        // remount counter. Singular (there is exactly one active-cell editor): the
        // buffer is seeded on EDIT entry and read UNTRACKED on remount, so typing
        // never remounts the bridge — only entering EDIT or a commit/revert
        // (`editor_revision`) does.
        let edit_text = RwSignal::new(String::new());
        let editor_rejections: RwSignal<Vec<GridEntryDiagnosticProjection>> =
            RwSignal::new(Vec::new());
        let editor_revision = RwSignal::new(0usize);

        // --- S3.9 scroll / S3.10 zoom: the shared reactive viewport + zoom ------
        // The ONE source of truth for the scrolled viewport (S3.9) and the zoom
        // factor (S3.10). Every consumer — the redraw effect, the click / dblclick
        // hit-test, the nav grammar, and the overlay editor's position — reads
        // THESE, so they can never disagree on where a cell is drawn. Created once
        // here in the stable owner so scroll/zoom survive a workspace re-projection.
        //
        // `Viewport::default()` is the origin (scroll 0, size 0) and `zoom = 1.0`
        // is 100% — the defaults the earlier beads hardcoded, so with no scroll
        // and no zoom the geometry is bit-for-bit today's (the redraw effect fills
        // `width`/`height` from the live canvas size each paint).
        let viewport: RwSignal<Viewport> = RwSignal::new(Viewport::default());
        let zoom = RwSignal::new(1.0_f64);

        // --- Resize-reflow (dtc-4erh) ----------------------------------------
        // The redraw effect measures the canvas box + sizes the device-px backing
        // store, but its reactive inputs are workspace / scroll / zoom / selection —
        // a window/container RESIZE is none of those, so without this the backing
        // store would keep its old device dimensions and the browser would CSS-scale
        // that fixed bitmap (stretched / blurry, clicks slightly off) until the next
        // interaction repaints. `resize_tick` is a signal the redraw effect reads;
        // a `ResizeObserver` on the canvas bumps it on any size change, re-firing the
        // redraw to re-measure + repaint at the new size. The observer ALSO fires
        // once on `observe()`, so a canvas that mounts 0-tall and later gets a size
        // repaints on its own (the robustness edge behind the S3.12 blank preview).
        let resize_tick = RwSignal::new(0u32);
        // Install the observer exactly once (on the first redraw run where the
        // canvas is bound). A plain `bool` in a `StoredValue` (Send + Sync, Copy),
        // captured into the redraw closure.
        let resize_installed = StoredValue::new(false);

        // The reactive redraw. Subscribes to the workspace (a re-projection
        // repaints) and to `canvas_ref` (bound after the canvas mounts). Every
        // JS-interop step degrades gracefully (`let ... else` / `ok()`), never
        // unwrapping: a not-yet-mounted canvas, a missing `window`, or a null 2d
        // context is an early return, not a panic. Under `ssr` (native) Leptos
        // never runs effects, so this closure is the wasm-only draw path with an
        // automatic native no-op — no `cfg` gate needed.
        Effect::new(move |_| {
            // The canvas may not be in the DOM yet (first run, or no grid): bail
            // until its ref binds, at which point this effect re-runs.
            let Some(el) = canvas_ref.get() else {
                return;
            };
            let Some(window) = web_sys::window() else {
                return;
            };

            // Resize-reflow: install the ResizeObserver ONCE, on the first run
            // where the canvas is bound, then subscribe to `resize_tick` so a
            // canvas resize re-fires this effect (re-measure + repaint below). The
            // observer fires repeatedly, so its callback is a `Closure::new`/FnMut;
            // it uses `try_update` so a fire after the stage unmounts (disposed
            // signal) is a silent no-op, not a panic. The closure + observer are
            // `.forget()`/`mem::forget`'d — web_sys types are `!Send` so cannot
            // live in the default (Sync) `StoredValue`, and this is the same bounded
            // one-shot leak the scroll-coalescer RAF already uses; the observe()
            // holds the callback for the stage's lifetime.
            if !resize_installed.get_value() {
                resize_installed.set_value(true);
                let on_resize = Closure::<dyn FnMut()>::new(move || {
                    let _ = resize_tick.try_update(|tick| *tick = tick.wrapping_add(1));
                });
                if let Ok(observer) =
                    web_sys::ResizeObserver::new(on_resize.as_ref().unchecked_ref())
                {
                    observer.observe(&el);
                    on_resize.forget();
                    std::mem::forget(observer);
                }
            }
            // Subscribe: a `ResizeObserver` bump re-runs this effect to re-measure
            // `client_width`/`client_height` below at the new size.
            resize_tick.get();

            // DPR sizing: the backing store is device px (crisp text) while the
            // CSS box stays the container size (the stylesheet sizes it to 100%).
            // Setting width/height also resets the 2d transform, so we re-apply
            // the DPR scale each redraw and thereafter draw in CSS px.
            let dpr = window.device_pixel_ratio().max(1.0);
            let css_w = f64::from(el.client_width());
            let css_h = f64::from(el.client_height());
            el.set_width((css_w * dpr).round().max(0.0) as u32);
            el.set_height((css_h * dpr).round().max(0.0) as u32);

            let Some(ctx2d) = el
                .get_context("2d")
                .ok()
                .flatten()
                .and_then(|obj| obj.dyn_into::<CanvasRenderingContext2d>().ok())
            else {
                return;
            };
            let _ = ctx2d.scale(dpr, dpr);

            // S3.10 zoom: the drawn metrics are the default scaled by the live
            // zoom, floored at the legibility floor (so text never shrinks below
            // readable — Structure/District hold the grid here and add the honest
            // note on top). Reading `zoom` subscribes this effect, so a zoom press
            // repaints. `legible_factor(1.0) == 1.0` and `scaled(1.0)` is the
            // identity, so at default zoom the metrics are exactly today's.
            let metrics = zoomed_metrics(zoom.get());
            // S3.9 scroll: read the shared viewport for scroll_x/scroll_y
            // (subscribing this effect, so a wheel-scroll repaints) and rebuild it
            // with the freshly measured canvas size. The signal is the single
            // source of truth for scroll; the size is measured here each paint.
            let scrolled = viewport.get();
            let vp = Viewport {
                scroll_x: scrolled.scroll_x,
                scroll_y: scrolled.scroll_y,
                width: css_w,
                height: css_h,
            };
            // Feed the measured size back into the shared viewport so the wheel
            // coalescer's scroll clamp uses the real data area. Written UNTRACKED
            // so this self-write does not re-fire the effect (an infinite repaint
            // loop) — only a scroll or zoom change repaints, never a resize echo.
            if scrolled.width != css_w || scrolled.height != css_h {
                viewport.update_untracked(|v| {
                    v.width = css_w;
                    v.height = css_h;
                });
            }
            let palette = resolve_palette(&el);

            // Build + draw the plan from the SAME workspace read, and publish its
            // honest cell count + extent for the debug readout. Reading the
            // workspace here is what subscribes this effect to re-projections.
            let published = workspace.with(|ws| {
                active_grid(ws).map(|grid| {
                    let plan = build_render_plan(grid, &metrics, &vp);
                    draw_render_plan(&ctx2d, &plan, &metrics, &vp, &palette);
                    // S3.7 overlays (tables / spills / merged): drawn right after
                    // the plan, ON TOP of the cells, but BEFORE the active-cell
                    // highlight below — so the selection box always stays the
                    // topmost chrome even where it happens to coincide with an
                    // overlay's outline. `grid.overlays` comes from the SAME
                    // workspace read that built `plan`, so it repaints with every
                    // re-projection; an empty bundle (the demo workbook, which is
                    // cells-only) draws nothing.
                    draw_overlays(&ctx2d, &grid.overlays, &metrics, &vp, &palette);
                    (plan.cells.len(), plan.extent_rows, plan.extent_cols)
                })
            });
            if let Some((count, rows, cols)) = published {
                plan_cell_count.set(count);
                plan_extent.set(format!("{rows}\u{00d7}{cols}"));
            }

            // S3.8 active-cell highlight — a follow-on pass over the drawn plan
            // AND overlays (see the z-order note above).
            // Reading `active_cell` here subscribes the effect to selection, so an
            // arrow-key move or a click repaints the 2px accent box. Drawn with the
            // SAME scaled metrics + scrolled viewport the plan used, so the box
            // tracks the cell under scroll/zoom. Drawn only when a grid was
            // actually drawn (`published` is Some) AND a cell is active — the
            // highlight is the ONLY selection chrome (no fake handles).
            if published.is_some()
                && let Some((row, col)) = active_cell.get()
            {
                let rect = cell_rect(&metrics, &vp, row, col);
                draw_active_cell(&ctx2d, &metrics, &vp, &rect, &palette);
            }
        });

        // S3.9 wheel scroll → RAF-coalesced viewport update. Every wheel event
        // adds its delta to a per-frame [`ScrollCoalescer`]; only the FIRST note
        // since the last flush schedules a `requestAnimationFrame`, and further
        // wheel events in that frame just accumulate. The RAF callback drains the
        // coalescer ONCE and applies a single clamped `viewport.scroll` update, so
        // a wheel storm coalesces to at most one repaint per frame (the estate's
        // interest-coalescer pattern — the same frame boundary G4 will emit real
        // `SetGridInterest` from). The shared per-frame state lives in
        // `StoredValue` (Copy + Send, so the view closure that embeds this handler
        // stays `Send` under `ssr`, unlike an `Rc<RefCell<..>>`); every access is a
        // panic-safe `try_*` so a callback that fires after the stage unmounts (the
        // StoredValue disposed) is a silent no-op, never a JS-interop panic. Native
        // builds never invoke this (no effects / event handlers under `ssr`).
        let scroll_coalescer = StoredValue::new(ScrollCoalescer::new());
        let frame_scheduled = StoredValue::new(false);
        let wheel_workspace = workspace;
        let on_canvas_wheel = move |ev: leptos::ev::WheelEvent| {
            // Keep the wheel over the grid from also scrolling the page. (Best
            // effort — a passive listener may ignore it; the grid still scrolls.)
            ev.prevent_default();
            let (dx, dy) = (ev.delta_x(), ev.delta_y());
            let _ = scroll_coalescer.try_update_value(|c| c.note_delta(dx, dy));

            // Coalesce: only schedule a frame if one is not already pending. A
            // disposed StoredValue (stage unmounted) reads as "scheduled", so we
            // skip rather than schedule a doomed callback.
            if frame_scheduled.try_with_value(|v| *v).unwrap_or(true) {
                return;
            }
            let _ = frame_scheduled.try_update_value(|v| *v = true);

            // The RAF flush: drain the frame's accumulated delta and apply ONE
            // clamped scroll update. Reads zoom / viewport / extent UNTRACKED (this
            // is not a reactive scope), then writes `viewport.scroll` TRACKED so
            // the redraw effect repaints. `viewport.width`/`height` were fed in by
            // the redraw effect, so the clamp bounds to the real data area.
            let flush: Closure<dyn FnMut()> = Closure::once(move || {
                let _ = frame_scheduled.try_update_value(|v| *v = false);
                let Some((dx, dy)) = scroll_coalescer
                    .try_update_value(|c| c.take_pending())
                    .flatten()
                else {
                    return;
                };
                let metrics = zoomed_metrics(zoom.get_untracked());
                let current = viewport.get_untracked();
                let (extent_rows, extent_cols) = wheel_workspace
                    .with_untracked(|ws| active_grid(ws).map(|g| (g.max_rows, g.max_cols)))
                    .unwrap_or((1, 1));
                let next_x = clamp_scroll(
                    current.scroll_x + dx,
                    extent_cols,
                    metrics.col_width,
                    metrics.header_w,
                    current.width,
                );
                let next_y = clamp_scroll(
                    current.scroll_y + dy,
                    extent_rows,
                    metrics.row_height,
                    metrics.header_h,
                    current.height,
                );
                // Only write (and thus repaint) when the clamped scroll actually
                // moved — a wheel already parked at an edge is a no-op, not a
                // needless repaint.
                if next_x != current.scroll_x || next_y != current.scroll_y {
                    viewport.update(|v| {
                        v.scroll_x = next_x;
                        v.scroll_y = next_y;
                    });
                }
            });
            let Some(window) = web_sys::window() else {
                return;
            };
            let _ = window.request_animation_frame(flush.as_ref().unchecked_ref());
            // The RAF callback fires at most once (`Closure::once`); leaking it
            // here is the standard wasm-bindgen pattern for a one-shot callback
            // that must outlive the JS call site — at most one leak per scrolled
            // frame, bounded by the `frame_scheduled` gate (never one per event).
            flush.forget();
        };

        // The section the keyboard grammar listens on (`tabindex=0`); focused on a
        // pointer select so keys land here immediately after a click.
        let section_ref = NodeRef::<leptos::html::Section>::new();

        // Enter EDIT: seed the one editor's buffer, clear any stale rejection
        // underline, remount the bridge, select the target cell, and flip `editing`
        // on (clearing the degrade note). Reused by the three EDIT entry gestures
        // (F2, a printable key, a double-click). Captures only `Copy`
        // signals, so the closure is itself `Copy` and re-usable across handlers.
        let open_editor = move |row: u32, col: u32, seed: String| {
            edit_text.set(seed);
            editor_rejections.set(Vec::new());
            editor_revision.update(|revision| *revision += 1);
            active_cell.set(Some((row, col)));
            editing.set(true);
            g3_note.set(None);
        };

        // Touch-gesture state (SHELL_SPEC §1.1): the active touch pointers,
        // the tap/pan classifier, and an optional pinch origin (start span,
        // start zoom). Same StoredValue discipline as the scroll coalescer:
        // Copy state, panic-safe try_* accessors, silent no-op after unmount.
        let touch_points = StoredValue::new(Vec::<(i32, (f64, f64))>::new());
        let pantap = StoredValue::new(PanTapTracker::default());
        let pinch_start = StoredValue::new(Option::<(f64, f64)>::None);

        // Span between the first two active touch pointers (0.0 when fewer).
        let touch_span = |points: &StoredValue<Vec<(i32, (f64, f64))>>| -> f64 {
            points
                .try_with_value(|list| match list.as_slice() {
                    [(_, (x1, y1)), (_, (x2, y2))] => point_distance(*x1, *y1, *x2, *y2),
                    _ => 0.0,
                })
                .unwrap_or(0.0)
        };

        // One clamped scroll update (the wheel flush math, applied directly —
        // a finger-drag produces a bounded stream of moves, not a storm).
        let pan_workspace = workspace;
        let apply_pan_delta = move |dx: f64, dy: f64| {
            let metrics = zoomed_metrics(zoom.get_untracked());
            let current = viewport.get_untracked();
            let (extent_rows, extent_cols) = pan_workspace
                .with_untracked(|ws| active_grid(ws).map(|g| (g.max_rows, g.max_cols)))
                .unwrap_or((1, 1));
            let next_x = clamp_scroll(
                current.scroll_x + dx,
                extent_cols,
                metrics.col_width,
                metrics.header_w,
                current.width,
            );
            let next_y = clamp_scroll(
                current.scroll_y + dy,
                extent_rows,
                metrics.row_height,
                metrics.header_h,
                current.height,
            );
            if next_x != current.scroll_x || next_y != current.scroll_y {
                viewport.update(|v| {
                    v.scroll_x = next_x;
                    v.scroll_y = next_y;
                });
            }
        };

        // Pointer select → SELECT (not EDIT) — for mouse, pen AND touch. Map
        // the pointer into the SAME origin viewport / metrics the redraw
        // effect draws with, hit-test, and move the active cell — the
        // highlight, not the editor. `offset_x`/`offset_y` are already in the
        // drawn coordinate space (the canvas fills its viewport wrapper with
        // no border/padding). All captures are `Copy`.
        //
        // Touch bookkeeping rides on the same handler: selection happens at
        // `pointerdown` exactly where the old `mousedown` did, then the
        // `gestures` classifiers take over (double-tap = EDIT on pointerup,
        // one-finger drag = pan, two-finger pinch = zoom).
        let click_workspace = workspace;
        let on_canvas_pointerdown = move |ev: leptos::ev::PointerEvent| {
            // Read the pointer offset FIRST, before any `focus()` side effect: some
            // browsers compute `offsetX`/`offsetY` lazily against the target's
            // CURRENT layout, so focusing the section (which can trigger a reflow /
            // scroll-into-view) before reading them would perturb the hit-test
            // coordinate. Offset is already in the drawn CSS-px space (the canvas
            // fills its viewport wrapper with no border/padding).
            let x = f64::from(ev.offset_x());
            let y = f64::from(ev.offset_y());
            // Hit-test in the SAME scaled metrics + scrolled viewport the redraw
            // effect last drew with (read untracked — a handler is not a reactive
            // scope), so a click while scrolled/zoomed lands on the drawn cell.
            let metrics = zoomed_metrics(zoom.get_untracked());
            let vp = viewport.get_untracked();
            let Some((extent_rows, extent_cols)) = click_workspace
                .with_untracked(|ws| active_grid(ws).map(|g| (g.max_rows, g.max_cols)))
            else {
                return; // No grid: nothing to select.
            };
            // Focus the section so arrow-key navigation lands here right after the
            // pointer select (a native no-op — refs are unbound under `ssr`). Done
            // AFTER the offset read above so it cannot move the hit-test point.
            if let Some(section) = section_ref.get() {
                let _ = section.focus();
            }
            // Shift+click asks for a RANGE — a G3 degrade: surface the reason and
            // change NOTHING (zero selection change beyond the single active cell,
            // zero mutation, no fabricated range).
            if ev.shift_key() {
                g3_note.set(Some(selection::RANGE_SELECT_DEFERRED_REASON.to_string()));
                return;
            }
            match hit_test(&metrics, &vp, extent_rows, extent_cols, x, y) {
                HitTarget::Cell { row, col } => {
                    // SELECT: move the highlight, close any open editor WITHOUT
                    // committing (an implicit revert — nothing was dispatched), and
                    // clear the degrade note. No editor opens on a single click;
                    // double-click / F2 / a printable key enter EDIT (Enter moves down).
                    active_cell.set(Some((row, col)));
                    editing.set(false);
                    g3_note.set(None);
                }
                // A header / corner click is a whole row / column / sheet select —
                // all G3. Surface the honest reason, change NOTHING (no fake band).
                HitTarget::ColumnHeader { .. }
                | HitTarget::RowHeader { .. }
                | HitTarget::Corner => {
                    g3_note.set(Some(selection::ROW_COL_SELECT_DEFERRED_REASON.to_string()));
                }
                // Past the extent: an honest no-op — Excel selects nothing there
                // either, so the active cell and editor are left untouched.
                HitTarget::Outside => {}
            }
            // --- Touch gestures (the canvas owns its viewport) ---
            if ev.pointer_type() == "touch" {
                ev.prevent_default();
                let id = ev.pointer_id();
                let _ = touch_points.try_update_value(|list| {
                    list.retain(|(pid, _)| *pid != id);
                    list.push((id, (x, y)));
                });
                let held = touch_points.try_with_value(|list| list.len()).unwrap_or(0);
                if held == 1 {
                    let _ = pantap.try_update_value(|t| t.pointer_down(performance_now_ms(), x, y));
                    let _ = pinch_start.try_update_value(|p| *p = None);
                    // Keep receiving moves even when the finger leaves the canvas.
                    if let Some(canvas) = canvas_ref.get() {
                        let _ = canvas.set_pointer_capture(id);
                    }
                } else if held == 2 {
                    let span = touch_span(&touch_points);
                    let zoom_now = zoom.get_untracked();
                    let _ = pinch_start.try_update_value(|p| *p = Some((span, zoom_now)));
                    // A pinch absorbs the armed tap: no phantom double-tap.
                    let _ = pantap.try_update_value(PanTapTracker::disarm);
                }
            }
        };

        // Touch moves: one finger PANS (content follows the finger), two
        // fingers PINCH around the pinch's starting zoom. Mouse moves are not
        // a gesture (no drag semantics exist yet) and fall through.
        let on_canvas_pointermove = move |ev: leptos::ev::PointerEvent| {
            if ev.pointer_type() != "touch" {
                return;
            }
            let id = ev.pointer_id();
            let x = f64::from(ev.offset_x());
            let y = f64::from(ev.offset_y());
            let _ = touch_points.try_update_value(|list| {
                if let Some((_, position)) = list.iter_mut().find(|(pid, _)| *pid == id) {
                    *position = (x, y);
                }
            });
            match touch_points.try_with_value(|list| list.len()).unwrap_or(0) {
                1 => {
                    if let Some((dx, dy)) =
                        pantap.try_update_value(|t| t.pointer_move(x, y)).flatten()
                    {
                        apply_pan_delta(dx, dy);
                    }
                }
                2 => {
                    let started = pinch_start.try_with_value(|p| *p).unwrap_or(None);
                    if let Some((start_span, start_zoom)) = started {
                        let scale = pinch_scale(start_span, touch_span(&touch_points));
                        zoom.set(clamp_zoom(start_zoom * scale));
                    }
                }
                _ => {}
            }
        };

        // Touch up: complete the gesture. A lone finishing finger is either a
        // tap pair completion (second tap = EDIT) or nothing; a finger lifting
        // out of a pinch disarms the residue so it can neither pan nor tap.
        let up_workspace = workspace;
        let on_canvas_pointerup = move |ev: leptos::ev::PointerEvent| {
            if ev.pointer_type() != "touch" {
                return;
            }
            let id = ev.pointer_id();
            let _ = touch_points.try_update_value(|list| list.retain(|(pid, _)| *pid != id));
            if touch_points.try_with_value(|list| list.len()).unwrap_or(0) != 0 {
                let _ = pantap.try_update_value(PanTapTracker::disarm);
                let _ = pinch_start.try_update_value(|p| *p = None);
                return;
            }
            let _ = pinch_start.try_update_value(|p| *p = None);
            let is_double = pantap
                .try_update_value(|t| t.pointer_up(performance_now_ms()))
                .unwrap_or(false);
            if !is_double {
                return;
            }
            // Double-tap → EDIT (the touch dblclick), seeded with the cell's
            // EXISTING authored text — the same seed F2 uses.
            let (x, y) = pantap
                .try_with_value(|t| t.tap_position())
                .unwrap_or((0.0, 0.0));
            let metrics = zoomed_metrics(zoom.get_untracked());
            let vp = viewport.get_untracked();
            let Some((grid, extent_rows, extent_cols)) = up_workspace.with_untracked(|ws| {
                active_grid(ws).map(|g| (g.grid_node_id.clone(), g.max_rows, g.max_cols))
            }) else {
                return;
            };
            if let HitTarget::Cell { row, col } =
                hit_test(&metrics, &vp, extent_rows, extent_cols, x, y)
            {
                let seed = up_workspace
                    .with_untracked(|ws| edit::current_authored_seed(ws, &grid, row, col));
                open_editor(row, col, seed);
            }
        };

        // Touch cancel (palm rejection, incoming call): abandon the gesture
        // honestly — no verdict, no half-state left behind.
        let on_canvas_pointercancel = move |ev: leptos::ev::PointerEvent| {
            if ev.pointer_type() != "touch" {
                return;
            }
            let id = ev.pointer_id();
            let _ = touch_points.try_update_value(|list| list.retain(|(pid, _)| *pid != id));
            let _ = pantap.try_update_value(PanTapTracker::disarm);
            let _ = pinch_start.try_update_value(|p| *p = None);
        };

        // Double-click → EDIT (the mouse path into the editor), seeded with the
        // cell's EXISTING authored text — the same seed F2 uses. Shift+double-click
        // has no cell meaning here and is ignored.
        let dblclick_workspace = workspace;
        let on_canvas_dblclick = move |ev: leptos::ev::MouseEvent| {
            if ev.shift_key() {
                return;
            }
            // Same scaled metrics + scrolled viewport the redraw effect drew with,
            // read untracked — so a double-click while scrolled/zoomed opens the
            // editor over the right cell.
            let metrics = zoomed_metrics(zoom.get_untracked());
            let vp = viewport.get_untracked();
            let Some((grid, extent_rows, extent_cols)) = dblclick_workspace.with_untracked(|ws| {
                active_grid(ws).map(|g| (g.grid_node_id.clone(), g.max_rows, g.max_cols))
            }) else {
                return;
            };
            let x = f64::from(ev.offset_x());
            let y = f64::from(ev.offset_y());
            if let HitTarget::Cell { row, col } =
                hit_test(&metrics, &vp, extent_rows, extent_cols, x, y)
            {
                let seed = dblclick_workspace
                    .with_untracked(|ws| edit::current_authored_seed(ws, &grid, row, col));
                open_editor(row, col, seed);
            }
        };

        // The one keydown pipeline (SELECT mode): build a [`selection::KeyChord`],
        // resolve it through the single [`sheet_keymap`] table, and act on the
        // honest availability. While the overlay editor's textarea is focused the
        // bridge owns every key (Enter=commit, Esc=revert, arrows=caret), so the
        // text-entry guard bails first — the select grammar never steals them.
        // dtc-j7n8.26: that guard is the first arm of [`section_key_route`]; the
        // second answers a key that reaches the section itself WHILE EDITING
        // (focus fell out of the editor: a header click, a focus race) by handing
        // focus back to the open editor's textarea — never by running the SELECT
        // grammar over it (which re-seeded the editor with each typed character
        // and turned Enter into Move(Down)). dtc-j7n8.25: a universal shell
        // chord the sheet grammar does not bind (Ctrl+S / Ctrl+O / Ctrl+K / F9)
        // is `ShellOwns` in that state — left untouched so it bubbles to the
        // shell's keydown pipeline instead of dying in the refocus.
        let keymap = sheet_keymap();
        let keydown_workspace = workspace;
        let on_section_keydown = move |ev: leptos::ev::KeyboardEvent| {
            let chord = chord_from_event(&ev);
            match section_key_route(
                event_target_is_text_entry(&ev),
                editing.get_untracked(),
                &chord,
                &keymap,
            ) {
                SectionKeyRoute::EditorOwns | SectionKeyRoute::ShellOwns => return,
                SectionKeyRoute::RefocusEditor => {
                    // Put focus back into the open editor and consume the key. A
                    // read-only note (EDIT on, but no textarea to hand the key to)
                    // falls through to the SELECT grammar so arrows still move
                    // away from it.
                    let refocused = section_ref
                        .get()
                        .is_some_and(|section| focus_editor_textarea(&section));
                    if refocused {
                        ev.prevent_default();
                        ev.stop_propagation();
                        return;
                    }
                }
                SectionKeyRoute::SelectGrammar => {}
            }
            let Some((grid, extent_rows, extent_cols)) = keydown_workspace.with_untracked(|ws| {
                active_grid(ws).map(|g| (g.grid_node_id.clone(), g.max_rows, g.max_cols))
            }) else {
                return; // No grid: let the keystroke bubble to the shell.
            };
            let extent = GridExtent::new(extent_rows, extent_cols);
            if let Some(action) = resolve_action(&keymap, &chord) {
                match action_availability(action) {
                    ActionAvailability::DisabledWithReason(reason) => {
                        // Honest G3 degrade (Shift+Arrow range extend): surface the
                        // reason, consume the chord, mutate NOTHING.
                        ev.prevent_default();
                        ev.stop_propagation();
                        g3_note.set(Some(reason.to_string()));
                    }
                    ActionAvailability::Live => match action {
                        SheetAction::Move(nav) => {
                            ev.prevent_default();
                            ev.stop_propagation();
                            g3_note.set(None);
                            // First nav key with nothing selected lands on the
                            // origin; otherwise step from the current active cell,
                            // edge-clamped by the pure `next_active`.
                            let next = match active_cell.get_untracked() {
                                Some(current) => next_active(current, nav, extent),
                                None => extent.clamp(1, 1),
                            };
                            active_cell.set(Some(next));
                            // dtc-m20s: keep the moved cell on screen (a step past
                            // the visible window scrolls it into view).
                            reveal_active_cell(viewport, zoom, next.0, next.1);
                        }
                        SheetAction::BeginEdit => {
                            // F2 opens the editor at the active cell (or the origin
                            // if nothing is selected yet), seeded with the cell's
                            // existing authored text. (Enter is not BeginEdit — it
                            // moves down via the Move arm above.)
                            ev.prevent_default();
                            ev.stop_propagation();
                            let (row, col) =
                                extent.clamp_pair(active_cell.get_untracked().unwrap_or((1, 1)));
                            let seed = keydown_workspace.with_untracked(|ws| {
                                edit::current_authored_seed(ws, &grid, row, col)
                            });
                            open_editor(row, col, seed);
                        }
                        SheetAction::Cancel => {
                            // Escape in SELECT mode clears the transient note.
                            ev.prevent_default();
                            ev.stop_propagation();
                            g3_note.set(None);
                        }
                        // ExtendSelection is never `Live`; keeps the match total.
                        SheetAction::ExtendSelection => {}
                    },
                }
                return;
            }
            // S3.9 Ctrl+arrow edge-jump — HONEST DEGRADE. The data-aware jump (stop
            // at the next data boundary) needs the G4 model query, which is
            // unbuilt, so we degrade to a WINDOW-LOCAL jump: the active cell lands
            // on the current window's edge ([`window_edge_jump`]) and the transient
            // note announces the degrade (citing G4). Not a keymap row (Ctrl+arrow
            // stays unbound in `sheet_keymap` so the chord is reserved for the real
            // G4 verb); handled here as a wiring fallback ahead of type-to-replace.
            // (Ctrl+Home is a bound `Move(GridStart)` and already returned above.)
            if (ev.ctrl_key() || ev.meta_key())
                && !ev.alt_key()
                && !ev.shift_key()
                && let Some(dir) = edge_dir_from_key(&ev.key())
            {
                ev.prevent_default();
                ev.stop_propagation();
                let current = active_cell.get_untracked().unwrap_or((1, 1));
                let jumped = window_edge_jump(current, dir, extent_rows, extent_cols);
                active_cell.set(Some(jumped));
                // dtc-m20s: an edge-jump lands on the window edge — reveal it (the
                // far edge scrolls into view when the window edge is off-screen).
                reveal_active_cell(viewport, zoom, jumped.0, jumped.1);
                g3_note.set(Some(EDGE_JUMP_DEGRADE_REASON.to_string()));
                return;
            }
            // Type-to-replace: a lone printable key with no Ctrl/Alt/Meta enters
            // EDIT seeding the buffer with THAT character (Excel behavior). An
            // unbound modified chord (Ctrl+C) falls through untouched so universal
            // shell verbs still bubble.
            if !ev.ctrl_key()
                && !ev.alt_key()
                && !ev.meta_key()
                && let Some(ch) = typed_char(&ev.key())
            {
                ev.prevent_default();
                ev.stop_propagation();
                let (row, col) = extent.clamp_pair(active_cell.get_untracked().unwrap_or((1, 1)));
                open_editor(row, col, ch.to_string());
            }
        };

        // The single overlay editor. Renders ONLY when EDITING at the active cell
        // AND the grid resolves; positioned at the active cell's `cell_rect` (the
        // same origin viewport / metrics the canvas drew with, so it sits over the
        // drawn cell). All captures are `Copy` (the `Arc` lives in `dispatch`, a
        // `StoredValue`), so this closure is `Copy` and safe to re-embed when the
        // `has_grid` branch re-runs.
        let overlay_workspace = workspace;
        let cell_overlay = move || {
            // Subscribe to the remount signal so a rejected commit re-applies the
            // underline (a rejection leaves the workspace untouched, so nothing else
            // would remount the bridge). `editing` + `active_cell` drive open/close:
            // SELECT (highlight-only) renders no overlay.
            editor_revision.get();
            if !editing.get() {
                return ().into_any();
            }
            let Some((row, col)) = active_cell.get() else {
                return ().into_any();
            };
            // Resolve the target from host truth (untracked: the click seeded the
            // buffer; a successful commit closes via `selected_cell`, and a
            // rejection re-runs via `editor_revision` — neither needs a workspace
            // subscription here, and reading untracked keeps typing undisturbed).
            let Some(target) =
                overlay_workspace.with_untracked(|ws| resolve_cell_edit_target(ws, row, col))
            else {
                return ().into_any();
            };
            // Reactive position (S3.9 scroll / S3.10 zoom): a closure bound to the
            // `style` attribute, so scrolling or zooming while EDITING repositions
            // the overlay by updating ONLY the style — it never re-runs this outer
            // closure (which reads only `editing`/`active_cell`/`editor_revision`),
            // so the bridge is NOT remounted and typing + focus survive a scroll.
            // Reads the SAME scaled metrics + scrolled viewport the canvas drew
            // with, so the overlay sits over the drawn cell. A cell scrolled out of
            // view gets an off-screen rect and is clipped away by the viewport's
            // `overflow:hidden` — the editor stays MOUNTED (its typed text survives)
            // and reappears when scrolled back, rather than closing under the user.
            //
            // Honor the cell rect as the anchor + minimum. The degrade bridge is a
            // full card (editor box + rejection list), so it cannot fit a 22px cell
            // exactly; anchoring at the cell's top-left with the cell's min-width /
            // min-height lets it grow honestly (Excel widens its editor too) rather
            // than clip to a fake cell-sized box.
            let position = move || {
                let metrics = zoomed_metrics(zoom.get());
                let vp = viewport.get();
                let rect = cell_rect(&metrics, &vp, row, col);
                format!(
                    "left:{}px;top:{}px;min-width:{}px;min-height:{}px",
                    rect.x, rect.y, rect.w, rect.h
                )
            };
            match target {
                CellEditTarget::ReadOnly { reason } => {
                    // A non-editable cell (repeated-region / merged / spill / table
                    // follower): an honest read-only note at the cell, never a fake
                    // editor (SHELL_SPEC §6 honesty).
                    view! {
                        <div
                            class="dna-sheet__cell-editor dna-sheet__cell-editor--readonly"
                            data-testid="sheet-cell-editor"
                            data-editable="false"
                            style=position
                        >
                            <p
                                class="dna-sheet__cell-readonly"
                                data-testid="sheet-cell-readonly"
                            >
                                {reason}
                            </p>
                        </div>
                    }
                    .into_any()
                }
                CellEditTarget::Editable { grid } => {
                    // Remount-gated seed: read the buffer + rejections UNTRACKED so
                    // typing (which updates `edit_text`) never remounts the bridge.
                    let seed_now = edit_text.get_untracked();
                    let rejections_now = editor_rejections.get_untracked();
                    let commit_grid = grid.clone();
                    let commit_dispatch = dispatch.with_value(Arc::clone);
                    // dtc-j7n8.26: the editor OWNS keyboard focus the moment it
                    // mounts. The bridge's `<textarea>` is a child node this stage
                    // holds no ref to, so the wrapper `<div>` is the anchor: an
                    // effect created HERE — inside the overlay's render closure, so
                    // it is disposed with the overlay and re-created on every
                    // remount, including the rejection remount whose fresh textarea
                    // would otherwise drop focus to `<body>` — runs after the DOM is
                    // patched, queries the textarea and focuses it with the caret at
                    // the END: F2 / double-click continue after the existing text,
                    // type-to-replace keeps typing after its seed character. Without
                    // this, focus stayed on the section and every further key
                    // re-ran the SELECT grammar over the open editor.
                    let editor_ref = NodeRef::<leptos::html::Div>::new();
                    Effect::new(move |_| {
                        if let Some(wrapper) = editor_ref.get() {
                            focus_editor_textarea(&wrapper);
                        }
                    });
                    let on_event = Callback::new(move |event: BridgeEvent| match event {
                        // Verbatim text; the host classifies `=`-vs-literal, never
                        // the skin (SHELL_SPEC §6 layering law).
                        BridgeEvent::TextEdited { text, .. } => edit_text.set(text),
                        BridgeEvent::CommitRequested { advance } => {
                            let text = edit_text.get_untracked();
                            let receipt = commit_dispatch.dispatch(enter_cell_intent(
                                commit_grid.clone(),
                                row,
                                col,
                                text,
                            ));
                            match interpret_receipt(&receipt) {
                                CellOutcome::Rejected(diagnostics) => {
                                    // Keep the editor open with the entered text
                                    // intact and underline the typed diagnostics
                                    // (the host guarantees no mutation on this path).
                                    editor_rejections.set(diagnostics);
                                    editor_revision.update(|revision| *revision += 1);
                                }
                                // Accepted (Literal / Formula / Cleared / NoChange):
                                // leave EDIT for SELECT and advance the active cell —
                                // Enter goes one row DOWN, Tab one column RIGHT
                                // (Excel grid commit, dtc-dzky) — clamped to the live
                                // extent. The workbook dispatcher re-projects, so the
                                // canvas repaints the new value AND the moved
                                // highlight automatically through the redraw effect.
                                _ => {
                                    editor_rejections.set(Vec::new());
                                    editing.set(false);
                                    let extent = overlay_workspace.with_untracked(|ws| {
                                        active_grid(ws)
                                            .map(|g| GridExtent::new(g.max_rows, g.max_cols))
                                            .unwrap_or_else(|| GridExtent::new(1, 1))
                                    });
                                    let nav = match advance {
                                        CommitAdvance::Down => NavAction::CommitDown,
                                        CommitAdvance::Right => NavAction::CommitRight,
                                    };
                                    let next = next_active((row, col), nav, extent);
                                    active_cell.set(Some(next));
                                    // dtc-m20s: reveal the post-commit cell (a commit
                                    // at the window edge scrolls the next cell in).
                                    reveal_active_cell(viewport, zoom, next.0, next.1);
                                    // dtc-j7n8.26: the editor is closing — hand focus
                                    // back to the section so the next arrow / F2 /
                                    // typed key lands in the SELECT grammar (the
                                    // focused textarea's removal would otherwise
                                    // drop focus to `<body>`).
                                    focus_section(section_ref);
                                }
                            }
                        }
                        // Esc: exact-revert — leave EDIT for SELECT (the active cell
                        // and its highlight stay put), dispatch NOTHING.
                        BridgeEvent::RevertRequested => {
                            editor_rejections.set(Vec::new());
                            editing.set(false);
                            focus_section(section_ref);
                        }
                        _ => {}
                    });
                    view! {
                        <div
                            class="dna-sheet__cell-editor"
                            data-testid="sheet-cell-editor"
                            data-editable="true"
                            data-cell=format!("{row}:{col}")
                            node_ref=editor_ref
                            style=position
                        >
                            <FormulaBridgeDegrade
                                text=seed_now
                                rejections=rejections_now
                                commit_on_tab=true
                                on_event=on_event
                            />
                        </div>
                    }
                    .into_any()
                }
            }
        };

        let view = view! {
            <style>{SHEET_CSS}</style>
            <section
                class="dna-sheet"
                data-stage="sheet"
                data-testid="sheet-root"
                data-dna-density="working"
                node_ref=section_ref
                tabindex="0"
                on:keydown=on_section_keydown
            >
                {move || {
                    if has_grid.get() {
                        // The real canvas in a POSITIONED viewport wrapper (the
                        // overlay editor's absolute-positioning ancestor), the one
                        // active-cell overlay, and a visually-hidden debug readout
                        // that mirrors the drawn plan (so S3.11 asserts the demo
                        // grid without pixel-reading). Built once per grid-presence
                        // flip; the redraw effect repaints on every projection, and
                        // the overlay reacts to the selected cell.
                        view! {
                            // S3.10 semantic-zoom control: +/-/reset scale the
                            // drawn metrics live (Detail tier), with the live
                            // percentage. `clamp_zoom` holds the factor to
                            // [10%, 400%]; reset returns to 100% (today's geometry).
                            <div
                                class="dna-sheet__toolbar"
                                data-testid="sheet-toolbar"
                                role="group"
                                aria-label="Zoom"
                            >
                                <button
                                    class="dna-sheet__zoom-btn"
                                    type="button"
                                    data-testid="sheet-zoom-out"
                                    aria-label="Zoom out"
                                    title="Zoom out"
                                    on:click=move |_| {
                                        zoom.update(|z| *z = clamp_zoom(*z / viewport::ZOOM_STEP));
                                    }
                                >
                                    "\u{2212}"
                                </button>
                                <span
                                    class="dna-sheet__zoom-level"
                                    data-testid="sheet-zoom-level"
                                    aria-live="polite"
                                >
                                    {move || format!("{}%", (zoom.get() * 100.0).round() as i64)}
                                </span>
                                <button
                                    class="dna-sheet__zoom-btn"
                                    type="button"
                                    data-testid="sheet-zoom-in"
                                    aria-label="Zoom in"
                                    title="Zoom in"
                                    on:click=move |_| {
                                        zoom.update(|z| *z = clamp_zoom(*z * viewport::ZOOM_STEP));
                                    }
                                >
                                    "+"
                                </button>
                                <button
                                    class="dna-sheet__zoom-btn"
                                    type="button"
                                    data-testid="sheet-zoom-reset"
                                    aria-label="Reset zoom to 100%"
                                    title="Reset zoom"
                                    on:click=move |_| zoom.set(1.0)
                                >
                                    "Reset"
                                </button>
                            </div>
                            <div class="dna-sheet__viewport" data-testid="sheet-viewport">
                                <canvas
                                    class="dna-sheet__canvas"
                                    data-testid="sheet-canvas"
                                    node_ref=canvas_ref
                                    on:pointerdown=on_canvas_pointerdown
                                    on:pointermove=on_canvas_pointermove
                                    on:pointerup=on_canvas_pointerup
                                    on:pointercancel=on_canvas_pointercancel
                                    on:dblclick=on_canvas_dblclick
                                    on:wheel=on_canvas_wheel
                                ></canvas>
                                {cell_overlay}
                            </div>
                            // S3.10 zoom degrade: in the Structure/District tiers
                            // the labeled-block / district renderers are NOT built,
                            // so this reactive disabled-with-reason note names the
                            // deferred tier (the Detail grid is still drawn, held at
                            // the legibility floor). Empty (hidden) in the Detail
                            // tier, so 100% renders no note.
                            <p
                                class="dna-sheet__zoom-note"
                                data-testid="sheet-zoom-degrade"
                                aria-live="polite"
                                role="status"
                            >
                                {move || {
                                    tier_degrade_reason(zoom_tier(zoom.get())).unwrap_or_default()
                                }}
                            </p>
                            // S3.9 honest bounded-scale interest note (persistent):
                            // `SetGridInterest` is a workbook no-op today (G4), so
                            // the stage renders the host's full window as-is and
                            // states that windowing / prefetch are deferred to G4 —
                            // no fabricated client-side virtualization.
                            <p
                                class="dna-sheet__interest-note"
                                data-testid="sheet-interest-degrade"
                            >
                                {BOUNDED_SCALE_INTEREST_NOTE}
                            </p>
                            // Honest G3 degrade surface. The reactive note announces
                            // the reason when the user ATTEMPTS a gated GESTURE
                            // (range / row-col / select-all); the static strip stands
                            // in for the gated TOOLS (fill, clipboard) as visible
                            // `disabled` chips citing G3 — no fake handles, zero
                            // behavior. (SHEET_SPEC §4: "affordances render disabled-
                            // with-reason until then (no fake handles)".)
                            <p
                                class="dna-sheet__g3-note"
                                data-testid="sheet-g3-degrade"
                                aria-live="polite"
                                role="status"
                            >
                                {move || g3_note.get().unwrap_or_default()}
                            </p>
                            <div
                                class="dna-sheet__degrades"
                                data-testid="sheet-g3-affordances"
                                aria-label="Disabled until the G3 interaction pack"
                            >
                                <button
                                    class="dna-sheet__degrade-chip"
                                    type="button"
                                    disabled=true
                                    data-testid="sheet-g3-fill"
                                    title=selection::FILL_DEFERRED_REASON
                                >
                                    "Fill"
                                </button>
                                <button
                                    class="dna-sheet__degrade-chip"
                                    type="button"
                                    disabled=true
                                    data-testid="sheet-g3-clipboard"
                                    title=selection::CLIPBOARD_DEFERRED_REASON
                                >
                                    "Copy / Paste"
                                </button>
                                <button
                                    class="dna-sheet__degrade-chip"
                                    type="button"
                                    disabled=true
                                    data-testid="sheet-g3-range"
                                    title=selection::RANGE_SELECT_DEFERRED_REASON
                                >
                                    "Range"
                                </button>
                            </div>
                            <span
                                class="dna-sheet__debug"
                                data-testid="sheet-render-plan"
                                aria-hidden="true"
                                data-cell-count=move || plan_cell_count.get().to_string()
                                data-extent=move || plan_extent.get()
                            >
                                {move || {
                                    format!(
                                        "{} cells \u{00b7} {}",
                                        plan_cell_count.get(),
                                        plan_extent.get(),
                                    )
                                }}
                            </span>
                        }
                        .into_any()
                    } else {
                        // Honest-empty: no grid to draw, never a blank canvas.
                        view! {
                            <p class="dna-sheet__empty" data-testid="sheet-empty">
                                "No grid in this workbook."
                            </p>
                        }
                        .into_any()
                    }
                }}
            </section>
        }
        .into_any();
        StageHandle::new(view)
    }
}

/// The drawn [`GridMetrics`] for a live zoom factor: the default metrics scaled
/// by the zoom, floored at the legibility floor ([`viewport::legible_factor`]) so
/// cell text never shrinks below readable. The ONE place the zoom→metrics mapping
/// lives, so the redraw effect, the click / dblclick hit-test, and the overlay
/// position can never disagree on the drawn cell size. `zoomed_metrics(1.0)` is
/// exactly [`GridMetrics::default`] (the identity `scaled(1.0)`), so default zoom
/// reproduces today's geometry.
#[must_use]
fn zoomed_metrics(zoom: f64) -> GridMetrics {
    GridMetrics::default().scaled(legible_factor(zoom))
}

/// Scroll the shared `viewport` so a just-navigated active cell is fully visible,
/// moving as little as possible (dtc-m20s). Reads the live zoom + the measured
/// viewport (the redraw effect feeds `width`/`height` back into the signal each
/// paint) UNTRACKED, computes the minimal per-axis reveal ([`reveal_scroll`],
/// column→x, row→y), and — only when it actually changes — writes the new scroll
/// back TRACKED so the redraw effect repaints with the highlight on screen.
///
/// Called from the KEYBOARD-nav sites (arrows / Enter-move / Ctrl+arrow edge-jump /
/// Enter-commit advance); mouse clicks deliberately skip it, since a clicked cell
/// is visible by construction. A no-op if the cell is already on screen (so nav
/// within the window never jitters the scroll) and pre-first-paint (size-0
/// viewport → [`reveal_scroll`] leaves the scroll put).
fn reveal_active_cell(viewport: RwSignal<Viewport>, zoom: RwSignal<f64>, row: u32, col: u32) {
    let metrics = zoomed_metrics(zoom.get_untracked());
    let current = viewport.get_untracked();
    let next_x = reveal_scroll(
        col,
        metrics.col_width,
        metrics.header_w,
        current.width,
        current.scroll_x,
    );
    let next_y = reveal_scroll(
        row,
        metrics.row_height,
        metrics.header_h,
        current.height,
        current.scroll_y,
    );
    if next_x != current.scroll_x || next_y != current.scroll_y {
        viewport.update(|v| {
            v.scroll_x = next_x;
            v.scroll_y = next_y;
        });
    }
}

/// Give the overlay editor's `<textarea>` under `root` keyboard focus with the
/// caret at the END of its text (Excel: F2 / a double-click continue after the
/// existing text; a typed character keeps typing after itself). `root` is the
/// overlay wrapper (the mount effect) or the stage section (the keydown refocus
/// route). Returns whether a textarea was found and focused — `false` for a
/// read-only note, which renders no editor. Native builds compile this but never
/// call it (no effects / event handlers under `ssr`).
fn focus_editor_textarea(root: &web_sys::Element) -> bool {
    let Ok(Some(node)) = root.query_selector(".dna-sheet__cell-editor textarea") else {
        return false;
    };
    let Ok(area) = node.dyn_into::<web_sys::HtmlTextAreaElement>() else {
        return false;
    };
    let _ = area.focus();
    // Caret to the end (the browser clamps `u32::MAX` to the text length).
    let _ = area.set_selection_range(u32::MAX, u32::MAX);
    true
}

/// Hand keyboard focus back to the stage's `<section>` (the SELECT grammar's
/// listener) as the overlay editor closes on commit / revert. Without this the
/// focused textarea's removal drops focus to `<body>`, and the next arrow /
/// Enter / F2 reaches neither the grammar nor the shell. A native no-op (refs are
/// unbound under `ssr`).
fn focus_section(section_ref: NodeRef<leptos::html::Section>) {
    if let Some(section) = section_ref.get() {
        let _ = section.focus();
    }
}

/// The active grid to render: the first sheet's backing grid (the active-sheet
/// selection is a sheet-tabs concern, S3 later). `None` when the workspace
/// carries no sheet, or the sheet's grid is not in the current window. A pure
/// function of [`WorkspaceState`], so it is unit-testable against the real demo
/// workbook.
#[must_use]
pub fn active_grid(ws: &WorkspaceState) -> Option<&GridProjection> {
    let sheet = ws.sheets.first()?;
    ws.grids.get(&sheet.grid_node_id)
}

/// The overlay decision for a selected cell, resolved from host truth: either an
/// editor for an editable cell, or an honest read-only note for a non-editable
/// one. Never a third case, and never a fake editor for a read-only cell.
enum CellEditTarget {
    /// The cell admits authoring: build the degrade editor, committing against
    /// this grid at the selected `(row, col)`.
    Editable { grid: NodeId },
    /// The cell is non-editable (a repeated-region / merged / spill / table
    /// follower) — carry the typed reason for the read-only note.
    ReadOnly { reason: &'static str },
}

/// Resolve the overlay decision for the selected `(row, col)` on the active grid.
/// `None` when the workspace carries no grid. A cell whose authored record is
/// non-`Editable` becomes [`CellEditTarget::ReadOnly`] (via
/// [`edit::editability_note`]); an editable cell — or a blank cell with no
/// authored record (Excel authors into it from empty) — becomes
/// [`CellEditTarget::Editable`]. Pure read of [`WorkspaceState`].
fn resolve_cell_edit_target(ws: &WorkspaceState, row: u32, col: u32) -> Option<CellEditTarget> {
    let grid = active_grid(ws)?;
    let grid_id = grid.grid_node_id.clone();
    let editability = grid
        .cells
        .iter()
        .find(|cell| cell.row == row && cell.col == col)
        .and_then(|cell| cell.authored.as_ref())
        .map(|authored| authored.editability.clone());
    match editability.as_ref().and_then(edit::editability_note) {
        Some(reason) => Some(CellEditTarget::ReadOnly { reason }),
        None => Some(CellEditTarget::Editable { grid: grid_id }),
    }
}

/// Translate a raw DOM keydown into the Sheet grammar's [`KeyChord`]: the platform
/// `meta` key folds into `ctrl` (estate convention, matching
/// `dnacalc_shell::keyboard::chord_from_key_event` and the Notebook stage's copy),
/// so a chord is layout-portable.
fn chord_from_event(ev: &leptos::ev::KeyboardEvent) -> KeyChord {
    KeyChord::new(
        ev.key(),
        ev.shift_key(),
        ev.ctrl_key() || ev.meta_key(),
        ev.alt_key(),
    )
}

/// True when the keydown originates from a text-entry element — the guard that
/// keeps the SELECT grammar (arrows, F2, type-to-replace) from firing while the
/// overlay editor's textarea is focused (the bridge owns those keys in EDIT).
/// Mirrors the Notebook stage's private copy (each independent TP stage owns one).
fn event_target_is_text_entry(ev: &leptos::ev::KeyboardEvent) -> bool {
    let Some(target) = ev.target() else {
        return false;
    };
    let Ok(element) = target.dyn_into::<web_sys::Element>() else {
        return false;
    };
    matches!(element.tag_name().as_str(), "INPUT" | "TEXTAREA" | "SELECT")
        || element
            .get_attribute("contenteditable")
            .is_some_and(|value| value != "false")
}

/// The Sheet stage's scoped stylesheet — Strand `--dna-*` tokens only. The
/// canvas fills a POSITIONED viewport wrapper (`.dna-sheet__viewport`) that is the
/// overlay editor's absolute-positioning ancestor; the canvas's CSS box is the
/// viewport size (the redraw effect sizes the device-px backing store off
/// `clientWidth`/`clientHeight`). The overlay editor is absolutely positioned at
/// the selected cell's rect, in the SAME coordinate space the canvas draws with.
/// The debug readout is visually hidden (off-screen, not `display:none`) so it
/// stays queryable by `data-testid`/`data-*` for the S3.11 browser test. The
/// `--g3-note` line is the honest degrade announcer (aria-live) and the
/// `--degrades` strip renders the gated tools as visible `disabled` chips.
pub const SHEET_CSS: &str = "\
.dna-sheet{display:flex;flex-direction:column;gap:var(--dna-gap-3);padding:var(--dna-gap-4);color:var(--dna-ink);height:100%;min-height:0}
.dna-sheet:focus{outline:none}
.dna-sheet:focus-visible{outline:2px solid var(--dna-accent);outline-offset:2px}
.dna-sheet__empty{margin:0;color:var(--dna-ink-3);font-style:italic}
.dna-sheet__toolbar{display:flex;align-items:center;gap:var(--dna-gap-2)}
.dna-sheet__zoom-btn{border:1px solid var(--dna-line);border-radius:var(--dna-radius-chip);background:var(--dna-paper-2);color:var(--dna-ink);min-width:32px;height:28px;padding:var(--dna-gap-1) var(--dna-gap-2);font-size:13px;line-height:1;cursor:pointer}
.dna-sheet__zoom-btn:focus-visible{outline:2px solid var(--dna-accent);outline-offset:1px}
.dna-sheet__zoom-level{min-width:3.5em;text-align:center;color:var(--dna-ink-2);font-size:12px;font-variant-numeric:tabular-nums}
.dna-sheet__zoom-note{margin:0;min-height:1.2em;color:var(--dna-ink-3);font-style:italic;font-size:12px}
.dna-sheet__canvas{position:absolute;inset:0;width:100%;height:100%;display:block;touch-action:none}
.dna-sheet__interest-note{margin:0;color:var(--dna-ink-3);font-size:11px}
.dna-sheet__viewport{position:relative;flex:1 1 auto;min-height:0;overflow:hidden}
.dna-sheet__canvas{position:absolute;inset:0;width:100%;height:100%;display:block}
.dna-sheet__cell-editor{position:absolute;z-index:5;box-sizing:border-box}
.dna-sheet__cell-editor--readonly{background:var(--dna-paper);border:1px solid var(--dna-line);border-radius:var(--dna-radius-chip);padding:var(--dna-gap-2) var(--dna-gap-3)}
.dna-sheet__cell-readonly{margin:0;color:var(--dna-ink-3);font-style:italic;font-size:12px;white-space:nowrap}
.dna-sheet__g3-note{margin:0;min-height:1.2em;color:var(--dna-ink-3);font-style:italic;font-size:12px}
/* Coarse pointers: chrome controls reach touch size (SHELL_SPEC §1.1). */
@media (pointer: coarse){
.dna-sheet__toolbar{gap:var(--dna-gap-3)}
.dna-sheet__zoom-btn{min-width:44px;height:40px;font-size:16px}
}
.dna-sheet__g3-note:empty{visibility:hidden}
.dna-sheet__degrades{display:flex;gap:var(--dna-gap-2);flex-wrap:wrap}
.dna-sheet__degrade-chip{border:1px dashed var(--dna-line);border-radius:var(--dna-radius-chip);background:var(--dna-paper-2);color:var(--dna-ink-3);padding:var(--dna-gap-1) var(--dna-gap-3);font-size:12px}
.dna-sheet__degrade-chip[disabled]{opacity:.55;cursor:not-allowed}
.dna-sheet__debug{position:absolute;width:1px;height:1px;padding:0;margin:-1px;overflow:hidden;clip:rect(0 0 0 0);white-space:nowrap;border:0}
";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sheet_stage_identity() {
        let stage = SheetStage::new();
        assert_eq!(stage.id(), StageId::Sheet);
        assert_eq!(stage.title(), "Sheet");
        assert!(stage.supports(&ProfileTag::ExcelStrict));
    }

    #[test]
    fn active_grid_is_honest_none_for_an_empty_workspace() {
        let ws = WorkspaceState::default();
        assert!(
            active_grid(&ws).is_none(),
            "no grid is fabricated for an empty workspace"
        );
    }

    /// Over the REAL host-core demo workbook (native dev-dep), the active grid
    /// resolves to the first sheet's backing grid and carries a real extent +
    /// windowed cells — proving the stage renders from genuine host truth, not a
    /// hand-mock.
    #[test]
    fn active_grid_resolves_the_real_demo_grid() {
        use dnacalc_host_core::{DocumentSession, build_demo_workbook};

        let session = build_demo_workbook().expect("demo workbook");
        let document = DocumentSession::Workbook(session);
        let ws = document.snapshot();

        let first_sheet = ws.sheets.first().expect("the demo workbook has a sheet");
        let grid = active_grid(&ws).expect("the first sheet's grid resolves");
        assert_eq!(grid.grid_node_id, first_sheet.grid_node_id);
        assert!(
            grid.max_rows > 0 && grid.max_cols > 0,
            "the grid has a real extent"
        );
        assert!(
            !grid.cells.is_empty(),
            "the demo grid windows at least one computed cell"
        );
    }
}
