use crate::adapters::oxfml::{
    BindSummary, CompletionProposal, CompletionProposalKind, EditorDocument, EditorSyntaxSnapshot,
    EditorToken, EvalSummary, FormulaDrillNodeState, FormulaDrillNodeViewModel,
    FormulaEditReuseSummary, FormulaTextChangeRange, FormulaTextSpan, FunctionHelpPacket,
    FunctionHelpSignatureForm, LiveDiagnostic, LiveDiagnosticSeverity, LiveDiagnosticSnapshot,
    LiveDiagnosticStage, ParseSummary, ProvenanceSummary, SignatureHelpContext,
};
use oxfml_core::source::FormulaChannelKind;
use oxfml_core::syntax::green::SyntaxKind;
use oxfml_core::syntax::token::TokenKind;

pub fn sample_editor_document(source_text: &str) -> EditorDocument {
    sample_editor_document_with_green_key(source_text, "green-1")
}

pub fn sample_editor_document_with_green_key(
    source_text: &str,
    green_tree_key: &str,
) -> EditorDocument {
    let tokens = sample_editor_tokens(source_text);

    EditorDocument {
        source_text: source_text.to_string(),
        text_change_range: Some(FormulaTextChangeRange {
            start: 0,
            old_len: 0,
            new_len: source_text.chars().count(),
        }),
        editor_syntax_snapshot: EditorSyntaxSnapshot {
            formula_stable_id: "formula-1".to_string(),
            formula_channel_kind: FormulaChannelKind::WorksheetA1,
            green_tree_key: green_tree_key.to_string(),
            tokens,
        },
        live_diagnostics: LiveDiagnosticSnapshot {
            formula_stable_id: "formula-1".to_string(),
            formula_token: "formula-1".to_string(),
            diagnostics: vec![sample_live_diagnostic(
                "diag-1",
                "sample diagnostic",
                0,
                source_text.chars().count(),
            )],
        },
        reuse_summary: FormulaEditReuseSummary {
            reused_green_tree: false,
            reused_red_projection: false,
            reused_bound_formula: false,
        },
        signature_help: Some(SignatureHelpContext {
            callee_text: "SUM".to_string(),
            call_span: FormulaTextSpan {
                start: 0,
                len: source_text.chars().count(),
            },
            active_argument_index: 0,
            invocation_kind: SyntaxKind::CallExpr,
        }),
        function_help: Some(FunctionHelpPacket {
            lookup_key: "SUM".to_string(),
            library_context_snapshot_ref: None,
            display_name: "SUM".to_string(),
            signature_forms: vec![FunctionHelpSignatureForm {
                display_signature: "SUM(number1, number2, ...)".to_string(),
                min_arity: 1,
                max_arity: None,
            }],
            argument_help: vec![
                "number1".to_string(),
                "number2".to_string(),
                "additional_numbers".to_string(),
            ],
            short_description: Some("Adds numbers together.".to_string()),
            availability_summary: Some("supported".to_string()),
            deferred_or_profile_limited: false,
        }),
        completion_proposals: vec![CompletionProposal {
            proposal_id: "proposal-1".to_string(),
            proposal_kind: CompletionProposalKind::Function,
            display_text: "SUM".to_string(),
            insert_text: "SUM(".to_string(),
            replacement_span: Some(FormulaTextSpan { start: 1, len: 3 }),
            documentation_ref: Some("preview:function:SUM".to_string()),
            profile_payload: None,
            requires_revalidation: true,
        }],
        formula_walk: vec![FormulaDrillNodeViewModel {
            node_id: "node-1".to_string(),
            label: "SUM".to_string(),
            developer_label: None,
            expression_text: None,
            kind: None,
            source_span_start: None,
            source_span_len: None,
            branch_disposition: None,
            argument_name: None,
            argument_role: None,
            error_message: None,
            value_preview: Some("3".to_string()),
            array_preview: None,
            state: FormulaDrillNodeState::Evaluated,
            children: vec![],
        }],
        parse_summary: Some(ParseSummary {
            status: "Valid".to_string(),
            token_count: 1,
        }),
        bind_summary: Some(BindSummary {
            variable_count: 0,
            reference_count: 0,
        }),
        eval_summary: Some(EvalSummary {
            step_count: 1,
            duration_text: "0.1ms".to_string(),
        }),
        provenance_summary: Some(ProvenanceSummary {
            profile_summary: "OC-H0".to_string(),
            blocked_reason: None,
        }),
        value_presentation: None,
    }
}

pub fn diagnostic_editor_document(source_text: &str) -> EditorDocument {
    let mut document = sample_editor_document_with_green_key(source_text, "green-diag-1");
    document.live_diagnostics = LiveDiagnosticSnapshot {
        formula_stable_id: "formula-1".to_string(),
        formula_token: "formula-1".to_string(),
        diagnostics: vec![sample_live_diagnostic(
            "diag-missing-arg",
            "Missing trailing argument",
            source_text.len().saturating_sub(2),
            1,
        )],
    };
    document.parse_summary = Some(ParseSummary {
        status: "Recoverable".to_string(),
        token_count: source_text.chars().count(),
    });
    document.eval_summary = Some(EvalSummary {
        step_count: 0,
        duration_text: "diagnostic-only".to_string(),
    });
    document
}

pub fn array_editor_document(source_text: &str) -> EditorDocument {
    let mut document = sample_editor_document_with_green_key(source_text, "green-array-1");
    document.function_help = Some(FunctionHelpPacket {
        lookup_key: "SEQUENCE".to_string(),
        library_context_snapshot_ref: None,
        display_name: "SEQUENCE".to_string(),
        signature_forms: vec![FunctionHelpSignatureForm {
            display_signature: "SEQUENCE(rows, columns, start, step)".to_string(),
            min_arity: 1,
            max_arity: Some(4),
        }],
        argument_help: vec![
            "rows".to_string(),
            "columns".to_string(),
            "start".to_string(),
            "step".to_string(),
        ],
        short_description: Some("Returns a spilled array of sequential numbers.".to_string()),
        availability_summary: Some("dynamic arrays supported".to_string()),
        deferred_or_profile_limited: false,
    });
    document.signature_help = Some(SignatureHelpContext {
        callee_text: "SEQUENCE".to_string(),
        call_span: FormulaTextSpan {
            start: 0,
            len: source_text.chars().count(),
        },
        active_argument_index: 1,
        invocation_kind: SyntaxKind::CallExpr,
    });
    document.formula_walk = vec![FormulaDrillNodeViewModel {
        node_id: "node-sequence".to_string(),
        label: "SEQUENCE".to_string(),
        developer_label: None,
        expression_text: None,
        kind: None,
        source_span_start: None,
        source_span_len: None,
        branch_disposition: None,
        argument_name: None,
        argument_role: None,
        error_message: None,
        value_preview: Some("{1,2;3,4}".to_string()),
        array_preview: Some(crate::adapters::oxfml::FormulaDrillArrayPreview {
            total_rows: 2,
            total_cols: 2,
            rows: vec![
                vec!["1".to_string(), "2".to_string()],
                vec!["3".to_string(), "4".to_string()],
            ],
            truncated: false,
        }),
        state: FormulaDrillNodeState::Evaluated,
        children: vec![
            FormulaDrillNodeViewModel {
                node_id: "node-rows".to_string(),
                label: "rows".to_string(),
                developer_label: None,
                expression_text: None,
                kind: None,
                source_span_start: None,
                source_span_len: None,
                branch_disposition: None,
                argument_name: None,
                argument_role: None,
                error_message: None,
                value_preview: Some("2".to_string()),
                array_preview: None,
                state: FormulaDrillNodeState::Bound,
                children: vec![],
            },
            FormulaDrillNodeViewModel {
                node_id: "node-cols".to_string(),
                label: "columns".to_string(),
                developer_label: None,
                expression_text: None,
                kind: None,
                source_span_start: None,
                source_span_len: None,
                branch_disposition: None,
                argument_name: None,
                argument_role: None,
                error_message: None,
                value_preview: Some("2".to_string()),
                array_preview: None,
                state: FormulaDrillNodeState::Bound,
                children: vec![],
            },
        ],
    }];
    document.eval_summary = Some(EvalSummary {
        step_count: 4,
        duration_text: "0.3ms".to_string(),
    });
    document
}

pub fn blocked_editor_document(source_text: &str) -> EditorDocument {
    let mut document = sample_editor_document_with_green_key(source_text, "green-blocked-1");
    document.formula_walk = vec![FormulaDrillNodeViewModel {
        node_id: "node-xlookup".to_string(),
        label: "XLOOKUP".to_string(),
        developer_label: None,
        expression_text: None,
        kind: None,
        source_span_start: None,
        source_span_len: None,
        branch_disposition: None,
        argument_name: None,
        argument_role: None,
        error_message: None,
        value_preview: None,
        array_preview: None,
        state: FormulaDrillNodeState::Blocked,
        children: vec![],
    }];
    document.provenance_summary = Some(ProvenanceSummary {
        profile_summary: "OxFml blocked lane".to_string(),
        blocked_reason: Some("Excel comparison lane unavailable on this host".to_string()),
    });
    document.eval_summary = Some(EvalSummary {
        step_count: 1,
        duration_text: "blocked".to_string(),
    });
    document
}

fn sample_editor_tokens(source_text: &str) -> Vec<EditorToken> {
    if source_text == "=SUM(1,2)" {
        vec![
            token("=", 0),
            token("SUM", 1),
            token("(", 4),
            token("1", 5),
            token(",", 6),
            token("2", 7),
            token(")", 8),
        ]
    } else if source_text == "=LET(x,1,x)" {
        vec![
            token("=", 0),
            token("LET", 1),
            token("(", 4),
            token("x", 5),
            token(",", 6),
            token("1", 7),
            token(",", 8),
            token("x", 9),
            token(")", 10),
        ]
    } else {
        vec![token(source_text, 0)]
    }
}

fn token(text: &str, span_start: usize) -> EditorToken {
    EditorToken {
        kind: TokenKind::Identifier,
        text: text.to_string(),
        leading_trivia: Vec::new(),
        trailing_trivia: Vec::new(),
        span: FormulaTextSpan {
            start: span_start,
            len: text.chars().count(),
        },
    }
}

fn sample_live_diagnostic(
    diagnostic_id: &str,
    message: &str,
    span_start: usize,
    span_len: usize,
) -> LiveDiagnostic {
    LiveDiagnostic {
        diagnostic_id: diagnostic_id.to_string(),
        severity: LiveDiagnosticSeverity::Error,
        stage: LiveDiagnosticStage::Bind,
        message: message.to_string(),
        primary_span: FormulaTextSpan {
            start: span_start,
            len: span_len,
        },
        related_spans: Vec::new(),
        code: None,
        worksheet_error_class: None,
        suggested_fix_kind: None,
    }
}

/// Public helper for legacy tests that want a default-shaped
/// `LiveDiagnosticSnapshot`. Upstream's `LiveDiagnosticSnapshot` doesn't
/// derive `Default`, so this wraps the empty-vector + empty-id case.
pub fn empty_live_diagnostic_snapshot() -> LiveDiagnosticSnapshot {
    LiveDiagnosticSnapshot {
        formula_stable_id: String::new(),
        formula_token: String::new(),
        diagnostics: Vec::new(),
    }
}

/// Public helper for legacy tests that build a `LiveDiagnosticSnapshot`
/// from a vector of diagnostics with default identity.
pub fn live_diagnostic_snapshot_with(diagnostics: Vec<LiveDiagnostic>) -> LiveDiagnosticSnapshot {
    LiveDiagnosticSnapshot {
        formula_stable_id: String::new(),
        formula_token: String::new(),
        diagnostics,
    }
}

/// Public helper for legacy tests that build a `LiveDiagnostic` with
/// the host-mirror's earlier `(diagnostic_id, message, span_start,
/// span_len)` shape. Severity defaults to Error and stage to Bind.
pub fn make_live_diagnostic(
    diagnostic_id: &str,
    message: &str,
    span_start: usize,
    span_len: usize,
) -> LiveDiagnostic {
    sample_live_diagnostic(diagnostic_id, message, span_start, span_len)
}

/// Public helper for legacy tests that build an `EditorToken` with
/// the host-mirror's earlier `(text, span_start, span_len)` shape.
pub fn make_editor_token(text: &str, span_start: usize) -> EditorToken {
    token(text, span_start)
}

/// Public helper for legacy tests that build a default-shaped
/// `EditorSyntaxSnapshot` from `(formula_stable_id, green_tree_key, tokens)`.
pub fn make_editor_syntax_snapshot(
    formula_stable_id: &str,
    green_tree_key: &str,
    tokens: Vec<EditorToken>,
) -> EditorSyntaxSnapshot {
    EditorSyntaxSnapshot {
        formula_stable_id: formula_stable_id.to_string(),
        formula_channel_kind: FormulaChannelKind::WorksheetA1,
        green_tree_key: green_tree_key.to_string(),
        tokens,
    }
}

/// Public helper for legacy tests that build a default-shaped
/// `SignatureHelpContext`.
pub fn make_signature_help_context(
    callee_text: &str,
    span_start: usize,
    span_len: usize,
    active_argument_index: usize,
) -> SignatureHelpContext {
    SignatureHelpContext {
        callee_text: callee_text.to_string(),
        call_span: FormulaTextSpan {
            start: span_start,
            len: span_len,
        },
        active_argument_index,
        invocation_kind: SyntaxKind::CallExpr,
    }
}

/// Public helper for legacy tests that build a default-shaped
/// `FunctionHelpPacket`.
pub fn make_function_help_packet(
    lookup_key: &str,
    display_name: &str,
    signature_forms: Vec<FunctionHelpSignatureForm>,
    argument_help: Vec<String>,
    short_description: Option<String>,
    availability_summary: Option<String>,
) -> FunctionHelpPacket {
    FunctionHelpPacket {
        lookup_key: lookup_key.to_string(),
        library_context_snapshot_ref: None,
        display_name: display_name.to_string(),
        signature_forms,
        argument_help,
        short_description,
        availability_summary,
        deferred_or_profile_limited: false,
    }
}
