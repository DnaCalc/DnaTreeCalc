// DNA Calc desktop shell (dtc-tsc.9).
//
// Thin Tauri v2 wrapper: the window loads the committed `dnacalc-app` WASM
// frontend (frontendDist in tauri.conf.json). Product behavior lives in the
// shared app/host crates; this host owns startup + window chrome only.
//
// Tauri GUI launch cannot be asserted headlessly in CI, so the S0 smoke is a
// `cargo build` of this binary plus a documented manual launch note (bead
// acceptance's honest fallback). The keyboard-verb-dispatch smoke is proven in
// the browser suite against the same shared shell.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("run DNA Calc desktop shell");
}
