use dnaonecalc_host::adapters::oxfml::{
    CoreValue, EditorAnalysisStage, FormulaEditRequest, NativeOxfmlHost, RecalcModeRequest,
    ScenarioPolicyRequest, TraceModeRequest,
};
use oxfml_core::interface::TypedContextQueryFamily;

fn native_request(id: &str, entered_text: &str) -> FormulaEditRequest {
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
        language_tag: "en-US".to_string(),
        formal_input_bindings: Vec::new(),
        trace_mode: TraceModeRequest::PreparedCalls,
    }
}

#[test]
fn native_host_returns_oxfunc_calc_value_for_scalar_formula() {
    let host = NativeOxfmlHost;
    let result = host
        .apply_formula_edit(native_request("native-sum", "=SUM(1,2,3)"), None)
        .expect("native host should evaluate SUM");
    let runtime = result.runtime_result.expect("runtime result");
    let value = runtime.published_calc_value();

    assert_eq!(value.as_number(), Some(6.0));
}

#[test]
fn native_host_preserves_native_array_value_shape() {
    let host = NativeOxfmlHost;
    let result = host
        .apply_formula_edit(native_request("native-sequence", "=SEQUENCE(2,2)"), None)
        .expect("native host should evaluate SEQUENCE");
    let runtime = result.runtime_result.expect("runtime result");
    let value = runtime.published_calc_value();

    let CoreValue::Array(array) = value.core() else {
        panic!("expected native OxFunc array value, got {value:?}");
    };
    let shape = array.shape();
    assert_eq!(shape.rows, 2);
    assert_eq!(shape.cols, 2);
}

#[test]
fn native_host_default_context_has_no_reference_or_extension_providers() {
    let host = NativeOxfmlHost;
    let result = host
        .apply_formula_edit(native_request("native-context", "=SUM(1,2,3)"), None)
        .expect("native host should evaluate SUM");
    let runtime = result.runtime_result.expect("runtime result");
    let families = &runtime.typed_query_bundle_spec.families;

    assert!(families.contains(&TypedContextQueryFamily::LocaleFormatContext));
    assert!(families.contains(&TypedContextQueryFamily::NowSerial));
    assert!(families.contains(&TypedContextQueryFamily::RandomProvider));
    assert!(!families.contains(&TypedContextQueryFamily::ReferenceSystemProvider));
    assert!(!families.contains(&TypedContextQueryFamily::HostFunction));
    assert!(!families.contains(&TypedContextQueryFamily::RegisteredExternal));
    assert!(!families.contains(&TypedContextQueryFamily::Rtd));
    assert!(!families.contains(&TypedContextQueryFamily::CellInfo));
    assert!(!families.contains(&TypedContextQueryFamily::Info));
    assert!(!families.contains(&TypedContextQueryFamily::FormulaText));
}
