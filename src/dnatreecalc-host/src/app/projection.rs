use std::collections::BTreeMap;

use dnatreecalc_skin_framework::{
    DependencyGraphProjection, NodeContentKind as FrameworkContentKind, NodeId, NodeKey,
    NodeValueProjection, NodeView, WorkspaceRevisionProjection, WorkspaceState,
};

use crate::model::{NodeContent, WorkspaceModel};

/// Project the host's `WorkspaceModel` (product model: structure +
/// per-node content text + meta flags) into the framework's
/// `WorkspaceState` (read-only observation surface skins consume).
///
/// The pure fixture projection leaves `computed_value` as
/// [`NodeValueProjection::Unevaluated`]. Product paths that need calculation
/// use [`crate::app::TreeWorkspaceSession`] so formula text, references, values,
/// tables, and diagnostics come from OxCalc.
#[must_use]
pub fn workspace_state_from_model(model: &WorkspaceModel) -> WorkspaceState {
    let mut nodes = BTreeMap::new();

    for (path, node) in &model.nodes {
        let id = NodeId::new(path.clone());
        let parent = node.parent_path.as_ref().map(|p| NodeId::new(p.clone()));
        let children = node
            .child_paths
            .iter()
            .map(|p| NodeId::new(p.clone()))
            .collect();
        let (content_kind, content_text) = match &node.content {
            NodeContent::Empty => (FrameworkContentKind::Empty, String::new()),
            NodeContent::Constant(text) => (FrameworkContentKind::Constant, text.clone()),
            NodeContent::Formula(text) => (FrameworkContentKind::Formula, text.clone()),
        };

        let view = NodeView {
            key: NodeKey::new(format!("model-path:{path}")),
            id: id.clone(),
            display_name: node.name.clone(),
            parent,
            children,
            depth: depth_of(path),
            content_kind,
            content_text,
            computed_value: NodeValueProjection::Unevaluated,
            calc_state: None,
            effective_format: None,
            binding_diagnostics: Vec::new(),
            is_meta: node.is_meta,
            table: None,
        };

        nodes.insert(id, view);
    }

    let paths_by_key = nodes
        .iter()
        .map(|(node_id, view)| (view.key.clone(), node_id.clone()))
        .collect::<BTreeMap<_, _>>();
    let nodes_by_key = nodes
        .values()
        .map(|view| (view.key.clone(), view.clone()))
        .collect::<BTreeMap<_, _>>();
    let root_keys = model
        .root_paths
        .iter()
        .filter_map(|path| {
            paths_by_key
                .iter()
                .find_map(|(key, node_id)| (node_id.as_str() == path).then(|| key.clone()))
        })
        .collect();

    WorkspaceState {
        workspace_id: model.workspace_id.clone(),
        profile: model.profile.as_str(),
        projection_seq: 0,
        revision: WorkspaceRevisionProjection::default(),
        last_run: None,
        node_order: model
            .node_order
            .iter()
            .map(|p| NodeId::new(p.clone()))
            .collect(),
        key_order: model
            .node_order
            .iter()
            .map(|p| NodeKey::new(format!("model-path:{p}")))
            .collect(),
        paths_by_key,
        root_keys,
        root_paths: model
            .root_paths
            .iter()
            .map(|p| NodeId::new(p.clone()))
            .collect(),
        nodes_by_key,
        nodes,
        dependencies: DependencyGraphProjection::default(),
        tables: BTreeMap::new(),
        diagnostics: Vec::new(),
    }
}

fn depth_of(path: &str) -> u32 {
    u32::try_from(path.matches('.').count()).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::WorkspaceFixture;

    #[test]
    fn projects_accounts_fixture_with_depths_and_content() {
        let fixture = WorkspaceFixture::from_repo_fixture("accounts").unwrap();
        let model = WorkspaceModel::try_from(fixture).unwrap();
        let state = workspace_state_from_model(&model);

        assert_eq!(state.workspace_id, "accounts");
        assert_eq!(state.profile, "treecalc-v1");
        assert!(state.node_order.len() >= 5);

        let root = state
            .node(&NodeId::new("Accounts"))
            .expect("root must project");
        assert_eq!(root.depth, 0);
        assert!(root.parent.is_none());

        let income = state
            .node(&NodeId::new("Accounts.2005.Q1.Income"))
            .expect("nested node must project");
        assert_eq!(income.depth, 3);
        assert_eq!(income.content_kind, FrameworkContentKind::Formula);
        assert_eq!(income.content_text, "=Sales*Margin");
        let income_key = income.key.clone();
        assert_eq!(
            state.node_id_for_key(&income_key),
            Some(&NodeId::new("Accounts.2005.Q1.Income"))
        );
        assert_eq!(
            state.node_by_key(&income_key).map(|node| &node.id),
            Some(&NodeId::new("Accounts.2005.Q1.Income"))
        );
    }
}
