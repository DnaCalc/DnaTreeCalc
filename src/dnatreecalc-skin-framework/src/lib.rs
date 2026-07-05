//! DNA TreeCalc skin framework — compatibility facade.
//!
//! This crate was split (bead dtc-ajl.1) into:
//!
//! - [`dnacalc_skin_ir`] — the pure UX wire protocol (no Leptos anywhere in
//!   its dependency tree): intents, receipts, deltas, projections, identity,
//!   selection, session channel, the `SkinState`/`SharedSkinState`
//!   contracts, and the signal-free `RecordingDispatcher`.
//! - [`dnacalc_skin_leptos`] — the Leptos mounting layer: the `WorkspaceSkin`
//!   trait, `SkinContext`, `SkinRegistry`, signal-backed state handles, the
//!   signal-backed `InMemoryDispatcher`, and the UI-side helper modules
//!   (`keybinding`, `style`, `theme`, `accessibility`).
//!
//! It re-exports `dnacalc-skin-leptos` verbatim so the existing consumers
//! that import `dnatreecalc_skin_framework::…` keep compiling unchanged. A
//! later cleanup bead retargets those imports and retires this facade.

pub use dnacalc_skin_leptos::*;
