# DNA Calc front-ends — UI design + route map (notebook B1 + workbook K)

> Status: DRAFT for owner review (2026-07-05). Authored against the AS-BUILT
> W062 R5 document surface in the OxCalc sibling (HEAD `8c4df820`, with R5.5/
> R5.6 verified in-flight in the working tree) and the AS-BUILT DnaTreeCalc
> substrate at `cd6b19b` + the committed C2 classifier (`6befbaf`). The engine
> surface is **frozen** for this design: every gap discovered goes to
> §"Engine asks (queue for R6+)", never to an engine redesign.
>
> **Header pin note:** the OxCalc anchor commit is `8c4df820`; R5.5
> (`db296db9`) and later wrap-up commits (including R5.9, `calc-5kqg.55`)
> have landed since. Line anchors in this doc are exact at the pinned
> commit; re-grep at HEAD before coding.
>
> **Owner decisions 2026-07-05:** F.1 (one shell, two routes), F.2 (xlsx +
> embedded manifest, no sidecar), and F.4 (`_names` hidden append-only
> sheet) are RATIFIED as the design defaults below. F.7 (browser-test
> harness) is OVERRIDDEN from "deferred" to "stand up now" — a bead (H11)
> is added to bootstrap it before the N/K UI beads run. See §F for the
> per-question decision records and §E.2/E.5 for H11's placement.
>
> Anchors: [`THREE_FRONTENDS_PLAN.md`](THREE_FRONTENDS_PLAN.md) (the approved
> strategy; this doc details its B1 and adds the strict-excel workbook skin),
> [`DNACALC_HOST_CORE_XLSX_NOTEBOOK_PROOF.md`](DNACALC_HOST_CORE_XLSX_NOTEBOOK_PROOF.md)
> (W011 — PAUSED for W062; its asks (b)/(e) are now BUILT upstream),
> [`../interop/UPSTREAM_OX_LANES.md`](../interop/UPSTREAM_OX_LANES.md),
> [`SKINS.md`](SKINS.md) (skin doctrine).

## 0. What changed under W011's feet — the as-built inventory

The W011 plan was written against an engine with no public formula binding,
no name seeding, no authored readout, and no clear verb. **All of that now
exists.** Verified against OxCalc git HEAD (`git show HEAD:src/oxcalc-core/src/consumer.rs`,
27,425 lines; working-tree diff read separately because two lanes are editing
`consumer.rs`):

| Surface | Verb / type | Evidence (consumer.rs @ HEAD unless noted) | Status |
|---|---|---|---|
| Formula binding | `bind_grid_formula(&self, ws, node, addr, source_text, channel) -> Result<Option<BoundGridFormula>, _>` | :4501 | LANDED (R5.1, `2033afea`) |
| Derived-key doctrine | authored truth stores text+channel only; keys minted, never hand-keyed | R5.2 `7a74c360` | LANDED — `W011_FIXTURE_NORMAL_FORM_KEY` is obsolete and confirmed absent from code (still quoted in the historical W011 doc text) (grep 2026-07-05) |
| Cell entry (three-way) | `enter_grid_cell(...text) -> Result<Option<GridCellEntryOutcome>, _>`; outcome `Literal{value,view} \| Formula{normal_form_key,view} \| Cleared{view}`; rejection = typed `Err(AuthoredInputDiagnostics)`, **no mutation on Err** | :4716, outcome enum :1096 | LANDED (R5.3); `Formula`'s `unresolved_names: Vec<String>` and typed `EntryRejectionDiagnostic{message, span: Option<(u32,u32)>}` land in R5.9 (`calc-5kqg.55`, engine wrap-up before Pivot A closes) |
| Typed set / clear | `set_grid_cell_value` :4808; `clear_grid_cell` :4835 (idempotent, revision-visible) | | LANDED (R5.3) |
| Defined names | `set_workbook_defined_name` :5340, `set_sheet_defined_name` :5362, dynamic variants :5380/:5401, `rename_defined_name` :5432, `delete_defined_name` :5481, `document_defined_names -> Vec<DefinedNameReadout>` :5499 (readout :140: scope/name/target Static(GridRect)\|Dynamic{source_text,channel}/is_dynamic) | | LANDED (R5.4) — W011 ask (e) satisfied |
| Authored readout + editability | `grid_authored_view(&self, ws, node, window: Option<GridRect>) -> Result<Option<Vec<GridAuthoredCellReadout>>, _>`; readout = address/kind/literal/source_text/channel/**editability** (`grid/authored.rs:505`); `GridCellEditability` = Editable \| RepeatedRegionMember{anchor} \| MergedFollower{anchor} \| SpillDisplay{anchor} \| TableStructural{table_id} (`grid/authored.rs:419`) — one classifier shared with entry-verb enforcement | working tree | LANDING NOW (R5.5) |
| Calc mode + F9 + provenance | `CalcMode::{Automatic,Manual}` (`workbook_settings.rs:53`) via `workbook_calc_settings`/`set_workbook_calc_settings` (:2362/:2386); `recalculate_workbook -> WorkbookRecalcOutcome{tick_id, drained}` (working tree); `PublishedValueProvenance::{Calculated{tick_id}, Stale{since_tick_id}, FileCached}` (`workbook_settings.rs:93`) | | LANDING NOW (R5.6) |
| Authored delta | `workbook_authored_delta(&self, ws, since: &WorkspaceRevisionId)` :5787 — typed edits-since-revision, authored truth only, replaces W011's `WorkbookEditLedger` mirror | | LANDED (R5.7, `8c4df820`) |
| Sheets + lifecycle | `sheets() -> Vec<SheetEnumerationRow>` :5741; `add_sheet` :4901, `rename_sheet` :4951 (node id stable; emits `SheetRenamed` fact), `move_sheet` :5006, `delete_sheet -> DeletedSheetFact` :5070 (meta-children-only guard; cross-sheet refs rewrite to `#REF!` in source text; undo restores) | | LANDED |
| Revision nav / undo | `workspace_revision` :5945, `navigate_workspace_revision` :5952 (typed `WorkspaceRevisionNotRetained` outside window) | | LANDED (pre-R5) |
| Interest + polling | `register_grid_interest(ws, node, GridInterestRegions{viewport, monitored})` :4416; `poll_grid_changes(ws, node, since_epoch)` :4447 | | LANDED; under Manual mode interest registration defers evaluation (T3) |
| Context rename | `OxCalcTreeContext` :2067 → **`OxCalcDocumentContext`** | D4 §1 (`docs/design/W062_D4_DOCUMENT_SURFACE_AND_INGESTION.md:124`) | R5.8 PENDING — lands before front-end coding starts; **this doc uses the new name throughout** |
| xlsx load/save | `load_workbook_model` / `project_workbook_model_output` over `oxdoc_model::DocumentEvent` | D4 Part II (§§8–14) | R6, NOT STARTED — designed-for, not depended-on (§B.9, §C.10) |

And on the DnaTreeCalc side (all paths under `C:\Work\DnaCalc\DnaTreeCalc`):

- Skin IR: `WorkspaceState` (`src/dnatreecalc-skin-framework/src/workspace.rs:19`)
  with `grids: BTreeMap<NodeId, GridProjection>` (:45); `GridProjection`
  (:788: bounds, windowed `cells`, `projection_epoch`, `overlays`,
  `overlay_epoch`, `differential_clean`); `GridCellProjection` (:901) is
  **value-only** — `row/col/value/value_epoch`, no authored kind, no source
  text, no editability. The authored-metadata IR extension is real front-end
  work (bead H3).
- `NodeClassification` (C2) is committed: enum at `workspace.rs:999`,
  derivation `node_classification()` at :197 (commit `6befbaf`).
- Intents: `WorkspaceIntent` (`intent.rs:103`) has `SetGridInterest` (:498)
  and the whole tree/table/candidate/scenario family, but **no grid-write
  intent of any kind** — the grid path is read-only end to end.
- Deltas: `WorkspaceDeltaChange` (`intent.rs:772`) carries `GridChanged
  (GridProjection)` (:790) and the narrow `GridOverlaysChanged` (:795); both
  classify fully-applicable in `is_delta_applicable`
  (`session_channel.rs:160`, grid arms at :168–171); `apply_delta` patches
  them in place (:202, tests :454/:470). Every new delta variant this design
  adds must classify there — the coverage test forces the decision.
- Sheet lens: `sheet.rs` — `grid_surface` (:907) renders a windowed,
  absolutely-positioned cell canvas inside a scroll container;
  `grid_interest_window` (:859) maps scroll→1-based window (22px rows, 84px
  cols, 1-cell overscan); scroll dispatches `SetGridInterest` per event with
  **no coalescing** (:1034) — a known must-fix before worker mode. Overlay
  boxes render read-only with clipped-edge affordances (:947–1012). The
  `?grid=1` demo attaches a 200-row fixture via `attach_demo_grid`
  (`dnatreecalc-web/src/lib.rs:224–250`; `src/dnatreecalc-host/src/app/session.rs:5482`).
- Styling: no stylesheet build; per-skin CSS constants (e.g. `SHEET_CSS`,
  `sheet.rs:1330–1441`) over `--dtc-*` design tokens; `style.rs` codifies the
  three visual rules (calc-state is the only saturated channel; provenance is
  structural; authoring is modeless 1-bit).
- Host: `TreeWorkspaceSession` (`src/dnatreecalc-host/src/app/session.rs`,
  10,061 lines, Leptos-free) owns the engine context; `HostDispatcher`
  (`src/dnatreecalc-host/src/app/dispatcher.rs`, 2,687 lines) routes intents
  (SetGridInterest arm at :302) into signals non-reactively.
- Worker: live opt-in (`?worker=1`, `dnatreecalc-web/src/lib.rs:272–299`);
  protocol `WorkerInbound::{Init{document}, Intent}` /
  `WorkerOutbound::{Ready, Response, Failed}` (`dnatreecalc-worker/src/lib.rs:17/:29`);
  `Init` still couples the worker to `DnaTreeWorkspaceDocument`.
- Crates: **no `dnacalc-skin-ir`, no `dnacalc-host-core`, no oxdoc dep yet**
  (root `Cargo.toml:3–10`, workspace deps :17–23). The W011 split beads
  (dtc-hj2.2/2.3) are still the entry ticket.
- Tests: native corpus tests under `src/dnatreecalc-host/tests/`;
  `grid_interest_dispatch.rs` (fixture → dispatcher → windowed-projection +
  delta assertions) and `tests/support/programmable.rs` are the patterns new
  beads imitate. Host tests run `-j 1 --no-fail-fast` against a recorded
  baseline of 8–9 pre-existing corpus failures.
- File I/O: localStorage only; zero file-picker/download plumbing; web-sys
  features lack `File`/`FileReader`/`HtmlInputElement`/`HtmlAnchorElement`
  (`dnatreecalc-web/Cargo.toml:26–42`).

**Decision:** this design keeps W011's crate architecture (skin-IR split +
`dnacalc-host-core` + `DocumentSession` enum) verbatim and re-scopes its
*content*: the literal-only `EditGridCell` scope is dead — the engine now
supports full cell entry, names, sheets, calc mode, and undo, so the
front-ends are designed against the whole R5 surface from day one.
**Rationale:** W011's restrictions existed *only* because the engine surface
was missing; building to the old restrictions would mean immediately
rebuilding. The xlsx open/save legs of W011 remain deferred to R6 exactly as
D4 sequences them.

---

## A. `dnacalc-host-core` — the shared SessionEngine

### A.1 Crate topology (unchanged from W011, restated as the contract)

Three new crates, exactly the W011 shapes
([`DNACALC_HOST_CORE_XLSX_NOTEBOOK_PROOF.md`](DNACALC_HOST_CORE_XLSX_NOTEBOOK_PROOF.md)
§Target Architecture):

- `dnacalc-skin-ir` — the pure protocol crate (WorkspaceState, intents,
  deltas, grid projections, selection, session channel, `Dispatcher` trait,
  `SKIN_IR_PROTOCOL_SCHEMA`). No Leptos anywhere in its tree, dev-deps
  included.
- `dnacalc-host-core` — Leptos-free host: `DocumentSession` enum
  (`RichTree(TreeWorkspaceSession)` | `Workbook(WorkbookSession)`),
  `HostCommand` surface, projection publication seam. **This is the Gap-4
  SessionEngine crate** (THREE_FRONTENDS_PLAN §A1, DECIDED line at :133).
- `dnacalc-skin-leptos` — mounting, registry, signal adapters, and the
  UI-side helper modules (`keybinding.rs`, `style.rs`, `theme.rs`,
  `accessibility.rs` move here per the W011 disposition rule).

**Decision:** `WorkbookSession` holds one `OxCalcDocumentContext` (the R5.8
name — the rename is a hard precondition for the first host-core bead so no
`Tree`-named type leaks into new code) with one workspace whose root is
`NodeRole::Workbook` and one grid-backed node per sheet, exactly the shape
`sheets()`/lifecycle verbs already manage (consumer.rs :4901–:5170).
**Rationale:** D4 §1 decided one context, no wrapper; the host adds document
identity, dirty state, and projection publication — never a second
calculation-state pot.

### A.2 Verb façade: intent → engine verb map

`WorkspaceIntent` grows a small **grid-write family**; everything else
reuses existing variants. The complete map the dispatcher implements (this
table is normative — beads cite rows, not prose):

| New/changed intent | Engine verb(s) | Receipt payload |
|---|---|---|
| `EnterGridCell { grid: NodeId, row: u32, col: u32, text: String }` | `enter_grid_cell` (:4716) | `GridCellEntered { outcome: GridEntryOutcomeProjection }` — Literal/Formula/Cleared, or the typed rejection (below) |
| `ClearGridCell { grid, row, col }` | `clear_grid_cell` (:4835) | as above (`Cleared`) |
| `SetDefinedName { scope, name, target }` (target = static rect or dynamic source text) | `set_workbook_defined_name` / `set_sheet_defined_name` / dynamic variants (:5340–:5401) | `DefinedNamesChanged` delta |
| `RenameDefinedName { scope, old, new }` / `DeleteDefinedName { scope, name }` | :5432 / :5481 | `DefinedNamesChanged` delta |
| `AddSheet { name }` / `RenameSheet { sheet, name }` / `MoveSheet { sheet, position }` / `DeleteSheet { sheet }` | :4901 / :4951 / :5006 / :5070 | `SheetsChanged` delta (+ `GridChanged` per surviving affected sheet) |
| `SetCalcMode { mode }` | `set_workbook_calc_settings` (:2386) | `CalcStateChanged` delta |
| `Recalculate` (existing, `intent.rs:121`) | workbook sessions route to `recalculate_workbook` (R5.6) | `CalcStateChanged` + `GridChanged` per drained sheet |
| `NavigateRevision` / `Undo` / `Redo` (existing) | `navigate_workspace_revision` (:5952) over the session-kept revision cursor | full snapshot (revision nav is a structural reset) |
| `SetGridInterest` (existing, :498) | `register_grid_interest` (:4416) | `GridChanged` (unchanged today) |

**Decision:** one entry verb, `EnterGridCell`, carries all cell writes; there
is **no separate formula intent** and no skin-side classification.
**Rationale:** the engine's three-way outcome (consumer.rs :1085–:1096) makes
OxFml the sole interpretation authority; a skin that pre-classifies `=`
re-implements the write algebra (the same trap B3's COM analysis names,
THREE_FRONTENDS_PLAN §B3). W011's `EditGridCell` name is retired with its
literal-only scope.

**Decision:** intent addressing uses `(grid: NodeId, row, col)` — the
projection's own coordinates — and host-core translates to
`ExcelGridCellAddress`; sheet identity stays the grid-backed node
(rename-stable per D1 C2). Skins never see engine addresses or `TreeNodeId`.

### A.3 Event/refresh model — how UI learns what changed

The spine stays "intent → `SessionResponse{receipt, delta|snapshot}`" with
the delta mirror (`session_channel.rs` `apply_delta` :202). What R5 changes
is *what the host does inside dispatch*:

1. **Mutating grid verbs return a view.** `enter_grid_cell`/`clear_grid_cell`
   return the post-edit `OxCalcTreeGridView` (consumer.rs :1065) for the
   edited sheet directly — no extra poll for the edited sheet.
2. **Cross-sheet fan-out rides `poll_grid_changes`.** After any mutating verb
   or `recalculate_workbook`, host-core polls each *other* interest-registered
   sheet with its last epoch (:4447) and emits one `GridChanged` per sheet
   that moved. The R4.6 cross-sheet recalc means an edit on Sheet1 can move
   Sheet2 cells; the poll surface is exactly the "don't re-read the world"
   tool.
3. **Three new fully-applicable deltas** (each must classify in
   `is_delta_applicable` — the coverage test at `session_channel.rs:392`
   forces it):
   - `GridAuthoredChanged { grid_node_id, cells: Vec<GridAuthoredCellProjection>, authored_epoch }`
     — complete replacement of the windowed authored layer (H3);
   - `DefinedNamesChanged(DefinedNamesProjection)` — complete replacement of
     the name catalog (H4);
   - `CalcStateChanged(WorkbookCalcProjection)` — mode + last-recalc tick +
     per-sheet dirty/stale summary (H5).
   All three are complete-replacement patches, matching the conservative
   doctrine documented at `session_channel.rs:145–158`.
4. **Projection extensions** (all `#[serde(default)]`, mirror-compatible the
   way `GridOverlayBundle` already is):
   - `GridCellProjection` gains `authored: Option<GridAuthoredCellProjection>`
     where `GridAuthoredCellProjection { kind, literal_text: Option<String>,
     source_text: Option<String>, editability: GridEditabilityProjection }` —
     a serde-clean mirror of `GridAuthoredCellReadout` (`grid/authored.rs:505`)
     with `CalcValue` rendered by the existing value projection rules;
   - `GridCellProjection` gains `provenance: Option<ValueProvenanceProjection>`
     (`Calculated | Stale | FileCached`, from `workbook_settings.rs:93`);
   - `WorkspaceState` gains `defined_names: DefinedNamesProjection` and
     `workbook_calc: Option<WorkbookCalcProjection>`.

**Decision:** authored metadata is carried **inside** `GridCellProjection`
(option-al), not as a parallel keyed map. **Rationale:** the mirror already
replaces whole windowed `GridProjection`s in place; a parallel map creates a
two-source consistency problem per cell, and the R5.5 readout is windowable
(`window: Option<GridRect>`) so the host fills authored fields for exactly
the interest window it already fetches values for.

### A.4 Error presentation model

Engine errors are data, not strings (W011 rule, kept). Host-core owns one
mapping layer, `dnacalc-host-core/src/present.rs`, from
`OxCalcDocumentContextError` to a typed `UserFacingRejection { code, summary,
detail, anchor: Option<GridCellRef>, remedy: Option<RemedyHint> }`:

| Engine error (consumer.rs) | UI presentation |
|---|---|
| `AuthoredInputDiagnostics { diagnostics: Vec<EntryRejectionDiagnostic> }` (:916; `EntryRejectionDiagnostic{message: String, span: Option<(u32,u32)>}`, R5.9) | inline under the editor: "Formula not accepted", per-diagnostic rows; a row with `span = Some(_)` highlights the span, a row with `span = None` renders **message-only, no highlight** (the UI MUST handle both — OxFml does not always have a span to offer, and R5.9 keeps that honest rather than fabricating one); **input preserved in the editor** (engine guaranteed no mutation) |
| `GridFormulaBindRejected { diagnostics: Vec<EntryRejectionDiagnostic> }` | same surface, bind-stage wording, same span-optional handling |
| `GridCellNotEditable { reason }` (R5.5) | pre-empted by affordance (cell renders read-only from `editability`), but if raced: toast + `anchor` = the classifier's anchor cell, remedy "Edit the anchor" (jump) |
| `SheetHasNonMetaChildren`, `SheetPositionOutOfRange`, `NodeIsNotSheet`, duplicate-name `Structural` | dialog-level validation messages in the sheet-tabs / names-manager UI |
| `WorkspaceRevisionNotRetained` (:946) | undo boundary: disable further undo, toast "History limit reached" |
| `DefinedNameCollidesWithTreeNode`, name-not-found `GridEngine` | names-manager inline validation |
| `WorkspaceRootIsNotWorkbook`, `UnknownWorkspace` | invariant violations: developer-grade error panel, never user-blamed |
| `IntentError::UnsupportedByModel` (host-core, W011 shape) | affordance never shown (capability-gated skins); receipt exists for transports |

**Decision:** unknown/unmapped engine errors render as a generic "The engine
rejected this change" with the Debug payload behind a disclosure — never
silently dropped, never a raw `Debug` string as the primary message.

**Decision:** rejection diagnostics (`EntryRejectionDiagnostic`, R5.9) and
bind diagnostics (§B.5's `GridBindDiagnostic`) are **distinct engine types**,
not one shared type — they arrive on different receipts
(`AuthoredInputDiagnostics`/`GridFormulaBindRejected` vs. the bind-stage
diagnostic path) and R5.9 does not unify them. They share only the
*rendering component* (`EntryDiagnostics`, §B.5/§D.1), which is written once
against the common `{message, span: Option<_>}` shape both types expose.
Pre-R5.9 Debug-formatted diagnostic strings are gone after R5.9 lands — no
skin code should format-match on them.

### A.5 Threading / WASM placement

Ground truth: the worker boundary exists and runs today behind `?worker=1`
(`dnatreecalc-web/src/lib.rs:272`), with a Leptos-free protocol
(`session_channel.rs`) and two known residues — `HostSessionExecutor`'s
throwaway signals in the worker and `WorkerInbound::Init` carrying the
tree-only document type (W011 dtc-hj2.13's scope, unchanged).

**Decision:** front-end code (both skins) binds **only** the `Dispatcher`
trait + delta mirror; engine placement is a shell concern. Ship order:
in-process main-thread engine first (the default path today), worker
alignment as a parallel non-gating bead (H10 = dtc-hj2.13 re-scoped to the
model-neutral `HostDocument` enum). **Rationale:** the fixture-scale
workbooks these skins launch with recalc in microseconds; the known perf
cliff is whole-sheet `GridEngineMode::Both` recalc on large sheets (W011
"Current Code Pointers"), which worker placement does not fix — it only
unblocks the UI thread. Do not couple skin correctness to worker landing.

### A.6 Host-core test double

`dnacalc-host-core` is tested against the **real engine** (path dep already
in the workspace, `Cargo.toml:17–23`) — no engine mocks, per the
fail-until-fixed doctrine. The `RecordingDispatcher` (skin-IR crate, W011
shape) is the *skin-side* double: it records intents and replays canned
snapshots so skin unit tests never pull the engine.

---

## B. B1 — the DNA Calc Notebook (detailed UI design)

This section (and its Entry-kind table below) **supersedes**
[`THREE_FRONTENDS_PLAN.md`](THREE_FRONTENDS_PLAN.md) §B1's cell-model table
for the workbook-profile notebook — that table was written under the
tree-model profile, before the dual-profile architecture decision; the
workbook-profile B1 design lives here from this doc forward.

### B.1 Information architecture

A notebook is **one workbook workspace rendered as a document**: a single
vertical list of *entries*, each entry backed by real workbook substance.
Four entry kinds:

| Entry kind | Backing (authored truth) | Mutable surface |
|---|---|---|
| **Name entry** — `rate = 0.065` or `monthly = =PMT(...)` | a defined name (workbook scope) whose static target is a host-allocated backing cell, or a dynamic name (formula) | name text via `RenameDefinedName`; body via `EnterGridCell` on the backing cell (static) or `SetDefinedName` dynamic re-bind |
| **Cell entry** — `Sheet1!B4 = =A1*3` | an authored grid cell without a covering name | `EnterGridCell` |
| **Table entry** | a structured table (overlay descriptor `workspace.rs:846` + backing cells) | cell edits inside the data region (`editability == Editable`); structural cells render locked |
| **Prose entry** | notebook manifest only (SkinState store; CustomXml at Wave 2) — never a model cell | edit in place; zero calc contact |

**Decision:** the notebook's primary idiom is **name entries**, with cell
entries as the escape hatch. **Rationale:** defined names are "the workbook
profile's named things — the bridge to the tree model's named nodes, the
home of LAMBDA, and the core of the literate-notebook thesis" (W011 §Upstream
Work (e)); R5.4 landed the full lifecycle, so the notebook can finally be a
Pluto-style *named* document rather than an address list. Cell entries keep
the surface total: any authored cell not covered by a name still appears
(collapsed into a per-sheet "Other cells" section) so the notebook never
hides model substance.

**Decision:** prose lives in the notebook manifest (interim
`SkinStatePersistenceStore`, target CustomXml per THREE_FRONTENDS_PLAN §A2 —
one `schema_version` line, Gap 5), not in text-literal cells.
**Rationale:** in a strict-excel workbook a text literal *is* calc-visible
authored truth (`ISTEXT`, `COUNTA` see it); the tree-model B1's
text-literal-cells answer does not transplant. ANNOTATION-layer text with
zero calc contact is exactly what the manifest store is sanctioned for; the
`[U-NOTES]` lane upgrades it to real Notes later. Entry order likewise lives
in the manifest (`SurfaceSectionDescriptor` order), defaulting to
name-declaration order + sheet/cell order for unlisted entries.

### B.2 Layout

```
+----------------------------------------------------------------------+
| DNA Calc  ▸ payroll.xlsx*        [Auto ▾] [F9 Recalc] [↶ Undo] [⋯]   |  toolbar
+------------------------------------------------+---------------------+
|                                                |  NAMES              |
|  ¶  Mortgage model                             |  ────────────────   |
|     Prose: goal, assumptions…                  |  rate      0.065    |
|                                                |  principal 250,000  |
|  ● rate                              [Input]   |  monthly   -1,580.17|
|    0.065                                       |  Scenarios (table)  |
|                                                |                     |
|  ● principal                         [Input]   |  + Add name         |
|    250000                                      |                     |
|                                                |  SHEET CELLS        |
|  ƒ monthly                    [Intermediate]   |  ▸ Sheet1 (3 cells) |
|    = PMT(rate/12, 360, -principal)             |                     |
|    → -1,580.17                                 |                     |
|                                                |                     |
|  ▦ Scenarios                          (table)  |                     |
|    | name   | rate  | payment |                |                     |
|    | base   | 6.5%  | -1,580  |                |                     |
|    | high   | 7.5%  | -1,748  |  [+ row]       |                     |
|                                                |                     |
|  ƒ annual                            [Output]  |                     |
|    = Scenarios[payment] * 12                   |                     |
|    → {5×1 array}  ▸ expand                     |                     |
|                                                |                     |
|  [+ name] [+ prose]                            |                     |
+------------------------------------------------+---------------------+
| ✓ Calculated · tick 41 · 6 cells   |  rev 12/12  |  .xlsx save: R6   |  status bar
+----------------------------------------------------------------------+
```

**v2 candidates (not in this wireframe):** `[+ table]` and `[+ cell]`
affordances are dropped from v1 — table-entry creation is explicitly out of
v1's bead set (no N-track bead authors a new `ListObject`; N3 covers name
entries only), and a bare `[+ cell]` affordance has no v1 consumer once
uncovered cells are surfaced read-only via the "Other cells" section (B.1).
Both are recorded here as v2 candidates, not designed further.

Left: the entry list (the document). Right rail: the **name panel** — the
live `DefinedNameReadout` catalog (:5499): name, scope badge (workbook/sheet),
static value or dynamic formula chip, `is_dynamic` marker. Clicking a name
scrolls to its entry. The rail collapses under 900px.

Entry anatomy (name entry shown):

```
 ● rate                                    [Input]   ⋮
 ┌──────────────────────────────────────────────┐
 │ 0.065                                        │   ← editor (one line, grows)
 └──────────────────────────────────────────────┘
   → 0.065        Calculated · tick 41               ← result row (formula entries only
                                                        show →; literal entries echo)
```

- Left gutter glyph: `¶` prose, `●` literal-backed name/cell, `ƒ`
  formula-backed, `▦` table. Classification tint (C2,
  `workspace.rs:197`) colors the *badge only* — `[Input]`, `[Intermediate]`,
  `[Output]`, `[Free]` — honoring `style.rs`'s "calc-state is the only
  saturated channel" rule.
- Result row provenance chip: `Calculated` (quiet), `Stale` (amber outline +
  "since tick N"), `FileCached` (dashed outline, "from file") — straight from
  `PublishedValueProvenance`.
- Array results render as the collapsed `{R×C array}` chip; expand = bounded
  read-only mini-grid (spill cells are `SpillDisplay` — never editable; only
  the anchor's formula text is).

### B.3 The interaction loops (exact)

**Edit-commit loop (the core loop):**

1. Click entry body (or `Enter` on focused entry) → editor state; `Esc`
   reverts buffer, no dispatch (modeless 1-bit, per `style.rs` rule 3).
2. `Enter`/blur commits → `EnterGridCell { grid, row, col, text }` at the
   entry's backing cell.
3. Receipt three-way:
   - `Literal` → entry re-renders value; glyph flips to `●` if it was `ƒ`.
   - `Formula` → glyph `ƒ`; result row from the returned view; if
     `GridCellEntryOutcome::Formula`'s `unresolved_names: Vec<String>` (R5.9,
     `calc-5kqg.55` — the engine already computes this internally and drops
     it today; R5.9 surfaces it on the receipt) is non-empty the entry shows
     a dismissable "`#NAME?` — 'TaxRate' is not defined [Create name…]" note
     (self-heals per the engine contract when the name appears).
   - Typed rejection (`AuthoredInputDiagnostics`, `Vec<EntryRejectionDiagnostic>`,
     R5.9) → editor **stays open** with the text intact (engine guaranteed no
     mutation), diagnostics listed under it; rows with a span highlight it,
     rows with `span = None` render message-only (no highlight — handled
     gracefully, not treated as a degraded case). No toast; the error lives
     where the text is.
4. Downstream entries re-render from the same tick's `GridChanged`(s) — in
   Automatic mode, one commit = one consistent repaint.
5. Committing empty text = `Cleared` → entry shows "(empty)"; deleting the
   *entry* (gutter menu) additionally deletes its covering name
   (`DeleteDefinedName`) after a confirm.

**Name loop:** `+ name` opens a two-field inline form (name, body). Host
allocates the next backing cell in the hidden `_names` block (see B.7),
dispatches `EnterGridCell` then `SetDefinedName{Static}` over it — or
`SetDefinedName{Dynamic}` directly when the user marks it dynamic. Rename
inline in the entry header → `RenameDefinedName` (engine heals references).
Validation errors (duplicate, collision, bad name text) render inline in the
form from the A.4 map.

**Decision (accepted v1 behavior):** creating a name via backing-cell +
define is **two dispatched intents** (`EnterGridCell` then
`SetDefinedName{Static}`) and therefore **two undo steps** — `Ctrl+Z` once
after creating `rate = 0.065` undoes only the name definition, leaving the
backing cell's literal in place; a second `Ctrl+Z` removes the cell write.
This is accepted, not hidden. A future host-core compound-intent
transaction (one undo step for the pair) is recorded as an Engine-ask/
host-ask (§Engine asks) and is explicitly out of v1.

**Recalc loop:** toolbar mode select (`Auto ▾`/`Manual`) →
`SetCalcMode`. In Manual: every edit still commits authored truth (engine
contract) but values go `Stale` — the status bar flips to
"● N sheets pending · press F9", stale chips appear per entry, and the F9
button gains emphasis. `F9`/button → `Recalculate` →
`WorkbookRecalcOutcome`; `drained.is_empty()` shows "Nothing to
recalculate" quietly in the status bar (the engine's suppression evidence,
surfaced honestly).

**Undo loop:** `Ctrl+Z`/toolbar → `Undo` (revision navigation). Status bar
shows the cursor ("rev 11/12"); `WorkspaceRevisionNotRetained` disables
further undo with "History limit reached". Undo restores deleted
sheets/names — the receipt's snapshot repaints everything (structural reset,
full snapshot by design).

### B.4 Keyboard model

Rides the existing unified `KeybindingRegistry`
(`skin-framework/src/keybinding.rs`; grammar-never-fires-while-typing guard
as `sheet.rs:556` does):

| Chord | Verb |
|---|---|
| `↑`/`↓` (entry focus) | move entry focus |
| `Enter` | edit focused entry; commit when editing |
| `Shift+Enter` | commit and focus next entry (notebook flow) |
| `Esc` | revert edit / clear focus |
| `Ctrl+Enter` | commit and stay |
| `F9` | Recalculate |
| `Ctrl+Z` / `Ctrl+Y` | Undo / Redo |
| `Ctrl+Shift+N` | new name entry below focus |
| `Delete` (focused, not editing) | delete entry (confirm if named) |
| `/` | jump to name panel filter |

### B.5 Diagnostics presentation

One component, `EntryDiagnostics` (shared with K's formula bar): a list of
`{message, span: Option<_>}` rows under the editor; hovering a row with
`span = Some(_)` highlights the span in the text; a row with `span = None`
renders as a plain message row with no highlight target (the component MUST
handle this gracefully — it is not an error case); the first row is
auto-focused for screen readers (`accessibility.rs` conventions). Bind
diagnostics (`GridBindDiagnostic{message, span_start, span_end}`) and entry
diagnostics (`EntryRejectionDiagnostic{message, span: Option<(u32,u32)>}`,
R5.9) are **distinct engine types carried on distinct receipts** — they are
not the same type and do not share a payload — but both expose a
`{message, span}`-shaped view, so one rendering component (`EntryDiagnostics`)
covers both.

### B.6 Empty / loading states

- **No document:** a centered starter card — "New notebook" (creates
  workbook workspace + Sheet1 + `_names` block) / "Open .xlsx" (disabled,
  badge "arrives with engine R6") / recent local documents (localStorage
  catalog, existing store).
- **Loading (worker/init):** skeleton entry rows (3 gray bars), name panel
  ghosted; no spinner-only screens.
- **Empty notebook:** one ghost name-entry with placeholder
  "name = value — try `rate = 0.065`".

### B.7 The backing-cell convention (host-core policy, not engine)

Name entries need cells. **Decision:** host-core allocates static-name
backing cells in a dedicated sheet `_names` (created lazily, hidden in the
notebook UI, ordinary sheet to the engine), one cell per name, column A,
append-only rows. **Rationale:** keeps user sheets pristine (a name entry
must not silently occupy `Sheet1!A1`), round-trips as a perfectly ordinary
sheet+names workbook in Excel, and needs zero engine features. The
allocation map is re-derivable from `document_defined_names()` (each static
readout carries its target rect) — no side ledger. Flagged as an owner
question (F.4) since it shapes the exported file.

### B.8 What the notebook is NOT (anti-scope)

No execution order, no cell "run" button (recalc is workbook-global — the
engine's dependency graph *is* the reactivity), no per-entry language
choice, no output-cell mutation ever (spill/`SpillDisplay` affordances are
read-only by classifier), no skin-side formula parsing (`=` never inspected
outside OxFml).

### B.9 xlsx story at R6

The design pre-plumbs: toolbar `Open`/`Save .xlsx` buttons exist from N1,
disabled with tooltip "Workbook file I/O arrives with engine R6"; status bar
carries the `.xlsx save: R6` note. When R6 lands
(`load_workbook_model`/`project_workbook_model_output`, D4 Part II), the
host-core beads swap the disabled affordances for
`HostCommand::OpenXlsxBytes`/`SaveActiveXlsx` (W011 §HostCommand shape,
unchanged) + the browser file plumbing (web-sys features to add:
`File`, `FileList`, `FileReader`, `HtmlInputElement`, `HtmlAnchorElement`).
`FileCached` provenance chips are already rendered from day one (they simply
never occur before R6), so loaded-not-yet-recalculated workbooks present
honestly with zero UI change.

---

## C. K — the strict-excel Workbook front-end (detailed UI design)

### C.1 Layout

```
+---------------------------------------------------------------------------+
| DNA Calc ▸ payroll.xlsx*     [Auto ▾] [F9] [↶] [↷] [Names…] [⋯]          |
+---------------------------------------------------------------------------+
| Name box ▾ |  fx  =PMT(rate/12, 360, -principal)                    ✓ ✗  |
+------+--------------------------------------------------------------------+
|      |   A         B         C         D         E         F              |
+------+--------------------------------------------------------------------+
|  1   | 0.065     250000    ┌─────────┐                                    |
|  2   |                     │ -1580.17│  ← selected: 2px accent border     |
|  3   |           ╔═ spill ═╗─────────┘                                    |
|  4   |           ║ 18962.04║                                              |
|  5   |           ║ 20975.52║  ← spill frame: dashed, cells read-only      |
|  6   |           ╚═════════╝                                              |
|  ⋮   |                                                                    |
+------+--------------------------------------------------------------------+
| ▸ Sheet1 | Sheet2 | _names | +                                            |
+---------------------------------------------------------------------------+
| B2 · =PMT(...) · Calculated t41 |  ● 1 sheet pending F9  |  rev 12/12     |
+---------------------------------------------------------------------------+
```

### C.2 Grid rendering

**Decision:** K's grid is the existing SheetLens machinery promoted, not a
rewrite: scroll container + virtual canvas sized from
`max_rows`/`max_cols`, absolutely-positioned windowed cells, overlay boxes —
exactly `grid_surface` (`sheet.rs:907–1054`), with two mandatory upgrades:

1. **Interest coalescing** (K1b): scroll events debounce/coalesce into one
   `SetGridInterest` per animation frame with the ±1-window overscan
   `grid_interest_window` already computes; the current
   per-scroll-event dispatch (`sheet.rs:1034`) is the recorded must-fix.
2. **Authored-aware cells** (K1b): a cell renders its computed value; when
   `authored.editability != Editable` it renders the classifier's affordance
   (below); "show formulas" mode (existing `SheetState::show_formulas`
   toggle) renders `authored.source_text` instead of values.

Windowing *is* the virtualization: only interest-window cells exist in the
DOM, scrollbars are sized by bounds — no additional virtual-list library.

### C.3 Cell editing loop + editability affordances

Selection model: single anchor cell (existing `SelectionState`), extended
later — range selection is explicitly **out of v1 scope** (F.6).

- Type-to-edit / `F2` / double-click → in-cell editor overlaid at the cell
  rect, mirrored live in the formula bar (one shared edit buffer).
- Commit (`Enter`/`Tab`) → `EnterGridCell`; three-way handling identical to
  B.3 (shared code); rejection keeps the editor open with diagnostics in a
  popover anchored to the formula bar.
- `Delete` on a selected cell → `ClearGridCell`.

**Keyboard / navigation model** (rides the same `KeybindingRegistry` as B.4,
kept minimal and consistent with it):

| Chord | Verb |
|---|---|
| `↑`/`↓`/`←`/`→` | move selection one cell |
| `Tab` / `Shift+Tab` | move selection horizontally (right/left) |
| `Enter` | commit edit and move selection down |
| `Shift+Enter` | commit edit and move selection up |
| `F2` | toggle edit mode on the selected cell |
| `Esc` | cancel edit (revert buffer, no dispatch) |
| `Ctrl+Z` / `Ctrl+Y` | Undo / Redo |

**v2 note:** IME/composition handling (multi-keystroke input commit,
composition-start/end suppressing the navigation chords above) is not
designed here — flagged as a v2 concern, not a v1 gap to close silently.

Editability affordances, one per `GridCellEditability` variant
(`grid/authored.rs:419`):

| Classification | Render | On edit attempt |
|---|---|---|
| `Editable` | normal | edit |
| `SpillDisplay { anchor }` | value in spill-frame tint; dashed border via existing spill overlay (`workspace.rs:886`) | flash frame + status hint "Spilled from B3 — edit the anchor"; `Enter` jumps to anchor |
| `RepeatedRegionMember { anchor }` | subtle region tint | jump-to-anchor hint ("Part of a filled region") |
| `MergedFollower { anchor }` | not separately rendered (merge overlay covers it) | selection snaps to merge anchor |
| `TableStructural { table_id }` | header/totals chrome from the table overlay | "Table header — rename via the table's header row" (table verbs, existing intent family) |

The skin never guesses: affordances derive from the projection's
`editability`, and the engine's `GridCellNotEditable` remains the backstop
for races (A.4).

### C.4 Formula bar + name box

- **Formula bar:** shows `authored.source_text` (formula) or literal text of
  the selected cell; editable; ✓/✗ commit/revert buttons (mouse parity with
  Enter/Esc). Diagnostics popover anchors here.
- **Name box:** shows the address (`B2`) or the covering defined name;
  dropdown lists `document_defined_names()`; typing a name navigates to its
  static target; typing `A1`-style text navigates. Typing a *new* name over a
  selection = "define name" fast path (`SetDefinedName{Static}` over the
  selected cell's rect).

**Decision:** for dynamic (formula-backed) names, the name box **always
opens the names manager** in v1 — there is no realized-anchor navigation.
**Rationale:** a dynamic name's target is a formula, not a fixed rect; "where
it currently resolves to" is a derived, potentially-multi-cell notion the
engine does not expose as a navigable anchor today, and guessing one
skin-side would re-implement resolution semantics. The realized-anchor
readout is recorded as an Engine-ask (§Engine asks) rather than approximated.

### C.5 Sheet tabs

Bottom tab strip over `sheets()` (:5741): click = active sheet (one
`SetGridInterest` swap; inactive sheets keep a 1-row monitored region so
their dirty badges stay live — `GridInterestRegions.monitored` exists for
exactly this); double-click = inline rename (`RenameSheet`; duplicate-name
`Structural` error renders inline); drag = `MoveSheet`; `+` = `AddSheet`;
context menu → Delete with confirm; `SheetHasNonMetaChildren` and the
delete-consequences warning ("references to this sheet become `#REF!` —
undo restores") render in the confirm dialog. The `_names` sheet renders in
K (it is an ordinary sheet; hiding it is a notebook-side choice only).

### C.6 Defined-names manager

Modal from `[Names…]`: table of `DefinedNameReadout` rows — name, scope
badge, target (rect as `Sheet1!$A$1:$B$3`, or dynamic formula text),
dynamic flag; row actions rename/delete/edit-target; "New name" form with
scope picker. All validation inline per A.4. (A "where used" column is an
engine ask — §Engine asks #3 — until then the column is absent, not faked.)

### C.7 Cross-sheet references UX

Formulas may reference `Sheet2!A1`; the engine owns resolution and
cross-sheet recalc (R4.6). K's obligations: (1) after any commit, apply the
polled `GridChanged`s for other sheets (A.3 step 2) so a visible second
sheet repaints in the same paint; (2) render `#REF!` cells (post
sheet-delete) as ordinary error values with a hover note "sheet was
deleted — undo restores"; (3) sheet rename needs no skin action (engine
emits `SheetRenamed` facts and heals authored text; the refreshed authored
window carries the new text).

### C.8 Manual mode, F9, provenance

Same components as B: mode select, F9 emphasis when pending, per-cell
provenance rendered as a corner tick only in "audit" toggle (default off in
K to keep the grid quiet — the status bar carries the aggregate "N sheets
pending"). `Stale` cells get the standing `dtc-calc--stale` treatment
(`style.rs` — calc-state is the saturated channel, so stale = the existing
stale class, not a new color).

### C.9 Undo/redo

Identical host path to B.3's undo loop. K additionally disables tab-strip
and names-manager mutations while a revision navigation is in flight
(single-flight guard in the dispatcher — intents queue, never interleave).

### C.10 xlsx at R6

Same affordance plan as B.9 — the toolbar `⋯` menu carries the disabled
Open/Save items. K is the skin where `FileCached` will be most visible
(every loaded cell pre-first-recalc); the C.8 audit toggle is designed for
that day.

---

## D. Shared concerns

### D.1 Component inventory (shared vs per-skin)

Shared (new, in `dnacalc-skin-leptos` as plain components; names final):

| Component | Used by | Substance |
|---|---|---|
| `CellEntryEditor` | B entry body, K in-cell + formula bar | one edit buffer, commit/revert, three-way receipt handling |
| `EntryDiagnostics` | B, K | diagnostics list + span highlight |
| `ValueChip` | B result row, K cells, name panel | value render via existing `value_render.rs` rules + provenance chip |
| `ProvenanceBadge` | B, K, status bars | Calculated/Stale/FileCached |
| `CalcModeControl` | B, K toolbars | mode select + F9 button + pending badge |
| `RevisionControl` | B, K | undo/redo + cursor label + retention-limit state |
| `NameForm` | B `+ name`, K names manager | name/scope/target fields + inline validation |
| `GridCanvas` | K (and B's array-expand mini-grid, read-only mode) | the promoted `grid_surface` with coalescing + authored awareness |

Per-skin: B's entry list/gutter/name rail; K's tab strip, name box, formula
bar chrome, names-manager modal.

### D.2 Styling / theming

Follow the codified rules (`skin-framework/src/style.rs` header): calc-state
is the only saturated channel; provenance is structural tint; authoring is
one border cue. Each skin ships its CSS as a constant over `--dtc-*` tokens
(the `SHEET_CSS` pattern, `sheet.rs:1330`); shared components ship
`DNACALC_SHARED_CSS` the same way. No stylesheet pipeline is introduced in
this scope. Classification badges use the existing structural tint scale,
not new colors.

### D.3 Testing strategy per layer

- **Host-core (the load-bearing tier):** native tests against the real
  engine over byte-free fixtures — imitate
  `src/dnatreecalc-host/tests/grid_interest_dispatch.rs` (fixture →
  dispatch → assert projection + delta) and drive multi-step flows through
  the `tests/support/programmable.rs` driver. Every A.2 table row gets at
  least one accept + one reject test; every A.3 delta gets an
  `apply_delta`-mirror test (imitate `session_channel.rs:454`).
- **Skin IR:** serde round-trips for every new intent/delta/projection;
  `delta_coverage_is_total` extended (it fails to compile/pass until the new
  variants are classified — that is the point).
- **Skins:** Leptos-free logic (window math, entry-list derivation,
  editability→affordance mapping) extracted into pure functions with unit
  tests (the `grid_interest_window` test at `sheet.rs:1455` is the
  pattern); component rendering verified by the existing native test style.
  **Update (2026-07-05, owner override, §F.7):** a `wasm-bindgen-test
  --headless` browser harness is stood up (bead H11) before the N/K UI beads
  run, superseding "manual click-throughs only" for mount/interaction
  smoke coverage. Every N/K bead that renders UI adds or extends at least
  one harness test for the surface it builds (verbatim rule in §E.0); manual
  Verify steps remain for anything the harness does not cover, but are no
  longer the sole check for whether a route mounts or a basic interaction
  works.
- **Suite gate (per repo, verbatim in every bead):** DnaTreeCalc:
  `cargo build --workspace && cargo clippy --workspace -- -D warnings &&
  cargo fmt --check && cargo test --workspace` with host tests `-j 1
  --no-fail-fast`, diffed against the recorded 8–9 pre-existing corpus
  failures (no new failures, no vanished tests). OxCalc is read-only for
  all beads in this doc.
- **Fail-until-fixed (verbatim policy):** a test that reproduces a real bug
  must FAIL until the bug is fixed. Never `#[ignore]` it, never weaken the
  assertion to match buggy behavior, never delete it to go green.

### D.4 Performance guardrails (standing rules)

1. Never re-read a whole grid on a keystroke: values ride the returned view
   + `poll_grid_changes` epochs; authored metadata rides the windowed
   `grid_authored_view` for the interest window only.
2. `SetGridInterest` is coalesced (≤1 per frame) before any worker-mode use.
3. The delta mirror stays authoritative: skins render from
   `WorkspaceState`, never from private caches of engine data.
4. Known engine cliff (accepted, watched): consumer recalc runs
   `GridEngineMode::Both` whole-sheet; interest scoping bounds the *readout*,
   not the recalc. Fine at fixture scale; the perf lane owns it upstream.
5. Notebook entry list re-derives only from changed projections
   (`GridChanged` carries one sheet; `DefinedNamesChanged` one catalog) —
   no full-document diff per tick.

---

## E. ROUTE MAP + BEAD STRUCTURE (the low-power-agent guardrails)

### E.0 Standing rules — copied verbatim into every bead description

> **Scope.** Do exactly this bead. If you find adjacent work, record it as a
> note for the coordinator; do NOT do it. Touching a file not listed under
> "Owns" is a bead failure unless the coordinator approves first.
> **Engine.** The OxCalc repo is READ-ONLY. If the engine surface seems to
> be missing something, STOP and report — the answer is an "engine ask",
> never a workaround that re-implements engine semantics host-side.
> **Tests.** Acceptance assertions below are the definition of done — each
> becomes a literal test (or a named manual Verify step where marked).
> A test that reproduces a real bug must FAIL until fixed; never #[ignore],
> never assert buggy behavior as correct.
> **Suite gate.** Before hand-off: `cargo build --workspace`, `cargo clippy
> --workspace -- -D warnings`, `cargo fmt --check`, `cargo test --workspace`
> (host tests `-j 1 --no-fail-fast`; the 8–9 known corpus failures on the
> recorded baseline are the only tolerated reds — zero NEW failures).
> **Fresh-eyes review clause (verbatim).** Before closing this bead, a
> fresh-eyes reviewer (a separate session/agent that did not write the code)
> re-reads the bead description, the diff, and the acceptance assertions,
> and answers in writing: (1) does the diff do exactly what the bead says,
> (2) does anything exceed the bead's file boundaries, (3) do the acceptance
> tests actually assert the stated behavior (not a weakened proxy)? The
> review verdict is pasted into the bead before close. Then commit — one
> bead, one commit (plus the review fixups).
> **Escalation.** If blocked twice on the same obstacle, stop and escalate
> to the coordinator with a written blocker note. Never improvise around a
> blocker.
> **Browser-harness rule (verbatim, applies once H11 lands).** Every N/K
> bead that renders UI adds or extends at least one harness test (H11's
> `wasm-bindgen-test --headless` lane, §F.7) for the surface it builds — a
> new route/component/interaction gets a proof test that fails if the mount
> or the interaction breaks, in addition to any native pure-fn tests the
> bead's own acceptance assertions already require. This is additive to,
> never a substitute for, the bead's stated acceptance assertions.

**Model policy.** Sonnet-class agents execute all S beads and most M beads;
the acceptance assertions + fresh-eyes clause are the safety net, and the
coordinator (owner session) reviews every commit. Beads marked ⚑ (judgment-
heavy) prefer Opus-class or get a pre-written skeleton from the coordinator.
Every bead is S or M by construction — anything that grows past M is split
by the coordinator, never "finished big".

**Bead mechanics.** Three epics under the existing `br` conventions
(`dtc-*` ids, `.N` children — the dtc-hj2 pattern): `H` (host-core), `N`
(notebook), `K` (workbook). Symbolic ids below (H1…) map to minted `dtc-*`
ids at creation; each bead body = the corresponding row of this section +
the standing rules block.

### E.1 Preconditions (not beads of this plan)

- **P0 — OxCalc R5.5/R5.6/R5.8 landed** (upstream, in flight now). H2+
  design against `OxCalcDocumentContext`; H1 does not need it.
- **P0.1 — OxCalc R5.9 (`calc-5kqg.55`) landed**, engine wrap-up bead before
  Pivot A closes. H6 specifically depends on it (`unresolved_names` on the
  Formula receipt, typed `EntryRejectionDiagnostic{message, span: Option}`
  replacing Debug-formatted diagnostics) — see H6's gating note (§B.3).
- **P1 — dtc-hj2.2 (skin-IR split) and dtc-hj2.3 (host-core skeleton +
  wasm spike)** executed as already specified in the W011 plan — they are
  re-labeled H1/H2 below with their W011 content unchanged where still true.
  The W011 beads dtc-hj2.5/2.6/2.8/2.10/2.12/2.14 (handovers, xlsx open,
  literal-only edit, save proof, strict lane, name-readiness) are
  superseded-or-deferred: xlsx legs park until R6; the handover asks (b)/(e)
  are BUILT; the coordinator reconciles the dtc-hj2 tree when minting these
  epics.

### E.2 H track — host-core (shared spine)

| Bead | Size | Title / substance | Owns (files) | Acceptance assertions (verbatim) | NON-goals | Imitate |
|---|---|---|---|---|---|---|
| **H1** | M ⚑ | Skin-IR split: create `dnacalc-skin-ir` + `dnacalc-skin-leptos`, move protocol types, `Dispatcher` trait + `RecordingDispatcher` | new `src/dnacalc-skin-ir/**`, new `src/dnacalc-skin-leptos/**`, mechanical import updates in existing crates, root `Cargo.toml` members | (1) `cargo tree -p dnacalc-skin-ir -e normal,dev` contains no `leptos`; (2) all existing workspace tests pass unchanged (baseline diff clean); (3) `keybinding.rs`/`style.rs`/`theme.rs`/`accessibility.rs` live in `dnacalc-skin-leptos`, not the IR crate | no behavior change of any kind; no new types; no renaming of protocol fields | W011 dtc-hj2.2 notes (module-level Leptos map in "Current Code Pointers") |
| **H2** | M ⚑ | `dnacalc-host-core` crate: `DocumentSession` enum, `WorkbookSession` over `OxCalcDocumentContext` (create workspace + `add_sheet`), `HostCommand` skeleton, publication seam; Send/Sync audit written into the bead | new `src/dnacalc-host-core/**`, root `Cargo.toml` | (1) no-Leptos gate as H1 for `dnacalc-host-core`; (2) native test: create workbook session → `sheets()` projects one sheet → `set_grid_cell_value(A1, 7)` → snapshot shows `7`; (3) `IntentError::UnsupportedByModel` receipt for `CreateScenario` on Workbook | no xlsx, no worker, no authored-metadata IR, no tree-session refactor beyond the enum seam; **no `EnterGridCell` handling (that is H6) — H2's write-path acceptance uses `set_grid_cell_value` only** | `tests/grid_interest_dispatch.rs`; W011 §`dnacalc-host-core` |
| **H3** | M | Authored-metadata IR: `GridAuthoredCellProjection` on `GridCellProjection` (`#[serde(default)]`), `GridAuthoredChanged` delta, host fill from `grid_authored_view` for the interest window | `src/dnacalc-skin-ir/src/workspace.rs`, `intent.rs`, `session_channel.rs`; `dnacalc-host-core` grid publication module | (1) `delta_coverage_is_total` passes with the new variant classified fully-applicable; (2) serde round-trip: a pre-H3 serialized `GridProjection` deserializes (defaults); (3) native test: formula cell projects `kind=Formula`, `source_text="=A1*3"`, `editability=Editable`; spill member projects `SpillDisplay{anchor}` | no provenance field (H5); no skin changes; no name projections | `GridOverlayBundle` `serde(default)` pattern (`workspace.rs:822`); mirror test at `session_channel.rs:454` |
| **H11** | M | Browser-test harness bootstrap (§F.7 override): stand up `wasm-bindgen-test --headless` for `dnatreecalc-web`; one proof test per shell route — app mounts (`mount_dnatreecalc`), and a smoke interaction (the `?grid=1` demo grid renders and a cell click selects) | new `src/dnatreecalc-web/tests/browser_smoke.rs` (or `tests/` module per wasm-bindgen-test convention), `src/dnatreecalc-web/Cargo.toml` (dev-dep: `wasm-bindgen-test`), no other files | (1) `cargo test -p dnatreecalc-web --target wasm32-unknown-unknown --headless --firefox` (or `--chrome`) exits green locally; (2) a mutation check: temporarily breaking `mount_dnatreecalc` (e.g. mounting to a nonexistent element id) makes the mount proof test fail — prove this once during the bead, then restore; (3) the demo-grid smoke test fails if `attach_demo_grid` or the cell-click→selection path breaks (same mutation-check treatment) | no coverage targets; no visual regression; no CI wiring; no Playwright; no exhaustive per-component tests (that's every later N/K bead's own addition per the standing-rules browser-harness rule) | `grid_interest_dispatch.rs` for the "fixture → drive → assert" shape, adapted to the browser DOM instead of native `Owner` |
| **H4** | S | Defined-names projection: `DefinedNamesProjection` on `WorkspaceState`, `DefinedNamesChanged` delta, host fill from `document_defined_names`; intents `SetDefinedName`/`RenameDefinedName`/`DeleteDefinedName` mapped per A.2 | same IR files as H3 (name sections), `dnacalc-host-core` names module | (1) coverage + serde as H3(1)/(2); (2) native test: `SetDefinedName` static → projection lists it with scope+rect; rename → old gone, new present; delete → dependents show `#NAME?` value in grid projection; recreate → self-heals on next tick; (3) duplicate name → typed rejection receipt, projection unchanged | no dynamic-name UI semantics beyond passthrough; no B/K components | H3's own diff (lands first) |
| **H5** | M | Calc mode + provenance + recalc: `WorkbookCalcProjection`, `CalcStateChanged` delta, `provenance` on cells, `SetCalcMode` intent, `Recalculate` routed to `recalculate_workbook` for workbook sessions | IR files (calc sections), `dnacalc-host-core` calc module | (1) coverage + serde gates; (2) native test: Manual mode → edit → cell provenance `Stale{since}` and values unchanged → `Recalculate` → `Calculated{tick}` and value updated; (3) `Recalculate` with nothing dirty → receipt carries `drained_any == false`, no `GridChanged` emitted | no F9 UI; no FileCached sourcing (R6) — the variant exists, unpopulated | H3 pattern |
| **H6** | M | Cell-entry intents end-to-end: `EnterGridCell`/`ClearGridCell` + three-way receipt projection + A.4 error mapping module (`present.rs`) | IR intent/receipt types, `dnacalc-host-core/src/present.rs` + dispatch module | (1) native tests: literal / formula / empty→Cleared / rejected (`=1+`) → receipt carries typed diagnostics AND a re-read of authored view proves no mutation; (2) `unresolved_names` surfaces in the Formula receipt; (3) every A.4 table row has a mapping test (unknown variant → generic rejection, never panic) | no cross-sheet poll fan-out (H7); no skins | OxCalc consumer tests named in W011 ("grid_edit_setcell_and_fillrange…") as choreography reference — read-only |

**Gating note:** H6 starts after R5.9 lands (it is in the engine wrap-up
before Pivot A closes) — `unresolved_names` and the typed
`EntryRejectionDiagnostic` shape (§A.4) are both R5.9 deliverables H6's
acceptance tests assert against.
| **H7** | M | Sheet lifecycle + cross-sheet fan-out: `AddSheet`/`RenameSheet`/`MoveSheet`/`DeleteSheet` intents; post-mutation `poll_grid_changes` fan-out emitting per-sheet `GridChanged` | IR intent types, `dnacalc-host-core` sheets module + dispatch | (1) native test: two sheets, Sheet2!A1 = `=Sheet1!A1*2`; edit Sheet1!A1 → exactly two `GridChanged` deltas (one per sheet) in one response; (2) delete Sheet1 → confirm-required receipt path → Sheet2 cell projects `#REF!`; undo → restored; (3) rename → formula text in authored projection shows new sheet name without any skin-side rewrite | no tab-strip UI; no interest-union logic changes | `grid_interest_dispatch.rs` |
| **H8** | S | Revision/undo routing for workbook sessions: `Undo`/`Redo`/`NavigateRevision` → `navigate_workspace_revision`; retention-limit receipt | `dnacalc-host-core` revision module | (1) native test: edit → undo → authored view shows pre-edit truth and values re-derive; redo restores; (2) undo past retention → typed `WorkspaceRevisionNotRetained`-backed receipt, session state unchanged | no UI; no tree-session undo changes | H5 pattern |
| **H9** | S | Notebook manifest store: entry order + prose in `SkinStatePersistenceStore` (interim), one `schema_version` line | `dnacalc-host-core` skin-state module (+ IR manifest types) | (1) serde round-trip incl. unknown-field tolerance; (2) prose edit does not advance the document revision (assert `produced_revision` unchanged — the non-interference proof, THREE_FRONTENDS_PLAN §P1) | no CustomXml (Wave 2 of the plan); no notebook UI | `SetGridInterest` revision-inert handling in dispatcher |
| **H10** | M (parallel, non-gating) | Worker alignment: model-neutral `HostDocument` init enum, `WorkerInbound::Command` — dtc-hj2.13 re-scoped | `src/dnatreecalc-worker/src/lib.rs`, `dnatreecalc-web/src/worker_client.rs`/`worker_runtime.rs`, host-core executor seam | (1) `?worker=1` click-through parity for the N/K flows (manual Verify); (2) worker crate no longer depends on `dnatreecalc-host` | no perf work; no protocol redesign | existing `WorkerProxyCore` |

### E.3 N track — notebook

| Bead | Size | Title / substance | Owns | Acceptance assertions | NON-goals | Imitate |
|---|---|---|---|---|---|---|
| **N1** | M | Read-only notebook skin: entry-list derivation (names + uncovered cells + tables), gutter glyphs, C2 badges, name rail; registered in skins lib | new `src/dnatreecalc-skins/src/notebook.rs` (+ registry line in `lib.rs`) | (1) pure-fn test: given a snapshot with 2 names + 1 uncovered cell + 1 table, `derive_entries` returns 4 entries in manifest-then-default order (**default order until H9's manifest lands; the manifest-order assertion moves to a post-H9 test**); (2) badge = `node_classification` output for the backing key, asserted per entry kind; (3) spill result renders `{R×C array}` chip, never editable (manual Verify + unit test on the entry model) | zero mutation dispatches; no editor; no prose | `sheet.rs` structure; `value_render.rs` for values |
| **N2** | M ⚑ | Entry editor + commit loop: `CellEntryEditor` + `EntryDiagnostics` shared components, three-way receipt handling, unresolved-name note | `src/dnacalc-skin-leptos/src/components/cell_entry.rs`, notebook.rs wiring | (1) commit literal/formula/empty drives exactly one `EnterGridCell` (RecordingDispatcher assertion); (2) on rejection receipt the editor retains the text and shows diagnostics (component state test); (3) Esc reverts without dispatch | no name creation (N3); no K usage yet | keybinding guard pattern `sheet.rs:545–626` |
| **N3** | M | Name entries + name rail actions: `+ name` form, rename inline, delete-with-name confirm, `_names` backing-cell allocation via host policy | notebook.rs, `components/name_form.rs`, host-core allocation helper (one function, H-track reviewed) | (1) creating `rate = 0.065` dispatches `EnterGridCell` then `SetDefinedName` with the allocated `_names` cell (recorded order asserted); (2) duplicate-name rejection renders inline, form stays open; (3) rename dispatches `RenameDefinedName` only | no dynamic-name authoring UI (defer; static + "advanced: dynamic" behind one flag); no names-manager modal (that is K5) | N2's components |
| **N4** | S | Prose entries + entry ordering via H9 manifest | notebook.rs prose section | (1) prose create/edit/reorder round-trips through the manifest store; (2) revision non-interference test rides H9(2) (cite, don't duplicate) | no markdown rendering beyond plain paragraphs (F.5) | H9 |
| **N5** | S | Calc-mode UI: `CalcModeControl`, provenance chips, stale styling, F9 | `components/calc_mode.rs`, notebook.rs toolbar/status | (1) Manual+edit renders Stale chip and pending badge (snapshot-driven component test); (2) F9 dispatches `Recalculate` exactly once; `drained_any == false` renders the quiet no-op note | no K wiring | `style.rs` calc-state classes |
| **N6** | S | Undo UI + revision cursor + history-limit state | `components/revision.rs`, notebook.rs | (1) Ctrl+Z dispatches `Undo`; (2) retention-limit receipt disables the control with the "History limit reached" note | no redo-branch visualization | keybinding registry |
| **N7** | S | Empty/loading/starter states + disabled xlsx affordances (B.6/B.9) | notebook.rs, shell starter card | (1) no-document state renders starter card with Open-.xlsx disabled + R6 badge (manual Verify + component test); (2) new-notebook creates workbook + Sheet1 via intents only | no file plumbing of any kind | existing shell catalog UI |
| **N8** | S | Keyboard polish per B.4 table + a11y pass on entries/diagnostics | notebook.rs, keybinding registrations | (1) every B.4 chord resolves to its verb via the registry (table-driven test); (2) diagnostics list is focusable and announced (manual a11y Verify) | no new chords beyond the table | `keybinding.rs` |

### E.4 K track — workbook

| Bead | Size | Title / substance | Owns | Acceptance assertions | NON-goals | Imitate |
|---|---|---|---|---|---|---|
| **K1a** | M ⚑ | `GridCanvas` promotion: extract sheet-lens grid into the shared component, unchanged behavior — pure extraction, no new capability | `src/dnacalc-skin-leptos/src/components/grid_canvas.rs`, `sheet.rs` (consume it), new `src/dnatreecalc-skins/src/workbook.rs` shell | (1) existing sheet-lens tests still pass (baseline diff clean); (2) `GridCanvas` renders byte-identical output to the pre-extraction `grid_surface` for a fixed fixture (snapshot/behavior-parity test) | no interest coalescing (K1b); no authored-aware rendering (K1b); no editing (K2); no tabs; no formula bar | `grid_surface` (`sheet.rs:907`) — this IS that code, moved |
| **K1b** | M | Interest coalescing (≤1 `SetGridInterest`/frame) + authored-aware cell render + show-formulas mode, on top of the K1a extraction | `src/dnacalc-skin-leptos/src/components/grid_canvas.rs`, workbook.rs shell | (1) coalescing unit test: N scroll events in one frame → one dispatch (pure scheduler fn test); (2) `SpillDisplay` cell renders read-only affordance from projection (unit test on the cell-render fn); (3) show-formulas mode renders `authored.source_text` instead of values | no editing (K2); no tabs; no formula bar | K1a's own diff (lands first) |
| **K2** | M | Cell edit loop: in-cell editor + shared `CellEntryEditor`, editability affordance table C.3, jump-to-anchor | workbook.rs, grid_canvas edit overlay | (1) each `GridCellEditability` variant maps to its C.3 affordance (table-driven unit test on the mapping fn); (2) commit/reject/clear loop assertions as N2(1–3) rerun in K context; (3) edit attempt on `SpillDisplay` dispatches nothing and focuses anchor | no range selection; no fill-drag | N2 components |
| **K3** | S | Formula bar + name box | workbook.rs top strip | (1) selection sync: cell↔bar buffer is one state (unit test); (2) name-box entry `rate` navigates to the name's target; unknown text `Q99` navigates to address; (3) new-name fast path dispatches `SetDefinedName` | no range names | C.4 spec |
| **K4** | M | Sheet tabs + lifecycle dialogs (C.5) | workbook.rs tab strip, `components/sheet_tabs.rs` | (1) add/rename/move/delete each dispatch exactly their A.2 intent (RecordingDispatcher); (2) delete confirm shows the `#REF!` consequence text; duplicate rename renders inline error; (3) inactive-sheet dirty badge updates from monitored-region `GridChanged` (native test via H7 fixture) | no sheet hiding; no color tabs | H7 fixture |
| **K5** | M | Defined-names manager modal (C.6) | `components/names_manager.rs`, workbook.rs | (1) lists exactly `DefinedNamesProjection` rows with scope/target rendering incl. dynamic formula text; (2) rename/delete/create paths dispatch per A.2 with inline validation per A.4; (3) no "where used" column exists (explicit assertion of absence — it is an engine ask) | no name auditing/usage search | N3 `NameForm` |
| **K6** | S | Manual mode + F9 + provenance audit toggle (C.8) | workbook.rs status/toolbar | (1) as N5(1–2) in K context; (2) audit toggle off by default; on → provenance ticks render from cell `provenance` only | no per-cell recalc menu | N5 |
| **K7** | S | Undo/redo + single-flight guard (C.9) | workbook.rs | (1) as N6; (2) mutation intents dispatched during in-flight revision nav are queued, not dropped (dispatcher-level test) | no branch UI | N6 |
| **K8** | S | Cross-sheet polish: `#REF!` hover note, same-paint multi-sheet repaint Verify, `_names` visible-in-K check | workbook.rs | (1) H7(1) fixture drives a two-sheet visible repaint (manual Verify + delta-count native assertion); (2) `#REF!` note renders from value + tombstone note | nothing new host-side | H7 |

### E.5 Sequencing DAG + parallel-safe sets

```
P0 (OxCalc R5.5/5.6/5.8) ──┐
H1 ── H2 ──┬── H3 ──┬── H11 ─┬── H6 ──┬── H7 ──── H8 ── H9 ── H10(parallel anytime ≥H2)
           │        │        │        │
           │        ├── H4   │        ├─(N track)  N1 ── N2 ── N3 ── N4
           │        └── H5   │        │                 └─ N5 ── N6 ── N7 ── N8
           │                 │        └─(K track)  K1a ── K1b ── K2 ──┬─ K3
           │                 │                                ├─ K4 ── K8
           │                 │                                ├─ K5
           │                 │                                └─ K6 ── K7
           │                 └── H11 gates every N/K bead below (harness rule, §E.0)
```

**H11 placement note:** H11 lands after H2 (the crate/shell skeleton it
mounts must exist) and after H3 (the demo grid's authored-aware smoke path
is more meaningful once H3's authored projection exists, though H11's
literal acceptance only needs the pre-H3 `?grid=1` fixture already in the
tree — H3 is a soft, not hard, dependency). H11 gates the **start** of every
N/K bead: N1/K1a are the first N/K beads and both begin only after H11 is
green locally, because the standing-rules browser-harness rule (§E.0)
requires every UI-rendering N/K bead to add or extend a harness test, which
presupposes the harness exists.

Parallel-safe sets (no shared file ownership):
- **Set A (after H2):** H3 ∥ H4 ∥ H5 — disjoint IR sections + host modules;
  the coordinator merges IR-file edits (same files, different sections) by
  landing H3 first, then H4/H5 rebase (stated in each bead).
- **H11** lands after H3, before any N/K bead starts (see placement note
  above) — it is on the critical path to N1/K1a, not parallel to them.
- **Set B (after H3+H11+H6):** N1 ∥ K1a (different skin files); K1b follows
  K1a sequentially (same files).
- **Set C:** N2..N8 sequential within N; K2..K7 mostly sequential within K;
  N and K tracks fully parallel to each other after Set B.
- **H10** parallel any time after H2; gates nothing.
- MVP milestone "notebook demo" = H1–H3 + H11 + H6 + N1–N3 + N5.
  MVP milestone "workbook demo" = +H7 + K1a–K4 + K6.

### E.6 Why this survives low-power execution (design notes for the owner)

The three historical failure modes and their counters: **scope creep** →
per-bead Owns lists + NON-goals + the fresh-eyes question (2); **weakened
tests** → acceptance assertions are phrased as the literal test content and
the fail-until-fixed clause is in every bead; **improvised engine
workarounds** → the READ-ONLY engine rule + "blocked twice → escalate"
gives an agent no legitimate path to reinventing `normal_form_key`-era
hacks. Every bead names a concrete file to imitate, because pattern-copying
is the highest-reliability instruction for Sonnet-class execution.

---

## F. Open questions for the owner (defaults chosen, flag to override)

1. **One shell or two routes? — DECIDED (2026-07-05, owner).** Original
   default (for the record): one shell, both skins registered
   (`SkinRegistry`), switchable — matching the SKINS.md multi-skin doctrine
   and the W011 "notebook + companion" layout proof; override option was to
   ship B and K as separately-branded entry points. **Ratified: ONE shell,
   two routes** — the design default stands as written; no separately-branded
   entry points. No further change to §A/§D required — this is what the doc
   already designs against.
2. **Notebook file format. — DECIDED (2026-07-05, owner).** Original default
   (for the record): the workbook itself is the file (post-R6, `.xlsx` +
   notebook manifest in SkinState→CustomXml); no `.dnanb` sidecar format;
   override option was a standalone notebook file for pre-R6 shareability.
   **Ratified: xlsx + embedded manifest, no sidecar** — confirms B.1/B.9/H9
   as written (manifest lives in SkinState now, CustomXml at Wave 2; no
   sidecar format is introduced at any point).
3. **Visual identity.** Defaults follow the existing `--dtc-*` token set and
   the three `style.rs` rules; no new palette. Owner may want a distinct
   notebook look (serif prose? wider measure?) — cosmetic, safe to defer.
4. **`_names` backing-sheet convention (B.7). — DECIDED (2026-07-05,
   owner).** Original default (for the record): hidden-in-notebook `_names`
   sheet, column A, append-only; flagged because it shapes exported files.
   **Ratified: `_names` hidden append-only sheet, column A** — B.7's
   convention is confirmed as the exported-file shape; no rename, no
   alternate layout.
5. **Prose richness.** Default: plain paragraphs now; markdown subset later.
6. **K v1 selection model.** Default: single-cell only; range selection +
   fill-drag deferred (fill maps to the engine's `FillRange` op when it
   comes). Flag if range ops are demo-critical.
7. **Browser-automation test harness. — DECIDED (2026-07-05, owner):
   OVERRIDE, stand up NOW.** Original default (for the record): none in this
   scope, manual Verify steps per bead; override option was a
   wasm-bindgen-test or Playwright lane as a pre-bead to H1. **Ratified:
   override taken** — the owner wants automated UI guardrails in place for
   the low-power executor rounds, not deferred. The harness is stood up
   **before the N/K UI beads run** via new bead **H11** (§E.2), sequenced
   after the shell exists (H2/H3; see §E.5 DAG). This is no longer "none in
   this scope" — see the Decision/Rationale immediately below and H11's row.

   **Decision:** v1 harness is **`wasm-bindgen-test` in `--headless` browser
   mode**, run locally per-crate against `dnatreecalc-web` — not Playwright,
   not a hosted CI lane.
   **Rationale (ground-truthed against this repo, 2026-07-05):**
   - No wasm-bindgen-test usage, no `Trunk.toml` /
     `[package.metadata.trunk]`, and no WebDriver binaries exist anywhere in
     this repo today (grep across all `Cargo.toml`s and the tree came back
     empty) — there is no existing browser-test infrastructure to extend,
     only a build-tool *mention* in `docs/ux/TECHNICAL.md:25`.
   - The mount entry point (`mount_dnatreecalc`,
     `src/dnatreecalc-web/src/lib.rs:202`, via `leptos::mount::mount_to` at
     :30/:317) targets a real `web_sys` DOM element — this genuinely
     requires a wasm32 + DOM environment to exercise; the repo's existing
     native `#[test]`s (e.g.
     `src/dnatreecalc-host/tests/grid_interest_dispatch.rs`, which uses a
     bare `leptos::prelude::Owner` with no DOM) prove component *logic* today
     but cannot prove the app actually mounts or that a click reaches the
     DOM. A mount/interaction smoke test needs a browser, full stop.
   - The toolchain is already present with zero new installs:
     `wasm32-unknown-unknown` is an installed rustup target, and
     `wasm-bindgen-cli 0.2.117` (matching the pinned
     `wasm-bindgen = "=0.2.117"` in `src/dnatreecalc-web/Cargo.toml`) is
     already globally installed, including `wasm-bindgen-test-runner.exe`.
     Only a local browser/WebDriver pairing (Firefox+geckodriver or
     Chrome+chromedriver) needs adding — a one-time local setup, not a new
     dependency in the dependency graph, and it satisfies the LOCAL_EXECUTION
     doctrine (`AGENTS.md` §"Build, test, verify": local checks, no CI) far
     more directly than standing up Playwright (a Node toolchain this
     all-Rust workspace does not otherwise need).
   - Playwright was considered and rejected for v1: it would require
     introducing an entire Node/npm toolchain into an all-Rust workspace
     purely to drive a page that `wasm-bindgen-test --headless` can already
     drive natively from `cargo test`.

   **Invocation (exact command developers/agents run):**
   ```
   cargo test -p dnatreecalc-web --target wasm32-unknown-unknown --headless --firefox
   ```
   (or `--chrome` if geckodriver is unavailable locally; both are supported
   by `wasm-bindgen-test-runner` — pick whichever browser/driver pair is
   installed on the executing machine). This must exit green locally before
   H11 and every H11-consuming N/K bead closes (§E.0 suite-gate rule extends
   to include this command once H11 lands).
8. **dtc-hj2 reconciliation.** These epics supersede parts of the W011 bead
   tree (E.1/P1). Default: coordinator closes/re-parents the affected
   dtc-hj2 children with cross-references when minting H/N/K.

## Engine asks (queue for R6+ — recorded, never worked around)

1. **Sheet-level authored epoch** — a cheap "authored truth changed since E"
   per grid, so hosts refresh `grid_authored_view` windows only when needed
   (today: refresh on every mutating receipt for the edited sheet;
   `workbook_authored_delta` is revision-grained, heavier than needed for
   per-keystroke UI).
2. **Aggregated cross-sheet change poll** — one `poll_workbook_changes`
   returning per-sheet packets, replacing the host's N-sheet poll fan-out
   (A.3 step 2). Convenience, not correctness.
3. **Defined-name usage readout** ("where used") for the names manager
   (K5/C.6) — dependents of a name identity are engine-known post-CTRO;
   a consumer readout would let the manager warn before delete.
4. **Dynamic-name realized-anchor readout** (C.4) — for a dynamic
   (formula-backed) defined name, a consumer-facing "where does this
   currently resolve to" readout, so the name box could navigate to a
   realized anchor instead of always opening the names manager (v1's
   default, §C.4). Not requested for v1; recorded for a later pass.
5. **Compound-intent transaction** (host-core or engine-level) — a single
   undo step covering the name-creation pair (`EnterGridCell` +
   `SetDefinedName`, §B.3 Name loop) instead of today's two. Explicitly out
   of v1; either an engine-level transaction primitive or a host-core
   compound-intent wrapper would satisfy this.
6. **R6 as designed** (D4 Part II): `load_workbook_model` /
   `project_workbook_model_output` — the only blockers for B.9/C.10 xlsx
   affordances; plus OxDoc-side granular cell modeled edit (already on D4
   §15's upstream list).

*(R5.9 (`calc-5kqg.55`) resolves what were previously engine asks here —
`GridCellEntryOutcome::Formula.unresolved_names` and typed
`EntryRejectionDiagnostic{message, span}` on `AuthoredInputDiagnostics` and
`GridFormulaBindRejected` — via engine enrichment ahead of front-end coding;
they are no longer on this queue. Nothing else in the R5 surface required a
redesign request — the front-end design above consumes it as frozen.)*
