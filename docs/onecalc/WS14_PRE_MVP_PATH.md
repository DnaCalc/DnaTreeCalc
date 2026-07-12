# WS14_PRE_MVP_PATH — Smallest live slice

> **Document role.** Concrete execution path from the current WS-13 reset
> state to a *pre-MVP* of the WS-14 progressive-disclosure home: only a
> formula editor and an evaluation result, **driven through the real
> `OxFml`/`OxFunc` machinery** via the existing live bridge. No saving, no
> drill-downs, no compare, no analysis surfaces.
>
> The point of this slice is to **prove the engine round-trip works through
> the new shell** before we add any of the WS-14 surfaces (drill-downs,
> scenarios, compare, palette).
>
> **Read alongside:**
> - [APP_UX_REALIZATION.md](APP_UX_REALIZATION.md) — full WS-14 realization
> - [WS14_DESIGN_FORMULA_EDITOR.md](WS14_DESIGN_FORMULA_EDITOR.md) — editor's
>   eventual design (this slice ships only **P1 + small P3** from §11 of that doc)
>
> **Status.** `pre_mvp_path_v1` · 2026-04-26.

---

## 1. Scope of the pre-MVP

### 1.1 In scope

- One always-mounted home shell component (`home_shell.rs`).
- Single seeded `untitled-1` formula space.
- Native `<textarea>` for input (no overlays).
- Live engine round-trip: every text change goes through
  `services/live_edit.rs::apply_live_editor_input` → `LiveOxfmlBridge` →
  reducer → state → re-render.
- A read-only result block that shows what `OxFml`'s `value_presentation`
  produced (one of: number, text, logical, error code, "Array[R × C]"
  placeholder).
- A status-foot strip with two facts: live-bridge dot (sage / amber) and
  the current `green_tree_key`.
- One end-to-end browser test that types `=SUM(1,2,3)` and asserts the
  result block shows `6`.

### 1.2 Explicitly out of scope (deferred)

| Deferred | Rationale |
|---|---|
| Save / load scenarios | Adds persistence surface; pre-MVP is ephemeral. |
| Scenario breadcrumb / dropdown | One scenario, no nav needed. |
| Multi-formula-space (left rail / tabs) | One scenario, no nav needed. |
| Mode tabs (Explore / Inspect / Workbench) | Retired in WS-14 anyway. |
| Drill-downs (formula / result) | These are WS-14 phases on top of the slice. |
| Command palette | Defer. |
| Compare-with-Excel | Defer. |
| Editor overlays — syntax color, completion popup, diagnostics, signature help, hover, bracket pair | All gated by the editor-design phases; pre-MVP ships zero overlays. |
| Workspace settings + seam status board | Defer. |
| Capability snapshot / extension surfaces | Defer. |
| Configure drawer | Defer. |
| Editor-foot metrics chip | Defer (no overlays; nothing to count yet). |
| Result drill-down cascade (`VerificationPublicationSurface`) | Defer to a later WS-14 phase. |
| Array preview (typed cells, virtualization) | Defer per [WS14_DESIGN_LARGE_ARRAY_RESULTS](WS14_DESIGN_LARGE_ARRAY_RESULTS.md); ship a minimal `Array[R × C]` placeholder for now. |

### 1.3 The pre-MVP screen

```
┌─────────────────────────────────────────────────────────────┐
│  DnaOneCalc                                                 │  titlebar (brand only, 36 px)
├─────────────────────────────────────────────────────────────┤
│                                                             │
│   formula                                                   │  caption (28 px)
│  ┌───────────────────────────────────────────────────────┐  │
│  │ =SUM(1, 2, 3)                                         │  │  textarea
│  │                                                       │  │  (3–6 lines auto-height)
│  └───────────────────────────────────────────────────────┘  │
│                                                             │
│   result                                                    │  caption (28 px)
│  ┌───────────────────────────────────────────────────────┐  │
│  │                  6                                    │  │  result block
│  │                                                       │  │  (single line, monospace large)
│  └───────────────────────────────────────────────────────┘  │
│                                                             │
├─────────────────────────────────────────────────────────────┤
│  ● live-bridge   ·   green-tree a3f91e…                    │  status-foot (22 px)
└─────────────────────────────────────────────────────────────┘
```

No drill-down chevrons, no chip strip, no scenario breadcrumb, no compare
button. Maximum simplicity; the only way to interact is to type.

---

## 2. Architectural choice — greenfield `home_shell.rs`, not strip

Two paths considered:

| Path | Pro | Con |
|---|---|---|
| **A. Greenfield `home_shell.rs`** as the minimal version of the eventual WS-14 shell. The existing `OneCalcShellApp` and mode shells stay in tree but are not mounted. | Clean. Each later WS-14 phase adds a piece on top. The eventual retire of `OneCalcShellApp` is a single later sweep. No dead-end code. | Two shells exist briefly; mount switch is binary. |
| B. Strip `OneCalcShellApp` directly: hardcode mode to a stub home, hide rail / context-bar / drawer behind feature flag. | One shell file. | Invasive; lots of callbacks and projections to neutralize; risk of breakage in code we'll delete anyway. |

**Choice: A.** This matches the WS-14 plan §14 retire list and the
realization doc's §10 file inventory (which expects `home_shell.rs` as the
single mounted shell). Each later WS-14 phase grows `home_shell.rs`; the
old shell retires once `home_shell.rs` is feature-complete.

---

## 3. Pre-flight checks (do these first)

Before writing any new code, confirm the current state is what we expect.

| # | Check | Pass criteria |
|---|---|---|
| 0.1 | `cargo build` at root builds clean. | exit 0 |
| 0.2 | `cargo test --workspace` passes. | exit 0 |
| 0.3 | `scripts/run-onecalc-preview.ps1` builds wasm and starts a server. | preview reachable at `http://127.0.0.1:<port>/` |
| 0.4 | Visit preview URL → existing three-mode shell renders. | rail + context bar + Explore body visible |
| 0.5 | Type into the existing editor → result updates. | `LiveOxfmlBridge` is live and round-tripping today |
| 0.6 | `cargo test -p dnaonecalc-host` runs the existing test corpus. | exit 0 |

If any check fails, stop and resolve before proceeding. The pre-MVP work
assumes a clean baseline.

---

## 4. The build steps

Eight small, reviewable commits. Each compiles and passes `cargo test` on
its own; each is a candidate stopping point for review.

### Step 1 — New file `home_shell.rs` skeleton

**File:** `src/dnaonecalc-host/src/ui/components/home_shell.rs` (new)

**Content:** Leptos component signature mirroring `OneCalcShellApp`:

```rust
#[component]
pub fn HomeShell(
    initial_state: OneCalcHostState,
    #[prop(default = None)] editor_bridge: Option<Arc<dyn OxfmlEditorBridge + Send + Sync>>,
) -> impl IntoView {
    let state: RwSignal<OneCalcHostState> = RwSignal::new(initial_state);
    view! {
        <ThemeStyleTag />
        <div class="onecalc-home-shell">
            <header class="onecalc-home-shell__titlebar">DnaOneCalc</header>
            <main class="onecalc-home-shell__body">
                // editor + result mount points (filled in next steps)
            </main>
            <footer class="onecalc-home-shell__statusfoot">
                // (filled in step 5)
            </footer>
        </div>
    }
}
```

**Module wiring:** add `pub mod home_shell;` to
`src/dnaonecalc-host/src/ui/components/mod.rs`.

**Exit criteria:** `cargo build` clean. Component is reachable but not yet
mounted.

### Step 2 — Minimal view-model service

**File:** `src/dnaonecalc-host/src/services/home_shell_view_model.rs` (new)

```rust
pub struct HomeShellViewModel {
    pub raw_entered_cell_text: String,
    pub editor_surface_state: EditorSurfaceState,
    pub result_view: ResultView,
    pub status: StatusView,
}

pub enum ResultView {
    Empty,                                             // entry mode Empty
    Display { text: String, kind: ResultKind },        // happy path
    Error { code: WorksheetErrorCode, surface_repr: String }, // ErrorWithMetadata
    Array { rows: usize, cols: usize, label: String }, // shape only — preview deferred
    Pending,                                           // bridge in flight
}

pub enum ResultKind { Number, Text, Logical, RichValue, Other }

pub struct StatusView {
    pub bridge_health: BridgeHealth,                   // Live | Stale
    pub truth_source: ProjectionTruthSource,
    pub green_tree_key: Option<String>,
}

pub enum BridgeHealth { Live, Stale }

pub fn build_home_shell_view_model(state: &OneCalcHostState) -> Option<HomeShellViewModel> {
    // 1. Resolve active formula space.
    // 2. Pull raw_entered_cell_text + editor_surface_state.
    // 3. Project result_view from formula_space.editor_document.value_presentation,
    //    or from latest_evaluation_summary as fallback.
    // 4. Build status view from context.truth_source + green_tree_key from snapshot.
}
```

**Module wiring:** add `pub mod home_shell_view_model;` to
`src/dnaonecalc-host/src/services/mod.rs`.

**Tests:** add unit tests in-tree:
- empty state → `ResultView::Empty`
- happy SUM → `ResultView::Display { text: "6", kind: Number }`
- malformed → `ResultView::Error { ... }` (or fallback when bridge stale)
- `=SEQUENCE(2,3)` → `ResultView::Array { rows: 2, cols: 3, label: "Array[2 × 3]" }`

**Exit criteria:** `cargo test -p dnaonecalc-host` adds 4+ green tests.

### Step 3 — Wire textarea input

**File:** `src/dnaonecalc-host/src/ui/components/home_shell.rs`

Inside the `<main>` body, mount a textarea + an inline event handler:

```rust
<section class="onecalc-home-shell__editor">
  <div class="caption">formula</div>
  <textarea
      class="onecalc-home-shell__textarea"
      spellcheck="false"
      autocomplete="off"
      autocorrect="off"
      autocapitalize="none"
      aria-label="formula editor"
      prop:value=move || view_model_signal.get().map(|vm| vm.raw_entered_cell_text).unwrap_or_default()
      on:input=move |ev| {
          let target = event_target_value(&ev);
          let event = EditorInputEvent { /* fill from target + textarea selection */ };
          if let Some(bridge) = editor_bridge.clone() {
              state.update(|s| {
                  let _ = services::live_edit::apply_live_editor_input(bridge.as_ref(), s, event);
              });
          } else {
              state.update(|s| {
                  app::reducer::apply_editor_input_to_active_formula_space(s, event);
              });
          }
      } />
</section>
```

The `services::live_edit::apply_live_editor_input` function already exists
and orchestrates: bridge call → reducer state mutation → re-eval. **No new
service code needed.**

**Notes:**
- Native browser handles arrow keys, selection, clipboard, IME — per
  [WS14_DESIGN_FORMULA_EDITOR §2 P-1..P-3](WS14_DESIGN_FORMULA_EDITOR.md#2-design-principles).
- We do *not* wire `keydown_to_command` in this slice. Pre-MVP doesn't
  need `Tab`/`F4`/`F9`/`Ctrl+Enter` chords. Native textarea behavior
  is enough.
- We do *not* implement debouncing yet. The bridge is fast for short
  formulas; debouncing is a perf optimization deferred to the editor
  design's P-3 phase.

**Exit criteria:** Visit preview, type into textarea, see state update in
devtools. (No visible result display yet.)

### Step 4 — Render result block

**File:** `src/dnaonecalc-host/src/ui/components/home_shell.rs`

Add a result section reading from `view_model.result_view`:

```rust
<section class="onecalc-home-shell__result">
  <div class="caption">result</div>
  <div class="onecalc-home-shell__result-block" data-kind=move || ...>
    {move || match vm.result_view {
        ResultView::Empty             => view! { <em class="muted">awaiting input</em> }.into_any(),
        ResultView::Pending           => view! { <em class="muted">…</em> }.into_any(),
        ResultView::Display { text, .. } => view! { <span class="value">{text}</span> }.into_any(),
        ResultView::Error { code, .. }   => view! {
            <span class="value error">{format!("#{:?}!", code)}</span>
        }.into_any(),
        ResultView::Array { rows, cols, label } => view! {
            <span class="value array">{label}</span>
        }.into_any(),
    }}
  </div>
</section>
```

**CSS:** put minimal CSS in a new
`src/dnaonecalc-host/src/ui/design_tokens/home_shell.rs` (or extend
`theme.rs`'s `ONECALC_THEME_CSS`). Reuse existing palette tokens
(parchment, surface, accent, warm).

**Exit criteria:** Type `=SUM(1,2,3)` → result block shows `6`.

### Step 5 — Status foot

**File:** `src/dnaonecalc-host/src/ui/components/home_shell.rs`

Render `view_model.status` in the footer:

```rust
<footer class="onecalc-home-shell__statusfoot">
  <span class="dot" data-health={status.bridge_health}></span>
  <span>"live-bridge"</span>
  <span class="sep">·</span>
  <span>"green-tree "{ status.green_tree_key.as_deref().unwrap_or("—") }</span>
</footer>
```

`bridge_health` is derived in the view-model: `Live` when last bridge call
succeeded and `truth_source == LiveBacked`; `Stale` otherwise (amber dot).

**Exit criteria:** Status foot renders; dot is sage when typing successful
formulas, amber when bridge errors.

### Step 6 — Single-scenario preview seed

**File:** `src/dnaonecalc-host/src/app/preview_state.rs`

Add a new function alongside the existing `preview_host_state()`:

```rust
pub fn preview_minimal_host_state() -> OneCalcHostState {
    let mut state = OneCalcHostState::default();
    let space_id = FormulaSpaceId::new("untitled-1");
    state.workspace_shell.active_formula_space_id = Some(space_id.clone());
    state.workspace_shell.open_formula_space_order.push(space_id.clone());
    state.formula_spaces.insert(FormulaSpaceState::new(space_id, ""));
    state
}
```

**Why a new function, not edit the existing one:** keeps the existing
`preview_host_state()` intact for the legacy three-mode shell and any
existing tests that depend on its shape. Once the legacy shell retires,
we'll consolidate.

**Exit criteria:** `cargo test` still passes (existing tests untouched).

### Step 7 — Switch the mount

**File:** `src/dnaonecalc-host/src/lib.rs`

Change two lines in `mount_onecalc_preview`:

```rust
// before:
let initial_state = app::preview_state::preview_host_state();
// ...
<ui::components::app_shell::OneCalcShellApp ... />

// after:
let initial_state = app::preview_state::preview_minimal_host_state();
// ...
<ui::components::home_shell::HomeShell
    initial_state=initial_state.clone()
    editor_bridge=Some(editor_bridge.clone())
/>
```

**Exit criteria:**
- `cargo build --target wasm32-unknown-unknown -p dnaonecalc-host --lib` clean.
- `scripts/run-onecalc-preview.ps1` rebuilds and serves.
- Visit preview → new minimal shell renders.
- Type `=SUM(1,2,3)` → result `6`.
- Type `=NOSUCH(1)` → result `#NAME?` (or whatever OxFml emits) in error styling.
- Type `=SEQUENCE(2,3)` → result `Array[2 × 3]`.
- Status-foot dot reflects bridge health.

### Step 8 — One end-to-end browser test

**File:** `src/dnaonecalc-host/tests/browser_home_shell.rs` (new, wasm-bindgen)

```rust
#[wasm_bindgen_test]
async fn home_shell_typing_sum_shows_result() {
    let host = mount_test_home_shell().await;
    type_text(&host, "=SUM(1,2,3)").await;
    wait_for_bridge_settle(&host).await;
    let result_text = host.query_selector(".onecalc-home-shell__result-block .value")
        .unwrap().unwrap().text_content().unwrap_or_default();
    assert_eq!(result_text.trim(), "6");
}
```

The `mount_test_home_shell` helper goes in
`src/dnaonecalc-host/src/test_support/`. It builds the same
`preview_minimal_host_state()` and mounts the component into a detached
DOM node, providing a stub or live bridge per the test's needs.

**Exit criteria:** the test runs green via
`wasm-pack test --headless --chrome` (or the equivalent invocation in
`scripts/run-onecalc-preview.ps1`'s test mode).

---

## 5. Acceptance test (the "done" check)

A reviewer should be able to run, in this order:

```
> cargo test --workspace
> scripts/run-onecalc-preview.ps1
> # (open preview URL in browser)
```

…and observe:

| # | Action | Expected |
|---|---|---|
| A.1 | Page loads | Minimal home shell visible: titlebar, formula caption, empty textarea, result caption, "awaiting input" muted, status foot |
| A.2 | Type `=SUM(1,2,3)` | Result shows `6` within ~200 ms; status-foot dot sage; green-tree key non-empty |
| A.3 | Append a stray `(` to the formula | Result still shows the last successful eval OR an error code; status-foot remains live; no crash |
| A.4 | Type `=NOSUCH(1)` | Result shows `#NAME?` in terracotta |
| A.5 | Type `=SEQUENCE(2,3)` | Result shows `Array[2 × 3]` (placeholder text only) |
| A.6 | Reload page | Empty editor; no scenario persistence (correct per scope) |
| A.7 | Resize browser | Layout stays centered; no broken overflow |
| A.8 | Tab from textarea | Focus moves predictably (no broken focus order) |
| A.9 | `cargo test --workspace` | All existing tests still pass; new home-shell tests pass |
| A.10 | Browser test | `home_shell_typing_sum_shows_result` green |

If A.1–A.10 pass, the pre-MVP is done. The next WS-14 phase can begin from
this baseline.

---

## 6. What we explicitly preserve

The pre-MVP **must not** touch any of the following — they continue working
as today and remain available to later WS-14 phases:

- `state/` and `OneCalcHostState`
- `app/reducer.rs`, `app/intents.rs`, `app/case_lifecycle.rs`
- `services/live_edit.rs`, `services/editor_session.rs`, `services/verification_bundle.rs`, `services/programmatic_testing.rs`, `services/retained_artifacts.rs`
- `services/explore_mode.rs`, `services/inspect_mode.rs`, `services/workbench_mode.rs`, `services/shell_composition.rs` (will retire later)
- `adapters/oxfml/` (bridge stays unchanged)
- `ui/editor/` (preserved per WS-14 plan §14.3 — `state.rs`, `commands.rs`, `bracket_matcher.rs`, `reference_cycle.rs`, `render_projection.rs`, `geometry.rs`, `browser_measurement.rs`)
- `ui/design_tokens/theme.rs` (palette unchanged; we may add CSS rules but keep tokens intact)
- `ui/components/app_shell.rs`, `shell_frame.rs`, `*_shell.rs`, `formula_editor_surface.rs`, `value_panel.rs` (kept until home_shell.rs is feature-complete; then retired in a sweep per WS-14 plan §14.1)

---

## 7. Risks

| ID | Risk | Mitigation |
|---|---|---|
| R-1 | `live_edit::apply_live_editor_input` expects an `EditorInputEvent` with input-kind classification; we have to construct one from the textarea event correctly. | The function signature is documented; existing `formula_editor_surface.rs` already constructs them. We can copy the construction logic (or extract it to a small helper) — not new logic. |
| R-2 | Bridge round-trip might not classify `=SEQUENCE(2,3)` shape into `value_presentation.array_preview` reliably for all inputs. | Step 4 fallback: if `array_preview.is_none()` but the result is array-shaped per `latest_evaluation_summary`, we still render `Array[...]` with the shape extracted from the summary string. Acceptance A.5 verifies. |
| R-3 | Switching the mount in `lib.rs` could break the existing `scripts/run-onecalc-preview.ps1` if any of its assumptions on DOM structure break. | The script just builds wasm and serves an HTML scaffold; no DOM-class dependencies. Verified in pre-flight. |
| R-4 | Some browser tests in `tests/browser_mounted.rs` (existing) may target the legacy three-mode shell selectors and fail when the mount switches. | Audit existing browser tests before Step 7; if any target legacy selectors, gate them behind `#[ignore]` with a comment pointing at WS-14 retirement, **not** delete. |
| R-5 | `EditorEntryMode` derivation for the entry-mode pill is wired into the existing surface but not into `home_shell.rs`. | Pre-MVP doesn't render the entry-mode pill (out of scope). Will be added in a later WS-14 phase. |
| R-6 | The current `LiveOxfmlBridge` may have non-trivial init cost on first call. | Pre-MVP accepts whatever the current cost is. Tuning is a later phase. Acceptance A.2 doesn't pin a tight latency. |
| R-7 | Two preview-state functions confuse contributors. | Add a comment header on each clearly labelling "legacy three-mode" vs "WS-14 minimal home"; consolidate when legacy retires. |

---

## 8. Follow-on roadmap (post-pre-MVP)

After the pre-MVP lands and is reviewed, the WS-14 phases land **in this
order**, each one a discrete commit set per
[WS14_DESIGN_FORMULA_EDITOR §11](WS14_DESIGN_FORMULA_EDITOR.md#11-implementation-phases)
or the realization doc's epic lanes:

1. **Editor P1 polish**: focus ring, a11y attributes, browser-test core
   invariants (T-EDIT-001..009).
2. **Editor P2**: entry-mode + commit/cancel + F2 chords. (Adds the
   entry-mode pill and dirty marker.)
3. **Editor P3**: live-bridge debouncing.
4. **Editor P4**: syntax overlay (first overlay; tests measurement model).
5. **Editor P5**: local-fallback truth-source.
6. **Editor P6**: diagnostic squiggles.
7. **Result drill P1**: cascade reading from
   `VerificationPublicationSurface` (gates on
   `SEAM-BRIDGE-PUBLICATION-SURFACE`).
8. **Formula drill P1**: walk-tree from `formula_walk`.
9. **Editor P9** (completion popup), **P10** (signature help), **P11**
   (reuse animation), **P12** (F4/F9/Ctrl+Shift+U), **P13** (function
   help hover), **P14** (cross-highlight to drill).
10. **Scenario lifecycle**: breadcrumb, save/load, `.dnascenario` JSON.
11. **Compare-with-Excel** workflow.
12. **Command palette + workspace settings + seam status board.**
13. **Array-preview** typed cells + virtualization (per
    [WS14_DESIGN_LARGE_ARRAY_RESULTS §14](WS14_DESIGN_LARGE_ARRAY_RESULTS.md#14-implementation-phases)).
14. **Retire** `OneCalcShellApp` + mode shells + `formula_editor_surface.rs`
    + `value_panel.rs` + `ui/modes/` + `ui/panels/`.
15. **Consolidate** preview-state functions; remove the legacy seed.

Each line above is a separable WS-14 phase with its own exit gate.

---

## 9. Why this is the best path

- **Real engine round-trip from day one.** No fake bridge; the `LiveOxfmlBridge`
  drives every keystroke to `OxFml`/`OxFunc` and back. This catches engine
  integration bugs before any UX layer is added.
- **No throwaway code.** `home_shell.rs` is the eventual permanent shell;
  this slice is its skeleton, not a parallel prototype.
- **Each step is reviewable.** Eight small commits, each compiling and
  testing green, allow incremental review and rollback if needed.
- **Existing services preserved.** `live_edit.rs`, `editor_session.rs`,
  `LiveOxfmlBridge`, and the entire state layer are reused. The only
  *new* code is the shell component, the view-model, and the seed.
- **Honesty preserved.** Status foot shows live vs stale truth from day one;
  no hidden degradation.
- **Tests go in early.** Browser-test infrastructure (Step 8) is
  established before any overlay work, so future overlay phases inherit a
  working test rig.
- **Aligns with WS-14 plan.** Every artifact created here is named the
  same as in the WS-14 file inventory and grows naturally into the full
  feature set.

---

## Appendix A — Reviewer's 60-second checklist

1. **Is `home_shell.rs` the only new component?** Yes — view-model and
   preview-seed are services/state additions, not new components.
2. **Does any new code mutate state outside the reducer?** No. All state
   mutations go through `services::live_edit` or the reducer.
3. **Is `LiveOxfmlBridge` instantiated exactly once at mount?** Yes —
   `bootstrap_editor_bridge` returns an `Arc` shared between component and
   any future debouncer.
4. **Are existing tests touched?** No; only added.
5. **Is the legacy shell retired in this slice?** No; it stays compiled
   in tree until the WS-14 sweep.

If those five hold, the pre-MVP plan is safe to execute.
