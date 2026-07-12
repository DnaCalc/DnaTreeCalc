//! Browser suite for the Bench formatting / CF / locale panel (BENCH_SPEC §7,
//! beads dtc-lfz.5 + dtc-lfz.13).
//!
//! Live-browser proofs:
//!   1. Mounted in the real [`BenchApp`], authoring a number format through the
//!      panel's preset gallery re-renders the RESULT through host truth (the
//!      displayed string changes with the format code — never a skin-side
//!      formatter).
//!   2. Mounted standalone over a hand-built projection, the panel RENDERS the
//!      typed CF color-scale rule with its thresholds, the per-cell CF outcomes
//!      off the array window, and exposes the live authoring controls.
//!   3. Mounted in the real app, authoring a font colour, a locale, and a
//!      cell-value CF rule each round-trips through the OneFormula host and
//!      re-renders the panel from host truth (bead dtc-lfz.13).

#![cfg(target_arch = "wasm32")]

use dnacalc_bench_app::app::BenchApp;
use dnacalc_bench_app::format_panel::FormatPanel;
use dnacalc_shell::RuntimeContext;
use dnacalc_skin_ir::formula::{
    ArrayCellFormatProjection, ArrayWindowCellProjection, ArrayWindowProjection,
    ColorScaleRuleProjection, ColorScaleStopProjection, ConditionalFormatRuleAuthoring,
    ConditionalFormattingRuleProjection, ConditionalFormattingThresholdProjection,
    ConditionalFormattingTypedRuleProjection, FormattingSurface, FormulaResultSurface,
    OneFormulaProjection,
};
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

async fn next_tick() {
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        web_sys::window()
            .expect("window")
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 0)
            .expect("setTimeout");
    });
    wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .expect("tick");
}

/// Let the OxFml pass + re-projection + remount settle.
async fn settle() {
    next_tick().await;
    next_tick().await;
    next_tick().await;
}

fn body_host() -> web_sys::HtmlElement {
    let document = web_sys::window().unwrap().document().unwrap();
    let host = document
        .create_element("div")
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap();
    document.body().unwrap().append_child(&host).unwrap();
    host
}

fn mount_app() -> web_sys::HtmlElement {
    let host = body_host();
    leptos::mount::mount_to(host.clone().unchecked_into(), move || {
        view! { <BenchApp runtime=RuntimeContext::Browser /> }
    })
    .forget();
    host
}

/// Mount just the panel over a fixed projection (the signal lives inside the
/// reactive owner the mount establishes).
fn mount_panel(projection_value: OneFormulaProjection) -> web_sys::HtmlElement {
    let host = body_host();
    leptos::mount::mount_to(host.clone().unchecked_into(), move || {
        let projection = RwSignal::new(projection_value.clone());
        view! {
            <FormatPanel
                projection=projection
                on_set_number_format=Callback::new(|_: Option<String>| {})
                on_set_font_color=Callback::new(|_: Option<String>| {})
                on_set_fill_color=Callback::new(|_: Option<String>| {})
                on_set_cf_rule=Callback::new(|_: (Option<usize>, ConditionalFormatRuleAuthoring)| {})
                on_remove_cf_rule=Callback::new(|_: usize| {})
                on_set_locale=Callback::new(|_: String| {})
                on_set_date1904=Callback::new(|_: bool| {})
            />
        }
    })
    .forget();
    host
}

fn query(host: &web_sys::HtmlElement, selector: &str) -> Option<web_sys::Element> {
    host.query_selector(selector).unwrap()
}

fn count(host: &web_sys::HtmlElement, selector: &str) -> u32 {
    host.query_selector_all(selector).unwrap().length()
}

fn text(host: &web_sys::HtmlElement, selector: &str) -> String {
    query(host, selector)
        .map(|el| el.text_content().unwrap_or_default())
        .unwrap_or_default()
}

fn textarea(host: &web_sys::HtmlElement) -> web_sys::HtmlTextAreaElement {
    query(host, ".dna-bridge__input")
        .expect("bridge textarea")
        .unchecked_into()
}

fn set_textarea(host: &web_sys::HtmlElement, value: &str) {
    let area = textarea(host);
    area.set_value(value);
    area.dispatch_event(&web_sys::Event::new("input").unwrap())
        .unwrap();
}

fn click(host: &web_sys::HtmlElement, selector: &str) {
    let button: web_sys::HtmlElement = query(host, selector)
        .unwrap_or_else(|| panic!("missing clickable {selector}"))
        .unchecked_into();
    button.click();
}

/// Set an `<input>` value and fire the given DOM event (`"input"` for text /
/// colour typing, `"change"` for a committed colour pick).
fn set_input(host: &web_sys::HtmlElement, selector: &str, value: &str, event_kind: &str) {
    let input: web_sys::HtmlInputElement = query(host, selector)
        .unwrap_or_else(|| panic!("missing input {selector}"))
        .unchecked_into();
    input.set_value(value);
    input
        .dispatch_event(&web_sys::Event::new(event_kind).unwrap())
        .unwrap();
}

/// Pick a `<select>` option by value and fire `change`.
fn select_option(host: &web_sys::HtmlElement, selector: &str, value: &str) {
    let select: web_sys::HtmlSelectElement = query(host, selector)
        .unwrap_or_else(|| panic!("missing select {selector}"))
        .unchecked_into();
    select.set_value(value);
    select
        .dispatch_event(&web_sys::Event::new("change").unwrap())
        .unwrap();
}

/// §7 — authoring a number format through the panel re-renders the result via
/// the host projection: the displayed value string changes with the code, and
/// the change is the host's rendered string (thousands separator), never a
/// skin formatter.
#[wasm_bindgen_test]
async fn number_format_preset_rerenders_the_result_via_host() {
    let host = mount_app();
    next_tick().await;

    // The panel mounts in the Inspector's StagePanel slot.
    assert!(
        query(&host, "[data-testid=\"format-panel\"]").is_some(),
        "the format panel mounts in the Inspector"
    );

    set_textarea(&host, "=1234.5");
    settle().await;

    let before = text(&host, "[data-result=\"display\"] .bench-result__value");
    assert!(
        before.contains("1234"),
        "the raw result shows the unformatted value first; got {before:?}"
    );
    assert!(
        !before.contains(','),
        "no thousands separator under General; got {before:?}"
    );

    // Author the thousands format from the preset gallery.
    click(&host, "[data-format-code=\"#,##0.00\"]");
    settle().await;

    let after = text(&host, "[data-result=\"display\"] .bench-result__value");
    assert_ne!(
        before, after,
        "the number-format change re-rendered the result via host projection"
    );
    assert!(
        after.contains(','),
        "the thousands separator comes from host truth; got {after:?}"
    );

    // The panel's own live preview mirrors the same host-rendered string.
    let preview = text(&host, "[data-testid=\"format-preview\"]");
    assert_eq!(
        preview, after,
        "the panel preview is the host's rendered string, not a skin formatter"
    );
}

/// §7 — a CF color-scale rule renders with its thresholds, and the per-cell CF
/// outcomes render off the array window's `ArrayCellFormatProjection`.
#[wasm_bindgen_test]
async fn cf_color_scale_rule_shows_rule_and_per_cell_outcomes() {
    let scaled = |text: &str, fill: &str| ArrayWindowCellProjection {
        display_text: text.to_string(),
        value: None,
        format: Some(ArrayCellFormatProjection {
            effective_font_color: None,
            effective_fill_color: Some(fill.to_string()),
            data_bar: None,
            icon: None,
        }),
    };
    let projection = OneFormulaProjection {
        formatting: FormattingSurface {
            conditional_formatting_rules: vec![ConditionalFormattingRuleProjection {
                operator: None,
                thresholds: Vec::new(),
                font_color: None,
                fill_color: None,
                typed_rule: Some(ConditionalFormattingTypedRuleProjection::ColorScale(
                    ColorScaleRuleProjection {
                        stops: vec![
                            ColorScaleStopProjection {
                                position: ConditionalFormattingThresholdProjection::Min,
                                color: "#FF0000".to_string(),
                            },
                            ColorScaleStopProjection {
                                position: ConditionalFormattingThresholdProjection::Max,
                                color: "#00FF00".to_string(),
                            },
                        ],
                    },
                )),
            }],
            ..FormattingSurface::default()
        },
        result: FormulaResultSurface::Array {
            total_rows: 1,
            total_cols: 2,
            label: "1\u{00D7}2".to_string(),
            window: ArrayWindowProjection {
                total_rows: 1,
                total_cols: 2,
                row_offset: 0,
                col_offset: 0,
                cells: vec![vec![scaled("1", "#FF0000"), scaled("9", "#00FF00")]],
            },
            truncated: false,
        },
        ..OneFormulaProjection::default()
    };

    let host = mount_panel(projection);
    next_tick().await;

    // The typed rule renders with its kind and thresholds.
    let rule = query(&host, "[data-cf-rule=\"0\"]").expect("the CF rule renders");
    assert_eq!(
        rule.get_attribute("data-cf-kind").as_deref(),
        Some("Color scale")
    );
    assert_eq!(
        count(&host, ".dna-format__cf-threshold"),
        2,
        "both color-scale stops render as thresholds"
    );

    // Per-cell CF outcomes render off the array window.
    assert!(
        query(&host, "[data-testid=\"cf-cells\"]").is_some(),
        "the per-cell outcomes block renders for an array result"
    );
    assert_eq!(
        count(&host, "[data-cf-cell]"),
        2,
        "both formatted cells surface an outcome"
    );
    assert!(
        query(&host, "[data-cf-cell][data-cf-fill=\"#FF0000\"]").is_some(),
        "the red-scaled cell carries its resolved fill"
    );
    assert!(
        query(&host, "[data-cf-cell][data-cf-fill=\"#00FF00\"]").is_some(),
        "the green-scaled cell carries its resolved fill"
    );

    // CF authoring is LIVE — the cell-value rule editor is present, and the
    // honest-degrade note is gone.
    assert!(
        query(&host, "[data-testid=\"cf-editor\"]").is_some(),
        "the CF rule editor is present (authoring is live, not a fake-degrade)"
    );
    assert!(
        query(&host, "[data-testid=\"cf-authoring-degrade\"]").is_none(),
        "the CF honest-degrade note is gone now that authoring is live"
    );
}

/// §7 — the locale section reflects the surface and exposes live authoring
/// controls: the language tag, the date1904 epoch, the selector, and the date
/// toggle. The old read-only note is gone.
#[wasm_bindgen_test]
async fn locale_section_reflects_the_surface_and_authors_live() {
    let projection = OneFormulaProjection {
        formatting: FormattingSurface {
            locale_language_tag: "af-ZA".to_string(),
            date1904: true,
            ..FormattingSurface::default()
        },
        ..OneFormulaProjection::default()
    };

    let host = mount_panel(projection);
    next_tick().await;

    let locale = query(&host, "[data-testid=\"format-locale\"]").expect("locale renders");
    assert_eq!(
        locale.get_attribute("data-locale-tag").as_deref(),
        Some("af-ZA"),
        "the locale tag reflects the surface verbatim"
    );
    assert!(
        query(&host, "[data-date1904=\"true\"]").is_some(),
        "the 1904 date-system indicator reflects the surface"
    );
    // Authoring is LIVE — the selector + date toggle are present, the
    // read-only note is gone.
    assert!(
        query(&host, "[data-testid=\"locale-select\"]").is_some(),
        "the locale selector is present (switching is live, not read-only)"
    );
    assert!(
        query(&host, "[data-testid=\"date1904-toggle\"]").is_some(),
        "the date-system toggle is present"
    );
    assert!(
        query(&host, "[data-testid=\"locale-degrade\"]").is_none(),
        "the locale read-only note is gone now that authoring is live"
    );
}

/// §7 (bead dtc-lfz.13) — authoring a font colour, a locale, and a cell-value
/// CF rule each round-trips through the OneFormula host and re-renders the
/// panel from host truth (never a skin-side edit). Fail-pre-fix: these controls
/// did not exist, and the verbs they author did not exist in `OneFormulaIntent`.
#[wasm_bindgen_test]
async fn font_locale_and_cf_authoring_round_trip_through_host() {
    let host = mount_app();
    next_tick().await;
    set_textarea(&host, "=1234.5");
    settle().await;

    // Font colour: author through the picker; the panel swatch (read off the
    // projection's FormattingSurface) reflects the round-tripped value.
    set_input(&host, "[data-testid=\"font-colour\"]", "#d02a23", "change");
    settle().await;
    assert!(
        query(&host, "[data-testid=\"font-fill\"] [data-colour=\"#d02a23\"]").is_some(),
        "the authored font colour round-trips into the panel swatch via host truth"
    );

    // Locale: select German; the read-model tag flips to a de-* tag.
    select_option(&host, "[data-testid=\"locale-select\"]", "de-DE");
    settle().await;
    let tag = query(&host, "[data-testid=\"format-locale\"]")
        .expect("locale renders")
        .get_attribute("data-locale-tag")
        .unwrap_or_default()
        .to_ascii_lowercase();
    assert!(
        tag.starts_with("de"),
        "authoring the locale flips the read-model tag through the host; got {tag:?}"
    );

    // CF: none authored yet.
    assert!(
        query(&host, "[data-cf-rule]").is_none(),
        "no CF rule before authoring"
    );
    // Author a cell-value rule (default operator greaterThan) with a threshold
    // and a fill colour, then add it.
    set_input(&host, "[data-testid=\"cf-threshold-1\"]", "100", "input");
    set_input(&host, "[data-testid=\"cf-fill-colour\"]", "#006600", "change");
    click(&host, "[data-testid=\"cf-add-rule\"]");
    settle().await;
    assert!(
        query(&host, "[data-cf-rule=\"0\"]").is_some(),
        "the authored CF rule round-trips into the panel from host truth"
    );

    // Remove it through the per-rule remove control.
    click(&host, "[data-testid=\"cf-remove-0\"]");
    settle().await;
    assert!(
        query(&host, "[data-cf-rule]").is_none(),
        "removing the rule round-trips through the host"
    );
}
