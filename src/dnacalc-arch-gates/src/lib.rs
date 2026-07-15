//! `dnacalc-arch-gates` — the D5 crate-tier dependency law, executable.
//!
//! `docs/ux/redesign/REDESIGN_PROGRAM.md` ("Repo consolidation & crate
//! tiers (D5)") and `docs/ux/redesign/SHELL_SPEC.md` section 9 ("Parity &
//! testing") both say the OneCalc OxCalc-free rule, and the sibling
//! Leptos/Ox separations, must be **pinned tests**, not a repo boundary or
//! a reviewer's memory: "a repo boundary only blocks path-dep
//! declarations; a dependency-graph gate asserts the whole resolved graph
//! on every commit." This crate is that assertion, for four laws:
//!
//! 1. **T0-gate** — `dnacalc-skin-ir` (the wire-protocol tier, T0) carries
//!    no Leptos and no engine (`ox*`) crate anywhere in its resolved
//!    graph.
//! 2. **F-gate** (the OneCalc forcing function) — the Bench app's three
//!    host/app crates, `dnacalc-bench-host`, `dnacalc-bench-desktop`, and
//!    `dnacalc-bench-app`, carry no `oxcalc*` crate. `oxfml*`/`oxfunc*` are
//!    the point of the Bench tier and stay allowed; one positive control
//!    per crate asserts `oxfml*` really is present, so the gate cannot pass
//!    because a Bench crate quietly stopped depending on the formula
//!    engine altogether. `dnacalc-bench-app-desktop` (the Tauri shell that
//!    wraps `dnacalc-bench-app`'s built WASM as a static `frontendDist`
//!    asset) is deliberately **not** F-gated on its own: it carries zero
//!    `dnacalc-*` crate edges at all (the frontend is loaded as a file, not
//!    a Cargo dependency), so a gate on it would be vacuous by
//!    construction — see the removed `f_gate_bench_app_desktop_has_no_oxcalc`
//!    test note in `tests/gates.rs`; its coverage lives entirely in the
//!    `dnacalc-bench-app` gate.
//! 3. **P-gate** — every crate named in [`TP_CRATES`] (the TP/Presentation
//!    tier) carries no `ox*` crate anywhere in its graph — skins speak
//!    Skin IR only.
//! 4. **Inverted control** — `dnacalc-host-core` (the TC/Calc tier) *does*
//!    contain `oxcalc*`. This is deliberately the opposite of gates 1-3:
//!    if dependency resolution or the harness itself silently broke (for
//!    example a `cargo tree` invocation that quietly returns empty
//!    output, or a crate rename nobody updated these tests for), gates
//!    1-3 would all pass vacuously and nobody would notice. The inverted
//!    control fails loudly in exactly that scenario, so this mechanism
//!    can never rot into a vacuously-green script the way
//!    `scripts/onecalc/run-host-integration.ps1` and its siblings did
//!    (dtc-tsc.2 marked them STALE after their `--exact` test names
//!    stopped existing upstream and they kept exiting 0 anyway).
//!
//! # Design
//!
//! Every gate shells out to:
//! ```text
//! cargo tree -p <crate> -e <edges> --prefix none --locked
//! ```
//! from the workspace root (see [`workspace_root`]), then matches crate
//! names **case-insensitively** against a forbidden or required prefix,
//! restricted to the crate-name column of each line (see
//! [`crate_name_of`]) so a path component (e.g. `..\OxCalc\...`) or a
//! version string can never produce a false hit or a false miss.
//!
//! # Edge scope (dtc-1tk.4 finding H4a)
//!
//! `<edges>` is **not** uniformly `normal` across every gate; the scope is
//! deliberately asymmetric and is spelled out here so prose never drifts
//! from enforcement again:
//!
//! - **F-gate**: [`run_cargo_tree_with_edges`] with `"normal,build"`. A
//!   Tauri desktop shell's `build.rs` (`dnacalc-bench-desktop`,
//!   `dnacalc-bench-app-desktop`) is a plausible, concrete vector for an
//!   `oxcalc*` edge that a plain `normal`-only scan would never see (a
//!   build-dependency never appears on the `normal` edge kind), so the
//!   F-gate widens to catch it. `tests/gates.rs`'s
//!   `f_gate_edge_widening_captures_build_dependencies` pins that the
//!   widening is actually effective — it would fail if `normal,build` ever
//!   silently stopped covering build-dependencies.
//! - **P-gate, T0-gate, inverted control**: [`run_cargo_tree`], i.e.
//!   `normal` edges only (default features). This is a deliberate,
//!   narrower scope, not an oversight: these gates assert what a *shipped*
//!   TP/T0/TC crate's runtime graph contains, and a dev-only or
//!   build-script-only dependency never ships in the artifact these tiers
//!   produce. Widening every gate to `normal,build` (or `--all-features`)
//!   would also start flagging legitimate build tooling (proc-macro
//!   build-helpers, codegen crates) that carries no runtime risk, for no
//!   real coverage gain on these particular tiers. If a TP/T0 crate ever
//!   grows a `build.rs`, revisit this decision rather than assuming it
//!   still holds.
//!
//! Neither scope covers non-default optional features (`--all-features` is
//! not used); this crate makes no claim about an `ox*` edge that only
//! exists behind a non-default Cargo feature nobody in this workspace
//! currently enables.
//!
//! # Running these tests against a different checkout
//!
//! Set `DNACALC_GATES_WORKSPACE_ROOT` ([`WORKSPACE_ROOT_ENV`]) to an
//! absolute path to a workspace root; when set, it is used verbatim
//! instead of walking up from `CARGO_MANIFEST_DIR`. This exists because an
//! isolated worktree checkout can have a stale `Cargo.lock` (so
//! `--locked` refuses to run) or unresolvable relative path deps to
//! sibling engine repos; pointing the gates at a checkout where
//! `cargo tree` actually resolves keeps the tests runnable without
//! weakening what they assert.
//!
//! # Crate-tier invariant
//!
//! This crate is a **dev/test-only workspace member**, exercised via
//! `cargo test -p dnacalc-arch-gates`. It must never appear as a
//! dependency — declared or transitive — of any other crate in this
//! workspace.

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Env var that, when set to a non-empty value, is used verbatim as the
/// workspace root instead of walking up from `CARGO_MANIFEST_DIR`. See the
/// crate-level docs' "Running these tests against a different checkout".
pub const WORKSPACE_ROOT_ENV: &str = "DNACALC_GATES_WORKSPACE_ROOT";

/// Crates in the TP (Presentation) tier (`SHELL_SPEC.md` section 2 /
/// `REDESIGN_PROGRAM.md` D5): may depend on T0 + Leptos/web-sys, but
/// **no `ox*` crate, ever**. `tests::p_gate_tp_crates_have_no_ox` asserts
/// every name here against its own resolved `cargo tree` graph. Appending
/// a crate to the tier is a one-line change: add its name below, and
/// nothing else in this crate needs to change.
///
/// Seeded 2026-07-12 (dtc-tsc.3) with the TP crates that both exist under
/// `src/` and were verified `ox*`-free at seed time:
/// - `dnacalc-strand` (dtc-tsc.5, the token crate).
/// - `dnacalc-skin-leptos` (pre-existing crate; verified clean at seed
///   time via `cargo tree -p dnacalc-skin-leptos -e normal --prefix none
///   --locked`).
///
/// Since landed and added here (dtc-tsc.9), both verified `ox*`-free the
/// same way:
/// - `dnacalc-shell` (`SHELL_SPEC.md` section 2, the cockpit composition
///   crate).
/// - `dnacalc-bridge` (`SHELL_SPEC.md` section 2, the TP-pure formula
///   bridge).
///
/// Also since landed and added here (S2.2):
/// - `dnacalc-stage-notebook` (S2.1, the Notebook stage surface; verified
///   clean at seed time via `cargo tree -p dnacalc-stage-notebook -e normal
///   --prefix none --locked`).
///
/// Also since landed and added here (S2.15):
/// - `dnacalc-stage-atlas` (the Atlas stage surface: structure map + calc
///   HUD; verified clean at seed time via `cargo tree -p dnacalc-stage-atlas
///   -e normal --prefix none --locked`).
/// - `dnacalc-stage-sheet` (S3, the Sheet stage: Canvas2D grid over a
///   RenderPlan IR; verified clean via `cargo tree -p dnacalc-stage-sheet -e
///   normal --prefix none --locked` — host-core is a native dev-dep only, off
///   the `normal` edge set).
///
/// Appending a future TP crate is still a one-line change: add its name
/// below once it exists under `src/` and passes `cargo tree -p <name> -e
/// normal --prefix none --locked` clean, and nothing else in this crate
/// needs to change.
pub const TP_CRATES: &[&str] = &[
    "dnacalc-strand",
    "dnacalc-skin-leptos",
    "dnacalc-shell",
    "dnacalc-bridge",
    "dnacalc-stage-notebook",
    "dnacalc-stage-atlas",
    "dnacalc-stage-sheet",
];

/// One resolved `cargo tree` run for a single crate: the crate name it was
/// run for, plus every non-empty stdout line, trimmed of trailing `\r` and
/// surrounding whitespace.
#[derive(Debug, Clone)]
pub struct TreeOutput {
    pub crate_name: String,
    pub lines: Vec<String>,
}

/// Resolve the workspace root: [`WORKSPACE_ROOT_ENV`] when set to a
/// non-empty value, otherwise walk up from `CARGO_MANIFEST_DIR` until a
/// directory containing a `Cargo.toml` with a `[workspace]` table is
/// found.
///
/// # Panics
/// Panics if the env var is unset (or empty) and no workspace root is
/// found by the time the filesystem root is reached.
#[must_use]
pub fn workspace_root() -> PathBuf {
    if let Ok(dir) = env::var(WORKSPACE_ROOT_ENV) {
        let trimmed = dir.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut dir = manifest_dir.clone();
    loop {
        if is_workspace_manifest(&dir.join("Cargo.toml")) {
            return dir;
        }
        if !dir.pop() {
            panic!(
                "dnacalc-arch-gates: walked up from {} to the filesystem root without finding a \
                 workspace Cargo.toml (a Cargo.toml containing a `[workspace]` table). Set \
                 {WORKSPACE_ROOT_ENV} explicitly to point at a workspace root.",
                manifest_dir.display()
            );
        }
    }
}

fn is_workspace_manifest(path: &Path) -> bool {
    path.is_file()
        && std::fs::read_to_string(path)
            .map(|contents| contents.contains("[workspace]"))
            .unwrap_or(false)
}

/// Run `cargo tree -p <crate_name> -e normal --prefix none --locked` from
/// [`workspace_root`] and return its stdout as [`TreeOutput`].
///
/// This is the `normal`-only scope used by the P-gate, T0-gate, and the
/// inverted control — see "Edge scope" in the crate docs for why those
/// gates deliberately do not widen to build/dev edges. The F-gate instead
/// calls [`run_cargo_tree_with_edges`] with `"normal,build"`.
///
/// # Panics
/// Same panic conditions as [`run_cargo_tree_with_edges`].
#[must_use]
pub fn run_cargo_tree(crate_name: &str) -> TreeOutput {
    run_cargo_tree_with_edges(crate_name, "normal")
}

/// Run `cargo tree -p <crate_name> -e <edges> --prefix none --locked` from
/// [`workspace_root`] and return its stdout as [`TreeOutput`].
///
/// `edges` is passed verbatim to `cargo tree -e` (e.g. `"normal"` or
/// `"normal,build"`; see `cargo tree --help` for the full `-e` grammar).
/// See "Edge scope" in the crate docs for which gate uses which value and
/// why.
///
/// Uses `std::process::Command` directly with an argument array (no
/// shell), so it behaves identically on Windows and Unix and needs no
/// argument quoting.
///
/// # Panics
/// Panics — naming the crate, the exact command, the workspace root used,
/// and full stdout/stderr — if the resolved workspace root has no
/// `Cargo.toml`, if `cargo` cannot be spawned, or if it exits non-zero. A
/// gate that cannot run `cargo tree` is not passing, it is broken, and
/// must never be allowed to read as green.
#[must_use]
pub fn run_cargo_tree_with_edges(crate_name: &str, edges: &str) -> TreeOutput {
    let root = workspace_root();
    if !root.join("Cargo.toml").is_file() {
        panic!(
            "dnacalc-arch-gates: resolved workspace root {} has no Cargo.toml (checked before \
             running `cargo tree -p {crate_name}`). If you set {WORKSPACE_ROOT_ENV}, check that \
             path points at a workspace root.",
            root.display()
        );
    }

    let output = Command::new("cargo")
        .current_dir(&root)
        .args([
            "tree", "-p", crate_name, "-e", edges, "--prefix", "none", "--locked",
        ])
        .output()
        .unwrap_or_else(|e| {
            panic!(
                "dnacalc-arch-gates: failed to spawn `cargo tree -p {crate_name} -e {edges} \
                 --prefix none --locked` in workspace root {}: {e}",
                root.display()
            )
        });

    if !output.status.success() {
        panic!(
            "dnacalc-arch-gates: `cargo tree -p {crate_name} -e {edges} --prefix none --locked` \
             exited with {:?} in workspace root {}.\n--- stdout ---\n{}\n--- stderr ---\n{}\n\
             If this is a stale-lockfile or unresolved-path-dep error specific to an isolated \
             worktree checkout, rerun with {WORKSPACE_ROOT_ENV}=<a checkout where `cargo tree` \
             resolves cleanly>, e.g. the primary checkout.",
            output.status.code(),
            root.display(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }

    let lines = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| line.trim_end_matches('\r').trim().to_string())
        .filter(|line| !line.is_empty())
        .collect();

    TreeOutput {
        crate_name: crate_name.to_string(),
        lines,
    }
}

/// The crate-name column of one `cargo tree --prefix none` line, e.g.
/// `"oxcalc-core v0.4.0 (..\OxCalc\src\oxcalc-core)"` -> `"oxcalc-core"`,
/// or `"proc-macro2 v1.0.106 (*)"` -> `"proc-macro2"`.
///
/// `--prefix none` means there is no tree-drawing indentation to strip, so
/// the crate name is always the line's first whitespace-separated token —
/// that holds whether the line carries a full `(path)` suffix or a
/// collapsed-subtree `(*)` marker.
#[must_use]
pub fn crate_name_of(line: &str) -> &str {
    line.split_whitespace().next().unwrap_or("")
}

/// The first line in `tree` whose crate-name column starts with `prefix`,
/// case-insensitively, as `(crate_name, full_line)`. `None` if no line
/// matches.
#[must_use]
pub fn find_crate_prefixed<'a>(tree: &'a TreeOutput, prefix: &str) -> Option<(&'a str, &'a str)> {
    let prefix_lower = prefix.to_ascii_lowercase();
    tree.lines.iter().find_map(|line| {
        let name = crate_name_of(line);
        if name.to_ascii_lowercase().starts_with(&prefix_lower) {
            Some((name, line.as_str()))
        } else {
            None
        }
    })
}

/// Every *distinct* line in `tree` whose crate-name column starts with
/// `prefix`, case-insensitively. `cargo tree` reprints a crate on every
/// edge that reaches it, so this dedupes by full line before returning —
/// callers reporting a failure want each offending edge once, not once per
/// place it happens to be reachable from.
#[must_use]
pub fn all_crates_prefixed<'a>(tree: &'a TreeOutput, prefix: &str) -> Vec<&'a str> {
    let prefix_lower = prefix.to_ascii_lowercase();
    let mut matches: Vec<&str> = Vec::new();
    for line in &tree.lines {
        let name = crate_name_of(line);
        if name.to_ascii_lowercase().starts_with(&prefix_lower) {
            let line_str = line.as_str();
            if !matches.contains(&line_str) {
                matches.push(line_str);
            }
        }
    }
    matches
}

/// Assert `tree` contains no crate whose name starts with any of
/// `forbidden_prefixes` (case-insensitive, crate-name column only, see
/// [`crate_name_of`]). Collects every violation across every prefix before
/// panicking once — never masks a second failure by stopping at the
/// first.
///
/// On failure, names the gate, the crate under test, every offending
/// dependency line, and a `cargo tree -p <crate> -i <offender> --locked`
/// command to diagnose why the edge exists.
pub fn assert_graph_excludes(gate: &str, tree: &TreeOutput, forbidden_prefixes: &[&str]) {
    let mut violations = Vec::new();
    for &prefix in forbidden_prefixes {
        for line in all_crates_prefixed(tree, prefix) {
            let name = crate_name_of(line);
            violations.push(format!(
                "  forbidden prefix `{prefix}` matched `{name}` via line: `{line}` (diagnose \
                 with: cargo tree -p {} -i {name} --locked)",
                tree.crate_name
            ));
        }
    }
    assert!(
        violations.is_empty(),
        "{gate} FAILED for `{}`:\n{}",
        tree.crate_name,
        violations.join("\n")
    );
}

/// Assert `tree` contains at least one crate whose name starts with
/// `required_prefix` (case-insensitive). Used for positive / inverted
/// controls, so a gate cannot pass vacuously because the crate it names
/// stopped existing, stopped building, or `cargo tree` silently returned
/// nothing.
pub fn assert_graph_includes(gate: &str, tree: &TreeOutput, required_prefix: &str) {
    assert!(
        find_crate_prefixed(tree, required_prefix).is_some(),
        "{gate} FAILED for `{}`: expected at least one crate starting with `{required_prefix}` \
         but found none. Full `cargo tree -p {}` output used for this gate (edges vary by gate \
         -- see this crate's \"Edge scope\" docs):\n{}",
        tree.crate_name,
        tree.crate_name,
        tree.lines.join("\n")
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(crate_name: &str, lines: &[&str]) -> TreeOutput {
        TreeOutput {
            crate_name: crate_name.to_string(),
            lines: lines.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    #[test]
    fn crate_name_of_strips_version_and_path() {
        assert_eq!(
            crate_name_of(r"oxcalc-core v0.4.0 (..\OxCalc\src\oxcalc-core)"),
            "oxcalc-core"
        );
    }

    #[test]
    fn crate_name_of_handles_collapsed_subtree_marker() {
        assert_eq!(crate_name_of("proc-macro2 v1.0.106 (*)"), "proc-macro2");
    }

    #[test]
    fn crate_name_of_handles_bare_line() {
        assert_eq!(crate_name_of("serde v1.0.228"), "serde");
    }

    #[test]
    fn prefix_match_is_case_insensitive_on_name_column_only() {
        let t = tree(
            "demo",
            &[r"OxCalc-Core v0.4.0 (C:\ox-path\OxCalc\src\oxcalc-core)"],
        );
        assert!(find_crate_prefixed(&t, "oxcalc").is_some());

        // The path component mentions "ox-path" / "OxCalc" too, but a
        // crate named e.g. "safe-crate" living under a path that happens
        // to say "ox" must never match: only the name column is
        // inspected.
        let t2 = tree("demo", &[r"safe-crate v1.0.0 (C:\ox-vendor\safe-crate)"]);
        assert!(find_crate_prefixed(&t2, "ox").is_none());
    }

    #[test]
    fn all_crates_prefixed_dedupes_repeated_lines() {
        let t = tree(
            "demo",
            &["oxfoo v1.0.0 (path)", "oxfoo v1.0.0 (path)", "other v1.0.0"],
        );
        assert_eq!(all_crates_prefixed(&t, "ox"), vec!["oxfoo v1.0.0 (path)"]);
    }

    #[test]
    fn assert_graph_excludes_passes_when_clean() {
        let t = tree("demo", &["serde v1.0.228", "thiserror v2.0.18"]);
        assert_graph_excludes("unit-test-gate", &t, &["leptos", "ox"]);
    }

    #[test]
    #[should_panic(expected = "unit-test-gate FAILED")]
    fn assert_graph_excludes_panics_on_violation() {
        let t = tree("demo", &["oxcalc-core v0.1.0 (path)"]);
        assert_graph_excludes("unit-test-gate", &t, &["oxcalc"]);
    }

    #[test]
    fn assert_graph_includes_passes_when_present() {
        let t = tree("demo", &["oxfml_core v0.1.0 (path)"]);
        assert_graph_includes("unit-test-control", &t, "oxfml");
    }

    #[test]
    #[should_panic(expected = "unit-test-control FAILED")]
    fn assert_graph_includes_panics_when_absent() {
        let t = tree("demo", &["serde v1.0.228"]);
        assert_graph_includes("unit-test-control", &t, "oxfml");
    }

    #[test]
    fn tp_crates_is_non_empty_and_one_line_appendable() {
        // Documents the "appending is a one-line change" shape: a flat
        // `&[&str]` literal. If this ever needs a second field per entry,
        // that is a deliberate design change, not an accident.
        assert!(!TP_CRATES.is_empty());
        for name in TP_CRATES {
            assert!(!name.is_empty());
        }
    }
}
