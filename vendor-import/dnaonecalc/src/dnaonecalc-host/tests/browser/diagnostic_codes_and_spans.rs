//! W067 diagnostic-shape invariants.
//!
//! OxFml's W067 diagnostic packet (`OxFml/docs/upstream/NOTES_FOR_DNAONECALC.md` §10)
//! commits to **stable codes** (`unknown_function`, `unknown_name`,
//! `function_arity_mismatch`, `known_symbol_not_callable`,
//! `function_gated_or_unavailable`), **stage classification**
//! (`Syntax` / `Bind` / `SemanticPlan`), **exact symbol spans** on
//! `LiveDiagnostic.primary_span`, and a **worksheet-error class**
//! (e.g. `#NAME?`) when OxFml already knows the worksheet-visible
//! consequence.
//!
//! These tests pin the contract that:
//!
//! 1. The host renders the upstream span verbatim — no host-side
//!    regex / symbol inference / span reconstruction.
//! 2. The squiggle's data attributes carry the upstream `code`,
//!    `stage`, and `worksheet_error_class` so the corpus and the
//!    eventual UI grouping surface can read them without inference.
//! 3. Functions catalogued (e.g. `ABS`) do not produce a diagnostic
//!    in a mixed formula.
//!
//! The canonical formula from W067 is `=YYYY(1,2)+ABS(-12)+QQQQ`:
//!   * `YYYY` at offset 1..5 → `unknown_function`, stage SemanticPlan
//!   * `ABS`  at offset 11..14 → no diagnostic
//!   * `QQQQ` at offset 20..24 → `unknown_name`, stage Bind
//! Both unknown-symbol diagnostics carry `worksheet_error_class="#NAME?"`.

#![cfg(target_arch = "wasm32")]

use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

use super::scaffold::{dispatch_input, mount_home_shell, wait_for};

wasm_bindgen_test_configure!(run_in_browser);

const W067_FORMULA: &str = "=YYYY(1,2)+ABS(-12)+QQQQ";

/// Find the squiggle span whose `data-code` attribute equals
/// `expected_code`. Returns `None` if not present after the bridge
/// round-trip settles.
async fn wait_for_squiggle_with_code(
    shell: &super::scaffold::MountedShell,
    expected_code: &str,
) -> Option<web_sys::Element> {
    for _ in 0..30 {
        super::scaffold::flush_microtasks(1).await;
        let squiggles = shell.select_all(".onecalc-home-shell__editor-squiggles .squiggle");
        for i in 0..squiggles.length() {
            let Some(node) = squiggles.item(i) else {
                continue;
            };
            let Ok(element) = node.dyn_into::<web_sys::Element>() else {
                continue;
            };
            if element.get_attribute("data-code").as_deref() == Some(expected_code) {
                return Some(element);
            }
        }
    }
    None
}

#[wasm_bindgen_test(async)]
async fn yyyy_unknown_function_carries_exact_span_and_code() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, W067_FORMULA);

    let element = wait_for_squiggle_with_code(&shell, "unknown_function")
        .await
        .expect("expected a squiggle with data-code=unknown_function for YYYY");

    assert_eq!(
        element.get_attribute("data-span-start").as_deref(),
        Some("1"),
        "YYYY squiggle must start at offset 1 verbatim from OxFml",
    );
    assert_eq!(
        element.get_attribute("data-span-len").as_deref(),
        Some("4"),
        "YYYY squiggle must be 4 chars long verbatim from OxFml",
    );
    assert_eq!(
        element.get_attribute("data-stage").as_deref(),
        Some("semantic-plan"),
        "unknown_function is a SemanticPlan-stage diagnostic",
    );
    assert_eq!(
        element
            .get_attribute("data-worksheet-error-class")
            .as_deref(),
        Some("#NAME?"),
        "unknown_function carries the #NAME? worksheet error class",
    );
    // Severity is upstream-owned and not pinned by the W067 note;
    // the host must not invent or override it. Just assert the
    // attribute is present.
    let severity = element.get_attribute("data-severity").unwrap_or_default();
    assert!(
        matches!(severity.as_str(), "error" | "warning" | "info"),
        "data-severity must be a known severity slug; got {severity:?}",
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn qqqq_unknown_name_carries_exact_span_and_code() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, W067_FORMULA);

    let element = wait_for_squiggle_with_code(&shell, "unknown_name")
        .await
        .expect("expected a squiggle with data-code=unknown_name for QQQQ");

    assert_eq!(
        element.get_attribute("data-span-start").as_deref(),
        Some("20"),
        "QQQQ squiggle must start at offset 20 verbatim from OxFml",
    );
    assert_eq!(
        element.get_attribute("data-span-len").as_deref(),
        Some("4"),
        "QQQQ squiggle must be 4 chars long verbatim from OxFml",
    );
    assert_eq!(
        element.get_attribute("data-stage").as_deref(),
        Some("bind"),
        "unknown_name is a Bind-stage diagnostic",
    );
    assert_eq!(
        element
            .get_attribute("data-worksheet-error-class")
            .as_deref(),
        Some("#NAME?"),
        "unknown_name carries the #NAME? worksheet error class",
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn abs_in_w067_formula_does_not_produce_a_diagnostic() {
    // ABS is a catalog-known function; even when surrounded by unknown
    // symbols, it must not surface a squiggle of its own.
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, W067_FORMULA);

    // Wait for at least one squiggle so we know diagnostics have run.
    let _ = wait_for_squiggle_with_code(&shell, "unknown_function").await;

    let squiggles = shell.select_all(".onecalc-home-shell__editor-squiggles .squiggle");
    for i in 0..squiggles.length() {
        let Some(node) = squiggles.item(i) else {
            continue;
        };
        let Ok(element) = node.dyn_into::<web_sys::Element>() else {
            continue;
        };
        let span_start = element
            .get_attribute("data-span-start")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(usize::MAX);
        let span_len = element
            .get_attribute("data-span-len")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);
        let span_end = span_start.saturating_add(span_len);
        // ABS lives at chars 11..14 in =YYYY(1,2)+ABS(-12)+QQQQ.
        // No squiggle's span may overlap that range.
        let abs_start = 11usize;
        let abs_end = 14usize;
        let overlaps = !(span_end <= abs_start || span_start >= abs_end);
        let title = element.get_attribute("title").unwrap_or_default();
        assert!(
            !overlaps,
            "no squiggle should overlap ABS at chars 11..14; got start={span_start} \
             len={span_len} title={title:?}",
        );
        assert!(
            !title.contains("ABS"),
            "no diagnostic message should mention ABS; got {title:?}",
        );
    }

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn w067_formula_squiggles_render_in_dom_order() {
    // Squiggles are sorted ascending by span_start in the projector;
    // the DOM order of `.squiggle` spans must follow that sort. Pin
    // it so the corpus catches any future re-ordering bug.
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, W067_FORMULA);

    // Wait for both unknown-* squiggles so we know diagnostics
    // have run.
    let _ = wait_for_squiggle_with_code(&shell, "unknown_function").await;
    let _ = wait_for_squiggle_with_code(&shell, "unknown_name").await;

    let squiggles = shell.select_all(".onecalc-home-shell__editor-squiggles .squiggle");
    let mut starts: Vec<usize> = Vec::new();
    for i in 0..squiggles.length() {
        let Some(node) = squiggles.item(i) else {
            continue;
        };
        let Ok(element) = node.dyn_into::<web_sys::Element>() else {
            continue;
        };
        if let Some(start) = element
            .get_attribute("data-span-start")
            .and_then(|s| s.parse::<usize>().ok())
        {
            starts.push(start);
        }
    }
    let mut sorted = starts.clone();
    sorted.sort();
    assert_eq!(
        starts, sorted,
        "squiggle DOM order must be ascending by data-span-start",
    );
    // The first squiggle is YYYY at 1, the last is QQQQ at 20 (the
    // dedup pass may drop equal-start duplicates but never reorders).
    assert!(
        starts.contains(&1),
        "expected a squiggle starting at 1 (YYYY)"
    );
    assert!(
        starts.contains(&20),
        "expected a squiggle starting at 20 (QQQQ)"
    );

    shell.tear_down();
}
