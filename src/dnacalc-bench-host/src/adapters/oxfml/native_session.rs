use oxfml_core::consumer::editor::{
    EditorDocument as UpstreamEditorDocument, EditorEditService, EditorEnvironment,
    EditorInteractionResult,
};
use oxfml_core::consumer::runtime::{
    RuntimeEnvironment, RuntimeFormalInputBinding, RuntimeFormulaRequest, RuntimeFormulaResult,
};
use oxfml_core::interface::TypedContextQueryBundle;
use oxfml_core::source::FormulaSourceRecord;
use oxfml_core::{BindContext, DefinedNameBinding, EvaluationTraceMode, FormulaChannelKind};

use super::bridge::{
    FormulaEditRequest, FormulaInputBindingRequest, RecalcModeRequest, TraceModeRequest,
};
use super::live_bridge::{
    build_publication_context, build_runtime_locale_context, scenario_runtime_context,
};

#[derive(Debug, Clone, PartialEq)]
pub struct NativeFormulaEditResult {
    pub source: FormulaSourceRecord,
    pub editor_interaction: EditorInteractionResult,
    pub runtime_result: Option<RuntimeFormulaResult>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeOxfmlHostError {
    UpstreamFailure(String),
}

#[derive(Debug, Default)]
pub struct NativeOxfmlHost;

impl NativeOxfmlHost {
    pub fn apply_formula_edit(
        &self,
        request: FormulaEditRequest,
        previous_document: Option<&UpstreamEditorDocument>,
    ) -> Result<NativeFormulaEditResult, NativeOxfmlHostError> {
        let source = FormulaSourceRecord::new(
            request.formula_stable_id.clone(),
            1,
            request.entered_text.clone(),
        )
        .with_formula_channel_kind(FormulaChannelKind::WorksheetA1);

        let editor_environment = EditorEnvironment::new(BindContext::default());
        let editor_service = EditorEditService::new(editor_environment);
        let edit_result = editor_service.apply_edit(
            source.clone(),
            previous_document,
            request.analysis_stage,
            None,
        );
        let editor_interaction =
            editor_service.interact_at_cursor(&edit_result.document, request.cursor_offset);

        let should_run_runtime = !request.skip_runtime_evaluation
            && !matches!(request.recalc_mode, RecalcModeRequest::Manual);
        let runtime_result = if should_run_runtime {
            run_native_runtime(source.clone(), &request)
        } else {
            None
        };

        Ok(NativeFormulaEditResult {
            source,
            editor_interaction,
            runtime_result,
        })
    }
}

fn run_native_runtime(
    source: FormulaSourceRecord,
    request: &FormulaEditRequest,
) -> Option<RuntimeFormulaResult> {
    let locale_ctx = build_runtime_locale_context(
        &request.language_tag,
        request
            .formatting_request
            .as_ref()
            .map(|formatting| formatting.date1904)
            .unwrap_or(false),
    );
    let (now_serial, random_provider) = scenario_runtime_context(request.scenario_policy);
    let typed_context = TypedContextQueryBundle::new(
        None,
        None,
        Some(&locale_ctx),
        Some(now_serial),
        Some(random_provider.as_ref()),
    );

    let mut runtime_request = RuntimeFormulaRequest::new(source, typed_context)
        .with_trace_mode(trace_mode(request.trace_mode));
    if let Some(formatting) = request.formatting_request.as_ref() {
        if let Some(context) = build_publication_context(formatting) {
            runtime_request = runtime_request.with_verification_publication_context(context);
        }
    }

    RuntimeEnvironment::new()
        .with_formal_input_bindings(runtime_formal_input_bindings(
            &request.formal_input_bindings,
        ))
        .execute(runtime_request)
        .ok()
}

fn runtime_formal_input_bindings(
    bindings: &[FormulaInputBindingRequest],
) -> Vec<RuntimeFormalInputBinding> {
    bindings
        .iter()
        .map(|binding| RuntimeFormalInputBinding {
            reference_handle: binding.reference_handle.clone(),
            reference_descriptor: binding.reference_descriptor.clone(),
            binding: DefinedNameBinding::Value(binding.value.clone()),
        })
        .collect()
}

fn trace_mode(trace_mode: TraceModeRequest) -> EvaluationTraceMode {
    match trace_mode {
        TraceModeRequest::ValueOnly => EvaluationTraceMode::ValueOnly,
        TraceModeRequest::PreparedCalls => EvaluationTraceMode::PreparedCalls,
    }
}
