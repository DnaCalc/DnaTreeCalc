# WS-13 DNA OneCalc UX Revamp Across Editor, Case Management, Host Config, And Value / Parity Surfaces

Status: `draft_ux_revamp`
Date: 2026-04-14

Companion notes:
1. [WORKSET_REGISTER.md](../WORKSET_REGISTER.md) — entry for WS-13 points here.
2. [APP_UX_BRIEF.md](../APP_UX_BRIEF.md)
3. [APP_UX_ARCHITECTURE.md](../APP_UX_ARCHITECTURE.md)
4. [APP_UX_FORMULA_EDITOR_SPEC.md](../APP_UX_FORMULA_EDITOR_SPEC.md)
5. [APP_UX_PANEL_INVENTORY.md](../APP_UX_PANEL_INVENTORY.md)
6. [APP_UX_SCREEN_SPEC_EXPLORE.md](../APP_UX_SCREEN_SPEC_EXPLORE.md)
7. [APP_UX_SCREEN_SPEC_INSPECT.md](../APP_UX_SCREEN_SPEC_INSPECT.md)
8. [APP_UX_SCREEN_SPEC_WORKBENCH.md](../APP_UX_SCREEN_SPEC_WORKBENCH.md)
9. [APP_UX_USE_CASES.md](../APP_UX_USE_CASES.md)
10. [APP_UX_HOST_STATE_SLICING.md](../APP_UX_HOST_STATE_SLICING.md)
11. [ux_artifacts/figma_make/2026-04-04/](../ux_artifacts/figma_make/2026-04-04/) — explore / workbench / inspect mockup variants.

## 0. Workset Contract

1. **workset id**: `WS-13`
2. **title**: DNA OneCalc UX Revamp Across Editor, Case Management, Host Config, And Value / Parity Surfaces
3. **purpose**:
   land a coordinated UX improvement wave across the formula editor, case /
   formula-space lifecycle, host/caller configuration (including the full Excel
   `Format Cells` and `Conditional Formatting` surface), and the cross-mode
   Value Panel and Parity Matrix surfaces — so that the full intended UX is
   visible to users with every engine-dependent control either live or
   explicitly marked `<NOT IMPLEMENTED>` with a `SEAM-*` id that names the
   engine work required to activate it.
4. **depends_on**: `WS-02`, `WS-05`, `WS-08`, `WS-11`
5. **parent_spec_sections**:
   `3`, `4.2`, `5`, `5.3`, `6.0` through `6.10`, `7.1`, `7.2`, `8`, `9.1`,
   `9.3`, `9.4`, `9.6`, `10`, `11`, `16`
6. **upstream_dependencies**:
   `OxFml`, `OxFunc` (both the `oxfunc_value_types` crate and the
   locale/format-code engine in `oxfunc_core`), `OxXlPlay`, `OxReplay` trace
   events as consumed through the verification bundle path.
7. **closure_condition**:
   the Explore / Inspect / Workbench shells expose the full editor,
   case-management, host-configuration, cell-formatting, conditional-formatting,
   scenario-policy, host-binding, calc-options, Value Panel, and Workbench
   Parity Matrix surfaces named in this workset; every control whose behaviour
   depends on engine work that has not yet landed renders in place with a
   visible `<NOT IMPLEMENTED>` badge carrying its `SEAM-*` id; the workspace
   JSON v1 persistence layer round-trips every affected field; and the seam
   status board on the workspace settings page enumerates the live set of
   pending seams so upstream work is visible to users and engineers.
8. **initial_epic_lanes**:
   formula editor control, case / formula-space lifecycle, configure drawer
   chrome plus the six `Format Cells` parity tabs and the CF rules manager,
   scenario policy / host bindings / calc options tabs, workspace settings page
   and seam status board, Value Panel component and cross-mode integration,
   Workbench Parity Matrix and trace consumption, Excel parity test harness.

## Context

DNA OneCalc is positioned in `APP_UX_BRIEF.md` and `APP_UX_FORMULA_EDITOR_SPEC.md` as an **interactive formula explorer** — a single‑formula host that lets a user type, discover, evaluate, inspect and compare Excel‑style formulas through a tight OxFml language‑service loop. Three task modes (Explore / Inspect / Workbench) are layered over a shared "formula space" object that is the unit of work.

The current Rust/Leptos implementation under `src/dnaonecalc-host/src/ui/` has shipped the structural skeleton for all three modes (`shell_frame.rs`, `explore_shell.rs`, `inspect_shell.rs`, `workbench_shell.rs`), a textarea‑based formula surface (`ui/editor/`, `ui/components/formula_editor_surface.rs`) wired to the OxFml edit‑packet loop, and read‑only view‑model projections of host context. But against both the UX docs and the three Figma Make explorations under `docs/ux_artifacts/figma_make/2026-04-04/{explore,workbench,inspect}/`, three areas are conspicuously thin:

1. **The formula editor surface itself.** State and overlays exist, but the live affordances a user actually feels — completion popup at the caret, signature ScreenTip, inline diagnostic squiggles, current‑help card, F4 reference cycling, F2 edit‑mode, syntax coloration in place — are either rendered out of band in the right "Assist" panel or not rendered at all. The editor anchors (`completion_anchor_offset`, `signature_help_anchor_offset`, `EditorMeasuredOverlayBox`) are tracked but no positioned popup consumes them.
2. **Case / formula‑space lifecycle.** `open_retained_artifact_from_catalog`, `import_manual_retained_artifact_into_active_formula_space` and `import_verification_bundle_report_json` exist in `services/` and `state/reducer.rs`, but nothing in the shell invokes them; there is no "New", no "Open", no "Save / Retain", no recents, no rename, no duplicate, no command palette. The shell is bootstrapped from an `initial_state` argument and a user cannot author or persist a case from the running app.
3. **Formatting, conditional formatting, and host/caller configuration.** `FormulaSpaceContextState` carries `host_profile`, `capability_floor`, `truth_source`, etc. as labels, and the panels in `APP_UX_PANEL_INVENTORY.md` for `scenario_policy_panel`, formatting and conditional formatting are still `planned` / `needs_clarification`. There is no UI through which the user edits any of these — they are facts shipped from the bootstrap and displayed.

This plan defines the user experience target for these three areas, anchored to the UX spec language and the warm "editorial" Figma direction (parchment / oxidized teal / amber brass / terracotta / moss), and lays out the concrete UX work — control inventories, layouts, interaction flows, and the seam each area pushes back to OxFml / OxXlPlay / OxFunc. It does **not** prescribe Rust module shapes; the implementation note that follows this plan should pick those up.

The aim across all three areas is to make the product behave, from the first second of use, like an *interactive formula explorer*: the user opens it, finds or creates a formula space, types or pastes a formula, sees it light up with structure and diagnostics in place, gets help inline at the caret, retains a run as evidence, and can adjust scenario / format context without leaving the formula. None of the three areas can be scoped tightly without touching the others, so this plan treats them as one coordinated UX wave.

### Engine updates that shape this plan

Two sibling‑repo changes landed recently and shape every area below. They are **not** UX changes on their own, but they move material that the UX must surface from "implied" to "observable".

**1. OxFunc value representation moved to `oxfunc_value_types`.** All value types (`EvalValue`, `CellContentValue`, `CallArgValue`, `EvalArray`, `ArrayCellValue`, `ArrayShape`, `RichValue`, `RichValueData`, `RichArray`, `RichValueType` with `key_flags` including `ExcludeFromCalcComparison`, `LambdaValue`, `CallableArityShape`, `CallableOriginKind`, `CallableCaptureMode`, `ReferenceLike`, `ReferenceKind`, `ExcelText` with 32,767 UTF‑16 code unit enforcement, `WorksheetErrorCode`, `ErrorSurface`, `ValueTag`, `ValueBoundary`, `PresentationHint`, `NumberFormatHint`, `CellStyleHint`, and `ExtendedValue { Core, RichValue, ValueWithPresentation { value, hint }, ErrorWithMetadata }`) now live in `C:/Work/DnaCalc/OxFunc/crates/oxfunc_value_types/src/lib.rs`. `oxfunc_core` re‑exports via `pub use oxfunc_value_types::*;` and OxFml consumes them through `oxfunc_core::value::*` at `consumer/editor/types.rs`, `interface/mod.rs`, `publication/mod.rs`, `eval/mod.rs`, `consumer/replay/mod.rs`, `host/mod.rs`. The UX implications are large: **presentation is now first‑class data on the value**, not a side channel; `ExtendedValue::ValueWithPresentation { value, hint }` gives OneCalc a typed carrier to render a result *and* its hinted number format together; and rich values / nested rich arrays are now structurally addressable, which means Inspect and Workbench can walk them instead of stringifying them.

**2. Richer Excel ↔ OxFml comparison.** `VerificationCaseReport` (`src/dnaonecalc-host/src/services/verification_bundle.rs` lines 123‑141) now carries three independent verdicts — `value_match`, `display_match`, `replay_equivalent` — plus `replay_mismatch_kinds: Vec<String>`, `replay_mismatch_records: Vec<OxReplayMismatchRecord>` (with `mismatch_kind`, `severity`, `view_family`, `left_value_repr`, `right_value_repr`, `detail`), and `replay_explain_records: Vec<OxReplayExplainRecord>` (adds `query_id` linking to trace events and `summary`). The summaries expose `OxfmlVerificationSummary { comparison_value, effective_display_summary, ... }` and `ExcelObservationSummary { comparison_value, observed_value_repr, effective_display_text, capture_status, ... }`. `RetainedArtifactRecord` in `src/dnaonecalc-host/src/state/types.rs` lines 197‑217 propagates these. `ReplayComparisonView { view_family, value }` on `ReplayProjectionResult` gives polymorphic comparison families. `ProgrammaticComparisonLane` enumerates `OxfmlOnly / OxfmlAndExcel / ExcelObservationBlocked`, and `ProgrammaticComparisonStatus` (`Matched / Mismatched / Blocked`) already drives the `open_mode_hint` (Matched → Inspect, Mismatched / Blocked → Workbench). The UX implications are that parity is now **a surface with structure**, not a single boolean: the user needs a matrix (value / display / replay) with mismatch taxonomy and trace‑linked explanations, not a "green / red" badge.

A fourth section of this plan (Area 4) covers the cross‑cutting UI surfaces these two changes enable. Areas 1‑3 are patched where they touch this material.

---

## Area 0 — Minimal Foundation Reset

### Why this section exists

Through the WS‑13 execution lane (`dno-yjk.1` through `dno-yjk.7`) the host accumulated a substantial Leptos UI surface — three‑mode shell with breadcrumbs, scope strip, mode accents, rail with case lifecycle affordances and verdict badges, ShellDrawer primitive, Value Panel, formula editor with overlay layers, completion popup, signature ScreenTip, settings popover, bracket pair highlight, F4 reference cycling, auto‑proof debounce, fallback mode, and the Explore vertical rhythm redesign. The view‑model layer that drives all of that is sound; the **interactive behaviour of the rendered surface is not**. Four interactive regressions in a row in the editor alone — spellcheck (`yjk.4`), arrow‑key hijacking (`yjk.5`), layout (`yjk.6`), and the cursor‑snap‑back from `yjk.5`'s sync helper — surfaced only when running the preview, not when running unit tests, because no test layer exercises the actual DOM. The user has lost confidence in the rendered surface and asked for a hard reset of the front‑end while the view‑model layer is preserved.

This section reframes WS‑13 as a two‑phase plan:

1. **Phase A (this section, Area 0).** Stand up a deliberately‑minimal text‑editor + result interface against the existing view‑model layer. Build the missing test foundation (a wasm‑bindgen browser test layer + automated UX validation criteria). Move the existing rich Leptos surface to an archive folder once the test foundation is in place. Ship a runnable preview that lets a user enter a formula and see the result from OxFml/OxFunc.
2. **Phase B (the existing Areas 1‑4 of this workset).** Reintroduce the rich UX surface incrementally, **gated by the new test foundation**, drawing on the archived implementation as a reference. Every reintroduced feature must pin its interactive behaviour with a wasm‑bindgen test before it's allowed back into the running shell.

The view‑model layer (state, reducers, services, adapters, editor model — everything documented in [`HOST_VIEW_MODEL_REFERENCE.md`](../HOST_VIEW_MODEL_REFERENCE.md)) is preserved unchanged. None of the WS‑13 plan's assumptions about that layer are invalidated. What changes is the rendering layer above it and the order in which the rich surfaces from Phase B come back.

### Anchor document for the reset

[`docs/HOST_VIEW_MODEL_REFERENCE.md`](../HOST_VIEW_MODEL_REFERENCE.md) is the load‑bearing reference for everything below. It documents:

- The state layer (every type and field on `OneCalcHostState` and its descendants).
- The app layer (every reducer in `app/reducer.rs` and `app/case_lifecycle.rs`, the intent shape, the bootstrap path).
- The services layer (every `build_*_view_model` function, the live‑edit bridge wrapper, the editor session service, the verification bundle parser, the retained‑artifact catalog manager).
- The adapters layer (the `OxfmlEditorBridge` trait, `EditorDocument`, `FormulaEditRequest` / `FormulaEditResult`, the preview and live bridge implementations).
- The editor model layer (commands, state, bracket matcher, reference cycle, render projection, geometry).
- A complete implementation matrix (§10) marking every feature 🟢 LIVE / 🟡 PARTIAL / 🔴 FACADE, cross‑referenced to the `SEAM-*` register in WS‑13 Appendix B.
- A test‑invariants section (§11) listing the invariants the new test suite should pin at every layer, including the wasm‑bindgen browser invariants the missing test layer must enforce.

The reference is the source of truth for what currently works and what doesn't. Read it before touching anything in the host.

### Phase A scope (what gets built)

#### A.1 — Minimal text editor

A single `<textarea>` element. No syntax highlighting overlay. No completion popup. No signature ScreenTip. No bracket pair highlight. No diagnostic squiggles. No settings popover. No live‑state pills. No expanded‑height toggle. No fallback mode (it *is* the fallback mode).

The textarea:

- Has `spellcheck="false"` and `autocomplete="off"`.
- Wires `on:input` to dispatch `EditorInputEvent { text, selection_start, selection_end, input_kind, inserted_text }` through the existing `apply_live_editor_input` path so the bridge round‑trips and `editor_document` populates.
- Wires `on:keydown` to **not** call `keydown_to_command` at all in Phase A — the editor owns no keys. Tab navigates focus the way the browser wants. Every key is the textarea's. This is deliberately stricter than the current behaviour and exists so the cursor‑tracking class of bug cannot reappear.
- Renders the textarea's value via Leptos `prop:value=raw_entered_cell_text` and **does not** ever try to push DOM selection from logical state. `schedule_textarea_selection_sync` is gone.

Reusable view‑model state is read but no overlay or popup chrome is rendered against it. The editor settings popover, the bracket pair, the completion popup, the signature ScreenTip, the syntax run overlay, and the diagnostic band footer all stay in the codebase but are not mounted by the minimal interface.

#### A.2 — Minimal result interface

A two‑pane layout immediately under the editor:

- **Top pane**: the textarea (above).
- **Bottom pane**: a single block showing the current `effective_display_summary` (or `latest_evaluation_summary` when display is empty), plus a small read‑only block showing the first three diagnostic messages from `editor_document.live_diagnostics` if any are present, plus the count of completion proposals (if any) as a number, plus the `green_tree_key` as a small monospace footnote.

That is the entire interface. No rail, no breadcrumbs, no scope strip, no mode tabs, no Configure button, no rich Value Panel rendering, no array preview grid (just the array as a flat one‑line string), no Inspect mode, no Workbench mode, no retained artifact catalog. The minimum surface that proves OneCalc can take a formula and show what OxFml/OxFunc say about it.

The minimal interface exists at a new component path so it doesn't entangle with the archived rich shell:

- `src/dnaonecalc-host/src/ui/components/minimal_app.rs` — top‑level `MinimalOneCalcApp` Leptos component. Wraps a `RwSignal<OneCalcHostState>` and renders `MinimalEditor` + `MinimalResult`. Wires `on_input_event` and `on_overlay_measurement` to the existing reducer entry points.
- `src/dnaonecalc-host/src/ui/components/minimal_editor.rs` — pure `<textarea>` mount as described in A.1.
- `src/dnaonecalc-host/src/ui/components/minimal_result.rs` — the read‑only result/diagnostics/help block.

`OneCalcShellApp` (the existing rich app) is left in place but is no longer the default entry point. `lib.rs::mount_onecalc_preview` is updated to mount `MinimalOneCalcApp` instead. `app/host_mount.rs::render_shell_html` is updated similarly. CLI entry points in `main.rs` are unchanged because they don't render a UI.

#### A.3 — Test foundation (the missing layer)

A new `tests/browser/` integration test crate using `wasm-bindgen-test` that runs against `wasm32-unknown-unknown` in a headless browser. Tests pin the interactive invariants listed in [`HOST_VIEW_MODEL_REFERENCE.md`](../HOST_VIEW_MODEL_REFERENCE.md) §11.6:

1. Click at character offset N in the textarea → DOM `selectionStart == selectionEnd == N`.
2. Press left arrow at offset N > 0 → DOM cursor moves to N − 1.
3. Press right arrow at offset N < len → DOM cursor moves to N + 1.
4. Press Enter at offset N → textarea contains `\n` at byte position N and DOM caret advances by one.
5. Press Backspace at offset N > 0 → character at N − 1 removed, caret moves to N − 1.
6. Press Delete at offset N < len → character at N removed, caret stays at N.
7. Type `=SUM(1,2,3)` and confirm `effective_display_summary` shows `6` (or whatever the live bridge says) inside one bridge round‑trip.
8. Type `=SEQUENCE(2,2)` and confirm `array_preview` populates with the four cells.
9. Type a malformed formula and confirm the diagnostic count rises.
10. Confirm `spellcheck="false"` is present on the textarea and the browser's spellchecker does not underline `SUM`, `LET`, `MOD`.

These tests are the **first‑priority test corpus**. They run on every CI execution (a new `scripts/run-browser-tests.ps1` invocation) and any future editor work must keep them green.

In addition, a new `tests/host_state/` integration test crate (plain `cargo test`, no browser) pins the state and reducer invariants from `HOST_VIEW_MODEL_REFERENCE.md` §§11.1–11.5. These are the invariants the existing 200‑test in‑tree suite already covers indirectly; restating them as a dedicated invariant suite makes them explicit and lets the host evolve without losing coverage.

#### A.4 — Automated UX validation criteria

A new `scripts/run-ux-validation.ps1` script that, after a successful `scripts/run-browser-tests.ps1`, exercises the minimal interface end to end and asserts the user‑visible invariants:

- The textarea is visible within the top 200 px of the viewport on a 1440×900 display.
- After typing `=1+1`, the result block contains `2` within one bridge round‑trip.
- After typing `=1/0`, the result block shows the error code (or the live bridge's blocked reason).
- After clicking somewhere in the textarea and pressing left arrow, the cursor lands one position left of the click position (the canonical regression check from `yjk.5`).

These are deliberately small. They run in CI alongside the browser tests. Any reintroduced rich surface (Phase B) is gated by an extension of this validation script; the rule is **no surface comes back without an automated UX validation criterion that exercises it**.

#### A.5 — Archive of the existing rich front‑end

Once A.1, A.2, A.3, A.4 are landing and green, move the existing rich Leptos surface to a new archive folder so it stays available for reference but doesn't compile or affect the running app:

- `src/dnaonecalc-host/src/ui_archive_2026_04/` — a new module (cfg‑gated `#[cfg(feature = "ui-archive-2026-04")]` so it doesn't appear in the default build).
- Move into it: `app_shell.rs`, `explore_shell.rs`, `inspect_shell.rs`, `workbench_shell.rs`, `shell_frame.rs`, `shell_drawer.rs`, `formula_editor_surface.rs`, `value_panel.rs`, plus their helper modules and tests.
- Delete the corresponding `pub mod` declarations from `src/dnaonecalc-host/src/ui/components/mod.rs`. The minimal app stays in `ui/components/minimal_*.rs`.
- Keep the editor model layer (`ui/editor/*`), the panels (`ui/panels/*`), and the design tokens (`ui/design_tokens/theme.rs`) in place; the minimal interface uses a small subset of them and the archive can be reactivated later via the cargo feature.
- Add `[features] ui-archive-2026-04 = []` to `Cargo.toml` so the archive can be selectively re‑built when someone wants to compare the new minimal interface against the old rich one.

Archiving is **gated on A.3 and A.4 being landed and green**. Until the test layer exists, the archive stays in place as a reference for what we're rebuilding away from.

### Phase A non‑goals

- No syntax highlighting in the textarea.
- No completion popup, signature help, or function help rendering.
- No mode switcher; Phase A surfaces only the equivalent of "Explore minimal".
- No retained artifact catalog UI; the catalog still exists in state and the CLI entry points still write to it, but Phase A doesn't render it.
- No Configure drawer, no scope strip, no breadcrumb, no rail.
- No mode accent colours.
- No Value Panel structural rendering.
- No persistence to disk.
- No session restore.

These are all deferred to Phase B and only return after the relevant test invariants are in place.

### Phase A acceptance

Phase A is complete when:

1. `scripts/run-onecalc-preview.ps1` opens a window where a user can type `=SUM(1,2,3)` into a textarea and see `6` (or the live bridge's evaluated result) in the result block.
2. `cargo test -p dnaonecalc-host` is green across lib + integration suites.
3. `scripts/run-browser-tests.ps1` runs and is green for the test corpus in A.3.
4. `scripts/run-ux-validation.ps1` runs and is green for the validation criteria in A.4.
5. `cargo check -p dnaonecalc-host --target wasm32-unknown-unknown` is clean.
6. The existing rich Leptos surface is moved to `ui_archive_2026_04/` behind the `ui-archive-2026-04` feature flag, with `cargo check --features ui-archive-2026-04` still building cleanly so the archive stays exercisable.
7. [`HOST_VIEW_MODEL_REFERENCE.md`](../HOST_VIEW_MODEL_REFERENCE.md) is up to date with any changes the reset introduces (it should mostly be unchanged because Phase A leaves the view‑model layer alone).

### Phase A work packaging

This section maps to a new bead family under `dno-yjk` (the WS‑13 epic). One bead per A.x slice:

- **`dno-yjk.A1`** — Build `MinimalOneCalcApp` + `MinimalEditor` + `MinimalResult`. Wire `lib.rs::mount_onecalc_preview` to it. P1 on the visible‑UI lane.
- **`dno-yjk.A2`** — Add `tests/host_state/` invariant suite covering `HOST_VIEW_MODEL_REFERENCE.md` §§11.1–11.5. P1 because it's the prerequisite for any further work.
- **`dno-yjk.A3`** — Stand up `tests/browser/` with `wasm-bindgen-test`. Pin the §11.6 corpus. Add `scripts/run-browser-tests.ps1`. P1 because this is the missing test layer.
- **`dno-yjk.A4`** — Add `scripts/run-ux-validation.ps1` exercising the canonical regressions. P2.
- **`dno-yjk.A5`** — Move the existing rich Leptos surface into `ui_archive_2026_04/` behind a cargo feature. Update `mod.rs` declarations. Verify both the default build and `--features ui-archive-2026-04` build cleanly. **Blocked by A3 and A4.**
- **`dno-yjk.A6`** — Update `HOST_VIEW_MODEL_REFERENCE.md` to record any changes the reset introduces. Trivial; runs alongside A5.

Phase B (Areas 1‑4 of this workset) does not start until Phase A is fully closed. The existing yjk beads `dno-yjk.6` (layout regressions) is folded into the archive — the archived code carries the regressions but doesn't run.

### Reasoning for the reset (cited honestly)

Four interactive regressions in a row in the editor alone, all caused by the same missing test layer:

1. **`dno-yjk.4`** — browser spellcheck on the textarea was never tested because no test exercises a real browser. Caught on first preview launch.
2. **`dno-yjk.5`** (first attempt) — arrow keys / backspace / delete / plain Enter were hijacked into reducer commands with `prevent_default`, breaking native cursor tracking. Caught on second preview launch.
3. **`dno-yjk.5`** (sync helper) — the fix for cursor reset after Tab indent / completion accept was a global `set_timeout(0)` sync helper that pushed logical selection to the DOM on every render. Because click and arrow keys don't update the logical selection, the helper unconditionally clobbered the user's cursor with the stale logical position, producing the canonical "click jumps to end" regression. Caught on third preview launch.
4. **`dno-yjk.6`** — layout regressions from Value Panel nesting / settings popover / toolbar pills / pipeline chip overflow. Caught visually only.

The pattern: the existing 200 unit + integration tests verify HTML markup and Rust‑level logic but **never simulate a click, a keydown, or a selection change against a live DOM**. The plan's verification section flagged this risk explicitly when `dno-yjk.5` was created ("Add a wasm-bindgen-test browser smoke test under `tests/editor/` that types a sequence into the textarea and asserts the logical caret equals the DOM selection after each keystroke — the missing test layer that let this regression past the existing HTML-assertion unit tests"), but the test layer was never built. Every editor slice landed against HTML‑assertion tests, every preview launch caught a new regression, every fix produced a different regression, and the cycle did not break until the user lost confidence and asked for a reset.

The reset is **not** a statement that the WS‑13 plan or the view‑model layer is wrong. The state, reducers, services, and adapters work — `HOST_VIEW_MODEL_REFERENCE.md` §10's matrix shows the surface is largely 🟢 LIVE or 🟡 PARTIAL on a real OxFml seam, with the few 🔴 FACADE items being intentional placeholders for upstream library work. The reset is a statement that **rendering work cannot be trusted without a test layer that pins interactive behaviour**, and the path back to a rich UX is "minimal interface first → test foundation second → rich surface gated on tests third".

---

## Area 1 — Formula Editor Control: Spec, Visual Design, Interaction

### Source‑of‑truth references

- `docs/APP_UX_FORMULA_EDITOR_SPEC.md` §§ 2A, 4.1, 7, 8, 11, 12, 14 — OxFml coupling, compat floor, core requirements, layered surfaces, contract shape, configuration, staged feature set.
- `docs/APP_UX_SCREEN_SPEC_EXPLORE.md` and `APP_UX_PANEL_INVENTORY.md` for the surrounding Explore layout.
- `docs/ux_artifacts/figma_make/2026-04-04/explore/REFINED_DESIGN.md` and `EXPLORE_MODE.md` — visual treatment, syntax overlay model, drawer affordances.
- Existing code: `src/dnaonecalc-host/src/ui/editor/{commands,state,geometry,render_projection,browser_measurement}.rs`, `ui/components/formula_editor_surface.rs`, `ui/components/explore_shell.rs`.

### Target experience (what the user should feel)

The formula editor should be the visual and interaction centre of Explore. It is a single multiline cell entry that *looks* like a code surface — line numbers, monospace, syntax coloration, gutter — but *behaves* like Excel's formula bar in every place where Excel has a documented behaviour, and is honest about the platform when the browser cannot match a desktop chord. Every help affordance the user needs is anchored to where their caret is, not parked in a separate panel. The reduced‑motion, monochrome fallback path remains usable when overlays cannot mount.

Central to this target experience is a **vertical‑rhythm promise**: the editor textarea's first line lands within the top `~200px` of the viewport on an ordinary laptop display, not buried below prose or metadata chrome. `APP_UX_SCREEN_SPEC_EXPLORE.md` §§5–7 names `formula_editor_panel` and `result_panel` as the primary panels and states "the editor should remain the dominant visible surface", "scrolling inside the editor should not displace the result entirely", and "the user should not need to leave `Explore` just to navigate the formula". The Figma `EXPLORE_MODE.md` layout diagram (lines 28‑103) and `CLARITY_IMPROVEMENTS.md` (§§172‑188) corroborate this shape: everything above the three‑column body is the shell chrome (top bar + formula‑space context bar), and the three columns — **Formula Editor 40% / Result + array 35% / Completion + help 25%** — occupy the full remaining canvas. `INFORMATION_ARCHITECTURE.md` (§§210‑250) enumerates exactly three things as "Always Visible (Default State)": the editor, the result panel, and the completion+help column. **Nothing else belongs above the three‑column body.** The implementation discipline that enforces this is called out below under *Explore shell layout discipline*.

The editor opens in one of three *entry modes*, which the spec documents but the current UI does not yet distinguish:

- **Formula entry** — text starts with `=`, formula affordances are armed.
- **Direct value entry** — `123.4`, `2026-04-14`, `true`, etc. No formula affordances appear.
- **String entry** — leading apostrophe forces string interpretation; preserved across commit.

These are detected by OxFml on every keystroke and reflected in a small *entry‑mode pill* on the editor's top‑right (so the user understands why function help is or isn't appearing). Once a proof is available, a second *result‑class pill* appears next to the entry‑mode pill showing the `EvalValue` variant of the current result: `Number`, `Text`, `Logical`, `Error(<code>)` where `<code>` is the `WorksheetErrorCode` (`#N/A`, `#VALUE!`, `#DIV/0!`, `#REF!`, `#NAME?`, `#NUM!`, `#NULL!`, `#SPILL!`, `#CALC!`, `#BUSY`, `#GETTING_DATA`, `#FIELD`, `#BLOCKED`, `#CONNECT`), `Array[r×c]`, `Reference(A1|Area|MultiArea|3D|Structured|SpillAnchor)`, `Lambda`, `Rich(<type_name>)`. This is the user‑visible answer to "what kind of thing did I get back?" and replaces the current editor's single "diagnostic count" label.

The result hero below the editor consumes `ExtendedValue`:

- For `Core(EvalValue)`, render `effective_display_text` from `VerificationPublicationSurface` as the large value and nothing else.
- For `ValueWithPresentation { value, hint }`, render `effective_display_text` large, with a small subline below showing the `PresentationHint` (`Currency`, `Percentage`, `DateLike`, `Scientific`, `Fraction`, `Custom`, `Hyperlink`) so the user can see the engine's classification even when their format tab hasn't been touched.
- For `RichValue(..)`, render the rich value's fallback as the large value and expose a "View rich structure →" affordance that opens Area 4's rich‑value inspector.
- For `ErrorWithMetadata { code, surface }`, render the error code large in the terracotta error colour, with the `ErrorSurface` (`Worksheet` / `XllTransferable` / `ExtendedWorksheetOnly`) as a small sidecar badge so power users can tell whether the error is worksheet‑visible or extended‑only.
- For `EvalValue::Array`, render a scrollable spill preview with `ArrayShape` as a header (`Array[3×4]`) and a "Full array in Inspect →" affordance.

### Control inventory and layered surfaces (spec §8)

| Layer | Control / element | UX role |
|---|---|---|
| Native input | `<textarea>` with transparent caret/selection (current) | Authoritative caret, IME, selection, paste, undo |
| Presentation | Syntax run overlay (current `render_projection.rs`) | Function/number/operator/identifier/text colours from `EditorSyntaxSnapshot` |
| Presentation | Bracket pair highlight | When caret is on a `(`, `)`, `{`, `}`, `[`, `]` token, fade match in same colour |
| Presentation | Diagnostic squiggle layer | Wavy underline per `LiveDiagnosticSnapshot` span, severity colour (warning amber, error terracotta) |
| Presentation | Selection‑aware reference highlight | When a reference token is in selection, faint pill so the user knows F4 will affect it |
| Presentation | Active‑argument highlight | When signature help is live, the current argument span gets a subtle background |
| Suggestion / help | Completion popup | Anchored to `completion_anchor_offset` via `EditorMeasuredOverlayBox`, max ~8 visible items, keyboard‑navigable, `Tab` accepts |
| Suggestion / help | Signature help ScreenTip | Anchored above caret while inside `func(`; bold active argument; collapses on `)` or comma navigation |
| Suggestion / help | Current help card | Sidecar (right column) when window is wide; collapses into a small "?" disclosure that opens an inline popover when narrow |
| Suggestion / help | Enum / constant picker | Activated when OxFunc metadata says the active argument is an enum |
| Inspect bridge | Selection → Inspect "send to walk" affordance | Right‑click or `Ctrl+Alt+I`: scrolls Formula Walk to the green‑tree node containing the selection |
| Editor chrome | Line number gutter | Already present; should adopt warm‑smoke token, dim past last line |
| Editor chrome | Entry‑mode pill | "Formula" / "Value" / "Text" — colour from theme |
| Editor chrome | Reuse / timing badge | Optional; surfaces the OxFml reuse summary as a small "incremental" / "full" pill, helps explain editor liveness |
| Editor chrome | Expand / collapse handle | Excel `Ctrl+Shift+U` equivalent — grows the editor vertically into the result column |
| Status footer | Caret position (line, column), selection length, character count | Same row as the existing capability floor footer |
| Fallback | Plain `<textarea>` mode | Activated when overlays fail to mount or `prefers-reduced-motion` + tokens flag set |

### Keyboard map (Excel compat floor, spec §4.1)

| Key | Behaviour | Notes |
|---|---|---|
| `=` | Enters formula entry mode | OxFml driven, not local |
| `'` (leading) | Enters string entry mode | OxFml driven |
| `Tab` | Accept selected completion when popup open; otherwise indent | Already partially implemented (`IndentWithSpaces`) |
| `Shift+Tab` | Outdent | Already implemented |
| `Enter` | Commit current entry, fire OxFml proof, retain run | Currently inserts newline; needs disambiguation: `Alt+Enter` = newline, `Enter` = commit, matching Excel |
| `Esc` | Cancel current entry, restore last committed text | Not implemented |
| `F2` | Toggle "edit" vs "navigate" sub‑mode for in‑formula reference picking | Honest browser approximation: cycles caret/range mode for arrow keys |
| `F4` | Cycle reference forms (relative / mixed / absolute) on selected reference token | Needs reference‑token range detection from `EditorSyntaxSnapshot` |
| `Ctrl+Shift+U` | Expand / collapse editor height | New chrome handle bound to the same key |
| `Shift+F3` / `Ctrl+A` (in function name) | Open function argument assistant — a richer help flyout from OxFunc | OneCalc owns the surface, OxFml owns the lookup subject |
| `Ctrl+Space` | Force show completion popup | Common explorer convention; degrades on browsers that intercept the chord |
| `Ctrl+Z` / `Ctrl+Shift+Z` / `Ctrl+Y` | Undo / redo | Use the native textarea history while overlays remain best‑effort |
| `Ctrl+Alt+I` | Send selection to Inspect Formula Walk (OneCalc enhancement) | Bridge layer §8.4 |
| `Ctrl+Enter` | Re‑evaluate without leaving the editor (retain a new run, do not commit text changes) | Explorer enhancement |

The Windows desktop build is the strict‑first compatibility target; the WASM build advertises its key differences in a help affordance reachable from the editor toolbar so the user is never lied to (per spec §4 platform priority rule).

### State model (the missing state machine)

The spec explicitly leaves the edit→evaluate→display state machine for later (§§ 14, 17). For UX coherence we pin it now in three states:

- **Idle** — formula text matches the last committed text. Result reflects last evaluation. No diagnostics dirtiness.
- **Editing‑live** — text is being edited; OxFml is producing snapshots on every change, completion / signature help are armed, but the result panel still shows the *last committed* result with a "stale" indicator pill. This protects the user from mid‑keystroke flicker that mid‑typing reproof would cause.
- **Proofed‑scratch** — user pressed `Ctrl+Enter` (or paused beyond a configurable quiet interval) and the result panel updates with the *current* text's evaluation. This is *not* a commit; the formula space is still dirty.
- **Committed** — `Enter` commits, the formula space's canonical text becomes the current text, and the run is added to the formula space's lineage as an observable evidence event.

The four states drive a small status indicator next to the entry‑mode pill: ✏ Editing • ⟳ Proofing • ✓ Committed. This is the user‑visible answer to "is what I see live?"

### Visual design (warm editorial palette)

Pulled from the Figma Make Explore variant and reconciled with `ui/design_tokens/theme.rs`:

- Editor surface: white card on parchment, `2px` border in `--oc-color-card-edge`, radius `--oc-radius-panel`.
- Line number gutter: `--oc-color-muted` text on `--oc-color-panel`, right‑aligned, dim past last line.
- Active line: subtle `--oc-color-accent-soft` background.
- Syntax tokens: function = oxidized teal bold, identifier = espresso ink, number = amber brass, operator = warm rust, delimiter = warm gray, text literal = moss.
- Diagnostic underlines: error = terracotta wavy, warning = amber dotted, info = teal dotted.
- Completion popup: parchment card, `shadow-strong`, max 360px wide, item rows `28px`, icon column for kind, monospaced name + dim signature suffix, selected row in `--oc-color-accent-soft`.
- Signature ScreenTip: night background (`--oc-color-night`) with parchment text — the only dark element in the editor — to keep it visually distinct from the page chrome and to read like Excel's tooltip.
- Current help card: sidecar in right column, parchment with brass left edge, function name + one‑line semantics + collapsible "Arguments" and "See also" sections.
- Fallback mode: every overlay disappears, the textarea remains, the entry‑mode pill says "Plain mode" and a small "?" explains why.

### Explore shell layout discipline

The editor spec above only holds if the Explore *shell around* the editor cooperates. During the outside‑in shell rollout (`dno-yjk.1/2/3`) the Explore surface accumulated an editorial header above the three‑column body — an H1 "Formula Explorer" hero, a lead paragraph, and a three‑card "overview deck" — plus the editor column itself gained a panel header, a panel‑intro paragraph, an editor summary row, and an editor‑note strip before the `FormulaEditorSurface` mounts. The net effect in the running preview is that the formula textarea first appears `~430–480px` from the top of the viewport, contradicting the vertical‑rhythm promise stated in *Target experience* above. This subsection captures the layout discipline that corrects that regression and prevents recurrence.

**Source‑of‑truth citations for every decision below.**

- `docs/APP_UX_SCREEN_SPEC_EXPLORE.md` §5 — "Primary: `formula_editor_panel`, `result_panel`".
- `docs/APP_UX_SCREEN_SPEC_EXPLORE.md` §7 — "the editor should remain the dominant visible surface"; "scrolling inside the editor should not displace the result entirely"; "the user should not need to leave `Explore` just to navigate the formula".
- `docs/APP_UX_BRIEF.md` §6.2 lists the formula editor first in the perspective hierarchy.
- `docs/APP_UX_BRIEF.md` §9 — "The explorer should keep the primary result visible while support surfaces are open. The main editing and reading path should not disappear behind drawers, overlays, or modal interruptions."
- `docs/APP_UX_FORMULA_EDITOR_SPEC.md` §7 — "result visibility while editing" is a core editor requirement; both editor and result are above the fold simultaneously.
- `docs/ux_artifacts/figma_make/2026-04-04/explore/EXPLORE_MODE.md` lines 28‑103 — authoritative Figma diagram. Top bar → formula‑space context bar → **three‑column body (Editor 40% / Result 35% / Help 25%)** → status footer. "Always Visible (Default State)" enumerates editor, result, completion+help only. No overview deck. No hero title. No lead paragraph.
- `docs/ux_artifacts/figma_make/2026-04-04/explore/EXPLORE_MODE.md` lines 17‑18 — "Parse errors and validation messages appear **directly below the editor**, not in a separate inspector panel." Not in a status card *above* it either.
- `docs/ux_artifacts/figma_make/2026-04-04/explore/INFORMATION_ARCHITECTURE.md` §§210‑250 — "Formula editor (dominant surface), Result panel (prominent feedback), Completion + help (function reference)" — no fourth row.
- `docs/ux_artifacts/figma_make/2026-04-04/explore/CLARITY_IMPROVEMENTS.md` §§55‑81, 172‑188 — explicit "Formula Explorer Layout" diagrams showing editor / result / walk+help as the *entire* main canvas, with nothing above it except the context bar.
- **Use cases driving editor immediacy** from `docs/APP_UX_USE_CASES.md`:
  - **EX‑01** unexpected scalar result — on arrival, author and read diagnostics + result side‑by‑side without hunting.
  - **EX‑02** very long multi‑line formula — the editor must scroll internally; it cannot be buried below `~400px` of prose.
  - **EX‑03** completion‑led function discovery — the completion column is always visible beside the editor; both must be above the fold.
  - **EX‑04** signature / argument guidance — signature help anchors to caret; the caret must be visible on arrival.
  - **EX‑05** array‑aware exploration — array preview visible beside the editor without scrolling.
  - **EX‑09** invalid formula repair — the error span stays spatially tied to the editor.

**Regression inventory.** Walking `src/dnaonecalc-host/src/ui/components/explore_shell.rs` from the top down, everything above the first line of the textarea in the current implementation:

| # | Element | Source | Approx. height | Duplicate of | Keep? |
|---|---|---|---|---|---|
| 1 | Global app chrome + `ThemeStyleTag` | `ui/components/app_shell.rs` | 0 | — | keep |
| 2 | `ShellFrame` brand block + context bar (breadcrumb + scope strip) | `ui/components/shell_frame.rs` | ~96px | — | keep |
| 3 | Explore header H1 "Formula Explorer" + eyebrow + hero badges + Configure button | `explore_shell.rs` `ExploreShell` header | ~80px | Breadcrumb already carries space label + mode; scope strip carries profile | **remove** |
| 4 | Lead paragraph "Author the cell entry on the left…" | `.onecalc-explore-shell__lead` | ~50px | Self‑evident once the three columns are visible | **remove** |
| 5 | Overview deck (3 cards: Current formula space / Visible display / Authoring posture) | `.onecalc-explore-shell__overview-deck` | ~150px | Scenario label → rail + breadcrumb; effective display → Value Panel; diagnostics count → editor toolbar pills | **remove** |
| 6 | Editor panel header ("Editor" / "Primary authoring surface") | `ExploreEditorPanel` `.onecalc-explore-shell__panel-header` | ~50px | The column is self‑evidently the editor | **remove** |
| 7 | Editor panel intro paragraph "Keep the cell entry dominant…" | `.onecalc-explore-shell__panel-intro` | ~50px | — | **remove** |
| 8 | Editor summary row (Diagnostics / Assist / Authoring state cards) | `.onecalc-explore-shell__editor-summary-row` | ~60px | Entry‑mode / result‑class / live‑state pills already in `FormulaEditorSurface` toolbar; reused‑green‑tree already in editor footer | **remove** (fold into existing toolbar pills) |
| 9 | Editor note ("Current help target" + trace summary) | `.onecalc-explore-shell__editor-note` | ~40px | Trace → shell footer fact; help target → Assist column | **remove** (trace moves to footer; help target already in Assist) |
| 10 | Blocked reason banner (conditional) | `.onecalc-explore-shell__blocked-reason` | 0–30px | — | **keep**, but rendered *below* the editor surface, only when present |
| 11 | `FormulaEditorSurface` toolbar | `formula_editor_surface.rs` | ~60px | — | keep |
| 12 | First line of the textarea | `formula_editor_surface.rs` | — | — | keep |

Rows 3–9 total `~480px` of chrome above the textarea. After the redesign only rows 1, 2, 11 live above the textarea, `~160px` total.

**Layout discipline principles.** Every redesign decision follows one of these:

1. **The editor, result, and help are the three things always visible in the Explore body.** Nothing else belongs above them. Everything currently above them either duplicates information carried by the shell frame / scope strip / Value Panel / editor toolbar pills, or is prose that the user does not need to read to use the screen. (EXPLORE_MODE.md §§28‑103, CLARITY_IMPROVEMENTS.md §§172‑188.)
2. **Diagnostics live spatially tied to the editor, not in a status card above it.** The existing `FormulaEditorSurface` already carries an inline diagnostic layer, wavy squiggles on diagnostic spans, and a diagnostic band footer — that's the complete story. The redundant "Diagnostics" summary card above the editor goes away. (EXPLORE_MODE.md §§17‑18.)
3. **Result stays visible while editing.** The three‑column grid is the mechanism; nothing in the redesign collapses or relocates the result column. (APP_UX_FORMULA_EDITOR_SPEC.md §7, APP_UX_BRIEF.md §9.)
4. **Shell chrome is the only thing above the body.** That's the brand block, context bar with breadcrumbs, and scope strip — all of which already live in `ShellFrame` from `dno-yjk.1/2`. The Explore mode contributes *no additional* top chrome. (EXPLORE_MODE.md layout diagram.)
5. **Decoration migrates down, not up.** Content that is genuinely ambient (trace summary, reuse hint, authoring‑posture blurb) moves into the shell footer or becomes tooltip content on existing elements. Prose educational copy is deleted; users learn the layout by using it.
6. **No content loss at the view‑model layer.** The existing `ExploreEditorClusterViewModel` / `ExploreResultClusterViewModel` stay intact; only rendering changes. `scenario_label`, `trace_summary`, `host_profile_summary`, `packet_kind_summary` continue to be produced by `build_explore_view_model` — they just stop being rendered as a stack of cards above the editor, so later beads can resurface them in the right places (tooltips, footer facts) without more view‑model surgery.

**Target layout (concretely).**

```
┌─────────────┬──────────────────────────────────────────────────────────┐
│             │  ShellFrame context bar                                  │
│   Rail      │    [breadcrumb: DNA OneCalc › space › Mode]              │
│             │    [scope strip: Locale · Date · Profile · Policy · Fmt] │
│ (pinned,    │    [mode tabs]              [Configure]                  │
│  open,      ├──────────────────────────────────────────────────────────┤
│  affordances│  Explore body (three-column grid, starts immediately)    │
│  per-row)   │  ┌──────────────────┬──────────────┬─────────────────┐   │
│             │  │ Formula Editor   │ Result       │ Completion +    │   │
│             │  │   (40%)          │   (35%)      │ Help (25%)      │   │
│             │  │                  │              │                 │   │
│             │  │ toolbar pills    │ Value Panel  │ completion list │   │
│             │  │ line rail +      │ effective    │ signature help  │   │
│             │  │   textarea       │   display    │ current help    │   │
│             │  │ inline squiggles │ array prev   │                 │   │
│             │  │ diagnostic band  │              │ (or ShellDrawer │   │
│             │  │                  │              │  when Configure │   │
│             │  │                  │              │  is open)       │   │
│             │  └──────────────────┴──────────────┴─────────────────┘   │
│             ├──────────────────────────────────────────────────────────┤
│             │  ShellFrame footer (capability facts, trace summary)     │
└─────────────┴──────────────────────────────────────────────────────────┘
```

Key invariants:

- The **editor textarea** is visible without scrolling on an ordinary laptop viewport (≥ 720px tall).
- The **result hero value** (Value Panel primary block) is visible without scrolling on the same viewport.
- The **first completion item** (or the current help card) is visible without scrolling.
- The editor column can grow vertically to accommodate multi‑line formulas (EX‑02) without pushing result or help off the fold — internal scrolling inside the editor handles overflow.
- When the Configure drawer is open, the help column is replaced by the drawer in place; the editor and result columns do not resize.

**Concrete changes to `src/dnaonecalc-host/src/ui/components/explore_shell.rs`.** Rendering‑only; no view‑model shape changes.

1. **Delete the Explore `<header>` block entirely** — the H1 hero, eyebrow, hero badges, lead paragraph, and overview deck. That's ~280px of vertical chrome gone in one edit. The Configure toggle button currently embedded in the hero badges moves to the shell context bar (see §7 below).
2. **Delete the `ExploreEditorPanel` panel header, panel intro, editor summary row, and editor note.** Leave the `<section>` wrapper and mount `FormulaEditorSurface` as the immediate first child so the editor column body *is* the surface. The blocked‑reason banner stays but renders *below* the surface and only when present.
3. **Fold the three editor‑summary cards** (Diagnostics / Assist / Authoring state) into the existing `FormulaEditorSurface` toolbar pills. Diagnostics count already maps to the "Review/Clean" state chip beside the live‑state pill; completion count (Assist card) gets a new small toolbar pill; authoring‑state (incremental reuse) becomes a tooltip on the existing reuse/timing badge behind the editor settings toggle.
4. **Move `trace_summary` to the shell footer** as a conditional `ShellChromeFactViewModel` populated by `build_shell_frame_view_model`. The footer already iterates `ShellChromeFactViewModel`s, so this is a data‑only change there.
5. **Delete the "Current help target" line.** The Assist column's `ExploreHelpPanel` already shows the function help card with the same lookup key; the duplication serves nobody.
6. **Three‑column grid width** stays `2fr 1.5fr 1fr` (reading to 40 / 30 / 20 after gutters). Add `min-height: 0` on each column so the editor and result panels can internally scroll without blowing out the row.
7. **Move the Configure toggle button** from the (deleted) hero badges into the shell context bar's right‑hand action area (next to the mode tabs). This matches EXPLORE_MODE.md's context‑bar spec ("Formatting, Settings buttons"). `ShellFrame` grows an `on_configure_toggle: Option<Callback<()>>` prop; `OneCalcShellApp` wires it to `EditorCommand::ToggleConfigureDrawer` through the existing reducer entry point.
8. **Scenario / truth‑source / host‑profile labels** currently duplicated in the hero badges are already covered by the breadcrumb (space label) and scope strip (host profile); delete them from the Explore surface. No data loss.

**CSS changes to `src/dnaonecalc-host/src/ui/design_tokens/theme.rs`.**

1. Delete `.onecalc-explore-shell__header`, `.onecalc-explore-shell__header-copy`, `.onecalc-explore-shell__hero-badges`, `.onecalc-explore-shell__lead`, `.onecalc-explore-shell__overview-deck`, `.onecalc-explore-shell__overview-card` rules. Stop carrying CSS for markup that no longer exists.
2. Delete `.onecalc-explore-shell__panel-header`, `.onecalc-explore-shell__panel-intro`, `.onecalc-explore-shell__editor-summary-row`, `.onecalc-explore-shell__status-card`, `.onecalc-explore-shell__editor-note` rules for the same reason.
3. Rewrite `.onecalc-explore-shell__body` so the grid attaches flush to the context bar (no inherited padding from the deleted header), with `grid-template-columns: 2fr 1.5fr 1fr`, gutter spacing from `--oc-space-3`, and `min-height: 0` on each column so the hosted surfaces can internally scroll.
4. Update `.onecalc-explore-shell__body-column--editor / --result / --help` to be flex containers with `min-height: 0` and `overflow: hidden`, so `FormulaEditorSurface` / `ExploreResultPanel` / `ExploreHelpPanel` or `ShellDrawer` each own their own scroll rather than the whole Explore section scrolling.
5. Add `.onecalc-shell-frame__configure-action` for the new context‑bar Configure button, matching the mode‑accent styling used for the existing mode tabs.

**`ShellFrame` contract changes (minimal).**

- New prop `on_configure_toggle: Option<Callback<()>>` alongside the existing `on_mode_select` / `on_formula_space_select` / `on_new_formula_space` / `on_close_formula_space` / `on_toggle_pin_formula_space` props.
- New action button "Configure" in the context bar's right‑hand action area, dispatching the new callback.
- New optional `trace_summary: Option<String>` field on `ShellFrameViewModel`, populated by `build_shell_frame_view_model` from `active_formula_space.context.trace_summary`, rendered as a `ShellChromeFactViewModel` in the footer row. No new footer logic — the footer already loops `ShellChromeFactViewModel`s.

**Work packaging.** This is the scope for a new child bead under `dno-yjk`, provisionally **`dno-yjk.7`**, titled *"Redesign Explore screen vertical rhythm so the formula editor is immediate"*. One bead, not several: it's a single coherent rendering change across `explore_shell.rs`, `theme.rs`, `shell_frame.rs`, `shell_composition.rs`, and two test files, introduces no new reducer commands or view‑model types, and is **P1** on the visible‑UI lane because the existing `dno-yjk.1/2/3` shell work is invisible to the user while the formula textarea is below the fold. It should run before any further outside‑in shell work and is independent of the deferred editor bugs (`dno-yjk.4/5/6`).

### Configurability (spec §12)

Editor settings live behind a small gear in the editor toolbar opening a popover (not a modal):

- Bracket pair auto‑close (default on)
- Bracket pair highlight (default on)
- Completion aggressiveness (manual / on identifier / always — default "on identifier")
- Inline vs sidecar current help (default sidecar)
- Reuse / timing badge visibility (default off)
- Reduce motion / overlays (default follows `prefers-reduced-motion`)

These are formula‑space‑agnostic and persist at the workspace level so they survive across reopen.

### Implicit explorer requirements honoured

- Result remains visible on the page while editing (state‑machine "stale" pill rather than collapsing the result).
- Keyboard‑first flow: every editor action has a chord; mouse never required.
- Mode switching from Explore to Inspect preserves caret and selection so `Ctrl+Alt+I` lands the Walk on the right node (`IN-09` use case).
- The editor is honest about platform: the WASM build does not pretend to handle `F2` like desktop Excel; it advertises its substitute and offers a help link.

### Gaps and open questions for this area

- Spec §14.3 "current help card" content payload is unspecified: needs a minimal OxFunc help shape (name, summary, signatures, argument briefs). Treated below as a back‑pressure on OxFml/OxFunc, not on OneCalc.
- "Send to Inspect" depends on green‑tree node identity exposed by OxFml; if not yet stable it degrades to "Inspect this formula space at the current caret offset".
- "Quiet interval" auto‑proofing (Editing‑live → Proofed‑scratch) needs a default; recommend 600ms with the toggle in the editor settings popover.
- **Scenario label prominence after the hero header is removed** (see *Explore shell layout discipline*). The breadcrumb already carries the space label, but at smaller typography than the old H1. If the space label needs more visual weight, grow the breadcrumb space segment's typography one step rather than re‑adding a hero header.
- **Onboarding copy placement.** The current Explore lead paragraph is functionally educational. If onboarding is still desired after the redesign, host it inside the Configure drawer under a "Getting started" card rather than reserving body chrome for it. No action required for the `dno-yjk.7` bead; flagged for a future onboarding‑specific bead if it ever matters.
- **Narrow‑height viewports.** At viewports under 720px tall, the three‑column layout remains horizontally fine but editor / result / help each need internal scroll. The CSS `min-height: 0` on each column is the enabler; no responsive breakpoints required for `dno-yjk.7`.

---

## Area 2 — File / Formula Management: Opening, Creating, Saving a Case

### Source‑of‑truth references

- `docs/APP_UX_USE_CASES.md` use cases EX‑01..EX‑10 (authoring / discovery), WB‑03 / WB‑07 / WB‑10 (retain & open evidence), IN‑09 (mode‑switch with context).
- `docs/APP_UX_BRIEF.md` §§ 5, 8 — multi‑space workspace, recent / pinned, scratch vs retained.
- `docs/APP_UX_ARCHITECTURE.md` §§ 7‑10 — formula space lifecycle, retained evidence, workspace navigation.
- `docs/APP_UX_PANEL_INVENTORY.md` workspace‑navigation and scenario‑policy panel rows.
- Figma Make: all three variants share the left‑rail "Formula Spaces" model; `inspect/INFORMATION_ARCHITECTURE.md` enumerates the four ownership levels.
- Existing code: `state/types.rs` (`WorkspaceShellState`, `RetainedArtifactOpenState`, `RetainedArtifactRecord`, `FormulaSpaceState`), `state/reducer.rs` (`open_retained_artifact_from_catalog`, `import_manual_retained_artifact_into_active_formula_space`, `import_verification_bundle_report_into_workspace`), `services/verification_bundle.rs`, `services/programmatic_testing.rs`, `persistence/` (placeholder), `ui/components/shell_frame.rs` (rail rendering).

### What we are *not* building

- Not a workbook / sheet model. There is one cell entry per formula space (BRIEF §3.1 lines 56‑60).
- Not a file dialog over arbitrary disk locations as the primary affordance — the explorer model is "named formula spaces in a workspace" first, "import / export" second.
- Not a multi‑user / shared workspace.

### Conceptual model (what a "case" is)

The user works in a **workspace**. The workspace contains an ordered list of **formula spaces**. Each formula space holds:

- A name (user editable).
- The current entered cell text (raw, OneCalc‑owned).
- Editor state (caret, selection, scroll, dirty flag, entry mode).
- The latest `EditorDocument` snapshot from OxFml.
- Result + effective display.
- Scenario policy + formatting context (Area 3).
- Optional **runs**: ordered evidence events from past evaluations.
- Optional **retained artifact**: when the user has explicitly retained an evaluation as evidence (Workbench WB‑03 / WB‑07), a `RetainedArtifactRecord` is bound to the formula space.

A formula space is *scratch* until it has either a non‑empty name set by the user **or** a retained run / artifact. Scratch spaces survive the session but are visually marked and can be cleared in bulk; named or retained spaces persist across sessions.

A **case** is the persisted serialization of one formula space *or* a bundle of formula spaces that share evidence (the existing verification‑bundle path is the bulk shape). The word "case" is reserved for the persistence concept; the in‑app concept is the formula space.

### Target user flows

**A. First launch (cold workspace).**
The shell opens to Explore on a single empty *Untitled* formula space. The editor is focused. The entry‑mode pill is grey. Zero modal dialog blocks the user from typing.

**B. Create a new formula space.**
- `Ctrl+N` from anywhere, or "+" button at the top of the rail's Formula Spaces section, or Command Palette → "New formula space".
- A new space appears in the rail with name `Untitled n` and is selected.
- The active mode for the new space is Explore.

**C. Switch between formula spaces.**
- Click in the rail, or `Ctrl+1..9` to jump to the first nine open spaces, or `Ctrl+Tab` / `Ctrl+Shift+Tab` to cycle most‑recently‑used.
- Per‑space mode is preserved (the `ActiveFormulaSpaceViewState.active_mode` already carries this).
- The editor focus is restored, and caret position is restored. Inspect tree expansion and Workbench scroll positions are restored.

**D. Rename / pin / duplicate / close.**
- Right‑click on a rail row, or use the row's hover affordances, surfaces: Rename (inline), Duplicate (creates a new scratch space with the same text and scenario policy), Pin (moves to the Pinned section at the top), Close (with dirty‑guard if not retained).
- Closing the last space leaves a fresh empty *Untitled*; the workspace is never empty.

**E. Retain a run as evidence (the "save" verb in OneCalc).**
- In Explore: a "Retain run" affordance appears in the Result panel header once a successful evaluation exists. `Ctrl+S` is bound to it. A small dialog (popover, not modal page) asks for an optional name and shows a preview of what is being captured (formula text, result, scenario policy, host profile, capability floor). Confirming materializes a `RetainedArtifactRecord` and adds it to the space's lineage.
- In Workbench: the larger "Retain bundle" flow already specified takes over. Retained artifacts are visible in the Catalog cluster.
- "Retain" replaces the file‑system "Save" verb everywhere in the UI; the Help text explains the difference in one line.

**F. Open existing case / retained artifact.**
- Three entry points, exactly one Command Palette and one menu, surfaced through:
  1. Rail header "Open" disclosure → submenu: "From workspace catalog…" / "From bundle file…" / "From verification report…"
  2. `Ctrl+O` → opens the catalog picker, which is a sheet (not modal) listing every `RetainedArtifactRecord` in the workspace catalog.
  3. Drag‑drop a `.bundle.json` or verification XML onto the shell → routes to `import_verification_bundle_report_into_workspace` (already exists in `state/reducer.rs`).
- Each catalog row shows: case id, formula space id, formula snippet, **three verdict badges** side‑by‑side (`value_match` / `display_match` / `replay_equivalent`, each as 🟢/🟡/🔴/⬜ where ⬜ means "not observable — comparison lane was `OxfmlOnly` or `ExcelObservationBlocked`"), the dominant `mismatch_kind` if any, the comparison lane pill (`OxfmlOnly` / `OxfmlAndExcel` / `ExcelObservationBlocked`), retained timestamp, and a one‑line discrepancy summary from `OxReplayExplainRecord.summary` if present.
- Rows are filterable by verdict combination ("show me only display mismatches", "only blocked observations", "only fully matched"), by comparison lane, and by `mismatch_kind` family.
- Opening a catalog entry calls the existing `open_retained_artifact_from_catalog` reducer; the resulting formula space is added to the rail (or focused if already open) and its mode is whatever the artifact's `open_mode_hint` says — Inspect when all verdicts pass, Workbench when any fails or observation was blocked.
- The same three‑badge strip appears on the rail row for each retained formula space so the user can scan the workspace state at a glance without opening the catalog.

**G. Recents and pinned.**
- The rail keeps the existing three sections from the Figma direction: Pinned, Recent (most‑recently‑used, capped at ~10), Open. A space can live in multiple sections — Open + Pinned, or Open + Recent — and is rendered once.
- Recents survive across sessions even if the space itself was closed, so reopening is cheap.

**H. Workspace persistence.**
The persistence layer (`src/dnaonecalc-host/src/persistence/`, currently empty) gains a workspace‑level on‑disk format that holds: ordered open spaces, pinned ids, recent ids, every retained artifact reference, and editor settings. Format is JSON with a schema version. Scratch‑only spaces are persisted with a `scratch: true` marker so they can be optionally cleared on launch.

**I. Command palette.**
- `Ctrl+Shift+P` opens a centred command palette with fuzzy filter and: New, Open…, Open recent, Switch to formula space…, Rename current, Duplicate current, Pin current, Close current, Retain run, Switch mode (Explore / Inspect / Workbench), Open settings.
- The command palette is the single discoverability surface for keyboard chords. Every chord above appears in it with its binding visible on the right of the row.
- This is also where future "Insert function…", "Insert reference…" and similar grow without adding rail clutter.

### Visual / chrome integration

- The left rail keeps the existing structure (`shell_frame.rs`) but gains:
  - A persistent "+" button in the "Formula Spaces" section header.
  - A "Open" disclosure button next to it.
  - Hover affordances per row: rename pencil, duplicate, pin, close.
  - The active row gets a `4px` left border in the active mode's accent colour (teal for Explore, moss for Inspect, terracotta for Workbench), per the Figma `CLARITY_IMPROVEMENTS.md` direction.
  - A dirty marker (a small dot) in the row when the space has uncommitted edits.
- The top context bar adds breadcrumbs: `Workspace › Formula space name › Mode`, where each segment is clickable.
- A small toast in the bottom of the page surfaces non‑modal feedback for retain / open / rename actions.

### Implicit explorer requirements honoured

- The user is never blocked behind a "Welcome / New / Open" modal; the app always opens to a usable empty formula space.
- "Save" maps to "Retain", aligning the verb with the evidence model rather than the Excel file model.
- The same physical formula space carries Explore, Inspect and Workbench state, so mode switches feel free.
- Cases imported from upstream verification runs (the existing programmatic / bundle path) can land in the same rail and be navigated identically to user‑authored ones.

### Gaps and back‑pressure

- The persistence layer is currently a placeholder. This area depends on giving it a concrete on‑disk format. Recommendation: ship workspace JSON v1 with explicit scratch/retained marking and forward‑compat versioning fields.
- `RetainedArtifactRecord` already carries `bundle_report_path`, `case_output_dir`, `xml_extraction` — confirm those are sufficient to round‑trip a formula space in workspace JSON v1, or add a `formula_space_snapshot` slot.
- Drag‑and‑drop import requires browser DOM listeners on the shell root; deferable to phase 2 with a visible "Import…" button as the first‑pass affordance.

---

## Area 3 — Formatting, Conditional Formatting, and Host / Caller Configuration

### Source‑of‑truth references

- `docs/APP_UX_BRIEF.md` § 9 (host truth, scenario policy as first‑class), § 10 (formatting and conditional formatting as `needs_clarification`).
- `docs/APP_UX_ARCHITECTURE.md` § 9 — scenario policy is product‑level, not hidden settings.
- `docs/APP_UX_PANEL_INVENTORY.md` `scenario_policy_panel`, `host_truth_panel`, `environment_truth_panel`, formatting / conditional formatting rows.
- `docs/APP_UX_HOST_STATE_SLICING.md` for the slicing rules between workspace and formula‑space scope.
- Figma Make Explore — formatting drawer, scenario flags drawer, scenario policy dropdown in context bar.
- Existing code: `state/types.rs` `FormulaSpaceContextState`, `CapabilityAndEnvironmentState`, `services/programmatic_testing.rs` `ProgrammaticHostProfile` and `ProgrammaticVerificationConfig`, `services/verification_bundle.rs`, the host profile rendering in `shell_frame.rs` and `shell_composition.rs`.
- Sibling repo investigations (see Appendix A): `C:/Work/DnaCalc/OxFml/crates/oxfml_core/src/{consumer/editor/types.rs, host/mod.rs, interface/mod.rs, publication/mod.rs, seam/mod.rs}`, `C:/Work/DnaCalc/OxFunc/crates/oxfunc_core/src/locale_format.rs`, `C:/Work/DnaCalc/OxFml/docs/spec/formatting/EXCEL_FORMATTING_HIERARCHY_AND_VISIBILITY_MODEL.md`, `C:/Work/DnaCalc/OxFml/docs/spec/OXFML_DNA_ONECALC_HOST_POLICY_BASELINE.md`, `C:/Work/DnaCalc/OxFml/docs/worksets/W030_*.md`, `W039_*.md`, `C:/Work/DnaCalc/OxXlPlay/src/oxxlplay-abstractions/src/lib.rs`, `C:/Work/DnaCalc/OxXlPlay/docs/spec/OXXLPLAY_ONECALC_OBSERVATION_CONSUMER_CONTRACT.md`.
- Excel's `Format Cells` dialog (Ctrl+1) and its `Conditional Formatting Rules Manager`, treated as the *full feature surface* DnaOneCalc must mirror in UI even where the engines do not yet realise the behaviour.

### Working principle for this area

The user is right that the only honest way to land this is to **build the entire Excel‑faithful Format Cells / CF / Workbook Options surface in OneCalc up front**, and to **mark every control whose semantics depend on engine work that does not yet exist with a visible `<NOT IMPLEMENTED>` badge**. We do not shim missing engine work in OneCalc; instead we render the control, disable the inputs, expose the badge, and capture the seam requirement in Appendix B so the engine repos can close the gap in a follow‑up pass. This makes the seam visible to every user of the app and to every engineer on every repo, instead of burying it.

The investigation behind this principle is summarised in Appendix A. Headlines:

- **OxFml has PRESENT support** for: locale (only `EnUs` and `CurrentExcelHost`), date system (`System1900` / `System1904`), `EditorPlanOptions` carries `locale_profile` + `date_system` + `format_profile` + `library_context_snapshot`, `SingleFormulaHost` carries defined names, table catalog, caller cell, volatile values, and `TypedContextQueryBundle` carries host‑info / RTD providers and locale context, with `VerificationPublicationSurface.effective_display_text` being the canonical effective display string.
- **OxFml has PARTIAL support** for: number format strings (TEXT/FIXED/DOLLAR through `FormatCodeEngine`, but no full Excel format‑grammar validation), conditional formatting (only operator and expression rules, no colour scales / data bars / icon sets, formula visibility unresolved), R1C1 reference style (parser recognises but no public toggle).
- **OxFml has ABSENT support** for: full cell font (family / size / weight / style / underline), borders, alignment, protection, style‑XF resolution, iterative calculation, data validation rules in publication.
- **OxXlPlay has PRESENT observation** for: `effective_display_text`, `number_format_code`, `style_id`, `font_color`, `fill_color`, `conditional_formatting_rules`, `conditional_formatting_effective_style` — all `derived` from SpreadsheetML, read‑only, no application.
- **OxXlPlay has ABSENT support** for: locale / date system / R1C1 / calc options configuration, full font/border/alignment/protection capture, non‑expression CF rule re‑evaluation.
- **OxFunc has PRESENT** locale and format scaffolding via `LocaleProfileId`, `FormatProfile`, `WorkbookDateSystem`, and the `FormatCodeEngine` trait wired through TEXT / FIXED / DOLLAR — but only two locale profiles are hard‑coded.

### Scoping rule (HOST_STATE_SLICING)

- **Workspace‑level**: host profile, capability floor, locale, date system, OxFml / OxFunc / OxXlPlay versions, default scenario policy, default reference style, editor settings, persistence target. Edited in a *workspace settings* page reached from the rail footer.
- **Formula‑space‑level**: scenario policy override, full cell formatting (number / alignment / font / border / fill / protection), conditional formatting rules, defined names, table catalog, host‑query bindings, calc options override. Edited in a *Configure* drawer reached from the Explore context bar.
- **Run‑level**: nothing user‑editable; the run freezes the workspace + formula‑space context as an evidence envelope.

A small *scope strip* in the formula‑space context bar makes the active scoping visible at a glance: `Locale: en-US · Date: 1900 · Profile: H1-Standard · Policy: Deterministic · Format: #,##0.00`. Clicking any segment opens the relevant drawer or page focused on that field. Segments that are tied to a `<NOT IMPLEMENTED>` engine surface render with a dotted underline so the user knows their effect is currently UI‑only; hovering shows the seam id (`SEAM-OXFML-FMT-01`, etc., from Appendix B).

### The "Configure" drawer (per formula space) — Excel Format Cells parity surface

Activated from a `Configure` button in the Explore context bar (also `Ctrl+1`, mirroring Excel). The drawer slides in from the right and replaces the Assist column. The editor and result remain visible so the user sees the live effect of every change.

The drawer is a tabbed surface with the same six tabs Excel exposes in `Format Cells`, plus three OneCalc‑specific tabs for the explorer model (Scenario Policy, Host Bindings, Calc Options). Every control has a status indicator from the legend below.

**Status legend (rendered as small badges next to each control):**

- 🟢 **LIVE** — the control writes through to OxFml and the engine evaluates / displays it correctly.
- 🟡 **PARTIAL** — the control writes through, but the engine only honours a subset of values; out‑of‑subset values are accepted by the UI and trigger a banner explaining the limitation.
- 🔴 **`<NOT IMPLEMENTED>`** — the control is rendered, accepts input, persists in the formula‑space JSON, is round‑tripped into the verification bundle, but the engines ignore it. The result panel shows a "format preview is host‑painted" footnote when at least one such control is active.

#### Tab 1: Number

Mirrors Excel's Number tab exactly. Left column is the category list, right column is the category‑specific control panel.

| Category | Controls | Status |
|---|---|---|
| General | (no controls — explanatory text) | 🟢 LIVE |
| Number | Decimal places (stepper 0–30); Use 1000 separator (toggle); Negative numbers (list of styles: `1234.10` / `-1234.10` / `(1234.10)` / `[Red]-1234.10` / `[Red](1234.10)`) | 🟡 PARTIAL — generated format code passes through to TEXT/FIXED, but `[Red]` colour token is NOT honoured by `FormatCodeEngine` (`SEAM-OXFUNC-FMT-RED`) |
| Currency | Decimal places; Symbol picker (built‑in list of ~30 currency symbols); Negative numbers (list of styles) | 🟡 PARTIAL — symbol is honoured for the locale's default currency; non‑default symbols `<NOT IMPLEMENTED>` (`SEAM-OXFUNC-FMT-CURRENCY`) |
| Accounting | Decimal places; Symbol picker; (no negative styles, fixed accounting layout) | 🔴 `<NOT IMPLEMENTED>` — `SEAM-OXFUNC-FMT-ACCOUNTING` |
| Date | Locale picker (mirrors workspace locale by default); Date format list (Excel's standard set: `*3/14/2012`, `*Wednesday, March 14, 2012`, `3/14`, `3/14/12`, `03/14/12`, `14-Mar`, `14-Mar-12`, `Mar-12`, `March-12`, `M`, `March 14, 2012`, `3/14/12 1:30 PM`, `3/14/12 13:30`, `2012-03-14`); Asterisked formats follow OS regional settings | 🟡 PARTIAL — basic d/m/y tokens render through `FormatCodeEngine`, locale date format expansion limited to the two hard‑coded `LocaleProfileId` profiles (`SEAM-OXFUNC-LOCALE-EXPAND`) |
| Time | Locale picker; Time format list (`13:30:55`, `1:30 PM`, `13:30`, `1:30:55 PM`, `30:55.2`, `[h]:mm:ss`, etc.); 12 vs 24 hour | 🟡 PARTIAL — h/m/s tokens render, `[h]` elapsed token `<NOT IMPLEMENTED>` (`SEAM-OXFUNC-FMT-ELAPSED`) |
| Percentage | Decimal places | 🟢 LIVE |
| Fraction | Type list (`Up to one digit`, `Up to two digits`, `Up to three digits`, `As halves`, `As quarters`, `As eighths`, `As sixteenths`, `As tenths`, `As hundredths`) | 🔴 `<NOT IMPLEMENTED>` — `SEAM-OXFUNC-FMT-FRACTION` |
| Scientific | Decimal places | 🟡 PARTIAL — only fixed exponent width currently rendered (`SEAM-OXFUNC-FMT-SCIENTIFIC`) |
| Text | (no controls — explanatory text) | 🟢 LIVE — apostrophe‑forced string entry path covered by the editor's entry‑mode pill |
| Special | Locale picker; Type list (`Zip Code`, `Zip Code +4`, `Phone Number`, `Social Security Number` for en‑US; locale equivalents for others) | 🔴 `<NOT IMPLEMENTED>` — `SEAM-OXFUNC-FMT-SPECIAL` |
| Custom | Format code text input with token autocomplete; Live sample line (uses the current result value when present, otherwise `1234.5`); Saved custom format list (workspace‑scoped) | 🟡 PARTIAL — code is sent verbatim to `FormatCodeEngine`; tokens outside the supported subset are flagged with an inline diagnostic from OxFunc when `SEAM-OXFUNC-FMT-GRAMMAR-VALIDATION` lands |

The active format code (always the canonical text Excel would build from the controls) is displayed in a read‑only strip at the bottom of the tab so the user understands which control wrote which token.

**Pre‑fill from engine hints.** When OxFml returns an `ExtendedValue::ValueWithPresentation { value, hint }` on the current formula space, the Number tab opens on the category implied by `hint.number_format`: `NumberFormatHint::Currency` → Currency, `DateLike` → Date, `Percentage` → Percentage, `Scientific` → Scientific, `Fraction` → Fraction, `Custom` → Custom, `General` → Number. Likewise `hint.style == Some(CellStyleHint::Hyperlink)` surfaces a small "Hyperlink style" pill in the tab header so the user is aware the engine classified the result that way even though OneCalc's font tab is `<NOT IMPLEMENTED>`.

#### Tab 2: Alignment

Mirrors Excel's Alignment tab. **Entire tab is `<NOT IMPLEMENTED>` against today's engines** — both OxFml and OxXlPlay are absent here. The controls render and persist for round‑trip into the verification bundle.

| Group | Controls | Status |
|---|---|---|
| Text alignment | Horizontal (General / Left (Indent) / Center / Right (Indent) / Fill / Justify / Center Across Selection / Distributed (Indent)); Vertical (Top / Center / Bottom / Justify / Distributed); Indent stepper; Justify Distributed toggle | 🔴 `<NOT IMPLEMENTED>` — `SEAM-OXFML-ALIGNMENT-MODEL` |
| Text control | Wrap text; Shrink to fit; Merge cells (disabled — single‑cell host) | 🔴 `<NOT IMPLEMENTED>` — `SEAM-OXFML-ALIGNMENT-MODEL` |
| Right‑to‑left | Text direction (Context / Left‑to‑Right / Right‑to‑Left) | 🔴 `<NOT IMPLEMENTED>` — `SEAM-OXFML-ALIGNMENT-MODEL` |
| Orientation | Degree picker (`-90` to `+90`), Vertical text toggle | 🔴 `<NOT IMPLEMENTED>` — `SEAM-OXFML-ALIGNMENT-MODEL` |

A persistent banner at the top of the tab reads: *"Cell alignment is not yet implemented in the OxFml / OxXlPlay engines. The values you set here are persisted and will be honoured automatically once the engines support them. See seam SEAM-OXFML-ALIGNMENT-MODEL."*

#### Tab 3: Font

Mirrors Excel's Font tab. **Most controls are `<NOT IMPLEMENTED>`.** Font colour is the only PARTIAL surface because OxXlPlay observes it from SpreadsheetML.

| Group | Controls | Status |
|---|---|---|
| Font family | Searchable picker (system fonts) | 🔴 `<NOT IMPLEMENTED>` — `SEAM-OXFML-FONT-MODEL` |
| Font style | Regular / Italic / Bold / Bold Italic | 🔴 `<NOT IMPLEMENTED>` — `SEAM-OXFML-FONT-MODEL` |
| Size | Stepper + presets (8, 9, 10, 11, 12, 14, 16, 18, 20, 22, 24, 26, 28, 36, 48, 72) | 🔴 `<NOT IMPLEMENTED>` — `SEAM-OXFML-FONT-MODEL` |
| Underline | None / Single / Double / Single Accounting / Double Accounting | 🔴 `<NOT IMPLEMENTED>` — `SEAM-OXFML-FONT-MODEL` |
| Color | Theme palette + standard colours + "More Colours…" picker | 🟡 PARTIAL — value persists; OxFml only honours the colour as a CF override today (`SEAM-OXFML-FONT-COLOR`); OxXlPlay observes it as `derived` |
| Effects | Strikethrough; Superscript; Subscript | 🔴 `<NOT IMPLEMENTED>` — `SEAM-OXFML-FONT-MODEL` |
| Preview | Live sample text using the current font configuration | UI‑only |

#### Tab 4: Border

Mirrors Excel's Border tab. **Entire tab is `<NOT IMPLEMENTED>`.**

| Group | Controls | Status |
|---|---|---|
| Line style | None / Hair / Dotted / Dashed / Thin / Medium / Thick / Double / various dash combos | 🔴 `<NOT IMPLEMENTED>` — `SEAM-OXFML-BORDER-MODEL` |
| Color | Colour picker | 🔴 `<NOT IMPLEMENTED>` — `SEAM-OXFML-BORDER-MODEL` |
| Presets | None / Outline / Inside (disabled — single cell) | 🔴 `<NOT IMPLEMENTED>` — `SEAM-OXFML-BORDER-MODEL` |
| Border buttons | Top / Bottom / Left / Right / Diagonal Up / Diagonal Down (each toggles application of current line style + colour) | 🔴 `<NOT IMPLEMENTED>` — `SEAM-OXFML-BORDER-MODEL` |
| Preview | Live sample showing the current border configuration | UI‑only |

#### Tab 5: Fill

Mirrors Excel's Fill tab.

| Group | Controls | Status |
|---|---|---|
| Background colour | Theme + standard + "More Colours…" + No Fill | 🟡 PARTIAL — round‑trips to / from OxXlPlay observation (`SEAM-OXFML-FILL-COLOR`) |
| Fill effects | "Fill Effects…" button opens a sub‑popover with Gradient (one / two colour / preset gradients) and Pattern (style + colour) tabs | 🔴 `<NOT IMPLEMENTED>` — `SEAM-OXFML-FILL-EFFECTS` |
| Pattern colour | Colour picker | 🔴 `<NOT IMPLEMENTED>` — `SEAM-OXFML-FILL-EFFECTS` |
| Pattern style | Picker with the 18 Excel pattern styles | 🔴 `<NOT IMPLEMENTED>` — `SEAM-OXFML-FILL-EFFECTS` |
| Sample | Live preview swatch | UI‑only |

#### Tab 6: Protection

Mirrors Excel's Protection tab. Both controls are `<NOT IMPLEMENTED>` — OneCalc has no notion of sheet protection, but the controls are present so the persisted state is faithful when imported from a real workbook.

| Group | Controls | Status |
|---|---|---|
| Locked | Toggle | 🔴 `<NOT IMPLEMENTED>` — `SEAM-OXFML-PROTECTION-MODEL` |
| Hidden (formula) | Toggle | 🔴 `<NOT IMPLEMENTED>` — `SEAM-OXFML-PROTECTION-MODEL` |
| Explanatory text | Excel's "Locking cells or hiding formulas has no effect until you protect the worksheet…" — re‑written for the OneCalc model: *"OneCalc has no sheet‑protection model. These values round‑trip into the verification bundle so they can be honoured by host Excel during replay."* | UI‑only |

#### Tab 7: Conditional Formatting (OneCalc rules manager)

A faithful port of Excel's `Conditional Formatting Rules Manager`. The single‑cell host means there is only one selection scope (the formula space's cell), but the rule grammar is the same as Excel's.

**Top toolbar:**

- `New Rule…` button opens a `New Formatting Rule` modal sheet (described below).
- `Edit Rule…` button (disabled if no rule selected).
- `Delete Rule` button.
- `↑ / ↓` buttons to reorder.

**Rule list (table):**

| Column | Notes |
|---|---|
| Rule (description) | E.g. "Cell Value > 100", "Top 10%", "Formula: =MOD(A1,2)=0" |
| Format (preview swatch) | Shows the format the rule would apply |
| Applies To | "This formula space" (read‑only — single cell host) |
| Stop If True | Toggle |
| Status | 🟢 / 🟡 / 🔴 badge per rule kind |

**`New Formatting Rule` sheet** mirrors Excel's `New Formatting Rule` dialog exactly:

| Rule type | Status | Seam |
|---|---|---|
| Format all cells based on their values — 2‑Color Scale | 🔴 `<NOT IMPLEMENTED>` | `SEAM-OXFML-CF-COLORSCALE` |
| Format all cells based on their values — 3‑Color Scale | 🔴 `<NOT IMPLEMENTED>` | `SEAM-OXFML-CF-COLORSCALE` |
| Format all cells based on their values — Data Bar | 🔴 `<NOT IMPLEMENTED>` | `SEAM-OXFML-CF-DATABAR` |
| Format all cells based on their values — Icon Set | 🔴 `<NOT IMPLEMENTED>` | `SEAM-OXFML-CF-ICONSET` |
| Format only cells that contain — Cell Value (operator + value) | 🟢 LIVE — operator rules are honoured by OxFml `evaluate_operator_rule()` |
| Format only cells that contain — Specific Text | 🟡 PARTIAL — accepted but engine evaluation depends on OxFml W039 closure (`SEAM-OXFML-CF-TEXT`) |
| Format only cells that contain — Dates Occurring | 🟡 PARTIAL — `SEAM-OXFML-CF-DATES` |
| Format only cells that contain — Blanks / No Blanks | 🟡 PARTIAL — `SEAM-OXFML-CF-BLANKS` |
| Format only cells that contain — Errors / No Errors | 🟡 PARTIAL — `SEAM-OXFML-CF-ERRORS` |
| Format only top or bottom ranked values | 🔴 `<NOT IMPLEMENTED>` (single‑cell scope makes it semantically dubious; we still render the rule for round‑trip) — `SEAM-OXFML-CF-RANK` |
| Format only values that are above or below average | 🔴 `<NOT IMPLEMENTED>` — `SEAM-OXFML-CF-AVERAGE` |
| Format only unique or duplicate values | 🔴 `<NOT IMPLEMENTED>` — `SEAM-OXFML-CF-UNIQUE` |
| Use a formula to determine which cells to format | 🟢 LIVE — expression rules are honoured by OxFml `evaluate_expression_rule()` |

The format target of every rule type uses the same `Format…` button that opens an embedded `Format Cells` dialog with the same Number / Alignment / Font / Border / Fill tabs as Tab 1‑5, with the same status badges.

**Live preview:** beside the rule list, a swatch shows what the current result value would look like under the rule set. When a rule's output depends on a `<NOT IMPLEMENTED>` engine surface, the swatch is rendered with a dashed border and a small "host‑painted" footnote.

#### Tab 8: Scenario Policy (OneCalc‑specific)

The single‑choice scenario policy selector that already lives in the context bar dropdown is mirrored here with full descriptions: Deterministic, Real‑time, Real Random. Plus a list of scenario flags as toggles with a one‑line explanation each:

| Flag | Status |
|---|---|
| Volatile functions allowed (NOW, TODAY, RAND, RANDBETWEEN re‑evaluate live) | 🟢 LIVE — wired through `SingleFormulaHost.now_serial` and `random_provider` |
| Freeze intermediate arrays | 🔴 `<NOT IMPLEMENTED>` — `SEAM-OXFML-EVAL-FREEZE` |
| Result caching | 🔴 `<NOT IMPLEMENTED>` — `SEAM-OXFML-EVAL-CACHE` |
| Strict evaluation | 🔴 `<NOT IMPLEMENTED>` — `SEAM-OXFML-EVAL-STRICT` |
| Use 1900 vs 1904 date system | 🟢 LIVE — wired through `EditorPlanOptions.date_system` |
| Reference style (A1 / R1C1) | 🟡 PARTIAL — parser admits R1C1 but no public toggle yet (`SEAM-OXFML-R1C1-PUBLIC`) |
| Iterative calculation (max iterations + max change) | 🔴 `<NOT IMPLEMENTED>` — `SEAM-OXFML-CALC-ITERATIVE` (single‑formula host model is fundamentally not iterative; this is for round‑trip only) |

#### Tab 9: Host Bindings (OneCalc‑specific)

The user can edit the host context that OxFml's `SingleFormulaHost` consumes for the active formula space. Each section corresponds directly to a `SingleFormulaHost` field, so this tab is the user‑facing surface of the existing host model.

All value inputs in this tab are **typed `EvalValue` inputs** — the editor is not a string field, it is a small form with a type picker (Number / Text / Logical / Error / Array / Reference / Lambda / RichValue) and type‑appropriate sub‑controls. Text inputs enforce the `ExcelText` 32,767 UTF‑16 code unit limit and show a live count. Errors use a `WorksheetErrorCode` dropdown. Arrays use a spreadsheet‑style grid with `ArrayShape` constraints and `ArrayCellValue` per cell. References use a `ReferenceKind` dropdown (`A1` / `Area` / `MultiArea` / `ThreeD` / `Structured` / `SpillAnchor`) with the appropriate target parser; multi‑area references accept parenthesis‑delimited CSV. Rich values use a schema‑driven form built from the `RichValueType { type_name, required_keys, key_flags }` — each key gets an input, keys marked `ExcludeFromCalcComparison` are marked accordingly, and nested `RichArray` / `RichValueData::RichValue` are rendered recursively.

| Section | Controls | Status |
|---|---|---|
| Caller cell | Row stepper, column stepper or A1 input (default A1) | 🟢 LIVE — wired to `caller_row` / `caller_col` |
| Defined names | Editable table: Name, typed `EvalValue` value via the type‑picker form above | 🟢 LIVE — wired to `defined_names: BTreeMap<String, DefinedNameBinding>` |
| Direct cell bindings | Editable table: A1 reference, typed `EvalValue` via the type‑picker form | 🟢 LIVE — wired to `cell_values: BTreeMap<String, EvalValue>` |
| Tables | Editable list: Name, columns (each with typed `ArrayShape`), range — opens a sub‑editor for table structure | 🟡 PARTIAL — `table_catalog` model exists in OxFml but the OneCalc UI grammar for editing it has gaps; document `SEAM-ONECALC-TABLE-EDITOR` (UI‑side only) |
| Enclosing table | Picker (None / one of the tables defined above) | 🟢 LIVE |
| Volatile values | NOW serial number (datetime picker) + random provider | 🟢 LIVE — wired to `now_serial` / `random_provider` |
| Host info provider | A small editable form for INFO/CELL function inputs (directory, OS version, file name, etc.) | 🟡 PARTIAL — `HostInfoProvider` trait exists; OneCalc does not yet implement it (`SEAM-ONECALC-HOST-INFO`) |
| RTD provider | A list of (server, topic, typed `EvalValue`) entries with auto‑refresh interval | 🔴 `<NOT IMPLEMENTED>` (`SEAM-ONECALC-RTD`) |
| Value boundaries | Diagnostic: for every typed value the user enters, the UI evaluates `ValueBoundary::CellContent.allows(tag)` or the appropriate boundary for that slot and surfaces a warning if the value is structurally invalid at the slot (e.g. `MissingArg` in a cell binding) | 🟢 LIVE — `ValueBoundary::allows()` is already in `oxfunc_value_types` |

#### Tab 10: Calc Options (OneCalc‑specific)

Mirrors Excel's `File → Options → Formulas → Calculation options` and the workbook calculation properties in `wb.xml`.

| Control | Status |
|---|---|
| Workbook Calculation: Automatic / Automatic except for data tables / Manual | 🔴 `<NOT IMPLEMENTED>` — `SEAM-OXFML-CALC-MODE` (single‑formula host has no concept) |
| Enable iterative calculation | 🔴 `<NOT IMPLEMENTED>` — `SEAM-OXFML-CALC-ITERATIVE` |
| Maximum iterations | 🔴 `<NOT IMPLEMENTED>` — `SEAM-OXFML-CALC-ITERATIVE` |
| Maximum change | 🔴 `<NOT IMPLEMENTED>` — `SEAM-OXFML-CALC-ITERATIVE` |
| R1C1 reference style | 🟡 PARTIAL — `SEAM-OXFML-R1C1-PUBLIC` |
| Formula AutoComplete | 🟢 LIVE — toggles editor completion (Area 1) |
| Use table names in formulas | 🟢 LIVE — depends on Host Bindings table catalog |
| Use 1904 date system | 🟢 LIVE — `SEAM` already closed |
| Set precision as displayed | 🔴 `<NOT IMPLEMENTED>` — `SEAM-OXFML-PRECISION-AS-DISPLAYED` |
| Save external link values | 🔴 `<NOT IMPLEMENTED>` — `SEAM-OXFML-EXTERNAL-LINKS` |

### Workspace settings page

Reachable from the rail footer's "Workspace" disclosure. Renders as a full page replacing the Explore main column (not a modal). Sections:

- **Locale and date system** — Locale picker (lists every `LocaleProfileId` known to OxFunc; flags non‑hard‑coded entries with `<NOT IMPLEMENTED>`); Date system radio (1900 / 1904); Decimal / thousands / list separators read‑only preview.
- **Default host profile** — Picker over `ProgrammaticHostProfile` ids; read‑only summary of the picked profile (function set, array support, requires Excel observation, capability floor, OxFml / OxFunc / OxXlPlay versions).
- **Default scenario policy** — Mirrors the formula‑space drawer Tab 8 control with workspace‑level scope.
- **Default reference style** — A1 / R1C1 (with `SEAM-OXFML-R1C1-PUBLIC` badge).
- **Editor settings** — Lifted from the editor gear popover so the user has one canonical place too: bracket auto‑close, bracket highlight, completion aggressiveness, current help placement, reuse/timing badge visibility, reduce motion.
- **Storage / persistence** — Workspace JSON path, "Clear scratch spaces" action, retained artifact catalog management, recents list management.
- **Engine versions and capability snapshot** — Read‑only display lifted from the existing `CapabilityAndEnvironmentState` (currently empty — `SEAM-ONECALC-CAPABILITY-SNAPSHOT`): OxFml version, OxFunc catalog identity, OxXlPlay version, capability floor, blocked capabilities, mode availability per profile.
- **Seam status board** — A live list of every `SEAM-*` id active in the current workspace, grouped by repo (OxFml / OxFunc / OxXlPlay / OneCalc), with the count of formula spaces affected and the current implementation status. This is the single place an engineer can scan to see the pending engine work.

### How edits flow to OxFml / OxXlPlay

OneCalc owns all the UI entry; OxFml owns all the interpretation. The shape of the contract OneCalc passes is:

- A `FormulaSpaceContext` value object that bundles every field above. It is composed of:
  - `EditorPlanOptions { oxfunc_catalog_identity, locale_profile, date_system, format_profile, library_context_snapshot }` — already exists in OxFml.
  - `SingleFormulaHost { caller_row, caller_col, defined_names, cell_values, table_catalog, enclosing_table_ref, caller_table_region, now_serial, random_provider }` — already exists in OxFml.
  - `TypedContextQueryBundle { host_info, rtd_provider, locale_ctx, now_serial, random_provider }` — already exists in OxFml.
  - `CellFormatPayload` (NEW) — number format code, font, fill, border, alignment, protection, CF rules. Most of this is `<NOT IMPLEMENTED>` on the engine side and round‑trips through `SEAM-OXFML-FORMAT-PAYLOAD`.
  - `CalcOptionsPayload` (NEW) — iterative, calc mode, precision as displayed, etc. Almost entirely `<NOT IMPLEMENTED>` and tracked by `SEAM-OXFML-CALC-OPTIONS`.
- This value object is included in every `FormulaEditRequest` and every replay request so OxFml's `EditorDocument`, `EditorSyntaxSnapshot`, and `VerificationPublicationSurface` are produced under it.
- OxFml is the only place "effective display" is computed; OneCalc renders the `effective_display_text` OxFml hands back through `VerificationPublicationSurface`.
- For OxXlPlay verification, the same `FormulaSpaceContext` is serialized into the verification‑bundle XML (extending `services/verification_bundle.rs`). OxXlPlay reads the parts it understands (currently: nothing user‑editable; only observation), and the rest survives the round‑trip for engine consumption later.
- Conditional formatting evaluation: OneCalc never paints a CF effect locally **except** when explicitly labelled as "host‑painted preview" — and even then only for the rule kinds OxFml's W039 / W036 worksets have not yet closed. The user always sees the host‑painted footnote.

### Visual treatment

- Drawer follows the parchment / amber / moss palette already in `theme.rs`.
- The status badges (🟢 LIVE / 🟡 PARTIAL / 🔴 `<NOT IMPLEMENTED>`) use the theme's success / warning / warm tokens respectively, with a small text label so the meaning is unambiguous to colour‑blind users.
- Active edits show an inline dirty marker in the drawer header until the next successful proof.
- The result panel grows a small "Effective display" subline directly under the hero result that updates live as the format string changes (sourced from `effective_display_text`).
- A small "Reset all formatting" affordance at the bottom of each tab reverts that tab to the workspace defaults.
- Every `<NOT IMPLEMENTED>` control is fully editable but a small `i` icon on the badge opens a popover with: the seam id, the engine repo it belongs to, a one‑line description of the missing engine work, and a link to the relevant docs (e.g. `OxFml/docs/spec/formatting/EXCEL_FORMATTING_HIERARCHY_AND_VISIBILITY_MODEL.md` § 19‑33).

### Implicit explorer requirements honoured

- Format and scenario context are *interactive*: changing a value updates the result without leaving the editor, even when the engine ignores the value (the UI is honest about that with the host‑painted footnote).
- Configuration is honest about scope: the user always sees whether they are editing workspace defaults or formula‑space overrides.
- The explorer never silently inherits a host profile; the active profile is always visible in the context bar.
- "Effective display" is sourced from OxFml, not invented locally, preserving the OxFml truth contract.
- Every gap in engine support is visible to the user, traceable to a seam id, and recorded on the seam status board.

---

## Area 4 — Value Representation and Excel Parity Surfaces (cross‑cutting, Inspect + Workbench)

This area covers the UI surfaces that the `oxfunc_value_types` split and the richer comparison pipeline now make possible. It is cross‑cutting because the new data lights up in Inspect (where the user wants to understand a value in depth) and Workbench (where the user wants to debug an Excel parity gap). Area 4 does not introduce new modes; it redefines the content of the existing ones.

### 4.1 Inspect mode — the Value Panel

The existing Inspect walk tree and summary cards stay. A new **Value Panel** joins them as the right‑column companion (or drawer on narrow widths), always showing the currently selected node's value or — when no node is selected — the root formula result. The panel is fed directly from `ExtendedValue` and `VerificationPublicationSurface.published_value` / `presentation_hint` / `effective_display_text`.

Layout:

- **Header**: the `EvalValue` variant name as a large label (`Number` / `Text` / `Logical` / `Error` / `Array` / `Reference` / `Lambda` / `RichValue`), with the `ValueTag` discriminant as a small monospace pill beside it.
- **Primary section** — depends on variant:
  - `Number` → the raw `f64` in a monospaced large display plus a "Scientific", "Hex bits", and "Nearest Excel integer" sub‑rows for power users.
  - `Text` → the `ExcelText` content rendered with a UTF‑16 code unit counter, the 32,767 code unit limit indicator, and a dangling‑surrogate warning if `to_string_lossy()` reports one.
  - `Logical` → `TRUE` / `FALSE` in a large display.
  - `Error` → the `WorksheetErrorCode` name, its numeric code, and the `ErrorSurface` classification (`Worksheet` — visible in any cell, `XllTransferable` — visible across XLL boundaries, `ExtendedWorksheetOnly` — not round‑trippable into a plain cell). Plus, when the source was `ExtendedValue::ErrorWithMetadata`, the metadata surface.
  - `Array` → an `ArrayShape` header (`3 rows × 4 cols`, `12 cells`), a scrollable grid with monospaced per‑cell `ArrayCellValue` rendering, row and column headers, and a "Send cell to inspector →" affordance per cell that drills into that cell's `ArrayCellValue → EvalValue` conversion.
  - `Reference` → the `ReferenceKind` as a badge, the `target` string, and for `MultiArea` references the expanded list of individual A1 targets via `multi_area_targets()`. A warning if the reference's `normalized()` form differs from the raw target.
  - `Lambda` → the `callable_token`, `origin_kind` (`HelperLambda` / `DefinedNameCallable` / `BuiltInCallable` / `ExternalRegisteredCallable`), `arity_shape` (`min`..`max`), `capture_mode` (`NoCapture` / `LexicalCapture`), and `invocation_contract_ref`. The user cannot invoke a lambda from Inspect, but they can see its shape.
  - `RichValue` → the `RichValueType.type_name` as header, the `fallback: RichValueData` rendered recursively, the `kvps: Vec<RichValueKeyValue>` as a table of (key, value, flags). Keys whose flag set contains `ExcludeFromCalcComparison` are marked with a small "not compared" pill. Nested `RichArray` and nested `RichValue` render recursively with indentation matching the Formula Walk tree.
- **Presentation section** (present only when the value is wrapped as `ExtendedValue::ValueWithPresentation`): the `NumberFormatHint` as a badge (`General` / `DateLike` / `Percentage` / `Currency` / `Scientific` / `Fraction` / `Custom`), the `CellStyleHint` as a secondary badge, and a read‑only format code string if one was derived. A subtle "Edit in Configure drawer →" link opens Area 3 Tab 1 on that value.
- **Effective display section**: the `effective_display_text` from `VerificationPublicationSurface` as the "what the user would see in a cell" line, with the contributing pipeline steps listed as small chips: `raw → format code → locale → CF override → effective`. Hovering a chip shows the intermediate form. When an engine pipeline step is `<NOT IMPLEMENTED>` it is greyed with its seam id.
- **Provenance footer**: for non‑root nodes, a "Show in Formula Walk" affordance that scrolls the Walk to the node, plus the green‑tree key if exposed.

### 4.2 Workbench mode — the Parity Matrix

The existing Workbench five‑cluster layout stays, but the **Outcome cluster** is replaced with a **Parity Matrix** fed directly from `VerificationCaseReport` and the rich mismatch / explain records.

**Top of the matrix** — the three verdicts as a horizontal strip of three large cards:

| Card | Value source | Colours |
|---|---|---|
| `value_match` | `VerificationCaseReport.value_match` | 🟢 pass / 🔴 fail / ⬜ not‑observable |
| `display_match` | `VerificationCaseReport.display_match` | 🟢 / 🔴 / ⬜ |
| `replay_equivalent` | `VerificationCaseReport.replay_equivalent` | 🟢 / 🔴 / ⬜ |

Each card shows, below its badge, a one‑line summary: for a passing card, "Matches" with a small icon; for a failing card, the dominant `mismatch_kind` from `replay_mismatch_kinds`; for `⬜`, the reason sourced from `ProgrammaticComparisonLane` (`OxfmlOnly`, `ExcelObservationBlocked`). Clicking a card scrolls the mismatch list (below) to the corresponding group.

**Side‑by‑side values** — two‑column panel directly under the verdict strip:

- Left column: **OxFml** — `OxfmlVerificationSummary.comparison_value` rendered with the same Value Panel component from Area 4.1, plus `evaluation_summary`, `effective_display_summary`, `parse_status`, `green_tree_key`, and a "Blocked reason" surfaced from `blocked_reason` when present.
- Right column: **Excel** — `ExcelObservationSummary.comparison_value` rendered through the same Value Panel component (so the user can structurally navigate both sides identically), plus `observed_value_repr`, `observed_formula_repr`, `effective_display_text`, `capture_status`.
- A central connector strip highlights where the two sides differ: aligned rows get a faint tinted background keyed to the mismatch family.

**Mismatch list** — a grouped list fed from `replay_mismatch_records`, grouped by `view_family`, each group header showing the family and count. Each row:

- `mismatch_kind` badge (`value_mismatch` / `display_mismatch` / `type_mismatch` / `array_shape_mismatch` / `format_dependency_mismatch` / others from `replay_mismatch_kinds`).
- `severity` pill (`critical` terracotta / `warning` amber / `info` teal).
- Left / right value representations (`left_value_repr`, `right_value_repr`) in a monospace diff strip.
- `detail` as a wrapped paragraph.

**Explain list** — fed from `replay_explain_records`. Each explain row is like a mismatch row but adds:

- The `query_id` as a clickable chip that scrolls the *trace* panel (below) to the corresponding query event.
- The `summary` as the row's primary line.

**Trace panel** — a chronological list of the replay trace events used to build the explains, so the user can see every evaluation step the comparison pipeline observed. Clicking a query id in an explain row highlights the corresponding event. This is the "why does this differ?" surface and is the single biggest lift over the current Workbench outcome list.

**Comparison lane indicator** — the `ProgrammaticComparisonLane` as a small banner at the top of the Parity Matrix (`Comparing against Excel observation` / `OxFml‑only comparison — Excel observation not requested` / `Excel observation blocked — reason: <blocked_reason>`). When the lane is `OxfmlOnly` or `ExcelObservationBlocked`, the right column of the side‑by‑side and the `display_match` / `value_match` cards degrade to ⬜ with the lane reason as the explanation; this prevents the user from reading "missing" as "matching".

### 4.3 Cross‑mode consistency

- The Value Panel component from 4.1 is **the same component** used in 4.2 for both the OxFml and Excel columns and for Inspect. There is exactly one value renderer in the app, driven by `ExtendedValue`; Inspect, Workbench side‑by‑side, the Explore result hero, and the Configure drawer's Tab 9 preview all mount it.
- The `PresentationHint` surface is consistent everywhere: same badge set, same classification tokens. When the Configure drawer Tab 1 changes the number format, the hint badge updates live across every mount.
- The Formula Walk in Inspect learns a new per‑node status: `node.value_class` sourced from the `ValueTag` of the node's `published_value`. This drives a small icon in the walk row (⟦N⟧ for number, ⟦T⟧ for text, ⟦A r×c⟧ for array, ⟦R⟧ for rich value, and so on) so the user sees the shape of every walk step.

### 4.4 Implicit explorer requirements honoured

- Values are *structurally* inspectable, not just stringified — the user can drill into an array cell, a rich value key, a reference's multi‑area list.
- Parity is *structural*, not binary — the three verdicts, the mismatch families, the trace‑linked explains are all visible.
- The same value renderer is used for "my result" and "Excel's result", so the user can compare apples to apples at every level of nesting.
- The parity matrix degrades honestly when the comparison lane cannot observe Excel; it does not silently paint passes over missing observations.

### 4.5 Gaps and back‑pressure

- Rendering `RichValue` structurally depends on OxFml forwarding the rich value intact through `VerificationPublicationSurface`. If the publication surface currently flattens rich values to their fallback, add `SEAM-OXFML-RICH-VALUE-PUBLICATION` — "carry the full `RichValue` through the publication surface, not just the fallback text".
- Rendering the trace panel depends on replay trace events being exposed through the retained artifact. The existing `oxreplay` trace emission exposes events; `SEAM-ONECALC-TRACE-CONSUMPTION` tracks the OneCalc‑side work to route them into the Parity Matrix.
- Displaying `green_tree_key` in the Value Panel footer depends on OxFml exposing green‑tree node identity for non‑root nodes. `SEAM-OXFML-GREEN-TREE-NODE-ID` tracks this.
- The side‑by‑side value panel can render an Excel value through the same component only if `ExcelObservationSummary.comparison_value` parses into an `ExtendedValue`. Today it is `serde_json::Value`; `SEAM-ONECALC-EXCEL-VALUE-INTO-EXTENDED` tracks the OneCalc‑side adapter that lifts it (engine side already produces the JSON, so this is a OneCalc adapter, not an engine seam).

---

## Cross‑cutting: Information architecture and visual coherence

To make all three areas feel like one product:

- **Single context bar grammar.** Across Explore / Inspect / Workbench the bar reads: `[breadcrumb] [mode pill] [scope summary strip] [actions]`. The actions vary per mode but the layout does not.
- **Single drawer pattern.** Editor settings, Configure (Area 3), Witness chain, Handoff history, Provenance, Host context, Node detail — all are right‑side drawers that replace the Assist column with the same animation, header treatment and close affordance.
- **Single feedback channel.** Toasts at bottom for transient confirmation; banners at top of the affected panel for persistent warnings; modal sheets only for irreversible destructive actions (close last unsaved space, clear scratch).
- **Mode accent.** The active mode's accent (teal Explore / moss Inspect / terracotta Workbench) is used for the rail row's left border, the mode pill, the primary button in that mode, and the breadcrumb "mode" segment — and nowhere else, so the user can locate themselves at a glance.
- **Explicit fallback per layer.** Editor falls back to plain textarea; completion popup falls back to the right‑column completion list; current help falls back to the right‑column card; Inspect Walk falls back to a flat node list. Each fallback has an honest user‑visible reason.

---

## Critical files to be modified (rough map for the implementation note)

These are the entry points the implementation note should cover; this plan does not prescribe their internals.

- Editor surface and overlays
  - `src/dnaonecalc-host/src/ui/components/formula_editor_surface.rs` — host the popups and decoration layers; consume `EditorMeasuredOverlayBox`.
  - `src/dnaonecalc-host/src/ui/editor/state.rs` — entry mode + state machine state (Idle / Editing‑live / Proofed‑scratch / Committed).
  - `src/dnaonecalc-host/src/ui/editor/commands.rs` — new commands: `CommitEntry`, `CancelEntry`, `RequestProof`, `CycleReferenceForm`, `ToggleEditMode`, `ToggleExpandedHeight`, `ForceShowCompletion`, `SendSelectionToInspect`.
  - `src/dnaonecalc-host/src/ui/editor/render_projection.rs` — bracket pair detection and active‑argument span derivation from `EditorSyntaxSnapshot`.
  - `src/dnaonecalc-host/src/ui/editor/geometry.rs` — popup anchor pixel translation.
  - New: editor settings popover component under `ui/components/`.
- Explore shell layout discipline (`dno-yjk.7`)
  - `src/dnaonecalc-host/src/ui/components/explore_shell.rs` — delete the hero header, lead paragraph, overview deck, panel header, panel intro, editor summary row, and editor note; mount `FormulaEditorSurface` as the immediate first child of the editor column; move the blocked‑reason banner below the surface.
  - `src/dnaonecalc-host/src/ui/design_tokens/theme.rs` — delete the now‑orphaned `.onecalc-explore-shell__header*`, `.onecalc-explore-shell__lead`, `.onecalc-explore-shell__overview-*`, `.onecalc-explore-shell__panel-header / -intro`, `.onecalc-explore-shell__editor-summary-row / __status-card / __editor-note` rules; rewrite `.onecalc-explore-shell__body` to a three‑column grid with `min-height: 0` per column so `FormulaEditorSurface` / `ExploreResultPanel` / `ExploreHelpPanel` each own their own scroll; add `.onecalc-shell-frame__configure-action` for the relocated Configure button.
  - `src/dnaonecalc-host/src/ui/components/shell_frame.rs` — add the `on_configure_toggle: Option<Callback<()>>` prop and render a new Configure action button in the context bar's right‑hand action area. Render the new optional `trace_summary` footer fact.
  - `src/dnaonecalc-host/src/services/shell_composition.rs` — add `trace_summary: Option<String>` to `ShellFrameViewModel` and populate from `active_formula_space.context.trace_summary`.
  - `src/dnaonecalc-host/src/ui/components/app_shell.rs` — wire `on_configure_toggle` to `EditorCommand::ToggleConfigureDrawer` via the existing reducer entry point.
- Shell, rail and case management
  - `src/dnaonecalc-host/src/ui/components/shell_frame.rs` — rail header buttons, hover affordances, breadcrumbs in context bar.
  - `src/dnaonecalc-host/src/state/types.rs` — workspace persistence shapes, dirty marker, recents list.
  - `src/dnaonecalc-host/src/state/reducer.rs` — wire `open_retained_artifact_from_catalog`, `import_manual_retained_artifact_into_active_formula_space`, `import_verification_bundle_report_json` to UI commands; add `new_formula_space`, `rename_formula_space`, `duplicate_formula_space`, `close_formula_space`, `pin_formula_space`, `retain_current_run`.
  - `src/dnaonecalc-host/src/persistence/mod.rs` — workspace JSON v1.
  - New: command palette component under `ui/components/`.
  - New: catalog picker sheet under `ui/components/`.
- Configuration
  - `src/dnaonecalc-host/src/state/types.rs` — `FormulaSpaceContext` value object encompassing locale, profile id, scenario policy, scenario flags, format string, CF rules.
  - `src/dnaonecalc-host/src/services/verification_bundle.rs` — extend bundle round‑trip to include `FormulaSpaceContext`.
  - `src/dnaonecalc-host/src/adapters/oxfml/` — pass `FormulaSpaceContext` into edit requests.
  - New: configure drawer component under `ui/components/`.
  - New: workspace settings page under `ui/components/`.
- Design tokens
  - `src/dnaonecalc-host/src/ui/design_tokens/theme.rs` — typography scale, animation durations, z‑index map, dark/high‑contrast variants reserved.

## Verification

This plan is a UX plan, so verification is staged behavioural, not just compile‑and‑test:

- **Editor**
  - Type `=SUM(` and confirm the signature ScreenTip appears anchored above the caret with the active argument bolded; advance with comma and confirm the active argument moves.
  - Press `Tab` over a completion item and confirm it inserts.
  - Press `Esc` mid‑edit and confirm the formula reverts.
  - Press `F4` with a reference token in selection and confirm reference forms cycle.
  - Inject a malformed formula and confirm a wavy underline appears at the diagnostic span with a hover tooltip.
  - Disable overlays via the editor settings popover and confirm the textarea remains usable.
- **Explore shell layout discipline**
  - Run `scripts/run-onecalc-preview.ps1`, hard‑reload the browser, and confirm the formula textarea's first line is within the top `~200px` of the viewport on a 1440×900 display.
  - Type a one‑line formula and confirm the result hero (Value Panel) is visible without scrolling.
  - Type a 30‑line formula and confirm (a) the editor column scrolls internally, (b) the result column does not drift off the fold, (c) the completion/help column is still visible.
  - Open the Configure drawer from the new context‑bar Configure button and confirm the help column swaps to the drawer without the editor or result columns resizing.
  - Switch mode Explore → Inspect → Workbench → Explore and confirm the mode accent flow (teal / moss / terracotta) still applies to the context bar Configure button and breadcrumb mode segment.
  - Inspect the rendered HTML and confirm that (a) no element with `data-role="explore-overview-deck"` / `explore-panel-intro" / `explore-editor-summary"` / `explore-editor-note"` exists, (b) exactly one `data-component="formula-editor-surface"` is the first descendant of `.onecalc-explore-shell__body-column--editor`, (c) `data-role="value-panel-effective-display"` is present inside the result column.
- **Case management**
  - Cold launch with empty workspace state → an Untitled formula space appears, focused.
  - Create three spaces, type into one, cycle with `Ctrl+Tab`, confirm caret and editor state are preserved per space.
  - Retain a run, close the workspace, relaunch, confirm the retained artifact reappears in the catalog and the formula space is restored.
  - Drag a verification‑bundle file onto the shell and confirm `import_verification_bundle_report_into_workspace` is invoked and the resulting formula space lands in Workbench mode.
  - Open Command Palette, run "Switch to formula space…", confirm fuzzy filter and selection.
- **Configuration**
  - Change the format code in the Configure drawer and confirm the result panel's effective display updates within one frame.
  - Add a CF rule "value > 100 → terracotta background" and confirm a current value of `120` paints terracotta in the result panel.
  - Switch scenario policy from Deterministic to Real‑time and confirm the context bar segment, the drawer toggle, and the dropdown all reflect the same value.
  - Open Workspace settings, change locale, confirm the formula‑space defaults inherit it and the change persists across relaunch.
- **Cross‑cutting**
  - Switch modes Explore → Inspect → Workbench → Explore on the same formula space and confirm caret, scroll, and drawer state all restore.
  - Run `cargo test -p dnaonecalc-host` and exercise the existing editor and reducer test suites; add per‑area tests as the implementation note specifies.
  - Build the WASM target and confirm the editor's keyboard map advertises its substitutes for any chord the browser intercepts.
- **Seam visibility**
  - With Tab 2 (Alignment) opened, confirm every control renders, accepts input, persists into the workspace JSON, and shows the `<NOT IMPLEMENTED>` badge with seam id `SEAM-OXFML-ALIGNMENT-MODEL` in the badge popover.
  - Open the Workspace settings → Seam status board and confirm that all `SEAM-*` ids referenced by active formula spaces are listed and grouped by repo.
  - Round‑trip a formula space through `services/verification_bundle.rs` and confirm the `CellFormatPayload` and `CalcOptionsPayload` survive in the bundle XML even when no engine consumes them.
- **Value Panel (Area 4.1)**
  - Evaluate `=SUM(1,2,3)` and confirm the Value Panel renders `Number` header, raw `6` with sub‑rows, and no presentation section.
  - Evaluate `=DOLLAR(1234.5)` and confirm the Value Panel renders `Number` with a Presentation section showing `NumberFormatHint::Currency` and the effective display `"$1,234.50"`.
  - Evaluate `=SEQUENCE(3,4)` and confirm the Value Panel renders `Array` with shape header `3 rows × 4 cols`, 12 scrollable cells, each cell drillable.
  - Evaluate `=1/0` and confirm the Value Panel renders `Error` with `#DIV/0!`, the `ErrorSurface::Worksheet` badge.
  - Evaluate `=LAMBDA(x, x*2)` and confirm the Value Panel renders `Lambda` with `callable_token`, `origin_kind`, `arity_shape`, `capture_mode`.
  - Feed a test fixture carrying an `ExtendedValue::RichValue` and confirm the Value Panel renders the `type_name`, the key‑value table, and any `ExcludeFromCalcComparison` key flags.
  - Confirm an `ExcelText` value near the 32,767 UTF‑16 code unit limit renders the counter and, if a dangling surrogate is injected, the warning.
- **Parity Matrix (Area 4.2)**
  - Retain a run where OxFml and Excel agree on value and display and confirm all three verdict cards are 🟢 and the Parity Matrix opens in Inspect via `open_mode_hint`.
  - Retain a run where OxFml and Excel disagree on display only (e.g. format code difference) and confirm `value_match` is 🟢 but `display_match` is 🔴; confirm the mismatch list groups under `view_family` for display.
  - Retain a run captured under `ProgrammaticComparisonLane::OxfmlOnly` and confirm all three verdict cards degrade to ⬜ with "Excel observation not requested" as the explanation.
  - Retain a run captured under `ExcelObservationBlocked` and confirm the blocked reason from `ExcelObservationSummary.capture_status` surfaces in the banner.
  - Click a `query_id` chip in an explain row and confirm the trace panel scrolls to the matching event.
  - Confirm the side‑by‑side Value Panels are the same component by injecting a structural mismatch (e.g. OxFml returns `Array[3×4]`, Excel returns `Array[2×4]`) and confirming both sides render with the shape header and the connector strip highlights the shape delta.

---

## Appendix A — OxFml / OxFunc / OxXlPlay engine inventory (driver of every `<NOT IMPLEMENTED>` marker)

### A.0 Recent crate split and comparison enrichment

- **`oxfunc_value_types` (NEW)** — `C:/Work/DnaCalc/OxFunc/crates/oxfunc_value_types/src/lib.rs`. Owns: `EvalValue` (461‑469), `CellContentValue` (472‑478), `CallArgValue` (481‑486), `EvalArray` (271‑401), `ArrayCellValue` (250‑268), `ArrayShape` (203‑212), `RichValue` (335‑339), `RichValueType` (284‑288) with `key_flags` including `ExcludeFromCalcComparison`, `RichValueData` (291‑305), `RichArray` (302‑326), `LambdaValue` (404‑458), `CallableArityShape` (63‑80), `CallableOriginKind` (49‑54), `CallableCaptureMode` (57‑60), `ReferenceLike` (93‑142), `ReferenceKind` (83‑90), `ExcelText` (215‑247), `WorksheetErrorCode` (15‑30), `ErrorSurface` (534‑538), `ValueTag` (33‑46), `ValueBoundary` (555‑626) with `allows(tag)` enforcement, `PresentationHint` (505‑531), `NumberFormatHint` (489‑497), `CellStyleHint` (500‑502), `ExtendedValue` (541‑552) with variants `Core(EvalValue)`, `RichValue(Box<RichValue>)`, `ValueWithPresentation { value, hint }`, `ErrorWithMetadata { code, surface }`.
- **Re‑export** — `oxfunc_core/src/lib.rs:1` does `pub use oxfunc_value_types::*;` so existing `oxfunc_core::value::*` imports still resolve.
- **OxFml consumption** — `oxfml_core/src/publication/mod.rs:2`, `interface/mod.rs:10`, `consumer/replay/mod.rs:8`, `host/mod.rs`, `eval/mod.rs`, `format/engine.rs` all import the value types via `oxfunc_core::value`.
- **Rich comparison plumbing**:
  - `ReplayComparisonView { view_family: String, value: serde_json::Value }` at `oxfml_core/src/consumer/replay/mod.rs:164‑167`.
  - `ReplayProjectionResult { comparison_views, verification_publication_surface, ... }` at `consumer/replay/mod.rs:170‑194`.
  - `OxReplayMismatchRecord { mismatch_kind, severity, view_family, left_value_repr, right_value_repr, detail }` at `dnaonecalc-host/src/services/verification_bundle.rs:101‑108`.
  - `OxReplayExplainRecord { query_id, summary, ... }` at `verification_bundle.rs:111‑120`.
  - `VerificationCaseReport { value_match, display_match, replay_equivalent, replay_mismatch_kinds, replay_mismatch_records, replay_explain_records, oxfml_summary, excel_summary }` at `verification_bundle.rs:123‑141`.
  - `OxfmlVerificationSummary { comparison_value, effective_display_summary, parse_status, green_tree_key, ... }` at `verification_bundle.rs:82‑89`.
  - `ExcelObservationSummary { comparison_value, observed_value_repr, effective_display_text, observed_formula_repr, capture_status, ... }` at `verification_bundle.rs:92‑98`.
  - `RetainedArtifactRecord { comparison_status, oxfml_comparison_value, excel_comparison_value, value_match, display_match, replay_equivalent, replay_mismatch_records, replay_explain_records, oxfml_effective_display_summary, excel_effective_display_text, ... }` at `state/types.rs:197‑217`.
  - `ProgrammaticComparisonStatus` (`Matched` / `Mismatched` / `Blocked`) and `ProgrammaticComparisonLane` (`OxfmlOnly` / `OxfmlAndExcel` / `ExcelObservationBlocked`) at `services/programmatic_testing.rs:40‑59`.
- **`VerificationPublicationSurface` effective display pipeline** — `oxfml_core/src/publication/mod.rs:65‑96` carries `published_value: EvalValue`, `published_value_class: WorksheetValueClass`, `visible_value_text`, `effective_display_text`, `presentation_hint: Option<PresentationHint>`, `number_format_code`, `format_profile`, `locale_format_context`, `format_dependency_facts`, `format_delta`, `display_delta`, conditional formatting rules with applied colours and effective displays. `render_effective_display_text()` at lines 231‑271 is the canonical render path.

### A.1 Feature coverage table

| Feature | OxFml | OxXlPlay | OneCalc UI plan implication |
|---|---|---|---|
| **Number format strings** | PARTIAL — TEXT/FIXED/DOLLAR accept `format_code` via `FormatCodeEngine` (`OxFunc/crates/oxfunc_core/src/locale_format.rs`). No full Excel grammar validation. | PARTIAL — captures `effective_display_text` + `number_format_code` (derived from XML, read‑only). | Wire format code field; mark grammar‑validation gaps with `SEAM-OXFUNC-FMT-GRAMMAR-VALIDATION`. |
| **Effective display text** | PRESENT — `VerificationPublicationSurface.effective_display_text` (`OxFml/crates/oxfml_core/src/publication/mod.rs`). | PRESENT — captures `Range.Text` directly. | Render OxFml's value as authoritative; show OxXlPlay's as comparand in Workbench. |
| **Locale** | PRESENT — `LocaleFormatContext` + `FormatProfile`, but only `EnUs` / `CurrentExcelHost` are hardcoded (`OxFunc/crates/oxfunc_core/src/locale_format.rs`). Wired via `EditorPlanOptions.locale_profile`. | ABSENT. | Expose locale picker; mark `<NOT IMPLEMENTED>` for any non‑hardcoded entry; `SEAM-OXFUNC-LOCALE-EXPAND`. |
| **Date system (1904)** | PRESENT — `WorkbookDateSystem::System1900 \| System1904`, wired via `EditorPlanOptions.date_system`. | ABSENT. | LIVE control. |
| **R1C1 reference style** | PARTIAL — parser admits R1C1 internally; no public toggle. | ABSENT. | Render toggle; `SEAM-OXFML-R1C1-PUBLIC`. |
| **CF rules** | PARTIAL — operator + expression rules only via `evaluate_operator_rule()` / `evaluate_expression_rule()` (`OxFml/crates/oxfml_core/src/publication/mod.rs`). No colour scales / data bars / icon sets. Formula visibility unresolved. | PARTIAL — captures rules + effective style as `derived` (expression subset only). | Render full rules manager; mark every non‑expression / non‑operator rule with the appropriate `SEAM-OXFML-CF-*` id. |
| **Font (full)** | ABSENT (only colour for CF overlay). | PARTIAL (font colour observed). | Render full Font tab; `SEAM-OXFML-FONT-MODEL`, `SEAM-OXFML-FONT-COLOR`. |
| **Borders** | ABSENT. | ABSENT. | Render full Border tab; `SEAM-OXFML-BORDER-MODEL`. |
| **Alignment** | ABSENT. | ABSENT. | Render full Alignment tab; `SEAM-OXFML-ALIGNMENT-MODEL`. |
| **Protection** | ABSENT. | ABSENT. | Render Protection tab; `SEAM-OXFML-PROTECTION-MODEL`. |
| **Style index / XF** | ABSENT in code (acknowledged in docs). | PARTIAL (captures `style_id`, no resolution). | Surface style id in read‑only diagnostic; `SEAM-OXFML-STYLE-XF`. |
| **Fill (solid)** | ABSENT (only CF overlay). | PARTIAL (`fill_color` observed). | Render fill colour; `SEAM-OXFML-FILL-COLOR`. |
| **Fill (gradient / pattern)** | ABSENT. | ABSENT. | Render Fill Effects sub‑popover; `SEAM-OXFML-FILL-EFFECTS`. |
| **Host context (`SingleFormulaHost`)** | PRESENT — `caller_row/col`, `defined_names`, `cell_values`, `table_catalog`, `enclosing_table_ref`, `caller_table_region`, `now_serial`, `random_provider` (`OxFml/crates/oxfml_core/src/host/mod.rs`). | n/a. | Wire Tab 9 controls directly. |
| **Host info / RTD providers** | PRESENT (traits) — `HostInfoProvider`, `RtdProvider` in `TypedContextQueryBundle`. | n/a. | OneCalc must implement `HostInfoProvider`; `SEAM-ONECALC-HOST-INFO`, `SEAM-ONECALC-RTD`. |
| **Defined names / table catalog** | PRESENT. | n/a. | LIVE Tab 9 surface. |
| **Iterative calc** | ABSENT. | ABSENT. | Render Calc Options controls; `SEAM-OXFML-CALC-ITERATIVE`. |
| **Calc mode (auto/manual)** | ABSENT (single‑formula model). | ABSENT. | Render; `SEAM-OXFML-CALC-MODE`. |
| **Precision as displayed** | ABSENT. | ABSENT. | Render; `SEAM-OXFML-PRECISION-AS-DISPLAYED`. |
| **External link values** | ABSENT. | ABSENT. | Render; `SEAM-OXFML-EXTERNAL-LINKS`. |
| **Data validation** | ABSENT in publication; W039 mentions DV sublanguage but not exercised. | ABSENT. | Out of scope for this plan; reserve a future tab. |
| **Format dependency tracking** | PRESENT — `format_dependency_facts` in `VerificationPublicationSurface`. | ABSENT. | Use to drive "recalc required" hints in the Configure drawer header. |
| **Capability snapshot** | n/a — provided indirectly through `EditorPlanOptions` and host artifacts. | n/a. | OneCalc must populate its empty `CapabilityAndEnvironmentState`; `SEAM-ONECALC-CAPABILITY-SNAPSHOT`. |

---

## Appendix B — Seam requirements (engine work the UI surface depends on)

Each item below is the contract the named repo must provide so the corresponding `<NOT IMPLEMENTED>` UI marker can be removed. None of these is implemented as part of this plan; they are listed for the follow‑up engine pass. The plan deliberately leaves the badge in the UI until the engine work lands and the OneCalc bridge is wired through.

### OxFml seams

- **`SEAM-OXFML-FORMAT-PAYLOAD`** — accept a new `CellFormatPayload` field on `FormulaEditRequest` and `VerificationPublicationContext` carrying number format code, font, fill, border, alignment, protection, and CF rule list. OneCalc will populate this from Tab 1‑6; OxFml may ignore unsupported sub‑fields, but must round‑trip them through `EditorDocument` so the Workbench replay envelope captures them.
- **`SEAM-OXFML-CF-COLORSCALE`**, **`SEAM-OXFML-CF-DATABAR`**, **`SEAM-OXFML-CF-ICONSET`** — extend the CF model in `publication/mod.rs` with `ColourScaleRule { gradient }`, `DataBarRule { min, max, colour }`, `IconSetRule { set, thresholds }` types and corresponding `evaluate_*_rule()` functions. Fits naturally into the existing CF workset W036.
- **`SEAM-OXFML-CF-TEXT`**, **`SEAM-OXFML-CF-DATES`**, **`SEAM-OXFML-CF-BLANKS`**, **`SEAM-OXFML-CF-ERRORS`**, **`SEAM-OXFML-CF-RANK`**, **`SEAM-OXFML-CF-AVERAGE`**, **`SEAM-OXFML-CF-UNIQUE`** — close the W039 sublanguage so each Excel rule type lowers into an evaluable form.
- **`SEAM-OXFML-FONT-MODEL`**, **`SEAM-OXFML-FONT-COLOR`** — extend the formatting hierarchy doc and code to model font (family, size, weight, style, underline, colour, effects). Hierarchy doc already acknowledges this gap (sections 19‑33).
- **`SEAM-OXFML-BORDER-MODEL`**, **`SEAM-OXFML-ALIGNMENT-MODEL`**, **`SEAM-OXFML-PROTECTION-MODEL`**, **`SEAM-OXFML-FILL-COLOR`**, **`SEAM-OXFML-FILL-EFFECTS`**, **`SEAM-OXFML-STYLE-XF`** — same: extend the model with the corresponding XF / style sub‑structures and resolution.
- **`SEAM-OXFML-R1C1-PUBLIC`** — promote R1C1 from internal parser detail to a public `EditorPlanOptions.reference_style` enum, document the supported R1C1 grammar, and expose the toggle through the editor facade.
- **`SEAM-OXFML-CALC-MODE`**, **`SEAM-OXFML-CALC-ITERATIVE`**, **`SEAM-OXFML-PRECISION-AS-DISPLAYED`**, **`SEAM-OXFML-EXTERNAL-LINKS`** — introduce a `CalcOptionsPayload` on `FormulaEditRequest`. The single‑formula host model inherently does not implement iterative calc; the field exists for round‑trip fidelity, and to allow a future multi‑cell host (OxCalc) to honour it.
- **`SEAM-OXFML-EVAL-FREEZE`**, **`SEAM-OXFML-EVAL-CACHE`**, **`SEAM-OXFML-EVAL-STRICT`** — add evaluation‑flag fields on the host context for the scenario flags the UI exposes.

### OxFunc seams

- **`SEAM-OXFUNC-FMT-GRAMMAR-VALIDATION`** — promote `FormatCodeEngine` from stub to full Excel format‑code grammar: section parsing (`positive;negative;zero;text`), colour codes (`[Red]`, `[Blue]`, `[Color3]`), conditional sections (`[>100]`, `[<0]`), elapsed time tokens (`[h]`, `[m]`, `[s]`), fraction denominator hints, locale tokens, asterisk fill, underscore spacing, escape characters. Validation should produce diagnostics that OxFml can surface through `LiveDiagnosticSnapshot`.
- **`SEAM-OXFUNC-FMT-RED`** — implement the colour‑token branch (`[Red]`, `[Blue]`, `[Color3]`, theme colours) in `render_with_code`.
- **`SEAM-OXFUNC-FMT-CURRENCY`**, **`SEAM-OXFUNC-FMT-ACCOUNTING`** — extend the currency / accounting renderer beyond locale default symbol.
- **`SEAM-OXFUNC-FMT-ELAPSED`** — implement `[h]`, `[m]`, `[s]` elapsed‑time tokens.
- **`SEAM-OXFUNC-FMT-FRACTION`** — implement fraction renderer (`# ?/?`, `# ??/??`, `# ???/???`, `# ?/2`, `# ?/4`, `# ?/8`, `# ?/16`, `# ?/10`, `# ?/100`).
- **`SEAM-OXFUNC-FMT-SCIENTIFIC`** — implement variable exponent width and `e+0` / `E-00` token variants.
- **`SEAM-OXFUNC-FMT-SPECIAL`** — implement Special category renderer (Zip Code, Zip+4, Phone Number, SSN, plus locale equivalents). Fundamentally this is a registry of locale → list of (label, format code) pairs.
- **`SEAM-OXFUNC-LOCALE-EXPAND`** — extend `LocaleProfileId` enum and `format_profile()` table to cover at minimum `de_DE`, `fr_FR`, `es_ES`, `it_IT`, `nl_NL`, `pt_BR`, `ja_JP`, `zh_CN`, `ko_KR`, `ru_RU`. Each entry needs decimal / thousands / list / currency / date / time separators and locale‑specific built‑in date formats.

### OxXlPlay seams

- OxXlPlay observation is read‑only by design; this plan does not require it to *evaluate* any of the formatting controls. The seams below are the ones needed for full round‑trip:
- **`SEAM-OXXLPLAY-CAPTURE-FONT`**, **`SEAM-OXXLPLAY-CAPTURE-BORDER`**, **`SEAM-OXXLPLAY-CAPTURE-ALIGNMENT`**, **`SEAM-OXXLPLAY-CAPTURE-PROTECTION`** — extend `ObservableSurfaceKind` to include the corresponding XF projections (font family / size / weight / style / underline; border sides + line styles + colours; alignment; protection). Source from SpreadsheetML XF resolution.
- **`SEAM-OXXLPLAY-CAPTURE-CF-VISUAL`** — extend `ConditionalFormattingRules` and `ConditionalFormattingEffectiveStyle` to capture colour scales, data bars, and icon sets from SpreadsheetML.
- **`SEAM-OXXLPLAY-INPUT-CONTEXT`** — accept a `FormulaSpaceContext`‑shaped payload as part of the verification request, even if OxXlPlay only echoes it back so OneCalc can see what the bundle round‑tripped.

### OneCalc‑side seams (UI work only — no shimming of engine logic)

- **`SEAM-ONECALC-CAPABILITY-SNAPSHOT`** — populate the empty `CapabilityAndEnvironmentState` from `ProgrammaticVerificationConfig`, host artifacts, and engine version reports so the workspace settings page has data to render.
- **`SEAM-ONECALC-HOST-INFO`** — implement the `HostInfoProvider` trait against the OS / browser environment so INFO and CELL functions work.
- **`SEAM-ONECALC-RTD`** — design and implement the RTD provider editor in Tab 9 and the corresponding `RtdProvider` impl.
- **`SEAM-ONECALC-TABLE-EDITOR`** — design the in‑drawer editor for `table_catalog` and `enclosing_table_ref` so users can express table context.
- **`SEAM-ONECALC-VERIFICATION-BUNDLE-CONTEXT`** — extend `services/verification_bundle.rs` to round‑trip the new `CellFormatPayload` and `CalcOptionsPayload` even though no current consumer reads them.
- **`SEAM-ONECALC-SEAM-BOARD`** — implement the seam status board on the Workspace settings page (it needs a workspace‑level registry of active seams referenced by formula spaces).
- **`SEAM-ONECALC-EXCEL-VALUE-INTO-EXTENDED`** — adapter lifting `ExcelObservationSummary.comparison_value: serde_json::Value` into an `ExtendedValue` the Value Panel component can render. No engine change — just a OneCalc‑side parser.
- **`SEAM-ONECALC-TRACE-CONSUMPTION`** — wire oxreplay trace events from the retained artifact into the Parity Matrix trace panel, with query id linkage to explain records.

### Value representation and parity surface seams (Area 4)

- **`SEAM-OXFML-RICH-VALUE-PUBLICATION`** — carry the full `RichValue` (including `kvps`, `key_flags`, nested `RichArray` / `RichValue`) through `VerificationPublicationSurface`, not just the flattened `fallback` text, so the Value Panel can render it structurally. Today the publication surface only exposes `EvalValue`, which loses the rich structure.
- **`SEAM-OXFML-GREEN-TREE-NODE-ID`** — expose green‑tree node identity on every Formula Walk node so Inspect's Value Panel footer can display the node's identity and the Explore → Inspect "send selection" bridge can land on the correct node.
- **`SEAM-OXFML-COMPARISON-VIEW-TAXONOMY`** — document the stable set of `view_family` strings produced by `ReplayComparisonView` so the Parity Matrix can render family‑specific UI (e.g. a dedicated subview for `format_dependency` mismatches). Today the set is implicit.
- **`SEAM-OXFML-PRESENTATION-PROPAGATION`** — guarantee that `ExtendedValue::ValueWithPresentation` is used for every function whose output has a natural presentation classification (TEXT, DOLLAR, FIXED, NOW, TODAY, DATE, TIME, PERCENT operators, HYPERLINK, etc.), not only the ones already converted. This unlocks the pre‑fill behaviour in the Configure drawer Number tab and the Value Panel's Presentation section.
- **`SEAM-OXFUNC-VALUE-BOUNDARY-HELP`** — attach human‑readable error messages to `ValueBoundary::allows()` rejections so the Host Bindings tab can show *why* a typed value is invalid at a slot rather than just refusing it.

### Excel parity tracking

For each of the Format Cells tabs and the CF rules manager, a small parity test suite under `tests/format_cells/` should exist that:

1. Loads a known reference workbook (`fixtures/format_cells/reference.xlsx`) through OxXlPlay.
2. Asserts that every Format Cells control has at least one fixture cell exercising it.
3. Asserts that for every fixture cell, OneCalc's Configure drawer renders the expected control values (whether LIVE, PARTIAL or `<NOT IMPLEMENTED>`), and that round‑trip back into a verification bundle preserves them.

This is the verification harness that prevents silent drift between the UI surface, the engine surfaces, and Excel itself.

---

## Phasing and ordering

This workset is now sequenced as **two phases**: a Phase A reset that ships the minimal interface and the missing test layer, and a Phase B that brings back the rich UX surfaces under test discipline.

### Phase A — Minimal foundation reset

These steps must complete before any Phase B work begins. See *Area 0 — Minimal Foundation Reset* for the full scope of each.

**A0. Anchor reference document.** ✅ Complete. [`HOST_VIEW_MODEL_REFERENCE.md`](../HOST_VIEW_MODEL_REFERENCE.md) describes the view‑model layer that survives the reset and lists every test invariant the reset must pin.

**A1. Minimal text editor + minimal result interface** (`dno-yjk.A1`, P1). New `MinimalOneCalcApp` / `MinimalEditor` / `MinimalResult` components mounted in place of the existing rich shell. Plain `<textarea>` with `spellcheck="false"` and `autocomplete="off"`. No keymap interception. No overlay layers. Read‑only result block showing `effective_display_summary`, the first three diagnostics, and the green‑tree key.

**A2. State and reducer invariant test suite** (`dno-yjk.A2`, P1). New `tests/host_state/` cargo integration test crate pinning every invariant from `HOST_VIEW_MODEL_REFERENCE.md` §§11.1–11.5. Runs in plain `cargo test` (no browser).

**A3. Browser test layer** (`dno-yjk.A3`, P1). New `tests/browser/` crate using `wasm-bindgen-test`. Pins the §11.6 corpus: click‑at‑offset, arrow keys, Backspace, Delete, Enter, Tab, F4, spellcheck‑off, type‑and‑see‑result, type‑and‑see‑error, type‑and‑see‑array. New `scripts/run-browser-tests.ps1` invocation. **This is the missing test layer that caused the cycle of regressions.**

**A4. Automated UX validation criteria** (`dno-yjk.A4`, P2). New `scripts/run-ux-validation.ps1` that exercises the canonical regression paths against the running preview and asserts user‑visible invariants. Includes the explicit "click in the middle, then press left, then assert cursor lands at click−1" canary.

**A5. Archive the existing rich front‑end** (`dno-yjk.A5`, P2, blocked by A3 + A4). Move the existing rich Leptos surface (`app_shell.rs`, `explore_shell.rs`, `inspect_shell.rs`, `workbench_shell.rs`, `shell_frame.rs`, `shell_drawer.rs`, `formula_editor_surface.rs`, `value_panel.rs`, helpers, tests) into `src/dnaonecalc-host/src/ui_archive_2026_04/` behind `#[cfg(feature = "ui-archive-2026-04")]`. Default build no longer compiles the archive. The cargo feature lets developers re‑enable it for reference comparisons.

**A6. Reference doc maintenance** (`dno-yjk.A6`, P3). Update `HOST_VIEW_MODEL_REFERENCE.md` with any reset‑induced changes.

Phase A acceptance is the seven items listed in *Area 0 → Phase A acceptance*. Until those pass, no Phase B work starts.

### Phase B — Reintroduce rich UX, gated on tests

Each Phase B step extends the test corpus from A3 + A4 *first*, then ships the rendering work. **No surface comes back without an automated UX validation criterion that exercises it.** The ordering below is unchanged from the original WS‑13 plan but is now re‑gated.

1. **Value Panel component** (Area 4.1 / 4.3) — a single reusable renderer for `ExtendedValue`, used by every subsequent surface.
2. **Editor state machine, result‑class pill, and overlay anchors** (Area 1) — depends on the Value Panel for the result hero. Reintroduces the editor settings popover, completion popup, signature ScreenTip, bracket pair highlight, and live‑state pills, **each with its own browser test invariant added to `tests/browser/`** before mounting.
3. **Shell frame grammar with mode accents, breadcrumbs, scope strip, and rail affordances** (Area 1B / Area 2) — reintroduces `ShellFrame`, the rail with case lifecycle, the breadcrumb and scope strip, and the mode tabs. The Explore vertical rhythm rules from Area 1B *Explore shell layout discipline* apply from the first reintroduction.
4. **Workspace persistence and case lifecycle** (Area 2) — unblocks the "come back to this later" flow.
5. **Parity Matrix in Workbench** (Area 4.2) — once the Value Panel and Area 2 exist, reshaping Workbench is mechanical.
6. **Configure drawer skeleton + status badges + seam status board** (Area 3 chrome).
7. **Configure drawer Number, Scenario Policy, Host Bindings tabs** (Area 3 LIVE / PARTIAL surfaces).
8. **Configure drawer Alignment, Font, Border, Fill, Protection tabs** (Area 3 `<NOT IMPLEMENTED>` surfaces).
9. **Configure drawer Conditional Formatting tab and rules manager**.
10. **Workspace settings page** — including the seam status board.
11. **Excel parity test harness** under `tests/format_cells/` — plus a new `tests/parity_matrix/` harness that uses retained artifacts with known mismatches to drive the Workbench Parity Matrix surface.

The engine‑side seam work (Appendix B) is a separate pass that follows. Each seam can land independently; as engines close them, the corresponding UI badge flips from 🔴 to 🟡 to 🟢 with no UI rework required because every control already exists.

The previously‑landed yjk beads (`dno-yjk.1/2/3/7`) are now considered part of the **archived** rich front‑end. Their implementation is preserved in `ui_archive_2026_04/` after A5 lands and serves as a reference for Phase B step 3. Their bead records remain closed but no longer represent the running interface.

