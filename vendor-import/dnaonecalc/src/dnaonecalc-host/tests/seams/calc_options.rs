//! SEAM-OXFML-CALC-MODE
//! SEAM-OXFML-CALC-ITERATIVE
//! SEAM-OXFML-CALC-OPTIONS
//! SEAM-OXFML-PRECISION-AS-DISPLAYED
//! SEAM-OXFML-EVAL-FREEZE
//! SEAM-OXFML-EVAL-CACHE
//! SEAM-OXFML-EVAL-STRICT
//!
//! Target: `FormulaSpaceState.calc_options` carries every calc setting
//! and the Configure drawer Scenario Settings cluster exposes them.

use super::common::seam_pending;

#[test]
#[ignore = "pending SEAM-OXFML-CALC-OPTIONS"]
fn formula_space_carries_calc_options_payload() {
    seam_pending(
        "SEAM-OXFML-CALC-OPTIONS",
        "FormulaSpaceState must carry a typed calc_options payload",
    );
}

#[test]
#[ignore = "pending SEAM-OXFML-CALC-MODE"]
fn calc_mode_round_trips_through_scenario_settings_cluster() {
    seam_pending(
        "SEAM-OXFML-CALC-MODE",
        "calc mode (Automatic / AutomaticExceptTables / Manual) must project to the cluster",
    );
}

#[test]
#[ignore = "pending SEAM-OXFML-CALC-ITERATIVE"]
fn calc_iterative_settings_round_trip_max_iter_and_tolerance() {
    seam_pending(
        "SEAM-OXFML-CALC-ITERATIVE",
        "iterative calc settings (enabled/max_iter/tolerance) must project to the cluster",
    );
}

#[test]
#[ignore = "pending SEAM-OXFML-PRECISION-AS-DISPLAYED"]
fn precision_as_displayed_flag_round_trips() {
    seam_pending(
        "SEAM-OXFML-PRECISION-AS-DISPLAYED",
        "precision-as-displayed flag must round-trip through calc options",
    );
}

#[test]
#[ignore = "pending SEAM-OXFML-EVAL-FREEZE"]
fn eval_freeze_flag_round_trips_through_scenario_settings() {
    seam_pending(
        "SEAM-OXFML-EVAL-FREEZE",
        "eval freeze flag must round-trip through scenario settings and reach the cluster",
    );
}

#[test]
#[ignore = "pending SEAM-OXFML-EVAL-CACHE"]
fn eval_cache_flag_round_trips_through_scenario_settings() {
    seam_pending(
        "SEAM-OXFML-EVAL-CACHE",
        "eval cache flag must round-trip through scenario settings and reach the cluster",
    );
}

#[test]
#[ignore = "pending SEAM-OXFML-EVAL-STRICT"]
fn eval_strict_flag_round_trips_through_scenario_settings() {
    seam_pending(
        "SEAM-OXFML-EVAL-STRICT",
        "eval strict flag must round-trip through scenario settings and reach the cluster",
    );
}
