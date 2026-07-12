//! The Bench X-Ray panel (BENCH_SPEC §4) and its pure view-model helpers.
//!
//! The panel renders `FormulaDrillSurface` drill rows — expression text, state
//! chip, value preview, argument name/role — plus windowed array previews. It
//! is a TF surface (bench-app), so it MAY construct the sanctioned
//! `OneFormulaIntent` *request* verbs (`RequestResultArrayWindow`,
//! `RequestDrillArrayWindow`); the layering law forbids only the TP *bridge*
//! from constructing intents. The panel never parses formula text and never
//! evaluates skin-side — every value it shows is host truth read off the drill
//! tree.
//!
//! The ring (step-out/in) model and the array-window request math are pure
//! functions here so they are unit-tested natively without a browser.

use leptos::prelude::*;

use dnacalc_bridge::{BridgeEvent, MAX_WINDOW_EDGE, drill_node_at_caret, drill_state_label};
use dnacalc_skin_ir::formula::{
    ArrayPreviewProjection, ArrayWindowProjection, FormulaDrillNodeProjection, FormulaDrillSurface,
    OneFormulaIntent,
};

// ---------------------------------------------------------------------------
// Ring (step-out / step-in) — pure model
// ---------------------------------------------------------------------------

/// One ring step: `Out` widens to the enclosing expression (`]` /
/// `TraceForward`); `In` narrows to the first child (`[` / `TraceBack`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RingStep {
    Out,
    In,
}

/// Find a node by id anywhere in the drill tree.
#[must_use]
pub fn find_node<'a>(
    nodes: &'a [FormulaDrillNodeProjection],
    id: &str,
) -> Option<&'a FormulaDrillNodeProjection> {
    for node in nodes {
        if node.node_id == id {
            return Some(node);
        }
        if let Some(hit) = find_node(&node.children, id) {
            return Some(hit);
        }
    }
    None
}

/// Find the parent of the node with `id` (the node whose `children` contain it).
#[must_use]
pub fn find_parent<'a>(
    nodes: &'a [FormulaDrillNodeProjection],
    id: &str,
) -> Option<&'a FormulaDrillNodeProjection> {
    for node in nodes {
        if node.children.iter().any(|child| child.node_id == id) {
            return Some(node);
        }
        if let Some(hit) = find_parent(&node.children, id) {
            return Some(hit);
        }
    }
    None
}

/// Compute the next ring node id after a step.
///
/// - When no ring is set yet, the step *establishes* the ring at the drill
///   node under the caret (or the first root when the caret matches nothing),
///   without moving — the first `]`/`[` selects the current subexpression.
/// - `Out` then walks to the parent (staying put at the root); `In` walks to
///   the first child (staying put at a leaf).
///
/// Returns `None` only for an empty tree.
#[must_use]
pub fn ring_step(
    tree: &[FormulaDrillNodeProjection],
    current: Option<&str>,
    caret: usize,
    step: RingStep,
) -> Option<String> {
    // Resolve the anchor node the step departs from.
    let anchor = current
        .and_then(|id| find_node(tree, id))
        .or_else(|| drill_node_at_caret(tree, caret))
        .or_else(|| tree.first());
    let anchor = anchor?;

    // No ring yet: establish it at the anchor, no movement.
    if current.is_none() || find_node(tree, current.unwrap_or_default()).is_none() {
        return Some(anchor.node_id.clone());
    }

    match step {
        RingStep::Out => Some(
            find_parent(tree, &anchor.node_id)
                .map_or_else(|| anchor.node_id.clone(), |parent| parent.node_id.clone()),
        ),
        RingStep::In => Some(
            anchor
                .children
                .first()
                .map_or_else(|| anchor.node_id.clone(), |child| child.node_id.clone()),
        ),
    }
}

// ---------------------------------------------------------------------------
// Array-window request intents (honoring the skin-IR caps)
// ---------------------------------------------------------------------------

/// The next drill-node array window request (BENCH_SPEC §4, 64×64 cap), or
/// `None` when the preview already covers the array. Row-major: finish the
/// rows before advancing a column. Node-addressed so the host resolves *which*
/// drill node's array to page.
#[must_use]
pub fn drill_array_window_intent(
    formula_space_id: String,
    node_id: String,
    preview: &ArrayPreviewProjection,
) -> Option<OneFormulaIntent> {
    let visible_rows = preview.rows.len();
    let visible_cols = preview.rows.first().map_or(0, Vec::len);
    let (row_offset, col_offset, row_count, col_count) = next_window(
        preview.row_offset,
        preview.col_offset,
        visible_rows,
        visible_cols,
        preview.total_rows,
        preview.total_cols,
    )?;
    Some(OneFormulaIntent::RequestDrillArrayWindow {
        formula_space_id,
        node_id,
        row_offset,
        col_offset,
        row_count,
        col_count,
    })
}

/// The next result array window request (BENCH_SPEC §3, ≤4096 cells), or `None`
/// when the window already covers the spill.
#[must_use]
pub fn result_array_window_intent(
    formula_space_id: String,
    window: &ArrayWindowProjection,
    total_rows: usize,
    total_cols: usize,
) -> Option<OneFormulaIntent> {
    let visible_rows = window.cells.len();
    let visible_cols = window.cells.first().map_or(0, Vec::len);
    let (row_offset, col_offset, row_count, col_count) = next_window(
        window.row_offset,
        window.col_offset,
        visible_rows,
        visible_cols,
        total_rows,
        total_cols,
    )?;
    Some(OneFormulaIntent::RequestResultArrayWindow {
        formula_space_id,
        row_offset,
        col_offset,
        row_count,
        col_count,
    })
}

/// Shared next-window math: advance row-major from the current window,
/// clamping each edge to the 64-cell cap (so the 64×64 = 4096 cell cap always
/// holds). `None` when the visible window already reaches both extents.
fn next_window(
    row_offset: usize,
    col_offset: usize,
    visible_rows: usize,
    visible_cols: usize,
    total_rows: usize,
    total_cols: usize,
) -> Option<(usize, usize, usize, usize)> {
    let rows_remain = row_offset.saturating_add(visible_rows) < total_rows;
    let cols_remain = col_offset.saturating_add(visible_cols) < total_cols;
    if !rows_remain && !cols_remain {
        return None;
    }
    let (next_row, next_col) = if rows_remain {
        (row_offset + visible_rows, col_offset)
    } else {
        (0, col_offset.saturating_add(visible_cols))
    };
    let row_count = total_rows
        .saturating_sub(next_row)
        .clamp(1, MAX_WINDOW_EDGE);
    let col_count = total_cols
        .saturating_sub(next_col)
        .clamp(1, MAX_WINDOW_EDGE);
    Some((next_row, next_col, row_count, col_count))
}

// ---------------------------------------------------------------------------
// Panel rendering
// ---------------------------------------------------------------------------

/// Render the drill tree rows recursively (pre-order, nested by depth). Each
/// row is addressable by its stable `node_id` (mechanism 20) and mirrors the
/// current ring via `data-ring`.
fn drill_rows(
    nodes: &[FormulaDrillNodeProjection],
    depth: usize,
    ring: RwSignal<Option<String>>,
    formula_space_id: String,
    on_intent: Callback<OneFormulaIntent>,
) -> AnyView {
    nodes
        .iter()
        .map(|node| drill_row(node, depth, ring, formula_space_id.clone(), on_intent))
        .collect_view()
        .into_any()
}

fn drill_row(
    node: &FormulaDrillNodeProjection,
    depth: usize,
    ring: RwSignal<Option<String>>,
    formula_space_id: String,
    on_intent: Callback<OneFormulaIntent>,
) -> AnyView {
    let node_id = node.node_id.clone();
    let ring_id = node_id.clone();
    let state_id = dnacalc_bridge::drill_state_id(node.state);
    let state_label = drill_state_label(node.state);
    let expression = node
        .expression_text
        .clone()
        .unwrap_or_else(|| node.label.clone());
    let indent = format!("padding-left:{}px", depth * 16);

    let arg = node.argument_name.clone().map(|name| {
        let role = node
            .argument_role
            .clone()
            .map(|role| format!(" · {role}"))
            .unwrap_or_default();
        view! { <span class="dna-xray__arg">{format!("{name}{role}")}</span> }
    });
    let value = node
        .value_preview
        .clone()
        .map(|value| view! { <span class="dna-xray__value">{value}</span> });
    let error = node
        .error_message
        .clone()
        .map(|message| view! { <span class="dna-xray__error">{message}</span> });

    let array = node
        .array_preview
        .clone()
        .map(|preview| array_preview_view(&node_id, &formula_space_id, &preview, on_intent));

    let children = drill_rows(&node.children, depth + 1, ring, formula_space_id, on_intent);

    view! {
        <div
            class="dna-xray__row"
            data-xray-node=node_id.clone()
            data-state=state_id
            data-ring=move || (ring.get().as_deref() == Some(ring_id.as_str())).to_string()
            style=indent
        >
            <span class="dna-xray__expr">{expression}</span>
            <span class="dna-xray__chip" data-state=state_id>{state_label}</span>
            {arg}
            {value}
            {error}
            {array}
        </div>
        {children}
    }
    .into_any()
}

/// A windowed array preview grid (drill node) with shape readout + truncation
/// honesty and a pager that emits the sanctioned `RequestDrillArrayWindow`.
fn array_preview_view(
    node_id: &str,
    formula_space_id: &str,
    preview: &ArrayPreviewProjection,
    on_intent: Callback<OneFormulaIntent>,
) -> AnyView {
    let shape = format!("{}\u{00D7}{}", preview.total_rows, preview.total_cols);
    let rows = preview.rows.clone();
    let pager =
        drill_array_window_intent(formula_space_id.to_string(), node_id.to_string(), preview);
    let truncated = preview.truncated;
    view! {
        <div class="dna-xray__array" data-array-node=node_id.to_string()>
            <div class="dna-xray__array-head">
                <span class="dna-xray__array-shape">{shape}</span>
                {truncated.then(|| view! {
                    <span class="dna-xray__array-trunc" data-truncated="true">"windowed"</span>
                })}
                {pager.map(|intent| view! {
                    <button
                        type="button"
                        class="dna-xray__array-more"
                        on:click=move |_| on_intent.run(intent.clone())
                    >
                        "More"
                    </button>
                })}
            </div>
            <div class="dna-xray__grid" role="grid">
                {rows.into_iter().map(|row| view! {
                    <div class="dna-xray__grid-row" role="row">
                        {row.into_iter().map(|cell| view! {
                            <span class="dna-xray__grid-cell" role="gridcell">{cell}</span>
                        }).collect_view()}
                    </div>
                }).collect_view()}
            </div>
        </div>
    }
    .into_any()
}

/// The dockable X-Ray panel (BENCH_SPEC §2/§4). Rendered by the Result stage
/// when `drill.expanded`. `tabindex` makes it a focus target so bare `]`/`[`
/// (ring out/in) reach the shell's `TraceForward`/`TraceBack` verbs from here
/// rather than being captured as formula characters in the editor.
#[component]
pub fn XRayPanel(
    drill: FormulaDrillSurface,
    formula_space_id: String,
    ring: RwSignal<Option<String>>,
    on_event: Callback<BridgeEvent>,
    on_intent: Callback<OneFormulaIntent>,
) -> impl IntoView {
    let phases = drill.phase_summaries.clone();
    let has_tree = !drill.tree.is_empty();
    let rows = drill_rows(&drill.tree, 0, ring, formula_space_id, on_intent);
    view! {
        <section
            class="dna-xray"
            data-region="xray"
            data-testid="xray-panel"
            tabindex="0"
            aria-label="Formula X-Ray"
        >
            <header class="dna-xray__head">
                <span class="dna-xray__title">"X-Ray"</span>
                <div class="dna-xray__phases">
                    {phases.into_iter().map(|phase| view! {
                        <span class="dna-xray__phase" data-phase=phase.label>
                            {format!("{}: {}", phase.label, phase.detail)}
                        </span>
                    }).collect_view()}
                </div>
                <button
                    type="button"
                    class="dna-xray__close"
                    aria-label="Close X-Ray"
                    on:click=move |_| on_event.run(BridgeEvent::DrillToggled)
                >
                    "×"
                </button>
            </header>
            {if has_tree {
                view! { <div class="dna-xray__rows">{rows}</div> }.into_any()
            } else {
                view! {
                    <p class="dna-xray__empty" data-xray-empty="true">
                        "Author a formula to trace its evaluation."
                    </p>
                }
                .into_any()
            }}
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dnacalc_skin_ir::formula::{ArrayWindowCellProjection, FormulaDrillNodeStateProjection};

    fn leaf(id: &str, span: (usize, usize)) -> FormulaDrillNodeProjection {
        FormulaDrillNodeProjection {
            node_id: id.into(),
            label: id.into(),
            developer_label: None,
            expression_text: Some(id.into()),
            kind: Some("Literal".into()),
            source_span_start: Some(span.0),
            source_span_len: Some(span.1 - span.0),
            branch_disposition: None,
            argument_name: None,
            argument_role: None,
            error_message: None,
            value_preview: Some("v".into()),
            array_preview: None,
            state: FormulaDrillNodeStateProjection::Evaluated,
            children: vec![],
        }
    }

    /// root [0,11) ⊃ call [1,10) ⊃ arg [5,6).
    fn tree() -> Vec<FormulaDrillNodeProjection> {
        let arg = leaf("arg", (5, 6));
        let mut call = leaf("call", (1, 10));
        call.children = vec![arg];
        let mut root = leaf("root", (0, 11));
        root.children = vec![call];
        vec![root]
    }

    #[test]
    fn ring_step_out_walks_child_to_parent_to_root() {
        let tree = tree();
        // No ring yet: the first step ESTABLISHES the ring at the caret's node
        // (caret 5 sits in `arg`), without moving.
        let seed = ring_step(&tree, None, 5, RingStep::Out).unwrap();
        assert_eq!(seed, "arg");
        // Out from arg -> call -> root, then stays at root.
        let up1 = ring_step(&tree, Some("arg"), 5, RingStep::Out).unwrap();
        assert_eq!(up1, "call");
        let up2 = ring_step(&tree, Some("call"), 5, RingStep::Out).unwrap();
        assert_eq!(up2, "root");
        let up3 = ring_step(&tree, Some("root"), 5, RingStep::Out).unwrap();
        assert_eq!(up3, "root", "the root has no enclosing expression");
    }

    #[test]
    fn ring_step_in_walks_parent_to_first_child_to_leaf() {
        let tree = tree();
        let down1 = ring_step(&tree, Some("root"), 0, RingStep::In).unwrap();
        assert_eq!(down1, "call");
        let down2 = ring_step(&tree, Some("call"), 0, RingStep::In).unwrap();
        assert_eq!(down2, "arg");
        let down3 = ring_step(&tree, Some("arg"), 0, RingStep::In).unwrap();
        assert_eq!(down3, "arg", "a leaf has no narrower expression");
    }

    #[test]
    fn ring_step_reseeds_when_the_current_id_is_stale() {
        let tree = tree();
        // A ring id no longer in the tree (formula changed) re-seeds at caret.
        let reseed = ring_step(&tree, Some("gone"), 5, RingStep::Out).unwrap();
        assert_eq!(reseed, "arg");
        // An empty tree rings nowhere.
        assert_eq!(ring_step(&[], None, 0, RingStep::Out), None);
    }

    #[test]
    fn find_parent_and_node_walk_the_nested_tree() {
        let tree = tree();
        assert_eq!(
            find_node(&tree, "arg").map(|n| n.node_id.as_str()),
            Some("arg")
        );
        assert_eq!(
            find_parent(&tree, "arg").map(|n| n.node_id.as_str()),
            Some("call")
        );
        assert_eq!(find_parent(&tree, "root"), None);
    }

    #[test]
    fn drill_array_window_intent_pages_row_major_honoring_the_64_cap() {
        // A 100×1 preview showing only row 0 pages to the next 64-row window.
        let preview = ArrayPreviewProjection {
            total_rows: 100,
            total_cols: 1,
            row_offset: 0,
            col_offset: 0,
            rows: vec![vec!["1".into()]],
            truncated: true,
        };
        let intent = drill_array_window_intent("f-1".into(), "node-7".into(), &preview)
            .expect("more rows remain");
        match intent {
            OneFormulaIntent::RequestDrillArrayWindow {
                formula_space_id,
                node_id,
                row_offset,
                col_offset,
                row_count,
                col_count,
            } => {
                assert_eq!(formula_space_id, "f-1");
                assert_eq!(node_id, "node-7");
                assert_eq!((row_offset, col_offset), (1, 0));
                assert_eq!(row_count, 64, "clamped to the 64-edge cap, not 99");
                assert_eq!(col_count, 1);
            }
            other => panic!("expected a drill window request, got {other:?}"),
        }
        // The intent it produces is valid under the skin-IR cap.
        assert_eq!(
            drill_array_window_intent("f-1".into(), "node-7".into(), &preview)
                .unwrap()
                .validate(),
            Ok(())
        );
        // A fully-covered preview pages nowhere.
        let done = ArrayPreviewProjection {
            total_rows: 1,
            total_cols: 1,
            rows: vec![vec!["x".into()]],
            truncated: false,
            ..preview
        };
        assert_eq!(
            drill_array_window_intent("f".into(), "n".into(), &done),
            None
        );
    }

    #[test]
    fn result_array_window_intent_advances_a_column_after_the_rows_finish() {
        // A 2×3 spill showing all 2 rows of column 0..1 pages to column 2.
        let cell = ArrayWindowCellProjection::default();
        let window = ArrayWindowProjection {
            total_rows: 2,
            total_cols: 3,
            row_offset: 0,
            col_offset: 0,
            cells: vec![vec![cell.clone(), cell.clone()], vec![cell.clone(), cell]],
        };
        let intent =
            result_array_window_intent("f-1".into(), &window, 2, 3).expect("a column remains");
        match intent {
            OneFormulaIntent::RequestResultArrayWindow {
                row_offset,
                col_offset,
                row_count,
                col_count,
                ..
            } => {
                assert_eq!((row_offset, col_offset), (0, 2), "rows done -> next column");
                assert_eq!((row_count, col_count), (2, 1));
            }
            other => panic!("expected a result window request, got {other:?}"),
        }
        // A window already covering the whole spill pages nowhere.
        let full = ArrayWindowProjection {
            total_rows: 1,
            total_cols: 1,
            row_offset: 0,
            col_offset: 0,
            cells: vec![vec![ArrayWindowCellProjection::default()]],
        };
        assert_eq!(result_array_window_intent("f".into(), &full, 1, 1), None);
    }
}

/// The X-Ray panel's component-scoped styles (strand `--dna-*` tokens only).
pub const XRAY_CSS: &str = "\
.dna-xray{border-top:1px solid var(--dna-line);background:var(--dna-paper);display:flex;flex-direction:column;gap:var(--dna-gap-2);padding:var(--dna-gap-3);max-height:38vh;overflow:auto;font-size:12px}
.dna-xray:focus{outline:2px solid var(--dna-accent);outline-offset:-2px}
.dna-xray__head{display:flex;align-items:center;gap:var(--dna-gap-3)}
.dna-xray__title{font-weight:600;color:var(--dna-accent-ink);text-transform:uppercase;letter-spacing:0.05em}
.dna-xray__phases{display:flex;flex-wrap:wrap;gap:var(--dna-gap-2);flex:1}
.dna-xray__phase{color:var(--dna-ink-3);background:var(--dna-paper-2);border-radius:var(--dna-radius-chip);padding:0 var(--dna-gap-2)}
.dna-xray__close{font:inherit;font-size:14px;line-height:1;color:var(--dna-ink-2);background:transparent;border:0;cursor:pointer;padding:0 var(--dna-gap-2)}
.dna-xray__rows{display:flex;flex-direction:column}
.dna-xray__row{display:flex;flex-wrap:wrap;align-items:baseline;gap:var(--dna-gap-3);padding:var(--dna-gap-1) var(--dna-gap-2);border-radius:var(--dna-radius-chip)}
.dna-xray__row[data-ring=\"true\"]{background:var(--dna-accent-soft);box-shadow:inset 2px 0 0 var(--dna-accent)}
.dna-xray__expr{font-family:'Recursive Mono','Cascadia Code',Consolas,ui-monospace,monospace;color:var(--dna-ink)}
.dna-xray__chip{font-size:10px;text-transform:uppercase;letter-spacing:0.04em;border-radius:var(--dna-radius-chip);padding:0 var(--dna-gap-2);background:var(--dna-paper-2);color:var(--dna-ink-2)}
.dna-xray__chip[data-state=\"evaluated\"]{background:var(--dna-accent-soft);color:var(--dna-accent-ink)}
.dna-xray__chip[data-state=\"bound\"]{background:var(--dna-signal-soft);color:var(--dna-prov-ext)}
.dna-xray__chip[data-state=\"error\"],.dna-xray__chip[data-state=\"blocked\"]{background:var(--dna-red-soft);color:var(--dna-red-ink)}
.dna-xray__chip[data-state=\"skipped\"],.dna-xray__chip[data-state=\"opaque\"]{color:var(--dna-ink-3)}
.dna-xray__arg{color:var(--dna-ink-3)}
.dna-xray__value{color:var(--dna-value-ink);font-weight:600}
.dna-xray__error{color:var(--dna-red-ink)}
.dna-xray__array{flex-basis:100%;display:flex;flex-direction:column;gap:var(--dna-gap-1);margin:var(--dna-gap-1) 0}
.dna-xray__array-head{display:flex;align-items:baseline;gap:var(--dna-gap-3)}
.dna-xray__array-shape{color:var(--dna-ink-3)}
.dna-xray__array-trunc{color:var(--dna-prov-ext);font-size:10px;text-transform:uppercase}
.dna-xray__array-more{font:inherit;font-size:11px;color:var(--dna-accent-ink);background:var(--dna-paper-2);border:1px solid var(--dna-line);border-radius:var(--dna-radius-chip);padding:0 var(--dna-gap-2);cursor:pointer}
.dna-xray__grid{display:flex;flex-direction:column;gap:1px;overflow-x:auto}
.dna-xray__grid-row{display:flex;gap:1px}
.dna-xray__grid-cell{min-width:2.5em;padding:0 var(--dna-gap-2);background:var(--dna-paper-2);color:var(--dna-ink);font-family:'Recursive Mono','Cascadia Code',Consolas,ui-monospace,monospace;text-align:right}
.dna-xray__empty{color:var(--dna-ink-3);font-style:italic;margin:0}
";
