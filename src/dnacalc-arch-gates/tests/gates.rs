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
//! `cargo tree -p <crate> -e <edges> --prefix none --locked` (`<edges>` is
//! `normal` for most gates, `normal,build` for the F-gate — see
//! `src/lib.rs`'s "Edge scope" section); nothing is mocked. That is the
//! point: a gate that does not actually resolve the real graph cannot catch
//! a real regression.

use dnacalc_arch_gates::{
    TP_CRATES, assert_graph_excludes, assert_graph_includes, find_crate_prefixed, run_cargo_tree,
    run_cargo_tree_with_edges,
};

/// The F-gate's edge scope (dtc-1tk.4 finding H4a): `normal,build`, so a
/// Tauri build script cannot smuggle an `oxcalc*` edge past a `normal`-only
/// scan. See `src/lib.rs`'s "Edge scope" section for the full rationale and
/// why the P-gate/T0-gate/inverted control stay at plain `normal`.
const F_GATE_EDGES: &str = "normal,build";

/// The F-gate's forbidden crate-name prefixes: no `oxcalc*` (the OneCalc
/// forcing function, REDESIGN_PROGRAM.md D5) and — since W011 (dtc-j7n8.1),
/// when `dnacalc-host-core` took `oxdoc-model`/`oxdoc-xlsx` as normal
/// dependencies — no `oxdoc*` either. The Bench tier is the formula tier
/// (`oxfml*`/`oxfunc*` are its point and stay allowed); a document-model or
/// xlsx-package crate reaching a Bench crate would mean the Calc/document
/// tier leaked across the F-line. Before dtc-j7n8.1 the no-oxdoc half of this
/// rule was only written down; the three F-gate tests below now enforce it,
/// and `inverted_control_host_core_contains_oxdoc` proves the `oxdoc` prefix
/// really matches something in this workspace so the exclusion cannot pass
/// vacuously.
const F_GATE_FORBIDDEN_PREFIXES: &[&str] = &["oxcalc", "oxdoc"];

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
/// app's resolved dependency graph contains no `oxcalc*` crate." Checked
/// over `normal,build` edges (dtc-1tk.4 finding H4a) so a `build.rs`
/// dependency can't slip an `oxcalc*` edge past the gate. Since dtc-j7n8.1
/// the same gate also forbids `oxdoc*` (see [`F_GATE_FORBIDDEN_PREFIXES`]);
/// the test name keeps its historical `no_oxcalc` spelling.
#[test]
fn f_gate_bench_host_has_no_oxcalc() {
    let tree = run_cargo_tree_with_edges("dnacalc-bench-host", F_GATE_EDGES);
    assert_graph_excludes("F-gate", &tree, F_GATE_FORBIDDEN_PREFIXES);
}

/// F-gate — the Bench app's desktop (Tauri) host. Same law as the browser
/// host above (no `oxcalc*`, no `oxdoc*`); both compose the Bench app
/// (SHELL_SPEC.md section 1).
#[test]
fn f_gate_bench_desktop_has_no_oxcalc() {
    let tree = run_cargo_tree_with_edges("dnacalc-bench-desktop", F_GATE_EDGES);
    assert_graph_excludes("F-gate", &tree, F_GATE_FORBIDDEN_PREFIXES);
}

/// F-gate — the Bench *app* crate (dtc-tsc.9). It composes the TP shell +
/// bridge over the Bench host, so it inherits oxfml/oxfunc — but never
/// oxcalc, and never oxdoc. This keeps the F-gate honest at the
/// app-composition boundary, not just the host boundary.
#[test]
fn f_gate_bench_app_has_no_oxcalc() {
    let tree = run_cargo_tree_with_edges("dnacalc-bench-app", F_GATE_EDGES);
    assert_graph_excludes("F-gate", &tree, F_GATE_FORBIDDEN_PREFIXES);
}

// NOTE (dtc-1tk.4 finding H4b): there is deliberately no
// `f_gate_bench_app_desktop_has_no_oxcalc` test. `dnacalc-bench-app-desktop`
// is a pure Tauri shell around `dnacalc-bench-app`'s built WASM, loaded as a
// static `frontendDist` asset, not a Cargo path dependency — its resolved
// `normal,build` graph carries **zero** `dnacalc-*` crate edges (verified:
// `cargo tree -p dnacalc-bench-app-desktop -e normal,build --prefix none
// --locked` lists only `tauri`/`tauri-build` and their transitive deps).
// A gate on this crate would therefore always pass regardless of whether
// `dnacalc-bench-app` ever grew an `oxcalc*` edge — vacuous by
// construction. Its coverage is fully subsumed by
// `f_gate_bench_app_has_no_oxcalc` above; do not re-add this test without
// first re-verifying the crate has grown a real `dnacalc-*` edge.

/// F-gate positive control — the Bench *app* really composes the formula
/// tier (oxfml present), so `f_gate_bench_app_has_no_oxcalc` cannot pass
/// vacuously because the app stopped reaching the host at all.
#[test]
fn f_gate_positive_control_bench_app_has_oxfml() {
    let tree = run_cargo_tree_with_edges("dnacalc-bench-app", F_GATE_EDGES);
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
    let tree = run_cargo_tree_with_edges("dnacalc-bench-host", F_GATE_EDGES);
    assert_graph_includes("F-gate positive control", &tree, "oxfml");
}

/// F-gate positive control — `dnacalc-bench-desktop` (dtc-1tk.4 finding
/// H4b). Before this test, `f_gate_bench_desktop_has_no_oxcalc` had no
/// positive control of its own: the crate only reaches `oxfml*`
/// transitively through its single edge to `dnacalc-bench-host`, so if that
/// edge were ever accidentally dropped (e.g. a refactor that stubs the host
/// out), the exclusion gate would keep passing vacuously — every "no
/// oxcalc" gate needs its own "yes oxfml" control, not a borrowed one.
#[test]
fn f_gate_positive_control_bench_desktop_has_oxfml() {
    let tree = run_cargo_tree_with_edges("dnacalc-bench-desktop", F_GATE_EDGES);
    assert_graph_includes("F-gate positive control", &tree, "oxfml");
}

/// Regression guard for the F-gate edge-scope widening (dtc-1tk.4 finding
/// H4a): `run_cargo_tree_with_edges(_, "normal,build")` must actually widen
/// the resolved graph beyond `normal`-only, or the widening is a no-op and
/// a build-dependency-only `oxcalc*` edge would still slip through
/// undetected. `tauri-build` is a real `[build-dependencies]` entry on
/// `dnacalc-bench-desktop` that never appears over `normal`-only edges —
/// if this test ever fails, the F-gate has silently narrowed back to
/// `normal` in effect even if `F_GATE_EDGES` still says `"normal,build"`.
#[test]
fn f_gate_edge_widening_captures_build_dependencies() {
    let normal_only = run_cargo_tree("dnacalc-bench-desktop");
    assert!(
        find_crate_prefixed(&normal_only, "tauri-build").is_none(),
        "test assumption broken: `tauri-build` should NOT appear over normal-only edges for \
         dnacalc-bench-desktop; if it now does, pick a different known build-dependency to pin \
         this regression guard on"
    );

    let widened = run_cargo_tree_with_edges("dnacalc-bench-desktop", F_GATE_EDGES);
    assert!(
        find_crate_prefixed(&widened, "tauri-build").is_some(),
        "F-gate edge widening regressed: `-e {F_GATE_EDGES}` no longer surfaces a known \
         build-dependency (tauri-build) on dnacalc-bench-desktop"
    );
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

/// Inverted control for the W011 xlsx slice (dtc-j7n8.1) — `dnacalc-host-core`
/// *does* contain both `oxdoc-model` and `oxdoc-xlsx`, on purpose: it opens
/// and saves real `.xlsx` bytes through OxDoc (the file-backed workbook
/// lifecycle of epic dtc-j7n8), and host-core may take OxDoc because it
/// already takes OxCalc. This is the `oxdoc` twin of
/// `inverted_control_host_core_contains_oxcalc`, and it is what keeps the
/// widened F-gate honest: [`F_GATE_FORBIDDEN_PREFIXES`] now forbids `oxdoc*`
/// in the Bench crates, and if the oxdoc edges silently vanished from the
/// workspace (or a crate rename made the `oxdoc` prefix match nothing), the
/// `oxdoc` half of those three exclusions would pass vacuously forever.
///
/// The two crates are asserted separately, not as one `oxdoc` prefix:
/// `oxdoc-model` was already reachable transitively through `oxcalc-core`
/// before this bead, so a single-prefix check could keep passing after
/// host-core's own `oxdoc-xlsx` edge (the one that carries the package
/// open/save surface) had been dropped. `assert_graph_includes` takes one
/// prefix, so it is called once per crate; both matched lines are printed so
/// `--nocapture` shows the exact resolved edges.
///
/// Like its oxcalc twin, this test must NEVER be "fixed" by loosening or
/// deleting it. If host-core ever legitimately stops opening xlsx through
/// OxDoc, retarget the control at whichever crate becomes the document
/// anchor -- don't just remove the assertion.
#[test]
fn inverted_control_host_core_contains_oxdoc() {
    let tree = run_cargo_tree("dnacalc-host-core");
    for required in ["oxdoc-model", "oxdoc-xlsx"] {
        assert_graph_includes("Inverted control (oxdoc)", &tree, required);
        let (name, line) = find_crate_prefixed(&tree, required)
            .expect("assert_graph_includes just proved a matching crate line exists");
        println!(
            "inverted_control_host_core_contains_oxdoc: `{required}` matched `{name}` via line: \
             `{line}`"
        );
    }
}

/// TC-gate (S4.P1, epic calc- / dtc-c0wf) — `dnacalc-host-core` (the TC/Calc
/// tier) must stay free of Leptos. host-core's own crate doc claims a
/// "no-Leptos gate, asserted by `cargo tree`" (`src/dnacalc-host-core/src/lib.rs`),
/// but until this test that invariant was only *documented*, never pinned:
/// `t0_gate_skin_ir_has_no_leptos_or_ox` covers skin-ir, and the inverted
/// control above proves host-core *includes* oxcalc, but nothing asserted the
/// absence of `leptos`.
///
/// This gate is filed BEFORE the S4 RichTree extraction moves the (verified
/// Leptos-free) `TreeWorkspaceSession` out of the Leptos-bound
/// `dnatreecalc-host` and into host-core — precisely the change most likely to
/// smuggle a `leptos` edge in through a missed facade import
/// (`dnatreecalc_skin_framework` is `pub use dnacalc_skin_leptos::*`, so a
/// stray `use` of the wrong re-export would pull Leptos in silently). The gate
/// makes that regression fail loudly the moment it happens.
///
/// The `inverted_control_host_core_contains_oxcalc` test guards the shared
/// `run_cargo_tree` mechanism for THIS crate against a silent vacuous pass, so
/// this exclusion cannot pass merely because `cargo tree` returned nothing.
#[test]
fn tc_gate_host_core_has_no_leptos() {
    let tree = run_cargo_tree("dnacalc-host-core");
    assert_graph_excludes("TC-gate", &tree, &["leptos"]);
}
