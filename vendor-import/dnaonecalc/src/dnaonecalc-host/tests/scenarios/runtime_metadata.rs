//! Runtime metadata propagation scenarios.

use dnaonecalc_host::adapters::oxfml::{
    CalcValue, EditorAnalysisStage, FormulaEditRequest, FormulaInputBindingRequest,
    NativeOxfmlHostSession, OxfmlHostSession, RecalcModeRequest, ScenarioPolicyRequest,
    TraceModeRequest,
};
use oxfml_core::consumer::runtime::{RuntimeEnvironment, RuntimeFormulaRequest};
use oxfml_core::format::oxfml_en_us_locale_context;
use oxfml_core::interface::{TypedContextQueryBundle, TypedContextQueryFamily};
use oxfml_core::source::FormulaSourceRecord;
use oxfml_core::FormulaChannelKind;
use oxfunc_core::functions::rand_fn::RandomProvider;

struct FixedRandomProvider {
    value: f64,
}

impl RandomProvider for FixedRandomProvider {
    fn random_unit(&self) -> f64 {
        self.value
    }
}

static FIXED_RANDOM_PROVIDER_05: FixedRandomProvider = FixedRandomProvider { value: 0.5 };

fn live_bridge_request(id: &str, entered_text: &str) -> FormulaEditRequest {
    FormulaEditRequest {
        formula_stable_id: id.to_string(),
        entered_text: entered_text.to_string(),
        cursor_offset: entered_text.len(),
        previous_green_tree_key: None,
        analysis_stage: EditorAnalysisStage::SyntaxAndBind,
        formatting_request: None,
        scenario_policy: ScenarioPolicyRequest::Deterministic,
        skip_runtime_evaluation: false,
        recalc_mode: RecalcModeRequest::Auto,
        trace_mode: TraceModeRequest::PreparedCalls,
        language_tag: "en-US".to_string(),
        formal_input_bindings: Vec::new(),
    }
}

fn evaluate_live_formula(id: &str, entered_text: &str) -> CalcValue {
    let bridge = NativeOxfmlHostSession::default();
    bridge
        .apply_formula_edit(live_bridge_request(id, entered_text))
        .expect("live bridge should evaluate guardrail formula")
        .document
        .value_presentation
        .expect("runtime pass should populate value presentation")
        .published_value
}

fn assert_live_number(id: &str, entered_text: &str, expected: f64) {
    let actual = evaluate_live_formula(id, entered_text);
    assert_eq!(
        actual.as_number(),
        Some(expected),
        "expected numeric {expected} for {entered_text}, got {actual:?}"
    );
}

#[test]
fn sum_value_presentation_carries_oxfunc_kernel_and_admission_versions() {
    let bridge = NativeOxfmlHostSession::default();
    let result = bridge
        .apply_formula_edit(FormulaEditRequest {
            formula_stable_id: "metadata-sum".to_string(),
            entered_text: "=SUM(1,2,3)".to_string(),
            cursor_offset: "=SUM(1,2,3)".len(),
            previous_green_tree_key: None,
            analysis_stage: EditorAnalysisStage::SyntaxAndBind,
            formatting_request: None,
            scenario_policy: ScenarioPolicyRequest::Deterministic,
            skip_runtime_evaluation: false,
            recalc_mode: RecalcModeRequest::Auto,
            trace_mode: TraceModeRequest::PreparedCalls,
            language_tag: "en-US".to_string(),
            formal_input_bindings: Vec::new(),
        })
        .expect("live bridge should evaluate SUM");

    let presentation = result
        .document
        .value_presentation
        .expect("runtime pass should populate value presentation");

    assert!(
        presentation
            .semantic_kernel_metadata_version
            .as_deref()
            .is_some_and(|version| version.contains("SequentialLeftFold")),
        "SUM should carry OxFunc semantic kernel metadata version; got {:?}",
        presentation.semantic_kernel_metadata_version,
    );
    assert!(
        presentation
            .arg_admission_metadata_version
            .as_deref()
            .is_some_and(|version| version.contains("values_only_pre_adapter")),
        "SUM should carry OxFunc arg admission metadata version; got {:?}",
        presentation.arg_admission_metadata_version,
    );
    assert!(presentation.producer_capability_set_keys.is_empty());
    assert!(presentation.exercised_capability_keys.is_empty());
}

#[test]
fn formal_input_binding_affects_single_formula_evaluation() {
    let bridge = NativeOxfmlHostSession::default();
    let result = bridge
        .apply_formula_edit(FormulaEditRequest {
            formula_stable_id: "formal-input-rate".to_string(),
            entered_text: "=Rate*10".to_string(),
            cursor_offset: "=Rate*10".len(),
            previous_green_tree_key: None,
            analysis_stage: EditorAnalysisStage::SyntaxAndBind,
            formatting_request: None,
            scenario_policy: ScenarioPolicyRequest::Deterministic,
            skip_runtime_evaluation: false,
            recalc_mode: RecalcModeRequest::Auto,
            trace_mode: TraceModeRequest::PreparedCalls,
            language_tag: "en-US".to_string(),
            formal_input_bindings: vec![FormulaInputBindingRequest {
                label: "Rate".to_string(),
                reference_descriptor: "name:Rate".to_string(),
                reference_handle: None,
                value: CalcValue::number(0.2),
            }],
        })
        .expect("live bridge should evaluate with formal input");

    let presentation = result
        .document
        .value_presentation
        .expect("runtime pass should populate value presentation");

    assert_eq!(presentation.effective_display_summary.as_deref(), Some("2"));
}

#[test]
fn deterministic_randarray_uses_provider_stream_not_one_scalar_seed() {
    let bridge = NativeOxfmlHostSession::default();
    let result = bridge
        .apply_formula_edit(FormulaEditRequest {
            formula_stable_id: "deterministic-randarray-provider".to_string(),
            entered_text: "=RANDARRAY(2,2)".to_string(),
            cursor_offset: "=RANDARRAY(2,2)".len(),
            previous_green_tree_key: None,
            analysis_stage: EditorAnalysisStage::SyntaxAndBind,
            formatting_request: None,
            scenario_policy: ScenarioPolicyRequest::Deterministic,
            skip_runtime_evaluation: false,
            recalc_mode: RecalcModeRequest::Auto,
            trace_mode: TraceModeRequest::PreparedCalls,
            language_tag: "en-US".to_string(),
            formal_input_bindings: Vec::new(),
        })
        .expect("live bridge should evaluate RANDARRAY");

    let presentation = result
        .document
        .value_presentation
        .expect("runtime pass should populate value presentation");
    let preview = presentation
        .array_preview
        .expect("RANDARRAY should publish an array preview");
    let values: Vec<&str> = preview.rows.iter().flatten().map(String::as_str).collect();

    assert_eq!(preview.rows.len(), 2);
    assert_eq!(preview.rows[0].len(), 2);
    assert_eq!(preview.rows[1].len(), 2);
    assert_eq!(values.len(), 4);
    assert!(
        values.windows(2).any(|pair| pair[0] != pair[1]),
        "provider stream should produce per-cell draws, got {values:?}"
    );
}

#[test]
fn ordinary_single_formula_runtime_uses_no_host_extension_providers() {
    let locale = oxfml_en_us_locale_context();
    let typed_context = TypedContextQueryBundle::new(
        None,
        None,
        Some(&locale),
        Some(46000.0),
        Some(&FIXED_RANDOM_PROVIDER_05),
    );
    let source = FormulaSourceRecord::new(
        "ordinary-no-host-namespace".to_string(),
        1,
        "=SUM(1,2,3)".to_string(),
    )
    .with_formula_channel_kind(FormulaChannelKind::WorksheetA1);

    let result = RuntimeEnvironment::new()
        .execute(RuntimeFormulaRequest::new(source, typed_context))
        .expect("ordinary single-formula runtime should evaluate without host providers");
    let families = &result.typed_query_bundle_spec.families;

    assert_eq!(result.published_worksheet_value, CalcValue::number(6.0));
    assert!(families.contains(&TypedContextQueryFamily::LocaleFormatContext));
    assert!(families.contains(&TypedContextQueryFamily::NowSerial));
    assert!(families.contains(&TypedContextQueryFamily::RandomProvider));
    assert!(
        !families.contains(&TypedContextQueryFamily::HostFunction),
        "ordinary formulas must not install the VBA/XLL host-function provider"
    );
    assert!(
        !families.contains(&TypedContextQueryFamily::RegisteredExternal),
        "ordinary formulas must not install the worksheet registered-external provider"
    );
    assert!(
        !families.contains(&TypedContextQueryFamily::Rtd),
        "ordinary formulas must not install RTD providers"
    );
    assert!(
        !families.contains(&TypedContextQueryFamily::CellInfo)
            && !families.contains(&TypedContextQueryFamily::Info)
            && !families.contains(&TypedContextQueryFamily::FormulaText),
        "ordinary formulas must not install host-query providers"
    );
    assert!(
        result
            .prepared_formula_identity
            .formal_references
            .is_empty(),
        "ordinary built-in execution should not create host formal-reference bindings"
    );
    assert!(
        result.host_formula_context.is_none(),
        "ordinary built-in execution should not create a host formula context"
    );
    assert!(
        result.host_reference_bind_results.is_empty(),
        "ordinary built-in execution should not perform host reference binding"
    );
}

#[test]
fn let_and_lambda_lexical_machinery_stays_internal_to_oxfml() {
    assert_live_number("guard-let-local", "=LET(local,2,local+3)", 5.0);
    assert_live_number(
        "guard-lambda-callable-local",
        "=LET(double,LAMBDA(x,x*2),double(4))",
        8.0,
    );
    assert_live_number(
        "guard-lambda-returned-capture",
        "=LET(makeAdder,LAMBDA(x,LAMBDA(y,x+y)),add2,makeAdder(2),add2(3))",
        5.0,
    );
}

#[test]
fn live_bridge_exposes_udf_registration_invalidation_hooks() {
    let bridge = NativeOxfmlHostSession::default();
    bridge
        .apply_formula_edit(live_bridge_request("guard-invalidate-one", "=SUM(1,2,3)"))
        .expect("first formula should populate bridge caches");
    bridge
        .apply_formula_edit(live_bridge_request("guard-invalidate-two", "=LET(x,2,x+1)"))
        .expect("second formula should populate bridge caches");

    assert_eq!(
        bridge
            .invalidate_formula_binding_state("guard-invalidate-one")
            .expect("formula invalidation should succeed"),
        2,
        "formula invalidation should clear editor and runtime projection caches"
    );
    assert_eq!(
        bridge
            .invalidate_formula_binding_state("guard-invalidate-one")
            .expect("repeat invalidation should succeed"),
        0,
        "repeat formula invalidation should be idempotent"
    );
    assert_eq!(
        bridge
            .invalidate_all_binding_state()
            .expect("workspace invalidation should succeed"),
        2,
        "workspace invalidation should clear remaining formula caches"
    );
}
