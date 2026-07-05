//! Shared UI components used by more than one skin (§D.1's shared inventory).
//!
//! These are plain Leptos components over the skin IR — no `WorkspaceSkin`
//! wiring — so both the notebook (B) and the workbook (K) mount the same
//! commit loop, diagnostics list, and (later) value/provenance chrome.

pub mod cell_entry;
