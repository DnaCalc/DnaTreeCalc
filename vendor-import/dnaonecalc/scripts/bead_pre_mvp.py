"""Roll out the WS-14 Pre-MVP bead set.

Creates the WS-14 umbrella epic + 10 Pre-MVP child beads (P0..P9) with
explicit dependencies, matching the WS-13 flat-children pattern.

Run once from any working directory; uses `br` from PATH.
"""

import subprocess
import sys
from typing import List


def br(*args: str, description: str | None = None) -> str:
    """Run `br <args>` and return stdout (stripped). Raises on non-zero exit."""
    cmd = ["br", *args]
    if description is not None:
        cmd.extend(["--description", description])
    result = subprocess.run(cmd, capture_output=True, text=True, encoding="utf-8")
    if result.returncode != 0:
        sys.stderr.write(f"FAIL: {' '.join(cmd[:6])}...\n")
        sys.stderr.write(f"stderr: {result.stderr}\n")
        sys.stderr.write(f"stdout: {result.stdout}\n")
        sys.exit(result.returncode)
    return result.stdout.strip()


def create_bead(
    *,
    title: str,
    description: str,
    labels: str,
    parent: str | None = None,
    issue_type: str = "task",
    priority: int = 2,
) -> str:
    args = [
        "create",
        "--type", issue_type,
        "--priority", str(priority),
        "--title", title,
        "--labels", labels,
        "--silent",
    ]
    if parent is not None:
        args.extend(["--parent", parent])
    return br(*args, description=description)


def add_dep(issue: str, depends_on: str) -> None:
    """`<issue>` depends on `<depends-on>` — adds a `blocks` edge."""
    br("dep", "add", issue, depends_on)
    print(f"  dep: {issue} <- depends on <- {depends_on}")


# ============================================================================
# A. WS-14 umbrella epic
# ============================================================================

WS14_DESC = """\
Run scope: roll out the WS-14 progressive-disclosure home redesign of \
DnaOneCalc per docs/APP_UX_REALIZATION.md. Replaces the three-mode shell \
(Explore / Inspect / Workbench) with a single home shell whose primary \
surface is editor + result, expanded by progressive drill-downs (formula \
structure, result presentation cascade) and the compare-with-Excel \
workflow. Scenarios become persistence shape via .dnascenario JSON; \
compare bundles become .dnacomparebundle. WS-14 supersedes WS-13.

Authoritative refs:
  - docs/APP_UX_REALIZATION.md (master realization map; OxFml-authoritative formatting model in §2A)
  - docs/ux_artifacts/ws14_progressive_home_mockup.html (interactive mockup)
  - docs/WS14_DESIGN_FORMULA_EDITOR.md (editor area design with FR/NFR/A11Y/BC + 70 test IDs + 16 phases)
  - docs/WS14_DESIGN_LARGE_ARRAY_RESULTS.md (array area design with FR/NFR/HON + 45 test IDs + 13 phases)
  - docs/WS14_PRE_MVP_PATH.md (smallest live slice: 8 build steps)

Acceptance: home_shell.rs is the single mounted shell; editor + result + \
formula drill + result drill + scenarios + compare all ship with SEAM \
stubs where backends pending; .dnascenario and .dnacomparebundle JSON \
round-trip; browser test corpus green; seam status board lists every \
seam; WORKSET_REGISTER.md marks WS-13 superseded; legacy shell \
(OneCalcShellApp + mode shells + formula_editor_surface.rs + \
value_panel.rs + ui/modes/ + ui/panels/) retired.

Roll-out lanes (each a separate set of child beads on this epic, added \
incrementally as the path becomes clear):
  1. Pre-MVP minimal home shell (this first lane; smallest live slice — child beads P0..P9)
  2. Editor surface buildout (overlays per WS14_DESIGN_FORMULA_EDITOR §11 P3..P14)
  3. Drill-downs (formula + result)
  4. Scenarios + persistence
  5. Compare-with-Excel
  6. Command palette + workspace settings + seam status board
  7. Array preview typed cells + virtualization (per WS14_DESIGN_LARGE_ARRAY_RESULTS §14)
  8. Cleanup + retire legacy shell"""

print("Creating WS-14 umbrella epic...")
WS14 = create_bead(
    title="WS-14 DNA OneCalc Progressive-Disclosure Home and Compare Workflow",
    description=WS14_DESC,
    labels="WS-14,ux,workset",
    issue_type="epic",
)
print(f"WS-14 epic: {WS14}\n")


# ============================================================================
# B. Pre-MVP P0..P9 child beads (flat children of WS14)
# ============================================================================

print("Creating Pre-MVP child beads P0..P9...")

P0 = create_bead(
    title="Pre-MVP P0: Verify baseline build, tests, and live preview before WS-14 home_shell work begins",
    labels="WS-14,ws14-pre-mvp,verification,pre-flight",
    parent=WS14,
    description="""\
Run scope: confirm the WS-13-reset baseline still compiles, all existing \
tests pass, and the legacy three-mode shell renders with a live \
OxFml/OxFunc round-trip in the preview server. Pre-flight gate before \
any home_shell.rs work begins. No code changes; closure note records \
evidence of the six pre-flight checks listed in WS14_PRE_MVP_PATH §3.

Outcome: closure note (in this bead's close --reason) recording one line \
of evidence per pre-flight check.

Acceptance:
  (1) `cargo build` exit 0,
  (2) `cargo test --workspace` exit 0,
  (3) scripts/run-onecalc-preview.ps1 starts a local server,
  (4) preview URL renders the existing rail + context bar + Explore body,
  (5) typing into the existing editor produces a live result update via LiveOxfmlBridge,
  (6) `cargo test -p dnaonecalc-host` runs the existing test corpus green.

Path doc: docs/WS14_PRE_MVP_PATH.md §3.
Workset: WS-14.""",
)
print(f"  P0: {P0}")

P1 = create_bead(
    title="Pre-MVP P1: Add home_shell.rs Leptos component skeleton (titlebar + main + statusfoot, no functionality)",
    labels="WS-14,ws14-pre-mvp,ui,home-shell",
    parent=WS14,
    description="""\
Run scope: introduce src/dnaonecalc-host/src/ui/components/home_shell.rs \
as a Leptos component matching the OneCalcShellApp signature shape \
(initial_state, optional editor_bridge), rendering only the empty shell \
scaffolding (titlebar with brand text, empty <main>, empty <footer> for \
status). Add `pub mod home_shell;` to ui/components/mod.rs. No mount \
switch yet; the component is reachable but unmounted so the legacy shell \
continues to render.

Outcome:
  - new file src/dnaonecalc-host/src/ui/components/home_shell.rs with HomeShell component,
  - one-line addition to src/dnaonecalc-host/src/ui/components/mod.rs.

Acceptance:
  (1) `cargo build` exit 0 (existing OneCalcShellApp still compiles),
  (2) `cargo build --target wasm32-unknown-unknown -p dnaonecalc-host --lib` exit 0,
  (3) `cargo test --workspace` exit 0 (no regressions).

Path doc: docs/WS14_PRE_MVP_PATH.md §4 Step 1.
Workset: WS-14.""",
)
print(f"  P1: {P1}")

P2 = create_bead(
    title="Pre-MVP P2: Add services/home_shell_view_model.rs with HomeShellViewModel + ResultView projection",
    labels="WS-14,ws14-pre-mvp,services,view-model",
    parent=WS14,
    description="""\
Run scope: introduce src/dnaonecalc-host/src/services/home_shell_view_model.rs \
exposing HomeShellViewModel { raw_entered_cell_text, editor_surface_state, \
result_view: ResultView, status: StatusView } and a pure function \
build_home_shell_view_model(state: &OneCalcHostState) -> Option<HomeShellViewModel>. \
ResultView covers Empty / Pending / Display { text, kind } / Error { code, \
surface_repr } / Array { rows, cols, label } projecting from \
formula_space.editor_document.value_presentation. StatusView covers \
bridge_health (Live | Stale), truth_source, green_tree_key. Add \
`pub mod home_shell_view_model;` to services/mod.rs.

Outcome:
  - new file src/dnaonecalc-host/src/services/home_shell_view_model.rs,
  - one-line addition to services/mod.rs,
  - in-tree #[cfg(test)] tests covering: empty state -> ResultView::Empty; \
happy SUM -> ResultView::Display { kind: Number }; malformed -> \
ResultView::Error or fallback; SEQUENCE(2,3) -> ResultView::Array { rows: 2, cols: 3 }.

Acceptance:
  (1) `cargo test -p dnaonecalc-host` green; at least 4 new tests in home_shell_view_model module pass,
  (2) build_home_shell_view_model is pure (no signal access, no IO),
  (3) returns None when no active formula space.

Path doc: docs/WS14_PRE_MVP_PATH.md §4 Step 2.
Workset: WS-14.""",
)
print(f"  P2: {P2}")

P3 = create_bead(
    title="Pre-MVP P3: Wire HomeShell <textarea> oninput through services::live_edit::apply_live_editor_input",
    labels="WS-14,ws14-pre-mvp,ui,editor,live-bridge",
    parent=WS14,
    description="""\
Run scope: extend home_shell.rs to mount a native <textarea> with formula \
caption above it (spellcheck="false" autocomplete="off" autocorrect="off" \
autocapitalize="none" aria-label="formula editor"). Wire on:input handler \
that constructs an EditorInputEvent from textarea.value + selection and \
dispatches it via services::live_edit::apply_live_editor_input(bridge, \
state, event) when bridge is Some, falling back to \
app::reducer::apply_editor_input_to_active_formula_space(state, event) \
when None. Read raw_entered_cell_text from the view-model into prop:value. \
Native browser handles arrow keys, selection, IME, clipboard per \
WS14_DESIGN_FORMULA_EDITOR §4 AD-1..AD-5; do NOT wire keydown_to_command \
in this slice; no debouncing.

Outcome:
  - home_shell.rs textarea mounted with on:input dispatcher,
  - reuses existing services::live_edit::apply_live_editor_input,
  - no new service code; no new EditorCommand variants.

Acceptance:
  (1) `cargo build` exit 0,
  (2) `cargo build --target wasm32-unknown-unknown -p dnaonecalc-host --lib` exit 0,
  (3) `cargo test --workspace` green,
  (4) typing into the textarea updates state.formula_spaces[active].raw_entered_cell_text \
(verified after Step 7 mount switch via manual or browser test).

Depends on: P1 (skeleton), P2 (view-model).
Path doc: docs/WS14_PRE_MVP_PATH.md §4 Step 3.
Workset: WS-14.""",
)
print(f"  P3: {P3}")

P4 = create_bead(
    title="Pre-MVP P4: Render result block in HomeShell from HomeShellViewModel.result_view (Number/Error/Array placeholder)",
    labels="WS-14,ws14-pre-mvp,ui,result",
    parent=WS14,
    description="""\
Run scope: extend home_shell.rs with a result section (caption + result \
block) that renders the five ResultView variants:
  - Empty   -> "awaiting input" muted
  - Pending -> "..." muted
  - Display { text, kind } -> large monospace value
  - Error   { code, surface_repr } -> error code in terracotta with light terracotta background
  - Array   { rows, cols, label } -> "Array[R x C]" placeholder text only \
(no preview grid; deferred per WS14_DESIGN_LARGE_ARRAY_RESULTS).
Add minimal CSS extending existing theme.rs ONECALC_THEME_CSS, reusing \
palette tokens (parchment, surface, accent, warm). Do not introduce new \
color tokens.

Outcome:
  - home_shell.rs result section renders all five ResultView variants,
  - CSS additions to theme.rs use existing palette tokens only.

Acceptance:
  (1) `cargo build` + wasm build exit 0,
  (2) `cargo test --workspace` green,
  (3) post-Step-7: typing `=SUM(1,2,3)` shows result `6`,
  (4) post-Step-7: typing `=NOSUCH(1)` shows error code in terracotta,
  (5) post-Step-7: typing `=SEQUENCE(2,3)` shows `Array[2 x 3]` placeholder.

Depends on: P2 (view-model), P3 (input wiring).
Path doc: docs/WS14_PRE_MVP_PATH.md §4 Step 4.
Workset: WS-14.""",
)
print(f"  P4: {P4}")

P5 = create_bead(
    title="Pre-MVP P5: Render HomeShell status foot (live-bridge dot + green-tree key)",
    labels="WS-14,ws14-pre-mvp,ui,status",
    parent=WS14,
    description="""\
Run scope: extend home_shell.rs to render a status foot strip with two \
facts: a colored dot whose color reflects view_model.status.bridge_health \
(sage when Live, amber when Stale), the literal label "live-bridge", a \
separator, and the literal "green-tree " followed by \
status.green_tree_key.as_deref().unwrap_or("—"). bridge_health is derived \
inside build_home_shell_view_model from truth_source plus presence of a \
green_tree_key; no scenario name, no save timestamp, no auto-proof timing \
in this slice.

Outcome:
  - home_shell.rs status foot renders both facts,
  - bridge_health derivation in home_shell_view_model.rs covered by a new in-tree test.

Acceptance:
  (1) `cargo test --workspace` green; new bridge-health derivation test passes,
  (2) status foot renders amber dot when truth_source is LocalFallback,
  (3) status foot renders sage dot when truth_source is LiveBacked.

Depends on: P2 (view-model).
Path doc: docs/WS14_PRE_MVP_PATH.md §4 Step 5.
Workset: WS-14.""",
)
print(f"  P5: {P5}")

P6 = create_bead(
    title="Pre-MVP P6: Add preview_minimal_host_state() seeding a single untitled-1 formula space",
    labels="WS-14,ws14-pre-mvp,preview-state",
    parent=WS14,
    description="""\
Run scope: add a new function preview_minimal_host_state() -> \
OneCalcHostState alongside the existing preview_host_state() in \
src/dnaonecalc-host/src/app/preview_state.rs. The new function seeds a \
single FormulaSpaceId("untitled-1") active formula space with empty text, \
no retained-artifact catalog entries, no demo modes. Do not modify or \
delete the existing preview_host_state(); both coexist until the legacy \
three-mode shell retires in a later WS-14 sweep. Add a clear doc-comment \
on each labelling "legacy three-mode" vs "WS-14 minimal home".

Outcome:
  - new function preview_minimal_host_state() in app/preview_state.rs,
  - existing preview_host_state() untouched.

Acceptance:
  (1) `cargo test --workspace` green (existing tests untouched),
  (2) new in-tree unit test asserts: preview_minimal_host_state() has \
exactly one formula_space, active_formula_space_id = Some("untitled-1"), \
workspace_shell.recent and pinned are empty, retained_artifacts empty.

Path doc: docs/WS14_PRE_MVP_PATH.md §4 Step 6.
Workset: WS-14.""",
)
print(f"  P6: {P6}")

P7 = create_bead(
    title="Pre-MVP P7: Switch wasm mount in lib.rs from OneCalcShellApp to HomeShell + preview_minimal_host_state()",
    labels="WS-14,ws14-pre-mvp,mount,integration",
    parent=WS14,
    description="""\
Run scope: in src/dnaonecalc-host/src/lib.rs::mount_onecalc_preview, \
change the initial_state line from preview_host_state() to \
preview_minimal_host_state(), and the view! macro from \
<OneCalcShellApp ... /> to <HomeShell initial_state=initial_state.clone() \
editor_bridge=Some(editor_bridge.clone()) />. Existing OneCalcShellApp and \
mode shells stay compiled in tree, just no longer mounted. Audit existing \
tests under src/dnaonecalc-host/tests/browser_mounted.rs (and similar) \
for selectors that target the legacy three-mode shell; gate any that \
fail with #[ignore] and a comment pointing at the WS-14 retire phase, \
NOT delete them.

Outcome:
  - 2-line edit in lib.rs::mount_onecalc_preview,
  - any legacy-shell-targeting browser tests gated with #[ignore] + retire comment.

Acceptance:
  (1) `cargo build --target wasm32-unknown-unknown -p dnaonecalc-host --lib` exit 0,
  (2) `cargo test --workspace` green (legacy-shell tests pass or are explicitly ignored),
  (3) scripts/run-onecalc-preview.ps1 builds and serves; preview URL renders \
the new minimal HomeShell instead of the three-mode shell,
  (4) typing `=SUM(1,2,3)` -> result `6`; `=NOSUCH(1)` -> `#NAME?` terracotta; \
`=SEQUENCE(2,3)` -> `Array[2 x 3]`,
  (5) status-foot dot reflects bridge health; green-tree key updates as formula changes.

Depends on: P4 (result block), P5 (status foot), P6 (preview seed).
Path doc: docs/WS14_PRE_MVP_PATH.md §4 Step 7.
Workset: WS-14.""",
)
print(f"  P7: {P7}")

P8 = create_bead(
    title="Pre-MVP P8: Add wasm-bindgen browser test home_shell_typing_sum_shows_result",
    labels="WS-14,ws14-pre-mvp,test,browser",
    parent=WS14,
    description="""\
Run scope: add src/dnaonecalc-host/tests/browser_home_shell.rs with one \
wasm-bindgen-test that mounts the minimal HomeShell into a detached DOM \
node, types `=SUM(1,2,3)` into the textarea, waits for the bridge \
round-trip to settle, and asserts the result block's .value text equals \
"6". Add a small mount_test_home_shell helper in \
src/dnaonecalc-host/src/test_support/ that uses preview_minimal_host_state() \
and wires either a live or stub bridge per the test's needs.

Outcome:
  - new test file tests/browser_home_shell.rs with exactly one passing #[wasm_bindgen_test],
  - new mount_test_home_shell helper in test_support module,
  - wasm-pack test --headless --chrome (or scripts/run-onecalc-preview.ps1's test mode) green.

Acceptance:
  (1) browser test green via headless Chromium,
  (2) test asserts result text content == "6" (post-bridge-settle),
  (3) test cleans up the detached DOM node on completion (no leak across tests).

Depends on: P7 (mount switch).
Path doc: docs/WS14_PRE_MVP_PATH.md §4 Step 8.
Workset: WS-14.""",
)
print(f"  P8: {P8}")

P9 = create_bead(
    title="Pre-MVP P9: Acceptance sweep — verify all 10 acceptance checks pass end-to-end",
    labels="WS-14,ws14-pre-mvp,verification",
    parent=WS14,
    description="""\
Run scope: execute the 10-point acceptance check from \
docs/WS14_PRE_MVP_PATH.md §5 against a fresh checkout. Capture evidence \
(screenshots or console logs as appropriate) for each check. This bead \
closes the Pre-MVP slice of WS-14.

Outcome:
  - documented closure note (in this bead's close --reason) covering all 10 checks (A.1 through A.10),
  - any newly discovered required work added to the bead graph as new \
beads (per BEADS.md §7).

Acceptance:
  (A.1)  page loads with minimal home shell,
  (A.2)  =SUM(1,2,3) -> 6 within ~200 ms; sage dot; non-empty green-tree key,
  (A.3)  appending stray ( does not crash; result remains stable,
  (A.4)  =NOSUCH(1) -> #NAME? terracotta,
  (A.5)  =SEQUENCE(2,3) -> Array[2 x 3] placeholder,
  (A.6)  reload yields empty editor (no persistence — correct per scope),
  (A.7)  browser resize keeps layout stable,
  (A.8)  Tab from textarea produces predictable focus order,
  (A.9)  `cargo test --workspace` green,
  (A.10) browser test home_shell_typing_sum_shows_result green.

Depends on: P8 (browser test).
Path doc: docs/WS14_PRE_MVP_PATH.md §5.
Workset: WS-14.""",
)
print(f"  P9: {P9}\n")


# ============================================================================
# C. Dependency edges (`<issue>` blocks `<depends-on>` => issue must wait)
# ============================================================================

print("Adding dependency edges...")

# P0 is pre-flight; everything downstream waits on it.
add_dep(P1, P0)
add_dep(P2, P0)
add_dep(P6, P0)

# P3 needs skeleton + view-model.
add_dep(P3, P1)
add_dep(P3, P2)

# P4 needs view-model + input wiring.
add_dep(P4, P2)
add_dep(P4, P3)

# P5 needs view-model.
add_dep(P5, P2)

# P7 needs result block + status foot + preview seed.
add_dep(P7, P4)
add_dep(P7, P5)
add_dep(P7, P6)

# P8 needs the mount switched.
add_dep(P8, P7)

# P9 needs the browser test in place.
add_dep(P9, P8)

print()
print("=" * 70)
print("Pre-MVP bead set rolled out under WS-14 epic.")
print("=" * 70)
print(f"Epic     : {WS14}")
print(f"Children : {P0}, {P1}, {P2}, {P3}, {P4}, {P5}, {P6}, {P7}, {P8}, {P9}")
print()
print("Inspect:")
print(f"  br show {WS14}")
print(f"  br dep tree {WS14}")
print(f"  br ready --json")
