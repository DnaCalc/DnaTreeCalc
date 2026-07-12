//! SEAM-ONECALC-CAPABILITY-SNAPSHOT
//!
//! Target: populate `CapabilityAndEnvironmentState` from the bootstrap and
//! `ProgrammaticVerificationConfig` so the workspace settings page has
//! real data to render. Today the state is empty.

use super::common::seam_pending;

/// Pending SEAM-ONECALC-CAPABILITY-SNAPSHOT: bootstrap should populate the
/// locale list, date system, host profile, and verification capabilities
/// on `CapabilityAndEnvironmentState`.
///
/// Passes when, after default state construction, the capability snapshot
/// exposes a non-empty locale list, a specific date system (1900/1904),
/// and the host profile label.
///
/// Ownership: future Phase B step 10 (Workspace settings page) bead.
#[test]
#[ignore = "pending SEAM-ONECALC-CAPABILITY-SNAPSHOT"]
fn capability_snapshot_populates_locale_list_from_bootstrap() {
    seam_pending(
        "SEAM-ONECALC-CAPABILITY-SNAPSHOT",
        "bootstrap must populate CapabilityAndEnvironmentState.locales",
    );
}

/// Pending SEAM-ONECALC-CAPABILITY-SNAPSHOT: the date system and host
/// profile must be carried on the capability snapshot, not fabricated per
/// formula space.
#[test]
#[ignore = "pending SEAM-ONECALC-CAPABILITY-SNAPSHOT"]
fn capability_snapshot_carries_date_system_and_host_profile() {
    seam_pending(
        "SEAM-ONECALC-CAPABILITY-SNAPSHOT",
        "CapabilityAndEnvironmentState must carry date_system and host_profile",
    );
}
