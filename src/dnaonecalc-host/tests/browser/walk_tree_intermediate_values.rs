//! Walk-tree intermediate-value rendering invariants.
//!
//! Pin the contract that the formula drill-down's value column
//! shows the actual computed intermediate value — the SUM call's
//! return, the IF call's return, the literal arg values — rather
//! than upstream debug strings (`eval=EagerValue`,
//! `args: 2 · profile: AllAsValues`).
//!
//! Backed by upstream `PreparedCall.returned_value` and
//! `PreparedArgument.resolved_value`, both `Option<CalcValue>`,
//! consumed by `live_bridge::map_formula_walk`. When upstream
//! exposes `None` (e.g. `ReferencePreserved` args, helper-parameter
//! name slots, lazy-skipped IF branches) the host falls back to
//! `reference_target` then to a debug stringification — these
//! tests assert the happy path where the values are present.

#![cfg(target_arch = "wasm32")]

use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

use super::scaffold::{
    dispatch_input, dispatch_keydown_with_modifiers, mount_home_shell, wait_for,
};

wasm_bindgen_test_configure!(run_in_browser);

async fn open_drill(
    shell: &super::scaffold::MountedShell,
    textarea: &web_sys::HtmlTextAreaElement,
    formula: &str,
) {
    dispatch_input(textarea, formula);
    super::scaffold::flush_microtasks(15).await;
    dispatch_keydown_with_modifiers(textarea, "d", true, false, false);
    let _ = wait_for(shell, ".onecalc-home-shell__formula-drill-panel", |el| {
        if el.get_attribute("data-expanded").as_deref() == Some("true") {
            Some(())
        } else {
            None
        }
    })
    .await;
    super::scaffold::flush_microtasks(15).await;
}

/// Collect all walk-tree rows' (label, value-column-text) pairs in
/// DOM order. Lets each test assert against the corpus rather than
/// relying on positional indices that drift with bridge output.
fn collect_label_value_pairs(shell: &super::scaffold::MountedShell) -> Vec<(String, String)> {
    let rows = shell.select_all(".onecalc-home-shell__formula-drill-row");
    let mut pairs = Vec::new();
    for i in 0..rows.length() {
        let Some(node) = rows.item(i) else { continue };
        let Ok(row) = node.dyn_into::<web_sys::Element>() else {
            continue;
        };
        let label = row
            .query_selector(".onecalc-home-shell__formula-drill-label")
            .ok()
            .flatten()
            .and_then(|el| el.text_content())
            .unwrap_or_default();
        let value = row
            .query_selector(".onecalc-home-shell__formula-drill-value")
            .ok()
            .flatten()
            .and_then(|el| el.text_content())
            .unwrap_or_default();
        pairs.push((label, value));
    }
    pairs
}

#[wasm_bindgen_test(async)]
async fn sum_call_row_shows_returned_value_in_user_mode() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    open_drill(&shell, &textarea, "=SUM(1,2,3)").await;

    let pairs = collect_label_value_pairs(&shell);
    let sum_row = pairs.iter().find(|(label, _)| label == "SUM");
    assert!(
        sum_row.is_some(),
        "expected a walk-tree row labelled 'SUM'; got {pairs:?}",
    );
    let value = &sum_row.unwrap().1;
    assert_eq!(
        value, "6",
        "SUM(1,2,3) call row must show its returned value '6', not a \
         debug summary; got {value:?}",
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn sum_arg_rows_show_resolved_values_not_debug_strings() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    open_drill(&shell, &textarea, "=SUM(1,2,3)").await;

    let pairs = collect_label_value_pairs(&shell);
    let arg_values: Vec<&String> = pairs
        .iter()
        .filter(|(label, _)| label.starts_with("arg["))
        .map(|(_, value)| value)
        .collect();
    assert!(
        arg_values.len() >= 3,
        "expected at least 3 arg rows under SUM; got pairs {pairs:?}",
    );

    // Each arg row must be the resolved literal — never the upstream
    // PreparedEvaluationMode debug string.
    for value in &arg_values {
        assert!(
            !value.contains("EagerValue"),
            "arg row value must be the resolved literal, not a debug \
             rendering of PreparedEvaluationMode; got {value:?}",
        );
    }
    let collected: Vec<&str> = arg_values.iter().map(|s| s.as_str()).collect();
    assert!(
        collected.contains(&"1") && collected.contains(&"2") && collected.contains(&"3"),
        "arg-row values for =SUM(1,2,3) must include 1, 2, 3 verbatim; got {collected:?}",
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn if_call_row_shows_chosen_branch_value() {
    // =IF(TRUE,42,99) — the IF call's returned_value is 42.
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    open_drill(&shell, &textarea, "=IF(TRUE,42,99)").await;

    let pairs = collect_label_value_pairs(&shell);
    let if_row = pairs.iter().find(|(label, _)| label == "IF");
    assert!(
        if_row.is_some(),
        "expected a walk-tree row labelled 'IF'; got {pairs:?}",
    );
    let value = &if_row.unwrap().1;
    assert_eq!(
        value, "42",
        "IF(TRUE,42,99) row must show its returned value '42'; got {value:?}",
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn nested_call_each_layer_shows_its_own_returned_value() {
    // =SUM(IF(1,2,3),4) — handoff doc's canonical example.
    // SUM should show its returned value (6); IF should show its
    // returned value (2); arg rows under SUM should include the
    // value 4 (the literal) — and the value passed to SUM as
    // arg[0] is also 2 (the IF's return).
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    open_drill(&shell, &textarea, "=SUM(IF(1,2,3),4)").await;

    let pairs = collect_label_value_pairs(&shell);

    let sum_row = pairs.iter().find(|(label, _)| label == "SUM");
    assert!(sum_row.is_some(), "expected a 'SUM' row; got {pairs:?}");
    assert_eq!(
        sum_row.unwrap().1,
        "6",
        "SUM row must show '6'; got {pairs:?}",
    );

    let if_row = pairs.iter().find(|(label, _)| label == "IF");
    assert!(if_row.is_some(), "expected an 'IF' row; got {pairs:?}");
    assert_eq!(
        if_row.unwrap().1,
        "2",
        "IF row must show '2' (its truthy branch); got {pairs:?}",
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn arg_row_with_logical_resolved_value_renders_as_true_glyph() {
    // Pins logical formatting: a TRUE-valued arg renders as "TRUE",
    // not "Logical(true)" / "true" / etc.
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    open_drill(&shell, &textarea, "=IF(TRUE,1,2)").await;

    let pairs = collect_label_value_pairs(&shell);
    let arg_with_true = pairs
        .iter()
        .filter(|(label, _)| label.starts_with("arg["))
        .find(|(_, value)| value == "TRUE");
    assert!(
        arg_with_true.is_some(),
        "expected an arg row whose resolved_value formats as 'TRUE'; \
         got {pairs:?}",
    );

    shell.tear_down();
}
