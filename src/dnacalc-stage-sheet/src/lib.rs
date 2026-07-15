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

use std::sync::Arc;

use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use web_sys::CanvasRenderingContext2d;

use dnacalc_bridge::{BridgeEvent, FormulaBridgeDegrade};
use dnacalc_shell::{ProfileTag, StageContext, StageHandle, StageId, StageSurface};
use dnacalc_skin_ir::intent::Dispatcher;
use dnacalc_skin_ir::{
    GridEntryDiagnosticProjection, GridProjection, NodeId, WorkspaceState,
};

pub mod canvas;
pub mod edit;
pub mod geometry;
pub mod render_plan;
pub mod selection;

pub use canvas::{Palette, draw_active_cell, draw_render_plan, looks_numeric, resolve_palette};
pub use edit::{CellOutcome, enter_cell_intent, interpret_receipt};
pub use geometry::{
    CellRect, GridMetrics, HitTarget, Viewport, cell_rect, hit_test, visible_col_range,
    visible_row_range,
};
pub use render_plan::{
    PlannedCell, PlannedColHeader, PlannedRowHeader, RenderPlan, build_render_plan, col_label,
    value_text,
};
pub use selection::{
    ActionAvailability, GridExtent, KeyBinding, KeyChord, NavAction, SheetAction,
    action_availability, next_active, resolve_action, sheet_keymap, typed_char,
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

            let metrics = GridMetrics::default();
            // Origin viewport: scroll/zoom is S3.9. The demo used-range fits the
            // top-left, which is exactly what the plan windows here.
            let viewport = Viewport {
                scroll_x: 0.0,
                scroll_y: 0.0,
                width: css_w,
                height: css_h,
            };
            let palette = resolve_palette(&el);

            // Build + draw the plan from the SAME workspace read, and publish its
            // honest cell count + extent for the debug readout. Reading the
            // workspace here is what subscribes this effect to re-projections.
            let published = workspace.with(|ws| {
                active_grid(ws).map(|grid| {
                    let plan = build_render_plan(grid, &metrics, &viewport);
                    draw_render_plan(&ctx2d, &plan, &metrics, &viewport, &palette);
                    (plan.cells.len(), plan.extent_rows, plan.extent_cols)
                })
            });
            if let Some((count, rows, cols)) = published {
                plan_cell_count.set(count);
                plan_extent.set(format!("{rows}\u{00d7}{cols}"));
            }

            // S3.8 active-cell highlight — a follow-on pass over the drawn plan.
            // Reading `active_cell` here subscribes the effect to selection, so an
            // arrow-key move or a click repaints the 2px accent box. Drawn only
            // when a grid was actually drawn (`published` is Some) AND a cell is
            // active — the highlight is the ONLY selection chrome (no fake handles).
            if published.is_some()
                && let Some((row, col)) = active_cell.get()
            {
                let rect = cell_rect(&metrics, &viewport, row, col);
                draw_active_cell(&ctx2d, &metrics, &viewport, &rect, &palette);
            }
        });

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

        // Single-click → SELECT (not EDIT). Map the pointer into the SAME origin
        // viewport / metrics the redraw effect draws with, hit-test, and move the
        // active cell — the highlight, not the editor. `offset_x`/`offset_y` are
        // already in the drawn coordinate space (the canvas fills its viewport
        // wrapper with no border/padding). All captures are `Copy`.
        let click_workspace = workspace;
        let on_canvas_mousedown = move |ev: leptos::ev::MouseEvent| {
            // Read the pointer offset FIRST, before any `focus()` side effect: some
            // browsers compute `offsetX`/`offsetY` lazily against the target's
            // CURRENT layout, so focusing the section (which can trigger a reflow /
            // scroll-into-view) before reading them would perturb the hit-test
            // coordinate. Offset is already in the drawn CSS-px space (the canvas
            // fills its viewport wrapper with no border/padding).
            let x = f64::from(ev.offset_x());
            let y = f64::from(ev.offset_y());
            let metrics = GridMetrics::default();
            let viewport = Viewport::default(); // origin (scroll 0): matches drawn pixels
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
            match hit_test(&metrics, &viewport, extent_rows, extent_cols, x, y) {
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
        };

        // Double-click → EDIT (the mouse path into the editor), seeded with the
        // cell's EXISTING authored text — the same seed F2 uses. Shift+double-click
        // has no cell meaning here and is ignored.
        let dblclick_workspace = workspace;
        let on_canvas_dblclick = move |ev: leptos::ev::MouseEvent| {
            if ev.shift_key() {
                return;
            }
            let metrics = GridMetrics::default();
            let viewport = Viewport::default();
            let Some((grid, extent_rows, extent_cols)) = dblclick_workspace.with_untracked(|ws| {
                active_grid(ws).map(|g| (g.grid_node_id.clone(), g.max_rows, g.max_cols))
            }) else {
                return;
            };
            let x = f64::from(ev.offset_x());
            let y = f64::from(ev.offset_y());
            if let HitTarget::Cell { row, col } =
                hit_test(&metrics, &viewport, extent_rows, extent_cols, x, y)
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
        let keymap = sheet_keymap();
        let keydown_workspace = workspace;
        let on_section_keydown = move |ev: leptos::ev::KeyboardEvent| {
            if event_target_is_text_entry(&ev) {
                return;
            }
            let Some((grid, extent_rows, extent_cols)) = keydown_workspace.with_untracked(|ws| {
                active_grid(ws).map(|g| (g.grid_node_id.clone(), g.max_rows, g.max_cols))
            }) else {
                return; // No grid: let the keystroke bubble to the shell.
            };
            let extent = GridExtent::new(extent_rows, extent_cols);
            let chord = chord_from_event(&ev);
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
            // Type-to-replace: a lone printable key with no Ctrl/Alt/Meta enters
            // EDIT seeding the buffer with THAT character (Excel behavior). An
            // unbound modified chord (Ctrl+C, Ctrl+arrow edge-jump) falls through
            // untouched so universal shell verbs still bubble.
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
            let rect = cell_rect(&GridMetrics::default(), &Viewport::default(), row, col);
            // Honor the cell rect as the anchor + minimum. The degrade bridge is a
            // full card (editor box + rejection list), so it cannot fit a 22px cell
            // exactly; anchoring at the cell's top-left with the cell's min-width /
            // min-height lets it grow honestly (Excel widens its editor too) rather
            // than clip to a fake cell-sized box.
            let position = format!(
                "left:{}px;top:{}px;min-width:{}px;min-height:{}px",
                rect.x, rect.y, rect.w, rect.h
            );
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
                    let on_event = Callback::new(move |event: BridgeEvent| match event {
                        // Verbatim text; the host classifies `=`-vs-literal, never
                        // the skin (SHELL_SPEC §6 layering law).
                        BridgeEvent::TextEdited { text, .. } => edit_text.set(text),
                        BridgeEvent::CommitRequested => {
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
                                // leave EDIT for SELECT and advance the active cell
                                // one row down (Excel's Enter-commit), clamped to the
                                // live extent. The workbook dispatcher re-projects,
                                // so the canvas repaints the new value AND the moved
                                // highlight automatically through the redraw effect.
                                _ => {
                                    editor_rejections.set(Vec::new());
                                    editing.set(false);
                                    let extent = overlay_workspace.with_untracked(|ws| {
                                        active_grid(ws)
                                            .map(|g| GridExtent::new(g.max_rows, g.max_cols))
                                            .unwrap_or_else(|| GridExtent::new(1, 1))
                                    });
                                    active_cell.set(Some(next_active(
                                        (row, col),
                                        NavAction::CommitDown,
                                        extent,
                                    )));
                                }
                            }
                        }
                        // Esc: exact-revert — leave EDIT for SELECT (the active cell
                        // and its highlight stay put), dispatch NOTHING.
                        BridgeEvent::RevertRequested => {
                            editor_rejections.set(Vec::new());
                            editing.set(false);
                        }
                        _ => {}
                    });
                    view! {
                        <div
                            class="dna-sheet__cell-editor"
                            data-testid="sheet-cell-editor"
                            data-editable="true"
                            data-cell=format!("{row}:{col}")
                            style=position
                        >
                            <FormulaBridgeDegrade
                                text=seed_now
                                rejections=rejections_now
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
                            <div class="dna-sheet__viewport" data-testid="sheet-viewport">
                                <canvas
                                    class="dna-sheet__canvas"
                                    data-testid="sheet-canvas"
                                    node_ref=canvas_ref
                                    on:mousedown=on_canvas_mousedown
                                    on:dblclick=on_canvas_dblclick
                                ></canvas>
                                {cell_overlay}
                            </div>
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
.dna-sheet__viewport{position:relative;flex:1 1 auto;min-height:0;overflow:hidden}
.dna-sheet__canvas{position:absolute;inset:0;width:100%;height:100%;display:block}
.dna-sheet__cell-editor{position:absolute;z-index:5;box-sizing:border-box}
.dna-sheet__cell-editor--readonly{background:var(--dna-paper);border:1px solid var(--dna-line);border-radius:var(--dna-radius-chip);padding:var(--dna-gap-2) var(--dna-gap-3)}
.dna-sheet__cell-readonly{margin:0;color:var(--dna-ink-3);font-style:italic;font-size:12px;white-space:nowrap}
.dna-sheet__g3-note{margin:0;min-height:1.2em;color:var(--dna-ink-3);font-style:italic;font-size:12px}
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
        assert!(grid.max_rows > 0 && grid.max_cols > 0, "the grid has a real extent");
        assert!(
            !grid.cells.is_empty(),
            "the demo grid windows at least one computed cell"
        );
    }
}
