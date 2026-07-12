//! Inspector slot model — SHELL_SPEC.md §7: projection-fed blocks
//! (value/shape, format + provenance, dependencies both ways) plus a typed
//! stage-specific extension point. The slot enum carries the reserved
//! `Evidence` variant, never populated this phase (PARITY_TRUST_UX.md §5:
//! "slot enum carries the variant; never populated").

use dnacalc_skin_ir::selection::SelectionState;
use dnacalc_skin_ir::workspace::WorkspaceState;

/// The Inspector's typed slots, top to bottom. No free-form injection —
/// a stage extends the Inspector only through [`InspectorSlotKind::StagePanel`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InspectorSlotKind {
    ValueShape,
    Format,
    Dependencies,
    StagePanel,
    /// Reserved for the parity/evidence story's return. Never populated
    /// this phase — it renders NOTHING (asserted in tests here and in the
    /// browser smoke).
    Evidence,
}

impl InspectorSlotKind {
    pub const ALL: [InspectorSlotKind; 5] = [
        InspectorSlotKind::ValueShape,
        InspectorSlotKind::Format,
        InspectorSlotKind::Dependencies,
        InspectorSlotKind::StagePanel,
        InspectorSlotKind::Evidence,
    ];

    #[must_use]
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::ValueShape => "value-shape",
            Self::Format => "format",
            Self::Dependencies => "dependencies",
            Self::StagePanel => "stage-panel",
            Self::Evidence => "evidence",
        }
    }
}

/// One rendered Inspector block: a titled body, honest absence, or nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InspectorSlotContent {
    /// A titled block with body lines.
    Block {
        title: &'static str,
        lines: Vec<String>,
    },
    /// The projection behind this block is not available — honest absence.
    Absent {
        title: &'static str,
        reason: &'static str,
    },
    /// Reserved slot: renders NOTHING (no title, no frame, no placeholder).
    Nothing,
}

/// Resolve one Inspector slot from the projections. Pure, so the
/// render-nothing law is testable natively; the DOM panel maps this 1:1.
#[must_use]
pub fn inspector_slot_content(
    kind: InspectorSlotKind,
    workspace: &WorkspaceState,
    selection: &SelectionState,
) -> InspectorSlotContent {
    match kind {
        InspectorSlotKind::ValueShape => match selected_node(workspace, selection) {
            Some((id, node)) => InspectorSlotContent::Block {
                title: "Value",
                lines: vec![
                    format!("node: {id}"),
                    format!("name: {}", node.display_name),
                    format!("value: {}", node.computed_value.display_text()),
                ],
            },
            None => InspectorSlotContent::Absent {
                title: "Value",
                reason: "no selection",
            },
        },
        InspectorSlotKind::Format => InspectorSlotContent::Absent {
            title: "Format",
            reason: "format projection lands with G2",
        },
        InspectorSlotKind::Dependencies => match selected_node_id(workspace, selection) {
            Some(id) => {
                let outgoing = workspace
                    .dependencies
                    .edges_by_owner
                    .get(id)
                    .map_or(0, Vec::len);
                let incoming = workspace
                    .dependencies
                    .reverse_edges
                    .get(id)
                    .map_or(0, Vec::len);
                InspectorSlotContent::Block {
                    title: "Dependencies",
                    lines: vec![
                        format!("outgoing: {outgoing}"),
                        format!("incoming: {incoming}"),
                    ],
                }
            }
            None => InspectorSlotContent::Absent {
                title: "Dependencies",
                reason: "no selection",
            },
        },
        InspectorSlotKind::StagePanel => InspectorSlotContent::Absent {
            title: "Stage",
            reason: "no stage panel registered",
        },
        // PARITY_TRUST_UX §5: reserved, never populated this phase.
        InspectorSlotKind::Evidence => InspectorSlotContent::Nothing,
    }
}

fn selected_node_id<'w>(
    workspace: &'w WorkspaceState,
    selection: &'w SelectionState,
) -> Option<&'w dnacalc_skin_ir::identity::NodeId> {
    selection
        .primary
        .as_ref()
        .filter(|id| workspace.nodes.contains_key(*id))
}

fn selected_node<'w>(
    workspace: &'w WorkspaceState,
    selection: &'w SelectionState,
) -> Option<(
    &'w dnacalc_skin_ir::identity::NodeId,
    &'w dnacalc_skin_ir::workspace::NodeView,
)> {
    let id = selection.primary.as_ref()?;
    workspace.nodes.get(id).map(|node| (id, node))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evidence_slot_renders_nothing_and_only_it_does() {
        let workspace = WorkspaceState::default();
        let selection = SelectionState::default();
        for kind in InspectorSlotKind::ALL {
            let content = inspector_slot_content(kind, &workspace, &selection);
            match kind {
                InspectorSlotKind::Evidence => assert_eq!(
                    content,
                    InspectorSlotContent::Nothing,
                    "Evidence is reserved and must render nothing"
                ),
                _ => assert_ne!(
                    content,
                    InspectorSlotContent::Nothing,
                    "{kind:?} must render a block or honest absence"
                ),
            }
        }
    }

    #[test]
    fn slot_ids_are_distinct() {
        let ids: std::collections::HashSet<_> = InspectorSlotKind::ALL
            .iter()
            .map(|kind| kind.stable_id())
            .collect();
        assert_eq!(ids.len(), InspectorSlotKind::ALL.len());
    }
}
