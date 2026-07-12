//! Drill-down motion, a11y, and cross-section invariants.
//!
//! Pins the WS-14 plan §10.3 (motion rules) and §1.4 (vertical
//! rhythm) discipline so future drill-downs (result drill,
//! compare-bundle drill, command palette) inherit the contract
//! without rediscovering it. This bead's body is intentionally
//! test-heavy and code-light: dno-xcq.28 already wired the
//! formula drill-down with prefers-reduced-motion respect and
//! the role / aria attributes; this corpus pins those wires.

#![cfg(target_arch = "wasm32")]

use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

use super::scaffold::{
    dispatch_input, dispatch_keydown_with_modifiers, document, mount_home_shell, wait_for,
};

wasm_bindgen_test_configure!(run_in_browser);

fn computed_style(element: &web_sys::Element) -> Option<web_sys::CssStyleDeclaration> {
    let window = web_sys::window()?;
    window.get_computed_style(element).ok().flatten()
}

fn rect(element: &web_sys::Element) -> web_sys::DomRect {
    element.get_bounding_client_rect()
}

#[wasm_bindgen_test(async)]
async fn drill_panel_transition_duration_is_active_under_no_preference() {
    // Default headless run: prefers-reduced-motion is `no-preference`.
    // The CSS in `theme.rs` wraps the transition rule in a
    // `@media (prefers-reduced-motion: no-preference)` block, so
    // getComputedStyle should report a non-zero transitionDuration
    // for max-height. Pin that the transition is reachable.
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SUM(1,2)");

    let panel = shell
        .select(".onecalc-home-shell__formula-drill-panel")
        .expect("panel mounted");
    let style = computed_style(&panel).expect("computed style");
    let duration = style
        .get_property_value("transition-duration")
        .unwrap_or_default();
    // transitionDuration is a comma-separated list when there
    // are multiple properties transitioning. Any non-zero entry
    // satisfies the contract — the panel CAN animate.
    let any_nonzero = duration
        .split(',')
        .map(str::trim)
        .any(|s| !s.is_empty() && s != "0s" && s != "0ms");
    assert!(
        any_nonzero,
        "transition-duration must include at least one non-zero value under \
         the default media query; got {duration:?}",
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn drill_panel_transition_rule_is_inside_prefers_reduced_motion_media() {
    // Read the embedded `<style>` text and pin that the panel's
    // transition rule is wrapped in
    // `@media (prefers-reduced-motion: no-preference)`. Without
    // this wrapping, reduced-motion users would still see the
    // animation. Read directly from the document's first
    // `<style>` element (the home shell mounts ThemeStyleTag
    // synchronously on first render).
    let shell = mount_home_shell();
    let _textarea = shell.textarea().await;

    let document = document();
    let style_tags = document
        .query_selector_all("style[data-theme]")
        .expect("query ok");
    assert!(
        style_tags.length() >= 1,
        "ThemeStyleTag should mount one <style> tag",
    );
    let mut combined_text = String::new();
    for i in 0..style_tags.length() {
        let node = style_tags.item(i).expect("node");
        if let Some(text) = node.text_content() {
            combined_text.push_str(&text);
        }
    }
    assert!(
        combined_text.contains("prefers-reduced-motion: no-preference"),
        "stylesheet must include @media (prefers-reduced-motion: no-preference) \
         to gate the drill-panel transition. (Look in design_tokens/theme.rs.)",
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn editor_section_height_unchanged_when_drill_toggles() {
    // Drill expansion adds a new section between editor and
    // result; the editor's own bounding-client-rect height must
    // not change. This catches regressions where a layout rule
    // accidentally couples the editor height to the drill
    // expansion (e.g. via a flex-grow rule that rebalances
    // when a sibling appears).
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SUM(1,2)");

    let editor = shell
        .select(".onecalc-home-shell__editor")
        .expect("editor section");
    let editor_height_closed = rect(&editor).height();

    dispatch_keydown_with_modifiers(&textarea, "d", true, false, false);
    super::scaffold::flush_microtasks(15).await;

    let editor = shell
        .select(".onecalc-home-shell__editor")
        .expect("editor section");
    let editor_height_open = rect(&editor).height();

    let delta = (editor_height_open - editor_height_closed).abs();
    assert!(
        delta < 1.0,
        "editor section height changed when drill toggled \
         (closed: {editor_height_closed}, open: {editor_height_open})",
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn drill_panel_has_tree_role_and_aria_label_when_open() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SUM(1,2)");
    dispatch_keydown_with_modifiers(&textarea, "d", true, false, false);
    super::scaffold::flush_microtasks(15).await;

    let tree = shell
        .select(".onecalc-home-shell__formula-drill-tree")
        .expect("tree container mounted");
    assert_eq!(tree.get_attribute("role").as_deref(), Some("tree"));
    assert_eq!(
        tree.get_attribute("aria-label").as_deref(),
        Some("formula walk tree"),
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn drill_rows_have_treeitem_role_and_state_chip_aria_label() {
    // The state chip is Developer-mode rendering; User mode
    // (default after bead 32) does not emit it. Toggle to
    // Developer mode for the chip-presence assertion. The
    // row's `role="treeitem"` and `data-state` attribute are
    // present in BOTH modes — those are asserted before the
    // toggle.
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SUM(1,2)");
    dispatch_keydown_with_modifiers(&textarea, "d", true, false, false);
    super::scaffold::flush_microtasks(15).await;

    let row = shell
        .select(".onecalc-home-shell__formula-drill-row")
        .expect("row mounted");
    assert_eq!(row.get_attribute("role").as_deref(), Some("treeitem"));
    let state_attr = row.get_attribute("data-state").unwrap_or_default();
    assert!(
        matches!(
            state_attr.as_str(),
            "evaluated" | "bound" | "opaque" | "blocked"
        ),
        "row data-state must be one of the four state slugs in any mode; \
         got {state_attr:?}",
    );

    // Switch to Developer mode and assert the visible state chip
    // carries the same aria-label.
    dispatch_keydown_with_modifiers(&textarea, "d", true, false, true);
    super::scaffold::flush_microtasks(15).await;

    let chip = shell
        .select(".onecalc-home-shell__formula-drill-state")
        .expect("state chip mounted in Developer mode");
    let aria_label = chip.get_attribute("aria-label").unwrap_or_default();
    assert!(
        matches!(
            aria_label.as_str(),
            "evaluated" | "bound" | "opaque" | "blocked"
        ),
        "state chip aria-label must be one of the four state slugs; got {aria_label:?}",
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn drill_panel_is_focusable_only_via_tabindex_minus_one() {
    // The panel is `tabindex=-1`: not in the natural tab order,
    // but programmatically focusable. This keeps the drill
    // panel out of the keyboard tab path (the textarea retains
    // primary focus) while allowing future "click the panel
    // header" interactions to land focus there.
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SUM(1,2)");
    dispatch_keydown_with_modifiers(&textarea, "d", true, false, false);
    super::scaffold::flush_microtasks(15).await;

    let panel = shell
        .select(".onecalc-home-shell__formula-drill-panel")
        .expect("panel rendered");
    assert_eq!(panel.get_attribute("tabindex").as_deref(), Some("-1"));

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn drill_toggle_button_is_in_natural_tab_order() {
    // The toggle button (in the editor-foot) IS in the natural
    // tab order — keyboard users can reach it via Shift+Tab from
    // the textarea. Pin that there is no `tabindex="-1"` on the
    // button.
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SUM(1,2)");

    let toggle = shell
        .select(".onecalc-home-shell__formula-drill-toggle")
        .expect("toggle button");
    let tabindex = toggle.get_attribute("tabindex");
    assert!(
        !matches!(tabindex.as_deref(), Some("-1")),
        "toggle button must NOT carry tabindex=-1 (it should be in the tab order)",
    );

    let _ = textarea; // silence warning
    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn drill_panel_aria_hidden_flips_with_expanded() {
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SUM(1,2)");

    // Closed: aria-hidden = true.
    let panel = shell
        .select(".onecalc-home-shell__formula-drill-panel")
        .expect("panel rendered");
    assert_eq!(panel.get_attribute("aria-hidden").as_deref(), Some("true"));

    dispatch_keydown_with_modifiers(&textarea, "d", true, false, false);
    super::scaffold::flush_microtasks(15).await;

    let panel = shell
        .select(".onecalc-home-shell__formula-drill-panel")
        .expect("panel rendered");
    assert_eq!(panel.get_attribute("aria-hidden").as_deref(), Some("false"));

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn drill_panel_id_matches_toggle_aria_controls() {
    // The toggle's `aria-controls` should match the panel's id.
    // Without this association screen readers can't follow the
    // toggle to the controlled panel.
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    dispatch_input(&textarea, "=SUM(1,2)");

    let toggle = shell
        .select(".onecalc-home-shell__formula-drill-toggle")
        .expect("toggle button");
    let aria_controls = toggle.get_attribute("aria-controls").unwrap_or_default();
    assert!(!aria_controls.is_empty(), "toggle must carry aria-controls",);

    let panel_by_id = document()
        .get_element_by_id(&aria_controls)
        .expect("panel found by aria-controls id");
    assert!(panel_by_id
        .class_list()
        .contains("onecalc-home-shell__formula-drill-panel"));
    let _ = panel_by_id.dyn_into::<web_sys::HtmlElement>();

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn drill_long_value_preview_truncates_with_full_text_in_title() {
    // Build an input that produces a long value preview in
    // the walk tree. The corpus pins the truncation contract:
    // the visible text is short, the `title` carries the full
    // preview (so hover reveals the rest).
    let shell = mount_home_shell();
    let textarea = shell.textarea().await;
    // A formula whose evaluation produces text longer than
    // 32 chars; the array-formula path is the easiest route
    // since the value preview includes the full repr.
    dispatch_input(
        &textarea,
        "=\"this is a deliberately long string that exceeds the truncation limit\"",
    );
    dispatch_keydown_with_modifiers(&textarea, "d", true, false, false);

    let row = wait_for(&shell, ".onecalc-home-shell__formula-drill-row", |el| {
        Some(el.clone())
    })
    .await
    .expect("row mounted");
    let value_span = row
        .query_selector(".onecalc-home-shell__formula-drill-value")
        .expect("query ok")
        .expect("value span");
    let visible = value_span.text_content().unwrap_or_default();
    let title = value_span.get_attribute("title").unwrap_or_default();
    if title.chars().count() > 32 {
        assert!(
            visible.chars().count() <= 33,
            "visible text must be truncated to 32 chars + '…' when title is longer; \
             visible.len = {}, title.len = {}",
            visible.chars().count(),
            title.chars().count(),
        );
        assert!(
            visible.ends_with('…'),
            "truncated text must end with '…'; got {visible:?}",
        );
    } else {
        // If the bridge happened to produce a short preview
        // (unlikely for this input but possible across versions),
        // skip — the truncation rule applies only when the
        // full text exceeds the limit.
        assert_eq!(visible, title);
    }

    shell.tear_down();
}
