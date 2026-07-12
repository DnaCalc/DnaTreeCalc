use std::collections::BTreeSet;

use crate::adapters::oxfml::EditorDocument;
use crate::ui::editor::render_projection::{syntax_runs_from_snapshot, SyntaxTokenRole};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionSemanticProfileRow {
    pub surface_name: String,
    pub function_id: String,
    pub semantic_kernel_metadata_version: String,
    pub reduction_sensitive: bool,
    pub error_collapse_sensitive: bool,
    pub numerical_reduction_policy: Option<String>,
    pub error_algebra: Option<String>,
    pub arg_admission_metadata_version: String,
    pub producer_capability_set_keys: Vec<String>,
}

pub fn project_function_semantic_profile(
    entered_text: &str,
    document: Option<&EditorDocument>,
) -> Vec<FunctionSemanticProfileRow> {
    let Some(document) = document else {
        return Vec::new();
    };
    if document.source_text != entered_text {
        return Vec::new();
    }

    let mut names = BTreeSet::new();
    for run in syntax_runs_from_snapshot(&document.editor_syntax_snapshot) {
        if run.role == SyntaxTokenRole::Function {
            names.insert(run.text.to_ascii_uppercase());
        }
    }

    let registry = oxfunc_core::registry::builtin_registry();
    names
        .into_iter()
        .filter_map(|name| registry.lookup_by_surface_name(&name))
        .map(|entry| FunctionSemanticProfileRow {
            surface_name: entry.surface_name.clone(),
            function_id: entry.meta.function_id.clone(),
            semantic_kernel_metadata_version: entry.meta.semantic_kernel_metadata_version.clone(),
            reduction_sensitive: entry.meta.semantic_kernel_metadata.reduction_sensitive,
            error_collapse_sensitive: entry.meta.semantic_kernel_metadata.error_collapse_sensitive,
            numerical_reduction_policy: entry
                .meta
                .semantic_kernel_metadata
                .numerical_reduction_policy
                .clone(),
            error_algebra: entry.meta.semantic_kernel_metadata.error_algebra.clone(),
            arg_admission_metadata_version: entry.meta.arg_admission_metadata_version.clone(),
            producer_capability_set_keys: entry.meta.producer_capability_set_keys.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{make_editor_syntax_snapshot, make_editor_token};

    #[test]
    fn sum_profile_reports_reduction_policy_and_error_algebra() {
        let mut document = crate::test_support::sample_editor_document("=SUM(1,2,3)");
        document.editor_syntax_snapshot = make_editor_syntax_snapshot(
            "formula-1",
            "green-1",
            vec![make_editor_token("=", 0), make_editor_token("SUM", 1)],
        );

        let rows = project_function_semantic_profile("=SUM(1,2,3)", Some(&document));

        let sum = rows
            .iter()
            .find(|row| row.surface_name == "SUM")
            .expect("SUM row");
        assert!(sum.reduction_sensitive);
        assert_eq!(
            sum.numerical_reduction_policy.as_deref(),
            Some("SequentialLeftFold")
        );
        assert_eq!(sum.error_algebra.as_deref(), Some("CanonicalExcelLegacy"));
    }
}
