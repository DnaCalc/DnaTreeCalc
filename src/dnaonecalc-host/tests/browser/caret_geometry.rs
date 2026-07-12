//! Browser invariants for the caret-box measurement adapter (bead
//! dno-xcq.22). These pin the contract that `measure_textarea_box`
//! actually populates `FormulaSpaceState.editor_box_metrics` with
//! non-zero pixel values when the home shell is mounted in a real
//! browser, so the popup view-model integration in later beads has a
//! reliable foundation.
//!
//! State-side assertions on the same code path live in
//! `src/app/reducer.rs#tests::editor_box_metrics_*` (synthetic
//! metrics); these tests assert the adapter end-to-end.

#![cfg(target_arch = "wasm32")]

use wasm_bindgen_test::*;

use super::scaffold::{dispatch_input, mount_home_shell, wait_for};

wasm_bindgen_test_configure!(run_in_browser);

/// First keystroke triggers `measure_textarea_box`; the editor frame
/// surfaces the resulting metrics via `data-char-width` and
/// `data-line-height` attributes. Both must be non-zero.
#[wasm_bindgen_test(async)]
async fn first_input_populates_char_width_and_line_height_on_editor_frame() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=");

    let char_width = wait_for(&shell, ".onecalc-home-shell__editor-frame", |element| {
        element
            .get_attribute("data-char-width")
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n > 0)
    })
    .await;
    assert!(
        char_width.is_some_and(|n| n > 0),
        "data-char-width should be populated and non-zero after first input; got {char_width:?}",
    );

    let line_height = shell
        .select(".onecalc-home-shell__editor-frame")
        .and_then(|el| el.get_attribute("data-line-height"))
        .and_then(|s| s.parse::<usize>().ok());
    assert!(
        line_height.is_some_and(|n| n > 0),
        "data-line-height should be populated and non-zero; got {line_height:?}",
    );

    shell.tear_down();
}

/// `data-measure-tick` increments at least once after the first input,
/// proving the reducer entry point fired. Multi-input scenarios bump
/// it further (or stay constant when the metrics are bit-identical).
#[wasm_bindgen_test(async)]
async fn measure_tick_advances_on_first_input() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;

    let initial = shell
        .select(".onecalc-home-shell__editor-frame")
        .and_then(|el| el.get_attribute("data-measure-tick"))
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    assert_eq!(initial, 0, "tick starts at 0 before any input");

    dispatch_input(&textarea, "=");

    let advanced = wait_for(&shell, ".onecalc-home-shell__editor-frame", |element| {
        element
            .get_attribute("data-measure-tick")
            .and_then(|s| s.parse::<u64>().ok())
            .filter(|n| *n >= 1)
    })
    .await;
    assert!(
        advanced.is_some_and(|n| n >= 1),
        "tick should advance to at least 1 after the first input; got {advanced:?}",
    );

    shell.tear_down();
}

/// Char-width matches what the syntax overlay's character cell width
/// looks like in practice (within a reasonable monospace tolerance).
/// This pins that the measurement isn't off by a factor of the sample
/// length or a missing division — the kind of mistake that broke the
/// previous attempt's popup positioning.
#[wasm_bindgen_test(async)]
async fn char_width_falls_within_typical_monospace_range() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SUM(1,2)");

    let char_width = wait_for(&shell, ".onecalc-home-shell__editor-frame", |element| {
        element
            .get_attribute("data-char-width")
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n > 0)
    })
    .await
    .expect("char width populated");

    // The default theme renders ~0.95rem (~15.2px) monospace. Real
    // measured char width sits in the ~6-12px range across browsers
    // and DPI settings. Assert a generous bound; the test fails noisy
    // if the calculation ever returns a wildly off value (e.g. 1px
    // because the sample-string division was skipped, or 80px because
    // the bounding rect width was used unscaled).
    assert!(
        (3..=30).contains(&char_width),
        "char_width {char_width}px is outside the plausible monospace range",
    );

    shell.tear_down();
}

/// Sanity invariant: even with a long multi-line formula in the
/// textarea, the metrics adapter still returns sensible values (no
/// division by zero, no panic, no NaN propagating into integer
/// state). Multi-line and long-text shouldn't perturb the per-character
/// measurement since the char-mirror span is decoupled from textarea
/// content.
#[wasm_bindgen_test(async)]
async fn metrics_remain_stable_under_long_multiline_input() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;

    dispatch_input(&textarea, "=SUM(1,2)");
    let baseline_width = wait_for(&shell, ".onecalc-home-shell__editor-frame", |element| {
        element
            .get_attribute("data-char-width")
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n > 0)
    })
    .await
    .expect("baseline width");

    dispatch_input(
        &textarea,
        "=LET(\n  alpha, 1,\n  beta, 2,\n  alpha + beta\n)",
    );
    let updated_width = wait_for(&shell, ".onecalc-home-shell__editor-frame", |element| {
        element
            .get_attribute("data-char-width")
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n > 0)
    })
    .await
    .expect("updated width");

    // The same font produces the same character width across edits.
    assert_eq!(
        updated_width, baseline_width,
        "char width must be stable across edits (same font); got {updated_width} after \
         {baseline_width}",
    );

    shell.tear_down();
}
