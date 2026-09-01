#![forbid(unsafe_code)]

//! DNA TreeCalc host crate.
//!
//! Owns the workspace fixture/model projection and the host-side wiring that
//! turns direct OxCalc context state into the typed surfaces the skin framework
//! and shell consume (`app` module): workspace projection, host dispatcher,
//! and the default skin registry.

pub mod app;
pub mod model;

/// The host-core command surface [`app::WorkbookHostDispatcher`] wraps (W011,
/// dtc-j7n8.8), re-exported so a shell names it through this facade instead of
/// taking `dnacalc-host-core` as a normal dependency (the Calc app reaches
/// host-core THROUGH this crate by design). `LoadRecalcPath` is the typed
/// field of an `Opened` outcome a shell shows; `WorkbookSessionError` is the
/// payload of `HostCommandError::Workbook` and the error of the constructors.
pub use dnacalc_host_core::{
    HostCommand, HostCommandError, HostCommandOutcome, LoadRecalcPath, WorkbookSessionError,
};

#[cfg(test)]
mod tests {
    use crate::model::{NodeContentKind, WorkspaceFixture, WorkspaceModel, WorkspaceNodeFixture};

    #[test]
    fn loads_accounts_fixture_as_tree_workspace() {
        let fixture = WorkspaceFixture::from_repo_fixture("accounts").unwrap();
        let workspace = WorkspaceModel::try_from(fixture).unwrap();

        let root = workspace.node("Accounts").unwrap();
        assert_eq!(root.child_paths, vec!["Accounts.2005"]);

        let q1 = workspace.node("Accounts.2005.Q1").unwrap();
        assert_eq!(
            q1.child_paths,
            vec![
                "Accounts.2005.Q1.Income",
                "Accounts.2005.Q1.Margin",
                "Accounts.2005.Q1.Net"
            ]
        );

        let margin = workspace.node("Accounts.2005.Q1.Margin").unwrap();
        assert_eq!(margin.content.kind(), NodeContentKind::Constant);
        assert_eq!(margin.content.text(), "0.2");

        let income = workspace.node("Accounts.2005.Q1.Income").unwrap();
        assert_eq!(income.content.kind(), NodeContentKind::Formula);
        assert_eq!(income.content.text(), "=Sales*Margin");
    }

    #[test]
    fn loads_and_round_trips_table_node_fixture_metadata() {
        let fixture = WorkspaceFixture::from_repo_fixture("tables").unwrap();
        let serialized = serde_json::to_string_pretty(&fixture).unwrap();
        let reparsed: WorkspaceFixture = serde_json::from_str(&serialized).unwrap();
        assert_eq!(reparsed, fixture);

        let workspace = WorkspaceModel::try_from(fixture).unwrap();
        let table = workspace.table_node("SalesTable").unwrap();

        assert_eq!(table.table_id, "tree-table:sales");
        assert_eq!(table.display_path.as_deref(), Some("SalesTable"));
        assert_eq!(table.canonical_path.as_deref(), Some("SalesTable"));
        assert_eq!(
            table
                .rows
                .iter()
                .map(|row| row.row_id.as_str())
                .collect::<Vec<_>>(),
            vec!["row:west", "row:east", "row:north"]
        );
        assert_eq!(
            table
                .columns
                .iter()
                .map(|column| (
                    column.column_id.as_str(),
                    column.name.as_str(),
                    column.ordinal
                ))
                .collect::<Vec<_>>(),
            vec![
                ("col:region", "Region", 1),
                ("col:amount", "Amount", 2),
                ("col:tax", "Tax", 3),
            ]
        );
        assert_eq!(
            table.columns[2].body.formula.as_ref().unwrap().formula_text,
            "=[@Amount] * 0.1"
        );
        assert!(table.identity_policy.rename_preserves_table_id);
        assert!(table.identity_policy.move_preserves_table_id);
        assert!(table.identity_policy.delete_releases_table_id);
    }

    #[test]
    fn table_node_default_canonical_path_stays_dotted_for_nested_paths() {
        let source = WorkspaceFixture::from_repo_fixture("tables").unwrap();
        let mut table_node = source
            .nodes
            .into_iter()
            .find(|node| node.table.is_some())
            .expect("tables fixture has a table node");
        table_node.node_id = "Root.Section.Sales".to_string();
        if let Some(table) = table_node.table.as_mut() {
            table.display_path = None;
            table.canonical_path = None;
        }

        let workspace = WorkspaceModel::try_from(WorkspaceFixture {
            schema_version: "treecalc-workspace-v1".to_string(),
            workspace_id: "nested-table-default-paths".to_string(),
            description: None,
            profile: None,
            nodes: vec![
                WorkspaceNodeFixture {
                    node_id: "Root".to_string(),
                    formula: String::new(),
                    is_meta: false,
                    table: None,
                },
                WorkspaceNodeFixture {
                    node_id: "Root.Section".to_string(),
                    formula: String::new(),
                    is_meta: false,
                    table: None,
                },
                table_node,
            ],
        })
        .unwrap();

        let table = workspace.table_node("Root.Section.Sales").unwrap();
        assert_eq!(table.display_path.as_deref(), Some("Root.Section.Sales"));
        assert_eq!(table.canonical_path.as_deref(), Some("Root.Section.Sales"));
    }

    #[test]
    fn catalog_workspace_names_filter_to_live_ids_and_default_when_absent() {
        use crate::app::WorkspaceDocumentCatalog;

        // A pre-rename `v1` catalog (no workspace_names field) loads with an
        // empty map — adding the field stays backward compatible.
        let legacy = r#"{"schema_version":"dnatreecalc-workspace-catalog-v1","workspace_ids":["ws-1"],"active_workspace_id":"ws-1"}"#;
        let parsed: WorkspaceDocumentCatalog = serde_json::from_str(legacy).unwrap();
        assert!(parsed.workspace_names.is_empty());

        // with_workspace_names keeps only entries for ids still present.
        let mut names = std::collections::BTreeMap::new();
        names.insert("ws-1".to_string(), "Quarterly model".to_string());
        names.insert("ws-gone".to_string(), "Deleted".to_string());
        let catalog =
            WorkspaceDocumentCatalog::new(vec!["ws-1".to_string()], Some("ws-1".to_string()))
                .with_workspace_names(&names);
        assert_eq!(catalog.workspace_names.len(), 1);
        assert_eq!(
            catalog.workspace_names.get("ws-1").map(String::as_str),
            Some("Quarterly model")
        );

        // Round-trips through JSON.
        let json = serde_json::to_string(&catalog).unwrap();
        let reparsed: WorkspaceDocumentCatalog = serde_json::from_str(&json).unwrap();
        assert_eq!(reparsed, catalog);
    }
}
