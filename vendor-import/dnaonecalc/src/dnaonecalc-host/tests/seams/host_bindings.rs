//! SEAM-ONECALC-HOST-BINDINGS-PLUMBING
//! SEAM-ONECALC-HOST-INFO
//! SEAM-ONECALC-RTD
//! SEAM-ONECALC-TABLE-EDITOR
//!
//! Target: carry `SingleFormulaHost`-shaped fields on `FormulaSpaceState`
//! so the Configure drawer's Host Bindings tab can edit them.

use super::common::seam_pending;

/// Pending SEAM-ONECALC-HOST-BINDINGS-PLUMBING: `FormulaSpaceState` must
/// carry a typed `host_bindings` field (defined names, cell values, table
/// catalog, RTD providers) that today lives only in the OxFml live bridge.
///
/// Passes when a new formula space exposes an empty but typed host
/// bindings map, and updating it through a reducer action surfaces on the
/// Configure drawer cluster.
#[test]
#[ignore = "pending SEAM-ONECALC-HOST-BINDINGS-PLUMBING"]
fn formula_space_carries_defined_names_and_cell_values() {
    seam_pending(
        "SEAM-ONECALC-HOST-BINDINGS-PLUMBING",
        "FormulaSpaceState must carry typed defined_names and cell_values",
    );
}

#[test]
#[ignore = "pending SEAM-ONECALC-HOST-BINDINGS-PLUMBING"]
fn configure_drawer_cluster_exposes_host_bindings() {
    seam_pending(
        "SEAM-ONECALC-HOST-BINDINGS-PLUMBING",
        "ConfigureDrawerClusterViewModel must project host_bindings entries for editing",
    );
}

/// Pending SEAM-ONECALC-HOST-INFO: implement `HostInfoProvider` against
/// the OS/browser environment so INFO and CELL functions work.
///
/// Passes when the host bindings cluster reports a non-empty host info
/// payload (os / version / user / locale).
#[test]
#[ignore = "pending SEAM-ONECALC-HOST-INFO"]
fn host_info_provider_surfaces_cell_info_payload() {
    seam_pending(
        "SEAM-ONECALC-HOST-INFO",
        "HostInfoProvider must supply a CellInfo payload for INFO/CELL functions",
    );
}

/// Pending SEAM-ONECALC-RTD: design and implement the RTD provider editor
/// in Configure drawer Tab 9 and the corresponding `RtdProvider` impl.
#[test]
#[ignore = "pending SEAM-ONECALC-RTD"]
fn rtd_provider_entries_surface_on_host_bindings_cluster() {
    seam_pending(
        "SEAM-ONECALC-RTD",
        "RtdProvider entries must surface on the Configure drawer host bindings cluster",
    );
}

/// Pending SEAM-ONECALC-TABLE-EDITOR: design the in-drawer editor for
/// `table_catalog` and `enclosing_table_ref`.
#[test]
#[ignore = "pending SEAM-ONECALC-TABLE-EDITOR"]
fn table_catalog_edits_round_trip_through_formula_space_state() {
    seam_pending(
        "SEAM-ONECALC-TABLE-EDITOR",
        "table_catalog edits must round-trip through FormulaSpaceState.host_bindings",
    );
}
