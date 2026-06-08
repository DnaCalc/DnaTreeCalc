#![allow(dead_code)]

use std::sync::{Arc, Mutex};

use dnatreecalc_host::app::{HostDispatcher, TreeWorkspaceSession};
use dnatreecalc_host::model::{WorkspaceFixture, WorkspaceModel};
use dnatreecalc_skin_framework::{
    ActiveNodeDetailProjection, Dispatcher, ErasedSkinContext, IntentReceipt, NodeId,
    RegisteredSkin, SelectionState, SharedSkinState, SharedSkinStateHandle, SkinCapabilities,
    SkinCategory, SkinContext, SkinHandle, SkinId, SkinManifest, SkinState, TableCellInput,
    TableRowInput, WorkspaceIntent, WorkspaceRecalcMode, WorkspaceRevisionProjection,
    WorkspaceSkin, WorkspaceState,
};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

const PROGRAMMABLE_SKIN_ID: SkinId = SkinId::new("programmable-test-skin");

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct ProgrammableSkinState {
    command_count: usize,
}

impl SkinState for ProgrammableSkinState {
    fn schema_version() -> u32 {
        1
    }
}

#[derive(Clone)]
pub struct ProgrammableDriver {
    workspace: ReadSignal<WorkspaceState>,
    selection: ReadSignal<SelectionState>,
    shared: SharedSkinStateHandle,
    dispatch: Arc<dyn Dispatcher>,
}

impl ProgrammableDriver {
    pub fn add_node(&self, parent: Option<&str>, symbol: &str, content: &str) {
        self.accept(self.add_node_intent(parent, symbol, content));
    }

    pub fn try_add_node(&self, parent: Option<&str>, symbol: &str, content: &str) -> IntentReceipt {
        self.dispatch
            .dispatch(self.add_node_intent(parent, symbol, content))
    }

    pub fn edit(&self, node: &str, content: &str) {
        self.accept(WorkspaceIntent::EditContent {
            node: NodeId::new(node),
            content: content.to_string(),
        });
    }

    pub fn try_edit(&self, node: &str, content: &str) -> IntentReceipt {
        self.dispatch.dispatch(WorkspaceIntent::EditContent {
            node: NodeId::new(node),
            content: content.to_string(),
        })
    }

    pub fn edit_deferred(&self, node: &str, content: &str) {
        self.accept(WorkspaceIntent::EditContentDeferred {
            node: NodeId::new(node),
            content: content.to_string(),
        });
    }

    pub fn recalc(&self) {
        self.accept(WorkspaceIntent::Recalculate);
    }

    pub fn try_recalc(&self) -> IntentReceipt {
        self.dispatch.dispatch(WorkspaceIntent::Recalculate)
    }

    pub fn rename(&self, node: &str, new_symbol: &str) {
        self.accept(WorkspaceIntent::RenameNode {
            node: NodeId::new(node),
            new_symbol: new_symbol.to_string(),
        });
    }

    pub fn try_rename(&self, node: &str, new_symbol: &str) -> IntentReceipt {
        self.dispatch.dispatch(WorkspaceIntent::RenameNode {
            node: NodeId::new(node),
            new_symbol: new_symbol.to_string(),
        })
    }

    pub fn move_node(&self, node: &str, new_parent: Option<&str>, new_index: Option<usize>) {
        self.accept(WorkspaceIntent::MoveNode {
            node: NodeId::new(node),
            new_parent: new_parent.map(NodeId::new),
            new_index,
        });
    }

    pub fn try_move_node(
        &self,
        node: &str,
        new_parent: Option<&str>,
        new_index: Option<usize>,
    ) -> IntentReceipt {
        self.dispatch.dispatch(WorkspaceIntent::MoveNode {
            node: NodeId::new(node),
            new_parent: new_parent.map(NodeId::new),
            new_index,
        })
    }

    pub fn reorder(&self, node: &str, new_index: usize) {
        self.accept(WorkspaceIntent::ReorderNode {
            node: NodeId::new(node),
            new_index,
        });
    }

    pub fn try_reorder(&self, node: &str, new_index: usize) -> IntentReceipt {
        self.dispatch.dispatch(WorkspaceIntent::ReorderNode {
            node: NodeId::new(node),
            new_index,
        })
    }

    pub fn delete(&self, node: &str) {
        self.accept(WorkspaceIntent::DeleteNode {
            node: NodeId::new(node),
        });
    }

    pub fn try_delete(&self, node: &str) -> IntentReceipt {
        self.dispatch.dispatch(WorkspaceIntent::DeleteNode {
            node: NodeId::new(node),
        })
    }

    pub fn edit_table_cell(&self, table: &str, row_id: &str, column_id: &str, content: &str) {
        self.accept(WorkspaceIntent::EditTableCell {
            table: NodeId::new(table),
            row_id: row_id.to_string(),
            column_id: column_id.to_string(),
            content: content.to_string(),
        });
    }

    pub fn try_edit_table_cell(
        &self,
        table: &str,
        row_id: &str,
        column_id: &str,
        content: &str,
    ) -> IntentReceipt {
        self.dispatch.dispatch(WorkspaceIntent::EditTableCell {
            table: NodeId::new(table),
            row_id: row_id.to_string(),
            column_id: column_id.to_string(),
            content: content.to_string(),
        })
    }

    pub fn add_table_row(&self, table: &str, row_id: &str, values: &[(&str, &str)]) {
        self.accept(self.add_table_row_intent(table, row_id, values));
    }

    pub fn try_add_table_row(
        &self,
        table: &str,
        row_id: &str,
        values: &[(&str, &str)],
    ) -> IntentReceipt {
        self.dispatch
            .dispatch(self.add_table_row_intent(table, row_id, values))
    }

    pub fn delete_table_row(&self, table: &str, row_id: &str) {
        self.accept(WorkspaceIntent::DeleteTableRow {
            table: NodeId::new(table),
            row_id: row_id.to_string(),
        });
    }

    pub fn try_delete_table_row(&self, table: &str, row_id: &str) -> IntentReceipt {
        self.dispatch.dispatch(WorkspaceIntent::DeleteTableRow {
            table: NodeId::new(table),
            row_id: row_id.to_string(),
        })
    }

    pub fn rename_table_row(&self, table: &str, row_id: &str, new_row_id: &str) {
        self.accept(WorkspaceIntent::RenameTableRow {
            table: NodeId::new(table),
            row_id: row_id.to_string(),
            new_row_id: new_row_id.to_string(),
        });
    }

    pub fn try_rename_table_row(
        &self,
        table: &str,
        row_id: &str,
        new_row_id: &str,
    ) -> IntentReceipt {
        self.dispatch.dispatch(WorkspaceIntent::RenameTableRow {
            table: NodeId::new(table),
            row_id: row_id.to_string(),
            new_row_id: new_row_id.to_string(),
        })
    }

    pub fn reorder_table_row(&self, table: &str, row_id: &str, new_index: usize) {
        self.accept(WorkspaceIntent::ReorderTableRow {
            table: NodeId::new(table),
            row_id: row_id.to_string(),
            new_index,
        });
    }

    pub fn try_reorder_table_row(
        &self,
        table: &str,
        row_id: &str,
        new_index: usize,
    ) -> IntentReceipt {
        self.dispatch.dispatch(WorkspaceIntent::ReorderTableRow {
            table: NodeId::new(table),
            row_id: row_id.to_string(),
            new_index,
        })
    }

    pub fn add_table_column(
        &self,
        table: &str,
        column_id: &str,
        name: &str,
        values: &[(&str, &str)],
    ) {
        self.accept(self.add_table_column_intent(table, column_id, name, values));
    }

    pub fn try_add_table_column(
        &self,
        table: &str,
        column_id: &str,
        name: &str,
        values: &[(&str, &str)],
    ) -> IntentReceipt {
        self.dispatch
            .dispatch(self.add_table_column_intent(table, column_id, name, values))
    }

    pub fn add_table_formula_column(
        &self,
        table: &str,
        column_id: &str,
        name: &str,
        formula_text: &str,
    ) {
        self.accept(WorkspaceIntent::AddTableFormulaColumn {
            table: NodeId::new(table),
            column_id: column_id.to_string(),
            name: name.to_string(),
            formula_text: formula_text.to_string(),
        });
    }

    pub fn try_add_table_formula_column(
        &self,
        table: &str,
        column_id: &str,
        name: &str,
        formula_text: &str,
    ) -> IntentReceipt {
        self.dispatch
            .dispatch(WorkspaceIntent::AddTableFormulaColumn {
                table: NodeId::new(table),
                column_id: column_id.to_string(),
                name: name.to_string(),
                formula_text: formula_text.to_string(),
            })
    }

    pub fn edit_table_column_formula(&self, table: &str, column_id: &str, formula_text: &str) {
        self.accept(WorkspaceIntent::EditTableColumnFormula {
            table: NodeId::new(table),
            column_id: column_id.to_string(),
            formula_text: formula_text.to_string(),
        });
    }

    pub fn try_edit_table_column_formula(
        &self,
        table: &str,
        column_id: &str,
        formula_text: &str,
    ) -> IntentReceipt {
        self.dispatch
            .dispatch(WorkspaceIntent::EditTableColumnFormula {
                table: NodeId::new(table),
                column_id: column_id.to_string(),
                formula_text: formula_text.to_string(),
            })
    }

    pub fn set_table_totals_formula(&self, table: &str, column_id: &str, formula_text: &str) {
        self.accept(WorkspaceIntent::SetTableTotalsFormula {
            table: NodeId::new(table),
            column_id: column_id.to_string(),
            formula_text: formula_text.to_string(),
        });
    }

    pub fn try_set_table_totals_formula(
        &self,
        table: &str,
        column_id: &str,
        formula_text: &str,
    ) -> IntentReceipt {
        self.dispatch
            .dispatch(WorkspaceIntent::SetTableTotalsFormula {
                table: NodeId::new(table),
                column_id: column_id.to_string(),
                formula_text: formula_text.to_string(),
            })
    }

    pub fn clear_table_totals_formula(&self, table: &str, column_id: &str) {
        self.accept(WorkspaceIntent::ClearTableTotalsFormula {
            table: NodeId::new(table),
            column_id: column_id.to_string(),
        });
    }

    pub fn try_clear_table_totals_formula(&self, table: &str, column_id: &str) -> IntentReceipt {
        self.dispatch
            .dispatch(WorkspaceIntent::ClearTableTotalsFormula {
                table: NodeId::new(table),
                column_id: column_id.to_string(),
            })
    }

    pub fn set_table_header_row_visible(&self, table: &str, visible: bool) {
        self.accept(WorkspaceIntent::SetTableHeaderRowVisible {
            table: NodeId::new(table),
            visible,
        });
    }

    pub fn try_set_table_header_row_visible(&self, table: &str, visible: bool) -> IntentReceipt {
        self.dispatch
            .dispatch(WorkspaceIntent::SetTableHeaderRowVisible {
                table: NodeId::new(table),
                visible,
            })
    }

    pub fn set_table_totals_row_visible(&self, table: &str, visible: bool) {
        self.accept(WorkspaceIntent::SetTableTotalsRowVisible {
            table: NodeId::new(table),
            visible,
        });
    }

    pub fn try_set_table_totals_row_visible(&self, table: &str, visible: bool) -> IntentReceipt {
        self.dispatch
            .dispatch(WorkspaceIntent::SetTableTotalsRowVisible {
                table: NodeId::new(table),
                visible,
            })
    }

    pub fn rename_table_column(&self, table: &str, column_id: &str, name: &str) {
        self.accept(WorkspaceIntent::RenameTableColumn {
            table: NodeId::new(table),
            column_id: column_id.to_string(),
            name: name.to_string(),
        });
    }

    pub fn try_rename_table_column(
        &self,
        table: &str,
        column_id: &str,
        name: &str,
    ) -> IntentReceipt {
        self.dispatch.dispatch(WorkspaceIntent::RenameTableColumn {
            table: NodeId::new(table),
            column_id: column_id.to_string(),
            name: name.to_string(),
        })
    }

    pub fn reorder_table_column(&self, table: &str, column_id: &str, new_index: usize) {
        self.accept(WorkspaceIntent::ReorderTableColumn {
            table: NodeId::new(table),
            column_id: column_id.to_string(),
            new_index,
        });
    }

    pub fn try_reorder_table_column(
        &self,
        table: &str,
        column_id: &str,
        new_index: usize,
    ) -> IntentReceipt {
        self.dispatch.dispatch(WorkspaceIntent::ReorderTableColumn {
            table: NodeId::new(table),
            column_id: column_id.to_string(),
            new_index,
        })
    }

    pub fn delete_table_column(&self, table: &str, column_id: &str) {
        self.accept(WorkspaceIntent::DeleteTableColumn {
            table: NodeId::new(table),
            column_id: column_id.to_string(),
        });
    }

    pub fn try_delete_table_column(&self, table: &str, column_id: &str) -> IntentReceipt {
        self.dispatch.dispatch(WorkspaceIntent::DeleteTableColumn {
            table: NodeId::new(table),
            column_id: column_id.to_string(),
        })
    }

    pub fn try_new_workspace(&self) -> IntentReceipt {
        self.dispatch.dispatch(WorkspaceIntent::NewWorkspace)
    }

    pub fn try_switch_workspace(&self, workspace_id: &str) -> IntentReceipt {
        self.dispatch.dispatch(WorkspaceIntent::SwitchWorkspace {
            workspace_id: workspace_id.to_string(),
        })
    }

    pub fn select(&self, node: Option<&str>) {
        self.accept(WorkspaceIntent::SelectNode(node.map(NodeId::new)));
    }

    pub fn try_select(&self, node: Option<&str>) -> IntentReceipt {
        self.dispatch
            .dispatch(WorkspaceIntent::SelectNode(node.map(NodeId::new)))
    }

    pub fn collapse(&self, node: &str) {
        let node = NodeId::new(node);
        self.shared.update(|state| {
            state.tree_collapsed.insert(node);
        });
    }

    pub fn pin(&self, node: &str) {
        let node = NodeId::new(node);
        self.shared.update(|state| {
            if !state.pinned.contains(&node) {
                state.pinned.push(node);
            }
        });
    }

    pub fn state(&self) -> WorkspaceState {
        self.workspace.get_untracked()
    }

    pub fn scalar(&self, node: &str) -> Option<String> {
        scalar_value(&self.state(), node).map(str::to_string)
    }

    pub fn selected(&self) -> Option<String> {
        self.selection
            .get_untracked()
            .primary
            .map(|node| node.as_str().to_string())
    }

    pub fn active_detail(&self) -> Option<ActiveNodeDetailProjection> {
        self.workspace
            .get_untracked()
            .active_node_detail(&self.selection.get_untracked())
    }

    pub fn set_recalc_mode(&self, mode: WorkspaceRecalcMode) {
        self.shared.update(|state| {
            state.recalc_mode = mode;
            if mode == WorkspaceRecalcMode::Auto {
                state.manual_recalc_pending = false;
            }
        });
    }

    pub fn recalc_mode(&self) -> WorkspaceRecalcMode {
        self.shared.get_untracked().recalc_mode
    }

    pub fn set_manual_recalc_pending(&self, pending: bool) {
        self.shared.update(|state| {
            state.manual_recalc_pending = pending;
        });
    }

    pub fn manual_recalc_pending(&self) -> bool {
        self.shared.get_untracked().manual_recalc_pending
    }

    pub fn outgoing_count(&self, node: &str) -> usize {
        self.state().dependencies.outgoing_count(&NodeId::new(node))
    }

    pub fn incoming_count(&self, node: &str) -> usize {
        self.state().dependencies.incoming_count(&NodeId::new(node))
    }

    pub fn assert_scalar(&self, node: &str, expected: &str) {
        assert_eq!(self.scalar(node).as_deref(), Some(expected), "{node}");
    }

    pub fn assert_children(&self, node: &str, expected: &[&str]) {
        let state = self.state();
        let actual = state
            .node(&NodeId::new(node))
            .unwrap_or_else(|| panic!("{node} must project"))
            .children
            .iter()
            .map(NodeId::as_str)
            .collect::<Vec<_>>();
        assert_eq!(actual, expected, "{node} children");
    }

    fn add_node_intent(
        &self,
        parent: Option<&str>,
        symbol: &str,
        content: &str,
    ) -> WorkspaceIntent {
        WorkspaceIntent::AddNode {
            parent: parent.map(NodeId::new),
            symbol: symbol.to_string(),
            content: content.to_string(),
        }
    }

    fn add_table_row_intent(
        &self,
        table: &str,
        row_id: &str,
        values: &[(&str, &str)],
    ) -> WorkspaceIntent {
        WorkspaceIntent::AddTableRow {
            table: NodeId::new(table),
            row_id: row_id.to_string(),
            values: values
                .iter()
                .map(|(column_id, content)| TableCellInput {
                    column_id: (*column_id).to_string(),
                    content: (*content).to_string(),
                })
                .collect(),
        }
    }

    fn add_table_column_intent(
        &self,
        table: &str,
        column_id: &str,
        name: &str,
        values: &[(&str, &str)],
    ) -> WorkspaceIntent {
        WorkspaceIntent::AddTableColumn {
            table: NodeId::new(table),
            column_id: column_id.to_string(),
            name: name.to_string(),
            values: values
                .iter()
                .map(|(row_id, content)| TableRowInput {
                    row_id: (*row_id).to_string(),
                    content: (*content).to_string(),
                })
                .collect(),
        }
    }

    fn accept(&self, intent: WorkspaceIntent) {
        let receipt = self.dispatch.dispatch(intent);
        assert!(receipt.accepted, "{:?}", receipt.error);
    }
}

#[derive(Clone)]
struct ProgrammableSkin {
    mounted: Arc<Mutex<Option<ProgrammableDriver>>>,
}

impl WorkspaceSkin for ProgrammableSkin {
    type State = ProgrammableSkinState;

    fn id(&self) -> SkinId {
        PROGRAMMABLE_SKIN_ID
    }

    fn manifest(&self) -> SkinManifest {
        SkinManifest {
            display_name: "Programmable test skin",
            description: "Test-only skin exposing the skin IR as a Rust command DSL.",
            category: SkinCategory::Inspector,
            version: "0.1.0",
        }
    }

    fn capabilities(&self) -> SkinCapabilities {
        SkinCapabilities {
            supports_multi_select: false,
            supports_inline_formula_edit: true,
            supports_meta_node_display: true,
            renders_arrays_inline: true,
            renders_table_values: true,
        }
    }

    fn mount(&self, cx: SkinContext<Self::State>) -> SkinHandle {
        *self
            .mounted
            .lock()
            .expect("programmable skin lock poisoned") = Some(ProgrammableDriver {
            workspace: cx.workspace,
            selection: cx.selection,
            shared: cx.shared,
            dispatch: cx.dispatch,
        });
        SkinHandle::new(view! { <div class="dtc-programmable-test-skin"></div> }.into_any())
    }
}

pub struct Harness {
    pub driver: ProgrammableDriver,
    pub session: Arc<Mutex<TreeWorkspaceSession>>,
    _owner: Owner,
}

impl Harness {
    pub fn empty() -> Self {
        Self::from_fixture(WorkspaceFixture {
            schema_version: "treecalc-workspace-v1".to_string(),
            workspace_id: "programmable-skin-ir".to_string(),
            description: None,
            profile: None,
            nodes: Vec::new(),
        })
    }

    pub fn from_repo_fixture(name: &str) -> Self {
        Self::from_fixture(WorkspaceFixture::from_repo_fixture(name).unwrap())
    }

    pub fn from_fixture(fixture: WorkspaceFixture) -> Self {
        let owner = Owner::new();
        let model = WorkspaceModel::try_from(fixture).unwrap();
        let session = Arc::new(Mutex::new(
            TreeWorkspaceSession::from_model(&model).unwrap(),
        ));
        let workspace = RwSignal::new(session.lock().unwrap().workspace_state().unwrap());
        let selection = RwSignal::new(SelectionState::default());
        let shared = SharedSkinStateHandle::new(SharedSkinState::default());
        let dispatcher = Arc::new(HostDispatcher::with_session(
            selection,
            workspace,
            session.clone(),
        ));
        let dispatch: Arc<dyn Dispatcher> = dispatcher;
        let mounted = Arc::new(Mutex::new(None));
        let skin = ProgrammableSkin {
            mounted: mounted.clone(),
        };
        let registered = RegisteredSkin::from_skin(skin);
        let handle = registered.mount(ErasedSkinContext {
            workspace: workspace.read_only(),
            selection: selection.read_only(),
            shared,
            dispatch,
        });
        drop(handle);
        let driver = mounted
            .lock()
            .expect("programmable skin lock poisoned")
            .clone()
            .expect("programmable skin mounted");
        Self {
            driver,
            session,
            _owner: owner,
        }
    }

    pub fn recalc_count(&self) -> usize {
        self.session.lock().unwrap().recalc_count()
    }
}

pub fn scalar_value<'a>(state: &'a WorkspaceState, node_id: &str) -> Option<&'a str> {
    state
        .node(&NodeId::new(node_id))
        .and_then(|node| node.computed_value.scalar_display_text())
}

pub fn revision_fingerprint(revision: &WorkspaceRevisionProjection) -> Vec<Option<String>> {
    vec![
        revision.structural_snapshot_id.clone(),
        revision.workspace_revision_id.clone(),
        revision.node_input_snapshot_id.clone(),
        revision.namespace_snapshot_id.clone(),
        revision.formula_binding_snapshot_id.clone(),
        revision.dependency_shape_snapshot_id.clone(),
        revision.publication_snapshot_id.clone(),
        revision.runtime_overlay_set_id.clone(),
        Some(revision.value_epoch.to_string()),
    ]
}
