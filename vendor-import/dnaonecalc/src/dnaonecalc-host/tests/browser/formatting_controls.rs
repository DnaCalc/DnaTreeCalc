//! Slice-5 formatting-controls invariants.
//!
//! Pin the contract that the formatting row under the result
//! section renders the four user-editable formatting fields
//! (number format code, font color, fill color, Date1904 toggle),
//! that input events dispatch the matching reducer setters, and
//! that the row's data attributes stay queryable for the corpus.

#![cfg(target_arch = "wasm32")]

use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

use super::scaffold::{flush_microtasks, mount_home_shell};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test(async)]
async fn formatting_row_renders_four_controls() {
    let shell = mount_home_shell();
    let _textarea = shell.textarea().await;

    let row = shell
        .select(".onecalc-home-shell__formatting-row")
        .expect("formatting row mounted");
    assert_eq!(
        row.get_attribute("role").as_deref(),
        Some("group"),
        "formatting row carries role=group for the four-control composite",
    );

    for field in ["number-format-code", "font-color", "fill-color", "date1904"] {
        let selector =
            format!(".onecalc-home-shell__formatting-row [data-formatting-field=\"{field}\"]",);
        assert!(
            shell.select(&selector).is_some(),
            "expected formatting field {field}",
        );
    }

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn number_format_preset_chips_render_with_format_codes() {
    let shell = mount_home_shell();
    let _textarea = shell.textarea().await;

    let chips = shell.select_all(".onecalc-home-shell__formatting-preset");
    assert!(
        chips.length() >= 5,
        "expected at least 5 preset chips (General/Number/Currency/Percent/Date); got {}",
        chips.length(),
    );

    let mut format_codes = Vec::new();
    for i in 0..chips.length() {
        let Some(node) = chips.item(i) else { continue };
        let Ok(element) = node.dyn_into::<web_sys::Element>() else {
            continue;
        };
        if let Some(code) = element.get_attribute("data-format-code") {
            format_codes.push(code);
        }
    }
    assert!(format_codes.iter().any(|code| code == ""), "General preset");
    assert!(
        format_codes.iter().any(|code| code == "0.00"),
        "Number preset"
    );
    assert!(
        format_codes.iter().any(|code| code == "$#,##0.00"),
        "Currency preset",
    );
    assert!(
        format_codes.iter().any(|code| code == "0.00%"),
        "Percent preset",
    );
    assert!(
        format_codes.iter().any(|code| code == "yyyy-mm-dd"),
        "Date preset",
    );

    shell.tear_down();
}

#[wasm_bindgen_test(async)]
async fn clicking_preset_updates_number_format_input() {
    let shell = mount_home_shell();
    let _textarea = shell.textarea().await;

    // Find the Currency preset chip and click it.
    let chips = shell.select_all(".onecalc-home-shell__formatting-preset");
    let mut currency_chip: Option<web_sys::Element> = None;
    for i in 0..chips.length() {
        let Some(node) = chips.item(i) else { continue };
        let Ok(element) = node.dyn_into::<web_sys::Element>() else {
            continue;
        };
        if element.get_attribute("data-format-code").as_deref() == Some("$#,##0.00") {
            currency_chip = Some(element);
            break;
        }
    }
    let chip = currency_chip.expect("Currency preset chip");
    let html: web_sys::HtmlElement = chip.unchecked_into();
    html.click();
    flush_microtasks(10).await;

    // The number-format-code input should now contain the preset's format code.
    let input = shell
        .select(
            ".onecalc-home-shell__formatting-row [data-formatting-field=\"number-format-code\"]",
        )
        .expect("number-format input")
        .dyn_into::<web_sys::HtmlInputElement>()
        .expect("input element");
    assert_eq!(
        input.value(),
        "$#,##0.00",
        "clicking Currency preset should set the input value",
    );

    shell.tear_down();
}
