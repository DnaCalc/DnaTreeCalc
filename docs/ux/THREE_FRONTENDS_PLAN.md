# Three front-ends over an idealised strict-excel substrate — skin & substrate plan

> **PLAN 1 of 2 (local / DnaTreeCalc).** The upstream Ox\* lanes (`[U-xxx]`) referenced throughout are specified in [`../interop/UPSTREAM_OX_LANES.md`](../interop/UPSTREAM_OX_LANES.md). This doc owns the DnaTreeCalc-side substrate, the canonical contracts, and the three skins.
>
> Provenance: split from an approved planning exercise, hardened by two read-only design passes (an Ox\*-stack capability audit and a design-deepening fan-out + fidelity critic). Related: [`SKINS.md`](SKINS.md) (skin doctrine), [`../model/CORE_MODEL_SPEC.md`](../model/CORE_MODEL_SPEC.md), [`../interop/EXCEL_EXPORT_AND_REPLAY.md`](../interop/EXCEL_EXPORT_AND_REPLAY.md).

## Context & intent

We are building **a literate, reactive, strict-Excel substrate you can skin three ways and publish as a real `.xlsx`.** One calculation model — cells, defined names, ListObjects, spill, strict-Excel formulas — sits underneath, and three radically different front-ends render it: a **Pluto-style literate notebook (B1)**, an **interactive model dashboard + builder (B2)**, and a **headless CLI/MCP transport (B3)**. The work also **proves and expands what the Skin IR can cover**.

The novelty is a collision of two worlds. Pluto/Observable **reactivity** — a dependency graph where changing one input ripples deterministically through every dependent, no manual recalc, no hidden execution order — is exactly what a spreadsheet engine already *is* internally, but spreadsheets bury that graph under a grid. OxCalc surfaces it as a first-class object (`DependencyGraphProjection`), so we get Pluto's reactive model *for free*, backed by **Excel-identical calculation** rather than a bespoke interpreter. The result is a literate, reactive computing environment whose every artifact is a real spreadsheet. That buys three things no existing tool offers together: **one model, many surfaces** (notebook for a researcher, locked-down app for a stakeholder, JSON API for an agent — switching is free because all three read the same projection); **published `.xlsx` apps** (a dashboard built in B2 *is* a workbook with form controls, hidden gridlines, protected cells — you hand someone a file, not a hosted app); and **agent-drivable models** (an LLM opens a model, lists inputs, sets values, recalcs, reads outputs over MCP with the *same* engine a human drives).

The discipline that makes this honest is the **three-layer separation** — CALC never carries presentation, OVERLAY never touches calc, ANNOTATION is non-calculating text. Hold that line and "everything round-trips to Excel" stays true by construction.

**Scope:** substrate-first; CLI/MCP home deferred (lean: in-workspace crates); `.xlsx` export is north-star — OxDoc owns the round-trip. The plan splits into PLAN 1 (this doc) and PLAN 2 (upstream lanes), and builds **B1 ∥ B3 in parallel** on one spine, B2 trailing its lanes.

---

## The unifying idea: one transport-agnostic model protocol

All three skins and all four bindings are **views over a single protocol**, not three apps sharing a library:

- **Read side** — `WorkspaceState` (`src/dnatreecalc-skin-framework/src/workspace.rs`): `NodeView`, `NodeValueProjection`, `TableProjection`, windowed `GridProjection` + read-only `GridOverlayBundle`, `DependencyGraphProjection`.
- **Write side** — the closed `WorkspaceIntent` enum (`intent.rs`): `EditContent`, `EditTableCell`, `AddTableRow`, `SetGridInterest`, `Recalculate`, …
- **Delta stream** — `WorkspaceDeltaChange` (`GridChanged`, `GridOverlaysChanged`, `ValuesChanged`, …); "viewing is subscribing" via `SetGridInterest`.
- **Headless `SessionEngine`** — the Leptos-free `apply`/`snapshot`/`init`/`export` core generalized from `session.rs`, reusing the serializable `WorkerInbound`/`WorkerOutbound` pair (`src/dnatreecalc-worker/src/lib.rs`).

The **four bindings** (web in-process, worker `postMessage`, CLI stdio NDJSON, MCP tools) are transports carrying the same intents/projections; the **three renderers** are views over the same `WorkspaceState`; and the **surface manifest** is the one overlay every consumer reads. Build the spine once; everything else is a projection or a transport.

## Design tenets (each constrains a PLAN 1 item *and* a PLAN 2 lane)

1. **Strict layer separation.** B1's outline nesting lives in CustomXml, never a node field recalc can read; `U-CXML` exposes CustomXml strictly as overlay parts, never projecting a value into a cell/name.
2. **Overlay never touches calc.** B2's builder placing a slider changes only the drawing layer + manifest; the slider's *linked cell* is the only calc contact — legitimate input. `U-CTRL` models the **modern** control's `fmlaLink` (the `controlPr`/`x14:formControlPr` attribute, held as raw text) so the control is provably absent from the dependency graph.
3. **Only three mutation surfaces:** (1) scalar/name value, (2) editable variable-size literal array input (≠ spill), (3) ListObject edit. Formula-result spills are read-only; "editing a dynamic array" is only a user-written `LAMBDA`/UDF. `U-ARR` delivers (2) distinctly from spill; `U-DEP` keeps spill children read-only.
4. **Derive, don't store.** Input/output classification is computed from `NodeContentKind` + dep-graph counts — never a stored tag. `U-DEP` guarantees the graph publishes completely enough for the helper to be total.
5. **No shims, upstream-first.** Where substrate is missing, skins degrade to an honest primitive (text-literal cells, `{…}` array constants, `node_order`, raw export) and wait on the lane — never skin-local JSON faking a model concept.
6. **Everything round-trips to `.xlsx`.** `U-WRITE` widens the writable surface behind a no-silent-loss `DocumentFidelityLedger`; `U-ORACLE` proves identical calc against real Excel.
7. **Profile-aware (strict-excel target).** Tree-only forms bake to strict-Excel on export (`U-REF`); skins surface profile violations pre-emit.

---

## Canonical contracts (single source of truth — all skins defer to these)

### C1. `SurfaceManifestProjection` — the one overlay
Lives on `WorkspaceState` beside `scenarios`/`sweeps`; derived from CustomXml (`[U-CXML]`) or the interim SkinState store; **never** stored as node fields.

```rust
pub struct SurfaceManifestProjection {     // sibling of ScenarioManifestProjection (workspace.rs)
    pub schema_version: u32,               // ONE version line across both stores (Gap 5)
    pub active_surface: Option<String>,
    pub surfaces: Vec<SurfaceProjection>,
    pub skin_state: SkinStateManifest,     // per-skin opaque bag (replaces serde-outside-model)
}
pub struct SurfaceProjection {
    pub id: String, pub title: String,
    pub sheet: Option<NodeId>,                     // None = workspace-wide notebook
    pub sections: Vec<SurfaceSectionDescriptor>,   // narrative/outline order
    pub inputs:   Vec<SurfaceInputDescriptor>,
    pub outputs:  Vec<SurfaceOutputDescriptor>,
    pub view_state: SurfaceViewStateDescriptor,    // GATED U-VIEW; default no-op
}
pub struct SurfaceSectionDescriptor {              // OUTLINE NESTING lives ONLY here (F3)
    pub id: String, pub title: Option<String>,
    pub parent: Option<String>, pub order: Vec<SurfaceItemRef>, pub collapsed: bool,
}
pub enum SurfaceItemRef { Node(NodeKey), Prose(NodeKey), Name(String), Range{sheet:String,a1:String} }
pub struct SurfaceInputDescriptor {
    pub id: String,                        // stable id for Update/Remove + path resolution
    pub path: String,                      // cached human address ("Inputs.Rate") — B3 resolves
    pub binding: SurfaceBindTarget,        // the LINKED CELL/name — legitimate input
    pub widget: SurfaceWidget, pub label: Option<String>, pub layout: SurfaceLayout,
    pub control_descriptor_id: Option<String>,     // -> drawing-layer control, GATED U-CTRL
}
pub struct SurfaceOutputDescriptor {
    pub id: String, pub path: String, pub source: SurfaceBindTarget,
    pub view: SurfaceOutputView, pub label: Option<String>, pub layout: SurfaceLayout,
}
pub enum SurfaceBindTarget {               // ONE binding enum
    Cell(NodeKey), Name(String), Range{sheet:String,a1:String}, ArrayInput{name:String}, // ArrayInput GATED U-ARR
}
pub enum SurfaceWidget {                    // ONE widget catalog; B1 cell-kinds derive FROM this
    NumberBox{min:Option<f64>,max:Option<f64>,step:Option<f64>}, TextBox,
    Dropdown{choices:SurfaceChoices},       // honest iff persisted in manifest (F2)
    Checkbox, Spinner{min:i64,max:i64,step:i64}, Scrollbar{min:i64,max:i64,step:i64,page:i64},
    Slider{min:i64,max:i64,step:i64}, ListBox{choices:SurfaceChoices}, OptionGroup{choices:SurfaceChoices}, // GATED U-CTRL
    Array{rows:Option<u32>,cols:Option<u32>},   // GATED U-ARR
}
pub enum SurfaceChoices { Inline(Vec<String>), Range{sheet:String,a1:String} }
pub enum SurfaceOutputView {
    Scalar{format:Option<String>}, Table,
    Chart{kind:SurfaceChartKind,series:Vec<SurfaceBindTarget>,categories:Option<SurfaceBindTarget>}, // GATED U-CHART
    Sparkline{kind:SurfaceSparkKind,data:SurfaceBindTarget},                                          // GATED U-CHART
}
pub struct SurfaceViewStateDescriptor { pub hide_gridlines:bool, pub protect_non_inputs:bool, pub frozen:Option<PaneSpec> } // GATED U-VIEW
pub struct SkinStateManifest { pub by_skin: BTreeMap<String, serde_json::Value> }
```
**Addressing:** store `NodeKey` as identity (survives row/col moves); `{sheet,a1}`/`path` are resolution inputs the host maps to `NodeKey` at author time, plus a cached `path` for display/B3. Never store a raw coordinate that breaks on insert. **`SurfaceLayout` must be binder-agnostic** (`section`/`order`/`span`/`size` + an *optional* `anchor` lowering hint), not grid-pixel-shaped — so one manifest renders to a grid (B2), an OxForms form (B2a), or DOM (html host). Landing this abstract shape **before** the B2 grid binder hard-codes grid rects is the one sequencing-critical decision B2a forces (see B2a).

### C2. `node_classification` — the one classifier (kills three divergent copies)
```rust
pub enum NodeClassification { Input, FreeValue, Intermediate, Output, Empty }
fn node_classification(&self, key:&NodeKey) -> NodeClassification {
    let inc = self.dependencies.incoming_count_by_key(key);   // depended-on-by count
    match node.content_kind {                                  // workspace.rs Empty|Constant|Formula
        Empty    => Empty,
        Constant => if inc > 0 { Input } else { FreeValue },
        Formula  => if inc > 0 { Intermediate } else { Output },
    }
}
```
**Axis rule (stated once):** content-kind = literal-vs-computed; incoming count = consumed-vs-terminal; **outgoing count is not used**. B1's tint and B3's `list_*` both call this — neither re-derives. Spill children are `Formula` non-anchors, read-only by the spill marker independent of class.

### C3. Lane legend
The authoritative `[U-xxx]` table + per-lane mini-specs live in [`../interop/UPSTREAM_OX_LANES.md`](../interop/UPSTREAM_OX_LANES.md). Quick legend: **U-CXML** typed CustomXml (P0 keystone — ✅ OxDoc substrate done W005, host A2 remains); **U-DEP** dep-graph completeness (P0); **U-CTRL** modern form controls + linked cells (P1 — ✅ OxDoc substrate done W008; residual = U-ORACLE renderability + host binding; VML preserve-only); **U-NOTES** notes/comments (P1 — ✅ OxDoc substrate done W009; threaded-creation deferred; residual = OxCalc name-comment surfacing + VML-render U-ORACLE check); **U-ARR** editable array-constant input (P1); **U-CHART** charts/sparklines (P1); **U-VIEW** sheet view-state write (P2); **U-WRITE** writable names/tables/CF + fidelity ledger (P2); **U-ORACLE** differential vs real Excel (P2); **U-HOST** standalone forms/DOM host of the SessionEngine for B2a (P2, mostly shipping); **U-REF** reference-array + cross-ws bake-on-export (P3).

### C4. Two cross-cutting decisions to pin before the first manifest commit
- **`SurfaceManifestChanged(SurfaceManifestProjection)` is a fully-applicable delta (F4).** `apply_delta` patches `state.surface` in place, exactly as `GridOverlaysChanged` patches overlays; it must classify in `delta_coverage_is_total` (the real exhaustiveness gate at `session_channel.rs`). This keeps overlay edits delta-only and *structurally proves* non-interference. **This is the single most likely place the overlay/calc wall silently regresses** — if left unspecified, the coverage gate gets "fixed" by forcing full snapshots on every manifest edit.
- **Manifest↔model referential integrity (Gap 2).** On `DeleteNode`/`ReorderNode`/row-insert, `NodeKey` survives moves but **deletion leaves a dangling binding**. `SessionEngine` prunes/flags dangling `SurfaceBindTarget`s on apply and records it in the `DocumentFidelityLedger` (no silent loss). No slice owned this — it is now A2's responsibility.

---

## A1. Headless `SessionEngine` (Leptos-free dispatch core)
Generalize the reducer in `dispatcher.rs` into a signal-free engine; the Leptos `HostDispatcher` becomes a thin adapter.
```rust
pub struct SessionEngine { /* owns DnaTreeWorkspaceDocument + dispatcher + last snapshot */ }
impl SessionEngine {
    pub fn init(doc: DnaTreeWorkspaceDocument) -> (Self, WorkspaceState);   // == WorkerInbound::Init -> Ready
    pub fn apply(&mut self, env: IntentEnvelope) -> SessionResponse;        // == Intent -> Response
    pub fn snapshot(&self) -> &WorkspaceState;
    pub fn export_xlsx(&self) -> Result<Vec<u8>, ExportError>;              // U-WRITE/U-ORACLE gated
}
```
Reuses `IntentEnvelope{seq,intent}` / `SessionResponse{seq,receipt,snapshot,selection}` verbatim; public API modeled on `tests/support/programmable.rs`. **Crate boundary (Gap 4):** the engine lives in a **host-adjacent crate the host depends on** — *not* inside `dnatreecalc-host` (would pull Leptos for `oxcli`) and *not* a separate `oxcli-engine` that re-imports host internals (version-skew seam). Decide the crate before A1; B1 and B3 both depend on it.

## A2. Surface-manifest schema + persistence (in-model gated `[U-CXML]`)
Implement C1 + the C4 decisions. A `SurfaceManifestStore` trait with two impls behind one interface: the **interim** `SkinStatePersistenceStore` (localStorage / co-located JSON — the sanctioned web/wasm small-model path, *not* a shim: it stores overlay state outside the file, compromising nothing in calc) and the **target** OxDoc CustomXml part (`[U-CXML]`). `schema_version` is **one line across both stores** (Gap 5) so a Wave-1 SkinState workbook opens in Wave-2 CustomXml. Authoring intents (below) are processed as overlay mutations that **do not advance the revision** (mirror `SetGridInterest`).

> **Upstream status (verified 2026-06-29): the `[U-CXML]` substrate is DONE — OxDoc epic W005 (`oxdoc-dpq`) is closed; see [`../interop/UPSTREAM_OX_LANES.md`](../interop/UPSTREAM_OX_LANES.md) §U-CXML.** The OxDoc CustomXml `SurfaceManifestStore` impl binds to the **as-built** API, not the idealized sketch: read via `read_custom_xml_store(session) -> CustomXmlStore` (raw `source_bytes` per item + parsed `item_props{ item_id /* GUID */, schema_refs /* namespace URIs */ }`); write via `CustomXmlEdit::{AddItem, UpdateItem, DeleteItem}` through `XlsxSaveRequest::with_custom_xml_edits(...)` → `DocumentFidelityLedger`. Concretely, A2 owns the host-side glue OxDoc deliberately does not: **(1)** locate our part by filtering `item_props.schema_refs` for `urn:dnatreecalc:surface-manifest:1` (+ `:skin-state:1`) — OxDoc keys items by datastore **GUID (`item_id`)**, not namespace; **(2)** **mint the `{GUID}`** on first insert (`AddItem` requires a caller-supplied `item_id`); **(3)** serde the manifest straight from `source_bytes` (no DOM intermediary); **(4)** `UpdateItem` is GUID/rels-stable, so manifest rewrites don't perturb the rest of the package — exactly the overlay-never-touches-calc guarantee. **Sharp edge to handle honestly:** OxDoc's `validate_custom_xml_store_editable` rejects *all* customXML edits if the workbook holds any orphan/ambiguous/malformed itemProps (preserve-or-reject doctrine); the store impl must surface that rejection (and fall back to the interim SkinState store) rather than assume the write landed. The remaining proof leg — survival across a real Excel open+resave — rides `[U-ORACLE]`, not OxDoc.

## A3. Overlay IR (derive, don't store)
- **Classifier** = C2, one helper on `WorkspaceState`. Skins tint from it; nothing stored. Optional opt-in: bake classification as Excel conditional-formatting on export (default = pure view tint, zero file impact).
- **Narrative** = text-literal cells now (`SurfaceItemRef::Prose`); Notes / name-comments / text boxes gated `[U-NOTES]`.
- **Widget/output descriptors** = the C1 catalogs; their Excel-native rendering is gated `[U-CTRL]`/`[U-CHART]`.

## B1 — Pluto-style literate notebook (lowest upstream load, ships first)
A skin in `src/dnatreecalc-skins/`, registered in `lib.rs`, reusing `sheet.rs`'s windowed-grid/overlay machinery.

**Cell model** — a notebook cell is a *view* over one top-level `NodeView`; `NotebookCellKind` is a skin-local view enum (never serialized), a pure function of `node.table` + `content_kind` + `computed_value`:

| Kind | Backing | Mutable surface |
|---|---|---|
| **Prose** | text-literal cell (now) / Note (gated) | edit text → `EditContent` |
| **ScalarInput** | `Constant` scalar cell/name | `EditContent` (surface 1) |
| **ArrayInput** | `Constant` cell projecting `Array`, or `{…}` array-constant name | `EditContent` now / first-class control gated `[U-ARR]` (surface 2) |
| **Formula** | `Formula` cell — result incl. spill | **formula** editable via `EditContent`; **result never editable** |
| **Table** | ListObject (`Some(TableProjection)`) | the rich table-edit intent family (surface 3) |

This taxonomy *is* the three mutation surfaces, one-to-one. **Read-only spill affordance:** a Formula cell whose value is an `Array` renders read-only with a spill frame from `GridSpillOverlayDescriptor` (anchor-marked; `#SPILL!` if `blocked`); only the formula text is editable (= authoring a LAMBDA). **Array rendering:** collapsed `"{rows}x{cols} array"` chip; expand to a bounded 2D mini-grid from `Array{rows,cols,cells}` — editable for ArrayInput, read-only for spill. **Classification tint** = C2. **Reflow:** order = `node_order` now; custom order + outline nesting via the manifest's `SurfaceSectionDescriptor` (gated `[U-CXML]`). **`NodeView.parent/children/depth` are the tree skin's structural projection — read for order, never written by B1, never the home of outline nesting (F3).** "A cell defines sub-nodes" → a named range/block or `LET` (display-only parse; host owns `=` classification). Windowed via `SetGridInterest` → `GridChanged`/`GridOverlaysChanged`.

**Intents:** the unblocked slice needs **no new intents** — `EditContent`, `AddNode`, `ReorderNode`, `DeleteNode`, `RenameNode`, `SetNote`, the table family, `SetGridInterest`, `Recalculate`. Two **gated, presentation-only** intents touch CustomXml not calc: `SetNotebookCellOrder{order}` and `SetNotebookSection{…}` (manifest writes, `[U-CXML]`).

**Walkthrough (compressed):** prose cell; `rate=0.065`, `principal=250000` (ScalarInput, Input tint); `monthly==PMT(rate/12,360,-principal)` (Formula, Intermediate, read-only result); a `Scenarios` ListObject with a calc column; `=Scenarios[payment]*12` (Formula spill, Output, read-only `5x1` chip); `weights={0.2;0.3;0.5}` (ArrayInput). Change `rate` → one `EditContent` → recalc → downstream cells re-render in one tick. Save → reopen in Excel: names, `PMT`, the ListObject, the spill, the array constant all real; prose = text cells; nesting = preserved CustomXml.

**Unblocked now:** full taxonomy, tint, reflow-by-`node_order`, scalar/formula/prose edit, full inline table authoring, read-only spill, array collapse/expand. **Gated:** custom order/outline `[U-CXML]`; rich narrative `[U-NOTES]`; first-class array control `[U-ARR]`. *If U-CXML slips, B1 still ships fully on node order + text cells.*

## B3 — CLI / MCP headless surface (smallest spine proof, ships ∥ B1)
The falsification test for "transport-agnostic spine": drive a model to completion with nothing but the intent enum + projection.

**Transport** = NDJSON over `SessionEngine`: one `IntentEnvelope` per line in, one `SessionResponse` per line out — the *same* pair the worker carries. **Path sugar** (the only B3-specific code of substance, deliberately thin): a dotted, case-insensitive address resolved in priority `node:#id` → A1/`Sheet!A1` cell → manifest role (`Inputs.Rate`, gated `[U-CXML]`) → defined name. Never invents identity; a `set` becomes an ordinary `EditContent`. **Mutation guard:** classify the target via C2 + spill marker; refuse read-only spills/formula-results *before dispatch* and surface the engine's own `IntentError`; the three surfaces map to `EditContent` / `{…}` array text (→ `[U-ARR]`) / the table intents. **`list_inputs`/`list_outputs`** have two tiers: **derived now** (`Input`/`Output` from C2, gated only on `[U-DEP]` completeness) and **manifest-rich later** (widget/view metadata, `[U-CXML]`).

**CLI grammar:** `oxcli run <model> [--set PATH=VAL]… [--get PATH]… [--recalc] [--export OUT] [--json]` (one-shot) and `oxcli repl <model>` (warm, resident engine, incremental `delta: ValuesChanged[…]` echo). Exit codes map from `IntentError`: 0 ok; 65 read-only-spill/`FormulaTableCellEdit`; 66 unknown path; 75 `ProjectionOutOfSync` (retry after recalc); 70 gated-verb `Unsupported`. **MCP tools:** `open_model`, `list_inputs`, `set_value`, `recalc`, `get_value`, `list_outputs`, `eval`, `snapshot` — each a thin verb→intent adapter over one resident `SessionEngine` per `session_id`. **`eval` non-interference (F5):** must use a provably non-mutating path — verify `OpenCandidate` leaves `produced_revision` unchanged, else fall back to the read-only `preview_formula_bind` path. **COM-vs-simpler:** thin path sugar over the JSON/intent binding wins; a COM-style object graph would re-implement the write algebra in a second place and tempt a setter that bypasses the mutation guard. Adopt addressing convenience (A1/structured-ref/name resolution) and stop there: *addressing convenience yes, parallel mutation API no.*

**Unblocked now:** the bulk — engine extraction, NDJSON, path tiers 1/2/4, `set` + table intents + spill refusal, CLI both modes, MCP `open_model`/`set_value`/`recalc`/`get_value`/`eval` + derived `list_*`. **Gated:** rich `list_*` + named-path tier `[U-CXML]`; `list_*` correctness `[U-DEP]`; `snapshot` fidelity guarantee `[U-WRITE]`/`[U-ORACLE]`; widget metadata `[U-CTRL]`; first-class array `set` `[U-ARR]`. **Home (deferred, lean in-workspace):** `crates/oxcli-engine` (or the A1 host-adjacent crate), `crates/oxcli`, `crates/oxcli-mcp` — so the spine-parity test compiles all transports against one protocol version.

## B2 — Model surface + interactive builder (most upstream-dependent, trails its lanes)
The skin that turns the substrate's honesty into a product you can hand to someone: a clean dashboard sheet (gridlines hidden, non-input cells protected), inputs as sliders/dropdowns/checkboxes wired to input cells, outputs as numbers/charts/sparklines — *and the artifact is a real `.xlsx`*.

**Consumer surface** renders from the C1 manifest + the existing read IR. **Widget value semantics are exactly Excel's** (what makes the round-trip honest): Dropdown→data-validation list (chosen value typed into the cell); NumberBox/TextBox→direct cell edit (surface 1); Checkbox→`fmlaLink` TRUE/FALSE; Spinner/Scrollbar/Slider→`fmlaLink` bounded integer; ListBox/OptionGroup→`fmlaLink` **1-based index**, with a *visible model cell* `=INDEX(choices, linkcell)` doing the lookup (F6 — the index→value indirection is a real authored formula cell the builder writes via `EditContent`, never a manifest-private computation). All `fmlaLink`/`fmlaRange` here are the **modern** `controlPr`/`x14:formControlPr` attributes OxDoc models as `FormControlProperties` (W008) — VML is preserve-only; see `[U-CTRL]`. **Outputs are read-only** (a chart over a spill is fine *because* spills are read-only). `Number`/`Table` views project existing IR (unblocked); `Chart`/`Sparkline` need real objects (`[U-CHART]`).

**Builder** is a second mode: select cell/name → assign role/widget/range/label/layout → live preview. A new authoring intent family on `WorkspaceIntent` (sibling to `CreateScenario`/`SetScenarioOverride`), all **overlay-only, revision-inert**: `CreateSurface`, `DeleteSurface`, `SetActiveSurface`, `AssignSurfaceInput`, `UpdateSurfaceInput`, `RemoveSurfaceInput`, `AssignSurfaceOutput`, `RemoveSurfaceOutput`, `ReorderSurfaceSection`, `SetSurfaceViewState`. They write the manifest (interim SkinState → `[U-CXML]`) and emit a drawing-layer `ControlDescriptor{kind,geometry,fmla_link,…}` for U-CTRL widgets (v1 Dropdown/NumberBox need no drawing object). The only calc contact is the *consumer* later editing a linked cell — an ordinary `EditContent`. Live preview rides the `SurfaceManifestChanged` delta (C4).

**Published artifact:** two outputs, one manifest — a self-contained **WASM page** (web host + baked doc + skin; achievable now with v1 widgets) and an **`.xlsx`-with-controls** (calc model always; manifest `[U-CXML]`; controls `[U-CTRL]`; view-state `[U-VIEW]`; charts `[U-CHART]`; gated through `[U-WRITE]`'s ledger; verified by `[U-ORACLE]`).

**Phasing (honest, F2):** a **v1** of NumberBox/TextBox + Dropdown-as-`<select>` + Number/Table outputs is genuinely closer — but the v1 `<select>` is honest **only because** its `widget: Dropdown{choices}` descriptor is persisted in the manifest (the overlay-of-record); in-file `dataValidation` is `[U-CTRL]`. Every v1 manifest must deserialize unchanged once v2 variants exist (`#[serde(default)]`, as `GridOverlayBundle` already does) — v1 is a strict subset of v2, nothing rebuilt.

## B2a — inversion-of-control host realization (flagged; interface-readiness only, not built in this scope)

**Intent.** Repackage a B2 control-surface + running model into a **top-level OxVba/OxForms application** (VB6-style host, *not* embedded VBA) that *contains* one `SessionEngine` over one model/file. The shippable artifact is the running forms app, not the `.xlsx`. Same B2 idea, **app-out** topology — and a stepping stone to the "html host for a model."

**De-risked by the audit (the pattern already ships):** OxForms `oxforms-bootstrap` is a standalone native binary that owns a top-level window + run loop, loads a VBA project into OxVba, registers a Rust host object via `PortableComProjection`, and runs reduce→dispatch-VBA-handler→read-back→re-render. B2a is that machine with the Rust host object swapped for the OxCalc `SessionEngine`. `prepare_bundle_package_session`→`ProjectRuntimeSession` is the activation-style (VB6-like) load; `PortableComProjection::register_object` is the `Model.SetInput` mechanism.

**The inversion (three axes only; calc untouched):** (1) **loop ownership** Excel/grid → OxVba/OxForms host; (2) **binding locus** in-file `fmlaLink` → in-host manifest→`NodeKey`→`WorkspaceIntent` resolved at load; (3) **artifact** the file → the app that holds the file.

**`SurfaceHost`/`SurfaceBinder` trait** (consumer-side, medium-neutral; generalize the web `HostDispatcher` minus Leptos): given a `SessionEngine` + `SurfaceManifest`, generate controls from inputs, bind control↔input, subscribe to deltas, refresh outputs. One impl per medium — `GridBinder` (B2), `OxFormsBinder` (B2a), `DomBinder` (html host), `CliBinder` (B3, the degenerate case). Universal loop: control change → resolve `SurfaceBindTarget`→`NodeKey` → `engine.apply(EditContent)` → `IntentReceipt{delta}` → `apply_delta` repaints affected outputs. No binder understands calc — only manifest + delta.

**OxVba `Model` host-object façade:** register one host object whose late-bound members **are the B3 verbs** — `Model.SetInput "Rate", 0.05` (→`set_value`→`EditContent`), `.GetOutput`, `.Recalc`, `.Eval`, `.Inputs`/`.Outputs` (1-based VBA collections over the manifest, same path sugar as B3). One verb set, two idioms: NDJSON for the CLI pipe, dotted object for VBA. (Reconciles with B3's "no parallel object model": that stance is about the *CLI* idiom; VBA can consume *only* an object model, so the façade is the same verbs re-skinned, not a second engine.)

**Generic OxVba runner + html-host:** a parameterized successor to `oxforms-bootstrap` (load model + manifest from a path, not `include_str!`), two modes — **(a) Model Player:** the manifest auto-generates a UserForm (input→control, output→Label/Table/Chart), zero authored VBA; **(b) Authored form:** a user `.frm`/`.vbp` code-behind calls the `Model` façade. Swap `OxFormsBinder`→`DomBinder` and the same inversion is the **html host for a model** (mode (a) = auto HTML form on the WASM engine; mode (b) = hand-authored HTML/JS over a JS-bound `Model`).

**U-CTRL sidestep (key insight):** B2a never places a control inside the `.xlsx`; the control lives in OxForms/DOM and its write is an ordinary `EditContent` at the host level. So even though U-CTRL's substrate is now shipped (W008), its open residual — the U-ORACLE renderability verdict + VML save policy — is **NOT on B2a's path** to an interactive running model; B2a reaches the same running B2 surface using only machinery the web shell already exercises plus the already-shipping OxForms/OxVba host (lane `[U-HOST]`). `engine.export(Xlsx)` remains the independent document-out artifact (still needs U-CTRL's renderability proof + U-WRITE) the *same* app can emit later — no second code path.

**Make-now interface-readiness (nothing implemented here):** ① abstract `SurfaceLayout` (the one sequencing constraint, see C1); ② the `SurfaceHost`/`SurfaceBinder` trait spec naming the four binders; ③ assert the manifest is *sufficient to auto-generate a form* (per-input widget+binding+label+layout, per-output source+view+label+layout, sections, view_state — flag `Chart`/`Sparkline` as owner-drawn-output work with no FM20 control); ④ a short OxVba `Model` façade spec with the verb→intent table; ⑤ the host-adjacent `SessionEngine` crate with a **must-not-depend-on-`leptos`** acceptance check (so OxVba can depend on it). Upstream: the new `[U-HOST]` lane.

## P1 verification
- **Spine/headless:** drive `SessionEngine` via the programmable driver — init fixture → set → assert outputs → `export`→re-`init`; manifest serde round-trip now, `.xlsx` CustomXml round-trip once `[U-CXML]` lands.
- **Spine-parity (the key proof):** the same intent sequence through (a) programmable driver, (b) CLI NDJSON, (c) MCP must produce identical `produced_revision` chains.
- **Non-interference:** apply any authoring/manifest intent and assert the document revision and every `value_epoch` are unchanged (mirrors `SetGridInterest`).
- **Mutation discipline:** property test — every read-only target (spill non-anchor, formula result, calc table column) refuses `set` and leaves the revision unchanged; binding an input widget to a spill anchor is rejected at author time.
- **Classification gold corpus (Gap 3):** one shared fixture set (chained formulas, `SEQUENCE` spill, cross-sheet, `LET`/`LAMBDA`) that A3, B1, and B3 *all* assert against, so the three skins can never disagree about what a node is.
- **GUI (B1/B2):** browser-preview — edit a cell, watch downstream re-render in one epoch; build a 2-input/1-output surface; confirm gridlines-off dashboard.
- **Strict-excel round-trip thesis:** export the B1 walkthrough; reopen and assert names/`PMT`/ListObject/spill/array-constant present, prose as text cells, nesting as CustomXml; assert `GridProjection.differential_clean` for the reflowed window (zero calc-visible state added).
- **Baseline guard:** host tests `-j 1 --no-fail-fast`; the known pre-existing corpus failures are unchanged.

---

## Build order (4 waves) — the PLAN 1 ↔ PLAN 2 join
Lanes (`[U-xxx]`) are specified in [`../interop/UPSTREAM_OX_LANES.md`](../interop/UPSTREAM_OX_LANES.md).
- **Wave 0 (spine, all unblocked, parallel):** A1 engine carve (host-adjacent crate, Gap 4) + `ProgrammableEngineDriver`; A2 manifest type + C2 classifier on `WorkspaceState` (interim SkinState); **start U-DEP audit + U-ORACLE skeleton now.**
- **Wave 1 (B1 ∥ B3 "ship first"):** B1 full taxonomy/tint/reflow/table/spill/array-text + text-literal narrative on `node_order`; B3 core + set/get/eval/derived `list_*`. **Gate done = no-Leptos engine driver green + spine-parity test (programmable ≡ CLI ≡ MCP revision chains).**
- **Wave 2 (CustomXml persistence — `[U-CXML]` substrate already shipped, W005 closed):** the work here is now host-side only — write the OxDoc `SurfaceManifestStore` impl over the as-built `read_custom_xml_store`/`CustomXmlEdit` API (schema_refs lookup, GUID minting, editability-rejection fallback; see A2) and swap SkinState→CustomXml behind the same trait (no skin change); B1 outline + B3 named-path light up; B2 builder persists in-file. *No longer blocked on an upstream lane.*
- **Wave 3 (B2 depth, gated):** U-CTRL (OxDoc OOXML substrate already shipped W008 — work here is the host `FormControlBinding` + the **U-ORACLE renderability verdict** on VML-free modern controls), U-CHART, U-VIEW+U-WRITE, U-ARR (also upgrades B1/B3); U-ORACLE gates each acceptance.
- **Wave 4:** U-REF (P3), last.
- **B2a (parallel realization, interface-readiness only — not built in this scope):** land the abstract `SurfaceLayout` in Wave 0 (before the B2 grid binder); the `SurfaceBinder` trait + `Model` façade spec alongside B2; track `[U-HOST]` upstream (baseline already ships). Not on the B1/B3 critical path; needs no U-CTRL.

## Open integration decisions (resolve as flagged)
1. `SurfaceManifestChanged` delta semantics + `delta_coverage_is_total` arm (C4 / F4) — **before the first manifest commit.**
2. Manifest↔model referential-integrity policy on structural delete (C4 / Gap 2) — A2.
3. The shared classification gold corpus (Gap 3) — Wave 0.
4. The host-adjacent `SessionEngine` crate boundary (Gap 4) — **before A1.**
5. Store-independent manifest `schema_version` across the SkinState→CustomXml swap (Gap 5) — A2.
6. `eval` non-mutation path proven, not assumed (F5) — B3.
7. U-ORACLE degraded stopgap if COM-Excel proves too fiddly in CI (Gap 7).
8. CLI vs MCP home; whether B2 ships a data-validation-only v1 before U-CTRL.
9. **Abstract, binder-agnostic `SurfaceLayout`** (grid rects become an optional `anchor` hint) — **before the B2 grid binder** (the one B2a sequencing constraint).
10. The `SurfaceHost`/`SurfaceBinder` trait + its four binders (Grid/OxForms/Dom/Cli), and the OxVba `Model` façade as the VBA-idiom realization of the B3 verbs — interface specs to land now so B2a/html-host stay unblocked.
