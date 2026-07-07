//! GRID CANVAS — the shared windowed-grid surface.
//!
//! This is the SheetLens grid machinery promoted into a shared component
//! (route-map §C.2, §E.4 K1a/K1b): a scroll container + virtual canvas sized
//! from `max_rows`/`max_cols`, absolutely-positioned windowed cells, and a
//! read-only overlay layer (tables, merged regions, spills). Only the
//! interest-window cells exist in the DOM; scrollbars are sized by bounds —
//! windowing *is* the virtualization, no virtual-list library.
//!
//! K1a was a **pure extraction** of the pre-extraction `grid_surface`
//! (`sheet.rs:907–1054`) and its two pure helpers, moved unchanged. K1b
//! (§C.2's two mandatory upgrades) adds, on top of that extraction:
//!
//! 1. **Interest coalescing** — [`GridInterestCoalescer`], a pure scheduler
//!    that collapses any number of scroll-driven window recomputations
//!    within one animation frame into a single pending window, flushed by
//!    one `requestAnimationFrame` callback (so a scroll storm dispatches at
//!    most one `SetGridInterest` per frame instead of one per scroll event).
//! 2. **Authored-aware cell rendering** — [`render_grid_cell`] renders the
//!    classifier's read-only affordance when `authored.editability !=
//!    Editable`, and (in show-formulas mode) `authored.source_text` in place
//!    of the computed value.
//!
//! K2 (§C.3) adds the in-grid cell edit loop on top: a single-anchor
//! selection model, the shared [`CellEntryEditor`]/`EntryDiagnostics` (N2)
//! mounted over the selected cell, and the C.3 keyboard model (arrows/Tab
//! move selection, `Enter`/`Shift+Enter` commit-and-move, `F2` toggles edit
//! mode, `Esc` cancels). [`edit_attempt_outcome`] is the table-driven mapping
//! from a cell's [`GridEditabilityProjection`] to what an edit attempt does
//! (edit / jump-to-anchor / hint), reused by both the click handler and the
//! keyboard model so the two entry points never diverge.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use dnatreecalc_skin_framework::{
    CellEntryEditor, Dispatcher, EntryDiagnostics, EntryFeedback, GridAuthoredCellProjection,
    GridAuthoredKindProjection, GridCellProjection, GridCellRefProjection,
    GridEditabilityProjection, GridOverlayRect, NodeId, WorkspaceIntent, WorkspaceState,
};
use leptos::prelude::*;
use leptos::wasm_bindgen::{self, JsCast};

use crate::value_render::render_value;

/// A pure per-frame coalescing scheduler for `SetGridInterest` windows
/// (route-map §C.2 K1b: "scroll events debounce/coalesce into one
/// `SetGridInterest` per animation frame"). Any number of `note` calls
/// between two `drain` calls collapse to the single most recent window; a
/// `drain` with nothing pending returns `None` so the caller dispatches
/// nothing. No DOM/RAF dependency — the frame boundary is owned by the
/// caller (one `drain` per `requestAnimationFrame` tick in the live
/// component), so this is unit-testable as a pure function of calls.
#[derive(Debug, Default)]
pub struct GridInterestCoalescer {
    pending: Option<(u32, u32, u32, u32)>,
}

impl GridInterestCoalescer {
    #[must_use]
    pub fn new() -> Self {
        Self { pending: None }
    }

    /// Record a newly computed interest window, overwriting any window
    /// already pending for this frame (the whole point of coalescing: only
    /// the latest scroll position matters, not the intermediate ones).
    pub fn note(&mut self, window: (u32, u32, u32, u32)) {
        self.pending = Some(window);
    }

    /// Take the pending window, if any, clearing it. Call once per frame
    /// boundary; returns `None` when no `note` happened since the last
    /// `drain` (so the frame dispatches nothing).
    pub fn drain(&mut self) -> Option<(u32, u32, u32, u32)> {
        self.pending.take()
    }
}

/// Fixed grid cell metrics for the windowed grid surface. Per-cell sizing and
/// text layout are Phase 5; the read path renders a uniform cell box.
pub const GRID_ROW_HEIGHT_PX: f64 = 22.0;
pub const GRID_COL_WIDTH_PX: f64 = 84.0;
/// Cap the virtual scroll canvas so a full Excel-extent sheet does not blow past
/// the browser's max element height; the window math still clamps to real bounds.
pub const GRID_VIRTUAL_CELL_CAP: u32 = 100_000;

/// The 1-based inclusive interest window a scroll offset exposes over a uniform
/// grid, clamped to the grid bounds with a one-cell trailing overscan. Pure, so
/// the scroll-to-window mapping is unit-testable without a DOM.
pub fn grid_interest_window(
    scroll_top: f64,
    scroll_left: f64,
    viewport_height: f64,
    viewport_width: f64,
    max_rows: u32,
    max_cols: u32,
) -> (u32, u32, u32, u32) {
    let axis = |offset: f64, viewport: f64, cell: f64, max: u32| -> (u32, u32) {
        let max = max.max(1);
        let first = (offset / cell).floor().max(0.0) as u32 + 1;
        let visible = (viewport / cell).ceil() as u32 + 1;
        let top = first.min(max);
        let bottom = top.saturating_add(visible).min(max).max(top);
        (top, bottom)
    };
    let (top_row, bottom_row) = axis(scroll_top, viewport_height, GRID_ROW_HEIGHT_PX, max_rows);
    let (left_col, right_col) = axis(scroll_left, viewport_width, GRID_COL_WIDTH_PX, max_cols);
    (top_row, left_col, bottom_row, right_col)
}

/// Absolute-position style for an overlay box over a window-clipped rect, in the
/// same coordinate space as the cells. Each edge is drawn `solid` when the rect
/// ends within the window and `clipped` (dashed) when the window cut it, so a
/// clipped border reads as "continues beyond the window".
fn overlay_box_style(rect: &GridOverlayRect, solid: &str, clipped: &str) -> String {
    let top = f64::from(rect.top_row.saturating_sub(1)) * GRID_ROW_HEIGHT_PX;
    let left = f64::from(rect.left_col.saturating_sub(1)) * GRID_COL_WIDTH_PX;
    let width = f64::from(rect.right_col.saturating_sub(rect.left_col) + 1) * GRID_COL_WIDTH_PX;
    let height = f64::from(rect.bottom_row.saturating_sub(rect.top_row) + 1) * GRID_ROW_HEIGHT_PX;
    let border = |is_clipped: bool| if is_clipped { clipped } else { solid };
    format!(
        "position:absolute;top:{top}px;left:{left}px;width:{width}px;height:{height}px;\
         box-sizing:border-box;pointer-events:none;\
         border-top:{};border-left:{};border-bottom:{};border-right:{};",
        border(rect.clipped_top),
        border(rect.clipped_left),
        border(rect.clipped_bottom),
        border(rect.clipped_right),
    )
}

/// The read-only affordance class for a non-`Editable` classification (route-map
/// §C.3's affordance table), or `None` for `Editable` (which renders as a normal
/// cell — no affordance class, no title hint).
fn editability_affordance_class(editability: &GridEditabilityProjection) -> Option<&'static str> {
    match editability {
        GridEditabilityProjection::Editable => None,
        GridEditabilityProjection::SpillDisplay { .. } => Some("dtc-grid__cell--spill-display"),
        GridEditabilityProjection::RepeatedRegionMember { .. } => {
            Some("dtc-grid__cell--repeated-member")
        }
        GridEditabilityProjection::MergedFollower { .. } => Some("dtc-grid__cell--merged-follower"),
        GridEditabilityProjection::TableStructural { .. } => {
            Some("dtc-grid__cell--table-structural")
        }
    }
}

/// The `title` hint for a non-`Editable` classification (§C.3 "On edit attempt"
/// column, surfaced here as a hover affordance since K1b ships no edit loop
/// yet — K2 adds the click-time flash/jump behavior on top of this same
/// classification).
fn editability_hint(editability: &GridEditabilityProjection) -> Option<String> {
    match editability {
        GridEditabilityProjection::Editable => None,
        GridEditabilityProjection::SpillDisplay { anchor } => Some(format!(
            "Spilled from {} — edit the anchor",
            grid_cell_ref_label(anchor.row, anchor.col)
        )),
        GridEditabilityProjection::RepeatedRegionMember { anchor } => Some(format!(
            "Part of a filled region — anchor {}",
            grid_cell_ref_label(anchor.row, anchor.col)
        )),
        GridEditabilityProjection::MergedFollower { anchor } => Some(format!(
            "Part of a merged cell — anchor {}",
            grid_cell_ref_label(anchor.row, anchor.col)
        )),
        GridEditabilityProjection::TableStructural { table_id } => Some(format!(
            "Table header — rename via the table's header row ({table_id})"
        )),
    }
}

/// What an edit attempt (click / `F2` / type-to-edit) does for a given
/// classification (route-map §C.3's "On edit attempt" column) — the
/// table-driven mapping acceptance (1) asserts, one arm per
/// [`GridEditabilityProjection`] variant. `Editable` opens the shared editor;
/// every other variant is read-only and either jumps the selection to its
/// anchor cell (with a status hint) or, for `TableStructural` (no single-cell
/// anchor), surfaces a hint with no selection change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditAttemptOutcome {
    /// Open the shared [`CellEntryEditor`] on this cell.
    Edit,
    /// Read-only: flash this cell and jump the selection to `anchor`,
    /// surfacing `hint` (e.g. "Spilled from B3 — edit the anchor").
    JumpToAnchor {
        anchor: GridCellRefProjection,
        hint: String,
    },
    /// Read-only with no single-cell anchor to jump to (`TableStructural`):
    /// surface `hint` only, selection unchanged.
    Hint { hint: String },
}

/// The table-driven mapping from a cell's editability classification to what
/// an edit attempt does (§C.3 acceptance (1)). Shared by the click handler
/// and the keyboard model (`Enter`/`F2`/type-to-edit) so both entry points
/// agree with the classifier — the skin never guesses past it (§C.3, §A.4).
#[must_use]
pub fn edit_attempt_outcome(editability: &GridEditabilityProjection) -> EditAttemptOutcome {
    match editability {
        GridEditabilityProjection::Editable => EditAttemptOutcome::Edit,
        GridEditabilityProjection::SpillDisplay { anchor } => EditAttemptOutcome::JumpToAnchor {
            anchor: *anchor,
            hint: format!(
                "Spilled from {} — edit the anchor",
                grid_cell_ref_label(anchor.row, anchor.col)
            ),
        },
        GridEditabilityProjection::RepeatedRegionMember { anchor } => {
            EditAttemptOutcome::JumpToAnchor {
                anchor: *anchor,
                hint: "Part of a filled region".to_string(),
            }
        }
        GridEditabilityProjection::MergedFollower { anchor } => EditAttemptOutcome::JumpToAnchor {
            anchor: *anchor,
            hint: "Part of a merged cell".to_string(),
        },
        GridEditabilityProjection::TableStructural { .. } => EditAttemptOutcome::Hint {
            hint: "Table header — rename via the table's header row".to_string(),
        },
    }
}

/// The committed text an authored cell's editor seeds from: a formula's
/// source text, a literal's display text, or empty (an empty/rich-stub cell
/// has no authored text to edit). Mirrors N2's `authored_initial_text`
/// (`notebook.rs`) for the grid's own authored-cell shape.
fn authored_initial_text(authored: &GridAuthoredCellProjection) -> String {
    match authored.kind {
        GridAuthoredKindProjection::Formula => authored.source_text.clone().unwrap_or_default(),
        GridAuthoredKindProjection::Literal => authored.literal_text.clone().unwrap_or_default(),
        GridAuthoredKindProjection::Empty | GridAuthoredKindProjection::RichStub => String::new(),
    }
}

/// A minimal `A1`-style label for a projected `(row, col)` ref, used only for
/// the hover-hint text (never for engine addressing — the projection never
/// carries engine addresses, §A.2).
fn grid_cell_ref_label(row: u32, col: u32) -> String {
    let mut col_label = String::new();
    let mut n = col;
    while n > 0 {
        let rem = (n - 1) % 26;
        col_label.insert(0, (b'A' + rem as u8) as char);
        n = (n - 1) / 26;
    }
    format!("{col_label}{row}")
}

/// K2's in-grid selection + edit state (§C.3): a single anchor cell
/// (`anchor_row`/`anchor_col`, range selection is out of v1 scope per the
/// bead's NON-goals), whether that cell is currently in edit mode, the
/// shared editor's post-commit feedback, and a transient status hint (the
/// jump-to-anchor / table-header message an edit attempt on a non-`Editable`
/// cell surfaces). One `GridSelectionState` is created per `grid_surface`
/// mount and threaded through every cell's click/keyboard handling so they
/// all observe and drive the same anchor.
#[derive(Debug, Clone, Copy)]
pub(crate) struct GridSelectionState {
    pub(crate) anchor_row: RwSignal<u32>,
    pub(crate) anchor_col: RwSignal<u32>,
    editing: RwSignal<bool>,
    feedback: RwSignal<EntryFeedback>,
    status_hint: RwSignal<Option<String>>,
}

impl GridSelectionState {
    pub(crate) fn new() -> Self {
        Self {
            anchor_row: RwSignal::new(1),
            anchor_col: RwSignal::new(1),
            editing: RwSignal::new(false),
            feedback: RwSignal::new(EntryFeedback::None),
            status_hint: RwSignal::new(None),
        }
    }

    /// Move the anchor to `(row, col)` (clamped to the grid bounds), closing
    /// any open editor and clearing stale feedback/hint — the "select a
    /// different cell" reset every navigation chord and every jump-to-anchor
    /// shares.
    fn move_to(&self, row: u32, col: u32, max_rows: u32, max_cols: u32) {
        self.anchor_row.set(row.clamp(1, max_rows.max(1)));
        self.anchor_col.set(col.clamp(1, max_cols.max(1)));
        self.editing.set(false);
        self.feedback.set(EntryFeedback::None);
        self.status_hint.set(None);
    }

    /// Act on an edit attempt at `(row, col)` per the classifier's
    /// [`EditAttemptOutcome`] (§C.3 acceptance (1)/(3)): `Edit` selects the
    /// cell and opens the editor; `JumpToAnchor`/`Hint` select nothing new to
    /// edit, dispatch nothing, and surface the status hint — a jump also
    /// moves the anchor to the resolved anchor cell.
    fn attempt_edit(
        &self,
        row: u32,
        col: u32,
        editability: Option<&GridEditabilityProjection>,
        max_rows: u32,
        max_cols: u32,
    ) {
        let outcome = editability.map_or(EditAttemptOutcome::Edit, edit_attempt_outcome);
        match outcome {
            EditAttemptOutcome::Edit => {
                self.anchor_row.set(row.clamp(1, max_rows.max(1)));
                self.anchor_col.set(col.clamp(1, max_cols.max(1)));
                self.feedback.set(EntryFeedback::None);
                self.status_hint.set(None);
                self.editing.set(true);
            }
            EditAttemptOutcome::JumpToAnchor { anchor, hint } => {
                self.move_to(anchor.row, anchor.col, max_rows, max_cols);
                self.status_hint.set(Some(hint));
            }
            EditAttemptOutcome::Hint { hint } => {
                self.status_hint.set(Some(hint));
            }
        }
    }
}

/// Render one windowed grid cell, authored-aware (route-map §C.2 K1b) and
/// selection/edit-aware (§C.3 K2): a plain `Editable` cell renders its
/// computed value (or, in show-formulas mode, its `authored.source_text`)
/// and, when it is the selected+editing anchor, the shared
/// [`CellEntryEditor`] instead; a non-`Editable` cell additionally carries
/// the classifier's read-only affordance class + hover hint, and an edit
/// attempt on it dispatches nothing (§C.3 acceptance (3)). Cells with no
/// authored projection at all (pre-H3 mirror, or a projection that never
/// carried authored fields) render exactly as K1a did — value-only, no
/// affordance, click selects but never edits — so the upgrade is purely
/// additive.
fn render_grid_cell(
    grid_id: &NodeId,
    cell: &GridCellProjection,
    show_formulas: bool,
    selection: GridSelectionState,
    max_rows: u32,
    max_cols: u32,
    dispatch: Arc<dyn Dispatcher>,
) -> AnyView {
    let top = f64::from(cell.row.saturating_sub(1)) * GRID_ROW_HEIGHT_PX;
    let left = f64::from(cell.col.saturating_sub(1)) * GRID_COL_WIDTH_PX;
    let style = format!(
        "position:absolute;top:{top}px;left:{left}px;width:{GRID_COL_WIDTH_PX}px;height:{GRID_ROW_HEIGHT_PX}px;"
    );

    let authored: Option<&GridAuthoredCellProjection> = cell.authored.as_ref();
    let affordance_class = authored.and_then(|a| editability_affordance_class(&a.editability));
    let hint = authored.and_then(|a| editability_hint(&a.editability));
    let is_selected =
        selection.anchor_row.get() == cell.row && selection.anchor_col.get() == cell.col;
    let is_editing = is_selected && selection.editing.get();
    let mut class = match affordance_class {
        Some(affordance) => format!("dtc-grid__cell {affordance}"),
        None => "dtc-grid__cell".to_string(),
    };
    if is_selected {
        class.push_str(" dtc-grid__cell--selected");
    }

    if is_editing {
        let grid = grid_id.clone();
        let row = cell.row;
        let col = cell.col;
        let initial_text = authored.map(authored_initial_text).unwrap_or_default();

        // §C.3's "Enter commits and moves selection down" / "Shift+Enter
        // commits and moves up": `CellEntryEditor`'s own keydown handler
        // commits on Enter and calls `stop_propagation`, so a bubble-phase
        // listener on this wrapper never sees the key. A capture-phase
        // listener runs before that (capture fires top-down, ahead of the
        // input's bubble-phase handler), so it can observe the Shift flag
        // and record which way to move; an `Effect` watching `editing` then
        // performs the move only once the commit actually closes the editor
        // (a rejection leaves `editing` true, so a rejected commit never
        // moves the selection — the editor stays open with its diagnostics,
        // per §B.3 step 3 reused in K context).
        let pending_move: RwSignal<Option<bool>> = RwSignal::new(None);
        let on_capture_keydown = move |ev: leptos::ev::KeyboardEvent| {
            if ev.key() == "Enter" {
                pending_move.set(Some(ev.shift_key()));
            }
        };
        Effect::new(move |prev: Option<bool>| {
            let now_editing = selection.editing.get();
            if prev == Some(true)
                && !now_editing
                && let Some(shift) = pending_move.get_untracked()
            {
                pending_move.set(None);
                if shift {
                    selection.move_to(row.saturating_sub(1).max(1), col, max_rows, max_cols);
                } else {
                    selection.move_to(row.saturating_add(1), col, max_rows, max_cols);
                }
            }
            now_editing
        });

        return view! {
            <div class=class style=style on:keydown:capture=on_capture_keydown>
                <CellEntryEditor
                    grid=grid
                    row=row
                    col=col
                    initial_text=initial_text
                    dispatch=dispatch
                    editing=selection.editing
                    feedback=selection.feedback
                />
                {move || match selection.feedback.get() {
                    EntryFeedback::None => ().into_any(),
                    EntryFeedback::Rejected { diagnostics } => {
                        view! { <EntryDiagnostics diagnostics=diagnostics /> }.into_any()
                    }
                    EntryFeedback::UnresolvedNames { note } => view! {
                        <div class="dtc-entry-note" role="status">{note}</div>
                    }
                    .into_any(),
                    EntryFeedback::OtherError { message } => view! {
                        <div class="dtc-entry-note dtc-entry-note--error" role="alert">{message}</div>
                    }
                    .into_any(),
                }}
            </div>
        }
        .into_any();
    }

    // Show-formulas mode renders the authored source text in place of the
    // computed value (route-map §C.2 K1b) whenever the cell has one; a
    // literal or empty cell has no `source_text` and still renders its value
    // (there is no "formula" to show).
    let body = if show_formulas {
        match authored.and_then(|a| a.source_text.as_ref()) {
            Some(source_text) => {
                view! { <div class="dtc-value-display dtc-value-display--formula">{source_text.clone()}</div> }
                    .into_any()
            }
            None => render_value(&cell.value),
        }
    } else {
        render_value(&cell.value)
    };

    let row = cell.row;
    let col = cell.col;
    let editability = authored.map(|a| a.editability.clone());
    let on_click = move |_: leptos::ev::MouseEvent| {
        selection.attempt_edit(row, col, editability.as_ref(), max_rows, max_cols);
    };

    match hint {
        Some(hint) => view! {
            <div class=class style=style title=hint on:click=on_click>{body}</div>
        }
        .into_any(),
        None => view! {
            <div class=class style=style on:click=on_click>{body}</div>
        }
        .into_any(),
    }
}

/// A windowed grid surface for a grid-backed sheet node. The scroll container and
/// virtual canvas are built once (stable across projection updates); only the
/// positioned cells re-render reactively, so a `SetGridInterest` re-scope swaps
/// the windowed cells without resetting the scroll position. Scrolling dispatches
/// [`WorkspaceIntent::SetGridInterest`] so OxCalc re-scopes the projection to the
/// newly visible window ("viewing is subscribing").
pub fn grid_surface(
    grid_id: NodeId,
    workspace: ReadSignal<WorkspaceState>,
    dispatch: Arc<dyn Dispatcher>,
    show_formulas: Signal<bool>,
) -> AnyView {
    grid_surface_with_selection(
        GridSelectionState::new(),
        grid_id,
        workspace,
        dispatch,
        show_formulas,
    )
}

/// Like [`grid_surface`], but the caller supplies the [`GridSelectionState`] so
/// it can also read and drive the selected cell — e.g. a formula bar mounted
/// above the grid (K3) that shows the selected cell's source text and commits
/// edits to the same anchor the in-grid editor observes.
pub(crate) fn grid_surface_with_selection(
    selection: GridSelectionState,
    grid_id: NodeId,
    workspace: ReadSignal<WorkspaceState>,
    dispatch: Arc<dyn Dispatcher>,
    show_formulas: Signal<bool>,
) -> AnyView {
    // The sheet extent is fixed, so the canvas size is read once (untracked); the
    // scroll handler clamps windows to it.
    let (max_rows, max_cols) = workspace
        .get_untracked()
        .grids
        .get(&grid_id)
        .map_or((1, 1), |grid| (grid.max_rows, grid.max_cols));
    let canvas_height = f64::from(max_rows.min(GRID_VIRTUAL_CELL_CAP)) * GRID_ROW_HEIGHT_PX;
    let canvas_width = f64::from(max_cols.min(GRID_VIRTUAL_CELL_CAP)) * GRID_COL_WIDTH_PX;

    // Reactive: only the windowed cells re-render when the projection changes; the
    // surrounding scroll box is created once, so scrollTop survives a re-scope.
    let cells_id = grid_id.clone();
    let cells_dispatch = dispatch.clone();
    let cells = move || {
        let ws = workspace.get();
        let Some(grid) = ws.grids.get(&cells_id) else {
            return Vec::new();
        };
        let show = show_formulas.get();
        grid.cells
            .iter()
            .map(|cell| {
                render_grid_cell(
                    &cells_id,
                    cell,
                    show,
                    selection,
                    max_rows,
                    max_cols,
                    cells_dispatch.clone(),
                )
            })
            .collect::<Vec<_>>()
    };

    // Reactive read-only overlay layer (tables, merged regions, spills) drawn over
    // the cells in the same coordinate space. Each box is `pointer-events:none` so
    // it never intercepts cell interaction; a dashed edge marks a window-clipped
    // boundary ("continues beyond the window").
    let overlays_id = grid_id.clone();
    let overlays = move || {
        let ws = workspace.get();
        let Some(grid) = ws.grids.get(&overlays_id) else {
            return Vec::new();
        };
        let mut boxes: Vec<AnyView> = Vec::new();
        for table in &grid.overlays.tables {
            if let Some(header) = &table.header_rect {
                let style = format!(
                    "{}background:rgba(59,125,216,0.12);",
                    overlay_box_style(header, "1px solid #3b7dd8", "1px dashed #3b7dd8")
                );
                boxes.push(
                    view! {
                        <div class="dtc-grid__overlay dtc-grid__overlay--table-header" style=style></div>
                    }
                    .into_any(),
                );
            }
            let style = overlay_box_style(
                &table.table_range,
                "2px solid #3b7dd8",
                "1px dashed #3b7dd8",
            );
            let label = table.table_name.clone();
            boxes.push(
                view! {
                    <div class="dtc-grid__overlay dtc-grid__overlay--table" style=style title=label></div>
                }
                .into_any(),
            );
        }
        for region in &grid.overlays.merged {
            let style = overlay_box_style(&region.rect, "1px solid #9a6b2f", "1px dashed #9a6b2f");
            boxes.push(
                view! { <div class="dtc-grid__overlay dtc-grid__overlay--merged" style=style></div> }
                    .into_any(),
            );
        }
        for spill in &grid.overlays.spills {
            let (style, class) = if spill.blocked {
                (
                    format!(
                        "{}background:rgba(192,57,43,0.10);",
                        overlay_box_style(&spill.extent, "2px solid #c0392b", "1px dashed #c0392b")
                    ),
                    "dtc-grid__overlay dtc-grid__overlay--spill-blocked",
                )
            } else {
                (
                    format!(
                        "{}background:rgba(58,138,58,0.08);",
                        overlay_box_style(
                            &spill.extent,
                            "1px dashed #3a8a3a",
                            "1px dashed #3a8a3a"
                        )
                    ),
                    "dtc-grid__overlay dtc-grid__overlay--spill",
                )
            };
            boxes.push(view! { <div class=class style=style></div> }.into_any());
        }
        boxes
    };

    // Interest coalescing (route-map §C.2 K1b): every scroll tick recomputes
    // the window and hands it to the coalescer, but only the *first* `note`
    // since the last flush schedules a `requestAnimationFrame` callback; any
    // further scroll ticks before that frame fires just overwrite the
    // pending window (`GridInterestCoalescer::note`). The RAF callback
    // drains the coalescer and dispatches at most once per frame, clearing
    // the "scheduled" flag so the next scroll tick schedules a fresh frame.
    let coalescer = Rc::new(RefCell::new(GridInterestCoalescer::new()));
    let frame_scheduled = Rc::new(RefCell::new(false));
    let scroll_id = grid_id.clone();
    // Cloned before `on_scroll` moves the outer `dispatch` — the keyboard
    // handler below needs its own handle to dispatch `ClearGridCell`
    // (§C.3's `Delete` chord).
    let keydown_dispatch = dispatch.clone();
    let on_scroll = move |ev: leptos::ev::Event| {
        let Some(target) = ev.target() else {
            return;
        };
        let Ok(element) = target.dyn_into::<leptos::web_sys::Element>() else {
            return;
        };
        let window = grid_interest_window(
            f64::from(element.scroll_top()),
            f64::from(element.scroll_left()),
            f64::from(element.client_height()),
            f64::from(element.client_width()),
            max_rows,
            max_cols,
        );
        coalescer.borrow_mut().note(window);

        if *frame_scheduled.borrow() {
            return;
        }
        *frame_scheduled.borrow_mut() = true;

        let coalescer = coalescer.clone();
        let frame_scheduled = frame_scheduled.clone();
        let dispatch = dispatch.clone();
        let scroll_id = scroll_id.clone();
        let flush: wasm_bindgen::closure::Closure<dyn FnMut()> =
            wasm_bindgen::closure::Closure::once(move || {
                *frame_scheduled.borrow_mut() = false;
                if let Some((top_row, left_col, bottom_row, right_col)) =
                    coalescer.borrow_mut().drain()
                {
                    dispatch.dispatch(WorkspaceIntent::SetGridInterest {
                        grid: scroll_id.clone(),
                        top_row,
                        left_col,
                        bottom_row,
                        right_col,
                    });
                }
            });
        let Some(window) = leptos::web_sys::window() else {
            return;
        };
        let _ = window.request_animation_frame(flush.as_ref().unchecked_ref());
        // The RAF callback fires at most once (`Closure::once`); leaking it
        // here is the standard wasm-bindgen pattern for a one-shot callback
        // whose owning closure must outlive the JS call that invokes it.
        flush.forget();
    };

    // K2's keyboard model (§C.3 table): arrows/Tab move the anchor, `F2`
    // toggles edit mode, `Enter` (not-yet-editing) opens the editor per the
    // same `attempt_edit` the click handler uses (so a non-Editable cell
    // dispatches nothing and only surfaces the anchor/hint, §C.3 acceptance
    // (3)), `Delete`/`Backspace` dispatches `ClearGridCell` on a selected
    // `Editable` cell (§C.3: "`Delete` on a selected cell → `ClearGridCell`"
    // — gated on `Editable` the same way the click/Enter path is, so a
    // non-Editable cell's Delete is a no-op rather than a raced dispatch),
    // `Esc` clears a stale hint when nothing is open to cancel. "Commit and
    // move down/up" for `Enter`/`Shift+Enter` while already editing is
    // handled in `render_grid_cell`'s capture-phase listener instead:
    // `CellEntryEditor`'s own bubble-phase handler commits on Enter and
    // calls `stop_propagation`, so this container-level handler never
    // observes Enter/Esc while an editor is mounted.
    let keydown_id = grid_id.clone();
    let on_keydown = move |ev: leptos::ev::KeyboardEvent| {
        let ws = workspace.get_untracked();
        let grid = ws.grids.get(&keydown_id);
        let (max_rows, max_cols) = grid.map_or((1, 1), |g| (g.max_rows, g.max_cols));
        let row = selection.anchor_row.get_untracked();
        let col = selection.anchor_col.get_untracked();
        let cell_editability = grid.and_then(|g| {
            g.cells
                .iter()
                .find(|c| c.row == row && c.col == col)
                .and_then(|c| c.authored.as_ref())
                .map(|a| a.editability.clone())
        });

        match ev.key().as_str() {
            "ArrowUp" => {
                ev.prevent_default();
                selection.move_to(row.saturating_sub(1).max(1), col, max_rows, max_cols);
            }
            "ArrowDown" => {
                ev.prevent_default();
                selection.move_to(row.saturating_add(1), col, max_rows, max_cols);
            }
            "ArrowLeft" => {
                ev.prevent_default();
                selection.move_to(row, col.saturating_sub(1).max(1), max_rows, max_cols);
            }
            "ArrowRight" => {
                ev.prevent_default();
                selection.move_to(row, col.saturating_add(1), max_rows, max_cols);
            }
            "Tab" => {
                ev.prevent_default();
                if ev.shift_key() {
                    selection.move_to(row, col.saturating_sub(1).max(1), max_rows, max_cols);
                } else {
                    selection.move_to(row, col.saturating_add(1), max_rows, max_cols);
                }
            }
            "F2" => {
                ev.prevent_default();
                if selection.editing.get_untracked() {
                    selection.editing.set(false);
                } else {
                    selection.attempt_edit(row, col, cell_editability.as_ref(), max_rows, max_cols);
                }
            }
            "Enter" if !selection.editing.get_untracked() => {
                // Not editing yet: Enter opens the editor (or, on a
                // non-Editable cell, dispatches nothing and surfaces the
                // anchor hint) — §C.3 acceptance (3).
                ev.prevent_default();
                selection.attempt_edit(row, col, cell_editability.as_ref(), max_rows, max_cols);
            }
            "Delete" | "Backspace" if !selection.editing.get_untracked() => {
                // §C.3: "Delete on a selected cell -> ClearGridCell". Gated
                // on Editable exactly like the click/Enter edit-attempt path
                // (a non-Editable cell's Delete is a no-op, not a raced
                // dispatch the engine would reject anyway).
                ev.prevent_default();
                if matches!(
                    cell_editability,
                    Some(GridEditabilityProjection::Editable) | None
                ) {
                    keydown_dispatch.dispatch(WorkspaceIntent::ClearGridCell {
                        grid: keydown_id.clone(),
                        row,
                        col,
                    });
                }
            }
            "Escape" if !selection.editing.get_untracked() => {
                // Nothing open to cancel; clear a stale status hint.
                selection.status_hint.set(None);
            }
            _ => {}
        }
    };

    let status_bar = move || {
        selection.status_hint.get().map(|hint| {
            view! {
                <div class="dtc-grid__status-hint" role="status">{hint}</div>
            }
            .into_any()
        })
    };

    view! {
        <div
            class="dtc-grid"
            aria-label="Grid surface"
            tabindex="0"
            on:scroll=on_scroll
            on:keydown=on_keydown
        >
            <div
                class="dtc-grid__canvas"
                style=format!("position:relative;height:{canvas_height}px;width:{canvas_width}px;")
            >
                {cells}
                {overlays}
            </div>
        </div>
        {status_bar}
    }
    .into_any()
}

/// The `.dtc-grid` surface styles, extracted verbatim from `SHEET_CSS` so the
/// shared grid component owns its own paint. Consumers concatenate this into
/// their lens stylesheet.
pub const GRID_CANVAS_CSS: &str = r#"
.dtc-grid {
  position: relative; overflow: auto; height: 320px; margin: 8px 0;
  border: 1px solid var(--dtc-border, #ccc); background: var(--dtc-surface);
}
.dtc-grid__cell {
  box-sizing: border-box; padding: 2px 4px; overflow: hidden;
  white-space: nowrap; text-overflow: ellipsis;
  border-right: 1px solid var(--dtc-border, #eee);
  border-bottom: 1px solid var(--dtc-border, #eee);
  font-variant-numeric: tabular-nums;
}
.dtc-grid__cell--spill-display { background: rgba(58,138,58,0.06); cursor: default; }
.dtc-grid__cell--repeated-member { background: rgba(59,125,216,0.05); cursor: default; }
.dtc-grid__cell--merged-follower { cursor: default; }
.dtc-grid__cell--table-structural { background: rgba(59,125,216,0.10); font-weight: 600; cursor: default; }
.dtc-grid__cell--selected { outline: 2px solid var(--dtc-accent, #1a4fa0); outline-offset: -1px; z-index: 1; }
.dtc-value-display--formula { font-family: var(--dtc-mono-font, monospace); }
.dtc-grid__status-hint {
  margin-top: 4px; padding: 3px 8px; border-radius: 5px; font-size: 12px;
  background: var(--dtc-warning-surface, #fff4e0); color: var(--dtc-warning-text, #8a5a00);
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use dnatreecalc_skin_framework::{
        GridAuthoredKindProjection, GridCellRefProjection, RecordingDispatcher,
    };

    /// A fresh, unselected [`GridSelectionState`] plus a no-op recording
    /// dispatcher, for tests that exercise [`render_grid_cell`] in its
    /// read-only (non-editing) rendering path — K2's selection/edit
    /// machinery is additive to K1b's authored-aware rendering, so these
    /// pre-K2 tests only need a default selection that never matches the
    /// cell under test (anchor defaults to (1, 1)).
    fn test_render_args() -> (NodeId, GridSelectionState, Arc<dyn Dispatcher>) {
        (
            NodeId::new("Sheet1"),
            GridSelectionState::new(),
            Arc::new(RecordingDispatcher::new()),
        )
    }

    #[test]
    fn grid_interest_window_maps_scroll_to_clamped_bounds() {
        // Top-left, a 220x420 viewport over uniform 22x84 cells: ~10 rows and
        // ~5 cols visible, plus a one-cell trailing overscan.
        assert_eq!(
            grid_interest_window(0.0, 0.0, 220.0, 420.0, 1000, 1000),
            (1, 1, 12, 7)
        );
        // Scrolling down one viewport advances the first row to 11.
        let (top, _, _, _) = grid_interest_window(220.0, 0.0, 220.0, 420.0, 1000, 1000);
        assert_eq!(top, 11);
        // The window clamps to a small grid's real bounds.
        assert_eq!(
            grid_interest_window(0.0, 0.0, 220.0, 420.0, 4, 3),
            (1, 1, 4, 3)
        );
    }

    // ---- K1b acceptance (1): coalescing unit test --------------------------
    // N scroll events "in one frame" (i.e. N `note` calls before any `drain`)
    // must collapse to exactly one dispatch — asserted here as "one `drain`
    // returns the latest window, and only the latest".

    #[test]
    fn coalescer_collapses_n_notes_into_one_pending_window() {
        let mut coalescer = GridInterestCoalescer::new();
        // Nothing pending before any scroll tick.
        assert_eq!(coalescer.drain(), None);

        // A storm of scroll-driven window recomputations within one frame.
        for row in 1..=50u32 {
            coalescer.note((row, 1, row + 10, 7));
        }

        // Exactly one dispatch's worth of data: the LATEST window, not the
        // first or an average of the fifty.
        assert_eq!(coalescer.drain(), Some((50, 1, 60, 7)));
        // The frame's pending window is consumed — draining again (as the
        // next animation frame would, with no scroll in between) dispatches
        // nothing, proving this frame's storm produced exactly one dispatch.
        assert_eq!(coalescer.drain(), None);
    }

    #[test]
    fn coalescer_reports_a_fresh_window_after_each_drain() {
        let mut coalescer = GridInterestCoalescer::new();
        coalescer.note((1, 1, 10, 5));
        assert_eq!(coalescer.drain(), Some((1, 1, 10, 5)));

        // A later frame's scroll ticks are independent of the drained one.
        coalescer.note((11, 1, 20, 5));
        coalescer.note((12, 1, 21, 5));
        assert_eq!(coalescer.drain(), Some((12, 1, 21, 5)));
        assert_eq!(coalescer.drain(), None);
    }

    // ---- K1b acceptance (2): authored-aware affordance ---------------------

    fn anchor(row: u32, col: u32) -> GridCellRefProjection {
        GridCellRefProjection { row, col }
    }

    /// A cell whose authored classification is `SpillDisplay { anchor: B3 }`,
    /// showing a spilled value.
    fn spill_display_cell() -> GridCellProjection {
        GridCellProjection {
            row: 4,
            col: 2,
            value: dnatreecalc_skin_framework::NodeValueProjection::Number {
                raw: "30".to_string(),
                display: "30".to_string(),
            },
            value_epoch: 1,
            authored: Some(GridAuthoredCellProjection {
                row: 4,
                col: 2,
                kind: GridAuthoredKindProjection::Empty,
                literal_text: None,
                source_text: None,
                editability: GridEditabilityProjection::SpillDisplay {
                    anchor: anchor(3, 2),
                },
            }),
            provenance: None,
        }
    }

    #[test]
    fn spill_display_cell_renders_read_only_affordance_from_projection() {
        let editability = GridEditabilityProjection::SpillDisplay {
            anchor: anchor(3, 2),
        };
        assert_eq!(
            editability_affordance_class(&editability),
            Some("dtc-grid__cell--spill-display")
        );
        assert_eq!(
            editability_hint(&editability).as_deref(),
            Some("Spilled from B3 — edit the anchor")
        );

        // And through the cell-render fn itself: the rendered markup carries
        // the read-only affordance class, the anchor hint, and still shows
        // the spilled value.
        let (grid_id, selection, dispatch) = test_render_args();
        let html = render_grid_cell(
            &grid_id,
            &spill_display_cell(),
            false,
            selection,
            100,
            10,
            dispatch.clone(),
        )
        .to_html();
        assert!(
            html.contains("dtc-grid__cell--spill-display"),
            "spill-display markup must carry the affordance class: {html}"
        );
        assert!(
            html.contains("Spilled from B3"),
            "spill-display markup must carry the anchor hint: {html}"
        );
        assert!(
            html.contains("30"),
            "spill-display markup still renders the spilled value: {html}"
        );

        // An Editable cell rendered by the same fn has neither class nor hint.
        let editable_html = render_grid_cell(
            &grid_id,
            &formula_cell("=A1*3"),
            false,
            selection,
            100,
            10,
            dispatch,
        )
        .to_html();
        assert!(
            !editable_html.contains("dtc-grid__cell--"),
            "an Editable cell must render no affordance modifier class: {editable_html}"
        );
        assert!(
            !editable_html.contains("title="),
            "an Editable cell must render no hover hint: {editable_html}"
        );
    }

    #[test]
    fn editable_cell_has_no_affordance_or_hint() {
        assert_eq!(
            editability_affordance_class(&GridEditabilityProjection::Editable),
            None
        );
        assert_eq!(editability_hint(&GridEditabilityProjection::Editable), None);
    }

    #[test]
    fn every_non_editable_variant_maps_to_a_distinct_affordance_class() {
        let repeated = GridEditabilityProjection::RepeatedRegionMember {
            anchor: anchor(1, 1),
        };
        let merged = GridEditabilityProjection::MergedFollower {
            anchor: anchor(1, 1),
        };
        let table = GridEditabilityProjection::TableStructural {
            table_id: "tbl:Scenarios".to_string(),
        };

        assert_eq!(
            editability_affordance_class(&repeated),
            Some("dtc-grid__cell--repeated-member")
        );
        assert_eq!(
            editability_affordance_class(&merged),
            Some("dtc-grid__cell--merged-follower")
        );
        assert_eq!(
            editability_affordance_class(&table),
            Some("dtc-grid__cell--table-structural")
        );
        assert!(editability_hint(&table).unwrap().contains("tbl:Scenarios"));
    }

    #[test]
    fn grid_cell_ref_label_formats_a1_style_addresses() {
        assert_eq!(grid_cell_ref_label(3, 2), "B3");
        assert_eq!(grid_cell_ref_label(1, 1), "A1");
        assert_eq!(grid_cell_ref_label(1, 27), "AA1");
    }

    // ---- K1b acceptance (3): show-formulas mode ----------------------------

    fn formula_cell(source_text: &str) -> GridCellProjection {
        GridCellProjection {
            row: 1,
            col: 2,
            value: dnatreecalc_skin_framework::NodeValueProjection::Number {
                raw: "3".to_string(),
                display: "3".to_string(),
            },
            value_epoch: 1,
            authored: Some(GridAuthoredCellProjection {
                row: 1,
                col: 2,
                kind: GridAuthoredKindProjection::Formula,
                literal_text: None,
                source_text: Some(source_text.to_string()),
                editability: GridEditabilityProjection::Editable,
            }),
            provenance: None,
        }
    }

    #[test]
    fn show_formulas_mode_renders_source_text_instead_of_the_value() {
        let cell = formula_cell("=A1*3");
        let (grid_id, selection, dispatch) = test_render_args();

        // Mode off: the computed value renders, never the source text.
        let value_html =
            render_grid_cell(&grid_id, &cell, false, selection, 100, 10, dispatch.clone())
                .to_html();
        assert!(
            value_html.contains('3') && !value_html.contains("=A1*3"),
            "with show-formulas off the computed value must render: {value_html}"
        );

        // Mode on: the authored source text renders in place of the value.
        let formula_html =
            render_grid_cell(&grid_id, &cell, true, selection, 100, 10, dispatch).to_html();
        assert!(
            formula_html.contains("=A1*3"),
            "with show-formulas on the authored source text must render: {formula_html}"
        );
        assert!(
            formula_html.contains("dtc-value-display--formula"),
            "the source text renders in the formula-text style: {formula_html}"
        );
    }

    #[test]
    fn show_formulas_mode_keeps_the_value_for_a_literal_cell() {
        let cell = GridCellProjection {
            row: 1,
            col: 1,
            value: dnatreecalc_skin_framework::NodeValueProjection::Number {
                raw: "7".to_string(),
                display: "7".to_string(),
            },
            value_epoch: 1,
            authored: Some(GridAuthoredCellProjection {
                row: 1,
                col: 1,
                kind: GridAuthoredKindProjection::Literal,
                literal_text: Some("7".to_string()),
                source_text: None,
                editability: GridEditabilityProjection::Editable,
            }),
            provenance: None,
        };
        assert!(cell.authored.as_ref().unwrap().source_text.is_none());

        // A literal has no formula to show — show-formulas mode still renders
        // its value (never a blank).
        let (grid_id, selection, dispatch) = test_render_args();
        let html = render_grid_cell(&grid_id, &cell, true, selection, 100, 10, dispatch).to_html();
        assert!(
            html.contains('7'),
            "a literal cell renders its value even in show-formulas mode: {html}"
        );
    }

    #[test]
    fn cell_without_authored_metadata_renders_value_only_as_before_k1b() {
        // Pre-H3 mirror payloads carry no authored fields; the K1b upgrades
        // must be purely additive for them.
        let cell = GridCellProjection {
            row: 2,
            col: 3,
            value: dnatreecalc_skin_framework::NodeValueProjection::Number {
                raw: "42".to_string(),
                display: "42".to_string(),
            },
            value_epoch: 1,
            authored: None,
            provenance: None,
        };
        let (grid_id, selection, dispatch) = test_render_args();
        let html = render_grid_cell(&grid_id, &cell, true, selection, 100, 10, dispatch).to_html();
        assert!(html.contains("42"), "value renders: {html}");
        assert!(
            !html.contains("dtc-grid__cell--") && !html.contains("title="),
            "no affordance or hint without authored metadata: {html}"
        );
    }

    // ---- K2 acceptance (1): editability -> affordance mapping (table-driven) ----

    #[test]
    fn edit_attempt_outcome_maps_editable_to_edit() {
        assert_eq!(
            edit_attempt_outcome(&GridEditabilityProjection::Editable),
            EditAttemptOutcome::Edit
        );
    }

    #[test]
    fn edit_attempt_outcome_maps_spill_display_to_jump_with_anchor_hint() {
        let outcome = edit_attempt_outcome(&GridEditabilityProjection::SpillDisplay {
            anchor: anchor(3, 2),
        });
        assert_eq!(
            outcome,
            EditAttemptOutcome::JumpToAnchor {
                anchor: anchor(3, 2),
                hint: "Spilled from B3 — edit the anchor".to_string(),
            }
        );
    }

    #[test]
    fn edit_attempt_outcome_maps_repeated_region_member_to_jump() {
        let outcome = edit_attempt_outcome(&GridEditabilityProjection::RepeatedRegionMember {
            anchor: anchor(2, 1),
        });
        assert_eq!(
            outcome,
            EditAttemptOutcome::JumpToAnchor {
                anchor: anchor(2, 1),
                hint: "Part of a filled region".to_string(),
            }
        );
    }

    #[test]
    fn edit_attempt_outcome_maps_merged_follower_to_jump() {
        let outcome = edit_attempt_outcome(&GridEditabilityProjection::MergedFollower {
            anchor: anchor(5, 5),
        });
        assert_eq!(
            outcome,
            EditAttemptOutcome::JumpToAnchor {
                anchor: anchor(5, 5),
                hint: "Part of a merged cell".to_string(),
            }
        );
    }

    #[test]
    fn edit_attempt_outcome_maps_table_structural_to_hint_with_no_anchor() {
        let outcome = edit_attempt_outcome(&GridEditabilityProjection::TableStructural {
            table_id: "tbl:Scenarios".to_string(),
        });
        assert_eq!(
            outcome,
            EditAttemptOutcome::Hint {
                hint: "Table header — rename via the table's header row".to_string(),
            }
        );
    }

    // ---- K2 acceptance (3): SpillDisplay edit attempt dispatches nothing,
    // ---- focuses the anchor ------------------------------------------------

    #[test]
    fn attempt_edit_on_spill_display_dispatches_nothing_and_focuses_anchor() {
        let selection = GridSelectionState::new();
        // Start selected somewhere else so a jump is observable.
        selection.move_to(4, 2, 100, 10);
        assert_eq!(selection.anchor_row.get_untracked(), 4);
        assert_eq!(selection.anchor_col.get_untracked(), 2);

        let editability = GridEditabilityProjection::SpillDisplay {
            anchor: anchor(3, 2),
        };
        selection.attempt_edit(4, 2, Some(&editability), 100, 10);

        // The selection jumped to the anchor cell, not the spill cell.
        assert_eq!(selection.anchor_row.get_untracked(), 3);
        assert_eq!(selection.anchor_col.get_untracked(), 2);
        // No edit was opened — a non-Editable cell never mounts the editor.
        assert!(!selection.editing.get_untracked());
        // The status hint names the anchor, proving the "focuses the anchor"
        // half of the acceptance, not just a silent no-op.
        assert_eq!(
            selection.status_hint.get_untracked().as_deref(),
            Some("Spilled from B3 — edit the anchor")
        );
    }

    #[test]
    fn attempt_edit_on_table_structural_surfaces_hint_without_moving_selection() {
        let selection = GridSelectionState::new();
        selection.move_to(3, 1, 100, 10);

        let editability = GridEditabilityProjection::TableStructural {
            table_id: "tbl:Scenarios".to_string(),
        };
        selection.attempt_edit(3, 1, Some(&editability), 100, 10);

        // No anchor to jump to: the selection stays put.
        assert_eq!(selection.anchor_row.get_untracked(), 3);
        assert_eq!(selection.anchor_col.get_untracked(), 1);
        assert!(!selection.editing.get_untracked());
        assert!(
            selection
                .status_hint
                .get_untracked()
                .unwrap()
                .contains("Table header")
        );
    }

    #[test]
    fn attempt_edit_on_editable_cell_opens_the_editor() {
        let selection = GridSelectionState::new();
        selection.attempt_edit(2, 3, Some(&GridEditabilityProjection::Editable), 100, 10);
        assert_eq!(selection.anchor_row.get_untracked(), 2);
        assert_eq!(selection.anchor_col.get_untracked(), 3);
        assert!(selection.editing.get_untracked());
        assert_eq!(selection.status_hint.get_untracked(), None);
    }
}
