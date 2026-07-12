use crate::adapters::oxfml::{
    EditorDocument, FormulaEditRequest, FormulaInputBindingRequest, FormulaResultViewModel,
    OxfmlHostSession, OxfmlHostSessionError,
};
use crate::app::intents::ApplyFormulaEditIntent;
use crate::domain::ids::FormulaSpaceId;
use crate::services::wall_clock::wall_clock_now_ms;
use crate::state::{
    CompletionHelpState, FormulaArrayPreviewState, FormulaSpaceCollectionState, FormulaSpaceState,
    ProjectionTruthSource,
};
use crate::ui::editor::state::EditorSurfaceState;

#[derive(Debug, Default)]
pub struct EditorSessionService;

impl EditorSessionService {
    pub fn handle_formula_edit_intent(
        bridge: &dyn OxfmlHostSession,
        formula_spaces: &mut FormulaSpaceCollectionState,
        intent: ApplyFormulaEditIntent,
    ) -> Result<(), EditorSessionError> {
        let formula_space = formula_spaces
            .get(&intent.formula_space_id)
            .ok_or_else(|| {
                EditorSessionError::UnknownFormulaSpace(intent.formula_space_id.clone())
            })?;
        let skip_runtime_evaluation = intent.skip_runtime_evaluation;
        // Snapshot the runtime-derived fields of the prior document
        // before the bridge call. When the bridge runs without a
        // runtime pass (caret-only navigation, ManualRecalc, or
        // auto-debounce), it reports `value_presentation = None`
        // and a single-node fallback `formula_walk`. The visible
        // result hero would blank out unless we restore these
        // fields from the previous document. The text and
        // syntax-overlay fields always come from the new document
        // (they reflect the new caret position / latest parse).
        let previous_runtime_snapshot = formula_space
            .editor_document
            .as_ref()
            .map(SkippedRuntimeFields::capture);
        let request = FormulaEditRequest {
            formula_stable_id: intent.formula_stable_id,
            entered_text: intent.entered_text,
            cursor_offset: intent.cursor_offset,
            previous_green_tree_key: formula_space
                .editor_document
                .as_ref()
                .map(|document| document.green_tree_key().to_string()),
            analysis_stage: intent.analysis_stage,
            formatting_request: intent.formatting_request,
            scenario_policy: intent.scenario_policy,
            skip_runtime_evaluation,
            recalc_mode: intent.recalc_mode,
            language_tag: intent.language_tag,
            formal_input_bindings: formula_space
                .formula_input_bindings
                .iter()
                .map(|binding| FormulaInputBindingRequest {
                    label: binding.label.clone(),
                    reference_descriptor: binding.reference_descriptor.clone(),
                    reference_handle: binding.reference_handle.clone(),
                    value: binding.value.clone(),
                })
                .collect(),
            trace_mode: intent.trace_mode,
        };
        // Wall-clock the bridge round-trip so the host can detect
        // "expensive runtime pass" and flip into auto-debounced
        // typing. We only record the elapsed time when the runtime
        // pass actually ran; caret-only events (skip_runtime=true)
        // would skew the timing low. The bridge's input-equality
        // short-circuit can also return in microseconds; that's a
        // legitimate cheap-pass observation we want to capture so
        // a fast cached round-trip clears the auto-debounce flag.
        let start_ms = wall_clock_now_ms();
        let result = bridge
            .apply_formula_edit(request)
            .map_err(EditorSessionError::Bridge)?;
        let elapsed_ms = wall_clock_now_ms() - start_ms;
        let mut document = result.document;
        if skip_runtime_evaluation {
            if let Some(snapshot) = previous_runtime_snapshot.as_ref() {
                snapshot.restore_into(&mut document);
            }
        }
        Self::apply_editor_document(formula_spaces, &intent.formula_space_id, document)?;
        if !skip_runtime_evaluation {
            if let Some(formula_space) = formula_spaces.get_mut(&intent.formula_space_id) {
                formula_space.last_bridge_pass_elapsed_ms = Some(elapsed_ms);
                formula_space.pending_runtime_recalc = false;
            }
        }
        Ok(())
    }

    pub fn apply_editor_document(
        formula_spaces: &mut FormulaSpaceCollectionState,
        formula_space_id: &FormulaSpaceId,
        document: EditorDocument,
    ) -> Result<(), EditorSessionError> {
        let formula_space = formula_spaces
            .get_mut(formula_space_id)
            .ok_or_else(|| EditorSessionError::UnknownFormulaSpace(formula_space_id.clone()))?;
        update_formula_space_from_editor_document(formula_space, document);
        Ok(())
    }
}

/// Snapshot of the runtime-derived fields on an `EditorDocument`
/// taken before a skip-runtime bridge round-trip. Restored into
/// the bridge's response so caret-only navigation (mouse click,
/// arrow keys) and auto-debounced typing don't blank out the
/// result hero.
///
/// The bridge's skip-runtime path produces a single-node
/// fallback `formula_walk` and clears `value_presentation` /
/// `eval_summary` / `provenance_summary` to "edit-only" defaults
/// (see `live_bridge::build_editor_document`). For the user, the
/// invariant we want to preserve is "the result they were just
/// looking at stays on screen until the next runtime pass either
/// confirms or replaces it". This struct is the ferry that holds
/// that data.
#[derive(Debug, Clone, PartialEq)]
struct SkippedRuntimeFields {
    value_presentation: Option<FormulaResultViewModel>,
    formula_walk: Vec<crate::adapters::oxfml::FormulaDrillNodeViewModel>,
    eval_summary: Option<crate::adapters::oxfml::EvalSummary>,
    bind_summary: Option<crate::adapters::oxfml::BindSummary>,
    provenance_summary: Option<crate::adapters::oxfml::ProvenanceSummary>,
}

impl SkippedRuntimeFields {
    fn capture(previous: &EditorDocument) -> Self {
        Self {
            value_presentation: previous.value_presentation.clone(),
            formula_walk: previous.formula_walk.clone(),
            eval_summary: previous.eval_summary.clone(),
            bind_summary: previous.bind_summary.clone(),
            provenance_summary: previous.provenance_summary.clone(),
        }
    }

    /// Restore the snapshot only when the new document carries
    /// the bridge's edit-only defaults. If the new document
    /// already has runtime data (e.g. the bridge ran runtime
    /// despite our `skip_runtime_evaluation = true` request, or
    /// the bridge implementation grew a path that preserves
    /// runtime fields itself), we leave it alone — the new
    /// document is by definition the freshest truth.
    fn restore_into(&self, document: &mut EditorDocument) {
        if document.value_presentation.is_none() && self.value_presentation.is_some() {
            document.value_presentation = self.value_presentation.clone();
        }
        // The bridge's edit-only fallback `formula_walk` is a
        // single CellEntry node. Recognise that shape and replace
        // it with the previous walk; otherwise keep the new walk
        // (it might carry richer parse data the previous didn't).
        if document.formula_walk.len() <= 1
            && document
                .formula_walk
                .first()
                .map(|node| node.label == "CellEntry")
                .unwrap_or(true)
            && !self.formula_walk.is_empty()
        {
            document.formula_walk = self.formula_walk.clone();
        }
        if let Some(eval_summary) = document.eval_summary.as_ref() {
            if eval_summary.duration_text == "edit-only" {
                document.eval_summary = self.eval_summary.clone();
            }
        }
        if let Some(bind_summary) = document.bind_summary.as_ref() {
            if bind_summary.reference_count == 0 {
                if let Some(prev_bind) = self.bind_summary.as_ref() {
                    if prev_bind.reference_count > 0 {
                        document.bind_summary = self.bind_summary.clone();
                    }
                }
            }
        }
        if let Some(provenance_summary) = document.provenance_summary.as_ref() {
            if provenance_summary.profile_summary == "OxFml editor"
                && self.provenance_summary.is_some()
            {
                document.provenance_summary = self.provenance_summary.clone();
            }
        }
    }
}

fn update_formula_space_from_editor_document(
    formula_space: &mut FormulaSpaceState,
    document: EditorDocument,
) {
    let truth_source = infer_truth_source(&document);
    let mut editor_surface_state = EditorSurfaceState::for_text_with_selection(
        &document.source_text,
        formula_space.editor_surface_state.selection.anchor,
        formula_space.editor_surface_state.selection.focus,
    );
    editor_surface_state.scroll_window = formula_space.editor_surface_state.scroll_window.clone();
    editor_surface_state.completion_anchor_offset = None;
    editor_surface_state.completion_selected_index =
        (!document.completion_proposals.is_empty()).then_some(0);
    editor_surface_state.signature_help_anchor_offset = None;

    formula_space.raw_entered_cell_text = document.source_text.clone();
    formula_space.editor_surface_state = editor_surface_state;
    formula_space.completion_help = CompletionHelpState {
        completion_count: document.completion_proposals.len(),
        has_signature_help: document.signature_help.is_some(),
        function_help_lookup_key: document
            .function_help
            .as_ref()
            .map(|packet| packet.lookup_key.clone()),
    };
    let derived_presentation = derive_formula_presentation(&document.source_text, &document);
    formula_space.editor_document = Some(document);
    formula_space.latest_evaluation_summary = derived_presentation.evaluation_summary;
    formula_space.effective_display_summary = derived_presentation.effective_display_summary;
    formula_space.array_preview = derived_presentation.array_preview;
    formula_space.context.truth_source = truth_source;
    if let Some(blocked_reason) = derived_presentation.blocked_reason {
        formula_space.context.blocked_reason = Some(blocked_reason);
    }
    // Hint-application is performed by the live-edit layer once the
    // bridge returns — that layer has access to the workspace's
    // `AmbientAppContext`, which the projection here doesn't.
}

fn infer_truth_source(document: &EditorDocument) -> ProjectionTruthSource {
    if let Some(provenance_summary) = document.provenance_summary.as_ref() {
        if provenance_summary.profile_summary.contains("OxFml") {
            return ProjectionTruthSource::LiveBacked;
        }
    }

    if document.value_presentation.is_some() {
        return ProjectionTruthSource::LiveBacked;
    }

    ProjectionTruthSource::LocalFallback
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DerivedFormulaPresentation {
    evaluation_summary: Option<String>,
    effective_display_summary: Option<String>,
    array_preview: Option<FormulaArrayPreviewState>,
    blocked_reason: Option<String>,
}

fn derive_formula_presentation(
    source_text: &str,
    document: &EditorDocument,
) -> DerivedFormulaPresentation {
    if let Some(value_presentation) = document.value_presentation.as_ref() {
        return derived_presentation_from_value_presentation(value_presentation);
    }

    if let Some(blocked_reason) = document
        .provenance_summary
        .as_ref()
        .and_then(|summary| summary.blocked_reason.clone())
    {
        return DerivedFormulaPresentation {
            evaluation_summary: Some(format!("Blocked · {blocked_reason}")),
            effective_display_summary: Some("Blocked on host lane".to_string()),
            array_preview: None,
            blocked_reason: Some(blocked_reason),
        };
    }

    if let Some(diagnostic) = document.live_diagnostics.diagnostics.first() {
        return DerivedFormulaPresentation {
            evaluation_summary: Some(format!("Diagnostic · {}", diagnostic.message)),
            effective_display_summary: Some("Input incomplete".to_string()),
            array_preview: None,
            blocked_reason: None,
        };
    }

    if let Some(forced_text) = source_text.strip_prefix('\'') {
        return DerivedFormulaPresentation {
            evaluation_summary: Some(format!("Text · {forced_text}")),
            effective_display_summary: Some(forced_text.to_string()),
            array_preview: None,
            blocked_reason: None,
        };
    }

    if !source_text.starts_with('=') {
        if let Ok(number) = source_text.parse::<f64>() {
            return DerivedFormulaPresentation {
                evaluation_summary: Some(format!("Number · {}", format_number(number))),
                effective_display_summary: Some(source_text.to_string()),
                array_preview: None,
                blocked_reason: None,
            };
        }

        if !source_text.is_empty() {
            return DerivedFormulaPresentation {
                evaluation_summary: Some(format!("Text · {source_text}")),
                effective_display_summary: Some(source_text.to_string()),
                array_preview: None,
                blocked_reason: None,
            };
        }
    }

    DerivedFormulaPresentation {
        evaluation_summary: None,
        effective_display_summary: None,
        array_preview: None,
        blocked_reason: None,
    }
}

fn derived_presentation_from_value_presentation(
    value_presentation: &FormulaResultViewModel,
) -> DerivedFormulaPresentation {
    DerivedFormulaPresentation {
        evaluation_summary: Some(value_presentation.evaluation_summary.clone()),
        effective_display_summary: value_presentation.effective_display_summary.clone(),
        array_preview: value_presentation.array_preview.as_ref().map(|preview| {
            FormulaArrayPreviewState {
                label: preview.label.clone(),
                rows: preview.rows.clone(),
                truncated: preview.truncated,
            }
        }),
        blocked_reason: value_presentation.blocked_reason.clone(),
    }
}

fn format_number(value: f64) -> String {
    if (value.fract()).abs() < f64::EPSILON {
        format!("{}", value as i64)
    } else {
        value.to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorSessionError {
    UnknownFormulaSpace(FormulaSpaceId),
    Bridge(OxfmlHostSessionError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::oxfml::{
        CompletionProposal, CompletionProposalKind, EditorAnalysisStage, FormulaEditResult,
        FormulaEditReuseSummary, ProvenanceSummary,
    };

    fn sample_document(source_text: &str) -> EditorDocument {
        EditorDocument {
            source_text: source_text.to_string(),
            text_change_range: None,
            editor_syntax_snapshot: crate::test_support::make_editor_syntax_snapshot(
                "formula-1",
                "green-1",
                vec![],
            ),
            live_diagnostics: crate::test_support::empty_live_diagnostic_snapshot(),
            reuse_summary: FormulaEditReuseSummary {
                reused_green_tree: true,
                reused_red_projection: true,
                reused_bound_formula: false,
            },
            signature_help: Some(crate::test_support::make_signature_help_context(
                "SUM",
                0,
                source_text.chars().count(),
                1,
            )),
            function_help: None,
            completion_proposals: vec![CompletionProposal {
                proposal_id: "proposal-1".to_string(),
                proposal_kind: CompletionProposalKind::Function,
                display_text: "SUM".to_string(),
                insert_text: "SUM(".to_string(),
                replacement_span: None,
                documentation_ref: None,
                profile_payload: None,
                requires_revalidation: true,
            }],
            formula_walk: vec![],
            parse_summary: None,
            bind_summary: None,
            eval_summary: None,
            provenance_summary: None,
            value_presentation: None,
        }
    }

    #[test]
    fn apply_editor_document_updates_formula_space_text_and_help() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut formula_spaces = FormulaSpaceCollectionState::default();
        formula_spaces.insert(FormulaSpaceState::new(formula_space_id.clone(), "=1+1"));

        EditorSessionService::apply_editor_document(
            &mut formula_spaces,
            &formula_space_id,
            sample_document("'123.4"),
        )
        .expect("known formula space should update");

        let updated = formula_spaces.get(&formula_space_id).expect("space exists");
        assert_eq!(updated.raw_entered_cell_text, "'123.4");
        assert_eq!(updated.completion_help.completion_count, 1);
        assert!(updated.completion_help.has_signature_help);
        assert_eq!(updated.editor_surface_state.completion_anchor_offset, None);
        assert_eq!(
            updated.editor_surface_state.completion_selected_index,
            Some(0)
        );
        assert_eq!(
            updated.editor_surface_state.signature_help_anchor_offset,
            None
        );
        assert_eq!(
            updated.latest_evaluation_summary.as_deref(),
            Some("Text · 123.4")
        );
        assert_eq!(updated.effective_display_summary.as_deref(), Some("123.4"));
        assert_eq!(
            updated.context.truth_source,
            ProjectionTruthSource::LocalFallback
        );
        assert_eq!(
            updated
                .editor_document
                .as_ref()
                .expect("editor document retained")
                .green_tree_key(),
            "green-1"
        );
    }

    struct FakeBridge {
        document: EditorDocument,
    }

    impl OxfmlHostSession for FakeBridge {
        fn apply_formula_edit(
            &self,
            request: FormulaEditRequest,
        ) -> Result<FormulaEditResult, OxfmlHostSessionError> {
            assert_eq!(request.formula_stable_id, "formula-1");
            assert_eq!(request.entered_text, "=SUM(1,2,3)");
            assert_eq!(request.cursor_offset, 4);
            assert_eq!(request.analysis_stage, EditorAnalysisStage::SyntaxAndBind);
            assert!(request.previous_green_tree_key.is_none());
            Ok(FormulaEditResult {
                document: self.document.clone(),
            })
        }
    }

    #[test]
    fn handle_formula_edit_intent_routes_through_bridge_and_updates_space() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut formula_spaces = FormulaSpaceCollectionState::default();
        formula_spaces.insert(FormulaSpaceState::new(formula_space_id.clone(), "=1+1"));
        let bridge = FakeBridge {
            document: sample_document("=SUM(1,2,3)"),
        };

        EditorSessionService::handle_formula_edit_intent(
            &bridge,
            &mut formula_spaces,
            ApplyFormulaEditIntent {
                formula_space_id: formula_space_id.clone(),
                formula_stable_id: "formula-1".to_string(),
                entered_text: "=SUM(1,2,3)".to_string(),
                cursor_offset: 4,
                analysis_stage: EditorAnalysisStage::SyntaxAndBind,
                formatting_request: None,
                scenario_policy: crate::adapters::oxfml::ScenarioPolicyRequest::Deterministic,
                skip_runtime_evaluation: false,
                recalc_mode: crate::adapters::oxfml::RecalcModeRequest::Auto,
                language_tag: "en-US".to_string(),
                trace_mode: crate::adapters::oxfml::TraceModeRequest::ValueOnly,
            },
        )
        .expect("edit intent should update via bridge");

        let updated = formula_spaces.get(&formula_space_id).expect("space exists");
        assert_eq!(updated.raw_entered_cell_text, "=SUM(1,2,3)");
        assert_eq!(
            updated.context.truth_source,
            ProjectionTruthSource::LocalFallback
        );
        // §11.3 invariant 4: after a successful bridge round-trip, the
        // retained editor document's source_text equals the formula
        // space's raw_entered_cell_text.
        assert_eq!(
            updated
                .editor_document
                .as_ref()
                .map(|document| document.source_text.as_str()),
            Some("=SUM(1,2,3)")
        );
    }

    /// §11.3 invariant 2 (LocalFallback arm): a document whose
    /// `provenance_summary` carries no OxFml marker and no
    /// `value_presentation`, the document must derive
    /// `ProjectionTruthSource::LocalFallback`.
    #[test]
    fn apply_editor_document_marks_neutral_provenance_as_local_fallback() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut formula_spaces = FormulaSpaceCollectionState::default();
        formula_spaces.insert(FormulaSpaceState::new(formula_space_id.clone(), "=1+1"));
        let mut document = sample_document("=SUM(1,2,3)");
        document.provenance_summary = Some(ProvenanceSummary {
            profile_summary: "OfflineTrace".to_string(),
            blocked_reason: None,
        });

        EditorSessionService::apply_editor_document(
            &mut formula_spaces,
            &formula_space_id,
            document,
        )
        .expect("known formula space should update");

        let updated = formula_spaces.get(&formula_space_id).expect("space exists");
        assert_eq!(
            updated.context.truth_source,
            ProjectionTruthSource::LocalFallback,
        );
    }

    /// §11.3 invariant 3: `derive_formula_presentation` returns
    /// `Unevaluated` (summary fields are `None`) for text that starts with
    /// `=`, has no `value_presentation`, no blocked reason, no diagnostic,
    /// and no hand-evaluator pattern match. This pins the floor of the
    /// facade so any future seam-routing work has a regression check.
    #[test]
    fn derive_formula_presentation_returns_unevaluated_for_unknown_pattern() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut formula_spaces = FormulaSpaceCollectionState::default();
        formula_spaces.insert(FormulaSpaceState::new(formula_space_id.clone(), "=1"));

        EditorSessionService::apply_editor_document(
            &mut formula_spaces,
            &formula_space_id,
            sample_document("=UNKNOWN(1,2)"),
        )
        .expect("known formula space should update");

        let updated = formula_spaces.get(&formula_space_id).expect("space exists");
        assert!(updated.latest_evaluation_summary.is_none());
        assert!(updated.effective_display_summary.is_none());
        assert!(updated.array_preview.is_none());
    }

    #[test]
    fn apply_editor_document_marks_live_oxfml_documents_as_live_backed() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut formula_spaces = FormulaSpaceCollectionState::default();
        formula_spaces.insert(FormulaSpaceState::new(formula_space_id.clone(), "=1+1"));
        let mut document = sample_document("=SUM(1,2,3)");
        document.provenance_summary = Some(ProvenanceSummary {
            profile_summary: "OxFml runtime · Number".to_string(),
            blocked_reason: None,
        });

        EditorSessionService::apply_editor_document(
            &mut formula_spaces,
            &formula_space_id,
            document,
        )
        .expect("known formula space should update");

        let updated = formula_spaces.get(&formula_space_id).expect("space exists");
        assert_eq!(
            updated.context.truth_source,
            ProjectionTruthSource::LiveBacked
        );
    }
}
