//! SEAM-OXFML-CF-COLORSCALE
//! SEAM-OXFML-CF-DATABAR
//! SEAM-OXFML-CF-ICONSET
//! SEAM-OXFML-CF-TEXT
//! SEAM-OXFML-CF-DATES
//! SEAM-OXFML-CF-BLANKS
//! SEAM-OXFML-CF-ERRORS
//! SEAM-OXFML-CF-RANK
//! SEAM-OXFML-CF-AVERAGE
//! SEAM-OXFML-CF-UNIQUE
//!
//! Target: CF rules from the upstream payload surface as typed
//! `ConditionalFormatRuleViewModel` entries on the cluster, with every
//! rule family covered.

use super::common::seam_pending;

#[test]
#[ignore = "pending SEAM-OXFML-CF-COLORSCALE"]
fn colorscale_rule_surfaces_as_typed_gradient() {
    seam_pending(
        "SEAM-OXFML-CF-COLORSCALE",
        "ColourScaleRule with typed gradient must project to ConditionalFormatRuleViewModel",
    );
}

#[test]
#[ignore = "pending SEAM-OXFML-CF-DATABAR"]
fn databar_rule_surfaces_with_min_max_colour() {
    seam_pending(
        "SEAM-OXFML-CF-DATABAR",
        "DataBarRule (min/max/colour) must project to ConditionalFormatRuleViewModel",
    );
}

#[test]
#[ignore = "pending SEAM-OXFML-CF-ICONSET"]
fn iconset_rule_surfaces_with_thresholds() {
    seam_pending(
        "SEAM-OXFML-CF-ICONSET",
        "IconSetRule (set/thresholds) must project to ConditionalFormatRuleViewModel",
    );
}

#[test]
#[ignore = "pending SEAM-OXFML-CF-TEXT"]
fn text_rule_surfaces_as_typed_comparison() {
    seam_pending(
        "SEAM-OXFML-CF-TEXT",
        "CF text rule must project to ConditionalFormatRuleViewModel with typed comparison",
    );
}

#[test]
#[ignore = "pending SEAM-OXFML-CF-DATES"]
fn dates_rule_surfaces_with_temporal_window() {
    seam_pending(
        "SEAM-OXFML-CF-DATES",
        "CF dates rule must project with its temporal window kind",
    );
}

#[test]
#[ignore = "pending SEAM-OXFML-CF-BLANKS"]
fn blanks_rule_surfaces_on_cluster() {
    seam_pending(
        "SEAM-OXFML-CF-BLANKS",
        "CF blanks rule must project to the cluster",
    );
}

#[test]
#[ignore = "pending SEAM-OXFML-CF-ERRORS"]
fn errors_rule_surfaces_on_cluster() {
    seam_pending(
        "SEAM-OXFML-CF-ERRORS",
        "CF errors rule must project to the cluster",
    );
}

#[test]
#[ignore = "pending SEAM-OXFML-CF-RANK"]
fn rank_rule_surfaces_with_top_bottom_count() {
    seam_pending(
        "SEAM-OXFML-CF-RANK",
        "CF rank rule must project with top/bottom count",
    );
}

#[test]
#[ignore = "pending SEAM-OXFML-CF-AVERAGE"]
fn average_rule_surfaces_with_direction_and_stddev() {
    seam_pending(
        "SEAM-OXFML-CF-AVERAGE",
        "CF average rule must project with direction and stddev",
    );
}

#[test]
#[ignore = "pending SEAM-OXFML-CF-UNIQUE"]
fn unique_rule_surfaces_with_mode() {
    seam_pending(
        "SEAM-OXFML-CF-UNIQUE",
        "CF unique rule must project with unique/duplicate mode",
    );
}
