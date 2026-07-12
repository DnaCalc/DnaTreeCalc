# DNA OneCalc Host — View-Model Layer Reference

Status: `anchor_v1`
Date: 2026-04-14
Scope: the Rust crates and modules under `src/dnaonecalc-host/src/` that hold state, mutate it, drive the upstream libraries (`OxFml` / `OxFunc` / `OxXlPlay` / `OxReplay`), and project state into shapes the UI can consume — but **not** the Leptos rendering layer (`ui/components/`, `ui/design_tokens/`, `ui/modes/`).

This document is the **anchor** for:

1. Building a fresh test suite that pins the host's view-model contract before any UI rework.
2. Identifying which features are end-to-end implemented inside OneCalc vs. which are local facades waiting on upstream library work (the `SEAM-*` register).
3. Driving upstream library development (OxFml, OxFunc, OxXlPlay, OxReplay) by saying "this is the OneCalc surface that's ready and waiting for your data".

It is read-only documentation — the source of truth is the code. Where the code disagrees with this document, the code wins; this document is a navigation aid, not a spec.

Companion documents:

- [`SCOPE_AND_SPEC.md`](SCOPE_AND_SPEC.md) — product scope and constraints.
- [`worksets/WS-13_dna_onecalc_ux_revamp.md`](worksets/WS-13_dna_onecalc_ux_revamp.md) — UX revamp workset that depends on this layer.
- [`APP_UX_HOST_STATE_SLICING.md`](APP_UX_HOST_STATE_SLICING.md) — the older host-state slicing note this document supersedes for code-level questions.
- [`BEADS.md`](BEADS.md) — execution-state truth.

## 1. Architecture

OneCalc's host crate (`src/dnaonecalc-host`) is a layered Rust workspace where every layer reads from the layer below it and writes only to the next layer up. The layering, from bottom to top:

```
                                 UI rendering
                              (Leptos components,
                               design tokens, CSS)
                                       ▲
                                       │   reads view-model
                                       │
        ┌──────────────────────────────┴───────────────────────────────┐
        │                                                              │
        │               src/services/                                  │
        │      build_*_view_model()  +  EditorSessionService           │
        │       shell_composition / explore / inspect / workbench      │
        │       live_edit  /  retained_artifacts  /  programmatic      │
        │       verification_bundle  /  spreadsheet_xml                │
        │                                                              │
        └──────────┬─────────────────────────────────┬─────────────────┘
                   │                                 │
       reads state │                                 │ calls bridge
                   ▼                                 ▼
        ┌────────────────────┐         ┌──────────────────────────────┐
        │   src/state/       │         │   src/adapters/oxfml/        │
        │   types.rs         │         │   types / preview / live     │
        │   (OneCalcHostState│         │   OxfmlEditorBridge trait    │
        │     and children)  │         │                              │
        └────────────────────┘         └─────────────┬────────────────┘
                   ▲                                 │
                   │ mutates state                   │ delegates to
                   │                                 ▼
        ┌──────────┴──────────┐         ┌──────────────────────────────┐
        │   src/app/          │         │   oxfml_core (upstream)      │
        │   reducer.rs        │         │   oxfunc_core / value_types  │
        │   case_lifecycle.rs │         │                              │
        │   intents.rs        │         └──────────────────────────────┘
        │   host_mount.rs     │
        │   preview_state.rs  │
        └─────────────────────┘
                   ▲
                   │  intents from UI events
                   │
              user actions
```

Key invariants:

- **`OneCalcHostState` is the single source of truth.** Everything that persists across user actions lives here. The Leptos `RwSignal<OneCalcHostState>` in `OneCalcShellApp` (`ui/components/app_shell.rs`) is the only stateful container in the running app; everything else is derived.
- **The reducer is pure.** Every `apply_*` function in `src/app/reducer.rs` and `src/app/case_lifecycle.rs` takes `&mut OneCalcHostState` (and a payload) and returns a `bool` indicating whether anything changed. No I/O. No async. No side effects beyond the state mutation.
- **Services are stateless.** `services/*.rs` are stateless functions that either build view-model structs from state, or call into adapters and apply the result back to state through a thin wrapper around the reducer.
- **Adapters are the only layer that talks to upstream libraries.** All `oxfml_core` / `oxfunc_core` types are accessed through `src/adapters/oxfml/`. The rest of the codebase consumes OneCalc-local mirror types (`adapters/oxfml/types.rs`).
- **The editor model is pure logic.** `src/ui/editor/state.rs`, `commands.rs`, `bracket_matcher.rs`, `reference_cycle.rs`, `render_projection.rs` carry editor data structures and pure functions over them. They don't render anything; they don't import Leptos types.
- **The persistence layer is empty.** `src/persistence/mod.rs` is `PersistencePlaceholder` — a marker type. Workspace state lives only in memory.

## 2. Reactive flow

Three flows cover everything the user can do:

### 2.1 User types or pastes text

```
<textarea> on:input
  └─▶ on_input_event callback (UI → app)
       └─▶ state.update(|state| {
              apply_live_editor_input(bridge, state, event)         services/live_edit.rs
                ├─▶ apply_editor_input_to_active_formula_space      app/reducer.rs
                │     └─▶ apply_local_editor_text_change            (clears stale analysis)
                │           ├─ raw_entered_cell_text  := event.text
                │           ├─ editor_surface_state   := for_text_with_selection(...)
                │           ├─ editor_document        := None
                │           ├─ completion_help        := default
                │           ├─ latest_evaluation_summary := None
                │           └─ effective_display_summary := None
                │
                └─▶ refresh_active_formula_space_from_bridge        services/live_edit.rs
                      └─▶ build_live_edit_intent                    (ApplyFormulaEditIntent)
                          └─▶ EditorSessionService::handle_formula_edit_intent
                                ├─▶ bridge.apply_formula_edit       adapters/oxfml/*
                                │     (OxfmlEditorBridge trait;
                                │      Live or Preview impl)
                                │
                                └─▶ apply_editor_document           services/editor_session.rs
                                      └─▶ update_formula_space_from_editor_document
                                            ├─ editor_document       := document
                                            ├─ completion_help       := derived
                                            ├─ latest_evaluation_..  := derived
                                            ├─ effective_display_..  := derived
                                            ├─ array_preview         := derived
                                            └─ context.truth_source  := infer_truth_source
            })
```

After the closure returns, Leptos re-runs everything that subscribed to the signal. View-model builders (`services/explore_mode.rs`, `services/inspect_mode.rs`, `services/workbench_mode.rs`, `services/shell_composition.rs`) re-read the new state and produce fresh view models for the renderer.

### 2.2 User issues an editor command (Tab, Esc, F4, Ctrl+Enter, Ctrl+Space, Ctrl+Shift+U, Ctrl+Alt+I, completion popup keys)

```
<textarea> on:keydown
  └─▶ keydown_to_command(key, shift, ctrl, alt, ctx)               ui/editor/commands.rs
        │  (returns None for arrow keys, backspace, delete,
        │   plain Enter — those go to the native textarea)
        │
        └─▶ Some(EditorCommand)
              └─▶ on_command callback (UI → app)
                   └─▶ state.update(|state| {
                          apply_live_editor_command(bridge, state, cmd)   services/live_edit.rs
                            ├─▶ apply_editor_command_to_active_formula_space  app/reducer.rs
                            │     ├─▶ ToggleEditorSettingsPopover  → toggle_editor_settings_popover
                            │     ├─▶ UpdateEditorSetting          → update_editor_setting
                            │     ├─▶ ToggleConfigureDrawer        → toggle_configure_drawer
                            │     │
                            │     ├─▶ apply_completion_command            (popup nav/accept)
                            │     │     └─ for AcceptSelectedCompletion:
                            │     │        apply_editor_command(InsertText(...))
                            │     │
                            │     ├─▶ apply_live_state_command            (commit / proof / cancel /
                            │     │     │                                  expand toggle / dismiss /
                            │     │     │                                  cycle reference)
                            │     │     ├─ CycleReferenceForm
                            │     │     │   └─ reference_cycle::cycle_reference_form
                            │     │     └─ apply_local_editor_text_change (when text was rewritten)
                            │     │
                            │     └─▶ apply_editor_command(text, state, cmd)  ui/editor/commands.rs
                            │           ├─ InsertText / InsertNewline
                            │           ├─ Backspace / Delete
                            │           ├─ IndentWithSpaces / OutdentWithSpaces
                            │           ├─ MoveCaret* / ExtendSelection*
                            │           └─ CutSelection
                            │
                            └─▶ refresh_active_formula_space_from_bridge   (skipped for nav-only
                                                                            and chrome-only commands)
                       })
```

`apply_live_editor_command` skips the bridge refresh for completion navigation (`SelectPreviousCompletion`, `SelectNextCompletion`, `SelectCompletionByIndex`), live-state commands that don't change text (`CommitEntry`, `RequestProof`, `DismissCompletion`, `ToggleExpandedHeight`), and chrome toggles (`ForceShowCompletion`, `SendSelectionToInspect`, `ToggleEditorSettingsPopover`, `UpdateEditorSetting`, `ToggleConfigureDrawer`). Everything else triggers a full bridge round-trip.

### 2.3 User opens / imports a retained artifact

```
catalog row click  /  file drop  /  programmatic CLI invocation
  └─▶ on_open_retained_artifact callback
       └─▶ state.update(|state| {
              open_retained_artifact_from_catalog(state, artifact_id)    app/reducer.rs
                └─▶ open_retained_artifact_by_id                          services/retained_artifacts.rs
                      ├─ resolve catalog entry
                      ├─ inject xml extraction + gap report
                      ├─ active_formula_space_view.active_mode := open_mode_hint
                      └─ retained_artifacts.open_artifact_id := Some(...)
            })

import_verification_bundle_report_into_workspace
  └─▶ import_verification_bundle_report_json                              services/retained_artifacts.rs
        └─ parses an OxReplay verification report, populates the catalog
           with one RetainedArtifactRecord per case, sets active.

import_manual_retained_artifact_into_active_formula_space
  └─▶ import_manual_artifact_for_active_formula_space                     services/retained_artifacts.rs
        └─ injects a hand-built ManualRetainedArtifactImportRequest into
           the catalog and binds it to the active formula space.
```

`switch_active_mode` and `select_active_formula_space` are simpler — they just mutate `active_formula_space_view` / `workspace_shell` and let the next view-model rebuild observe the change.

## 3. State layer

All state types live in `src/dnaonecalc-host/src/state/types.rs`. `state/mod.rs` is a re-export. `src/dnaonecalc-host/src/domain/ids.rs` defines the typed id newtypes.

### 3.1 Top-level container

**`OneCalcHostState`** (`state/types.rs:18`)

```text
pub struct OneCalcHostState {
    pub workspace_shell: WorkspaceShellState,
    pub formula_spaces: FormulaSpaceCollectionState,
    pub active_formula_space_view: ActiveFormulaSpaceViewState,
    pub retained_artifacts: RetainedArtifactOpenState,
    pub capability_and_environment: CapabilityAndEnvironmentState,  // FACADE
    pub extension_surface: ExtensionSurfaceState,                   // FACADE
    pub global_ui_chrome: GlobalUiChromeState,
}
```

`Default` produces an empty workspace with no formula spaces; the running app boots from `preview_host_state()` (`app/preview_state.rs`) which seeds three demo scenarios. The CLI entry points in `main.rs` start from `OneCalcHostState::default()` and layer in programmatic test cases.

### 3.2 Workspace + navigation

**`WorkspaceShellState`** (`state/types.rs:42`)

```text
pub struct WorkspaceShellState {
    pub active_formula_space_id: Option<FormulaSpaceId>,
    pub open_formula_space_order: Vec<FormulaSpaceId>,
    pub pinned_formula_space_ids: BTreeSet<FormulaSpaceId>,
    pub navigation_selection: WorkspaceNavigationSelection,
}
```

**`WorkspaceNavigationSelection`** (`state/types.rs:259`) — `enum { Overview, Recent, Pinned, FormulaSpace(FormulaSpaceId) }`. Set by left-rail clicks; consumed by view-model builders.

**`ActiveFormulaSpaceViewState`** (`state/types.rs:202`)

```text
pub struct ActiveFormulaSpaceViewState {
    pub active_mode: AppMode,
    pub selected_formula_space_id: Option<FormulaSpaceId>,
}
```

**`AppMode`** (`state/types.rs:267`) — `enum { Explore, Inspect, Workbench }`. The single source of truth for "which mode is the user in".

### 3.3 Formula spaces

**`FormulaSpaceCollectionState`** (`state/types.rs:61`) — `BTreeMap<FormulaSpaceId, FormulaSpaceState>` with `insert`/`get`/`get_mut` helpers.

**`FormulaSpaceState`** (`state/types.rs:81`) — the unit of work in OneCalc.

```text
pub struct FormulaSpaceState {
    pub formula_space_id: FormulaSpaceId,
    pub raw_entered_cell_text: String,                        // live text in the editor
    pub editor_surface_state: EditorSurfaceState,             // caret + selection + completion UI
    pub editor_overlay_geometry: Option<EditorOverlayGeometrySnapshot>,  // browser-measured
    pub editor_document: Option<EditorDocument>,              // last bridge response (nullable)
    pub completion_help: CompletionHelpState,                 // proposal count + help key
    pub latest_evaluation_summary: Option<String>,            // "Number" / "Array[3×4]" / etc.   FACADE
    pub effective_display_summary: Option<String>,            // "3" / "$6.00" / etc.             FACADE
    pub context: FormulaSpaceContextState,                    // scenario / host / capability
    pub array_preview: Option<FormulaArrayPreviewState>,      // spill preview rows
    pub committed_cell_text: Option<String>,                  // last committed text
    pub proofed_cell_text: Option<String>,                    // last proved text
    pub expanded_editor: bool,                                // per-space editor height toggle
}
```

Construction: `FormulaSpaceState::new(id, text)` (`state/types.rs:96`).

Method: `live_state() -> EditorLiveState` (`state/types.rs:117`) — derives `Idle / EditingLive / ProofedScratch / Committed` from `committed_cell_text`, `proofed_cell_text`, and `raw_entered_cell_text`. This is consumed by the rail's dirty marker and the editor's live-state pill in the running UI.

**`FormulaSpaceContextState`** (`state/types.rs:161`)

```text
pub struct FormulaSpaceContextState {
    pub scenario_label: String,
    pub host_profile: String,
    pub packet_kind: String,
    pub capability_floor: String,
    pub mode_availability: String,
    pub truth_source: ProjectionTruthSource,
    pub trace_summary: Option<String>,
    pub blocked_reason: Option<String>,
}
```

Most of these fields are populated by the editor session service from bridge responses (`services/editor_session.rs::infer_truth_source`). `host_profile` and `packet_kind` are seeded from the bootstrap and currently treated as opaque strings.

**`ProjectionTruthSource`** (`state/types.rs:144`) — `enum { LiveBacked, LocalFallback }`. Determines the "truth source" badge in the rail and the scope strip's profile segment.

**`FormulaArrayPreviewState`** (`state/types.rs:188`) — `{ label, rows: Vec<Vec<String>>, truncated: bool }`. Populated by `derive_formula_presentation` for `=SEQUENCE(...)` and similar; live bridge will eventually fill richer cases.

**`CompletionHelpState`** (`state/types.rs:195`) — `{ completion_count, has_signature_help, function_help_lookup_key }`.

### 3.4 Retained artifacts (verification bundle data)

**`RetainedArtifactOpenState`** (`state/types.rs:217`)

```text
pub struct RetainedArtifactOpenState {
    pub open_artifact_id: Option<String>,
    pub catalog: BTreeMap<String, RetainedArtifactRecord>,
}
```

**`RetainedArtifactRecord`** (`state/types.rs:223`)

```text
pub struct RetainedArtifactRecord {
    pub artifact_id: String,
    pub case_id: String,
    pub formula_space_id: FormulaSpaceId,
    pub comparison_status: ProgrammaticComparisonStatus,
    pub open_mode_hint: ProgrammaticOpenModeHint,
    pub discrepancy_summary: Option<String>,
    pub bundle_report_path: Option<String>,
    pub case_output_dir: Option<String>,
    pub xml_extraction: Option<SpreadsheetXmlCellExtraction>,
    pub upstream_gap_report: Option<VerificationObservationGapReport>,
    pub oxfml_comparison_value: Option<serde_json::Value>,
    pub excel_comparison_value: Option<serde_json::Value>,
    pub value_match: Option<bool>,
    pub display_match: Option<bool>,
    pub replay_equivalent: Option<bool>,
    pub replay_mismatch_records: Vec<OxReplayMismatchRecord>,
    pub replay_explain_records: Vec<OxReplayExplainRecord>,
    pub oxfml_effective_display_summary: Option<String>,
    pub excel_effective_display_text: Option<String>,
}
```

This is the **richest part of the host state** by feature surface. The data model cleanly carries every field a verification bundle exposes. The Inspect and Workbench mode view-model builders project this into reviewable UI shapes; the consumer surfaces are partly built (Inspect comparison records render; Workbench Parity Matrix is still a flat outcome cluster, not a structured matrix).

### 3.5 UI chrome state

**`GlobalUiChromeState`** (`state/types.rs:252`)

```text
pub struct GlobalUiChromeState {
    pub editor_settings: EditorSettings,
    pub editor_settings_popover_open: bool,
    pub configure_drawer_open: bool,
}
```

`EditorSettings` is defined in `ui/editor/state.rs` (see §6.1). The popover and drawer flags are flipped by `toggle_editor_settings_popover` and `toggle_configure_drawer` reducers.

### 3.6 Facade-only state types

**`CapabilityAndEnvironmentState`** (`state/types.rs`) — currently carries `selected_diff_target: CapabilityDiffTarget`. This is the shared shell-level selection for Capability Center diffing (`workspace-baseline`, `open:<formula-space-id>`, `recent:<formula-space-id>`). Broader capability snapshot population is still pending under `SEAM-ONECALC-CAPABILITY-SNAPSHOT`.

**`ExtensionSurfaceState`** (`state/types.rs`) — currently carries `admitted_contract: FrozenExtensionHostContract`, `provider_catalog: ExtensionProviderCatalog`, `rtd_host: ExtensionRtdHostState`, and `upstream_pressure: ExtensionUpstreamPressureRegister`. The host state freezes the admitted OneCalc extension ABI v0 contract into runtime-owned truth, keeps a platform-gated native-provider loader catalog alongside it, carries the host-owned RTD lifecycle state, and now also records the unresolved upstream pressure items that the extension lane must keep explicit instead of normalizing locally. `provider_catalog` preserves the lifecycle distinction between `discovered`, `admitted`, `enabled`, `active`, `blocked-by-platform`, and `rejected-by-contract` providers. `rtd_host` preserves the declared topic/request lifecycle (`activation-pending`, `awaiting-value`, `value-ready`, `update-pending`, `provider-not-ready`, `blocked-by-platform`, `disconnected`) over the runtime-specific RTD host contract (`in-process-com`, `minimal-com-like-registry`, or `unsupported`). `upstream_pressure` currently records the OxVba-specific draft, planned, and Windows-only seams that must remain visible as pressure rather than private OneCalc ABI drift. Packaging parity proof for Windows `.xll` and Linux `.so` lives in the executable harnesses under `extensions/packaging_conformance.rs`, while the explicit OxVba pressure register lives in `extensions/upstream_pressure.rs`.

### 3.7 Identifier types

**`FormulaSpaceId`** (`domain/ids.rs:1`) — newtype around `String`. `new(value)` constructor and `as_str()` accessor. Implements `Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug`. Every map keyed by formula-space identity uses this.

## 4. App layer (intents, reducers, bootstrap)

### 4.1 Intents

`src/app/intents.rs` is small but central.

**`AppIntent`** (`intents.rs:4`) — `enum { ApplyFormulaEdit(ApplyFormulaEditIntent) }`. Currently only one variant; left open for future intent kinds.

**`ApplyFormulaEditIntent`** (`intents.rs:9`)

```text
pub struct ApplyFormulaEditIntent {
    pub formula_space_id: FormulaSpaceId,
    pub formula_stable_id: String,
    pub entered_text: String,
    pub cursor_offset: usize,
    pub analysis_stage: EditorAnalysisStage,  // SyntaxOnly | SyntaxAndBind | SyntaxBindAndEval
}
```

This is the canonical "ask the bridge to re-analyze this formula" payload. Constructed inside `services/live_edit.rs::build_live_edit_intent`.

### 4.2 Editor reducers (`app/reducer.rs`)

The full set of pure-mutation entry points. Each takes `&mut OneCalcHostState` (some take a payload) and returns `bool`.

| Function | Lines | What it does |
|---|---|---|
| `apply_editor_input_to_active_formula_space(state, event)` | 13–34 | Routes a raw `EditorInputEvent` to the active formula space. Updates `raw_entered_cell_text` + `editor_surface_state`; clears stale analysis via `apply_local_editor_text_change`. |
| `apply_editor_command_to_active_formula_space(state, command)` | 36–72 | Routes an `EditorCommand`. Delegates to UI-chrome handlers (`toggle_editor_settings_popover`, `update_editor_setting`, `toggle_configure_drawer`), then `apply_completion_command`, then `apply_live_state_command`, then `apply_editor_command` for text mutations. |
| `apply_completion_command(formula_space, command)` | 236–351 | Completion popup navigation and acceptance: `SelectPreviousCompletion`, `SelectNextCompletion`, `SelectCompletionByIndex`, `AcceptSelectedCompletion`, `AcceptCompletionByIndex`. Uses `cycle_completion_selection` for circular indexing. |
| `apply_live_state_command(formula_space, command)` | 74–148 | Live-state lifecycle: `CommitEntry`, `RequestProof`, `CancelEntry`, `ToggleExpandedHeight`, `DismissCompletion`, `CycleReferenceForm` (delegates to `reference_cycle::cycle_reference_form`). |
| `apply_editor_overlay_measurement_to_active_formula_space(state, event)` | 170–180 | Stores the browser-measured `EditorOverlayGeometrySnapshot` on the active formula space. |
| `update_editor_setting(state, update)` | 150–156 | Applies an `EditorSettingUpdate` to `global_ui_chrome.editor_settings`. |
| `toggle_editor_settings_popover(state)` | 158–162 | Flips `global_ui_chrome.editor_settings_popover_open`. |
| `toggle_configure_drawer(state)` | 164–168 | Flips `global_ui_chrome.configure_drawer_open`. |
| `open_retained_artifact_from_catalog(state, artifact_id)` | 182–187 | Calls `retained_artifacts::open_retained_artifact_by_id`. |
| `open_retained_artifact_from_catalog_in_inspect(state, artifact_id)` | 189–194 | Calls `retained_artifacts::open_retained_artifact_in_inspect_by_id`. |
| `import_manual_retained_artifact_into_active_formula_space(state, request)` | 196–201 | Calls `retained_artifacts::import_manual_artifact_for_active_formula_space`. |
| `import_verification_bundle_report_into_workspace(state, request)` | 203–208 | Calls `retained_artifacts::import_verification_bundle_report_json`. |

Internal helper: `active_formula_space_mut(state)` resolves the active formula space from `workspace_shell.active_formula_space_id` falling back to `active_formula_space_view.selected_formula_space_id`. `apply_local_editor_text_change(formula_space, text, state)` is the centralised "text changed → invalidate everything derived from text" routine.

Test coverage (in-file tests):

- `input_event_updates_raw_text_and_editor_state_for_active_formula_space`
- `input_event_preserves_selection_metadata_when_provided`
- `command_updates_editor_state_and_clears_stale_analysis`
- `commit_entry_records_committed_text_and_transitions_live_state`
- `configure_drawer_toggle_flips_global_chrome_flag`
- `editor_settings_popover_toggle_and_update_apply_to_global_chrome`
- `cycle_reference_form_rewrites_cell_and_selects_new_span`
- `dismiss_completion_clears_anchor_and_selected_index`
- `toggle_expanded_height_flips_flag`
- `overlay_measurement_event_updates_geometry_on_active_formula_space`
- `open_retained_artifact_routes_shell_to_workbench_context`
- `importing_manual_retained_artifact_routes_shell_to_workbench_context`
- `open_retained_artifact_in_inspect_routes_shell_to_inspect_context`

### 4.3 Case-lifecycle reducers (`app/case_lifecycle.rs`)

Workspace-level formula-space management. Every function is pure on `&mut OneCalcHostState`.

| Function | Lines | What it does |
|---|---|---|
| `new_formula_space(state) -> FormulaSpaceId` | 14–29 | Generates `untitled-N`, inserts an empty `FormulaSpaceState`, appends to `open_formula_space_order`, sets active. |
| `rename_formula_space(state, id, label) -> bool` | 45–62 | Updates `context.scenario_label`. Rejects empty labels. |
| `duplicate_formula_space(state, id) -> Option<FormulaSpaceId>` | 64–91 | Clones text, context, committed/proofed text, expanded toggle. Appends `(copy)` to label. |
| `close_formula_space(state, id) -> bool` | 93–127 | Removes from collection, order, pinned set. Activates next in order. **Spins a fresh untitled if the workspace would otherwise be empty** — workspace is never blank. |
| `toggle_pin_formula_space(state, id) -> bool` | 129–146 | Flips membership in `pinned_formula_space_ids`. |

Internal: `next_untitled_index(state)` scans existing ids for the highest `untitled-N` integer.

Test coverage: 8 tests covering every path including the "close last → fresh untitled" fallback and the pin toggle.

### 4.4 Bootstrap (`app/host_mount.rs`, `app/preview_state.rs`)

**`HostMountTarget`** (`host_mount.rs:11`) — `enum { DesktopTauri, WebBrowser }`. Used by `bootstrap_spec` and `bootstrap_editor_bridge` to select the appropriate engine / mount path.

**`HostBootstrapSpec`** (`host_mount.rs:17`) — `{ target, mount_element_id: "onecalc-root", document_title: "DNA OneCalc" }`.

**`bootstrap_editor_bridge(target) -> Arc<dyn OxfmlEditorBridge>`** (`host_mount.rs:32`) — returns `LiveOxfmlBridge` for both desktop and web mounts. The preview host still seeds demo state, but the running bridge is always the real OxFml/OxFunc path.

**`render_shell_html(target, state) -> String`** (`host_mount.rs:43`) and **`render_shell_document(...)`** (`host_mount.rs:60`) — render the Leptos `OneCalcShellApp` to HTML for SSR / static documents.

**`preview_host_state() -> OneCalcHostState`** (`preview_state.rs:17`) — populates state with three demo scenarios (success SUM, diagnostic, array SEQUENCE) plus a retained-artifact catalog with three demo cases (matched, mismatched, blocked). This is what the running preview server boots from.

`OneCalcHostApp` (`app/mod.rs:9`) is the headless wrapper used by the CLI entry points in `main.rs`. Holds an `OneCalcHostState` and exposes `state()` and `launch_message()` accessors.

## 5. Services layer

Every file under `src/dnaonecalc-host/src/services/` is a stateless module that either projects state into a view model, or wraps the reducer with bridge-aware behaviour.

### 5.1 `services/explore_mode.rs` — Explore view model

Builds the view-model the Explore mode renders.

**`ExploreViewModel`** (`explore_mode.rs:14`) — large flat struct with every field the Explore screen could need: scenario / truth / host / capability strings, `raw_entered_cell_text`, `editor_surface_state`, `overlay_geometry`, `syntax_runs`, `diagnostics`, `completion_count`, `completion_items`, `signature_help`, `function_help`, `result_value_summary`, `effective_display_summary`, `latest_evaluation_summary`, `array_preview`, `green_tree_key`, `reused_green_tree`, `entry_mode`, `live_state`, `expanded_editor`, `editor_settings`, `editor_settings_popover_open`, `configure_drawer_open`.

**Sub-types**: `ExploreArrayPreviewView`, `ExploreDiagnosticView`, `ExploreCompletionItemView`, `ExploreCompletionKindView` (`enum`), `ExploreSignatureHelpView`, `ExploreFunctionHelpView`, `ExploreFunctionHelpSignatureView`. All are flat data structures designed for direct rendering.

**`build_explore_view_model(formula_space, editor_settings, popover_open, drawer_open) -> ExploreViewModel`** (`explore_mode.rs:110`) is the projection function. Logic:

1. If `editor_document.source_text == raw_entered_cell_text`, project from the document (syntax runs from snapshot, diagnostics from `live_diagnostics`, completions, signature help, function help, walk reuse summary).
2. Otherwise call `fallback_projection` which uses `syntax_runs_from_text` (local tokenizer) and empty everything else.
3. Fill context, display/evaluation summaries, array preview from formula-space fields.
4. Auto-select the first completion proposal when the list is non-empty and nothing was selected.
5. Stamp `entry_mode = EditorEntryMode::classify(text)` and `live_state = formula_space.live_state()`.

Tests: `explore_view_model_projects_editor_document_and_help_state`, `explore_view_model_falls_back_to_local_tokenization_when_document_is_stale`.

### 5.2 `services/inspect_mode.rs` — Inspect view model

**`InspectViewModel`** (`inspect_mode.rs:11`) — context strings, `inspect_result_summary`, `green_tree_key`, `formula_walk_nodes: Vec<InspectFormulaWalkNodeView>`, parse/bind/eval/provenance summaries, `retained_artifact_context: Option<InspectRetainedArtifactContextView>`.

**Sub-types**:

- `InspectFormulaWalkNodeView` — recursive `{ node_id, label, value_preview, state: FormulaWalkNodeState, children }`.
- `InspectRetainedArtifactContextView` — full retained-artifact projection: `artifact_id`, `case_id`, `comparison_status`, `value_match / display_match / replay_equivalent`, `discrepancy_summary`, `bundle_report_path`, `xml_source_summary`, `display_comparison_summary`, `upstream_gap_summary: Vec<String>`, `comparison_records / explain_records: Vec<...>`.
- `InspectComparisonRecordView`, `InspectExplainRecordView` — projected from `OxReplayMismatchRecord` and `OxReplayExplainRecord` with derived family/status/severity labels.

**`build_inspect_view_model(formula_space, retained_artifact) -> InspectViewModel`** (`inspect_mode.rs:85`) projects the document + retained artifact and labels every replay record with `replay_family_label` / `replay_status_label` / `replay_record_summary` / `default_record_severity`.

Tests: `inspect_view_model_projects_walk_and_summary_state`, `inspect_view_model_projects_open_retained_artifact_context`.

### 5.3 `services/workbench_mode.rs` — Workbench view model

**`WorkbenchViewModel`** (`workbench_mode.rs:7`) — context strings, `outcome_summary`, `evidence_summary`, `lineage_items: Vec<String>`, `action_items: Vec<String>`, `recommended_action`, retained-artifact verdicts (`value_match / display_match / replay_equivalent`), `retained_discrepancy_summary`, `imported_bundle_summary`, `xml_source_summary`, `display_comparison_summary`, `upstream_gap_summary`, `comparison_records`, `explain_records`, `retained_catalog_items`.

**Sub-types**: `WorkbenchComparisonRecordView`, `WorkbenchExplainRecordView` (identical to the Inspect equivalents), `WorkbenchRetainedCatalogItemView`.

**`build_workbench_view_model(formula_space, retained_artifact, retained_catalog) -> WorkbenchViewModel`** (`workbench_mode.rs:73`) synthesizes the lineage timeline, action items, and recommended-next-step text. The inputs are rich; the projection so far flattens them into mostly-string outcomes — Area 4.2 of WS-13 calls for replacing this with a structured Parity Matrix.

Tests: `workbench_view_model_projects_outcome_and_evidence_summary`, `workbench_view_model_prefers_open_retained_discrepancy_artifact`, `workbench_view_model_prefers_per_family_gap_and_display_summaries`.

### 5.4 `services/shell_composition.rs` — Shell frame view model

The biggest service module by surface area. Holds all the cross-mode chrome view-model types and the active-mode router.

**`ActiveModeProjection`** (`shell_composition.rs:9`) — `enum { Explore(ExploreViewModel), Inspect(InspectViewModel), Workbench(WorkbenchViewModel) }`.

**`ShellFrameViewModel`** (`shell_composition.rs:66`) — the top-level shell view model: `active_mode`, `active_formula_space_label`, breadcrumb, scope strip, mode tabs, formula spaces list, context facts, footer facts, workspace summary.

**Sub-types**:

- `ShellModeTabViewModel` — `{ mode, label, is_active }`.
- `ShellFormulaSpaceListItemViewModel` — `{ formula_space_id, label, truth_source_label, packet_kind_summary, is_active, is_pinned, is_dirty, section: ShellRailSection, retained_verdicts: Option<...> }`. The `is_dirty` flag is derived from `formula_space.live_state()`.
- `ShellRailSection` — `enum { Pinned, Open }` with `slug()` and `label()`.
- `ShellRetainedVerdictsViewModel` — `{ value_match, display_match, replay_equivalent, comparison_lane_label }`.
- `ShellBreadcrumbViewModel` — `{ workspace_label, space_label, mode_label }`.
- `ShellScopeSegmentViewModel` — `{ slug, label, value, status: ShellScopeSegmentStatus }`.
- `ShellScopeSegmentStatus` — `enum { Live, NotImplemented { seam_id: &'static str } }`. **This is how the shell carries facade markers visibly into the rendered chrome.**
- `ShellChromeFactViewModel` — `{ label, value, tone }` for both context and footer facts.

**Functions**:

- `mode_accent_slug(mode)` (`:121`) — maps `AppMode` to `"explore" | "inspect" | "workbench"` for CSS theming.
- `active_formula_space(state)` (`:136`) — resolves the active formula space.
- `build_active_mode_projection(state)` (`:148`) — picks one of the three mode builders, threads in retained-artifact context for Inspect / Workbench when the active artifact's `formula_space_id` matches.
- `build_shell_frame_view_model(state)` (`:171`) — builds the entire frame view model. Composes the breadcrumb, the five-segment scope strip (Locale / Date / Profile / Policy / Format) marking facade segments with their `SEAM-*` ids, the mode tabs, the formula-space list with sectioning and dirty markers and retained-verdict badges, the workspace summary, and the context/footer fact rows.
- `switch_active_mode(state, next_mode)` (`:379`) — mutates `active_formula_space_view.active_mode`.
- `select_active_formula_space(state, id)` (`:383`) — sets both `workspace_shell.active_formula_space_id` and `active_formula_space_view.selected_formula_space_id`.

Tests: 11 covering every routing path and the formula-space list projection.

### 5.5 `services/live_edit.rs` — Bridge-aware editor reducer wrapper

**`LiveEditError`** (`live_edit.rs:11`) — `enum { NoActiveFormulaSpace, UnknownFormulaSpace(FormulaSpaceId), Session(EditorSessionError), Bridge(OxfmlEditorBridgeError) }`.

**`apply_live_editor_input(bridge, state, input_event) -> Result<bool, LiveEditError>`** (`live_edit.rs:19`) — calls the editor reducer, then refreshes from the bridge.

**`apply_live_editor_command(bridge, state, command) -> Result<bool, LiveEditError>`** (`live_edit.rs:34`) — calls the editor reducer, skips the bridge refresh for commands that are pure UI / chrome / completion-navigation, otherwise refreshes from the bridge.

Internal: `refresh_active_formula_space_from_bridge` builds an `ApplyFormulaEditIntent` via `build_live_edit_intent` and hands it to `EditorSessionService::handle_formula_edit_intent`.

Tests: `live_input_refreshes_active_formula_space_through_bridge`, `live_command_refreshes_after_local_command_path`, `live_completion_navigation_stays_local_without_bridge_refresh`, `live_caret_movement_refreshes_signature_help_argument_index`.

### 5.6 `services/editor_session.rs` — Bridge call + document application

**`EditorSessionService`** (`editor_session.rs:13`) — stateless. Two methods:

1. `handle_formula_edit_intent(bridge, formula_spaces, intent)` — calls `bridge.apply_formula_edit(request)`, then `apply_editor_document(...)`.
2. `apply_editor_document(formula_spaces, formula_space_id, document)` — the heart of the bridge response handler.

**`EditorSessionError`** (`editor_session.rs:294`) — `enum { UnknownFormulaSpace(FormulaSpaceId), Bridge(OxfmlEditorBridgeError) }`.

The internal `update_formula_space_from_editor_document` function does everything you'd expect: stores the document, derives presentation summaries via `derive_formula_presentation`, sets the truth source via `infer_truth_source`, resets `editor_surface_state`, and auto-selects the first completion proposal when the list is non-empty.

`derive_formula_presentation` is **the single most important facade in OneCalc** for the formula-result path. Its current logic:

1. If the document carries `value_presentation`, project from it.
2. If provenance contains a blocked reason, return a blocked summary.
3. If a diagnostic exists, return a diagnostic summary.
4. If the text starts with `'`, return text-mode summary.
5. If the text parses as a number, return a number summary.
6. Otherwise `Unevaluated`.

Steps 6, 7, and 8 are **explicit local fakes** that exist only to make the preview look alive while the real bridge isn't routing `EvalValue` through. They are scheduled for removal once `SEAM-ONECALC-EXTENDED-VALUE-ROUTING` lands and `value_presentation` carries real data for every formula.

Tests: 6 covering document application, derivation paths (sum / sequence), bridge round-trip, truth-source inference for live and preview.

### 5.7 `services/programmatic_testing.rs` — Verification config + catalog

Configuration types for programmatic test orchestration. Used by the CLI entry points (`verify-formula`, `verify-xml-cell`, `verify-batch`) and by `preview_state.rs` when seeding the demo retained-artifact catalog.

Types: `ProgrammaticSpreadsheetXmlSource`, `ProgrammaticFormulaCase`, `ProgrammaticHostProfile`, `ProgrammaticCapabilityProfile`, `ProgrammaticVerificationConfig`, `ProgrammaticComparisonLane` (`enum { OxfmlOnly, OxfmlAndExcel, ExcelObservationBlocked }`), `ProgrammaticBatchPlan`, `ProgrammaticComparisonStatus` (`enum { Matched, Mismatched, Blocked }`), `ProgrammaticOpenModeHint` (`enum { Inspect, Workbench }`), `ProgrammaticArtifactCatalogEntry`.

Functions: `default_windows_excel_host_profile`, `default_windows_excel_capability_profile`, `default_verification_config`, `load_verification_config_xml(path)`, `build_programmatic_batch_plan(cases, profile, capabilities)`, `build_programmatic_artifact_catalog_entry(artifact_id, case_id, status)`.

### 5.8 `services/verification_bundle.rs` — OxReplay verification report parsing

Carries the rich data shapes from the upstream verification bundle JSON.

Key types: `OxReplayMismatchRecord`, `OxReplayExplainRecord`, `VerificationCaseReport`, `OxfmlVerificationSummary`, `ExcelObservationSummary`, `VerificationObservationGapReport`. These match the upstream contract closely.

Helpers: `display_comparison_summary(display_match, oxfml_summary, excel_summary)`, `replay_projection_coverage_gap_summaries(records)`.

This is the **one place in OneCalc where the full Excel ↔ OxFml comparison contract is faithfully represented**. It's the ground truth for the Workbench Parity Matrix view-model work.

### 5.9 `services/retained_artifacts.rs` — Catalog management

Imports and opens for the retained-artifact catalog.

- `import_programmatic_artifact(state, request)` — adds an artifact to the catalog and routes the shell to the hint mode.
- `open_retained_artifact_by_id(state, id)` — opens an artifact and switches to Workbench (or whichever mode the hint says).
- `open_retained_artifact_in_inspect_by_id(state, id)` — opens an artifact and switches to Inspect.
- `import_manual_artifact_for_active_formula_space(state, request)` — adds a hand-built artifact to the active formula space.
- `import_verification_bundle_report_json(state, request)` — parses an entire OxReplay verification report (multiple cases) into the catalog.

### 5.10 `services/spreadsheet_xml.rs` — XML cell extraction

`SpreadsheetXmlCellExtraction` carries `workbook_path`, `cell_locator`, format/style/data type, and any conditional formatting from a SpreadsheetML file. `VerificationObservationScope` enumerates the surfaces and views that OxFml / OxXlPlay / OxReplay are expected to observe for a given case.

## 6. Editor model layer (pure logic, no rendering)

`src/dnaonecalc-host/src/ui/editor/` carries the editor's data structures and pure-function operations over them. None of these files import Leptos types or touch the DOM.

### 6.1 `ui/editor/state.rs` — Editor state types

**`EditorEntryMode`** (`state.rs:1`) — `enum { Formula, Value, Text, Empty }`. Method `classify(text)` returns the mode based on leading character (`=` → Formula, `'` → Text, non-empty → Value, empty → Empty). `label()` and `slug()` for rendering.

**`EditorLiveState`** (`state.rs:67`) — `enum { Idle, EditingLive, ProofedScratch, Committed }`. Methods `label()`, `slug()`, `glyph()`. Computed by `FormulaSpaceState::live_state()`.

**`CompletionAggressiveness`** (`state.rs:7`) — `enum { Manual, OnIdentifier, Always }`.

**`HelpPlacement`** (`state.rs:33`) — `enum { Inline, Sidecar }`.

**`EditorSettings`** (`state.rs:48`)

```text
pub struct EditorSettings {
    pub auto_close_brackets: bool,
    pub highlight_bracket_pairs: bool,
    pub completion_aggressiveness: CompletionAggressiveness,
    pub help_placement: HelpPlacement,
    pub reuse_timing_badge_visible: bool,
    pub reduce_motion: bool,
    pub auto_proof_quiet_interval_ms: Option<u32>,
}
```

`EditorSettings::apply(EditorSettingUpdate)` is the only mutation entry point.

**`EditorSettingUpdate`** — enum with one variant per setting (`ToggleAutoCloseBrackets`, `ToggleHighlightBracketPairs`, `SetCompletionAggressiveness(...)`, etc.).

**`EditorCaret`** — `{ offset: usize }`.

**`EditorSelection`** (`state.rs:204`) — `{ anchor, focus }`. Methods `collapsed(offset)`, `start()`, `end()`, `is_collapsed()`.

**`EditorScrollWindow`** — `{ first_visible_line, visible_line_count }`.

**`EditorSurfaceState`** (`state.rs:237`)

```text
pub struct EditorSurfaceState {
    pub caret: EditorCaret,
    pub selection: EditorSelection,
    pub scroll_window: EditorScrollWindow,
    pub completion_anchor_offset: Option<usize>,
    pub completion_selected_index: Option<usize>,
    pub signature_help_anchor_offset: Option<usize>,
}
```

Constructors: `for_text(text)` (caret at end, no completion anchor) and `for_text_with_selection(text, anchor, focus)`.

### 6.2 `ui/editor/commands.rs` — Editor commands and keymap

**`EditorCommand`** (`commands.rs:4`) — the single enum that names every action the editor can perform. Variants:

| Group | Variants |
|---|---|
| Text mutation | `InsertText(String)`, `InsertNewline`, `Backspace`, `Delete`, `IndentWithSpaces`, `OutdentWithSpaces`, `CutSelection` |
| Caret/selection movement | `MoveCaretLeft`, `MoveCaretRight`, `ExtendSelectionLeft`, `ExtendSelectionRight` |
| Completion | `SelectPreviousCompletion`, `SelectNextCompletion`, `SelectCompletionByIndex(usize)`, `AcceptSelectedCompletion`, `AcceptCompletionByIndex(usize)`, `ForceShowCompletion`, `DismissCompletion` |
| Live state | `CommitEntry`, `CancelEntry`, `RequestProof` |
| Reference manipulation | `CycleReferenceForm` |
| Chrome / cross-mode | `ToggleExpandedHeight`, `SendSelectionToInspect`, `ToggleEditorSettingsPopover`, `UpdateEditorSetting(EditorSettingUpdate)`, `ToggleConfigureDrawer` |

**`EditorKeyContext`** (`commands.rs:34`) — `{ completion_active: bool }`. Single contextual flag `keydown_to_command` reads.

**`EditorInputKind`** — `enum { InsertText, DeleteBackward, DeleteForward, InsertFromPaste, Other }`.

**`EditorInputEvent`** — `{ text, selection_start, selection_end, input_kind, inserted_text }`.

**`EditorCommandResult`** — `{ text: String, state: EditorSurfaceState }`.

**Functions**:

- `apply_editor_command(text, state, command) -> EditorCommandResult` (`:63`) — pure function that applies an `EditorCommand` to a `(text, state)` pair, returning a new pair. Routes text mutations to the appropriate inner helper (`insert_text`, `cut_selection`, `backspace`, `delete`, `indent_with_spaces`, `outdent_with_spaces`).
- `keydown_to_command(key, shift, shortcut, alt, context) -> Option<EditorCommand>` (`:157`) — maps a keydown event to a command **only when the editor legitimately re-owns that key**. Returns `None` for arrow keys, Backspace, Delete, plain Enter, plain character entry — those go to the native textarea.
- `classify_dom_input(input_type) -> EditorInputKind` (`:198`) — classifies a DOM `InputEvent.inputType` string.
- `cycle_completion_selection(current, count, delta) -> Option<usize>` (`:208`) — circular indexing for completion list navigation.

The keymap is the **single source of truth for "which keys does the editor own"**. Reading `keydown_to_command` is the fastest way to understand the editor's intercept policy.

### 6.3 `ui/editor/bracket_matcher.rs` — Bracket pair matching

**`BracketPairHighlight`** — `{ open_offset, close_offset }`.

**`bracket_pair_for_caret(text, caret_offset) -> Option<BracketPairHighlight>`** (`bracket_matcher.rs:13`) — finds the matching bracket pair at or before the caret, ignoring brackets inside string literals, handling nesting and mixed bracket types `()`, `[]`, `{}`.

Internal helpers: `is_inside_string_literal`, `bracket_at`, `is_open_bracket`, `is_close_bracket`, `matches_pair`, `find_matching_close`, `find_matching_open`. Pure character-level scanning.

Tests cover: matching at and around the caret, nested brackets, mixed bracket kinds, string literal exclusion, unbalanced cases.

### 6.4 `ui/editor/reference_cycle.rs` — F4 reference form cycling

**`ReferenceCycleResult`** — `{ text: String, reference_start: usize, reference_end: usize }`.

**`cycle_reference_form(text, selection_start, selection_end) -> Option<ReferenceCycleResult>`** (`reference_cycle.rs:31`) — detects an A1-style reference at or spanning the selection and returns the next form in the cycle `A1 → $A$1 → A$1 → $A1 → A1`. Returns the rewritten text and the new span so the caller can re-select.

Internal: `find_reference_for_range`, `try_read_reference`, `is_reference_char`, `is_reference_continuation`, `next_form`, `rebuild_text`. All pure.

Tests cover all four cycle steps, references inside parens, multi-cell references in `=SUM(A1:B2)`, defined-name-shaped tokens being skipped, and the Excel column cap (max 3 letters).

### 6.5 `ui/editor/render_projection.rs` — Syntax run projection

**`SyntaxTokenRole`** — `enum { Operator, Function, Number, Delimiter, Identifier, Text }`.

**`SyntaxRun`** — `{ text, span_start, span_len, role }`.

**`syntax_runs_from_snapshot(snapshot) -> Vec<SyntaxRun>`** (`render_projection.rs:21`) — projects from a bridge `EditorSyntaxSnapshot` (preferred path).

**`syntax_runs_from_text(text) -> Vec<SyntaxRun>`** (`render_projection.rs:34`) — local fallback tokenizer when the bridge document is stale or absent. Splits on whitespace and operator characters. Heuristic role classification via `classify_token_role`.

### 6.6 `ui/editor/geometry.rs` — Overlay geometry

Pixel measurement and projection types for the editor overlay layer. All pure.

Types: `EditorOverlayMeasurementSource` (`enum { DerivedGrid, DomMeasured }`), `EditorLineColumn`, `EditorOverlayBox`, `EditorOverlayMeasurement`, `EditorMeasuredOverlayBox`, `EditorOverlayGeometrySnapshot`, `EditorOverlayMeasurementEvent`, `TextareaMeasurementMetrics`.

Functions: `offset_to_line_column(text, offset)`, `resolve_overlay_box(measured, derived)`, `derive_overlay_snapshot(...)`, `derive_overlay_snapshot_with_metrics(...)`. The `EditorOverlayMeasurement::derived_grid()` constant is the local fallback (8px char width, 22px line height).

### 6.7 `ui/editor/browser_measurement.rs` — Browser textarea measurement

`#[cfg(target_arch = "wasm32")]` only. `capture_overlay_measurement_event(textarea, editor)` reads `textarea.rows / cols / clientHeight / clientWidth / scrollTop / scrollLeft`, computes character width and line height, and produces an `EditorOverlayMeasurementEvent`.

A non-wasm fallback is provided that calls `derive_overlay_snapshot` with the static grid metrics so SSR rendering still produces measurement events.

## 7. Adapters layer — boundary to upstream libraries

`src/dnaonecalc-host/src/adapters/` is the only place OneCalc imports from `oxfml_core`, `oxfunc_core`, etc. Everything else consumes OneCalc-local mirror types defined here.

### 7.1 `adapters/oxfml/types.rs` — OneCalc-local mirror of OxFml editor types

This is the **most important file in the adapter layer**. Every type here is a OneCalc-local copy of an upstream OxFml type, deliberately decoupled so the rest of OneCalc never has to know about feature flags or OxFml versioning.

Span / range types:

- `FormulaTextChangeRange { start, old_len, new_len }`
- `FormulaTextSpan { start, len }`

Syntax / diagnostics:

- `EditorToken { text, span_start, span_len }`
- `EditorSyntaxSnapshot { formula_stable_id, green_tree_key, tokens: Vec<EditorToken> }`
- `LiveDiagnostic { diagnostic_id, message, span_start, span_len }`
- `LiveDiagnosticSnapshot { diagnostics: Vec<LiveDiagnostic> }`
- `FormulaEditReuseSummary { reused_green_tree, reused_red_projection, reused_bound_formula }`

Completion / help:

- `CompletionProposalKind` — `enum { Function, DefinedName, TableName, TableColumn, StructuredSelector, SyntaxAssist }`
- `CompletionProposal { proposal_id, proposal_kind, display_text, insert_text, replacement_span, documentation_ref, requires_revalidation }`
- `SignatureHelpContext { callee_text, call_span, active_argument_index }`
- `FunctionHelpSignatureForm { display_signature, min_arity, max_arity }`
- `FunctionHelpPacket { lookup_key, display_name, signature_forms, argument_help, short_description, availability_summary, deferred_or_profile_limited }`

Walk / summaries:

- `FormulaWalkNodeState` — `enum { Evaluated, Bound, Opaque, Blocked }`
- `FormulaWalkNode { node_id, label, value_preview, state, children: Vec<FormulaWalkNode> }`
- `ParseSummary { status, token_count }`
- `BindSummary { variable_count, reference_count }`
- `EvalSummary { step_count, duration_text }`
- `ProvenanceSummary { profile_summary, blocked_reason }`

Result presentation:

- `FormulaArrayPreview { label, rows: Vec<Vec<String>>, truncated }`
- **`FormulaValuePresentation { evaluation_summary, effective_display_summary, array_preview, blocked_reason }`** (`types.rs:142`) — **FACADE**. This is OneCalc's local stand-in for the upstream presentation hint. Its purpose is to give `editor_session.rs` a place to drop derived presentation data while we wait for the real `oxfunc_value_types::ExtendedValue` / `PresentationHint` plumbing to arrive in OneCalc state.

The composite document:

**`EditorDocument`** (`types.rs:150`)

```text
pub struct EditorDocument {
    pub source_text: String,
    pub text_change_range: Option<FormulaTextChangeRange>,
    pub editor_syntax_snapshot: EditorSyntaxSnapshot,
    pub live_diagnostics: LiveDiagnosticSnapshot,
    pub reuse_summary: FormulaEditReuseSummary,
    pub signature_help: Option<SignatureHelpContext>,
    pub function_help: Option<FunctionHelpPacket>,
    pub completion_proposals: Vec<CompletionProposal>,
    pub formula_walk: Vec<FormulaWalkNode>,
    pub parse_summary: Option<ParseSummary>,
    pub bind_summary: Option<BindSummary>,
    pub eval_summary: Option<EvalSummary>,
    pub provenance_summary: Option<ProvenanceSummary>,
    pub value_presentation: Option<FormulaValuePresentation>,
}
```

Method `green_tree_key() -> &str` is the only behaviour. Everything else is data.

### 7.2 `adapters/oxfml/intent.rs` — Bridge request / result / trait

**`EditorAnalysisStage`** — `enum { SyntaxOnly, SyntaxAndBind, SyntaxBindAndEval }`. Currently the live edit path always uses `SyntaxAndBind`.

**`FormulaEditRequest`** — `{ formula_stable_id, entered_text, cursor_offset, previous_green_tree_key, analysis_stage }`.

**`FormulaEditResult`** — `{ document: EditorDocument }`.

**`OxfmlEditorBridgeError`** — `enum`, bridge errors.

**`OxfmlEditorBridge`** — the trait every bridge implements. Single method:

```text
fn apply_formula_edit(&self, request: FormulaEditRequest)
    -> Result<FormulaEditResult, OxfmlEditorBridgeError>;
```

This is the **single point of contact** between OneCalc and any downstream formula evaluator.

### 7.3 `adapters/oxfml/live_bridge.rs` — Real OxFml integration

**`LiveOxfmlBridge`** — implements `OxfmlEditorBridge` by delegating to the real `oxfml_core` library. This is the only place in OneCalc where `oxfml_core::*` types are imported.

Deterministic bridge stand-ins now live only in tests as local fakes or fixture documents from `test_support/mod.rs` (`sample_editor_document`, `diagnostic_editor_document`, `array_editor_document`, `blocked_editor_document`).

## 8. Panel view-models (UI-adjacent but rendering-free)

`src/dnaonecalc-host/src/ui/panels/` carries cluster view-models and builders that group the mode view-model fields into rendering-friendly chunks. They do not import Leptos.

### 8.1 `ui/panels/explore.rs`

**`ExploreEditorClusterViewModel`** — mirror of `ExploreViewModel` editor-relevant fields plus `selected_completion_proposal_id`, `selected_completion_item`, `help_sync_lookup_key`, `completion_anchor_span`, `bracket_pair`.

**`ExploreResultClusterViewModel`** — `{ result_value_summary, effective_display_summary, latest_evaluation_summary, array_preview, value_panel: ValuePanelViewModel }`.

**Functions**: `build_explore_editor_cluster(view_model)`, `build_explore_result_cluster(view_model)`. Both pure projections that consume an `ExploreViewModel` and emit a cluster.

### 8.2 `ui/panels/inspect.rs` and `ui/panels/workbench.rs`

Mirror the inspect / workbench mode view models into cluster shapes. Same pattern: cluster view-models + builder functions.

### 8.3 `ui/panels/value_panel_model.rs` — **FACADE**

Local mirror of the upstream value type system. Roughly 500 lines that re-implement the `oxfunc_value_types` shape entirely in OneCalc-local types so the UI layer can be feature-flag-independent.

Types: `ValuePanelValue` (large enum covering `Number`, `Text`, `Logical`, `Error`, `Array`, `Reference`, `Lambda`, `RichValue`, `Unevaluated`), `ValuePanelKeyValue`, `ValuePanelErrorSurface`, `ValuePanelArrayShape`, `ValuePanelArrayCell`, `ValuePanelReferenceKind`, `ValuePanelRichValue`, `ValuePanelRichValueData`, `ValuePanelRichKvp`, `ValuePanelPresentation`, `ValuePanelNumberFormatHint`, `ValuePanelStyleHint`, `ValuePanelViewModel`, `ValuePanelPipelineStep`, `ValuePanelPipelineStepStatus`, `ValuePanelProvenance`.

Function: `build_value_panel_from_explore_strings(result_summary, display_summary, green_tree_key, live_state) -> ValuePanelViewModel` — assembles a panel view model from string-only summaries, with every engine-dependent pipeline step marked `NotImplemented { seam_id }`.

This entire module is deliberately a facade: it gives the UI the structural shape it needs while the upstream `ExtendedValue` routing is pending. When `SEAM-ONECALC-EXTENDED-VALUE-ROUTING` lands, this module's builders gain a real input source instead of strings; the type definitions can stay unchanged.

## 9. Persistence and test support

### 9.1 `persistence/mod.rs` — **FACADE**

The entire content of this file is `pub struct PersistencePlaceholder;`. Workspace state is in-memory only. Reopening a session loses everything.

### 9.2 `test_support/mod.rs`

Factory helpers used by the in-tree tests:

- `sample_editor_document(source_text)` — generic bridge response with diagnostics, signature help, completion.
- `sample_editor_document_with_green_key(source_text, key)` — parameterised green-tree key for incremental-reuse tests.
- `diagnostic_editor_document(source_text)` — a "missing trailing argument" diagnostic case.
- `array_editor_document(source_text)` — `SEQUENCE` walk with array preview.
- `blocked_editor_document(source_text)` — XLOOKUP blocked on host capability.

These factories are the closest thing OneCalc has today to a fixture corpus.

## 10. Implementation matrix — what actually works vs what is a facade

The matrix below is the load-bearing section of this document. It tells you, for each user-visible feature, whether the host has end-to-end implementation, partial implementation gated on a single seam, or only a local facade waiting for upstream work.

Legend: 🟢 LIVE = end-to-end through a real bridge or pure logic with no facade. 🟡 PARTIAL = OneCalc has the data shape and the wiring but the bridge / upstream library only delivers a subset. 🔴 FACADE = OneCalc has the type and the surface; data is fabricated locally or absent. Seam ids reference WS-13 Appendix B (`docs/worksets/WS-13_dna_onecalc_ux_revamp.md`).

### 10.1 Editor model — keystroke and command processing

| Feature | Status | Notes |
|---|---|---|
| Insert text via input event | 🟢 | Fully through `apply_editor_input_to_active_formula_space` + `live_edit` bridge refresh. |
| Tab indent / Shift+Tab outdent | 🟢 | Pure logic in `apply_editor_command` → `indent_with_spaces` / `outdent_with_spaces`. |
| Backspace / Delete | 🟢 | Model logic exists in `commands.rs`; today routed via native textarea per the keymap. |
| MoveCaretLeft/Right, ExtendSelectionLeft/Right | 🟢 | Model logic exists; today routed via native textarea per the keymap. |
| InsertNewline | 🟢 | Native textarea handles it (keymap returns `None` for plain Enter); model variant exists for programmatic use. |
| CutSelection | 🟢 | Pure logic; `on:cut` ClipboardEvent path. |
| CycleReferenceForm (F4) | 🟢 | `reference_cycle::cycle_reference_form` is a pure function. |
| Bracket pair matching | 🟢 | `bracket_matcher::bracket_pair_for_caret`. |
| Local syntax tokenizer fallback | 🟢 | `render_projection::syntax_runs_from_text`. |
| Editor settings storage | 🟢 | `EditorSettings` + reducer. View consumers wired. |
| `EditorEntryMode::classify` | 🟢 | Pure classification. |
| `EditorLiveState` derivation | 🟢 | `FormulaSpaceState::live_state` derives from committed/proofed/current text. |

### 10.2 Editor model — bridge-dependent commands

| Feature | Status | Notes |
|---|---|---|
| `SelectPreviousCompletion` / `SelectNextCompletion` | 🟡 | Navigation logic is pure; depends on the bridge to populate `completion_proposals`. Most real-path tests use `LiveOxfmlBridge`; deterministic unit tests use local fixture documents or fake bridges. |
| `SelectCompletionByIndex` / `AcceptSelectedCompletion` / `AcceptCompletionByIndex` | 🟡 | Acceptance applies `InsertText(proposal.insert_text)`; depends on the bridge to populate proposals with correct `insert_text` and `replacement_span`. |
| `ForceShowCompletion` / `DismissCompletion` | 🟢 | Force-show is a state mutation only; dismiss clears the anchor on the editor surface state. No bridge call. |
| `CommitEntry` / `RequestProof` / `CancelEntry` | 🟡 | OneCalc tracks `committed_cell_text` / `proofed_cell_text` and flips `live_state` correctly. **No real "commit" semantic with the bridge** — the bridge has no notion of "this proof is now canonical". This is fine for the in-memory preview, broken for any persistence story. |
| `SendSelectionToInspect` | 🔴 | Command exists but no `Inspect` bridge consumer. Reducer just no-ops it. |
| `ToggleExpandedHeight` | 🟢 | Per-formula-space `expanded_editor` flag flip. View-only. |
| `ToggleEditorSettingsPopover` / `UpdateEditorSetting` | 🟢 | Workspace-level chrome state. |
| `ToggleConfigureDrawer` | 🟢 | Workspace-level chrome state. |

### 10.3 Formula evaluation pipeline

| Feature | Status | Seam | Notes |
|---|---|---|---|
| Send entered text to bridge | 🟢 | — | `ApplyFormulaEditIntent` → `bridge.apply_formula_edit`. |
| Receive `EditorDocument` back | 🟢 | — | `EditorSessionService::apply_editor_document`. |
| `editor_syntax_snapshot` (tokens) | 🟢 | — | Stored on the formula space, projected via `syntax_runs_from_snapshot`. |
| `live_diagnostics` | 🟡 | — | `LiveOxfmlBridge` returns real diagnostics when OxFml surfaces them; deterministic tests also use fake bridge documents. The Inspect mode renders them but the editor surface doesn't yet show squiggles. |
| `completion_proposals` | 🟡 | — | Same shape on the live bridge and in fixture documents used by deterministic tests. |
| `signature_help` / `function_help` | 🟡 | — | Same. |
| `formula_walk` (recursive nodes) | 🟡 | — | Inspect mode renders the walk. Deterministic tests can still inject synthetic walks through fixture documents. |
| `parse_summary` / `bind_summary` / `eval_summary` / `provenance_summary` | 🟡 | — | All carried; rendered partially in Inspect; no Workbench consumer yet. |
| `value_presentation: FormulaValuePresentation` | 🔴 | `SEAM-ONECALC-EXTENDED-VALUE-ROUTING` | OneCalc-local mirror. Real value routing will provide a typed `EvalValue` / `ExtendedValue` from `oxfunc_value_types`. Until then, projection comes either from bridge-supplied `value_presentation` or from limited local fallback classification for blocked, diagnostic, forced-text, and plain text/number entry. |
| `latest_evaluation_summary` (string) | 🔴 | `SEAM-ONECALC-EXTENDED-VALUE-ROUTING` | Derived from the current bridge projection or the limited local fallback classification above. |
| `effective_display_summary` (string) | 🔴 | `SEAM-ONECALC-EXTENDED-VALUE-ROUTING` | Same. |
| `array_preview` (rows of strings) | 🟡 | `SEAM-ONECALC-EXTENDED-VALUE-ROUTING` | Synthesised for `=SEQUENCE(...)`; will be real once typed values flow. |

### 10.4 Formula spaces and workspace

| Feature | Status | Seam | Notes |
|---|---|---|---|
| New / rename / duplicate / close formula space | 🟢 | — | `case_lifecycle.rs`, fully tested. |
| Pin / unpin | 🟢 | — | Same. |
| Activate / select formula space | 🟢 | — | `select_active_formula_space`. |
| Switch active mode | 🟢 | — | `switch_active_mode`. |
| Untitled-N id generation | 🟢 | — | `next_untitled_index`. |
| Last-space-creates-untitled fallback | 🟢 | — | `close_formula_space` enforces this. |
| Workspace persistence to disk | 🔴 | `SEAM-ONECALC-PERSISTENCE-V1` | `persistence/mod.rs` is `PersistencePlaceholder`. No serialisation. |
| Recents tracking | 🔴 | `SEAM-ONECALC-PERSISTENCE-V1` | No `recent_formula_space_ids` field anywhere. |
| Workspace settings page | 🔴 | `SEAM-ONECALC-CAPABILITY-SNAPSHOT` | View not implemented; data source `CapabilityAndEnvironmentState` is empty. |

### 10.5 Retained artifacts (verification bundles)

| Feature | Status | Seam | Notes |
|---|---|---|---|
| `RetainedArtifactRecord` data shape | 🟢 | — | Carries every field from `VerificationCaseReport`. |
| `import_verification_bundle_report_json` | 🟢 | — | Full JSON parse → catalog. |
| `import_manual_artifact_for_active_formula_space` | 🟢 | — | Hand-built imports. |
| `open_retained_artifact_by_id` | 🟢 | — | Routes to Workbench (or hint mode). |
| `open_retained_artifact_in_inspect_by_id` | 🟢 | — | Routes to Inspect. |
| `value_match` / `display_match` / `replay_equivalent` verdict triple | 🟢 | — | Carried, rendered in rail badges and Workbench summary. `value_match` stays exact on normalized comparison JSON and `display_match` stays exact on final display strings. Future “near” diagnostics, if added, should be secondary evidence only, not pass/fail verdicts. |
| `OxReplayMismatchRecord` / `OxReplayExplainRecord` | 🟢 | — | Full data; Inspect renders them as comparison records. |
| `oxfml_comparison_value` / `excel_comparison_value` (raw JSON) | 🟡 | `SEAM-ONECALC-EXTENDED-VALUE-ROUTING` | Carried as `serde_json::Value`; not parsed into typed values. |
| `oxfml_effective_display_summary` / `excel_effective_display_text` | 🟢 | — | Strings from upstream; rendered. Ordinary programmatic corpus verification supplies a host-owned explicit default context (`en-US`, 1900 system) so the OxFml display side stays live; only malformed or specially contextless programmatic requests should end up display-blocked. |
| `display_comparison_summary` helper | 🟢 | — | `verification_bundle::display_comparison_summary`. Reports exact display-string divergence; numeric-equivalent displays are still verdict reds until a separate diagnostic surface exists. |
| `replay_projection_coverage_gap_summaries` | 🟢 | — | Aggregates gap reports into reviewable strings. |
| `VerificationObservationGapReport` | 🟢 | — | Full shape; rendered as upstream-gap summaries in Inspect / Workbench. |
| Workbench "Parity Matrix" view (structured value/display/replay matrix with trace links) | 🔴 | `SEAM-ONECALC-PARITY-MATRIX-VIEW` | Today the workbench builder produces flat `outcome_summary` / `evidence_summary` / `lineage_items` strings instead of a structured matrix. |
| Trace event consumption from `oxreplay` | 🔴 | `SEAM-ONECALC-TRACE-CONSUMPTION` | No trace events carried in `RetainedArtifactRecord`. |

### 10.6 Locale, format, host config, scenario policy

| Feature | Status | Seam | Notes |
|---|---|---|---|
| Locale selection | 🟢 | — | OxFunc W094 ships 30 canonical `LocaleProfileId`s + `format_profile()`; OxFml exposes `oxfml_locale_context`; host plumbs `OneCalcHostState.ambient_app_context.language_tag` through `live_bridge::build_runtime_locale_context` per bridge round-trip. Workspace-locale dropdown surfaces in the formatting panel. |
| Date system (1900 / 1904) | 🔴 | `SEAM-ONECALC-CAPABILITY-SNAPSHOT` | OxFml `EditorPlanOptions` carries it; OneCalc never sets it. |
| Host profile string | 🟢 | — | `FormulaSpaceContextState.host_profile` is a string field; populated by bootstrap; rendered in the scope strip. No editing UI. |
| Reference style (A1 vs R1C1) | 🔴 | `SEAM-OXFML-R1C1-PUBLIC` | No state. |
| Number format string editing | 🔴 | `SEAM-ONECALC-FORMAT-PAYLOAD` | No model. |
| Conditional formatting rules | 🔴 | `SEAM-OXFML-CF-COLORSCALE`, `SEAM-OXFML-CF-DATABAR`, etc. | No model. |
| Cell font / fill / border / alignment / protection | 🔴 | `SEAM-OXFML-FONT-MODEL`, `SEAM-OXFML-BORDER-MODEL`, etc. | No model. |
| Scenario policy (Deterministic / Real-time / Real Random) | 🔴 | `SEAM-OXFML-EVAL-FREEZE` | No model. |
| Scenario flag toggles (volatile / freeze / cache / strict) | 🔴 | `SEAM-OXFML-EVAL-FREEZE` | No model. |
| Calc options (mode, iterative, max iter, tolerance, precision-as-displayed) | 🔴 | `SEAM-OXFML-CALC-OPTIONS` | No model. |
| Host bindings (defined names, cell values, table catalog, RTD, host info) | 🔴 | `SEAM-ONECALC-HOST-BINDINGS-PLUMBING` | OxFml `SingleFormulaHost` has the shape; OneCalc state has no carrier. |

### 10.7 Inspect mode

| Feature | Status | Seam | Notes |
|---|---|---|---|
| Formula walk projection | 🟢 | — | Recursive `InspectFormulaWalkNodeView`. |
| Parse / bind / eval / provenance summaries | 🟢 | — | Stored, rendered. |
| Retained artifact context with comparison records | 🟢 | — | Full shape. |
| Display comparison summary | 🟢 | — | `display_comparison_summary` helper. |
| Per-node value drill-down | 🔴 | `SEAM-ONECALC-EXTENDED-VALUE-ROUTING` | No way to ask "what's the value of this walk node?" against the bridge. |
| Selection-to-walk bridge (`SendSelectionToInspect` from editor) | 🔴 | — | Reducer no-ops the command. |

### 10.8 Workbench mode

| Feature | Status | Seam | Notes |
|---|---|---|---|
| Outcome summary string | 🟢 | — | Synthesised from comparison status + evidence. |
| Evidence summary string | 🟢 | — | Green tree + diagnostic count + retained discrepancy. |
| Lineage timeline (string list) | 🟢 | — | Synthesised: scenario opened → editor projected → evaluation captured → artifact opened. |
| Action items (string list) | 🟢 | — | Retain / compare / handoff / review. |
| Retained catalog projection | 🟢 | — | `WorkbenchRetainedCatalogItemView`. |
| Side-by-side OxFml vs Excel value comparison | 🔴 | `SEAM-ONECALC-EXTENDED-VALUE-ROUTING` | Today only `comparison_records` (string-shaped projections) exist. |
| Trace panel with replay events | 🔴 | `SEAM-ONECALC-TRACE-CONSUMPTION` | No trace carrier. |
| Witness chain / handoff history | 🔴 | `SEAM-ONECALC-WITNESS-HANDOFF-MODEL` | No models. |

### 10.9 Shell chrome view models

| Feature | Status | Notes |
|---|---|---|
| `ShellFrameViewModel` with mode tabs / breadcrumb / scope strip / formula spaces / context facts / footer facts | 🟢 | `services/shell_composition.rs::build_shell_frame_view_model`. |
| `mode_accent_slug` for theming | 🟢 | — |
| `ShellRailSection { Pinned, Open }` classification | 🟢 | — |
| `ShellRetainedVerdictsViewModel` (V/D/R badges) | 🟢 | — |
| `ShellScopeSegmentStatus { Live, NotImplemented { seam_id } }` | 🟢 | The shell renders facade markers with the seam id in the tooltip. |

### 10.10 Bootstrap and CLI

| Feature | Status | Notes |
|---|---|---|
| `OneCalcHostApp` headless wrapper | 🟢 | Used by CLI. |
| `bootstrap_editor_bridge(target)` | 🟢 | Switches Live vs Preview based on feature flag and env var. |
| `render_shell_html` / `render_shell_document` | 🟢 | SSR rendering paths used by tests and the static document case. |
| `preview_host_state` demo seed | 🟢 | Three demo scenarios, three retained artifacts. |
| `verify-formula` CLI | 🟢 | `main.rs::run_verify_formula`. |
| `verify-xml-cell` CLI | 🟢 | `main.rs::run_verify_xml_cell`. |
| `verify-batch` CLI | 🟢 | `main.rs::run_verify_batch`. |

## 11. Test invariants the new test suite should pin

When the new test suite is built (the missing layer that's been the subject of the recent regressions), these are the invariants worth pinning at each layer.

### 11.1 State invariants (test against `OneCalcHostState`)

1. `OneCalcHostState::default()` produces an empty workspace with zero formula spaces, mode = Explore, no active formula space id, empty pinned set, empty retained artifact catalog, default editor settings, no popover or drawer open.
2. `new_formula_space` always produces a unique id of the form `untitled-N` where N is greater than every existing id of that form.
3. After `new_formula_space`, the workspace has the new space as both `active_formula_space_id` and the last entry in `open_formula_space_order`.
4. `close_formula_space` of the only space leaves the workspace with one space (a fresh untitled).
5. `close_formula_space` of a non-active space does not change the active id; closing the active space promotes the first remaining space to active.
6. `toggle_pin_formula_space` is involutive (two calls return to the starting set).
7. `apply_editor_input_to_active_formula_space` clears `editor_document`, `completion_help`, `latest_evaluation_summary`, `effective_display_summary` (the "stale analysis" set) but preserves `committed_cell_text` and `proofed_cell_text`.
8. `commit_entry` makes `committed_cell_text == raw_entered_cell_text`; subsequent `live_state()` returns `Committed`.
9. `request_proof` makes `proofed_cell_text == raw_entered_cell_text`; subsequent `live_state()` returns `ProofedScratch` if not committed.
10. `cancel_entry` reverts `raw_entered_cell_text` to `committed_cell_text` (when present) and resets the editor surface state.

### 11.2 Reducer invariants

1. Every reducer function is pure and idempotent for fixed inputs.
2. `apply_editor_command_to_active_formula_space` returns `false` only when no active formula space exists.
3. UI-chrome commands (`ToggleConfigureDrawer`, `ToggleEditorSettingsPopover`, `UpdateEditorSetting`) succeed even when no active formula space exists.
4. `live_edit::apply_live_editor_command` skips the bridge refresh for commands listed in its match arm and refreshes for everything else.

### 11.3 Bridge / session invariants

1. `EditorSessionService::handle_formula_edit_intent` always either updates the formula space or returns an error — never partial.
2. `apply_editor_document` derives a `truth_source` of `LiveBacked` for any document where `provenance_summary.profile_summary` contains "OxFml" or `value_presentation` is present, and `LocalFallback` otherwise.
3. `derive_formula_presentation` returns `Unevaluated` for formula text that has no `value_presentation`, blocked reason, or diagnostic from the bridge.
4. After a successful bridge round-trip, `editor_document.source_text == raw_entered_cell_text`.

### 11.4 Adapter contract invariants

1. `OxfmlEditorBridge::apply_formula_edit` is the only method on the trait — every bridge implementation must satisfy it.
2. Deterministic adapter tests use fake bridges or fixture documents rather than a separate preview bridge implementation.
3. `EditorDocument::green_tree_key()` returns the snapshot's green tree key verbatim.

### 11.5 Editor model invariants (pure logic)

1. `keydown_to_command` returns `None` for all of: `ArrowLeft`, `ArrowRight`, `ArrowUp` (without completion), `ArrowDown` (without completion), `Backspace`, `Delete`, plain `Enter` (without completion), plain `x` (without Ctrl).
2. `keydown_to_command` returns `Some(SelectPreviousCompletion)` for `ArrowUp` only when `completion_active`, similarly for `ArrowDown`.
3. `keydown_to_command` returns `Some(AcceptSelectedCompletion)` for `Enter` and `Tab` only when `completion_active`.
4. `apply_editor_command` of `InsertText("foo")` against a state with caret at offset 5 produces text with "foo" inserted at byte position 5 (correctly handling multi-byte chars) and caret at offset 5 + chars("foo").
5. `bracket_pair_for_caret` returns `None` for any caret position not adjacent to a bracket character outside string literals.
6. `cycle_reference_form` produces the exact 4-step cycle `A1 → $A$1 → A$1 → $A1 → A1` for `A1` selected at any character offset within the reference.
7. `EditorEntryMode::classify("=…")` returns `Formula`, `classify("'…")` returns `Text`, `classify("123")` returns `Value`, `classify("")` returns `Empty`.
8. `EditorLiveState` derivation from `FormulaSpaceState`:
   - If `committed_cell_text == raw_entered_cell_text`, returns `Committed`.
   - Else if `proofed_cell_text == raw_entered_cell_text`, returns `ProofedScratch`.
   - Else if `raw_entered_cell_text.is_empty()` and no committed text, returns `Idle`.
   - Else returns `EditingLive`.

### 11.6 Browser-level invariants the new wasm-bindgen test layer must pin

These are the invariants the **missing test layer** must pin once it exists. They are the ones that have been bitten in production over the last several slices:

1. After clicking at character offset N in the textarea, the DOM `selectionStart == selectionEnd == N`.
2. After pressing left arrow at offset N > 0, the DOM `selectionStart == selectionEnd == N - 1`.
3. After pressing right arrow at offset N < len, the DOM `selectionStart == selectionEnd == N + 1`.
4. After pressing Enter at offset N (no completion popup), the textarea contains a `\n` at byte position N and the DOM caret advances by one.
5. After pressing Backspace at offset N > 0, the character at offset N - 1 is removed and the DOM caret moves to N - 1.
6. After pressing Delete at offset N < len, the character at offset N is removed and the DOM caret stays at N.
7. After pressing Tab with no completion popup, four spaces are inserted at the start of the line containing the caret.
8. After pressing Tab with a completion popup visible, the selected proposal's `insert_text` replaces the `replacement_span` and the DOM caret lands at the end of the inserted text.
9. After pressing F4 with a single-cell reference under the caret, the reference text rotates one step in the cycle and the DOM selection covers the rewritten reference.
10. After typing `SUM` rapidly into an empty formula, the spellcheck popup never appears (the textarea has `spellcheck="false"`).
11. After typing for 600ms then pausing, `live_state` transitions from `EditingLive` to `ProofedScratch` (auto-proof debounce).

These invariants form the **first-priority test corpus** for the wasm-bindgen layer.

## 12. Open seams summary (cross-reference to WS-13 Appendix B)

The following seams referenced in this document are listed in WS-13 Appendix B with full descriptions. Cross-reference to that document for the engine-side contract each one represents.

**OneCalc-side seams** (UI / state work; no engine change required):

- `SEAM-ONECALC-EXTENDED-VALUE-ROUTING` — route the upstream `EvalValue` / `ExtendedValue` / `PresentationHint` through `FormulaSpaceState` so the result panel renders structurally.
- `SEAM-ONECALC-CAPABILITY-SNAPSHOT` — populate `CapabilityAndEnvironmentState` from `ProgrammaticVerificationConfig` + bootstrap so the workspace settings page has data.
- `SEAM-ONECALC-HOST-BINDINGS-PLUMBING` — carry `SingleFormulaHost`-shaped fields (defined names, cell values, table catalog, RTD providers) on `FormulaSpaceState` so the Configure drawer Host Bindings tab can edit them.
- `SEAM-ONECALC-PERSISTENCE-V1` — workspace JSON v1 in `persistence/mod.rs` for round-trip across sessions.
- `SEAM-ONECALC-PARITY-MATRIX-VIEW` — replace the flat workbench outcome strings with a structured Parity Matrix view model.
- `SEAM-ONECALC-TRACE-CONSUMPTION` — carry `oxreplay` trace events in `RetainedArtifactRecord` and surface them in the Workbench trace panel.
- `SEAM-ONECALC-WITNESS-HANDOFF-MODEL` — carry witness chain and handoff history models.
- `SEAM-ONECALC-RAIL-INLINE-RENAME` — implement inline rename in the rail.
- `SEAM-ONECALC-VERIFICATION-BUNDLE-CONTEXT` — extend `services/verification_bundle.rs` to round-trip `CellFormatPayload` and `CalcOptionsPayload`.

**OxFml-side seams** (engine work in `OxFml/`):

- `SEAM-OXFML-FORMAT-PAYLOAD`, `SEAM-OXFML-CF-*` (colorscale / databar / iconset / text / dates / blanks / errors / rank / average / unique), `SEAM-OXFML-FONT-MODEL`, `SEAM-OXFML-BORDER-MODEL`, `SEAM-OXFML-ALIGNMENT-MODEL`, `SEAM-OXFML-PROTECTION-MODEL`, `SEAM-OXFML-FILL-COLOR`, `SEAM-OXFML-FILL-EFFECTS`, `SEAM-OXFML-STYLE-XF`, `SEAM-OXFML-R1C1-PUBLIC`, `SEAM-OXFML-CALC-MODE`, `SEAM-OXFML-CALC-ITERATIVE`, `SEAM-OXFML-PRECISION-AS-DISPLAYED`, `SEAM-OXFML-EXTERNAL-LINKS`, `SEAM-OXFML-EVAL-FREEZE`, `SEAM-OXFML-EVAL-CACHE`, `SEAM-OXFML-EVAL-STRICT`, `SEAM-OXFML-RICH-VALUE-PUBLICATION`, `SEAM-OXFML-GREEN-TREE-NODE-ID`, `SEAM-OXFML-COMPARISON-VIEW-TAXONOMY`, `SEAM-OXFML-PRESENTATION-PROPAGATION`.

**OxFunc-side seams** (engine work in `OxFunc/`):

- `SEAM-OXFUNC-FMT-GRAMMAR-VALIDATION`, `SEAM-OXFUNC-FMT-RED`, `SEAM-OXFUNC-FMT-CURRENCY`, `SEAM-OXFUNC-FMT-ACCOUNTING`, `SEAM-OXFUNC-FMT-ELAPSED`, `SEAM-OXFUNC-FMT-FRACTION`, `SEAM-OXFUNC-FMT-SCIENTIFIC`, `SEAM-OXFUNC-FMT-SPECIAL`, `SEAM-OXFUNC-LOCALE-EXPAND`, `SEAM-OXFUNC-VALUE-BOUNDARY-HELP`.

**OxXlPlay-side seams** (engine work in `OxXlPlay/`):

- `SEAM-OXXLPLAY-CAPTURE-FONT`, `SEAM-OXXLPLAY-CAPTURE-BORDER`, `SEAM-OXXLPLAY-CAPTURE-ALIGNMENT`, `SEAM-OXXLPLAY-CAPTURE-PROTECTION`, `SEAM-OXXLPLAY-CAPTURE-CF-VISUAL`, `SEAM-OXXLPLAY-INPUT-CONTEXT`.

## 13. What this document does not cover

- The Leptos rendering layer (`ui/components/`, `ui/design_tokens/`, `ui/modes/`). That layer is being reset under WS-13 Area 0 (Minimal foundation reset) and will be rewritten against a wasm-bindgen browser test layer.
- The CSS theme (`ui/design_tokens/theme.rs`).
- The CLI command surface in `main.rs` beyond noting that it exists.
- Upstream library internals (`OxFml/`, `OxFunc/`, `OxXlPlay/`, `OxReplay/`). Those have their own reference docs in their own repos.

## 14. Maintenance rules

- When you add a new state field on `FormulaSpaceState` or `OneCalcHostState`, add it to §3 here and to the Implementation Matrix in §10 with the appropriate status.
- When you add a new `EditorCommand` variant, add it to §6.2 and the keymap section (§11.5).
- When you flip a feature from 🔴 FACADE to 🟡 PARTIAL or 🟢 LIVE in the matrix, also strike the corresponding seam in §12.
- When you add a new `SEAM-*` id, register it both here in §12 and in WS-13 Appendix B.
- When you add a new test invariant, append to §11. Keep the section numbered so individual invariants can be cited.
