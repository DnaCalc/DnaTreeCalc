//! Native unit tests for the bridge's pure view-model layer (`vm.rs`) and its
//! semantic-event construction. Everything the DOM components render is
//! computed here, so this is where the layering laws are proven without a
//! browser: token rendering, UTF-8 span math, completion payloads,
//! diagnostics span math over multi-byte text, the degrade-mode no-token-color
//! law, and verbatim passthrough.
//!
//! The golden fixture `dnacalc-skin-ir/fixtures/one_formula.json` is consumed
//! through the real `SkinSnapshot` wire type, proving the bridge speaks the
//! actual IR — then richer surfaces are built in-test for the substantive
//! assertions the (deliberately empty) fixture document cannot exercise.

use dnacalc_bridge::{
    ALL_COMPLETION_KINDS, BridgeEvent, DegradeKeyDisposition, EditDiscipline, buffer_is_dirty,
    completion_applied, completion_kind_glyph, completion_kind_id, completion_next,
    degrade_key_disposition, degrade_segments, drill_node_at_caret, drill_node_for_selection,
    drill_state_id, drill_state_label, editor_segments, is_stale, is_undo_redo_chord,
    next_preview_window, partial_eval, readout, role_class, role_id, segment_lit_by_caret,
    segments_snapshot, segments_text, selection_from_dom, severity_label,
    should_consume_undo_redo_locally, stage_label, text_edited_from_dom, utf8_to_utf16,
    utf16_to_utf8,
};
use dnacalc_skin_ir::formula::{
    ArrayPreviewProjection, CompletionItemProjection, CompletionSurface,
    DiagnosticSeverityProjection, DiagnosticStageProjection, FormulaAssistSurface,
    FormulaDiagnosticProjection, FormulaDrillNodeProjection, FormulaDrillNodeStateProjection,
    FormulaDrillSurface, FormulaEditorSurface, SignatureHelpParameterProjection,
    SignatureHelpSurface, SyntaxRunProjection, SyntaxTokenRoleProjection,
};
use dnacalc_skin_ir::protocol::{SkinDocumentProjection, SkinSnapshot};
use dnacalc_skin_ir::workspace::{
    FormulaBindPreviewDiagnosticProjection, FormulaBindPreviewDiagnosticStage,
    GridEntryDiagnosticProjection, SourceSpanProjection,
};

const GOLDEN_FIXTURE: &str = include_str!("../../dnacalc-skin-ir/fixtures/one_formula.json");

fn run(
    text: &str,
    start: usize,
    len: usize,
    role: SyntaxTokenRoleProjection,
) -> SyntaxRunProjection {
    SyntaxRunProjection {
        text: text.to_string(),
        span_start: start,
        span_len: len,
        role,
    }
}

fn diag(
    id: &str,
    start: usize,
    len: usize,
    severity: DiagnosticSeverityProjection,
    stage: DiagnosticStageProjection,
    message: &str,
) -> FormulaDiagnosticProjection {
    FormulaDiagnosticProjection {
        diagnostic_id: id.to_string(),
        severity,
        stage,
        code: None,
        worksheet_error_class: None,
        message: message.to_string(),
        span_start: start,
        span_len: len,
    }
}

// ---------------------------------------------------------------------------
// Golden fixture consumption
// ---------------------------------------------------------------------------

/// The bridge consumes the real IR wire shape. The shipped fixture is an
/// empty OneCalc document, so its editor legitimately yields zero token
/// segments — the meaningful assertion is that deserialization succeeds and
/// the vm handles the empty surface without panic or fabricated tokens.
#[test]
fn golden_fixture_deserializes_and_yields_empty_token_underlay() {
    let snapshot: SkinSnapshot =
        serde_json::from_str(GOLDEN_FIXTURE).expect("golden fixture is a valid SkinSnapshot");
    let SkinDocumentProjection::OneFormula(formula) = snapshot.document else {
        panic!("golden fixture is a one_formula document");
    };
    let segments = editor_segments(&formula.editor);
    assert!(
        segments.is_empty(),
        "empty fixture source_text must produce no segments, got {segments:?}"
    );
    // The fixture's editor is marked not-fresh — the bridge renders it but
    // flags it (stale-surface law).
    assert!(is_stale(&formula.editor));
    // Reassembly identity holds trivially on the empty text.
    assert_eq!(segments_text(&segments), formula.editor.source_text);
}

// ---------------------------------------------------------------------------
// Token render snapshot
// ---------------------------------------------------------------------------

#[test]
fn token_render_snapshot_maps_runs_to_role_classes_and_reassembles_source() {
    let editor = FormulaEditorSurface {
        source_text: "=SUM(1,2)".to_string(),
        syntax_runs: vec![
            run("SUM", 1, 3, SyntaxTokenRoleProjection::Function),
            run("(", 4, 1, SyntaxTokenRoleProjection::Delimiter),
            run("1", 5, 1, SyntaxTokenRoleProjection::Number),
            run(",", 6, 1, SyntaxTokenRoleProjection::Delimiter),
            run("2", 7, 1, SyntaxTokenRoleProjection::Number),
            run(")", 8, 1, SyntaxTokenRoleProjection::Delimiter),
        ],
        document_is_fresh: true,
        ..Default::default()
    };
    let segments = editor_segments(&editor);
    // The leading "=" has no run: it renders as a plain (role=None) segment.
    assert_eq!(
        segments_snapshot(&segments),
        "[=|plain][SUM|function][(|delimiter][1|number][,|delimiter][2|number][)|delimiter]"
    );
    // Reassembly identity: the underlay renders the host's text byte-for-byte.
    assert_eq!(segments_text(&segments), "=SUM(1,2)");
    // The function run carries the role's class.
    let function_seg = segments.iter().find(|s| s.text == "SUM").unwrap();
    assert_eq!(function_seg.role, Some(SyntaxTokenRoleProjection::Function));
    assert_eq!(
        role_class(SyntaxTokenRoleProjection::Function),
        "dna-bridge__seg--role-function"
    );
}

// ---------------------------------------------------------------------------
// Caret / selection span mapping (UTF-8 <-> UTF-16), incl. multi-byte + astral
// ---------------------------------------------------------------------------

#[test]
fn caret_offset_maps_across_multibyte_and_surrogate_boundaries() {
    // "=A𝐀B": '𝐀' (U+1D400) is 4 UTF-8 bytes and 2 UTF-16 code units — the
    // case that separates a correct byte<->unit mapping from a naive
    // char-count.
    let text = "=A\u{1D400}B";
    assert_eq!(text.len(), 7, "4-byte astral char => 7 bytes total");

    // Byte 6 (the 'B') is UTF-16 unit 4 (=,A each 1; astral 2).
    assert_eq!(utf8_to_utf16(text, 6), 4);
    assert_eq!(utf16_to_utf8(text, 4), 6);

    // A UTF-16 offset landing *inside* the surrogate pair snaps forward to the
    // next character boundary (byte 6), never into the middle of the char.
    assert_eq!(utf16_to_utf8(text, 3), 6);

    // Round-trip every char boundary.
    for (byte_index, _) in text.char_indices() {
        let units = utf8_to_utf16(text, byte_index);
        assert_eq!(
            utf16_to_utf8(text, units),
            byte_index,
            "round-trip failed at byte {byte_index}"
        );
    }
}

#[test]
fn selection_from_dom_orders_anchor_focus_by_direction_over_multibyte() {
    // "=Ω+1": 'Ω' (U+03A9) is 2 UTF-8 bytes, 1 UTF-16 unit.
    let text = "=\u{03A9}+1";
    assert_eq!(text.len(), 5);
    // Select the "Ω+" run: UTF-16 units [1, 3) -> UTF-8 bytes [1, 4).
    let forward = selection_from_dom(text, 1, 3, false);
    assert_eq!(
        forward,
        BridgeEvent::SelectionSet {
            anchor: 1,
            focus: 4
        }
    );
    // Backward selection swaps anchor/focus so focus marks the caret end.
    let backward = selection_from_dom(text, 1, 3, true);
    assert_eq!(
        backward,
        BridgeEvent::SelectionSet {
            anchor: 4,
            focus: 1
        }
    );
}

// ---------------------------------------------------------------------------
// Diagnostics span math over multi-byte UTF-8 (the IR spans are byte offsets)
// ---------------------------------------------------------------------------

#[test]
fn diagnostic_spans_underline_the_correct_multibyte_slice() {
    // "=SÜM(Δ)": 'Ü' (U+00DC, 2 bytes) inside the function name, 'Δ'
    // (U+0394, 2 bytes) as the argument. Byte layout:
    //   = 0 | S 1 | Ü 2..4 | M 4 | ( 5 | Δ 6..8 | ) 8   => len 9
    let text = "=S\u{00DC}M(\u{0394})";
    assert_eq!(text.len(), 9);

    let editor = FormulaEditorSurface {
        source_text: text.to_string(),
        // Function run "SÜM" spans bytes 1..5 (len 4, not 3 — the Ü is 2 bytes).
        syntax_runs: vec![
            run("S\u{00DC}M", 1, 4, SyntaxTokenRoleProjection::Function),
            run("\u{0394}", 6, 2, SyntaxTokenRoleProjection::Identifier),
        ],
        // A Bind-stage error underlines the argument 'Δ' at bytes 6..8.
        diagnostics: vec![diag(
            "d1",
            6,
            2,
            DiagnosticSeverityProjection::Error,
            DiagnosticStageProjection::Bind,
            "unknown name",
        )],
        document_is_fresh: true,
        ..Default::default()
    };
    let segments = editor_segments(&editor);
    assert_eq!(
        segments_text(&segments),
        text,
        "reassembly must be byte-exact"
    );

    // The function segment is exactly "SÜM" (proves the 4-byte span sliced
    // the multibyte char whole, not mid-codepoint).
    let function_seg = segments
        .iter()
        .find(|s| s.role == Some(SyntaxTokenRoleProjection::Function))
        .expect("function segment present");
    assert_eq!(function_seg.text, "S\u{00DC}M");
    assert_eq!((function_seg.byte_start, function_seg.byte_end), (1, 5));

    // The diagnostic underline lands on exactly "Δ" — not "(" and not ")".
    let underlined: Vec<&str> = segments
        .iter()
        .filter(|s| s.diag == Some(DiagnosticSeverityProjection::Error))
        .map(|s| s.text.as_str())
        .collect();
    assert_eq!(underlined, vec!["\u{0394}"]);

    // Verbatim IR passthrough of the row form (no reclassification).
    assert_eq!(severity_label(DiagnosticSeverityProjection::Error), "Error");
    assert_eq!(stage_label(DiagnosticStageProjection::Bind), "Bind");
    assert_eq!(
        stage_label(DiagnosticStageProjection::SemanticPlan),
        "SemanticPlan"
    );
}

#[test]
fn overlapping_diagnostics_take_the_strongest_severity_per_slice() {
    let editor = FormulaEditorSurface {
        source_text: "=ABCDE".to_string(),
        diagnostics: vec![
            diag(
                "warn",
                1,
                4,
                DiagnosticSeverityProjection::Warning,
                DiagnosticStageProjection::Bind,
                "broad warning",
            ),
            diag(
                "err",
                2,
                1,
                DiagnosticSeverityProjection::Error,
                DiagnosticStageProjection::Syntax,
                "narrow error",
            ),
        ],
        document_is_fresh: true,
        ..Default::default()
    };
    let segments = editor_segments(&editor);
    // The single byte covered by both marks reports Error (rank > Warning).
    let strongest = segments
        .iter()
        .find(|s| s.byte_start == 2)
        .and_then(|s| s.diag);
    assert_eq!(strongest, Some(DiagnosticSeverityProjection::Error));
}

// ---------------------------------------------------------------------------
// Completion payload correctness (all 7 kinds, wrap navigation, id passthrough)
// ---------------------------------------------------------------------------

fn completion_with_all_kinds() -> CompletionSurface {
    CompletionSurface {
        anchor_left_px: 40,
        anchor_top_px: 80,
        line_height_px: 18,
        selected_index: 0,
        items: ALL_COMPLETION_KINDS
            .iter()
            .enumerate()
            .map(|(index, kind)| CompletionItemProjection {
                proposal_id: format!("p{index}"),
                display_text: format!("item-{}", completion_kind_id(*kind)),
                kind: *kind,
                documentation_ref: None,
            })
            .collect(),
    }
}

#[test]
fn completion_apply_carries_proposal_id_verbatim_never_display_text() {
    let surface = completion_with_all_kinds();
    // Applying index 3 emits exactly that item's proposal_id.
    let event = completion_applied(&surface.items, 3).expect("index 3 exists");
    assert_eq!(
        event,
        BridgeEvent::CompletionApplied {
            proposal_id: "p3".to_string()
        }
    );
    // Out-of-range yields nothing (no fabricated proposal).
    assert_eq!(completion_applied(&surface.items, 99), None);
}

#[test]
fn completion_keyboard_navigation_wraps_at_both_ends() {
    let count = ALL_COMPLETION_KINDS.len(); // 7
    assert_eq!(
        completion_next(count, 0, -1),
        count - 1,
        "up from top wraps to bottom"
    );
    assert_eq!(
        completion_next(count, count - 1, 1),
        0,
        "down from bottom wraps to top"
    );
    assert_eq!(completion_next(count, 2, 1), 3);
    assert_eq!(completion_next(count, 2, -1), 1);
    // Empty list pins to 0 (no panic, no underflow).
    assert_eq!(completion_next(0, 0, 1), 0);
}

#[test]
fn all_seven_completion_kinds_render_distinctly() {
    use std::collections::HashSet;
    let ids: HashSet<_> = ALL_COMPLETION_KINDS
        .iter()
        .map(|k| completion_kind_id(*k))
        .collect();
    let glyphs: HashSet<_> = ALL_COMPLETION_KINDS
        .iter()
        .map(|k| completion_kind_glyph(*k))
        .collect();
    assert_eq!(ids.len(), 7, "all 7 kind ids are distinct");
    assert_eq!(
        glyphs.len(),
        7,
        "all 7 kind glyphs are distinct (non-color cue)"
    );
}

// ---------------------------------------------------------------------------
// Readout row (signature help + caret-in-drill)
// ---------------------------------------------------------------------------

#[test]
fn readout_summarizes_signature_and_drill_node_under_caret() {
    let assist = FormulaAssistSurface {
        signature_help: Some(SignatureHelpSurface {
            callee_text: "XLOOKUP".to_string(),
            anchor_left_px: 0,
            anchor_top_px: 0,
            line_height_px: 18,
            parameters: vec![
                SignatureHelpParameterProjection {
                    name: "lookup_value".to_string(),
                    is_active: false,
                },
                SignatureHelpParameterProjection {
                    name: "lookup_array".to_string(),
                    is_active: true,
                },
            ],
            active_parameter: Some(1),
        }),
        ..Default::default()
    };
    let drill = FormulaDrillSurface {
        expanded: true,
        tree: vec![FormulaDrillNodeProjection {
            node_id: "n1".to_string(),
            label: "Rates[ISO]".to_string(),
            developer_label: None,
            expression_text: Some("Rates[ISO]".to_string()),
            kind: Some("reference".to_string()),
            source_span_start: Some(1),
            source_span_len: Some(10),
            branch_disposition: None,
            argument_name: None,
            argument_role: None,
            error_message: None,
            value_preview: Some("21 values".to_string()),
            array_preview: Some(ArrayPreviewProjection {
                total_rows: 21,
                total_cols: 1,
                row_offset: 0,
                col_offset: 0,
                rows: vec![vec!["ZAR".to_string()]],
                truncated: true,
            }),
            state: FormulaDrillNodeStateProjection::Evaluated,
            children: vec![],
        }],
        diagnostics: vec![],
        phase_summaries: vec![],
        document_is_fresh: true,
    };
    // Caret at byte 5 sits inside the reference span [1, 11).
    let vm = readout(&assist, &drill, 5);
    assert_eq!(vm.callee.as_deref(), Some("XLOOKUP"));
    assert_eq!(vm.active_parameter.as_deref(), Some("lookup_array"));
    assert_eq!(vm.target_label.as_deref(), Some("Rates[ISO]"));
    assert_eq!(vm.value_preview.as_deref(), Some("21 values"));
    assert_eq!(vm.shape.as_deref(), Some("21\u{00D7}1"));

    // A collapsed drill contributes no target/preview (host truth only when
    // expanded), but signature help still shows.
    let collapsed = FormulaDrillSurface {
        expanded: false,
        ..drill.clone()
    };
    let vm2 = readout(&assist, &collapsed, 5);
    assert_eq!(vm2.callee.as_deref(), Some("XLOOKUP"));
    assert_eq!(vm2.target_label, None);

    // Caret-in-node resolution finds the deepest containing node.
    let hit = drill_node_at_caret(&drill.tree, 5).expect("node under caret");
    assert_eq!(hit.node_id, "n1");
    assert_eq!(
        drill_node_at_caret(&drill.tree, 0),
        None,
        "caret before span => no node"
    );
}

#[test]
fn truncated_preview_pages_next_window_within_the_64_cap() {
    let preview = ArrayPreviewProjection {
        total_rows: 100,
        total_cols: 1,
        row_offset: 0,
        col_offset: 0,
        rows: vec![vec!["1".to_string()]],
        truncated: true,
    };
    let event = next_preview_window(&preview).expect("more rows remain");
    assert_eq!(
        event,
        BridgeEvent::ArrayWindowRequested {
            row_offset: 1,
            col_offset: 0,
            row_count: 64, // clamped to the 64-edge cap, not 99
            col_count: 1,
        }
    );
    // A fully-covered preview pages nowhere.
    let done = ArrayPreviewProjection {
        total_rows: 1,
        total_cols: 1,
        rows: vec![vec!["x".to_string()]],
        truncated: false,
        ..preview
    };
    assert_eq!(next_preview_window(&done), None);
}

// ---------------------------------------------------------------------------
// Partial-evaluation pill (BENCH_SPEC §4): select subexpression -> value/shape
// ---------------------------------------------------------------------------

/// A nested drill tree over `=SUM(1,2,3)` for the pill/selection tests:
/// FormulaRoot [0,11) ⊃ FunctionCall [1,10) ⊃ Argument [5,6) ⊃ Literal [5,6).
fn nested_drill_tree() -> FormulaDrillSurface {
    let literal = FormulaDrillNodeProjection {
        node_id: "lit-1".into(),
        label: "1 = 1".into(),
        developer_label: None,
        expression_text: Some("1".into()),
        kind: Some("Literal".into()),
        source_span_start: Some(5),
        source_span_len: Some(1),
        branch_disposition: None,
        argument_name: None,
        argument_role: None,
        error_message: None,
        value_preview: Some("1".into()),
        array_preview: None,
        state: FormulaDrillNodeStateProjection::Evaluated,
        children: vec![],
    };
    let arg = FormulaDrillNodeProjection {
        node_id: "arg-1".into(),
        label: "number1: 1".into(),
        developer_label: None,
        expression_text: Some("1".into()),
        kind: Some("Argument".into()),
        source_span_start: Some(5),
        source_span_len: Some(1),
        branch_disposition: None,
        argument_name: Some("number1".into()),
        argument_role: Some("Number".into()),
        error_message: None,
        value_preview: None,
        array_preview: None,
        state: FormulaDrillNodeStateProjection::Bound,
        children: vec![literal],
    };
    let call = FormulaDrillNodeProjection {
        node_id: "call-1".into(),
        label: "SUM".into(),
        developer_label: None,
        expression_text: Some("SUM(1,2,3)".into()),
        kind: Some("FunctionCall".into()),
        source_span_start: Some(1),
        source_span_len: Some(9),
        branch_disposition: None,
        argument_name: None,
        argument_role: None,
        error_message: None,
        value_preview: None,
        array_preview: None,
        state: FormulaDrillNodeStateProjection::Evaluated,
        children: vec![arg],
    };
    let root = FormulaDrillNodeProjection {
        node_id: "root-1".into(),
        label: "Formula = 6".into(),
        developer_label: None,
        expression_text: Some("=SUM(1,2,3)".into()),
        kind: Some("FormulaRoot".into()),
        source_span_start: Some(0),
        source_span_len: Some(11),
        branch_disposition: None,
        argument_name: None,
        argument_role: None,
        error_message: None,
        value_preview: Some("6".into()),
        array_preview: None,
        state: FormulaDrillNodeStateProjection::Evaluated,
        children: vec![call],
    };
    FormulaDrillSurface {
        expanded: true,
        tree: vec![root],
        diagnostics: vec![],
        phase_summaries: vec![],
        document_is_fresh: true,
    }
}

#[test]
fn partial_eval_pill_matches_the_deepest_node_covering_the_selection() {
    let drill = nested_drill_tree();

    // Selecting the literal `1` (bytes [5,6)) resolves the innermost node.
    let pill = partial_eval(&drill, 5, 6).expect("selection matches a node");
    assert_eq!(pill.node_id, "lit-1", "deepest containing node wins");
    assert_eq!(pill.expression, "1");
    assert_eq!(pill.value_preview.as_deref(), Some("1"));
    assert_eq!(pill.type_label, "Literal");
    assert_eq!(pill.state, FormulaDrillNodeStateProjection::Evaluated);
    assert_eq!(pill.shape, None, "a scalar node has no array shape");

    // Widening the selection to the whole call resolves the call node.
    let pill = partial_eval(&drill, 1, 10).expect("call selection matches");
    assert_eq!(pill.node_id, "call-1");
    assert_eq!(pill.expression, "SUM(1,2,3)");

    // A selection that no node fully covers falls back to the enclosing node.
    let pill = partial_eval(&drill, 5, 7).expect("enclosing node matches");
    assert_eq!(pill.node_id, "call-1");

    // A collapsed selection yields no pill (the readout row handles the caret).
    assert_eq!(partial_eval(&drill, 5, 5), None);
    // Order-independence: anchor after focus resolves the same span.
    assert_eq!(
        partial_eval(&drill, 6, 5).map(|p| p.node_id),
        Some("lit-1".to_string())
    );
    // Direct span matcher agrees.
    assert_eq!(
        drill_node_for_selection(&drill.tree, 5, 6).map(|n| n.node_id.as_str()),
        Some("lit-1")
    );
    assert_eq!(drill_node_for_selection(&drill.tree, 5, 5), None);
}

/// Pills carry array shape for a node whose value spills (mechanism 20: the
/// pill's `node_id` equals the drill row's, so pill and row point at one node).
#[test]
fn partial_eval_pill_reports_array_shape_for_a_spilling_node() {
    let mut drill = nested_drill_tree();
    drill.tree[0].array_preview = Some(ArrayPreviewProjection {
        total_rows: 21,
        total_cols: 1,
        row_offset: 0,
        col_offset: 0,
        rows: vec![vec!["ZAR".into()]],
        truncated: true,
    });
    let pill = partial_eval(&drill, 0, 11).expect("root selection matches");
    assert_eq!(pill.node_id, "root-1");
    assert_eq!(pill.shape.as_deref(), Some("21\u{00D7}1"));
}

/// Reference X-Ray (mechanism 07): only the segment the caret rests in lights.
#[test]
fn caret_lights_only_the_token_it_rests_in() {
    let editor = FormulaEditorSurface {
        source_text: "=SUM".into(),
        caret_offset: 2,
        selection_anchor: 2,
        selection_focus: 2,
        syntax_runs: vec![
            SyntaxRunProjection {
                text: "=".into(),
                span_start: 0,
                span_len: 1,
                role: SyntaxTokenRoleProjection::Operator,
            },
            SyntaxRunProjection {
                text: "SUM".into(),
                span_start: 1,
                span_len: 3,
                role: SyntaxTokenRoleProjection::Function,
            },
        ],
        diagnostics: vec![],
        metrics: Default::default(),
        document_is_fresh: true,
    };
    let segments = editor_segments(&editor);
    // caret 2 is inside `SUM` [1,4), not `=` [0,1).
    let lit: Vec<bool> = segments
        .iter()
        .map(|s| segment_lit_by_caret(s, 2))
        .collect();
    assert_eq!(lit, vec![false, true]);
    // At the boundary byte 1, the caret belongs to the *next* token.
    assert!(!segment_lit_by_caret(&segments[0], 1));
    assert!(segment_lit_by_caret(&segments[1], 1));
    // Caret at 0 lights the leading operator.
    assert!(segment_lit_by_caret(&segments[0], 0));
}

#[test]
fn drill_state_vocabulary_is_total_and_stable() {
    use FormulaDrillNodeStateProjection as S;
    let all = [
        (S::Pending, "pending", "Pending"),
        (S::Evaluated, "evaluated", "Evaluated"),
        (S::Bound, "bound", "Bound"),
        (S::Skipped, "skipped", "Skipped"),
        (S::Opaque, "opaque", "Opaque"),
        (S::Blocked, "blocked", "Blocked"),
        (S::Error, "error", "Error"),
    ];
    for (state, id, label) in all {
        assert_eq!(drill_state_id(state), id);
        assert_eq!(drill_state_label(state), label);
    }
}

// ---------------------------------------------------------------------------
// Degrade mode: ZERO token-role classes (SHELL_SPEC §6)
// ---------------------------------------------------------------------------

#[test]
fn degrade_segments_never_carry_a_token_role() {
    let rejections = vec![
        GridEntryDiagnosticProjection {
            message: "circular reference".to_string(),
            span: Some((1, 4)),
        },
        GridEntryDiagnosticProjection {
            // A span-less rejection contributes no underline (message-only).
            message: "engine unavailable".to_string(),
            span: None,
        },
    ];
    let dry_bind = vec![FormulaBindPreviewDiagnosticProjection {
        stage: FormulaBindPreviewDiagnosticStage::Bind,
        message: "name not found".to_string(),
        span: SourceSpanProjection {
            start_utf8: 5,
            end_utf8: 8,
        },
    }];
    let segments = degrade_segments("=ABC+DE", &rejections, &dry_bind);
    // The core degrade law: no segment ever carries a token role.
    assert!(
        segments.iter().all(|s| s.role.is_none()),
        "degrade mode must render ZERO token roles, got {segments:?}"
    );
    // The role-class marker string never appears in any degrade class name.
    for role in [
        SyntaxTokenRoleProjection::Function,
        SyntaxTokenRoleProjection::Identifier,
        SyntaxTokenRoleProjection::Number,
    ] {
        let class = role_class(role);
        assert!(
            class.contains("--role-"),
            "sanity: full-mode class has the marker"
        );
    }
    // Reassembly still exact.
    assert_eq!(segments_text(&segments), "=ABC+DE");
    // The rejection span [1,4) underlines as Error; the dry-bind span [5,8)
    // underlines as Warning — distinct severities, distinct from any token role.
    let error_slices: Vec<&str> = segments
        .iter()
        .filter(|s| s.diag == Some(DiagnosticSeverityProjection::Error))
        .map(|s| s.text.as_str())
        .collect();
    assert_eq!(error_slices.concat(), "ABC");
    let warn_slices: Vec<&str> = segments
        .iter()
        .filter(|s| s.diag == Some(DiagnosticSeverityProjection::Warning))
        .map(|s| s.text.as_str())
        .collect();
    assert_eq!(warn_slices.concat(), "DE");
}

// ---------------------------------------------------------------------------
// Verbatim passthrough: arbitrary pasted input reaches the event byte-for-byte
// ---------------------------------------------------------------------------

#[test]
fn text_edited_passes_input_through_byte_for_byte() {
    // The bridge NEVER tokenizes or `=`-sniffs: whatever the DOM holds is the
    // event's text, exactly. Cover leading `=`, no `=`, non-ASTRAL and astral
    // multibyte, embedded newlines, quotes, and control chars.
    let inputs = [
        "=SUM(1,2,3)",
        "not a formula at all",
        "=XLOOKUP(Code, Rates[ISO], Rates[Fx])",
        "   =leading spaces kept",
        "=Ω+π+\u{1D400}",       // multibyte + astral
        "line1\nline2\ttabbed", // control chars
        "=\"embedded = signs & quotes\"",
        "", // empty
        "==double equals==",
    ];
    for input in inputs {
        let event = text_edited_from_dom(input.to_string(), 0);
        let BridgeEvent::TextEdited { text, .. } = event else {
            panic!("expected TextEdited for {input:?}");
        };
        assert_eq!(text, input, "input must survive byte-for-byte: {input:?}");
    }
}

#[test]
fn text_edited_caret_maps_utf16_units_to_utf8_bytes() {
    // Caret reported by the DOM in UTF-16 units becomes a UTF-8 byte offset.
    let text = "=\u{03A9}\u{03A9}"; // "=ΩΩ": bytes 0,1..3,3..5 => len 5; utf16 units 3
    let event = text_edited_from_dom(text.to_string(), 3);
    assert_eq!(
        event,
        BridgeEvent::TextEdited {
            text: text.to_string(),
            caret: 5, // end of the 5-byte string
        }
    );
}

// ---------------------------------------------------------------------------
// Edit discipline (modeless 1-bit) + stale flag
// ---------------------------------------------------------------------------

#[test]
fn edit_discipline_is_one_bit() {
    assert_eq!(EditDiscipline::default(), EditDiscipline::Selected);
    assert_ne!(
        EditDiscipline::Selected.as_str(),
        EditDiscipline::Editing.as_str()
    );
}

#[test]
fn role_ids_are_stable_and_total() {
    for role in [
        SyntaxTokenRoleProjection::Operator,
        SyntaxTokenRoleProjection::Function,
        SyntaxTokenRoleProjection::Number,
        SyntaxTokenRoleProjection::Delimiter,
        SyntaxTokenRoleProjection::Identifier,
        SyntaxTokenRoleProjection::Text,
        SyntaxTokenRoleProjection::Trivia,
    ] {
        assert!(!role_id(role).is_empty());
        assert!(role_class(role).ends_with(role_id(role)));
    }
}

// ---------------------------------------------------------------------------
// Undo/redo carve-out (bead dtc-lfz.2 / S1.1, owner-ratified 2026-07-12):
// the "dirty" predicate + the chord recognizer + the combined decision the
// editor's keydown handler gates `stop_propagation()` on.
// ---------------------------------------------------------------------------

#[test]
fn buffer_is_dirty_compares_local_buffer_against_committed_source() {
    // Clean: buffer matches the host's committed source_text exactly,
    // including the empty-vs-empty edge (a freshly mounted, untouched editor).
    assert!(!buffer_is_dirty("", ""));
    assert!(!buffer_is_dirty("=SUM(1,2)", "=SUM(1,2)"));
    // Dirty: any divergence at all, not just a length change.
    assert!(buffer_is_dirty("=SUM(1,2,3)", "=SUM(1,2)"));
    assert!(buffer_is_dirty("", "=A1"));
    assert!(buffer_is_dirty("=A1", ""));
    // Same length, different content — the naive "did the length change?"
    // check a weaker implementation might use would wrongly call this clean.
    assert!(buffer_is_dirty("=A2", "=A1"));
}

#[test]
fn is_undo_redo_chord_recognizes_ctrl_z_y_shift_z_case_insensitively() {
    // Ctrl+Z / Ctrl+z (undo) and Ctrl+Y / Ctrl+y (redo): ctrl, no alt.
    assert!(is_undo_redo_chord("z", true, false));
    assert!(is_undo_redo_chord("Z", true, false));
    assert!(is_undo_redo_chord("y", true, false));
    assert!(is_undo_redo_chord("Y", true, false));
    // Ctrl+Shift+Z arrives as key="Z" (Shift capitalizes on common layouts);
    // shift itself doesn't disqualify — the recognizer doesn't need shift to
    // decide whether the carve-out applies, only ctrl/alt/letter.
    assert!(is_undo_redo_chord("Z", true, false));

    // Not recognized: no Ctrl at all (plain typing of the letter).
    assert!(!is_undo_redo_chord("z", false, false));
    assert!(!is_undo_redo_chord("y", false, false));
    // Not recognized: Alt held (never an undo/redo chord per SHELL_SPEC §5.1).
    assert!(!is_undo_redo_chord("z", true, true));
    assert!(!is_undo_redo_chord("y", true, true));
    // Not recognized: any other letter, even with Ctrl (e.g. Ctrl+K, Ctrl+S).
    assert!(!is_undo_redo_chord("k", true, false));
    assert!(!is_undo_redo_chord("s", true, false));
    assert!(!is_undo_redo_chord("x", true, false));
}

/// dtc-j7n8.25 — the degrade editor's keydown policy. Its `on_keydown` used to
/// `stop_propagation()` on EVERY key before matching, so Ctrl+S typed inside
/// the Sheet stage's overlay editor (which owns focus since dtc-j7n8.26) never
/// reached the shell's Save verb. Only the keys the editor consumes may stop;
/// a universal shell chord (Ctrl+S / Ctrl+O / Ctrl+K / F9) must bubble.
#[test]
fn degrade_editor_lets_shell_chords_bubble_and_stops_only_what_it_consumes() {
    use DegradeKeyDisposition::{Bubble, CommitDown, CommitRight, ConsumeUndoRedoLocally, Revert};

    // The bead's chord: Ctrl+S from a dirty or clean buffer, on or off a grid,
    // is NOT the editor's — it must bubble to the shell (the Save verb).
    for (commit_on_tab, dirty) in [(true, true), (true, false), (false, true), (false, false)] {
        assert_eq!(
            degrade_key_disposition("s", false, true, false, commit_on_tab, dirty),
            Bubble,
            "Ctrl+S must bubble past the degrade editor (commit_on_tab={commit_on_tab}, dirty={dirty})"
        );
    }
    // The rest of the shell's exemption class bubbles too.
    assert_eq!(
        degrade_key_disposition("o", false, true, false, true, true),
        Bubble
    );
    assert_eq!(
        degrade_key_disposition("k", false, true, false, true, true),
        Bubble
    );
    assert_eq!(
        degrade_key_disposition("F9", false, false, false, true, true),
        Bubble
    );
    // Plain typing and Shift+Enter (a newline) are not consumed here either —
    // the shell's text-entry guard suppresses them downstream.
    assert_eq!(
        degrade_key_disposition("a", false, false, false, true, true),
        Bubble
    );
    assert_eq!(
        degrade_key_disposition("Enter", true, false, false, true, true),
        Bubble
    );

    // What the editor DOES consume, exactly as before.
    assert_eq!(
        degrade_key_disposition("Enter", false, false, false, false, false),
        CommitDown
    );
    assert_eq!(
        degrade_key_disposition("Tab", false, false, false, true, false),
        CommitRight
    );
    assert_eq!(
        degrade_key_disposition("Tab", false, false, false, false, false),
        Bubble,
        "off a grid Tab keeps its browser focus-move"
    );
    assert_eq!(
        degrade_key_disposition("Tab", true, false, false, true, false),
        Bubble,
        "Shift+Tab is deliberately not captured"
    );
    assert_eq!(
        degrade_key_disposition("Escape", false, false, false, true, true),
        Revert
    );

    // The dtc-lfz.2 carve-out: Ctrl+Z/Y/Shift+Z are text-local only while dirty.
    assert_eq!(
        degrade_key_disposition("z", false, true, false, true, true),
        ConsumeUndoRedoLocally
    );
    assert_eq!(
        degrade_key_disposition("Z", true, true, false, true, true),
        ConsumeUndoRedoLocally
    );
    assert_eq!(
        degrade_key_disposition("y", false, true, false, true, true),
        ConsumeUndoRedoLocally
    );
    assert_eq!(
        degrade_key_disposition("z", false, true, false, true, false),
        Bubble,
        "a clean buffer hands Ctrl+Z to the shell's model Undo"
    );
}

#[test]
fn should_consume_undo_redo_locally_gates_the_chord_on_dirty() {
    // The chord matches AND the buffer is dirty: consume locally.
    assert!(should_consume_undo_redo_locally("z", true, false, true));
    assert!(should_consume_undo_redo_locally("y", true, false, true));
    assert!(should_consume_undo_redo_locally("Z", true, false, true));

    // The chord matches but the buffer is CLEAN: must NOT consume — this is
    // the exact case that must bubble so the shell's Undo/Redo verb fires.
    assert!(!should_consume_undo_redo_locally("z", true, false, false));
    assert!(!should_consume_undo_redo_locally("y", true, false, false));

    // The buffer is dirty but the chord isn't an undo/redo chord: no effect
    // on unrelated keys (Ctrl+K, Ctrl+S, plain letters) regardless of dirt.
    assert!(!should_consume_undo_redo_locally("k", true, false, true));
    assert!(!should_consume_undo_redo_locally("s", true, false, true));
    assert!(!should_consume_undo_redo_locally("z", false, false, true));

    // Dirty + Ctrl+Alt+Z: Alt disqualifies regardless of dirt.
    assert!(!should_consume_undo_redo_locally("z", true, true, true));
}
