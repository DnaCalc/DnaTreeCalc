# W011 - DnaCalc Host Core + B1 `.xlsx` Notebook Proof

## Status

This is the execution plan for workset
`W011_dnacalc_host_core_xlsx_notebook_proof`. The chat typo `dnascalc` is
normalized to `dnacalc` because this work pivots the new core naming away from
the tree-only model.

W011 is not a replacement for W010. W010 remains the parked UDF-hosting
workset. W011 is the immediate integration proof that OxDoc and OxCalc can be
hosted together cleanly by a DnaCalc host and surfaced through Skin IR.

## Goal

Build the first visible reference host for the full stack:

1. Open a small `.xlsx` file in the browser.
2. Load it through OxDoc with `LoadProfile::full()`.
3. Keep the OxDoc source/model context owned by the host.
4. Create or reset the OxCalc workbook context owned by the same host.
5. Pass the neutral workbook model into OxCalc through an OxCalc-owned ingest
   API.
6. Publish workbook sheets and cells through Skin IR.
7. Render them in a B1 Pluto-style notebook skin.
8. Edit a grid cell through `WorkspaceIntent::EditGridCell`.
9. Recalculate dependents through OxCalc and emit `GridChanged`.
10. Save/download a round-tripped `.xlsx` through OxDoc.

The proof fixture is intentionally small: `A1 = 7`, `B1 = =A1*3`. The proof is
complete when editing `A1` to `10` makes `B1` render as `30`, and the saved
workbook reopens through OxDoc with `A1` changed and `B1` formula text
preserved.

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

## Current Code Pointers

These observations are the starting map for implementers:

- `src/dnatreecalc-skin-framework` currently mixes pure Skin IR and Leptos
  mounting. Pure protocol types such as `WorkspaceState`, `WorkspaceIntent`,
  deltas, identity, selection, `GridProjection`, and session-channel logic sit
  beside Leptos signal/context code. W011 splits this into `dnacalc-skin-ir`
  and `dnacalc-skin-leptos`.
- `WorkspaceIntent` already carries `SetGridInterest`; `WorkspaceDeltaChange`
  already carries `GridChanged` and `GridOverlaysChanged`. W011 extends this
  shape with `EditGridCell { grid, row, col, content }`.
- `GridProjection` already carries grid identity, bounds, cells, projection
  epochs, overlays, and differential-clean state. W011 needs authored cell
  metadata in the projection/readout: empty/literal/formula, source text,
  source channel, and editability.
- `src/dnatreecalc-host` is currently tree-session shaped.
  `TreeWorkspaceSession` owns an `OxCalcTreeContext`, projection maps, grid
  interest, persistence, and worker-facing execution. W011 extracts a
  Leptos-free `dnacalc-host-core` and introduces model-neutral sessions:
  `CalcModelSession`, `RichTreeSession`, and `WorkbookSession`.
- `src/dnatreecalc-worker` and host worker proxy code currently depend on the
  existing host/framework split. W011 should move the worker protocol toward
  the pure Skin IR and host-core boundary rather than keep worker logic tied to
  Leptos host state.
- `src/dnatreecalc-skins` already has grid-facing reference material,
  especially SheetLens and inspector-style skins. B1 should reuse the Skin IR
  windowing and delta machinery, but B1 is a workbook notebook skin, not a
  direct clone of the current sheet lens.
- The browser shell is `.dnatree` and localStorage oriented today. W011 adds a
  true `.xlsx` byte lifecycle through host commands.

Sibling repo pointers are read-only from this repo:

- OxDoc already exposes host-owned open/save concepts, including
  `open_host_owned_xlsx_source`, `HostOwnedXlsxSource`, `XlsxPackageSession`,
  `WorkbookModelContext`, `WorkbookModelOutput`, `XlsxSaveRequest::round_trip`,
  and `write_save_request`.
- OxCalc has substantial grid machinery, including authored `GridFormulaCell`
  data, grid edits, grid views, grid interest, and formula binding helpers, but
  the neutral `oxdoc-model` ingest/readout/output contract still needs to be
  exposed through public APIs for this host proof.

## Target Architecture

### `dnacalc-skin-ir`

Pure Skin IR crate. It has no Leptos dependency and owns the UX protocol:

- identity types and stable projection keys;
- `WorkspaceState` and sub-projections;
- `WorkspaceIntent`;
- `WorkspaceDeltaChange`;
- grid interest and `GridProjection`;
- selection state;
- session-channel protocol and delta application.

This crate is the interface between the host core, browser UI, worker, future
CLI/MCP transport, and every skin.

### `dnacalc-skin-leptos`

Leptos adapter crate. It owns UI mounting concepts only:

- `WorkspaceSkin`;
- `SkinContext`;
- skin registry;
- skin-state handles;
- Leptos signal adapters;
- view mounting and composition helpers.

It depends on `dnacalc-skin-ir`; the pure IR crate never depends back on it.

### `dnacalc-host-core`

Leptos-free reference host crate. It owns the root context:

- active document identity;
- OxDoc source package session and model context;
- OxCalc tree or workbook context;
- dirty state and save ledgers;
- command execution;
- Skin IR snapshot and delta publication;
- single-skin and multi-skin layout state.

The host command surface starts with:

- `OpenXlsxBytes`;
- `SaveActiveXlsx`;
- `SetSkinLayout`;
- `DispatchWorkspaceIntent`.

Current `dnatreecalc-host` should become an adapter over this core where that
is cleaner than preserving the old tree-shaped crate boundary.

### Model-Neutral Sessions

The host core should not bake in tree-only naming. It needs a small model
session abstraction:

- `CalcModelSession`: common host-owned command/snapshot/delta lifecycle;
- `RichTreeSession`: current tree workspace model;
- `WorkbookSession`: strict-grid workbook model loaded from `.xlsx`.

Skin IR should speak in model-neutral terms where possible. Tree-specific and
workbook-specific projections can exist, but the host protocol must support a
single skin or multiple skins over either model family.

## Upstream Work

Because this repo may not write sibling repos, W011 records upstream needs as
handovers under `docs/handovers/` and local beads depend on those handovers.

OxCalc needs:

- a public neutral `oxdoc-model` ingest API, depending on `oxdoc-model` and not
  `oxdoc-xlsx`;
- formula binding/normalization from A1/R1C1 workbook source into
  `GridFormulaCell`;
- authored grid readout metadata: empty/literal/formula, source text, source
  channel, and editability;
- a neutral `WorkbookModelOutput` path for existing-cell literal/formula edits.

OxDoc changes are expected only if the existing `WorkbookModelOutput` and
modeled edit contract cannot express the narrow save path. The default
assumption is that OxDoc already owns the package read/write machinery and the
host must use it, not duplicate it.

## Execution Path

### Wave 0 - Register and Boundaries

Land this workset, the W011 plan, the epic, and child beads. Create the OxCalc
and OxDoc handovers before implementation depends on undocumented assumptions.

### Wave 1 - Pure Protocol and Host Core

Split Skin IR from Leptos and create `dnacalc-host-core`. Prove both compile
without Leptos where required. Introduce the model-neutral session shape before
the workbook code grows around tree-specific names.

### Wave 2 - Open and Render

Implement `.xlsx` open:

1. Browser/shell supplies bytes.
2. Host calls OxDoc with `LoadProfile::full()`.
3. Host retains source/model context and load ledger.
4. Host initializes a workbook OxCalc context.
5. Host drives OxCalc ingest from neutral model access.
6. Host publishes a workbook `GridProjection`.
7. B1 renders through Skin IR.

### Wave 3 - Edit, Recalc, Save

Add `EditGridCell`, route it through OxCalc, update projections, and save
existing-cell literal/formula edits through neutral `WorkbookModelOutput` plus
OxDoc round-trip save. Unsupported workbook edits must be preserved or rejected
with a ledger entry, never silently dropped.

### Wave 4 - Host Realization Proofs

Add browser open/download UI, multi-skin layout proof, and the first strict-grid
profile lane. This wave makes W011 visible and keeps the architecture honest by
mounting the same document as notebook-only and notebook-plus-companion.

## Bead Graph

Epic: `dtc-hj2` - `W011: dnacalc_host_core_xlsx_notebook_proof`.

| Bead | Purpose | Depends on |
|---|---|---|
| `dtc-hj2.1` | Register/spec anchoring | epic |
| `dtc-hj2.2` | Split pure Skin IR from Leptos mounting | `dtc-hj2.1` |
| `dtc-hj2.3` | Create Leptos-free host-core skeleton | `dtc-hj2.2` |
| `dtc-hj2.4` | Introduce model-neutral sessions | `dtc-hj2.3` |
| `dtc-hj2.5` | Raise OxCalc/OxDoc handovers | `dtc-hj2.1` |
| `dtc-hj2.6` | Open `.xlsx` through OxDoc full profile into OxCalc | `dtc-hj2.4`, `dtc-hj2.5` |
| `dtc-hj2.7` | Render read-only B1 notebook from `GridProjection` | `dtc-hj2.6` |
| `dtc-hj2.8` | Add `EditGridCell` and recalc loop | `dtc-hj2.7`, `dtc-hj2.5` |
| `dtc-hj2.9` | Add browser `.xlsx` open/download UI | `dtc-hj2.7` |
| `dtc-hj2.10` | Save/reopen existing-cell edits | `dtc-hj2.8`, `dtc-hj2.9`, `dtc-hj2.5` |
| `dtc-hj2.11` | Prove notebook plus companion skin layout | `dtc-hj2.8` |
| `dtc-hj2.12` | Add strict-grid profile fixture lane | `dtc-hj2.10` |

## Acceptance Tests

- Open proof: fixture `A1 = 7`, `B1 = =A1*3` appears in B1.
- Edit proof: edit `A1` to `10`; `B1` becomes `30` through OxCalc and the
  notebook consumes `GridChanged`.
- Save proof: downloaded workbook reopens through OxDoc; `A1` changed, `B1`
  formula text preserved, no silent ledger drops.
- Architecture proof: B1 uses Skin IR only; no direct OxDoc/OxCalc/file calls.
- Core proof: `dnacalc-host-core` compiles/tests without Leptos.
- Layout proof: same workbook mounts as notebook-only and notebook plus
  companion sheet/inspector slot.
- Strict lane: same fixture documents full/strict/values-focused profile
  behavior, including safe preserve/reject outcomes.

## Verification

Use the local checks that apply to each bead:

- `cargo build --workspace`;
- `cargo test --workspace`;
- `cargo clippy --workspace -- -D warnings`;
- `cargo fmt --check`;
- `trunk build` for browser-facing changes.

W011 also needs targeted checks:

- no-Leptos dependency check for `dnacalc-skin-ir` and `dnacalc-host-core`;
- Skin IR protocol tests for `EditGridCell` and `GridChanged`;
- host-core tests for open/edit/recalc/save command sequencing;
- browser click-through for open/edit/recalc/download;
- OxDoc reopen assertions for saved bytes;
- strict-grid fixture lane.

Excel-anchor applies to workbook round-trip and formula/value preservation.
Formal verification remains a standing design aim, but does not gate the first
host proof.

## Non-Goals

- Do not build a broad Excel importer in W011. The first target is host
  lifecycle proof, not general workbook fidelity.
- Do not let the notebook call OxDoc, OxCalc, browser file APIs, or host
  internals directly.
- Do not create tree-named core crates for new generic infrastructure.
- Do not write sibling repo files from DnaTreeCalc. Use handovers.
- Do not silently save unsupported workbook edits.
