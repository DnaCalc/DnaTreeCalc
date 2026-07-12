//! SEAM-ONECALC-WITNESS-HANDOFF-MODEL
//!
//! Target: `RetainedArtifactRecord` carries a witness chain and handoff
//! history; projections surface them on Inspect / Workbench clusters.

use super::common::seam_pending;

/// Pending SEAM-ONECALC-WITNESS-HANDOFF-MODEL: retained artifact records
/// must carry `witness_chain: Vec<WitnessLink>` and `handoff_history` and
/// surface both in the Inspect mode retained artifact context view.
///
/// Passes when an imported artifact with a two-step witness chain
/// surfaces two entries on the Inspect cluster's witness view.
#[test]
#[ignore = "pending SEAM-ONECALC-WITNESS-HANDOFF-MODEL"]
fn retained_artifact_carries_witness_chain_and_handoff_history() {
    seam_pending(
        "SEAM-ONECALC-WITNESS-HANDOFF-MODEL",
        "RetainedArtifactRecord must carry witness_chain and handoff_history and project them",
    );
}
