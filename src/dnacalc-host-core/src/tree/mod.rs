//! Tree (RichTree) model layer — the Leptos-free tree fixtures / workspace model
//! and built-in templates, migrated out of `dnatreecalc-host` in S4.P2 so the
//! RichTree session (S4.P3) can live behind the Skin IR inside host-core without
//! pulling a UI framework in (the TC no-Leptos gate, `tc_gate_host_core_has_no_leptos`).
//!
//! - [`model`] is pure `serde`/`std` (no engine, no IR, no Leptos) — the workspace
//!   fixture schema + the parsed [`model::WorkspaceModel`].
//! - [`templates`] speaks `dnacalc-skin-ir` projections only (retargeted off the
//!   `dnatreecalc_skin_framework` facade during the move).
//!
//! `dnatreecalc-host` keeps re-export shims at the old paths (`crate::model`,
//! `crate::app::templates`) so the legacy estate compiles unchanged.

pub mod model;
pub mod templates;
