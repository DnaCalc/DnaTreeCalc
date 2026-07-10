#![cfg(target_arch = "wasm32")]

use std::sync::{Arc, Mutex};

use dnacalc_formula_skin_leptos::{FormulaSurface, FormulaSurfaceProps};
use dnacalc_skin_ir::{
    ArrayWindowCellProjection, ArrayWindowProjection, FormulaResultSurface, OneFormulaIntent,
    OneFormulaProjection, SkinIntent,
};
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

fn mount(intents: Arc<Mutex<Vec<SkinIntent>>>) -> web_sys::HtmlElement {
    let document = web_sys::window().unwrap().document().unwrap();
    let host = document
        .create_element("div")
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap();
    document.body().unwrap().append_child(&host).unwrap();
    let mut projection = OneFormulaProjection::default();
    projection.formula_space_id = "f".into();
    projection.editor.source_text = "=SEQUENCE(2,2)".into();
    projection.result = FormulaResultSurface::Array {
        total_rows: 100,
        total_cols: 2,
        label: "100x2".into(),
        window: ArrayWindowProjection {
            total_rows: 100,
            total_cols: 2,
            row_offset: 0,
            col_offset: 0,
            cells: vec![vec![
                ArrayWindowCellProjection {
                    display_text: "1".into(),
                    ..Default::default()
                },
                ArrayWindowCellProjection {
                    display_text: "2".into(),
                    ..Default::default()
                },
            ]],
        },
        truncated: true,
    };
    leptos::mount::mount_to(host.clone().unchecked_into(), move || {
        FormulaSurface(FormulaSurfaceProps {
            projection,
            on_intent: Callback::new(move |intent| intents.lock().unwrap().push(intent)),
        })
    })
    .forget();
    host
}

#[wasm_bindgen_test]
fn editor_and_array_expansion_emit_skin_intents_in_live_dom() {
    let intents = Arc::new(Mutex::new(Vec::new()));
    let host = mount(intents.clone());
    let editor = host
        .query_selector("textarea")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlTextAreaElement>()
        .unwrap();
    editor.set_value("=SUM(1,2,3)");
    editor
        .dispatch_event(&web_sys::Event::new("input").unwrap())
        .unwrap();
    let button = host
        .query_selector(".dna-formula-array button")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap();
    button.click();
    let observed = intents.lock().unwrap();
    assert!(matches!(
        observed[0],
        SkinIntent::OneFormula(OneFormulaIntent::EditText { .. })
    ));
    assert!(matches!(
        observed[1],
        SkinIntent::OneFormula(OneFormulaIntent::RequestResultArrayWindow { row_offset: 1, .. })
    ));
    assert_eq!(
        host.query_selector_all(".dna-formula-array__row")
            .unwrap()
            .length(),
        1
    );
    assert_eq!(host.query_selector_all("th").unwrap().length(), 0);
}
