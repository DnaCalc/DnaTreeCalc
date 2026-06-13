# HANDOVER_OXCALC_map_lambda_node_array

Status: Open
Target: OxCalc
Ask: Add and satisfy an OxCalc TreeCalc-context regression for `MAP` over a node-published array where the inline `LAMBDA` also captures a TreeCalc host name.
Context: In the DnaTreeCalc browser, selected node `m` has `=MAP(a, LAMBDA(v, v+x))`, with `a = SEQUENCE(5,5)` and `x = 1`. The node is `verified_clean`, but the published value projects as a `1x1` array whose only cell is `Value`. Expected is a `5x5` numeric array from `2` through `26`.
Evidence: DnaTreeCalc bead `dtc-z0i.8.2`; ignored reproducer `programmable_skin_maps_node_array_with_lambda_host_capture`; control test `programmable_skin_maps_inline_sequence_with_local_lambda`; OxFml already has `evaluator_executes_map_with_local_lambda_callable` for `=MAP(SEQUENCE(3),LAMBDA(x,x+1))`; OxCalc already has `treecalc_context_can_call_lambda_value_published_by_another_node`.

## Proposed OxCalc Regression

Add this at the OxCalc TreeCalc public-context layer, near the existing callable/host-name tests in `src/oxcalc-core/src/consumer.rs`:

```rust
#[test]
fn treecalc_context_maps_node_array_with_lambda_capturing_host_name() {
    let mut context = OxCalcTreeContext::default();
    let workspace_id = context
        .create_workspace(OxCalcTreeWorkspaceCreate::new("workspace:map-lambda-node-array"))
        .unwrap();
    let _x_id = context
        .add_node(&workspace_id, OxCalcTreeNodeCreate::new("x", "1"))
        .unwrap();
    let _a_id = context
        .add_node(&workspace_id, OxCalcTreeNodeCreate::new("a", "=SEQUENCE(5,5)"))
        .unwrap();
    let m_id = context
        .add_node(
            &workspace_id,
            OxCalcTreeNodeCreate::new("m", "=MAP(a,LAMBDA(v,v+x))"),
        )
        .unwrap();

    let result = context.recalculate(&workspace_id).unwrap();

    assert_eq!(result.run_state, OxCalcTreeRunState::Published);
    let mapped = result
        .published_calc_values
        .get(&m_id)
        .expect("mapped node publishes a CalcValue");
    let CoreValue::Array(array) = &mapped.core else {
        panic!("expected mapped array, got {mapped:?}");
    };
    assert_eq!((array.shape().rows, array.shape().cols), (5, 5));
    assert_eq!(array.get(0, 0), Some(&CalcValue::number(2.0)));
    assert_eq!(array.get(4, 4), Some(&CalcValue::number(26.0)));
}
```

## Current DnaTreeCalc Observation

The DnaTreeCalc projection code is not collapsing a valid array: it projects `CalcValue::Array` cell-by-cell and error cells through `NodeValueProjection::Error`. The browser's `1x1 array Value` therefore appears to be the actual engine value shape: an array wrapper containing a `WorksheetErrorCode::Value` cell.

The likely gap is not plain OxFml higher-order execution. Inline `MAP(SEQUENCE(...), LAMBDA(...))` works. The failing surface is the TreeCalc host bridge when a higher-order lambda is invoked per element and its body reads host-resolved names/values (`a` as node-published array and `x` as captured host name).
