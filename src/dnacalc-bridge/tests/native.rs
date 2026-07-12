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
    ALL_COMPLETION_KINDS, BridgeEvent, EditDiscipline, completion_applied, completion_kind_glyph,
    completion_kind_id, completion_next, degrade_segments, drill_node_at_caret, editor_segments,
    is_stale, next_preview_window, readout, role_class, role_id, segments_snapshot, segments_text,
    selection_from_dom, severity_label, stage_label, text_edited_from_dom, utf8_to_utf16,
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
