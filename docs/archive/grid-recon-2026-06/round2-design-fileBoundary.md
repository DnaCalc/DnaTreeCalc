# DESIGN PROPOSAL — File-I/O Repo ↔ OxCalc Boundary Contract

## 0. Headline shape

A **types-only contract crate** (working name `oxdoc-model`, leaf crate of the new file repo's workspace, zero IO, zero OxCalc deps) defines a two-tier surface: a full-fidelity `WorkbookDocumentModel` and a **geometry-aligned typed event stream** for bulk cell traffic. The file repo owns ZIP/XML/round-trip; OxCalc consumes/produces only contract types through a new `grid_io` ingest/export seam on the grid facet. **OxCalc takes a direct dependency on `oxdoc-model`** — mirroring the TreeCalc→`WorkbookConstructionSpec` precedent ("direct API calls, no shims", `docs/interop/EXCEL_EXPORT_AND_REPLAY.md:5,112`). Sans-IO discipline: `Read+Seek`/`Write` appear only inside the file repo; the engine sees byte-free typed data.

## 1. Direction + shape

**Read = pull, write = push.** Load: file repo exposes `DocumentReader::next_event() -> Option<DocumentEvent>` (pull; consumer paces, enabling bounded-memory streaming and early abort). Save: `DocumentWriter::write_event(DocumentEvent)` (push; engine-side exporter walks block storage in key order). Both directions share one event vocabulary:

```rust
enum DocumentEvent {
    WorkbookHeader { date_system, calc_mode, schema: WireSchemaId }, // "oxdoc.document_event.v1"
    StringTable(Vec<SharedStringEntry>),        // guaranteed before any CellChunk
    StyleTable(Vec<ResolvedStyleSpec>),         // ditto; xf-chain pre-resolved, theme refs kept symbolic
    SheetBegin { sheet_id, name, props }, SheetEnd { sheet_id },
    ColumnProps(Vec<AxisRun>), RowProps(Vec<AxisRun>),   // run-length, (start, len, Props)
    CellChunk { row_band: u16, cells: Vec<(PackedCellAddr, CellPayload)> },  // sorted, one 256-row band
    SharedFormulaRegion { region_id: u32, anchor, extent, r1c1_text: String },
    TableOverlay(TableSpec), DefinedName(DefinedNameSpec), ExternalLink(ExternalLinkSpec),
    CalcChainHint(Vec<SheetCellRef>),           // advisory only, ignorable
    OpaquePartNotice { part_name, kind },       // presence signal; bytes never cross
}
```

`CellPayload`: `Number(f64) | Bool | Error(u8) | SharedText(u32) | InlineText(String) | Formula { region: Option<u32>, text: Option<String>, cached: Option<Box<CellPayload>> } | RichStub(u32)`. This mirrors `CompactCell` (`.grid-recon/designs-storage.md:21`) so ingest is near-memcpy, with a defined lossless lowering to the `oxfunc_value_types.aligned_json.v1` wire for evidence emission only — JSON never carries bulk cells.

**Chunk geometry:** 256-row bands matching block row-bands (`designs-storage.md:11`); xlsx is row-major so the reader buffers ≤1 band; the engine splits bands into 32-col blocks trivially; on export, merging col-bands within a row-band is a linear walk of `BTreeMap<BlockKey, Arc<Block>>`. Contract guarantees `StringTable`/`StyleTable` precede cells (file repo owns `Seek`; reads parts in dependency order via the ZIP central directory).

**Extend or supersede `WorkbookConstructionSpec`?** Neither replaces the other. The spec (`EXCEL_EXPORT_AND_REPLAY.md:118-166`) stays OxXlPlay's COM-construction input — per-cell `Vec`, right at verification scale, wrong at 10^7 cells. `oxdoc-model` reuses its leaf vocabulary verbatim (`DefinedNameSpec`, `TableSpec`, `CfRuleSpec`, `DateSystem`, `CalcMode`) and adds what construction never needed (string/style tables, shared-formula regions, axis runs, cached values, opaque parts). Ship a **declared-lossy lowering** `WorkbookDocumentModel::to_construction_spec()` so the three-way conformance triangle (file-write vs COM-build vs Excel-observed) is one function call.

**OxCalc side:** `grid_io::SheetGridIngest` builder on the sheet facet (sibling of `SetTableShape`, `OxCalc/src/oxcalc-core/src/structural.rs:144`): `ingest_chunk`, `ingest_template_region` (→ `GridTemplateCatalog`, `designs-calcgraph.md:15`), `ingest_axis_runs`, `seal(mode)`. Export walks blocks + `GridTemplateCatalog` + the style/string interners, emitting events. Hosts orchestrate which file maps to which workspace; the engine never sees a path.

## 2. Fidelity mapping table

| xlsx construct | Engine construct | Notes |
|---|---|---|
| `<f t="shared" si ref>` | `GridFormulaTemplate` region (`designs-calcgraph.md:14`) | si→region_id; non-rect groups split to maximal rects; `t="array"` distinct payload flag |
| sharedStrings.xml | workbook interned string table (`CompactCell` u32 id) | indices pass through; original si order recorded for write stability |
| cellXfs + numFmts/fonts/fills/borders | interned `StyleId(u32)` table (`designs-storage.md:25`) | file repo pre-resolves xf chains into `ResolvedStyleSpec`; theme colors stay symbolic (Derived if resolved) |
| `<col min max .../>`, `<row ht hidden outlineLevel>` | axis runs `Vec<(start,len,Props)>` (`designs-storage.md:27`) | cols already run-shaped; rows coalesced by reader |
| definedNames | `DefinedNameSpec` → `HostNameResolver` binding | formula text verbatim, zero-rewrite (`CORE_MODEL_SPEC.md:531`) |
| xl/tables/*.xml | grid-overlay table facet (Excel profile, owner decision 1) | `TableSpec` shape; column formulas → templates over column extent |
| calcChain.xml | `CalcChainHint` → warm-start scheduling seed | never trusted for correctness; on write, regenerate from last run order or omit |
| `<v>` cached values | computed/published block layer (`designs-storage.md:23`) | epoch class `FileCached`; enables load-without-recalc |
| vbaProject.bin, drawings, pivotCache, customXml | opaque preserved parts (file repo only) | engine sees `OpaquePartNotice` |

## 3. Lossiness / provenance ledger

Three dispositions, ledgered per part/feature: **PreservedOpaque** (raw bytes + relationship edges retained, rewritten verbatim — engine never sees), **Projected{status: Direct|Derived, loss: Option<LossKind>}** (crossed the boundary; e.g., theme-resolved color = Derived), **Dropped{reason}** (file repo refuses only malformed/unsupported encodings). **Profile rejection is not the file repo's job**: it surfaces everything projectable; the engine's profile machinery (CapabilityOverlay, strict-excel gating) decides acceptance and the engine-side **IngestReport** records per-region/cell rejections with typed reasons. Ledger home: `DocumentFidelityLedger` emitted by the file repo per load/save, schema-versioned, reusing the OxXlPlay honesty vocabulary (`SurfaceStatus`, `CaptureLoss`, `oxxlplay-abstractions/src/lib.rs:71-107`) and `projection_status: Lossless|Lossy` (`oxreplay-bundle/src/lib.rs:33`). Host pairs ledger+IngestReport; for evidence, both embed as bundle sidecars. Hard invariant: **Lossy is always declared, never silent** (locked decision 4).

## 4. Modes

1. **Values-only fast load:** stream `<v>`+strings+minimal styles; formula text retained raw-unparsed in the authored layer (upgradeable later); no bind, no graph. Viewer-grade open.
2. **Full load:** everything; shared-formula regions ingested as host-declared templates; engine verification pass may additionally coalesce R1C1-identical loners (owner decision 4 — both authorship modes flow through the same `ingest_template_region`).
3. **Incremental save:** file repo retains a `DocumentSession` (part map + source bytes); engine supplies dirty sheets/blocks from value epochs / `WorkspaceRevision` deltas (`treecalc.rs:160-179`); untouched parts copied byte-identical.
4. **Export of never-opened model:** synthesize minimal package (Content_Types, rels, default theme; omit calcChain); engine streams blocks→events.
5. **Replay-evidence emission:** the loader doubles as an **xlsx-file lane**: `ReplayAdapterCapabilityManifest` claiming C0–C2 (`oxreplay-conformance:10-35`), bundles in `replay.bundle.v1` with `comparison_value` in the aligned wire — file-parse claims diff against COM-observed truth, never vice versa (locked decision 5).

## 5. Versioning / evolution

- `WireSchemaId` string on every stream/ledger (`oxdoc.document_event.v1`), bundle_schema precedent; reader/ingest negotiate declared-supported sets; mismatch = typed error, no silent downgrade (fixes the spec-violating `Box::leak` tolerance pattern, `session.rs:7017-7022`).
- **Boundary conformance corpus**, owned by the file repo, vendored by OxCalc: .xlsx fixtures + golden JSON-serialized event transcripts + golden ledgers. File repo asserts parse→events; OxCalc asserts events→ingest→logical reads (via `SparseRangeReader`); round-trip asserts load→save fixpoint. Integration gate run via OxXlPlay: **every written corpus file must open in Excel without repair dialog** — an automated COM scenario (`BatchWorkbookKind::FileBacked`, `oxxlplay-scenario:53`).
- Event enum `#[non_exhaustive]` with explicit unknown-event policy on ingest (skip+ledger vs reject, per mode).

## 6. Seams for later lanes

- **C API:** event payloads are C-representable by construction — `PackedCellAddr(u64)` already is; `CellPayload` gets explicit discriminants, owned buffers, no `Rc`/lifetimes (the `Rc` ban already required by block storage, `designs-storage.md:21`). Do this now; retrofitting repr is a rewrite.
- **VBA:** xlsm/xlsb content types handled day one as PreservedOpaque (never strip macros on save); `OpaquePartNotice{kind: VbaProject}` is the future VBA lane's discovery hook.
- **RTD/volatile:** file repo emits cached values uniformly (it cannot know volatility); engine ingest classifies via OxFml `ExecutionProfileSummary` and the epoch vocabulary distinguishes `FileCached` from locally-computed — the RTD lane later adds `LiveFeed` epochs without touching the file contract.

## Three hardest problems + derisking

1. **Shared-formula ↔ template impedance.** Excel's si groups are writer-discretionary, sometimes non-rectangular, sometimes absent for identical formulas; engine coalescing will form *different* regions than the source file, so structural round-trip is unstable by nature. **Decide now:** round-trip stability is *semantic* (per-cell formula text identical after R1C1 normalization), not structural; region provenance (`FromSharedSi(u32) | HostCoalesced | EngineVerified`) recorded in ledger. Derisk: corpus of real Excel-authored files + load→save→load fixpoint metamorphic test before any optimization.
2. **Incremental save vs Excel's cross-part invariants.** Stale calcChain/metadata after partial rewrite triggers Excel repair. Derisk: the OxXlPlay opens-without-repair gate on every corpus write from day one; ship mode 3 last (it's the only deferred mode with corruption risk).
3. **Streaming memory bounds.** Strings/styles parts vs sheet parts arrive in arbitrary ZIP order; band buffering must keep peak memory ∝ band occupancy, not sheet size. Derisk: 1M-row stream benchmark with byte-budget assertion, reusing the closed-form scale-runner pattern (`treecalc_scale.rs:32`; TECHNICAL.md §7.6 doctrine) — paired simple (whole-DOM roxmltree reference reader) vs optimized (streaming) implementations diffed on event transcripts, per owner decision 5.

## Build-first vs defer

**First:** `oxdoc-model` crate (events, `CellPayload`, ledger types, wire id); full-load + values-only read paths; `grid_io` ingest builder; conformance corpus + golden transcripts; mode-4 export; OxXlPlay repair gate. **Second:** opaque-part preservation map; ledger completeness; replay-evidence lane manifest; `to_construction_spec()` triangle test. **Defer:** incremental save, xlsb, encrypted/protected workbooks, chartsheet/drawing modeling, VBA extraction, parallel part parsing.

## Open questions for the owner

1. **Dependency direction:** confirm OxCalc may depend directly on the types-only `oxdoc-model` crate (recommended, matches the no-shim doctrine), vs strict isolation where only hosts depend on both and re-project (extra copy of every chunk).
2. **Cached-value trust per profile:** is load-without-recalc a profile capability (strict-excel-grid treats `FileCached` as published truth until first dirty)? It must interact with prepared-identity/compatibility-basis machinery (`treecalc.rs:186-195`) — does a `FileCached` epoch participate in identity or sit outside it?
3. **Write-stability bar for touched parts:** semantic-only (recommended), or also stable si/xf index assignment (costs permanent index bookkeeping in the interners)?
4. **Vocabulary migration:** move `DefinedNameSpec`/`TableSpec`/`CfRuleSpec` leaf types from OxXlPlay into `oxdoc-model` with OxXlPlay re-exporting (one shared vocabulary), or keep duplicated with converters during transition?
5. **Corpus residency:** boundary corpus in the file repo with OxCalc vendoring (recommended), or a shared fixtures location? Affects CI wiring in both repos.