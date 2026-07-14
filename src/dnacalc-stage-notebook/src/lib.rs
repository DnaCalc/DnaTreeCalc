//! TIER: TP (Presentation) — T0 skin-ir + Leptos + TP crates only; no Ox* ever (P-gate).
//!
//! The Notebook stage (S2.5): a [`dnacalc_shell::StageSurface`] registered
//! under [`dnacalc_shell::StageId::Notebook`]. `mount` renders the reactive
//! single-column block list (NOTEBOOK_SPEC §3) from host truth: a closure over
//! `ctx.workspace` re-derives [`model::derive_entries`] on every workspace
//! change and paints one block per entry. This stage **renders only** — it
//! authors nothing (editing is a later bead), honoring the SHELL_SPEC §6
//! layering law. It is never rendered blank: an empty derivation renders an
//! explicit, testable honest-empty card.

use leptos::prelude::*;

use dnacalc_shell::{ProfileTag, StageContext, StageHandle, StageId, StageSurface};
use dnacalc_skin_ir::{
    DefinedNameProjection, DefinedNameTargetProjection, GridCellProjection,
    GridTableOverlayDescriptor, NodeClassification, NodeValueProjection,
};

pub mod model;

pub use model::{NotebookEntry, NotebookEntryKind, derive_entries};

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
                        entries.into_iter().map(render_block).collect_view().into_any()
                    }
                }}
            </section>
        }
        .into_any();
        StageHandle::new(view)
    }
}

/// Render one derived entry as a single block (NOTEBOOK_SPEC §3 block anatomy):
/// a gutter carrying a kind glyph + a structural classification tint, a name row
/// (`display_name` · classification chip · liveness dot), and a result region.
fn render_block(entry: NotebookEntry) -> AnyView {
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
            </div>
        </article>
    }
    .into_any()
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
