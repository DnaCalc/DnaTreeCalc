use serde::Serialize;

use crate::adapters::oxfml::{FormulaDrillNodeState, NativeOxfmlHostSession};
use crate::app::case_lifecycle::new_formula_space;
use crate::services::home_shell_view_model::{
    build_home_shell_view_model, FormulaDrillNode, FormulaDrillPhaseChip, FormulaDrillPhaseState,
    HomeShellViewModel, ValueCapabilityFactKind,
};
use crate::services::live_edit::apply_live_editor_input;
use crate::state::{OneCalcHostState, ViewMode};
use crate::ui::editor::commands::{EditorInputEvent, EditorInputKind};

pub const DEFAULT_FORMULA_DRILL_AUDIT_CASES: &[(&str, &str)] = &[
    ("sum_literals", "=SUM(1,2,3)"),
    ("nested_if_sum", "=SUM(IF(TRUE,2,3),4)"),
    ("if_skipped_branch", "=IF(FALSE,SUM(1,2),SUM(3,4))"),
    ("let_reuse", "=LET(x,1,y,2,SUM(x,y))"),
    ("array_sequence", "=SEQUENCE(2,2)"),
    ("error_div_zero", "=1/0"),
    ("incomplete_sum", "=SUM("),
];

#[derive(Debug, Clone, Serialize)]
pub struct FormulaDrillAuditReport {
    pub schema_id: &'static str,
    pub cases: Vec<FormulaDrillAuditCase>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FormulaDrillAuditCase {
    pub case_id: String,
    pub formula: String,
    pub bridge_status: String,
    pub result_summary: String,
    pub user_display: FormulaDrillDisplaySnapshot,
    pub developer_display: FormulaDrillDisplaySnapshot,
    pub metrics: FormulaDrillAuditMetrics,
    pub assessment: FormulaDrillCaseAssessment,
}

#[derive(Debug, Clone, Serialize)]
pub struct FormulaDrillDisplaySnapshot {
    pub mode: &'static str,
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct FormulaDrillAuditMetrics {
    pub top_level_rows: usize,
    pub total_rows: usize,
    pub max_depth: usize,
    pub function_rows: usize,
    pub argument_rows: usize,
    pub diagnostic_rows: usize,
    pub phase_rows: usize,
    pub capability_rows: usize,
    pub missing_value_rows: usize,
    pub debug_fallback_rows: usize,
    pub top_level_function_rows: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct FormulaDrillCaseAssessment {
    pub user_usefulness: FormulaDrillUsefulness,
    pub key_gaps: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum FormulaDrillUsefulness {
    UsefulForSimpleFormulas,
    PartialForNestedFormulas,
    InsufficientForDebugging,
}

pub fn build_default_formula_drill_audit_report() -> FormulaDrillAuditReport {
    build_formula_drill_audit_report(DEFAULT_FORMULA_DRILL_AUDIT_CASES)
}

pub fn build_formula_drill_audit_report(cases: &[(&str, &str)]) -> FormulaDrillAuditReport {
    FormulaDrillAuditReport {
        schema_id: "dnaonecalc.formula-drill-audit.v1",
        cases: cases
            .iter()
            .map(|(case_id, formula)| audit_formula_drill_case(case_id, formula))
            .collect(),
    }
}

pub fn audit_formula_drill_case(case_id: &str, formula: &str) -> FormulaDrillAuditCase {
    let bridge = NativeOxfmlHostSession::default();
    let mut state = OneCalcHostState::default();
    let _ = new_formula_space(&mut state);
    if let Some(active_id) = state.workspace_shell.active_formula_space_id.clone() {
        if let Some(space) = state.formula_spaces.get_mut(&active_id) {
            space.formula_drill_open = true;
        }
    }

    let caret_offset = formula.chars().count();
    let bridge_status = match apply_live_editor_input(
        &bridge,
        &mut state,
        EditorInputEvent {
            text: formula.to_string(),
            selection_start: Some(caret_offset),
            selection_end: Some(caret_offset),
            input_kind: EditorInputKind::InsertText,
            inserted_text: Some(formula.to_string()),
        },
    ) {
        Ok(_) => "ok".to_string(),
        Err(error) => format!("error: {error:?}"),
    };

    let view_model = build_home_shell_view_model(&state);
    let (result_summary, user_display, developer_display, metrics, assessment) = match view_model
        .as_ref()
    {
        Some(view_model) => {
            let metrics = compute_metrics(view_model);
            let user_display = render_formula_drill_display(view_model, ViewMode::User);
            let developer_display = render_formula_drill_display(view_model, ViewMode::Developer);
            let assessment = assess_case(formula, &metrics);
            (
                result_summary(view_model),
                user_display,
                developer_display,
                metrics,
                assessment,
            )
        }
        None => (
            "no active formula space".to_string(),
            FormulaDrillDisplaySnapshot {
                mode: "user",
                lines: vec!["<no view model>".to_string()],
            },
            FormulaDrillDisplaySnapshot {
                mode: "developer",
                lines: vec!["<no view model>".to_string()],
            },
            FormulaDrillAuditMetrics::default(),
            FormulaDrillCaseAssessment {
                user_usefulness: FormulaDrillUsefulness::InsufficientForDebugging,
                key_gaps: vec!["host did not produce a home-shell view-model".to_string()],
            },
        ),
    };

    FormulaDrillAuditCase {
        case_id: case_id.to_string(),
        formula: formula.to_string(),
        bridge_status,
        result_summary,
        user_display,
        developer_display,
        metrics,
        assessment,
    }
}

pub fn render_formula_drill_audit_markdown(report: &FormulaDrillAuditReport) -> String {
    let mut out = String::new();
    out.push_str("# Formula Drill Audit\n\n");
    out.push_str("Schema: `");
    out.push_str(report.schema_id);
    out.push_str("`\n\n");
    for case in &report.cases {
        out.push_str("## ");
        out.push_str(&case.case_id);
        out.push_str("\n\n");
        out.push_str("- formula: `");
        out.push_str(&case.formula);
        out.push_str("`\n");
        out.push_str("- bridge: `");
        out.push_str(&case.bridge_status);
        out.push_str("`\n");
        out.push_str("- result: `");
        out.push_str(&case.result_summary);
        out.push_str("`\n");
        out.push_str("- usefulness: `");
        out.push_str(match case.assessment.user_usefulness {
            FormulaDrillUsefulness::UsefulForSimpleFormulas => "useful_for_simple_formulas",
            FormulaDrillUsefulness::PartialForNestedFormulas => "partial_for_nested_formulas",
            FormulaDrillUsefulness::InsufficientForDebugging => "insufficient_for_debugging",
        });
        out.push_str("`\n");
        if !case.assessment.key_gaps.is_empty() {
            out.push_str("- gaps:\n");
            for gap in &case.assessment.key_gaps {
                out.push_str("  - ");
                out.push_str(gap);
                out.push('\n');
            }
        }
        out.push_str("\nUser display:\n\n```text\n");
        for line in &case.user_display.lines {
            out.push_str(line);
            out.push('\n');
        }
        out.push_str("```\n\nDeveloper display:\n\n```text\n");
        for line in &case.developer_display.lines {
            out.push_str(line);
            out.push('\n');
        }
        out.push_str("```\n\n");
    }
    out
}

pub fn render_formula_drill_display(
    view_model: &HomeShellViewModel,
    mode: ViewMode,
) -> FormulaDrillDisplaySnapshot {
    let mut lines = Vec::new();
    let drill = &view_model.formula_drill;
    lines.push(format!("formula drill-down ({})", mode.slug()));
    lines.push(format!(
        "expanded={} fresh={} rows={}",
        drill.expanded,
        drill.document_is_fresh,
        total_rows(&drill.tree)
    ));
    if !drill.document_is_fresh {
        lines.push("(loading...)".to_string());
    } else {
        if drill.diagnostics.is_empty() {
            lines.push("diagnostics: none".to_string());
        } else {
            lines.push("diagnostics:".to_string());
            for diagnostic in &drill.diagnostics {
                match mode {
                    ViewMode::User => lines.push(format!(
                        "  {}: {}",
                        diagnostic.severity.slug(),
                        diagnostic.message
                    )),
                    ViewMode::Developer => lines.push(format!(
                        "  {} [{}] span={}..{} {}",
                        diagnostic.severity.slug(),
                        diagnostic.stage.slug(),
                        diagnostic.span_start,
                        diagnostic.span_start + diagnostic.span_len,
                        diagnostic.message
                    )),
                }
            }
        }
        lines.push("tree:".to_string());
        if drill.tree.is_empty() {
            lines.push("  <empty>".to_string());
        } else {
            for node in &drill.tree {
                render_node_lines(&mut lines, node, mode, 1);
            }
        }
        lines.push(match mode {
            ViewMode::User => render_user_phase_line(&drill.phase_summaries),
            ViewMode::Developer => render_developer_phase_line(&drill.phase_summaries),
        });
        render_capability_lines(&mut lines, view_model, mode);
    }

    FormulaDrillDisplaySnapshot {
        mode: mode.slug(),
        lines,
    }
}

fn render_node_lines(
    lines: &mut Vec<String>,
    node: &FormulaDrillNode,
    mode: ViewMode,
    depth: usize,
) {
    let indent = "  ".repeat(depth);
    let value = node.value_preview.as_deref().unwrap_or("...");
    let line = match mode {
        ViewMode::User => {
            if node.state == FormulaDrillNodeState::Blocked
                || node.state == FormulaDrillNodeState::Error
            {
                if label_includes_value(&node.label) {
                    format!("{indent}{} blocked", node.label)
                } else {
                    format!("{indent}{} blocked {}", node.label, value)
                }
            } else if node.state == FormulaDrillNodeState::Skipped {
                if node.label.to_ascii_lowercase().contains("skipped") {
                    format!("{indent}{}", node.label)
                } else {
                    format!("{indent}{} skipped", node.label)
                }
            } else if label_includes_value(&node.label) {
                format!("{indent}{}", node.label)
            } else {
                format!("{indent}{} = {}", node.label, value)
            }
        }
        ViewMode::Developer => {
            let label = node.developer_label.as_deref().unwrap_or(&node.label);
            if label_includes_value(label) {
                format!(
                    "{indent}[{}] {}",
                    formula_walk_state_slug(node.state),
                    label
                )
            } else {
                format!(
                    "{indent}[{}] {} {}",
                    formula_walk_state_slug(node.state),
                    label,
                    value
                )
            }
        }
    };
    lines.push(line);
    for child in &node.children {
        render_node_lines(lines, child, mode, depth + 1);
    }
}

fn render_user_phase_line(chips: &[FormulaDrillPhaseChip]) -> String {
    if chips.is_empty() {
        return "status: <none>".to_string();
    }
    let any_blocked = chips
        .iter()
        .any(|chip| chip.state == FormulaDrillPhaseState::Blocked);
    if any_blocked {
        return chips
            .iter()
            .find(|chip| chip.state == FormulaDrillPhaseState::Blocked)
            .map(|chip| format!("status: blocked at {}: {}", chip.label, chip.detail))
            .unwrap_or_else(|| "status: blocked".to_string());
    }
    chips
        .iter()
        .find(|chip| chip.label == "eval")
        .map(|chip| {
            let last_segment = chip.detail.rsplit(" - ").next().unwrap_or(&chip.detail);
            let fallback_segment = chip
                .detail
                .rsplit('\u{00b7}')
                .next()
                .map(str::trim)
                .unwrap_or(last_segment);
            format!("status: evaluated in {fallback_segment}")
        })
        .unwrap_or_else(|| "status: evaluated".to_string())
}

fn render_developer_phase_line(chips: &[FormulaDrillPhaseChip]) -> String {
    if chips.is_empty() {
        return "phase strip: <none>".to_string();
    }
    let chunks: Vec<String> = chips
        .iter()
        .map(|chip| format!("{}: {} ({})", chip.label, chip.detail, chip.state.slug()))
        .collect();
    format!("phase strip: {}", chunks.join(" | "))
}

fn render_capability_lines(
    lines: &mut Vec<String>,
    view_model: &HomeShellViewModel,
    mode: ViewMode,
) {
    if mode != ViewMode::Developer {
        return;
    }
    let context = &view_model.capability_context;
    lines.push("capability context:".to_string());
    lines.push(format!(
        "  {} OxFunc metadata version set(s)",
        context
            .snapshot
            .oxfunc_metadata
            .semantic_kernel_metadata_versions
            .len()
    ));
    for row in &context.function_profiles {
        match mode {
            ViewMode::User => lines.push(format!(
                "  {} | {} | {}",
                row.surface_name,
                if row.reduction_sensitive || row.error_collapse_sensitive {
                    "semantic profile active"
                } else {
                    "ordinary function profile"
                },
                row.function_id
            )),
            ViewMode::Developer => lines.push(format!(
                "  {} | {} | semantic={} | admission={}",
                row.surface_name,
                row.numerical_reduction_policy
                    .as_deref()
                    .unwrap_or("no reduction policy"),
                row.semantic_kernel_metadata_version,
                row.arg_admission_metadata_version
            )),
        }
    }
    for fact in &context.value_capability_facts {
        let kind = match fact.fact_kind {
            ValueCapabilityFactKind::ProducerCanProvide => "producer",
            ValueCapabilityFactKind::ExercisedThisRun => "exercised",
        };
        lines.push(format!("  {kind}: {}", fact.key));
    }
    for input in &context.formula_inputs {
        lines.push(format!(
            "  input {} | {} | {}",
            input.label, input.reference_descriptor, input.value_preview
        ));
    }
}

fn result_summary(view_model: &HomeShellViewModel) -> String {
    use crate::services::home_shell_view_model::ResultView;
    match &view_model.result_view {
        ResultView::Empty { .. } => "empty".to_string(),
        ResultView::Pending { .. } => "pending".to_string(),
        ResultView::Display { text, kind, .. } => format!("{kind:?}: {text}"),
        ResultView::Error { code, surface_repr } => format!(
            "error {code}: {}",
            surface_repr.as_deref().unwrap_or("<no detail>")
        ),
        ResultView::Array {
            total_rows,
            total_cols,
            ..
        } => format!("array {total_rows}x{total_cols}"),
    }
}

fn compute_metrics(view_model: &HomeShellViewModel) -> FormulaDrillAuditMetrics {
    let drill = &view_model.formula_drill;
    let mut metrics = FormulaDrillAuditMetrics {
        top_level_rows: drill.tree.len(),
        diagnostic_rows: drill.diagnostics.len(),
        phase_rows: drill.phase_summaries.len(),
        capability_rows: view_model.capability_context.function_profiles.len()
            + view_model.capability_context.value_capability_facts.len()
            + view_model.capability_context.formula_inputs.len(),
        ..FormulaDrillAuditMetrics::default()
    };
    for node in &drill.tree {
        accumulate_node_metrics(node, 0, true, &mut metrics);
    }
    metrics
}

fn accumulate_node_metrics(
    node: &FormulaDrillNode,
    depth: usize,
    top_level: bool,
    metrics: &mut FormulaDrillAuditMetrics,
) {
    metrics.total_rows += 1;
    metrics.max_depth = metrics.max_depth.max(depth);
    if is_argument_row(node) {
        metrics.argument_rows += 1;
    } else if is_function_row(node) {
        metrics.function_rows += 1;
        if top_level {
            metrics.top_level_function_rows += 1;
        }
    }
    let value = node.value_preview.as_deref().unwrap_or("");
    if node.value_preview.is_none() {
        metrics.missing_value_rows += 1;
    }
    if value.contains("eval=") || value.contains("profile:") || value.contains("args:") {
        metrics.debug_fallback_rows += 1;
    }
    for child in &node.children {
        accumulate_node_metrics(child, depth + 1, false, metrics);
    }
}

fn assess_case(formula: &str, metrics: &FormulaDrillAuditMetrics) -> FormulaDrillCaseAssessment {
    let mut gaps = Vec::new();
    let nested_formula = likely_nested_formula(formula);
    if nested_formula && metrics.top_level_function_rows > 1 {
        gaps.push(
            "nested function calls appear as top-level siblings rather than as expression-tree children"
                .to_string(),
        );
    }
    if metrics.debug_fallback_rows > 0 {
        gaps.push("one or more rows display bridge/debug fallback text".to_string());
    }
    if metrics.diagnostic_rows > 0 && metrics.total_rows <= 1 {
        gaps.push("diagnostic case has no failing expression node to focus".to_string());
    }
    if metrics.function_rows == 0 {
        gaps.push("no function or operator rows are available for inspection".to_string());
    }
    if metrics.argument_rows > 0 && metrics.debug_fallback_rows > 0 {
        gaps.push(
            "argument rows have ordinals but no function-specific argument names".to_string(),
        );
    }
    let user_usefulness = if metrics.diagnostic_rows > 0 || metrics.function_rows == 0 {
        FormulaDrillUsefulness::InsufficientForDebugging
    } else if nested_formula || !gaps.is_empty() {
        FormulaDrillUsefulness::PartialForNestedFormulas
    } else {
        FormulaDrillUsefulness::UsefulForSimpleFormulas
    };

    FormulaDrillCaseAssessment {
        user_usefulness,
        key_gaps: gaps,
    }
}

fn total_rows(nodes: &[FormulaDrillNode]) -> usize {
    nodes
        .iter()
        .map(|node| 1 + total_rows(&node.children))
        .sum()
}

fn is_argument_row(node: &FormulaDrillNode) -> bool {
    node.argument_name.is_some()
        || matches!(
            node.kind.as_deref(),
            Some("Argument" | "LetBinding" | "LambdaBinding")
        )
        || node.label.starts_with("arg[")
}

fn is_function_row(node: &FormulaDrillNode) -> bool {
    matches!(
        node.kind.as_deref(),
        Some("FunctionCall" | "OperatorCall" | "FormulaRoot")
    ) || (!node.label.starts_with("arg[") && node.kind.is_none())
}

fn likely_nested_formula(formula: &str) -> bool {
    let upper = formula.to_ascii_uppercase();
    upper.matches("SUM(").count()
        + upper.matches("IF(").count()
        + upper.matches("LET(").count()
        + upper.matches("SEQUENCE(").count()
        > 1
}

fn label_includes_value(label: &str) -> bool {
    label.contains(" = ")
}

fn formula_walk_state_slug(state: FormulaDrillNodeState) -> &'static str {
    match state {
        FormulaDrillNodeState::Pending => "pending",
        FormulaDrillNodeState::Evaluated => "evaluated",
        FormulaDrillNodeState::Bound => "bound",
        FormulaDrillNodeState::Skipped => "skipped",
        FormulaDrillNodeState::Opaque => "opaque",
        FormulaDrillNodeState::Blocked => "blocked",
        FormulaDrillNodeState::Error => "error",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_renders_nested_formula_display_text() {
        let report = build_formula_drill_audit_report(&[("nested", "=SUM(IF(TRUE,2,3),4)")]);
        let case = &report.cases[0];
        let text = case.user_display.lines.join("\n");

        assert!(text.contains("SUM = 6"), "{text}");
        assert!(text.contains("IF = 2"), "{text}");
        assert!(
            case.metrics.max_depth >= 3,
            "trace projection should preserve nested function/argument structure: {:?}",
            case.metrics
        );
        assert!(!case
            .assessment
            .key_gaps
            .iter()
            .any(|gap| gap.contains("top-level siblings")));
    }

    #[test]
    fn audit_markdown_includes_user_and_developer_display_blocks() {
        let report = build_formula_drill_audit_report(&[("sum", "=SUM(1,2,3)")]);
        let markdown = render_formula_drill_audit_markdown(&report);

        assert!(markdown.contains("## sum"));
        assert!(markdown.contains("User display:"));
        assert!(markdown.contains("Developer display:"));
        assert!(markdown.contains("SUM = 6"));
    }
}
