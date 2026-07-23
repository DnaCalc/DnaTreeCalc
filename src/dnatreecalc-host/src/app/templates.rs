//! Shim: the built-in node templates moved into `dnacalc-host-core`
//! (`tree::templates`) in S4.P2. Re-exported at the old `app::templates` path so
//! the estate's `super::templates::…` call sites (session, projection) compile
//! unchanged.

pub(crate) use dnacalc_host_core::tree::templates::{
    built_in_template_initial_content, built_in_template_manifest_projection,
};
