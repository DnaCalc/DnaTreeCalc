//! The D5 crate-tier dependency law, pinned as tests (dtc-tsc.3).
//!
//! See `docs/ux/redesign/REDESIGN_PROGRAM.md` "Repo consolidation & crate
//! tiers (D5)" and `docs/ux/redesign/SHELL_SPEC.md` section 9 ("Parity &
//! testing") for the law these tests enforce, and this crate's `src/lib.rs`
//! for the mechanism (subprocess runner, prefix matching, workspace-root
//! resolution including the `DNACALC_GATES_WORKSPACE_ROOT` escape hatch for
//! checkouts where `cargo tree --locked` cannot resolve, e.g. an isolated
//! worktree with a stale `Cargo.lock`).
//!
//! Every test here shells out to a real
//! `cargo tree -p <crate> -e normal --prefix none --locked`; nothing is
//! mocked. That is the point: a gate that does not actually resolve the
//! real graph cannot catch a real regression.

use dnacalc_arch_gates::{TP_CRATES, assert_graph_excludes, assert_graph_includes, run_cargo_tree};

/// T0-gate — `dnacalc-skin-ir` (the wire-protocol tier, T0) stays free of
/// Leptos and of every engine (`ox*`) crate anywhere in its resolved
/// graph. REDESIGN_PROGRAM.md D5: "T0 - Protocol | serde only -- no
/// engines, no UI framework | `dnacalc-skin-ir`".
#[test]
fn t0_gate_skin_ir_has_no_leptos_or_ox() {
    let tree = run_cargo_tree("dnacalc-skin-ir");
    assert_graph_excludes("T0-gate", &tree, &["leptos", "ox"]);
}

/// F-gate (the OneCalc forcing function) — the Bench app's browser host
/// carries no `oxcalc*` crate. REDESIGN_PROGRAM.md D5: "F-gate: the Bench
/// app's resolved dependency graph contains no `oxcalc*` crate."
#[test]
fn f_gate_bench_host_has_no_oxcalc() {
    let tree = run_cargo_tree("dnacalc-bench-host");
    assert_graph_excludes("F-gate", &tree, &["oxcalc"]);
}

/// F-gate — the Bench app's desktop (Tauri) host. Same law as the browser
/// host above; both compose the Bench app (SHELL_SPEC.md section 1).
#[test]
fn f_gate_bench_desktop_has_no_oxcalc() {
    let tree = run_cargo_tree("dnacalc-bench-desktop");
    assert_graph_excludes("F-gate", &tree, &["oxcalc"]);
}

/// F-gate — the Bench *app* crate (dtc-tsc.9). It composes the TP shell +
/// bridge over the Bench host, so it inherits oxfml/oxfunc — but never
/// oxcalc. This keeps the F-gate honest at the app-composition boundary,
/// not just the host boundary.
#[test]
fn f_gate_bench_app_has_no_oxcalc() {
    let tree = run_cargo_tree("dnacalc-bench-app");
    assert_graph_excludes("F-gate", &tree, &["oxcalc"]);
}

/// F-gate — the Bench app's Tauri desktop shell (dtc-tsc.9).
#[test]
fn f_gate_bench_app_desktop_has_no_oxcalc() {
    let tree = run_cargo_tree("dnacalc-bench-app-desktop");
    assert_graph_excludes("F-gate", &tree, &["oxcalc"]);
}

/// F-gate positive control — the Bench *app* really composes the formula
/// tier (oxfml present), so `f_gate_bench_app_has_no_oxcalc` cannot pass
/// vacuously because the app stopped reaching the host at all.
#[test]
fn f_gate_positive_control_bench_app_has_oxfml() {
    let tree = run_cargo_tree("dnacalc-bench-app");
    assert_graph_includes("F-gate positive control", &tree, "oxfml");
}

/// Inverted control for the Calc app root (dtc-tsc.9) — `dnacalc-app` is
/// TC-adjacent: it drives a real host-core workbook, so oxcalc IS in its
/// graph by design. This is the opposite assertion from the Bench F-gate
/// and proves the two products sit on opposite sides of the F-line (the
/// Calc app is deliberately NOT F-gated and NOT a TP crate).
#[test]
fn calc_app_root_is_tc_adjacent_contains_oxcalc() {
    let tree = run_cargo_tree("dnacalc-app");
    assert_graph_includes("Calc app TC-adjacency control", &tree, "oxcalc");
}

/// F-gate positive control — `dnacalc-bench-host` must still depend on
/// `oxfml*` (OxFml is allowed and is the whole point of the Bench tier;
/// only OxCalc is forbidden). If this ever goes red, the F-gate above is
/// at risk of passing vacuously because the Bench host quietly stopped
/// depending on the formula engine at all, not because it honestly avoids
/// OxCalc.
#[test]
fn f_gate_positive_control_bench_host_has_oxfml() {
    let tree = run_cargo_tree("dnacalc-bench-host");
    assert_graph_includes("F-gate positive control", &tree, "oxfml");
}

/// P-gate — every crate in [`TP_CRATES`] (the TP/Presentation tier)
/// carries no `ox*` crate anywhere in its graph: "skins speak Skin IR
/// only" (REDESIGN_PROGRAM.md D5, SHELL_SPEC.md section 2). Appending a
/// crate to the tier is a one-line change in `src/lib.rs`'s `TP_CRATES`
/// list; this test picks it up with no further edits.
#[test]
fn p_gate_tp_crates_have_no_ox() {
    assert!(
        !TP_CRATES.is_empty(),
        "P-gate: TP_CRATES is empty -- the gate would pass vacuously"
    );
    for &crate_name in TP_CRATES {
        let tree = run_cargo_tree(crate_name);
        assert_graph_excludes("P-gate", &tree, &["ox"]);
    }
}

/// Permanent inverted control — `dnacalc-host-core` (the TC/Calc tier)
/// *does* contain `oxcalc*`, on purpose. This is deliberately the opposite
/// assertion from the gates above: if the subprocess mechanism itself
/// silently broke (for example `cargo tree` returning empty stdout, or a
/// crate rename nobody updated these tests for), every exclusion gate
/// above would pass vacuously and nobody would notice. This control fails
/// loudly in exactly that scenario -- this repo just learned that lesson
/// the hard way from `scripts/onecalc/run-host-integration.ps1` and its
/// siblings, which kept exiting 0 after their `--exact` test names stopped
/// existing upstream (dtc-tsc.2 marked them STALE).
///
/// This test must NEVER be "fixed" by loosening or deleting it. If
/// `dnacalc-host-core` ever legitimately drops OxCalc, retarget this
/// control at whichever crate becomes the new TC-tier engine anchor --
/// don't just remove the assertion.
#[test]
fn inverted_control_host_core_contains_oxcalc() {
    let tree = run_cargo_tree("dnacalc-host-core");
    assert_graph_includes("Inverted control", &tree, "oxcalc");
}
