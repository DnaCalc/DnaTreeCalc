use std::sync::Arc;

use crate::adapters::oxfml::NativeOxfmlHostSession;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostMountTarget {
    DesktopTauri,
    WebBrowser,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostBootstrapSpec {
    pub target: HostMountTarget,
    pub mount_element_id: &'static str,
    pub document_title: &'static str,
}

pub fn bootstrap_spec(target: HostMountTarget) -> HostBootstrapSpec {
    HostBootstrapSpec {
        target,
        mount_element_id: "onecalc-root",
        document_title: "DNA OneCalc",
    }
}

pub fn bootstrap_editor_bridge(_target: HostMountTarget) -> Arc<NativeOxfmlHostSession> {
    Arc::new(NativeOxfmlHostSession::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::oxfml::OxfmlHostSession;

    #[test]
    fn bootstrap_editor_bridge_is_available_for_desktop_and_web() {
        let desktop = bootstrap_editor_bridge(HostMountTarget::DesktopTauri);
        let web = bootstrap_editor_bridge(HostMountTarget::WebBrowser);

        assert!(desktop
            .apply_formula_edit(crate::adapters::oxfml::FormulaEditRequest {
                formula_stable_id: "formula-1".to_string(),
                entered_text: "=SUM(1,2)".to_string(),
                cursor_offset: 8,
                previous_green_tree_key: None,
                analysis_stage: crate::adapters::oxfml::EditorAnalysisStage::SyntaxAndBind,
                formatting_request: None,
                scenario_policy: crate::adapters::oxfml::ScenarioPolicyRequest::Deterministic,
                skip_runtime_evaluation: false,
                recalc_mode: crate::adapters::oxfml::RecalcModeRequest::Auto,
                language_tag: "en-US".to_string(),
                formal_input_bindings: Vec::new(),
                trace_mode: crate::adapters::oxfml::TraceModeRequest::ValueOnly,
            })
            .is_ok());
        assert!(web
            .apply_formula_edit(crate::adapters::oxfml::FormulaEditRequest {
                formula_stable_id: "formula-1".to_string(),
                entered_text: "=SUM(1,2)".to_string(),
                cursor_offset: 8,
                previous_green_tree_key: None,
                analysis_stage: crate::adapters::oxfml::EditorAnalysisStage::SyntaxAndBind,
                formatting_request: None,
                scenario_policy: crate::adapters::oxfml::ScenarioPolicyRequest::Deterministic,
                skip_runtime_evaluation: false,
                recalc_mode: crate::adapters::oxfml::RecalcModeRequest::Auto,
                language_tag: "en-US".to_string(),
                formal_input_bindings: Vec::new(),
                trace_mode: crate::adapters::oxfml::TraceModeRequest::ValueOnly,
            })
            .is_ok());
    }

    #[test]
    fn bootstrap_spec_carries_canonical_mount_id_and_title() {
        let spec = bootstrap_spec(HostMountTarget::DesktopTauri);
        assert_eq!(spec.mount_element_id, "onecalc-root");
        assert_eq!(spec.document_title, "DNA OneCalc");
        assert_eq!(spec.target, HostMountTarget::DesktopTauri);
    }
}
