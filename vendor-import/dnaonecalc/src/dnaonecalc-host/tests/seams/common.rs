//! Shared helpers for the seam red-driver suite.
//!
//! The seam suite is gated so that `cargo test -p dnaonecalc-host` stays
//! green by default. To see the red drivers, run:
//!
//!     cargo test -p dnaonecalc-host --test seams -- --ignored
//!
//! Every test in this crate must call `seam_pending` (or set its own
//! explicit assertion) so the failure message names the seam id. The
//! message pattern the human reader picks up is:
//!
//!     pending SEAM-XXX: <target behaviour spec>

#![allow(dead_code)]

/// Standard red-driver panic. Every seam test calls this so the failure
/// message format is uniform.
#[track_caller]
pub fn seam_pending(seam_id: &str, target: &str) -> ! {
    panic!("pending {seam_id}: {target}");
}
