//! Host-neutral projection of native OxFml/OxFunc surfaces into Skin IR.

use dnacalc_skin_ir::{
    ArrayPreviewProjection, ArrayShapeProjection, ArrayWindowCellProjection, ArrayWindowError,
    ArrayWindowProjection, AverageRuleProjection, CalcValueProjection, ColorScaleRuleProjection,
    ColorScaleStopProjection, ComparisonDetailProjection, ComparisonOutcomeProjection,
    ComparisonSurface, CompletionItemProjection, CompletionKindProjection, CompletionSurface,
    ConditionalFormattingRuleProjection, ConditionalFormattingThresholdProjection,
    ConditionalFormattingTypedRuleProjection, CoreValueProjection, DataBarDirectionProjection,
    DataBarRuleProjection, DiagnosticSeverityProjection, DiagnosticStageProjection,
    EditorMetricsProjection, FormattingSurface, FormulaDiagnosticProjection,
    FormulaDrillNodeProjection, FormulaDrillNodeStateProjection, FormulaDrillSurface,
    FormulaEditorSurface, FormulaResultSurface, FunctionHelpSurface, IconSetRuleProjection,
    MAX_ARRAY_WINDOW_CELLS, RankRuleProjection, SignatureHelpParameterProjection,
    SignatureHelpSurface, SyntaxRunProjection, SyntaxTokenRoleProjection,
};
use oxfml_core::consumer::editor::{
    CompletionProposalKind, CompletionResult, EditorDocument, FunctionHelpPacket,
    LiveDiagnosticSeverity, LiveDiagnosticStage,
};
use oxfml_core::consumer::runtime::{FormulaDrillEvaluationState, FormulaDrillTrace};
use oxfml_core::{
    ConditionalFormattingThreshold, DataBarDirection, VerificationConditionalFormattingRule,
    VerificationPublicationContext,
};
use oxfunc_core::value::{CalcArray, CalcValue, CoreValue};

#[must_use]
pub fn project_calc_value(value: &CalcValue) -> CalcValueProjection {
    let core = match value.core() {
        CoreValue::Number(number) => CoreValueProjection::Number {
            raw: number.to_string(),
        },
        CoreValue::Text(text) => CoreValueProjection::Text {
            text: text.to_string_lossy(),
        },
        CoreValue::Logical(value) => CoreValueProjection::Logical { value: *value },
        CoreValue::Error(code) => CoreValueProjection::Error {
            code: format!("{code:?}"),
            display: format!("{code:?}"),
        },
        CoreValue::Empty => CoreValueProjection::Empty,
        CoreValue::Missing => CoreValueProjection::Missing,
        CoreValue::Reference(reference) => CoreValueProjection::Reference {
            target: format!("{reference:?}"),
        },
        CoreValue::Array(array) => {
            let shape = array.shape();
            CoreValueProjection::Array {
                shape: ArrayShapeProjection {
                    rows: shape.rows,
                    cols: shape.cols,
                },
            }
        }
    };
    CalcValueProjection {
        display_text: display_core(value.core()),
        core,
        presentation_hint: value.presentation_value().map(|presentation| {
            dnacalc_skin_ir::PresentationHintProjection {
                family: format!("{:?}", presentation.hint.style),
                format_code: presentation
                    .hint
                    .number_format
                    .as_ref()
                    .map(|hint| format!("{hint:?}")),
            }
        }),
        rich_value_kind: value.rich().map(|rich| format!("{rich:?}")),
        callable: value.callable_value().is_some(),
    }
}

pub fn project_runtime_result(value: &CalcValue) -> Result<FormulaResultSurface, ArrayWindowError> {
    if let Some(array) = value.as_array() {
        let shape = array.shape();
        let rows = shape.rows.min(32);
        let cols = shape
            .cols
            .min((MAX_ARRAY_WINDOW_CELLS / rows.max(1)).max(1));
        return Ok(FormulaResultSurface::Array {
            total_rows: shape.rows,
            total_cols: shape.cols,
            label: format!("{}x{} array", shape.rows, shape.cols),
            window: project_array_window(array, 0, 0, rows, cols)?,
            truncated: rows < shape.rows || cols < shape.cols,
        });
    }
    let value = project_calc_value(value);
    Ok(FormulaResultSurface::Display {
        text: value.display_text.clone(),
        value,
        applied_font_color: None,
        applied_fill_color: None,
    })
}

#[must_use]
pub fn project_comparison(baseline: &CalcValue, candidate: &CalcValue) -> ComparisonSurface {
    let equal = baseline == candidate;
    ComparisonSurface {
        available: true,
        baseline_label: Some("baseline".into()),
        candidate_label: Some("candidate".into()),
        outcome: Some(if equal {
            ComparisonOutcomeProjection::Equal
        } else {
            ComparisonOutcomeProjection::Different
        }),
        details: if equal {
            vec![]
        } else {
            vec![ComparisonDetailProjection {
                path: "value".into(),
                baseline: display_core(baseline.core()),
                candidate: display_core(candidate.core()),
                message: Some("native CalcValue differs".into()),
            }]
        },
    }
}

#[must_use]
pub fn project_formatting(context: &VerificationPublicationContext) -> FormattingSurface {
    FormattingSurface {
        number_format_code: context.number_format_code.clone(),
        font_color: context.font_color.clone(),
        fill_color: context.fill_color.clone(),
        conditional_formatting_rules: context
            .conditional_formatting_rules
            .iter()
            .map(project_cf_rule)
            .collect(),
        ..Default::default()
    }
}

fn project_cf_rule(
    rule: &VerificationConditionalFormattingRule,
) -> ConditionalFormattingRuleProjection {
    let typed_rule = rule.typed_rule.as_ref().and_then(|typed| {
        if let Some(options) = typed.color_scale.as_ref() {
            return Some(ConditionalFormattingTypedRuleProjection::ColorScale(
                ColorScaleRuleProjection {
                    stops: options
                        .stops
                        .iter()
                        .map(|stop| ColorScaleStopProjection {
                            position: project_threshold(&stop.position),
                            color: stop.color.clone(),
                        })
                        .collect(),
                },
            ));
        }
        if let Some(bar) = typed.data_bar.as_ref() {
            return Some(ConditionalFormattingTypedRuleProjection::DataBar(
                DataBarRuleProjection {
                    minimum: bar.minimum.as_ref().map(project_threshold),
                    maximum: bar.maximum.as_ref().map(project_threshold),
                    bar_color: bar.bar_color.clone(),
                    direction: bar.direction.map(|direction| match direction {
                        DataBarDirection::Left => DataBarDirectionProjection::Left,
                        DataBarDirection::Right => DataBarDirectionProjection::Right,
                    }),
                    show_bar_only: bar.show_bar_only,
                },
            ));
        }
        if let Some(options) = typed.icon_set.as_ref() {
            return Some(ConditionalFormattingTypedRuleProjection::IconSet(
                IconSetRuleProjection {
                    set_kind: options.set_kind.clone(),
                    thresholds: options.thresholds.iter().map(project_threshold).collect(),
                },
            ));
        }
        if let Some(options) = typed.rank.as_ref() {
            return Some(ConditionalFormattingTypedRuleProjection::Rank(
                match options.rank {
                    oxfml_core::ConditionalFormattingRank::Count(value) => {
                        RankRuleProjection::Count(value)
                    }
                    oxfml_core::ConditionalFormattingRank::Percent(value) => {
                        RankRuleProjection::Percent(value)
                    }
                },
            ));
        }
        typed.average.as_ref().map(|options| {
            ConditionalFormattingTypedRuleProjection::Average(AverageRuleProjection {
                include_equal: options.include_equal,
                stddev_multiplier: options.stddev_multiplier,
            })
        })
    });
    ConditionalFormattingRuleProjection {
        operator: rule.operator.clone(),
        thresholds: rule
            .thresholds
            .iter()
            .cloned()
            .map(ConditionalFormattingThresholdProjection::Text)
            .collect(),
        font_color: rule.font_color.clone(),
        fill_color: rule.fill_color.clone(),
        typed_rule,
    }
}

fn project_threshold(
    value: &ConditionalFormattingThreshold,
) -> ConditionalFormattingThresholdProjection {
    match value {
        ConditionalFormattingThreshold::Min => ConditionalFormattingThresholdProjection::Min,
        ConditionalFormattingThreshold::Mid => ConditionalFormattingThresholdProjection::Mid,
        ConditionalFormattingThreshold::Max => ConditionalFormattingThresholdProjection::Max,
        ConditionalFormattingThreshold::Percent(v) => {
            ConditionalFormattingThresholdProjection::Percent(*v)
        }
        ConditionalFormattingThreshold::Percentile(v) => {
            ConditionalFormattingThresholdProjection::Percentile(*v)
        }
        ConditionalFormattingThreshold::Number(v) => {
            ConditionalFormattingThresholdProjection::Number(*v)
        }
    }
}

fn display_core(value: &CoreValue) -> String {
    match value {
        CoreValue::Number(value) => value.to_string(),
        CoreValue::Text(value) => value.to_string_lossy(),
        CoreValue::Logical(value) => {
            if *value {
                "TRUE".into()
            } else {
                "FALSE".into()
            }
        }
        CoreValue::Empty => String::new(),
        CoreValue::Missing => "<missing>".into(),
        CoreValue::Error(code) => format!("{code:?}"),
        CoreValue::Reference(reference) => format!("{reference:?}"),
        CoreValue::Array(array) => {
            let shape = array.shape();
            format!("{{{}x{} array}}", shape.rows, shape.cols)
        }
    }
}

pub fn project_array_window(
    array: &CalcArray,
    row_offset: usize,
    col_offset: usize,
    row_count: usize,
    col_count: usize,
) -> Result<ArrayWindowProjection, ArrayWindowError> {
    if row_count == 0 || col_count == 0 {
        return Err(ArrayWindowError::Empty);
    }
    if row_count
        .checked_mul(col_count)
        .ok_or(ArrayWindowError::Overflow)?
        > MAX_ARRAY_WINDOW_CELLS
    {
        return Err(ArrayWindowError::TooLarge);
    }
    let shape = array.shape();
    let row_end = row_offset
        .checked_add(row_count)
        .ok_or(ArrayWindowError::Overflow)?;
    let col_end = col_offset
        .checked_add(col_count)
        .ok_or(ArrayWindowError::Overflow)?;
    if row_end > shape.rows || col_end > shape.cols {
        return Err(ArrayWindowError::OutsideArray);
    }
    let mut cells = Vec::with_capacity(row_count);
    for row in row_offset..row_end {
        let mut projected = Vec::with_capacity(col_count);
        for col in col_offset..col_end {
            let value = array.get(row, col).expect("validated native array window");
            let value = project_calc_value(value);
            projected.push(ArrayWindowCellProjection {
                display_text: value.display_text.clone(),
                value: Some(value),
                format: None,
            });
        }
        cells.push(projected);
    }
    let window = ArrayWindowProjection {
        total_rows: shape.rows,
        total_cols: shape.cols,
        row_offset,
        col_offset,
        cells,
    };
    window.validate()?;
    Ok(window)
}

#[must_use]
pub fn project_editor(document: &EditorDocument, caret_offset: usize) -> FormulaEditorSurface {
    let diagnostics = document
        .live_diagnostics
        .diagnostics
        .iter()
        .map(|diagnostic| FormulaDiagnosticProjection {
            diagnostic_id: diagnostic.diagnostic_id.clone(),
            severity: match diagnostic.severity {
                LiveDiagnosticSeverity::Error => DiagnosticSeverityProjection::Error,
                LiveDiagnosticSeverity::Warning => DiagnosticSeverityProjection::Warning,
                LiveDiagnosticSeverity::Info => DiagnosticSeverityProjection::Info,
            },
            stage: match diagnostic.stage {
                LiveDiagnosticStage::Syntax => DiagnosticStageProjection::Syntax,
                LiveDiagnosticStage::Bind => DiagnosticStageProjection::Bind,
                LiveDiagnosticStage::SemanticPlan => DiagnosticStageProjection::SemanticPlan,
            },
            code: diagnostic.code.clone(),
            worksheet_error_class: diagnostic.worksheet_error_class.clone(),
            message: diagnostic.message.clone(),
            span_start: diagnostic.primary_span.start,
            span_len: diagnostic.primary_span.len,
        })
        .collect::<Vec<_>>();
    let syntax_runs = document
        .editor_syntax_snapshot
        .tokens
        .iter()
        .enumerate()
        .map(|(index, token)| SyntaxRunProjection {
            text: token.text.clone(),
            span_start: token.span.start,
            span_len: token.span.len,
            role: if format!("{:?}", token.kind) == "Identifier"
                && document
                    .editor_syntax_snapshot
                    .tokens
                    .get(index + 1)
                    .is_some_and(|next| format!("{:?}", next.kind) == "LParen")
            {
                SyntaxTokenRoleProjection::Function
            } else {
                token_role(&format!("{:?}", token.kind))
            },
        })
        .collect::<Vec<_>>();
    FormulaEditorSurface {
        source_text: document.source.entered_formula_text.clone(),
        caret_offset,
        selection_anchor: caret_offset,
        selection_focus: caret_offset,
        metrics: EditorMetricsProjection {
            token_count: syntax_runs.len(),
            function_count: syntax_runs
                .iter()
                .filter(|run| run.role == SyntaxTokenRoleProjection::Function)
                .count(),
            diagnostic_count: diagnostics.len(),
            first_diagnostic_message: diagnostics.first().map(|d| d.message.clone()),
        },
        syntax_runs,
        diagnostics,
        document_is_fresh: true,
    }
}

fn token_role(kind: &str) -> SyntaxTokenRoleProjection {
    match kind {
        "Number" => SyntaxTokenRoleProjection::Number,
        "StringLiteral" => SyntaxTokenRoleProjection::Text,
        "Identifier" => SyntaxTokenRoleProjection::Identifier,
        "Whitespace" => SyntaxTokenRoleProjection::Trivia,
        "LParen" | "RParen" | "Comma" => SyntaxTokenRoleProjection::Delimiter,
        _ => SyntaxTokenRoleProjection::Operator,
    }
}

#[must_use]
pub fn project_completion(result: &CompletionResult) -> CompletionSurface {
    CompletionSurface {
        anchor_left_px: 0,
        anchor_top_px: 0,
        line_height_px: 0,
        selected_index: 0,
        items: result
            .proposals
            .iter()
            .map(|proposal| CompletionItemProjection {
                proposal_id: proposal.proposal_id.clone(),
                display_text: proposal.display_text.clone(),
                kind: match proposal.proposal_kind {
                    CompletionProposalKind::Function => CompletionKindProjection::Function,
                    CompletionProposalKind::DefinedName => CompletionKindProjection::DefinedName,
                    CompletionProposalKind::TableName => CompletionKindProjection::TableName,
                    CompletionProposalKind::TableColumn => CompletionKindProjection::TableColumn,
                    CompletionProposalKind::StructuredSelector => {
                        CompletionKindProjection::StructuredSelector
                    }
                    CompletionProposalKind::SyntaxAssist => CompletionKindProjection::SyntaxAssist,
                    CompletionProposalKind::ProfileReference => {
                        CompletionKindProjection::ProfileReference
                    }
                },
                documentation_ref: proposal.documentation_ref.clone(),
            })
            .collect(),
    }
}

#[must_use]
pub fn project_signature(result: &CompletionResult) -> Option<SignatureHelpSurface> {
    let context = result.signature_help_context.as_ref()?;
    if context.callee_text.starts_with('_') {
        return None;
    }
    Some(SignatureHelpSurface {
        callee_text: context.callee_text.clone(),
        anchor_left_px: 0,
        anchor_top_px: 0,
        line_height_px: 0,
        parameters: Vec::new(),
        active_parameter: Some(context.active_argument_index),
    })
}

#[must_use]
pub fn project_signature_with_help(
    result: &CompletionResult,
    packet: &FunctionHelpPacket,
) -> Option<SignatureHelpSurface> {
    let mut surface = project_signature(result)?;
    surface.parameters = packet
        .argument_help
        .iter()
        .enumerate()
        .map(|(index, name)| SignatureHelpParameterProjection {
            name: name.strip_prefix('*').unwrap_or(name).to_string(),
            is_active: Some(index) == surface.active_parameter,
        })
        .collect();
    Some(surface)
}

#[must_use]
pub fn project_function_help(packet: &FunctionHelpPacket) -> FunctionHelpSurface {
    FunctionHelpSurface {
        lookup_key: packet.lookup_key.clone(),
        display_name: packet.display_name.clone(),
        signature: packet
            .signature_forms
            .first()
            .map(|form| form.display_signature.clone()),
        short_description: packet.short_description.clone(),
        availability_summary: packet.availability_summary.clone(),
        deferred_or_profile_limited: packet.deferred_or_profile_limited,
    }
}

#[must_use]
pub fn project_drill(trace: &FormulaDrillTrace) -> FormulaDrillSurface {
    fn node_for(trace: &FormulaDrillTrace, id: &str) -> Option<FormulaDrillNodeProjection> {
        let node = trace.nodes.iter().find(|node| node.node_id.0 == id)?;
        let array_preview = node.value_preview.as_ref().and_then(|preview| {
            let shape = preview.array_shape?;
            let raw_limit = preview
                .preview
                .len()
                .min(MAX_ARRAY_WINDOW_CELLS)
                .min(shape.rows.saturating_mul(shape.cols));
            let limit = raw_limit - (raw_limit % shape.cols.max(1));
            if limit == 0 {
                return None;
            }
            let rows = preview.preview[..limit]
                .chunks(shape.cols.max(1))
                .map(|row| row.to_vec())
                .collect();
            let projected = ArrayPreviewProjection {
                total_rows: shape.rows,
                total_cols: shape.cols,
                rows,
                truncated: preview.truncated || limit < shape.rows.saturating_mul(shape.cols),
            };
            projected.validate().ok().map(|_| projected)
        });
        Some(FormulaDrillNodeProjection {
            node_id: node.node_id.0.clone(),
            label: node.label_user.clone(),
            developer_label: Some(node.label_developer.clone()),
            expression_text: node.expression_text.clone(),
            kind: Some(format!("{:?}", node.kind)),
            source_span_start: node.source_span.map(|span| span.start),
            source_span_len: node.source_span.map(|span| span.len),
            branch_disposition: node.branch_disposition.map(|value| format!("{value:?}")),
            argument_name: node.argument_name.clone(),
            argument_role: node
                .argument_role
                .as_ref()
                .map(|value| format!("{value:?}")),
            error_message: node.error.as_ref().map(|error| error.message.clone()),
            value_preview: node
                .value_preview
                .as_ref()
                .and_then(|preview| preview.preview.first().cloned()),
            array_preview,
            state: match node.evaluation_state {
                FormulaDrillEvaluationState::Pending => FormulaDrillNodeStateProjection::Pending,
                FormulaDrillEvaluationState::Bound => FormulaDrillNodeStateProjection::Bound,
                FormulaDrillEvaluationState::Evaluated => {
                    FormulaDrillNodeStateProjection::Evaluated
                }
                FormulaDrillEvaluationState::Skipped
                | FormulaDrillEvaluationState::ShortCircuited
                | FormulaDrillEvaluationState::Omitted => FormulaDrillNodeStateProjection::Skipped,
                FormulaDrillEvaluationState::Blocked => FormulaDrillNodeStateProjection::Blocked,
                FormulaDrillEvaluationState::Error => FormulaDrillNodeStateProjection::Error,
            },
            children: node
                .child_node_ids
                .iter()
                .filter_map(|child| node_for(trace, &child.0))
                .collect(),
        })
    }
    let nodes = node_for(trace, &trace.root_node_id.0).into_iter().collect();
    let diagnostics = trace
        .diagnostics
        .iter()
        .map(|diagnostic| FormulaDiagnosticProjection {
            diagnostic_id: diagnostic.diagnostic_id.clone(),
            severity: DiagnosticSeverityProjection::Error,
            stage: DiagnosticStageProjection::SemanticPlan,
            code: None,
            worksheet_error_class: None,
            message: diagnostic.message.clone(),
            span_start: diagnostic.source_span.start,
            span_len: diagnostic.source_span.len,
        })
        .collect();
    FormulaDrillSurface {
        expanded: true,
        tree: nodes,
        diagnostics,
        phase_summaries: Vec::new(),
        document_is_fresh: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxfml_core::binding::BindContext;
    use oxfml_core::consumer::editor::{
        CompletionProposal, EditorEditService, EditorEnvironment, FunctionHelpSignatureForm,
        SignatureHelpContext,
    };
    use oxfml_core::consumer::runtime::{
        FormulaArgumentNameSource, FormulaDrillArrayShape, FormulaDrillNodeId,
        FormulaDrillNodeKind, FormulaDrillTraceNode, FormulaDrillValuePreview,
    };
    use oxfml_core::source::FormulaSourceRecord;
    use oxfml_core::syntax::green::SyntaxKind;
    use oxfml_core::syntax::token::TextSpan;
    use oxfunc_core::value::CalcArray;

    #[test]
    fn native_values_and_arrays_project_without_semantic_duplicates() {
        let array = CalcArray::from_rows(vec![
            vec![CalcValue::number(1.0), CalcValue::number(2.0)],
            vec![CalcValue::number(3.0), CalcValue::number(4.0)],
        ])
        .unwrap();
        let projected = project_calc_value(&CalcValue::array(array.clone()));
        assert!(matches!(
            projected.core,
            CoreValueProjection::Array {
                shape: ArrayShapeProjection { rows: 2, cols: 2 }
            }
        ));
        let window = project_array_window(&array, 0, 0, 2, 2).unwrap();
        assert_eq!(window.cells[1][1].display_text, "4");
        assert!(matches!(
            project_runtime_result(&CalcValue::array(array)).unwrap(),
            FormulaResultSurface::Array {
                total_rows: 2,
                total_cols: 2,
                ..
            }
        ));
        assert_eq!(
            project_comparison(&CalcValue::number(1.0), &CalcValue::number(2.0)).outcome,
            Some(ComparisonOutcomeProjection::Different)
        );
    }

    #[test]
    fn native_editor_completion_signature_and_help_project() {
        let service = EditorEditService::new(EditorEnvironment::new(BindContext::default()));
        let document =
            service.open_document(FormulaSourceRecord::new("f1", 1, "=SUM(1,2,3)"), None);
        let editor = project_editor(&document, 11);
        assert_eq!(editor.source_text, "=SUM(1,2,3)");
        assert!(!editor.syntax_runs.is_empty());

        let completion = CompletionResult {
            replacement_span: Some(TextSpan::new(1, 2)),
            proposals: vec![CompletionProposal {
                proposal_id: "sum".into(),
                proposal_kind: CompletionProposalKind::Function,
                display_text: "SUM".into(),
                insert_text: "SUM".into(),
                replacement_span: Some(TextSpan::new(1, 2)),
                documentation_ref: Some("SUM".into()),
                profile_payload: None,
                requires_revalidation: false,
            }],
            signature_help_context: Some(SignatureHelpContext {
                callee_text: "SUM".into(),
                call_span: TextSpan::new(1, 4),
                active_argument_index: 0,
                invocation_kind: SyntaxKind::CallExpr,
            }),
        };
        assert_eq!(project_completion(&completion).items[0].display_text, "SUM");
        assert_eq!(project_signature(&completion).unwrap().callee_text, "SUM");

        let help = FunctionHelpPacket {
            lookup_key: "SUM".into(),
            library_context_snapshot_ref: None,
            display_name: "SUM".into(),
            signature_forms: vec![FunctionHelpSignatureForm {
                display_signature: "SUM(number1, [number2], ...)".into(),
                min_arity: 1,
                max_arity: None,
            }],
            argument_help: vec!["*number1".into(), "number2".into()],
            short_description: Some("Adds numbers".into()),
            availability_summary: None,
            deferred_or_profile_limited: false,
        };
        assert_eq!(
            project_function_help(&help).signature.as_deref(),
            Some("SUM(number1, [number2], ...)")
        );
        let signature = project_signature_with_help(&completion, &help).unwrap();
        assert_eq!(signature.parameters[0].name, "number1");
        assert!(signature.parameters[0].is_active);
        let placeholder = CompletionResult {
            signature_help_context: Some(SignatureHelpContext {
                callee_text: "_xlfn.PLACEHOLDER".into(),
                call_span: TextSpan::new(0, 1),
                active_argument_index: 0,
                invocation_kind: SyntaxKind::CallExpr,
            }),
            ..completion.clone()
        };
        assert!(project_signature(&placeholder).is_none());

        let context = VerificationPublicationContext {
            number_format_code: Some("0.00".into()),
            conditional_formatting_rules: vec![VerificationConditionalFormattingRule {
                typed_rule: Some(oxfml_core::ConditionalFormattingTypedRule {
                    data_bar: Some(oxfml_core::DataBarRuleOptions {
                        minimum: Some(ConditionalFormattingThreshold::Min),
                        maximum: Some(ConditionalFormattingThreshold::Max),
                        bar_color: Some("#00AAFF".into()),
                        direction: Some(DataBarDirection::Right),
                        show_bar_only: false,
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            }],
            ..Default::default()
        };
        let formatting = project_formatting(&context);
        assert_eq!(formatting.number_format_code.as_deref(), Some("0.00"));
        assert!(matches!(
            formatting.conditional_formatting_rules[0].typed_rule,
            Some(ConditionalFormattingTypedRuleProjection::DataBar(_))
        ));
        let typed_rules = vec![
            oxfml_core::ConditionalFormattingTypedRule {
                color_scale: Some(oxfml_core::ColorScaleRuleOptions { stops: vec![] }),
                ..Default::default()
            },
            oxfml_core::ConditionalFormattingTypedRule {
                icon_set: Some(oxfml_core::IconSetRuleOptions {
                    set_kind: "3Arrows".into(),
                    thresholds: vec![],
                }),
                ..Default::default()
            },
            oxfml_core::ConditionalFormattingTypedRule {
                rank: Some(oxfml_core::RankRuleOptions {
                    rank: oxfml_core::ConditionalFormattingRank::Count(3),
                }),
                ..Default::default()
            },
            oxfml_core::ConditionalFormattingTypedRule {
                average: Some(oxfml_core::AverageRuleOptions {
                    include_equal: true,
                    stddev_multiplier: None,
                }),
                ..Default::default()
            },
        ];
        let projected: Vec<_> = typed_rules
            .into_iter()
            .map(|typed_rule| {
                project_cf_rule(&VerificationConditionalFormattingRule {
                    typed_rule: Some(typed_rule),
                    ..Default::default()
                })
                .typed_rule
                .unwrap()
            })
            .collect();
        assert!(matches!(
            projected.as_slice(),
            [
                ConditionalFormattingTypedRuleProjection::ColorScale(_),
                ConditionalFormattingTypedRuleProjection::IconSet(_),
                ConditionalFormattingTypedRuleProjection::Rank(_),
                ConditionalFormattingTypedRuleProjection::Average(_)
            ]
        ));
    }

    #[test]
    fn native_drill_array_preview_stays_bounded_and_expandable() {
        let node_id = FormulaDrillNodeId("root".into());
        let node = FormulaDrillTraceNode {
            node_id: node_id.clone(),
            parent_node_id: None,
            source_span: Some(TextSpan::new(0, 14)),
            expression_text: Some("=SEQUENCE(2,2)".into()),
            kind: FormulaDrillNodeKind::FormulaRoot,
            function_id: None,
            function_surface_name: None,
            operator_kind: None,
            argument_ordinal: None,
            argument_name: None,
            argument_role: None,
            argument_name_source: FormulaArgumentNameSource::NotApplicable,
            label_user: "SEQUENCE".into(),
            label_developer: "formula root".into(),
            evaluation_state: FormulaDrillEvaluationState::Evaluated,
            branch_disposition: None,
            value_before_coercion: None,
            value_after_coercion: None,
            returned_value: None,
            published_value: None,
            value_preview: Some(FormulaDrillValuePreview {
                value_kind: "array".into(),
                array_shape: Some(FormulaDrillArrayShape { rows: 2, cols: 2 }),
                preview: vec!["1".into(), "2".into(), "3".into(), "4".into()],
                truncated: false,
                rich_value_type_name: None,
            }),
            error: None,
            child_node_ids: vec![FormulaDrillNodeId("child".into())],
            prepared_call_index: None,
            prepared_argument_index: None,
        };
        let mut child = node.clone();
        child.node_id = FormulaDrillNodeId("child".into());
        child.parent_node_id = Some(node_id.clone());
        child.child_node_ids.clear();
        child.value_preview = Some(FormulaDrillValuePreview {
            value_kind: "array".into(),
            array_shape: Some(FormulaDrillArrayShape {
                rows: 100,
                cols: 100,
            }),
            preview: vec!["x".into(); 10_000],
            truncated: false,
            rich_value_type_name: None,
        });
        let trace = FormulaDrillTrace {
            schema_id: "oxfml.formula_drill.v1",
            formula_stable_id: "f".into(),
            source_text: "=SEQUENCE(2,2)".into(),
            root_node_id: node_id.clone(),
            nodes: vec![node, child],
            evaluation_order: vec![node_id],
            diagnostics: vec![],
            final_value: CalcValue::empty(),
            projection_losses: vec![],
        };
        let projected = project_drill(&trace);
        let preview = projected.tree[0].array_preview.as_ref().unwrap();
        assert_eq!((preview.total_rows, preview.total_cols), (2, 2));
        assert_eq!(preview.validate(), Ok(()));
        assert_eq!(projected.tree[0].children[0].node_id, "child");
        let bounded = projected.tree[0].children[0]
            .array_preview
            .as_ref()
            .unwrap();
        assert!(bounded.truncated);
        assert!(bounded.rows.len() * bounded.rows[0].len() <= MAX_ARRAY_WINDOW_CELLS);
    }
}
