//! DNA TreeCalc shell — composition layer for the skin framework.
//!
//! Provides the universal chrome (context strip + status foot) and the
//! `WorkspaceShell` Leptos component that mounts whichever skin is currently
//! active in the main slot. The shell occupies the thin border around the
//! mounted skin; everything inside the slot is the skin's domain
//! (`docs/ux/SKINS.md` §10).
//!
//! The shell does not know about any concrete skin or about DnaTreeCalc's
//! OxCalc bridge — those wirings live in the host crate. Switching skins
//! never calls the bridge: the slot tears down the previous skin's view,
//! mounts the new one, and shared signals (workspace, selection,
//! shared-skin-state) survive the switch unchanged.

mod shell;
mod theme;

pub use shell::WorkspaceShell;
pub use theme::SHELL_CSS;
