//! Seam red-driver TDD suite.
//!
//! Every test in this crate pins the target behaviour of a seam in
//! `docs/worksets/WS-13_dna_onecalc_ux_revamp.md` Appendix B. Tests are
//! annotated `#[ignore = "pending SEAM-XXX"]` so the default `cargo test`
//! run stays green. To see the seam suite go red while you work on a
//! seam, run:
//!
//!     cargo test -p dnaonecalc-host --test seams -- --ignored
//!
//! When a seam lands, remove the `#[ignore]` from the relevant test as
//! part of the implementing PR and the test becomes a normal PIN.
//!
//! Tests that reference types the view-model does not yet carry live
//! behind `#[cfg(feature = "seam-tests")]` so they stay out of the
//! default build. Enable with `--features seam-tests`.
//!
//! See `~/.claude/plans/fuzzy-spinning-flame.md` → "Seam test catalogue"
//! and "Seam-id crosswalk" for the mapping from seam id to test file.

#[path = "seams/calc_options.rs"]
mod calc_options;
#[path = "seams/capability_snapshot.rs"]
mod capability_snapshot;
#[path = "seams/cell_style.rs"]
mod cell_style;
#[path = "seams/common.rs"]
mod common;
#[path = "seams/comparison_taxonomy.rs"]
mod comparison_taxonomy;
#[path = "seams/conditional_formatting.rs"]
mod conditional_formatting;
#[path = "seams/extended_value_routing.rs"]
mod extended_value_routing;
#[path = "seams/external_links.rs"]
mod external_links;
#[path = "seams/format_payload.rs"]
mod format_payload;
#[path = "seams/green_tree_node.rs"]
mod green_tree_node;
#[path = "seams/host_bindings.rs"]
mod host_bindings;
#[path = "seams/locale_expand.rs"]
mod locale_expand;
#[path = "seams/native_oxfml_host.rs"]
mod native_oxfml_host;
#[path = "seams/parity_matrix.rs"]
mod parity_matrix;
#[path = "seams/persistence.rs"]
mod persistence;
#[path = "seams/presentation_propagation.rs"]
mod presentation_propagation;
#[path = "seams/rail_rename.rs"]
mod rail_rename;
#[path = "seams/reference_style.rs"]
mod reference_style;
#[path = "seams/replay_capture.rs"]
mod replay_capture;
#[path = "seams/rich_value.rs"]
mod rich_value;
#[path = "seams/seam_board.rs"]
mod seam_board;
#[path = "seams/trace_consumption.rs"]
mod trace_consumption;
#[path = "seams/value_boundary_help.rs"]
mod value_boundary_help;
#[path = "seams/verification_context.rs"]
mod verification_context;
#[path = "seams/witness_handoff.rs"]
mod witness_handoff;
