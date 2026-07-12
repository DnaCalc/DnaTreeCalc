//! SEAM-ONECALC-TRACE-CONSUMPTION
//!
//! Target: `RetainedArtifactRecord` carries `oxreplay` trace events and the
//! home-shell compare drill-down exposes a trace timeline. Today no trace
//! events are carried.

use super::common::seam_pending;

/// Pending SEAM-ONECALC-TRACE-CONSUMPTION: `RetainedArtifactRecord` must
/// carry `trace_events: Vec<OxReplayTraceEvent>` populated from the
/// verification bundle report.
///
/// Passes when a bundle import that includes trace events produces a
/// retained artifact record with the event list populated.
///
/// Ownership: WS-14 Compare-with-Excel epic.
#[test]
#[ignore = "pending SEAM-ONECALC-TRACE-CONSUMPTION"]
fn retained_artifact_carries_oxreplay_trace_events() {
    seam_pending(
        "SEAM-ONECALC-TRACE-CONSUMPTION",
        "RetainedArtifactRecord must carry trace_events from the verification bundle",
    );
}

/// Pending SEAM-ONECALC-TRACE-CONSUMPTION: the compare drill-down view-model
/// projects the trace events as a timeline with query id linkage.
#[test]
#[ignore = "pending SEAM-ONECALC-TRACE-CONSUMPTION"]
fn compare_drill_exposes_trace_timeline() {
    seam_pending(
        "SEAM-ONECALC-TRACE-CONSUMPTION",
        "Compare drill-down view-model must expose a typed trace_timeline linked by query_id",
    );
}
