//! Browser suite for the Bench formatting / CF / locale panel (BENCH_SPEC §7,
//! bead dtc-lfz.5).
//!
//! Two live-browser proofs:
//!   1. Mounted in the real [`BenchApp`], authoring a number format through the
//!      panel's preset gallery re-renders the RESULT through host truth (the
//!      displayed string changes with the format code — never a skin-side
//!      formatter).
//!   2. Mounted standalone over a hand-built projection, the panel RENDERS the
//!      typed CF color-scale rule with its thresholds and the per-cell CF
//!      outcomes off the array window, and reflects the locale read-only.
//!
//! Every assertion is a behaviour the pre-fix build did not have (there was no
//! format panel at all), so the suite fails against pre-fix code.

#![cfg(target_arch = "wasm32")]

use dnacalc_bench_app::app::BenchApp;
use dnacalc_bench_app::format_panel::FormatPanel;
use dnacalc_shell::RuntimeContext;
use dnacalc_skin_ir::formula::{
    ArrayCellFormatProjection, ArrayWindowCellProjection, ArrayWindowProjection,
    ColorScaleRuleProjection, ColorScaleStopProjection, ConditionalFormattingRuleProjection,
    ConditionalFormattingThresholdProjection, ConditionalFormattingTypedRuleProjection,
    FormattingSurface, FormulaResultSurface, OneFormulaProjection,
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
        let on_set_number_format = Callback::new(|_code: Option<String>| {});
        view! { <FormatPanel projection=projection on_set_number_format=on_set_number_format /> }
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

    // CF authoring degrades honestly — display-only affordance present.
    assert!(
        query(&host, "[data-testid=\"cf-authoring-degrade\"]").is_some(),
        "CF authoring degrades with an honest note, never a fake editor"
    );
}

/// §7 — the locale section reflects the surface read-only: the language tag and
/// the date1904 epoch, plus the honest read-only note.
#[wasm_bindgen_test]
async fn locale_section_reflects_the_surface_read_only() {
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
    assert!(
        query(&host, "[data-testid=\"locale-degrade\"]").is_some(),
        "locale switching degrades to a read-only note (no write verb yet)"
    );
}
