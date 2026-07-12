//! WS-16 shared Skin IR formula-surface integration proof.

#![cfg(target_arch = "wasm32")]

use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

use super::scaffold::{dispatch_input, flush_microtasks, mount_home_shell};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test(async)]
async fn shared_formula_surface_renders_snapshot_and_dispatches_typed_edit_intent() {
    let shell = mount_home_shell();
    let legacy_editor = shell.textarea().await;
    let shared_editor = shell
        .select(".onecalc-home-shell__shared-formula-surface .dna-formula__editor")
        .expect("shared FormulaSurface editor rendered from SkinSnapshot")
        .dyn_into::<web_sys::HtmlTextAreaElement>()
        .expect("shared editor is a textarea");

    let next_formula = "=SUM(1,2,3)";
    dispatch_input(&shared_editor, next_formula);
    flush_microtasks(4).await;

    assert_eq!(
        legacy_editor.value(),
        next_formula,
        "FormulaSurface EditText SkinIntent must update the host state consumed by HomeShell",
    );
    assert_eq!(
        shell
            .select(".onecalc-home-shell__shared-formula-surface")
            .and_then(|element| element.get_attribute("data-skin-driven"))
            .as_deref(),
        Some("true"),
    );

    shell.tear_down();
}
