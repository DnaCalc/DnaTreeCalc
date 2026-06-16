use std::collections::BTreeMap;

use crate::permissions::Persona;
use crate::selection::SelectionState;
use crate::workspace::{
    ClipboardPayloadProjection, NodeContentKind, RevisionHistoryEntryProjection, WorkspaceState,
};

/// Read-side command catalog for skins, menus, and shortcut help.
///
/// This is metadata over the closed [`WorkspaceIntent`](crate::WorkspaceIntent)
/// surface. It never executes a command and it does not synthesize missing
/// intent payloads; target construction remains the caller's job.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommandCatalogProjection {
    pub entries: Vec<CommandMetaProjection>,
}

impl CommandCatalogProjection {
    #[must_use]
    pub fn get(&self, intent_kind: CommandIntentKindProjection) -> Option<&CommandMetaProjection> {
        self.entries
            .iter()
            .find(|entry| entry.intent_kind == intent_kind)
    }

    #[must_use]
    pub fn by_kind(&self) -> BTreeMap<CommandIntentKindProjection, CommandMetaProjection> {
        self.entries
            .iter()
            .map(|entry| (entry.intent_kind, entry.clone()))
            .collect()
    }

    /// Overlay a persona's governance on top of the context-driven catalog:
    /// any command the persona may not dispatch is disabled with a persona
    /// reason, regardless of whether the selection/clipboard context would
    /// otherwise enable it. This mirrors the dispatcher's persona gate
    /// ([`Persona::allows_intent_kind`]) so a lens pre-disables forbidden
    /// affordances rather than letting the user discover the typed `Forbidden`
    /// rejection after the fact. Governance is the stronger constraint, so a
    /// persona block replaces any context disabled-reason.
    #[must_use]
    pub fn governed_by(mut self, persona: Persona) -> Self {
        for entry in &mut self.entries {
            if !persona.allows_intent_kind(entry.intent_kind) {
                entry.enabled = false;
                entry.disabled_reason =
                    Some(format!("the {persona} persona cannot run this command"));
            }
        }
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandMetaProjection {
    pub intent_kind: CommandIntentKindProjection,
    pub title: &'static str,
    pub shortcut: Option<&'static str>,
    pub effective_binding: Option<&'static str>,
    pub enabled: bool,
    pub disabled_reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CommandIntentKindProjection {
    NewWorkspace,
    Recalculate,
    AddNode,
    RenameNode,
    MoveNode,
    ReorderNode,
    DeleteNode,
    EditContent,
    SetNumberFormat,
    SetNote,
    CopyValues,
    CopyFormula,
    CopyFormat,
    CopySubtree,
    PasteClipboardValues,
    PasteClipboardFormat,
    OpenCandidate,
    EvaluateCandidate,
    RebaseCandidate,
    CommitCandidate,
    DiscardCandidate,
    ReapCandidates,
    CreateScenario,
    ActivateScenario,
    DeleteScenario,
    CreateScenarioSweep,
    ActivateSweep,
    DeleteSweep,
    Undo,
    Redo,
    EditTableCell,
    AddTableRow,
    AddTableColumn,
}

impl CommandIntentKindProjection {
    #[must_use]
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::NewWorkspace => "new_workspace",
            Self::Recalculate => "recalculate",
            Self::AddNode => "add_node",
            Self::RenameNode => "rename_node",
            Self::MoveNode => "move_node",
            Self::ReorderNode => "reorder_node",
            Self::DeleteNode => "delete_node",
            Self::EditContent => "edit_content",
            Self::SetNumberFormat => "set_number_format",
            Self::SetNote => "set_note",
            Self::CopyValues => "copy_values",
            Self::CopyFormula => "copy_formula",
            Self::CopyFormat => "copy_format",
            Self::CopySubtree => "copy_subtree",
            Self::PasteClipboardValues => "paste_clipboard_values",
            Self::PasteClipboardFormat => "paste_clipboard_format",
            Self::OpenCandidate => "open_candidate",
            Self::EvaluateCandidate => "evaluate_candidate",
            Self::RebaseCandidate => "rebase_candidate",
            Self::CommitCandidate => "commit_candidate",
            Self::DiscardCandidate => "discard_candidate",
            Self::ReapCandidates => "reap_candidates",
            Self::CreateScenario => "create_scenario",
            Self::ActivateScenario => "activate_scenario",
            Self::DeleteScenario => "delete_scenario",
            Self::CreateScenarioSweep => "create_scenario_sweep",
            Self::ActivateSweep => "activate_sweep",
            Self::DeleteSweep => "delete_sweep",
            Self::Undo => "undo",
            Self::Redo => "redo",
            Self::EditTableCell => "edit_table_cell",
            Self::AddTableRow => "add_table_row",
            Self::AddTableColumn => "add_table_column",
        }
    }
}

impl WorkspaceState {
    #[must_use]
    pub fn command_catalog(&self, selection: &SelectionState) -> CommandCatalogProjection {
        let selected_node = selection
            .primary
            .as_ref()
            .and_then(|node_id| self.node(node_id));
        let selected_node_exists = selected_node.is_some();
        let selected_formula_node =
            selected_node.is_some_and(|node| node.content_kind == NodeContentKind::Formula);
        let selected_table_cell = selection
            .table_cell
            .as_ref()
            .is_some_and(|cell| self.tables.contains_key(&cell.table));
        let has_nodes = !self.nodes_by_key.is_empty();
        let has_candidates = !self.candidates.is_empty();
        let has_reclaimable_candidates = self.speculation_pressure.reclaimable_candidate_count > 0;
        let has_scenarios = !self.scenarios.entries.is_empty();
        let has_sweeps = !self.sweeps.entries.is_empty();
        let can_undo = self
            .current_revision_history_entry()
            .is_some_and(|entry| entry.parent_revision_id.is_some());
        let can_redo = self.revision_history.entries.iter().any(|entry| {
            !entry.is_current
                && entry.parent_revision_id.as_deref()
                    == self.revision_history.current_revision_id.as_deref()
        });

        let values_clipboard = matches!(
            self.clipboard.as_ref().map(|clipboard| &clipboard.payload),
            Some(ClipboardPayloadProjection::Values { .. })
        );
        let format_clipboard = matches!(
            self.clipboard.as_ref().map(|clipboard| &clipboard.payload),
            Some(ClipboardPayloadProjection::Format { .. })
        );

        let entries = vec![
            command(
                CommandIntentKindProjection::NewWorkspace,
                "New Workspace",
                Some("Ctrl+N"),
                true,
                None,
            ),
            command(
                CommandIntentKindProjection::Recalculate,
                "Recalculate",
                Some("F9"),
                true,
                None,
            ),
            command(
                CommandIntentKindProjection::AddNode,
                "Add Node",
                Some("A"),
                true,
                None,
            ),
            command(
                CommandIntentKindProjection::RenameNode,
                "Rename Node",
                // F2 edits content in place (SkinVerb::EditInPlace), not rename;
                // renaming is via the inspector field / inline label edit, so this
                // command advertises no universal shortcut rather than a stale F2.
                None,
                selected_node_exists,
                selected_node_reason(selected_node_exists),
            ),
            command(
                CommandIntentKindProjection::MoveNode,
                "Move Node",
                None,
                selected_node_exists,
                selected_node_reason(selected_node_exists),
            ),
            command(
                CommandIntentKindProjection::ReorderNode,
                "Reorder Node",
                None,
                selected_node_exists,
                selected_node_reason(selected_node_exists),
            ),
            command(
                CommandIntentKindProjection::DeleteNode,
                "Delete Node",
                // Structural removal is button-only and confirmed: the bare
                // Delete/Backspace keys clear contents (SkinVerb::ClearContents),
                // so this command must not advertise them as its shortcut.
                None,
                selected_node_exists,
                selected_node_reason(selected_node_exists),
            ),
            command(
                CommandIntentKindProjection::EditContent,
                "Edit Content",
                Some("Enter"),
                selected_node_exists,
                selected_node_reason(selected_node_exists),
            ),
            command(
                CommandIntentKindProjection::SetNumberFormat,
                "Set Number Format",
                None,
                selected_node_exists,
                selected_node_reason(selected_node_exists),
            ),
            command(
                CommandIntentKindProjection::SetNote,
                "Set Note",
                None,
                selected_node_exists,
                selected_node_reason(selected_node_exists),
            ),
            command(
                CommandIntentKindProjection::CopyValues,
                "Copy Values",
                Some("Ctrl+C"),
                selected_node_exists,
                selected_node_reason(selected_node_exists),
            ),
            command(
                CommandIntentKindProjection::CopyFormula,
                "Copy Formula",
                None,
                selected_formula_node,
                formula_node_reason(selected_node_exists, selected_formula_node),
            ),
            command(
                CommandIntentKindProjection::CopyFormat,
                "Copy Format",
                None,
                selected_node_exists,
                selected_node_reason(selected_node_exists),
            ),
            command(
                CommandIntentKindProjection::CopySubtree,
                "Copy Subtree",
                None,
                selected_node_exists,
                selected_node_reason(selected_node_exists),
            ),
            command(
                CommandIntentKindProjection::PasteClipboardValues,
                "Paste Values",
                Some("Ctrl+V"),
                selected_node_exists && values_clipboard,
                paste_reason(selected_node_exists, values_clipboard, "values"),
            ),
            command(
                CommandIntentKindProjection::PasteClipboardFormat,
                "Paste Format",
                None,
                selected_node_exists && format_clipboard,
                paste_reason(selected_node_exists, format_clipboard, "format"),
            ),
            command(
                CommandIntentKindProjection::OpenCandidate,
                "Open Candidate",
                None,
                has_nodes,
                model_reason(has_nodes),
            ),
            command(
                CommandIntentKindProjection::EvaluateCandidate,
                "Evaluate Candidate",
                None,
                has_candidates,
                candidate_reason(has_candidates),
            ),
            command(
                CommandIntentKindProjection::RebaseCandidate,
                "Rebase Candidate",
                None,
                has_candidates,
                candidate_reason(has_candidates),
            ),
            command(
                CommandIntentKindProjection::CommitCandidate,
                "Commit Candidate",
                None,
                has_candidates,
                candidate_reason(has_candidates),
            ),
            command(
                CommandIntentKindProjection::DiscardCandidate,
                "Discard Candidate",
                None,
                has_candidates,
                candidate_reason(has_candidates),
            ),
            command(
                CommandIntentKindProjection::ReapCandidates,
                "Reap Candidates",
                None,
                has_reclaimable_candidates,
                if has_reclaimable_candidates {
                    None
                } else {
                    Some("no reclaimable candidates".to_string())
                },
            ),
            command(
                CommandIntentKindProjection::CreateScenario,
                "Create Scenario",
                None,
                has_nodes,
                model_reason(has_nodes),
            ),
            command(
                CommandIntentKindProjection::ActivateScenario,
                "Activate Scenario",
                None,
                has_scenarios,
                scenario_reason(has_scenarios),
            ),
            command(
                CommandIntentKindProjection::DeleteScenario,
                "Delete Scenario",
                None,
                has_scenarios,
                scenario_reason(has_scenarios),
            ),
            command(
                CommandIntentKindProjection::CreateScenarioSweep,
                "Create Sweep",
                None,
                selected_node_exists,
                selected_node_reason(selected_node_exists),
            ),
            command(
                CommandIntentKindProjection::ActivateSweep,
                "Activate Sweep",
                None,
                has_sweeps,
                sweep_reason(has_sweeps),
            ),
            command(
                CommandIntentKindProjection::DeleteSweep,
                "Delete Sweep",
                None,
                has_sweeps,
                sweep_reason(has_sweeps),
            ),
            command(
                CommandIntentKindProjection::Undo,
                "Undo",
                Some("Ctrl+Z"),
                can_undo,
                if can_undo {
                    None
                } else {
                    Some("no parent revision to navigate to".to_string())
                },
            ),
            command(
                CommandIntentKindProjection::Redo,
                "Redo",
                Some("Ctrl+Y"),
                can_redo,
                if can_redo {
                    None
                } else {
                    Some("no redo revision is projected".to_string())
                },
            ),
            command(
                CommandIntentKindProjection::EditTableCell,
                "Edit Table Cell",
                Some("Enter"),
                selected_table_cell,
                table_cell_reason(selected_table_cell),
            ),
            command(
                CommandIntentKindProjection::AddTableRow,
                "Add Table Row",
                None,
                selected_table_cell,
                table_cell_reason(selected_table_cell),
            ),
            command(
                CommandIntentKindProjection::AddTableColumn,
                "Add Table Column",
                None,
                selected_table_cell,
                table_cell_reason(selected_table_cell),
            ),
        ];

        CommandCatalogProjection { entries }
    }

    fn current_revision_history_entry(&self) -> Option<&RevisionHistoryEntryProjection> {
        let current = self.revision_history.current_revision_id.as_deref()?;
        self.revision_history
            .entries
            .iter()
            .find(|entry| entry.revision_id == current)
    }
}

fn command(
    intent_kind: CommandIntentKindProjection,
    title: &'static str,
    shortcut: Option<&'static str>,
    enabled: bool,
    disabled_reason: Option<String>,
) -> CommandMetaProjection {
    CommandMetaProjection {
        intent_kind,
        title,
        shortcut,
        effective_binding: shortcut,
        enabled,
        disabled_reason,
    }
}

fn selected_node_reason(selected_node_exists: bool) -> Option<String> {
    if selected_node_exists {
        None
    } else {
        Some("no node is selected".to_string())
    }
}

fn formula_node_reason(selected_node_exists: bool, selected_formula_node: bool) -> Option<String> {
    if selected_formula_node {
        None
    } else if selected_node_exists {
        Some("selected node is not a formula".to_string())
    } else {
        Some("no node is selected".to_string())
    }
}

fn paste_reason(
    selected_node_exists: bool,
    has_payload: bool,
    expected_payload: &str,
) -> Option<String> {
    if selected_node_exists && has_payload {
        None
    } else if !selected_node_exists {
        Some("no node is selected".to_string())
    } else {
        Some(format!(
            "clipboard does not contain a {expected_payload} payload"
        ))
    }
}

fn model_reason(has_nodes: bool) -> Option<String> {
    if has_nodes {
        None
    } else {
        Some("workspace has no nodes".to_string())
    }
}

fn candidate_reason(has_candidates: bool) -> Option<String> {
    if has_candidates {
        None
    } else {
        Some("no candidate is available".to_string())
    }
}

fn scenario_reason(has_scenarios: bool) -> Option<String> {
    if has_scenarios {
        None
    } else {
        Some("no scenario is available".to_string())
    }
}

fn sweep_reason(has_sweeps: bool) -> Option<String> {
    if has_sweeps {
        None
    } else {
        Some("no sweep is available".to_string())
    }
}

fn table_cell_reason(selected_table_cell: bool) -> Option<String> {
    if selected_table_cell {
        None
    } else {
        Some("no table cell is selected".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enabled_entry(intent_kind: CommandIntentKindProjection) -> CommandMetaProjection {
        CommandMetaProjection {
            intent_kind,
            title: "Command",
            shortcut: None,
            effective_binding: None,
            enabled: true,
            disabled_reason: None,
        }
    }

    #[test]
    fn governed_by_disables_forbidden_commands_with_a_persona_reason() {
        let catalog = CommandCatalogProjection {
            entries: vec![
                enabled_entry(CommandIntentKindProjection::Recalculate),
                enabled_entry(CommandIntentKindProjection::CommitCandidate),
                enabled_entry(CommandIntentKindProjection::DeleteNode),
            ],
        };

        // Author governs nothing — every command stays as the context left it.
        assert!(
            catalog
                .clone()
                .governed_by(Persona::Author)
                .entries
                .iter()
                .all(|entry| entry.enabled)
        );

        // Reviewer keeps recalc but loses the publishing/mutating verbs, with
        // the persona reason replacing any context reason.
        let reviewer = catalog.clone().governed_by(Persona::Reviewer);
        assert!(
            reviewer
                .get(CommandIntentKindProjection::Recalculate)
                .unwrap()
                .enabled
        );
        let commit = reviewer
            .get(CommandIntentKindProjection::CommitCandidate)
            .unwrap();
        assert!(!commit.enabled);
        assert_eq!(
            commit.disabled_reason.as_deref(),
            Some("the reviewer persona cannot run this command")
        );

        // ReadOnly forbids every command in the catalog.
        assert!(
            catalog
                .governed_by(Persona::ReadOnly)
                .entries
                .iter()
                .all(|entry| !entry.enabled)
        );
    }
}
