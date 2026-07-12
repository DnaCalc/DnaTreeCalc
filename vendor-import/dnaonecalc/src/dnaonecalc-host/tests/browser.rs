//! WS-14 wasm-bindgen browser corpus harness.
//!
//! Single integration-test binary that mounts the home shell into a real
//! headless browser (Microsoft Edge via wasm-bindgen-test). The
//! `scaffold` module owns the mount / teardown / DOM-query helpers; the
//! per-surface modules (`editor_core`, ...) own the invariants.
//!
//! Runs only on `wasm32-unknown-unknown`; the file is empty on every other
//! target so `cargo test --workspace` on the host stays clean.

#![cfg(target_arch = "wasm32")]

#[path = "browser/scaffold.rs"]
mod scaffold;

#[path = "browser/editor_core.rs"]
mod editor_core;

#[path = "browser/shared_formula_surface.rs"]
mod shared_formula_surface;

#[path = "browser/caret_geometry.rs"]
mod caret_geometry;

#[path = "browser/completion.rs"]
mod completion;

#[path = "browser/signature_help.rs"]
mod signature_help;

#[path = "browser/function_help_hover.rs"]
mod function_help_hover;

#[path = "browser/formula_drill.rs"]
mod formula_drill;

#[path = "browser/drill_motion_a11y.rs"]
mod drill_motion_a11y;

#[path = "browser/repro_enter_eq_aaa.rs"]
mod repro_enter_eq_aaa;

#[path = "browser/repro_eq_space_a_space_a.rs"]
mod repro_eq_space_a_space_a;

#[path = "browser/buffer_integrity.rs"]
mod buffer_integrity;

#[path = "browser/view_mode.rs"]
mod view_mode;

#[path = "browser/foot_chip_modes.rs"]
mod foot_chip_modes;

#[path = "browser/walk_tree_modes.rs"]
mod walk_tree_modes;

#[path = "browser/walk_tree_intermediate_values.rs"]
mod walk_tree_intermediate_values;

#[path = "browser/scenario_breadcrumb.rs"]
mod scenario_breadcrumb;

#[path = "browser/diagnostic_codes_and_spans.rs"]
mod diagnostic_codes_and_spans;

#[path = "browser/formatting_controls.rs"]
mod formatting_controls;

#[path = "browser/caret_buffer_sync.rs"]
mod caret_buffer_sync;
