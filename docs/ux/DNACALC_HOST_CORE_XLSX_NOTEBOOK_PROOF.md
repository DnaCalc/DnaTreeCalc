# W011 - DnaCalc Host Core + B1 `.xlsx` Notebook Proof

## Status

This is the execution plan for workset
`W011_dnacalc_host_core_xlsx_notebook_proof`. The chat typo `dnascalc` is
normalized to `dnacalc` because this work pivots the new core naming away from
the tree-only model.

W011 is not a replacement for W010. W010 remains OPEN and off the critical
path as the separate UDF-hosting workset. W011 is the immediate integration
proof that OxDoc and OxCalc can be hosted together cleanly by a DnaCalc host
and surfaced through Skin IR.

**Charter note (2026-07-02).** CHARTER.md's "Not a grid" clause predates the
owner-approved dual-profile decision (treecalc-v1 + strict-excel-grid; see the
grid workstream decision record and OxCalc
`CORE_ENGINE_GRID_MODEL.md` §2). W011's workbook-grid hosting is the sanctioned
DnaCalc-host pivot recorded in the register's "W011 pivot" paragraph; CHARTER.md
carries a dated mark note pointing here until a full charter amendment lands.

## Goal

Build the first visible reference host for the full stack:

1. Open a small `.xlsx` file in the browser.
2. Load it through OxDoc with `LoadProfile::full()`.
3. Keep the OxDoc source/model context owned by the host.
4. Create or reset the OxCalc workbook state owned by the same host.
5. Drive OxCalc ingest from neutral `oxdoc-model` access — the translation
   lives in the host (see "Upstream Work" for what is local vs upstream).
6. Publish workbook sheets and cells through Skin IR.
7. Render them in a B1 Pluto-style notebook skin.
8. Edit a grid cell through `WorkspaceIntent::EnterGridCell` (the engine's
   three-way literal/formula/clear branch; see "Edit scope").
9. Recalculate dependents through OxCalc and emit `GridChanged`.
10. Save/download a round-tripped `.xlsx` through OxDoc.

The proof fixture is intentionally small: `A1 = 7`, `B1 = =A1*3`. The proof is
complete when editing `A1` to `10` makes `B1` render as `30`, and the saved
workbook reopens through OxDoc with `A1` changed, `B1` formula text preserved,
**and `B1`'s cached value updated to `30`** (the stale-cached-value trap is the
most likely silent failure; see "Save path").

## Anchors

- DnaTreeCalc charter: [`../../CHARTER.md`](../../CHARTER.md).
- Skin doctrine: [`SKINS.md`](SKINS.md).
- Three-front-ends plan: [`THREE_FRONTENDS_PLAN.md`](THREE_FRONTENDS_PLAN.md).
- Upstream lane ledger: [`../interop/UPSTREAM_OX_LANES.md`](../interop/UPSTREAM_OX_LANES.md).
- OxDoc host boundary: [`DOCUMENT_LIFECYCLE_AND_HOST_BOUNDARIES.md`](C:/Work/DnaCalc/OxDoc/docs/DOCUMENT_LIFECYCLE_AND_HOST_BOUNDARIES.md).
- OxCalc grid model: [`CORE_ENGINE_GRID_MODEL.md`](C:/Work/DnaCalc/OxCalc/docs/spec/core-engine/CORE_ENGINE_GRID_MODEL.md).

The decisive boundary comes from the OxDoc host-boundary document:

- The host owns the source package/model context and the OxCalc context.
- OxDoc loads and saves workbook packages.
- OxCalc consumes the neutral model and owns calculation/edit semantics.
- The host passes context into clean stateless library calls.
- Skins use Skin IR only and never call OxDoc, OxCalc, or file APIs directly.

### Relation to THREE_FRONTENDS_PLAN

- **`dnacalc-host-core` IS the Gap-4 crate.** THREE_FRONTENDS_PLAN §A1/Gap 4
  mandates a host-adjacent, Leptos-free SessionEngine crate decided "before
  A1", which B1 and B3 both depend on. W011 answers that decision:
  `dnacalc-host-core` is that crate. `SessionEngine::init/apply/snapshot/
  export_xlsx` map onto `HostCommand::OpenXlsxBytes` /
  `DispatchWorkspaceIntent` / the snapshot accessor / `SaveActiveXlsx`.
  dtc-hj2.3 updates the Gap-4 line in THREE_FRONTENDS_PLAN in the same commit
  that creates the crate.
- **One B1 cell model, two substrates.** THREE_FRONTENDS_PLAN §B1 defines the
  notebook over the tree workspace (NodeView/`node_order`, C2 classification
  tint, NotebookCellKind taxonomy). W011's B1 renders a workbook
  `GridProjection`. The mapping is explicit so the two never fork: authored
  cell kind (empty/literal/formula) aligns with `NodeContentKind`;
  classification tint (C2) applies to grid cells only once U-DEP-grade
  dependency metadata reaches grid projections — it is **out of scope for the
  W011 read-only slice**, not re-derived ad hoc; `EnterGridCell` is mutation
  surface (1) applied to grid cells.
- **Skin state.** The workbook notebook persists skin state (scroll, collapse,
  layout) through the §A2 interim `SkinStatePersistenceStore`, targeting the
  CustomXml store later — never a new ad-hoc store, honoring Gap 5's single
  schema-version line.

## Current Code Pointers

These observations are the verified starting map for implementers
(surveyed 2026-07-02; re-verified 2026-07-04 against OxCalc `e069136e` after
the CTRO rework landed — every W011-critical API below survived unchanged,
and DnaTreeCalc compiles green with grid tests passing against it).

DnaTreeCalc:

- `src/dnatreecalc-skin-framework` mixes pure Skin IR and Leptos mounting at
  the **crate** level only: exactly 4 of 18 modules import Leptos (`skin.rs`
  wholly; `state.rs` only its two signal handles; `intent.rs` only
  `InMemoryDispatcher`; `tests.rs`). No protocol type (`WorkspaceState`,
  intents, deltas, `GridProjection`, `SelectionState`) contains a signal; all
  carry serde derives; `session_channel.rs` is already written "no Leptos, no
  web-sys". The dtc-hj2.2 split is mostly mechanical moves plus three careful
  relocations (`SharedSkinStateHandle`, `InMemoryDispatcher`,
  `SkinRegistry`/`RegisteredSkin`).
- `WorkspaceIntent` already carries `SetGridInterest`; `WorkspaceDeltaChange`
  already carries `GridChanged` and `GridOverlaysChanged`. **Stale since H6:**
  `WorkspaceIntent::EnterGridCell` dispatches end to end (`dtc-j7n8.6` proves
  it on the loaded fixture).
- `GridCellProjection` carried only `row/col/value/value_epoch` at survey
  time — no authored kind, no formula source text, no editability. **Stale
  since H3/H5:** it now carries `authored: Option<GridAuthoredCellProjection>`
  (kind, literal_text, source_text, editability) and `provenance:
  Option<ValueProvenanceProjection>` (`Calculated` / `Stale` / `FileCached`),
  and host-core's `grid_projection_for` folds both. dtc-j7n8.5 verified that
  fold over the LOADED fixture (`snapshot_of_loaded_fixture_projects_authored_and_provenance`,
  `workbook.rs`; `snapshot_of_loaded_fixture_through_document_session_is_not_defaulted`,
  `lib.rs`): `A1` `Literal` `"7"` = 7, `B1` `Formula` `"=A1*3"` = 21, both
  `Calculated` under the fixture's `calcMode="auto"` open-recalc,
  `authored_epoch > 0`. `FileCached` gets its first live assertion in the
  Manual-mode lane (dtc-j7n8.13). B1 renders `=A1*3` from today's projection
  with no skin change.
- `src/dnatreecalc-host`'s `TreeWorkspaceSession` (session.rs, ~10k lines) is
  already Leptos-free and owns the `OxCalcTreeContext` + node-id maps; grid
  interest is delegated to OxCalc, not stored in the session. The Leptos
  entanglement lives in `HostDispatcher` (~2.7k lines) — and its signal usage
  is non-reactive (`get_untracked`/`set` only), so a publication-trait rewrite
  is mechanical-but-wide. It also leans on a `thread_local` HOST_SESSIONS
  registry, assigns `projection_seq` via AtomicU64, and drags
  `build_default_registry` (Leptos skins) plus a dead `dnatreecalc-shell` dep
  into the host crate.
- The live web_sys worker **already exists** (opt-in `?worker=1`):
  `dnatreecalc-web/src/worker_client.rs` + `worker_runtime.rs`. The protocol
  (`session_channel.rs`, `WorkerProxyCore`, `WorkerInbound/Outbound`) is
  already Leptos-free. The only Leptos residue is `HostSessionExecutor`'s
  throwaway RwSignals + reactive Owner inside the worker, and
  `WorkerInbound::Init` carrying `DnaTreeWorkspaceDocument` (which couples
  dnatreecalc-worker to dnatreecalc-host). dtc-hj2.13 owns this alignment; the
  W011 split unblocks it but no protocol redesign is needed.
- `src/dnatreecalc-skins`' SheetLens (`grid_surface`/`grid_interest_window`)
  plus `session_channel::apply_delta`'s in-place `GridChanged` mirror patching
  are the reusable machinery for B1. Note SheetLens dispatches
  `SetGridInterest` per scroll event with no coalescing — B1 must coalesce
  before worker-mode use.
- The browser shell is `.dnatree`/localStorage oriented; there is **zero**
  file-picker/download plumbing and `dnatreecalc-web`'s web-sys features lack
  `File`/`FileList`/`FileReader`/`HtmlInputElement`/`HtmlAnchorElement`.
- The workspace's oxdoc dependency landed with dtc-j7n8.1 (W011-S1):
  `[workspace.dependencies]` carries `oxdoc_model`/`oxdoc_xlsx` (plus dev-only
  `oxdoc_conformance`) beside the `oxcalc_core`/`oxfml_core`/`oxfunc_core`
  sibling path deps, and `dnacalc-host-core` takes `oxdoc_model`/`oxdoc_xlsx` as
  normal deps; `cargo check --target wasm32-unknown-unknown` of `dnacalc-app` and
  `dnatreecalc-web` stayed green after the edge landed (no cfg-gate needed).
  Trunk builds `dnatreecalc-web` from the root `index.html`; `wasm-bindgen` is
  pinned `=0.2.117`; the root `Cargo.toml` `[profile.dev]` strips debuginfo to
  avoid Windows paging failures — new crates must inherit these.
- `workspace.rs` carries ~230 uncommitted lines adding `NodeClassification`
  (contract C2, pure + tested). Committing it is a precondition of dtc-hj2.2;
  it lands verbatim in `dnacalc-skin-ir`.

Sibling repo pointers are read-only from this repo (verified against code,
not docs):

- **OxDoc has everything W011 needs for the fixture, today.**
  `open_host_owned_xlsx_source(reader, profile)` → `HostOwnedXlsxSource
  { source_context: XlsxPackageSession, model_context, load_ledger }`;
  `LoadProfile::{full, values_only (Default), strict_values_only, empty}`;
  `XlsxSaveRequest::round_trip` + `write_save_request`; the whole API is
  bytes-in/bytes-out (`Read+Seek`/`Write+Seek`). OxDoc also already ships the
  neutral ingest seam: `oxdoc_model::OxCalcIngestSink` +
  `drive_oxcalc_ingest_from_model_access`.
- **OxDoc save-path limits (load-bearing for W011 scope):** there is no
  granular per-cell modeled edit — `ModeledEdit(Replace(CellChunk))` replaces
  ALL cell chunks for a sheet and a partial chunk fails downstream as cell
  removal. The supported cell-edit path is
  `WorkbookModelOutput::whole_model_projection` over a host-mutated clone of
  the session event stream. Round-trip save rejects (typed
  `UnsupportedRoundTripFeature`, never silent): adding/removing cells,
  adding/removing formulas, formula text changes without synchronized
  FormulaTopology edits, and edits to cells whose start tag carries any
  attribute beyond `r`/`t` (so styled cells `s="n"` reject). Formula-cell edits
  drop calcChain package edges by policy (harmless; do not assert calcChain
  byte-identity).
- **OxCalc has substantial, public grid machinery** — `GridAuthoredCell`,
  `GridFormulaCell { source_text, normal_form_key, source_channel }`,
  `ExcelGridCellAddress`, `GridBackingSeed` → `OxCalcTreeContext::
  set_node_grid`, `apply_grid_edit(OxCalcTreeGridOp::{SetCell, FillRange})`,
  `grid_view`, `register_grid_interest`, `poll_grid_changes`. The exact W011
  fixture choreography exists as the consumer test
  `grid_edit_setcell_and_fillrange_publish_and_bump_epochs`.
- **OxCalc gaps (drive the handover ranking below):** no oxdoc dependency and
  no ingest of workbook models exists at any level (`SheetGridIngest` is
  spec-doc only). All A1/R1C1 binding/normalization is `pub(super)`
  (`bind_grid_formula_for_transform`, now at `optimized_sheet.rs:7241`;
  `normal_form_key_for_reference`); `GridFormulaCell::new` trusts a
  caller-supplied `normal_form_key` with no coherence check — the key is
  engine-opaque (never parsed, only compared for equality), but CTRO deepened
  its use as formula-template identity, so a wrong hand-key silently poisons
  plan caches. No consumer-level authored readout (`OxCalcTreeGridCell
  Readout` is `{address, value, value_epoch}` only; the authored sheet is a
  private field). No `ClearCell` verb. **No consumer-level defined-name
  seeding**: `GridOptimizedSheet::set_defined_name` is `pub` but unreachable
  through `OxCalcTreeContext`/`GridBackingSeed`, and as of `e069136e`
  formulas referencing unseeded names deterministically evaluate to `#NAME?`
  (Excel-faithful, self-healing if the name is later created) — a general
  workbook using defined names will show `#NAME?` cells in B1 until the
  ingest handover covers name seeding; the W011 fixture is unaffected. No
  workbook-shaped context — a workbook is modeled as one workspace with one
  grid-backed node per sheet, and the host aggregates per-node
  `poll_grid_changes` into `GridChanged`.
- **OxCalc CTRO rework LANDED** (`3181900a`/`d1bb5cb4`/`e069136e`, verified
  2026-07-04): ~28.5k lines, all engine-internal (effective calculation
  dependency graph = structural edges + calc-time realized overlay edges;
  dependency-ordered dirty recalc replaces address-order passes; the old
  claim/consequence `rebind.rs` is deleted). The consumer surface was not
  touched; the W011 choreography (`create_workspace → add_node →
  set_node_grid → register_grid_interest → apply_grid_edit →
  poll_grid_changes`) is proven verbatim by the four consumer grid tests
  passing on HEAD. What CTRO buys the notebook proof: forward formula chains
  recalc in correct dependency order, spill/table/name lifecycle changes feed
  dirty seeds correctly, and effective-graph cycles (including
  `INDIRECT`-realized ones) are reported as
  `GridRefError::EffectiveDependencyCycleDetected` (iterative convergence is
  parked upstream — the host surfaces the cycle error, never spins).
  Remaining overlap risk: the unreviewed `stash@{0}` touches `consumer.rs`;
  if it ever lands, fast-re-verify the seven consumer APIs. Perf note: every
  consumer recalc still runs `GridEngineMode::Both` over the whole sheet
  (interest scoping applies to the cached readout only) — fine for the
  fixture, a known cliff for large real sheets, watch it when B1 opens bigger
  workbooks.
- **`.xlsx` conventions that bite:** `<f>` stores formula text **without** the
  leading `=` while OxCalc `GridFormulaCell.source_text` convention is
  `=`-prefixed — normalize explicitly; `SharedText(u32)` needs the StringTable
  event resolved first; `Error(u8)` maps to `WorksheetErrorCode`.

## Target Architecture

### `dnacalc-skin-ir`

Pure Skin IR crate. It has no Leptos dependency (including dev-dependencies)
and owns the UX protocol:

- identity types and stable projection keys;
- `WorkspaceState` and sub-projections (including `NodeClassification`/C2);
- `WorkspaceIntent`;
- `WorkspaceDeltaChange`;
- grid interest and `GridProjection` (extended with authored cell metadata by
  dtc-hj2.6);
- selection state;
- session-channel protocol and delta application;
- the `Dispatcher` trait and a signal-free `RecordingDispatcher` test
  dispatcher (`Arc<Mutex<SelectionState>>` + intent log), so IR and host-core
  tests never pull Leptos;
- `pub const SKIN_IR_PROTOCOL_SCHEMA: &str = "dnacalc.skin_ir.v1";` carried on
  the session handshake (worker `Init`/`Ready`, future MCP hello) — not on
  every envelope. Unknown-variant policy documented at the crate root: serde
  rejects unknown intent variants, so a version-skewed peer gets a typed
  decode failure, never a silently-ignored intent.

This crate is the interface between the host core, browser UI, worker, future
CLI/MCP transport, and every skin.

**Disposition rule (three-way, not two-way):** UI-side helpers that are
Leptos-free but not protocol — `keybinding.rs`, `style.rs` (CSS constants),
`theme.rs`, `accessibility.rs` — go to `dnacalc-skin-leptos` as plain modules,
not to the IR crate. `LocalFileSkinStatePersistenceStore` (std::fs) goes to
`dnacalc-host-core` beside the other native persistence. A B3 transport
linking the wire-protocol crate must not carry CSS strings and key chords.

### `dnacalc-skin-leptos`

Leptos adapter crate. It owns UI mounting concepts only:

- `WorkspaceSkin`;
- `SkinContext`;
- skin registry (`SkinRegistry`/`RegisteredSkin`; the pure
  `SkinManifest`/`SkinCapabilities` stay in the IR crate);
- skin-state handles (`SkinStateHandle`, `SharedSkinStateHandle` — Copy only
  via the Leptos arena, so they cannot live in a Leptos-free crate);
- the signal-backed `InMemoryDispatcher` for skin tests;
- Leptos signal adapters, view mounting and composition helpers;
- the UI-side helper modules listed in the disposition rule.

It depends on `dnacalc-skin-ir` and re-exports it
(`pub use dnacalc_skin_ir::*`) so downstream import churn stays minimal; the
pure IR crate never depends back on it.

### `dnacalc-host-core`

Leptos-free reference host crate — and, by declaration above, the Gap-4
SessionEngine crate. It owns the root context:

- active document identity;
- OxDoc source package session and model context;
- OxCalc context (tree or workbook-shaped composition);
- dirty state and save ledgers;
- command execution;
- Skin IR snapshot and delta publication (via a `ProjectionPublisher` seam —
  the Leptos adapter binds signals to it; the worker binds postMessage);
- single-skin and multi-skin layout state (`SkinLayoutSpec`, serializable,
  defined in `dnacalc-skin-ir`; the shell renders host-owned layout).

The host command surface is typed end to end:

```rust
pub enum HostCommand {
    OpenXlsxBytes { bytes: Vec<u8>, name: Option<String> },
    SaveActiveXlsx,
    CloseActiveDocument { discard_unsaved: bool },
    SetSkinLayout(SkinLayoutSpec),
    DispatchWorkspaceIntent(WorkspaceIntent),
}

fn execute(&mut self, cmd: HostCommand) -> Result<HostCommandOutcome, HostCommandError>;
// HostCommandOutcome::Opened { name, sheet_count, cells, formulas_bound,
//                              recalc_path, load_ledger }   // landed dtc-j7n8.3 + .4
//                  ::Saved  { bytes: Vec<u8>, save_ledger: DocumentFidelityLedger }
//                  ::Dispatched(IntentReceipt) | ::LayoutSet | ::Closed
// HostCommandError wraps oxdoc_xlsx::XlsxError (incl. UnsupportedRoundTripFeature)
// as data, not strings — landed as HostCommandError::Workbook(WorkbookSessionError::Xlsx(_))
// (dtc-j7n8.3). Landed so far: OpenXlsxBytes (replaces the active session on
// success, leaves it untouched on failure; the host owns the HostOwnedXlsxSource
// via WorkbookSession::xlsx_source; since dtc-j7n8.4 the source's event stream
// is ingested into the engine through OxCalc's own load_workbook_model verb —
// the engine's WorkbookLoadReport is kept as WorkbookSession::load_report, and
// ingest-created grids are addressed under the `book:{workspace}` token the
// session's single workbook-token authority derives per origin) and
// DispatchWorkspaceIntent (always Ok — intent rejections travel inside the
// IntentReceipt).

fn document_status(&self) -> DocumentStatus; // { dirty: bool, save_restrictions: Vec<..> }
```

**Boundary rule:** `WorkspaceIntent` mutates the open model; `HostCommand`
manages documents, files, and layout. The existing
`WorkspaceIntent::{NewWorkspace, SwitchWorkspace, RenameWorkspace}` lifecycle
intents violate this rule; migrating them to `HostCommand` is a recorded
follow-up bead, not W011 scope.

**Send/Sync decision (resolve before the `Dispatcher` trait lands in the IR
crate):** audit whether `OxCalcTreeContext`/`TreeWorkspaceSession` are `Send`.
If Send: host-core owns sessions as plain `Arc<Mutex<DocumentSession>>` fields
and the `thread_local` HOST_SESSIONS registry is deleted. If !Send: keep
`Dispatcher: Send + Sync` for native/worker transports and add a
`LocalDispatcher` supertrait-free variant for the wasm main thread. Either
way, the decision is written into dtc-hj2.3, and the thread_local is not
re-invented.

Current `dnatreecalc-host` becomes the Leptos adapter over this core
(signals + `SharedSkinStateHandle` bound to the publisher seam).

### Model-Neutral Sessions

The host core does not bake in tree-only naming, but the common abstraction is
deliberately a **closed enum, not a trait**:

```rust
pub enum DocumentSession {
    RichTree(RichTreeSession),   // current tree workspace model
    Workbook(WorkbookSession),   // strict-grid workbook loaded from .xlsx
}
```

`TreeWorkspaceSession` (scenarios, sweeps, revision cursors, `.dnatree`
persistence) and `WorkbookSession` (HostOwnedXlsxSource + OxCalc grid state +
edit ledger) share almost no lifecycle beyond "consume `WorkspaceIntent`,
publish `WorkspaceState`" — that pair of methods IS the common surface for
now. A trait is extracted only when a third model family exists. Host-core
matches per intent and returns a typed `IntentError::UnsupportedByModel`
receipt for intents a family does not support (e.g. `CreateScenario` on
Workbook, `Undo`/`Redo` on Workbook in W011 — the workbook edit ledger is the
future undo substrate, recorded as a follow-up). Workbook dirty state =
edit ledger non-empty since last successful `SaveActiveXlsx`.

Skin IR speaks in model-neutral terms where possible. Tree-specific and
workbook-specific projections can exist, but the host protocol must support a
single skin or multiple skins over either model family.

### Worker placement

`dnacalc-host-core` is transport-agnostic. In worker mode it runs **entirely
inside the worker** — for workbook sessions the OxDoc source context must live
with the OxCalc state, because save needs both. The main thread holds only the
Skin IR mirror (`apply_delta`) + `WebWorkerDispatcher`. The worker protocol
gains `WorkerInbound::Command(HostCommand)` / `WorkerOutbound::CommandOutcome`,
and `.xlsx` bytes cross postMessage as a transferable ArrayBuffer alongside
the JSON envelope, not base64 inside serde_json. `WorkerInbound::Init` moves
to a model-neutral document enum
(`HostDocument { Tree(DnaTreeWorkspaceDocument), WorkbookXlsx { bytes } }`),
breaking dnatreecalc-worker's dependency on dnatreecalc-host. dtc-hj2.13 owns
this; the fixture proof does not gate on it.

## Edit scope (W011)

`WorkspaceIntent::EnterGridCell { grid, row, col, text }` is the universal
entry verb (H6). The engine's `enter_grid_cell` does the three-way
literal/formula/clear branch with OxFml as the sole interpretation authority
(OxCalc `consumer.rs`); the receipt carries `GridCellEntered { outcome }`
(`Literal`/`Formula`/`Cleared`). `dtc-j7n8.6` proves it on the xlsx-loaded
fixture: `A1` 7 → 10 publishes `B1` = 30, and `=A1*4` into `B1` is accepted
(no host-side `=` classification; the former `=`-prefix and no-`ClearCell`
typed rejections no longer exist in code). The typed rejections that remain
are the engine's own: unparseable formula text (diagnostics on the receipt)
and non-editable targets (spill/merged followers, table-structural cells).

Save scope is narrower than edit scope: a formula-text change or a new cell
is accepted into the live model but is save-restricted (OxDoc's round-trip
policy needs a synchronized `FormulaTopology`; `dtc-j7n8.7` documents the
typed `UnsupportedRoundTripFeature` rejection). The `SaveExpressible` /
`SaveRestricted` classification and `DocumentStatus.save_restrictions`
machinery remain a later shape, not W011 scope.

## Save path (W011)

The narrow first save is existing-cell literal edits plus engine-driven
cached-value refresh of existing formulas, via the **whole-model projection**
recipe (the only supported cell-edit path in OxDoc today):

1. Host-core keeps a `WorkbookEditLedger` of applied grid edits (it authored
   every seed and every `OxCalcTreeGridOp`).
2. At save: clone `source_context.events()`; replace A1's payload with
   `CellPayload::Number(10.0)` and B1's with `CellPayload::Formula { region:
   None, text: Some("A1*3") /* unchanged from load */, cached:
   Some(Box::new(CellPayload::Number(30.0))) }` — the cached value taken from
   OxCalc's post-recalc readout.
3. Wrap with `WorkbookModelOutput::whole_model_projection(events)`; call
   `write_save_request(XlsxSaveRequest::round_trip(&source_context, &output),
   Cursor::new(Vec::new()))`.
4. Surface the returned `DocumentFidelityLedger` through
   `HostCommandOutcome::Saved`.

Do **not** use `WorkbookModelEditKind::Replace(CellChunk)` for cell edits.
OxCalc is not asked for output in W011 — a future authored-cells-changed
readout is the (downgraded) handover ask (d); a first-class granular
`CellEdit` modeled edit is a non-blocking OxDoc handover ask.

No silent loss is allowed: anything outside the narrow edit scope is preserved
or rejected with a visible typed reason, never dropped.

## Upstream Work

Because this repo may not write sibling repos, W011 records upstream needs as
handovers under `docs/handovers/` and registers them as an upstream lane
(e.g. `[U-INGEST]`) in `UPSTREAM_OX_LANES.md` §C3 so the ledger stays the one
authoritative upstream surface. Local beads depend on the handover **documents
existing** (dtc-hj2.5), never on upstream implementations landing.

**Ask ranking (verified against code):**

> **Superseded-by-native-verbs update (2026-07-06, OxCalc W062 R6.7, bead
> `calc-5kqg.64`):** every ask below is now answered by a landed,
> consumer-level OxCalc verb. W011 was paused pending these; W062 R5/R6 have
> now shipped all of them on OxCalc `main` (HEAD `03bc5058` at the time of
> this note). Each mapping was verified directly against
> `src/oxcalc-core/src/consumer.rs` (grepped, not assumed) rather than taken
> from OxCalc's own docs. W011 resumes in R7, with two additional
> real-multi-sheet-xlsx prerequisites recorded below (`calc-5kqg.65`,
> `calc-5kqg.66`) beyond what the original fixture-scoped asks anticipated.

- **(b) is the first hard blocker for general workbooks:** a public
  `bind_grid_formula(source_text, channel, address, bounds) ->
  Result<GridFormulaCell, _>` wrapping the existing `pub(super)`
  `bind_grid_formula_for_transform` recipe (key =
  `BoundFormula.formula_template_identity.key`). Small, additive, independent
  of the CTRO rework's semantics. Nothing in W011's fixture waits on it.

  **SUPERSEDED.** Landed as `OxCalcDocumentContext::bind_grid_formula`
  (`consumer.rs:5084`, tagged "W062 R5.1, D4 §3" in its own doc comment at
  `consumer.rs:5065`) — signature:
  `bind_grid_formula(&self, workspace_id, node_id, address: &ExcelGridCellAddress, source_text: &str, channel: FormulaChannelKind) -> Result<Option<BoundGridFormula>, OxCalcDocumentError>`.
  This is now the one public binding authority (D4 C10: "the only key mint");
  W011's former `=`-prefix typed rejection is lifted: `EnterGridCell` binds
  formula text through it (`dtc-j7n8.6`).

- **(e) defined-name seeding is the second hard blocker, immediately behind
  (b) — ask upstream to land them together.** There is no consumer-level way
  to seed defined names (`GridOptimizedSheet::set_defined_name` /
  `set_sheet_defined_name` / `set_dynamic_defined_name` are `pub` but
  unreachable through `OxCalcTreeContext`/`GridBackingSeed`); as of
  `e069136e`, formulas referencing unseeded names deterministically evaluate
  to `#NAME?` (self-healing when the name appears). This is **not
  peripheral**: defined names are the workbook profile's named things — the
  bridge to the tree model's named nodes, the home of LAMBDA, and the core of
  the literate-notebook thesis (B1's named inputs/outputs, B3's named-path
  access). The moment (b) lands, nearly every real workbook hits name
  references — landing (b) without (e) unblocks almost nothing in practice.
  The engine side is already complete post-CTRO (name-identity edges,
  `GridDirtySeed::Name`, lifecycle reports, `#NAME?` self-healing); the ask
  is thin consumer plumbing: a `defined_names` field on `GridBackingSeed`
  (workbook- and sheet-scoped, plus dynamic) or consumer-level verbs, wired
  to the existing sheet setters. OxDoc already models the name catalog
  (`DefinedNameSpec`), so host-side name extraction at open is purely local.

  **SUPERSEDED.** Landed as a family of consumer-level verbs (W062 R5.4, D4
  §4): `set_workbook_defined_name(&mut self, workspace_id, node_id, name, target: GridRect) -> Result<(), OxCalcDocumentError>`
  (`consumer.rs:6992`) and `set_sheet_defined_name(&mut self, workspace_id, sheet_node, name, target: GridRect) -> Result<(), OxCalcDocumentError>`
  (`consumer.rs:7014`) for seeding (workbook- and sheet-scoped static names,
  with sheet-scoped shadowing workbook-scoped per R3.5 precedence), plus
  `document_defined_names(&self, workspace_id, node_id) -> Result<Vec<DefinedNameReadout>, OxCalcDocumentError>`
  (`consumer.rs:7151`) for readout. dtc-hj2.14's pending name-fixture lane
  (`TheInput -> Sheet1!A1`, `D1 = =TheInput*2`) can now activate.

- **(a) neutral ingest API — de-scoped for W011:** the host implements
  `struct OxDocGridIngest` (in dnacalc-host-core) as an
  `oxdoc_model::OxCalcIngestSink`, accumulating one `GridBackingSeed` per
  sheet and calling `set_node_grid`. The long-term OxCalc-owned ingest lane
  (new OxCalc→oxdoc-model dependency edge; axis state; feature-rendered
  regions) is raised for owner ratification, not waited on.

  **Also now superseded, beyond the original scope of this ask:** OxCalc grew
  its own `oxdoc-model`-consuming ingest module (`oxcalc-core/src/
  oxdoc_ingest.rs`, W062 R6.1-R6.5) plus a consumer-level load verb,
  `load_workbook_model` (W062 R6.5, `calc-5kqg.62`, landed `be8ef7ee`). The
  host-side `OxDocGridIngest` shim this ask originally scoped for W011 is no
  longer the load path R7 should build on; W011's host-side ingest can be
  replaced by the upstream verb directly.

- **(c) authored readout metadata — local fallback:** host keeps its own
  authored mirror keyed by `ExcelGridCellAddress`, updated at seed time and on
  every applied edit. The upstream ask (extend `OxCalcTreeGridCellReadout`
  with `authored: Option<GridAuthoredCell>` + derived per-cell editability) is
  needed for cells the host did not author — post-W011.

  **SUPERSEDED.** Landed as `OxCalcDocumentContext::grid_authored_view`
  (W062 R5.5, D4 §5, tagged in its own doc comment at `consumer.rs:4849`) —
  signature: `grid_authored_view(&self, workspace_id, node_id, window: Option<GridRect>) -> Result<Option<Vec<GridAuthoredCellReadout>>, OxCalcDocumentError>`,
  reading `GridInputState` directly (per-cell kind/source_text/channel, plus
  verb-enforced editability per contract C11) rather than the host's own
  authored mirror. The host-side local-fallback mirror this ask worked
  around is no longer needed for cells the engine authored.

- **(d) WorkbookModelOutput production — downgraded:** host assembles output
  itself (see "Save path"). Upstream ask becomes an "authored cells changed
  since epoch" readout, only if the mirror proves fragile.

  **SUPERSEDED** (upgraded beyond the "if the mirror proves fragile"
  downgrade — the full ask now has two landed verbs): `workbook_authored_delta(&self, workspace_id, since: &WorkspaceRevisionId) -> Result<WorkbookAuthoredDelta, OxCalcDocumentError>`
  (W062 R5.7, `consumer.rs:7439`) gives the "authored cells changed since
  epoch" readout directly (diffs grid-input snapshots only, per D1 C6), and
  `project_workbook_model_output(&self, workspace_id) -> Result<oxdoc_model::WorkbookModelOutput, OxCalcDocumentError>`
  (W062 R6.6, `calc-5kqg.63`, landed `03bc5058`, at `consumer.rs:7509`)
  round-trips Tier A from the model plus Tier B verbatim, with formula cached
  values read fresh from publication (C12) — this is the full
  `WorkbookModelOutput` production ask, not just the delta readout. **Now
  general (R6.66, `calc-5kqg.66`, landed):** `project_workbook_model_output`
  re-emits authored Tier-A *collections* — merged regions (`MergedCellRegions`),
  table overlays (`TableOverlay`), defined names + their Tier-B metadata half
  (`DefinedName`), and repeated/shared-formula regions (`SharedFormulaRegion`) —
  so it round-trips a collection-bearing workbook, not just the W011 fixture
  class; the former `UnprojectableTierACollections` typed refusal is removed. The
  host's own "Save path" whole-model-projection recipe (this doc, above) can now
  be replaced by `project_workbook_model_output` directly.

**The full W011 xlsx round trip is now proven at the fixture level.** OxCalc
W062 R6.6's acceptance test *is* the W011 five-step contract, run
constant-free against the real verbs above (no hand-keyed
`W011_FIXTURE_NORMAL_FORM_KEY`): load a two-cell workbook via
`load_workbook_model` -> `enter_grid_cell` literal edit -> recalc ->
`project_workbook_model_output` -> `workbook_authored_delta` reports exactly
the one changed cell -> reload the projected stream into a fresh
`OxCalcDocumentContext` and confirm authored views and published values both
agree. W011 (R7) resumes against this proven surface. Two additional
prerequisites recorded during R6 for **real** multi-sheet/collection-bearing
`.xlsx` files (beyond the two-cell fixture this doc's asks originally
scoped for) have both **landed**: **`calc-5kqg.65`** (cross-sheet reference
*evaluation* for freshly-loaded sheets — a loaded `Sheet2!B1 = Sheet1!A1+10`
now resolves rather than publishing `#VALUE!`; a general cross-sheet *range*
`#REF!` gap was spun to `calc-5kqg.67`) and **`calc-5kqg.66`** (Tier-A
collection projection — merges/tables/defined-names/repeated-regions — now
round-trips; the `UnprojectableTierACollections` refusal is removed).
Iteration-calc settings (OxDoc `WorkbookHeader` gap, filed as an OxDoc-repo
handover, `OxDoc/docs/handovers/W062-INGEST-UPSTREAM-GAPS.md`) is the
remaining real-multi-feature-xlsx prerequisite for R7.

**Local fallback boundaries.** The W011 fixture's single formula is hand-keyed
behind one named const, `W011_FIXTURE_NORMAL_FORM_KEY =
"excel.grid.v1:cell:R[0]C[-1]*3"` (the exact pair proven in OxCalc's own
consumer test, re-verified passing on `e069136e`), with a follow-up bead to
delete it when the OxCalc binding API lands. Hand-keying is **forbidden** for
any non-fixture formula; ingest returns a typed error for any formula cell it
cannot hand-key. (`attach_demo_grid`'s pairs turn out to be engine-harmless —
the key is opaque and never parsed — but its relative key misdescribes its
absolute source reference, and CTRO deepened the key's role as template
identity; it remains a pattern not to copy.)

**OxDoc asks are confirm-shaped, not build-shaped:** confirm the
whole-model-projection save contract and wasm32 byte-slice sources; raise the
granular `CellEdit` modeled edit and a wasm-lean zip feature selection only if
the wasm spike fails. The default assumption stands: OxDoc owns package
read/write; the host uses it, never duplicates it.

**Timing:** the CTRO rework has LANDED (OxCalc `e069136e`, 2026-07-04) — the
files ask (b) touches are committed and stable, so the binding handover can be
raised and implemented upstream immediately; there is no longer a sequencing
constraint. `stash@{0}` (unreviewed, touches `consumer.rs`) is the only
outstanding overlap — if it lands, fast-re-verify the consumer surface.
Handovers still specify contracts (signatures), not file locations, as
general hygiene; line-number evidence is re-verified at authoring time
(current: `bind_grid_formula_for_transform` at `optimized_sheet.rs:7241`).

**Defined names — the W011 boundary and the readiness lane.** Within W011 the
gap is contained, loudly: name-using formulas are unkeyable in fixture scope,
so they already fail with the typed ingest error, and dtc-hj2.6 additionally
reads the OxDoc defined-name catalog at open and surfaces a typed
names-present note — a W011 open never renders `#NAME?` for a name the file
actually defines. Two guards protect the post-(b) horizon: the save path must
never overwrite a file's cached value with a `#NAME?` that stems from
unseeded names (recorded in dtc-hj2.10's future shape), and dtc-hj2.14 keeps
a prepared name fixture (`TheInput -> Sheet1!A1`, `D1 = =TheInput*2`) as a
pending lane that activates the day the OxCalc seeding surface lands —
asserting seed → value, unseeded → `#NAME?` → self-heal, and the
cached-value guard.

## Execution Path

### Wave 0 - Register and Boundaries

Land this workset, the W011 plan, the epic, and child beads. Create the OxCalc
and OxDoc handovers before implementation depends on undocumented assumptions.

### Wave 1 - Pure Protocol and Host Core

Split Skin IR from Leptos and create `dnacalc-host-core`. Prove both compile
without Leptos (including dev-dependencies). Introduce the model-neutral
session enum before workbook code grows around tree-specific names. Run the
**oxdoc wasm spike** here (`cargo check -p oxdoc-model -p oxdoc-xlsx --target
wasm32-unknown-unknown` from this workspace, after adding the path deps) — a
failure becomes an OxDoc handover ask while there is schedule slack.

### Wave 2 - Open and Render

Implement `.xlsx` open:

1. Shell/test supplies bytes.
2. Host calls `open_host_owned_xlsx_source(Cursor::new(bytes),
   LoadProfile::full())`.
3. Host retains source/model context and load ledger.
4. Host models the workbook as one OxCalc workspace, one grid-backed node per
   sheet (`grid_id = "{workbook_id}:{sheet_id}"`).
5. Host drives ingest via its own `OxDocGridIngest` translation.
6. Host publishes a workbook `GridProjection` with authored metadata.
7. B1 renders through Skin IR.

All of Wave 2 runs native-first; the browser is not required until Wave 4.

### Wave 3 - Edit, Recalc, Save

Route `EnterGridCell` (the engine's three-way branch; `dtc-j7n8.6`) through
`apply_grid_edit(SetCell)`, update projections via `poll_grid_changes` →
`GridChanged`, and save through the whole-model-projection recipe. The save
proof (dtc-hj2.10) is **native tests over byte buffers** — it does not wait
for browser UI.

### Wave 4 - Host Realization Proofs

Add browser open/download UI, multi-skin layout proof, and the first
strict-grid profile lane. This wave makes W011 visible and keeps the
architecture honest by mounting the same document as notebook-only and
notebook-plus-companion.

## Bead Graph

Epic: `dtc-hj2` - `W011: dnacalc_host_core_xlsx_notebook_proof`.

| Bead | Purpose | Depends on |
|---|---|---|
| `dtc-hj2.1` | Register/spec anchoring | epic |
| `dtc-hj2.2` | Split pure Skin IR from Leptos mounting | `dtc-hj2.1` |
| `dtc-hj2.3` | Create Leptos-free host-core skeleton (+ wasm spike) | `dtc-hj2.2` |
| `dtc-hj2.4` | Introduce model-neutral sessions (enum) | `dtc-hj2.3` |
| `dtc-hj2.5` | Raise OxCalc/OxDoc handovers + `[U-INGEST]` lane | `dtc-hj2.1` |
| `dtc-hj2.6` | Open `.xlsx` through OxDoc into OxCalc (host-side ingest, fixture, authored-metadata IR) | `dtc-hj2.4`, `dtc-hj2.5` |
| `dtc-hj2.7` | Render read-only B1 notebook from `GridProjection` | `dtc-hj2.6` |
| `dtc-hj2.8` | `EnterGridCell` edit + recalc loop (landed as `dtc-j7n8.6`) | `dtc-hj2.7`, `dtc-hj2.5` |
| `dtc-hj2.9` | Add browser `.xlsx` open/download UI | `dtc-hj2.7` |
| `dtc-hj2.10` | Save/reopen existing-cell edits (native proof) | `dtc-hj2.8`, `dtc-hj2.5` |
| `dtc-hj2.11` | Prove notebook plus companion skin layout | `dtc-hj2.8` |
| `dtc-hj2.12` | Add strict-grid profile fixture lane | `dtc-hj2.10` |
| `dtc-hj2.13` | Align worker boundary with host-core | `dtc-hj2.3` |
| `dtc-hj2.14` | Defined-name readiness: detection now, pending name-fixture lane on upstream verb | `dtc-hj2.6` |

Note: `dtc-hj2.10` deliberately does **not** depend on `dtc-hj2.9` — the save
proof is native; the browser download click-through of the same bytes belongs
to `dtc-hj2.9` and epic closure, in either order. The `dtc-hj2.5` edges mean
"the handover documents exist to be cited", never "upstream landed".
`dtc-hj2.13` does not gate the fixture closure.

## Acceptance Tests

- Open proof: fixture `A1 = 7`, `B1 = =A1*3` appears in B1; `GridProjection`
  shows B1 `kind=Formula` with `source_text="=A1*3"`. Opening a workbook
  whose formulas use defined names yields the typed unkeyable-formula
  rejection plus a typed names-present note from the OxDoc name catalog —
  never silent `#NAME?` cells for names the file defines.
- Edit proof: edit `A1` to `10`; `B1` becomes `30` through OxCalc and the
  notebook consumes `GridChanged`. Typing `=A1*4` into `B1` is accepted by the
  engine's three-way branch (`GridCellEntered { Formula }`, `dtc-j7n8.6`) and
  makes the document save-restricted — never a silent drop.
- Save proof: reopening the saved bytes with `LoadProfile::full()` asserts all
  three: `A1 == Number(10)`; `B1` formula text `== Some("A1*3")`; **and `B1`
  cached `== Number(30)`**. The save ledger contains no `Dropped` entries for
  the fixture. An intentionally out-of-scope edit (adding a new cell `C1`) is
  rejected with `UnsupportedRoundTripFeature` before bytes are written.
- Architecture proof: B1 uses Skin IR only; no direct OxDoc/OxCalc/file calls.
- Core proof: `dnacalc-skin-ir` and `dnacalc-host-core` compile/test without
  Leptos anywhere in their dependency trees (dev-deps included).
- Layout proof: same workbook mounts as notebook-only and notebook plus
  companion sheet/inspector slot; host-core unions multi-skin grid interest
  into the single OxCalc registration.
- Strict lane: same fixture documents `full()`/`strict_values_only()`/
  `values_only()` profile behavior, including typed preserve/reject outcomes.

## Verification

Use the local checks that apply to each bead:

- `cargo build --workspace`;
- `cargo test --workspace` (host tests: `-j 1 --no-fail-fast`; record the
  pre-refactor baseline first — 8-9 known corpus failures on main — and diff
  against it after each structural bead);
- `cargo clippy --workspace -- -D warnings`;
- `cargo fmt --check`;
- `trunk build` for browser-facing changes.

W011 also needs targeted checks:

- no-Leptos dependency gate for `dnacalc-skin-ir` and `dnacalc-host-core`:
  `cargo tree -p dnacalc-skin-ir -p dnacalc-host-core -e normal,dev` contains
  no `leptos`;
- oxdoc wasm spike: `cargo check -p oxdoc-model -p oxdoc-xlsx --target
  wasm32-unknown-unknown` (result recorded in dtc-hj2.3);
- Skin IR protocol tests: serde round-trip for `EnterGridCell`, delta coverage
  (`delta_coverage_is_total`), `apply_delta` mirror application;
- host-core tests for open/edit/recalc/save command sequencing over byte
  buffers;
- W011 fixture: `fixtures/w011/a1_times_three/` — `parts/**` (readable XML) is
  the source of truth and `a1_times_three.xlsx` the committed binary for the
  click-through; host-core tests zip the parts in memory
  (`src/dnacalc-host-core/src/xlsx_fixture.rs`, through the dev-only
  `oxdoc_conformance`, never a zip crate) and
  `w011_fixture_opens_through_oxdoc_with_two_cells` pins the binary to the
  parts by OxDoc event-stream equality (dtc-j7n8.2);
- browser click-through for open/edit/recalc/download;
- OxDoc reopen assertions for saved bytes (including the cached-value
  assertion);
- strict-grid fixture lane.

Excel-anchor applies to workbook round-trip and formula/value preservation —
honestly qualified: until the U-ORACLE differential harness exists, the
Excel-anchor here is the degraded stopgap (reopen our bytes via OxDoc and
assert invariants), not a real-Excel differential. Formal verification remains
a standing design aim but does not gate the first host proof.

## Non-Goals

- Do not build a broad Excel importer in W011. The first target is host
  lifecycle proof, not general workbook fidelity.
- Do not classify `=`-prefixed text host-side (the engine's three-way branch
  owns it; `dtc-j7n8.6`); do not reimplement `normal_form_key` derivation
  host-side.
- Do not let the notebook call OxDoc, OxCalc, browser file APIs, or host
  internals directly.
- Do not create tree-named core crates for new generic infrastructure. Naming
  rule: new generic infrastructure = `dnacalc-*`; tree-specific legacy =
  `dnatreecalc-*`.
- Do not write sibling repo files from DnaTreeCalc. Use handovers.
- Do not silently save unsupported workbook edits.
