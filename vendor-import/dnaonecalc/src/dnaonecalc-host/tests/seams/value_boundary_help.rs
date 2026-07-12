//! SEAM-OXFUNC-VALUE-BOUNDARY-HELP
//!
//! Target: `ValueBoundary::allows()` rejections carry human-readable
//! messages so the Host Bindings tab can show *why* a typed value is
//! invalid at a slot.

use super::common::seam_pending;

#[test]
#[ignore = "pending SEAM-OXFUNC-VALUE-BOUNDARY-HELP"]
fn value_boundary_rejection_carries_human_readable_message() {
    seam_pending(
        "SEAM-OXFUNC-VALUE-BOUNDARY-HELP",
        "ValueBoundary::allows() rejections must carry human-readable messages to Host Bindings",
    );
}
