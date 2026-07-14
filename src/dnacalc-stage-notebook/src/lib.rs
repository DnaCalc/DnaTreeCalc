//! TIER: TP (Presentation) — T0 skin-ir + Leptos + TP crates only; no Ox* ever (P-gate).
//!
//! The Notebook stage (S2.5 render + S2.6 edit): a [`dnacalc_shell::StageSurface`]
//! registered under [`dnacalc_shell::StageId::Notebook`]. `mount` renders the
//! reactive single-column block list (NOTEBOOK_SPEC §3) from host truth: a
//! closure over `ctx.workspace` re-derives [`model::derive_entries`] on every
//! workspace change and paints one block per entry.
//!
//! Each authored **Cell** block is *editable* (S2.6): it embeds a degrade
//! [`FormulaBridgeDegrade`] editor seeded from the cell's own authored text and,
//! on commit, dispatches `WorkspaceIntent::EnterGridCell` built from the block's
//! OWN `grid`/`row`/`col` (via [`edit::enter_cell_intent`]) through
//! `ctx.dispatch`. The host's three-way outcome is read back with
//! [`edit::interpret_receipt`] and rendered as an honest per-block chip. The
//! skin never classifies `=`-vs-literal itself (SHELL_SPEC §6 layering law) and
//! never refreshes the workspace itself — the workbook dispatcher re-projects
//! into `ctx.workspace`, so the reactive closure repaints the block's value
//! automatically after a commit.
//!
//! It is never rendered blank: an empty derivation renders an explicit, testable
//! honest-empty card.

use std::collections::HashMap;
use std::sync::Arc;

use leptos::prelude::*;

use dnacalc_bridge::{BridgeEvent, FormulaBridgeDegrade};
use dnacalc_shell::{ProfileTag, StageContext, StageHandle, StageId, StageSurface};
use dnacalc_skin_ir::intent::Dispatcher;
use dnacalc_skin_ir::{
    DefinedNameProjection, DefinedNameTargetProjection, GridAuthoredCellProjection,
    GridCellProjection, GridEditabilityProjection, GridEntryDiagnosticProjection,
    GridTableOverlayDescriptor, NodeClassification, NodeId, NodeValueProjection, WorkspaceState,
};

pub mod edit;
pub mod model;

pub use edit::{CellOutcome, enter_cell_intent, interpret_receipt};
pub use model::{NotebookEntry, NotebookEntryKind, derive_entries};

/// Stable per-block identity: a Cell block's grid plus its 1-based address. The
/// per-block editor state map ([`BlockEditState`]) is keyed on this so a block's
/// in-progress text, rejections, and outcome are anchored to its cell — they can
/// neither cross-contaminate another block nor be reset when the reactive
/// closure re-derives the whole list after some *other* cell's commit.
type BlockKey = (NodeId, u32, u32);

/// One editable Cell block's live editor state. Every field is a `Copy`
/// [`RwSignal`], so [`BlockEditState`] is `Copy` and cheaply captured into a
/// block's event callback and its outcome-chip closure.
///
/// PER-BLOCK STATE / no cross-contamination: these signals live in a map keyed
/// by [`BlockKey`] (see [`block_edit_state`]), created ONCE per block under the
/// stable *mount* owner — never inside the re-running derivation closure. That
/// is the crux:
///  - **No bleed between blocks** — each block resolves its own distinct
///    `BlockEditState` by its own address; a callback captures only its block's
///    signals, and a commit builds its intent from its own `grid`/`row`/`col`.
///  - **In-progress text survives re-derivation** — when any block commits, the
///    workspace re-projects and the derivation closure re-runs, remounting every
///    block's editor. Each editor re-seeds from *its own* `edit_text` signal
///    (updated on every keystroke, read untracked), so a sibling block's
///    half-typed formula is preserved rather than reset to its authored text.
///  - **Signals outlive the re-render** — created under the mount owner
///    ([`Owner`]), they are disposed only when the stage unmounts, not when the
///    inner closure re-runs (which would dispose signals parented to it).
#[derive(Clone, Copy)]
struct BlockEditState {
    /// The live edit-buffer text; updated on every `TextEdited` and read
    /// untracked to re-seed the editor on each remount.
    edit_text: RwSignal<String>,
    /// Typed rejections from this block's last commit attempt; drives the
    /// degrade underline, cleared on a successful commit.
    rejections: RwSignal<Vec<GridEntryDiagnosticProjection>>,
    /// This block's last committed outcome; drives its outcome chip. `None`
    /// until the block has been committed at least once.
    outcome: RwSignal<Option<CellOutcome>>,
    /// Bumped on every commit/revert to remount THIS block's degrade editor
    /// (re-seeding from `edit_text`, re-applying `rejections`). The bridge reads
    /// its `rejections` prop only at mount, and a *rejected* commit leaves the
    /// workspace signal untouched (the host publishes an unchanged delta), so
    /// without this the rejection underline would never appear — mirrors the
    /// app's `CalcBridgeSurface` revision discipline, keyed per block.
    revision: RwSignal<usize>,
}

impl BlockEditState {
    /// Fresh state seeded from the cell's authored text (a formula's source
    /// text, else a literal's text, else empty).
    fn new(seed: &str) -> Self {
        Self {
            edit_text: RwSignal::new(seed.to_string()),
            rejections: RwSignal::new(Vec::new()),
            outcome: RwSignal::new(None),
            revision: RwSignal::new(0),
        }
    }
}

/// The Notebook stage surface.
#[derive(Debug, Clone, Copy, Default)]
pub struct NotebookStage;

impl NotebookStage {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl StageSurface for NotebookStage {
    fn id(&self) -> StageId {
        StageId::Notebook
    }

    fn title(&self) -> &'static str {
        "Notebook"
    }

    fn supports(&self, profile: &ProfileTag) -> bool {
        matches!(profile, ProfileTag::ExcelStrict)
    }

    fn mount(&self, ctx: StageContext) -> StageHandle {
        // The one host-truth read this stage makes. `ReadSignal` is `Copy`, so
        // the closure below can re-run `derive_entries` on every workspace
        // change without holding any owned state — the block list is a *view*
        // of the workspace, never a copy that can drift (NOTEBOOK_SPEC §1).
        let workspace = ctx.workspace;
        // The one dispatcher a Cell block commits through (SHELL_SPEC §6). The
        // workbook dispatcher owns the app's workspace signal and re-projects on
        // every dispatch, so a committed edit repaints via the closure below —
        // this stage wires no workspace refresh of its own.
        let dispatch = ctx.dispatch.clone();
        // Per-block editor state, keyed by stable block identity. Lives OUTSIDE
        // the reactive derivation closure so a workspace change (which re-derives
        // and remounts every block) cannot reset a block's in-progress text,
        // rejections, or last outcome. See [`BlockEditState`] for the full
        // no-cross-contamination / survives-re-derivation argument.
        let editors: StoredValue<HashMap<BlockKey, BlockEditState>> =
            StoredValue::new(HashMap::new());
        // The stable owner for lazily-created per-block signals: the mount owner
        // outlives every re-derivation (only the inner closure re-runs). Creating
        // a block's signals under it — not under the transient render effect —
        // is what keeps them alive across re-renders instead of being disposed on
        // the next workspace change. `None` only off a reactive runtime (e.g. a
        // bare SSR render), where re-derivation cannot happen anyway.
        let owner = Owner::current();
        let view = view! {
            <style>{NOTEBOOK_CSS}</style>
            <section
                class="dna-notebook"
                data-stage="notebook"
                data-testid="notebook-root"
                data-dna-density="reading"
            >
                {move || {
                    let entries = workspace.with(model::derive_entries);
                    if entries.is_empty() {
                        // Never blank: the honest-empty card is an explicit,
                        // testable state.
                        view! {
                            <p class="dna-notebook__empty" data-testid="notebook-empty">
                                "No notebook content yet."
                            </p>
                        }
                        .into_any()
                    } else {
                        entries
                            .into_iter()
                            .map(|entry| {
                                render_block(entry, &dispatch, workspace, editors, owner.as_ref())
                            })
                            .collect_view()
                            .into_any()
                    }
                }}
            </section>
        }
        .into_any();
        StageHandle::new(view)
    }
}

/// Resolve (creating on first sight) the [`BlockEditState`] for a Cell block.
/// The signals are created under `owner` — the stable mount owner — so they
/// persist across the derivation closure's re-runs; storing them in `editors`
/// (a non-reactive [`StoredValue`] map) anchors them to the block's address so
/// no two blocks ever share edit state.
fn block_edit_state(
    editors: StoredValue<HashMap<BlockKey, BlockEditState>>,
    owner: Option<&Owner>,
    key: &BlockKey,
    seed: &str,
) -> BlockEditState {
    if let Some(state) = editors.with_value(|map| map.get(key).copied()) {
        return state;
    }
    let state = match owner {
        Some(owner) => owner.with(|| BlockEditState::new(seed)),
        None => BlockEditState::new(seed),
    };
    editors.update_value(|map| {
        map.insert(key.clone(), state);
    });
    state
}

/// Render one derived entry as a single block (NOTEBOOK_SPEC §3 block anatomy):
/// a gutter carrying a kind glyph + a structural classification tint, a name row
/// (`display_name` · classification chip · liveness dot), a result region
/// showing the live computed value, and — for an editable Cell — a degrade
/// editor plus its honest outcome chip.
fn render_block(
    entry: NotebookEntry,
    dispatch: &Arc<dyn Dispatcher>,
    workspace: ReadSignal<WorkspaceState>,
    editors: StoredValue<HashMap<BlockKey, BlockEditState>>,
    owner: Option<&Owner>,
) -> AnyView {
    let classification = entry.classification;
    let class_id = classification.stable_id();
    let chip_label = classification_label(classification);
    let liveness = liveness_id(classification);
    let kind_id = entry_kind_id(&entry.kind);
    let glyph = entry_kind_glyph(&entry.kind);
    // The classification tint is a *structural* modifier class (per the style
    // law: it encodes the classification, it is not decorative color) — the
    // color itself resolves through Strand's soft-token palette in NOTEBOOK_CSS.
    let gutter_class = format!("dna-notebook__gutter dna-notebook__gutter--{class_id}");
    let display_name = entry.display_name.clone();
    let value_view = match &entry.kind {
        NotebookEntryKind::Cell { value, .. } => render_value(value),
        NotebookEntryKind::Name { name, backing_cell } => render_name_value(name, backing_cell),
        NotebookEntryKind::Table { table, .. } => render_table_value(table),
    };
    // The interactive region: an editor + outcome for an editable Cell, an
    // honest read-only note otherwise. Names/Tables are read-only here (a Name
    // is edited through its backing cell, not fabricated a target — SHELL_SPEC
    // §6 honesty).
    let edit_view = match &entry.kind {
        NotebookEntryKind::Cell {
            grid, authored, ..
        } => render_cell_editor(grid, authored, dispatch, workspace, editors, owner),
        NotebookEntryKind::Name { .. } => read_only_note("defined name — edit its backing cell"),
        NotebookEntryKind::Table { .. } => ().into_any(),
    };

    view! {
        <article
            class="dna-notebook__block"
            data-block-kind=kind_id
            data-classification=class_id
            data-testid="notebook-block"
        >
            <div class=gutter_class aria-hidden="true">
                <span class="dna-notebook__glyph">{glyph}</span>
            </div>
            <div class="dna-notebook__body">
                <div class="dna-notebook__name-row">
                    <span class="dna-notebook__name" data-testid="notebook-block-name">
                        {display_name}
                    </span>
                    <span class="dna-notebook__chip" data-testid="notebook-classification">
                        {chip_label}
                    </span>
                    <span class="dna-notebook__dot" data-liveness=liveness aria-hidden="true"></span>
                </div>
                {value_view}
                {edit_view}
            </div>
        </article>
    }
    .into_any()
}

/// The editable region of a Cell block: an editable cell gets a degrade editor
/// (seeded from its own authored text) plus its honest outcome chip; a
/// non-editable cell gets a read-only note naming why (never a fake editor).
fn render_cell_editor(
    grid: &NodeId,
    authored: &GridAuthoredCellProjection,
    dispatch: &Arc<dyn Dispatcher>,
    workspace: ReadSignal<WorkspaceState>,
    editors: StoredValue<HashMap<BlockKey, BlockEditState>>,
    owner: Option<&Owner>,
) -> AnyView {
    // Respect editability honestly: a non-`Editable` cell is read-only.
    if let Some(reason) = editability_note(&authored.editability) {
        return read_only_note(reason);
    }

    let row = authored.row;
    let col = authored.col;
    let key: BlockKey = (grid.clone(), row, col);
    let seed = authored_seed_text(authored);
    // This block's own edit state — resolved by its own address, so it can never
    // share signals with (or be reset by) another block.
    let state = block_edit_state(editors, owner, &key, &seed);

    // The commit path: build the entry intent from THIS block's own address and
    // dispatch it; read the host's three-way outcome back. The bridge emits only
    // semantic events — the STAGE constructs the intent (layering law). No
    // workspace refresh here: the workbook dispatcher re-projects, so the value
    // region above repaints through the reactive closure.
    let commit_grid = grid.clone();
    let commit_dispatch = Arc::clone(dispatch);
    let on_event = Callback::new(move |event: BridgeEvent| match event {
        // Verbatim text; the host classifies it — the skin never inspects `=`.
        BridgeEvent::TextEdited { text, .. } => state.edit_text.set(text),
        BridgeEvent::CommitRequested => {
            let text = state.edit_text.get_untracked();
            let receipt = commit_dispatch.dispatch(enter_cell_intent(
                commit_grid.clone(),
                row,
                col,
                text,
            ));
            let resolved = interpret_receipt(&receipt);
            if let CellOutcome::Rejected(diagnostics) = &resolved {
                // Keep the rejected text intact so the user can fix it; underline
                // the typed diagnostics.
                state.rejections.set(diagnostics.clone());
            } else {
                state.rejections.set(Vec::new());
            }
            state.outcome.set(Some(resolved));
            // Remount the editor to re-seed committed text / apply rejections,
            // independent of whether the workspace fired (a rejection does not).
            state.revision.update(|revision| *revision += 1);
        }
        BridgeEvent::RevertRequested => {
            // Esc: drop the in-progress edit back to the cell's current authored
            // text (read live from host truth), and clear the transient outcome.
            let authored_now = workspace
                .with_untracked(|ws| current_authored_seed(ws, &commit_grid, row, col));
            state.edit_text.set(authored_now);
            state.rejections.set(Vec::new());
            state.outcome.set(None);
            state.revision.update(|revision| *revision += 1);
        }
        _ => {}
    });

    view! {
        <div class="dna-notebook__edit" data-testid="notebook-block-edit">
            {move || {
                // Remount on a commit/revert (`revision`) — never per keystroke —
                // re-seeding from `edit_text` and re-applying `rejections`, both
                // read untracked so typing does not remount. Same discipline the
                // app's `CalcBridgeSurface` uses, keyed here to THIS block.
                state.revision.get();
                let seed_now = state.edit_text.get_untracked();
                let rejections_now = state.rejections.get_untracked();
                view! {
                    <FormulaBridgeDegrade
                        text=seed_now
                        rejections=rejections_now
                        on_event=on_event
                    />
                }
            }}
            {move || {
                state
                    .outcome
                    .get()
                    .map(|current| {
                        let label = current.label();
                        let detail = outcome_detail(&current);
                        view! {
                            <div
                                class="dna-notebook__outcome"
                                data-testid="notebook-block-outcome"
                                data-outcome=label
                            >
                                <span class="dna-notebook__outcome-label">{label}</span>
                                <span class="dna-notebook__outcome-detail">{detail}</span>
                            </div>
                        }
                    })
            }}
        </div>
    }
    .into_any()
}

/// An honest, testable read-only note — a cell/name the Notebook deliberately
/// does not offer an editor for, with the reason stated (never a fake editor).
fn read_only_note(reason: &'static str) -> AnyView {
    view! {
        <p class="dna-notebook__readonly" data-testid="notebook-block-readonly">
            {reason}
        </p>
    }
    .into_any()
}

/// The reason a cell is not directly editable, or `None` when it is `Editable`.
/// A skin renders the non-`Editable` variants read-only (the entry verb would
/// reject a write to them anyway, H6).
fn editability_note(editability: &GridEditabilityProjection) -> Option<&'static str> {
    match editability {
        GridEditabilityProjection::Editable => None,
        GridEditabilityProjection::RepeatedRegionMember { .. } => {
            Some("read-only — part of a repeated-formula region")
        }
        GridEditabilityProjection::MergedFollower { .. } => {
            Some("read-only — follower of a merged region")
        }
        GridEditabilityProjection::SpillDisplay { .. } => {
            Some("read-only — spilled from an array formula")
        }
        GridEditabilityProjection::TableStructural { .. } => {
            Some("read-only — structural table cell")
        }
    }
}

/// The editor seed for a cell: its formula source text, else its literal text,
/// else empty — never a computed value.
fn authored_seed_text(authored: &GridAuthoredCellProjection) -> String {
    authored
        .source_text
        .clone()
        .or_else(|| authored.literal_text.clone())
        .unwrap_or_default()
}

/// The current authored seed text for `(grid, row, col)` read live from the
/// workspace projection (used by Esc/revert). Empty when the cell has no
/// authored record in the current window.
fn current_authored_seed(ws: &WorkspaceState, grid: &NodeId, row: u32, col: u32) -> String {
    ws.grids
        .get(grid)
        .and_then(|grid| grid.cells.iter().find(|cell| cell.row == row && cell.col == col))
        .and_then(|cell| cell.authored.as_ref())
        .map(authored_seed_text)
        .unwrap_or_default()
}

/// The outcome chip's detail text — the same honest three-way readout the app's
/// `adapter.rs` renders, ported here (presentation only, no engine truth
/// re-derived).
fn outcome_detail(outcome: &CellOutcome) -> String {
    match outcome {
        CellOutcome::Literal { value } => format!("literal value {value}"),
        CellOutcome::Formula { value, unresolved } if unresolved.is_empty() => {
            format!("formula → {value}")
        }
        CellOutcome::Formula { value, unresolved } => {
            format!("formula → {value} (unresolved: {})", unresolved.join(", "))
        }
        CellOutcome::Cleared => "cell cleared".to_string(),
        CellOutcome::Rejected(diagnostics) => diagnostics
            .first()
            .map(|diagnostic| diagnostic.message.clone())
            .unwrap_or_else(|| "rejected".to_string()),
        CellOutcome::NoChange => "no change".to_string(),
    }
}

/// Render a scalar value as its display TEXT, or an ARRAY as a shape summary
/// plus an honest degrade note — NEVER a fabricated full array render, and NOT
/// a live link (Sheet is a stub + the Inspector is unmounted in S2).
fn render_value(value: &NodeValueProjection) -> AnyView {
    match value {
        NodeValueProjection::Array { rows, cols, .. } => {
            let summary = array_shape_summary(*rows, *cols);
            view! {
                <div
                    class="dna-notebook__result"
                    data-dna-density="working"
                    data-value-kind="array"
                    data-testid="notebook-block-value"
                >
                    <span class="dna-notebook__shape">{summary}</span>
                    <span class="dna-notebook__degrade" data-testid="notebook-array-degrade">
                        "array result — open in Sheet (not yet available in S2)"
                    </span>
                </div>
            }
            .into_any()
        }
        scalar => {
            let text = scalar_value_text(scalar);
            view! {
                <div
                    class="dna-notebook__result"
                    data-dna-density="working"
                    data-value-kind="scalar"
                    data-testid="notebook-block-value"
                >
                    <span class="dna-notebook__value">{text}</span>
                </div>
            }
            .into_any()
        }
    }
}

/// A defined name's result region: the backing cell's computed value when the
/// window resolved one, else an honest passthrough — the dynamic name's source
/// text, or a "not in window" note for a static name outside the current
/// window. Never a fabricated value.
fn render_name_value(name: &DefinedNameProjection, backing: &Option<GridCellProjection>) -> AnyView {
    if let Some(cell) = backing {
        return render_value(&cell.value);
    }
    match &name.target {
        DefinedNameTargetProjection::Dynamic { source_text } => {
            let text = source_text.clone();
            view! {
                <div
                    class="dna-notebook__result"
                    data-dna-density="working"
                    data-value-kind="formula"
                    data-testid="notebook-block-value"
                >
                    <span class="dna-notebook__formula">{text}</span>
                </div>
            }
            .into_any()
        }
        DefinedNameTargetProjection::Static(_) => view! {
            <div
                class="dna-notebook__result"
                data-dna-density="working"
                data-value-kind="unresolved"
                data-testid="notebook-block-value"
            >
                <span class="dna-notebook__note">"value not in the current window"</span>
            </div>
        }
        .into_any(),
    }
}

/// A table overlay's result region: a structural summary of its range and
/// column count. A table is not a single scalar — the block shows its shape,
/// not a fabricated value (full table rendering is a Sheet/Model concern).
fn render_table_value(table: &GridTableOverlayDescriptor) -> AnyView {
    let rect = &table.table_range;
    let rows = rect.bottom_row.saturating_sub(rect.top_row) + 1;
    let cols = if table.columns.is_empty() {
        rect.right_col.saturating_sub(rect.left_col) + 1
    } else {
        table.columns.len() as u32
    };
    let summary = format!("table — {rows}×{cols}");
    view! {
        <div
            class="dna-notebook__result"
            data-dna-density="working"
            data-value-kind="table"
            data-testid="notebook-block-value"
        >
            <span class="dna-notebook__shape">{summary}</span>
        </div>
    }
    .into_any()
}

/// The stable `data-block-kind` id for an entry.
fn entry_kind_id(kind: &NotebookEntryKind) -> &'static str {
    match kind {
        NotebookEntryKind::Name { .. } => "name",
        NotebookEntryKind::Cell { .. } => "cell",
        NotebookEntryKind::Table { .. } => "table",
    }
}

/// The gutter kind glyph — a per-kind mark that reads alongside the (structural)
/// classification tint. Purely decorative signposting; the load-bearing kind
/// signal is `data-block-kind`.
fn entry_kind_glyph(kind: &NotebookEntryKind) -> &'static str {
    match kind {
        NotebookEntryKind::Name { .. } => "◈",
        NotebookEntryKind::Cell { .. } => "▦",
        NotebookEntryKind::Table { .. } => "▤",
    }
}

/// The human-readable classification chip label.
fn classification_label(classification: NodeClassification) -> &'static str {
    match classification {
        NodeClassification::Input => "input",
        NodeClassification::FreeValue => "free value",
        NodeClassification::Intermediate => "intermediate",
        NodeClassification::Output => "output",
        NodeClassification::Empty => "empty",
    }
}

/// The liveness-dot state derived from the classification: a formula-backed
/// entry is "live", a literal-backed entry is "static", an empty one "inert".
fn liveness_id(classification: NodeClassification) -> &'static str {
    match classification {
        NodeClassification::Intermediate | NodeClassification::Output => "live",
        NodeClassification::Input | NodeClassification::FreeValue => "static",
        NodeClassification::Empty => "inert",
    }
}

/// The bounded shape summary for an array result (`3×2 array`).
fn array_shape_summary(rows: usize, cols: usize) -> String {
    format!("{rows}×{cols} array")
}

/// A scalar value's display TEXT. Every non-array projection maps to honest
/// text (the marker variants render as parenthesized notes so no value silently
/// disappears); the array arm is unreachable — callers route arrays through the
/// shape-summary path in [`render_value`].
fn scalar_value_text(value: &NodeValueProjection) -> String {
    match value {
        NodeValueProjection::Number { display, .. } => display.clone(),
        NodeValueProjection::Text(text) => text.clone(),
        NodeValueProjection::Logical { display, .. } => display.clone(),
        NodeValueProjection::Scalar(text) => text.clone(),
        NodeValueProjection::Reference { target } => target.clone(),
        NodeValueProjection::Empty => "(empty)".to_string(),
        NodeValueProjection::Missing => "(missing)".to_string(),
        NodeValueProjection::Unevaluated => "(unevaluated)".to_string(),
        NodeValueProjection::Pending => "(pending)".to_string(),
        NodeValueProjection::Error(message) => message.clone(),
        NodeValueProjection::Array { rows, cols, .. } => array_shape_summary(*rows, *cols),
    }
}

/// The Notebook stage's scoped stylesheet — Strand `--dna-*` tokens only. The
/// classification tints resolve through Strand's semantic soft-token palette
/// (`--dna-{green,amber,accent,signal}-soft` + `--dna-paper-2`); the density
/// values (`68ch` measure, `1.6`/`1.35` leading) are the Strand
/// `Density::Reading`/`Density::Working` constants, applied structurally to the
/// Reading spine and its Working result rows.
pub const NOTEBOOK_CSS: &str = "\
.dna-notebook{display:flex;flex-direction:column;gap:var(--dna-gap-4);max-width:68ch;line-height:1.6;padding:var(--dna-gap-5);color:var(--dna-ink)}
.dna-notebook__empty{margin:0;color:var(--dna-ink-3);font-style:italic}
.dna-notebook__block{display:flex;align-items:stretch;border:1px solid var(--dna-line);border-radius:var(--dna-radius-card);background:var(--dna-paper);overflow:hidden}
.dna-notebook__gutter{display:flex;align-items:flex-start;justify-content:center;padding:var(--dna-gap-3) var(--dna-gap-2);min-width:1.9rem;border-right:1px solid var(--dna-line);background:var(--dna-paper-2)}
.dna-notebook__gutter--input{background:var(--dna-green-soft)}
.dna-notebook__gutter--free_value{background:var(--dna-amber-soft)}
.dna-notebook__gutter--intermediate{background:var(--dna-accent-soft)}
.dna-notebook__gutter--output{background:var(--dna-signal-soft)}
.dna-notebook__gutter--empty{background:var(--dna-paper-2)}
.dna-notebook__glyph{font-size:13px;line-height:1;color:var(--dna-ink-2)}
.dna-notebook__body{display:flex;flex-direction:column;gap:var(--dna-gap-2);padding:var(--dna-gap-3) var(--dna-gap-4);flex:1;min-width:0}
.dna-notebook__name-row{display:flex;align-items:baseline;gap:var(--dna-gap-2);flex-wrap:wrap}
.dna-notebook__name{font-weight:600;color:var(--dna-ink);font-family:'Recursive Mono','Cascadia Code',Consolas,ui-monospace,monospace;font-size:12px;word-break:break-word}
.dna-notebook__chip{font-size:10px;text-transform:uppercase;letter-spacing:0.04em;color:var(--dna-accent-ink);background:var(--dna-accent-soft);border-radius:var(--dna-radius-chip);padding:0 var(--dna-gap-2)}
.dna-notebook__dot{width:6px;height:6px;border-radius:50%;display:inline-block;align-self:center;background:var(--dna-ink-3)}
.dna-notebook__dot[data-liveness=\"live\"]{background:var(--dna-value-ink)}
.dna-notebook__dot[data-liveness=\"static\"]{background:var(--dna-prov-const)}
.dna-notebook__dot[data-liveness=\"inert\"]{background:var(--dna-ink-3)}
.dna-notebook__result{line-height:1.35;display:flex;align-items:baseline;gap:var(--dna-gap-2);flex-wrap:wrap;font-family:'Recursive Mono','Cascadia Code',Consolas,ui-monospace,monospace;font-size:12px}
.dna-notebook__value{color:var(--dna-value-ink);font-weight:600;word-break:break-word}
.dna-notebook__formula{color:var(--dna-ink-2)}
.dna-notebook__shape{color:var(--dna-value-ink);font-weight:600}
.dna-notebook__degrade{color:var(--dna-ink-3);font-style:italic;font-family:inherit;font-size:11px}
.dna-notebook__note{color:var(--dna-ink-3);font-style:italic;font-family:inherit}
.dna-notebook__edit{display:flex;flex-direction:column;gap:var(--dna-gap-2);margin-top:var(--dna-gap-2)}
.dna-notebook__readonly{margin:var(--dna-gap-2) 0 0;color:var(--dna-ink-3);font-style:italic;font-size:11px}
.dna-notebook__outcome{display:flex;gap:var(--dna-gap-2);align-items:baseline;padding:var(--dna-gap-1) var(--dna-gap-3);border-radius:var(--dna-radius-chip);background:var(--dna-paper-2);font-family:'Recursive Mono','Cascadia Code',Consolas,ui-monospace,monospace}
.dna-notebook__outcome-label{font-weight:600;text-transform:uppercase;letter-spacing:0.05em;font-size:10px;color:var(--dna-accent-ink)}
.dna-notebook__outcome-detail{font-size:11px;color:var(--dna-ink-2)}
";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notebook_stage_identity() {
        let stage = NotebookStage::new();
        assert_eq!(stage.id(), StageId::Notebook);
        assert_eq!(stage.title(), "Notebook");
        assert!(stage.supports(&ProfileTag::ExcelStrict));
    }

    #[test]
    fn classification_maps_to_chip_liveness_and_tint_class() {
        // Each classification produces a stable chip label, a liveness state,
        // and (via `stable_id`) the tint modifier class NOTEBOOK_CSS styles.
        for (classification, chip, liveness, tint) in [
            (NodeClassification::Input, "input", "static", "input"),
            (
                NodeClassification::FreeValue,
                "free value",
                "static",
                "free_value",
            ),
            (
                NodeClassification::Intermediate,
                "intermediate",
                "live",
                "intermediate",
            ),
            (NodeClassification::Output, "output", "live", "output"),
            (NodeClassification::Empty, "empty", "inert", "empty"),
        ] {
            assert_eq!(classification_label(classification), chip);
            assert_eq!(liveness_id(classification), liveness);
            assert_eq!(classification.stable_id(), tint);
            // The tint class every gutter carries is styled in NOTEBOOK_CSS.
            assert!(NOTEBOOK_CSS.contains(&format!(".dna-notebook__gutter--{tint}")));
        }
    }

    #[test]
    fn scalar_value_text_renders_each_projection_honestly() {
        assert_eq!(
            scalar_value_text(&NodeValueProjection::Number {
                raw: "42".to_string(),
                display: "42".to_string(),
            }),
            "42"
        );
        assert_eq!(
            scalar_value_text(&NodeValueProjection::Text("hi".to_string())),
            "hi"
        );
        assert_eq!(
            scalar_value_text(&NodeValueProjection::Logical {
                value: true,
                display: "TRUE".to_string(),
            }),
            "TRUE"
        );
        // Marker variants never vanish — they render as parenthesized notes.
        assert_eq!(
            scalar_value_text(&NodeValueProjection::Empty),
            "(empty)"
        );
        assert_eq!(
            scalar_value_text(&NodeValueProjection::Unevaluated),
            "(unevaluated)"
        );
    }

    #[test]
    fn array_summary_is_shape_only_never_a_full_render() {
        assert_eq!(array_shape_summary(3, 2), "3×2 array");
        assert_eq!(array_shape_summary(1, 1), "1×1 array");
    }

    #[test]
    fn kind_ids_and_glyphs_are_distinct_per_kind() {
        use dnacalc_skin_ir::{GridAuthoredCellProjection, GridAuthoredKindProjection, NodeId};
        let cell = NotebookEntryKind::Cell {
            grid: NodeId::new("Sheet1"),
            authored: GridAuthoredCellProjection {
                row: 1,
                col: 1,
                kind: GridAuthoredKindProjection::Literal,
                literal_text: Some("1".to_string()),
                source_text: None,
                editability: dnacalc_skin_ir::GridEditabilityProjection::Editable,
            },
            value: NodeValueProjection::Number {
                raw: "1".to_string(),
                display: "1".to_string(),
            },
        };
        assert_eq!(entry_kind_id(&cell), "cell");
        assert_eq!(entry_kind_glyph(&cell), "▦");
    }
}
