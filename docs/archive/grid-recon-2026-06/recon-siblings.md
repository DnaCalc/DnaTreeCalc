# Sibling-repo recon for OxCalc full-grid effort

## Top load-bearing findings

1. **The "simple-correct vs optimized, equivalence-traceable" pattern already has a working precedent in DnaVisiCalc.** It ran 5+ interchangeable core engines (rust-original, rust-fml, C, OCaml, .NET Native AOT) behind one C ABI, registered in `C:\Work\DnaCalc\DnaVisiCalc\coreengines\catalog.json`, with conformance docs (`docs/ENGINE_CONFORMANCE_TESTS.md`, `docs/ENGINE_API.md`, `docs/ENGINE_FORMAL_PROPERTIES.md`, `docs/SPEC_v0.md`). Foundation owns the spec/conformance artifact contract (`C:\Work\DnaCalc\Foundation\REFERENCE_SPEC_FORMAT_AND_CONFORMANCE.md`). Reuse this scaffolding shape, not the code.
2. **No repo in the family parses real .xlsx (ZIP/OOXML).** The only workbook-XML code is SpreadsheetML 2003 (flat XML) parsing in DnaOneCalc (`C:\Work\DnaCalc\DnaOneCalc\src\dnaonecalc-host\src\services\spreadsheet_xml.rs`, 512 lines, quick-xml 0.38; archived copy in `src_archive_ref`). OxXlPlay gets Excel truth by *driving live Excel via COM* (`C:\Work\DnaCalc\OxXlPlay\scripts\invoke-excel-observation.ps1`), not by parsing files. xlsx import for the grid effort is greenfield; no calamine/umya/zip deps anywhere (grep of all Cargo.tomls).
3. **The only existing grid data structure is deliberately tiny and dense, and the only in-family R1C1 implementation lives next to it.** `CellGrid<T>` = `Vec<Option<T>>` ("Replaces `HashMap<CellRef, T>` on hot paths") at `C:\Work\DnaCalc\DnaVisiCalc\crates\dnavisicalc-core\src\cell_grid.rs:7-8`, bounded to 63 cols x 254 rows (`address.rs:4-5`). R1C1 parse/resolve: `crates\dnavisicalc-core\src\eval.rs:2241` (`resolve_reference_text_r1c1`) and `:2323` (`parse_r1c1_ref`). Nothing in any sibling addresses sparsity, 1M x 16K bounds, or block occupancy. R1C1 *authority* going forward is OxFml (see DnaOneCalc pending seam below and `OxCalc\docs\worksets\W050_OXCALC_OXFML_FORMULA_AUTHORITY_REWORK.md`).

## Per repo

### DnaVisiCalc — Round 0 pathfinder (most relevant code)
- **Purpose/maturity:** complete, released (v0.2.0), now historical. TUI spreadsheet over a small Rust engine; crates: `dnavisicalc-core`, `-core-fml`, `-engine`, `-file`, `-tui`; plus multi-language reimplementations under `engines/`.
- **(a) Grid:** dense `Vec<Option<T>>` `CellGrid` + `CellBitset` (`cell_grid.rs`), bounds 63x254 — anti-model for sparse 1Mx16K but shows the dense-block idea works for hot paths.
- **(c) Rendering:** real viewport-driven cell render: `crates\dnavisicalc-tui\src\render.rs` (730 lines), `app.visible_grid(grid_width, grid_height)` at render.rs:126 — a small but genuine "visibility-driven" rendering seam precedent.
- **(d) R1C1:** working parse/resolve in `eval.rs` (above); README/spec document R1C1 display mode.
- **(e) Reusable/lessons:** multi-engine catalog + C API conformance harness (finding 1); perf retros in repo root: `OPTIMIZATION_REPORT.md`, `RUST_COREENGINE_OPTIMIZATION_SUGGESTIONS.md`. Also `docs/DYNAMIC_ARRAYS_DESIGN.md` (spill) and FEC/F3E redesign docs that fed OxFml.

### OxXlPlay — Excel observation harness
- **Purpose/maturity:** turns controlled Excel runs into replay-ready evidence bundles for OxReplay. Early-mid: ~2.3K lines of Rust across 7 crates (`src\oxxlplay-{abstractions,bridge,bundle,capture,cli,provenance,scenario}`) — mostly schema/validation types; bundle envelope declares `ComAutomation`/`DotNetInterop` invocation modes (`oxxlplay-bridge\src\lib.rs:9-14`); actual Excel driving is `scripts\invoke-excel-observation.ps1`. Bootstrap "complete through W006".
- **(b) xlsx:** references `.xlsx` fixtures and a `SpreadsheetMl2003Import` scenario kind (`oxxlplay-scenario\src\lib.rs:56-57`) but never parses xlsx itself — Excel opens it.
- **(e) Reusable:** 8 retained capture families under `states\excel\` (values/formulae, SpreadsheetML formatting, structured references, tables, VBA UDF, provenance) — ready-made oracles for grid-semantics differential tests. Lesson: explicit lossiness/provenance labeling on every captured surface.

### DnaOneCalc — single-formula proving host
- **Purpose/maturity:** substantial, active host ("Twin Oracle Workbench"); wasm/browser preview + desktop; `src\dnaonecalc-host` with `adapters/oxfml`, `services`, `ui`, `persistence`, `state`.
- **(b) xlsx:** SpreadsheetML 2003 only — `services\spreadsheet_xml.rs` (512 lines) feeding `verify-xml-cell` single-cell Excel-vs-OxFml verification (`services\verification_bundle.rs`).
- **(c) Rendering:** no grid — panel/editor UX for one formula (`ui\components\home_shell.rs`, `ui\editor`).
- **(d) R1C1:** explicitly *pending* upstream seam: `tests\seams\reference_style.rs:1` `SEAM-OXFML-R1C1-PUBLIC` — A1↔R1C1 round-trip is expected to come from OxFml's public surface, not host code. The grid effort should plan the same dependency.
- **(e) Reusable:** host-state slicing and UX formalization docs (`docs\APP_UX_HOST_STATE_SLICING.md`, `APP_UX_*`); ~40 `HANDOFF_OXFML_*` docs model the cross-repo handoff discipline. Hard rule: hosts read siblings, never write (README "Cross-repo rule").

### OxFunc — function-semantics lane
- **Purpose/maturity:** active, large (~34K lines in `crates\oxfunc_core` + `oxfunc_value_types`).
- **(d) References:** engine-agnostic reference abstractions — `ReferenceKind`, `ReferenceHandle`, `ReferenceIdentity`, `CompositeReferenceIdentity` (`crates\oxfunc_value_types\src\lib.rs:53-110`) — designed so the *host* owns reference resolution.
- **(e) Reusable:** README "Optimization Direction" (W096) is directly on-point for repeated R1C1-identical formula regions: resolve function identity once via `SurfaceCallSite`, catalog-keyed dispatch, hoistability/optimizer metadata as contract data, reusable runtime-context/scratch-buffer seams. The OxCalc grid's shared-formula-region design should consume these handles rather than re-dispatch per cell.

### OxReplay — shared replay appliance
- **Purpose/maturity:** early-mid (~6K lines across 9 crates: `oxreplay-{abstractions,bundle,conformance,core,diff,distill,explain,governance,dnarecalc-cli}` under `src\`). Owns canonical bundle parsing/diff/explain/witness lifecycle; explicitly not a semantics lane.
- **(e) Reusable:** the natural evidence channel for proving simple-vs-optimized grid implementation equivalence (replay same intent log into both, diff). No grid/xlsx/rendering content.

### OxVba — VBA language/runtime
- **Purpose/maturity:** the most mature sibling: 13 crates (`oxvba-{syntax,bind,symbol,vm2,jit,com,hal,host,runtime,project,bundle,cli,lib}`), conformance matrices, JIT+VM, COM interop.
- **(a-d):** nothing grid/xlsx/rendering; quick-xml is for project files (`crates\oxvba-project`). Future Range-object host integration is the only grid touchpoint.
- **(e) Lesson:** validation-matrix-driven development (`docs\validation\*_VALIDATION_MATRIX_V1.csv`) as a maturity model for a large spec-driven Rust workspace.

### Foundation — doctrine/conformance owner
- **Purpose/maturity:** docs-only, authoritative for process and conformance format.
- **(e) Reusable for the semi-formal spec goal:** `CORE_ENGINE_FORMAL_MODEL.md` (524-line mirror; canonical editable copy is `OxCalc\docs\spec\core-engine\CORE_ENGINE_FORMAL_MODEL.md`) and `CORE_ENGINE_THEORY_AND_ALTERNATIVE_PATHS.md`; `REFERENCE_SPEC_FORMAT_AND_CONFORMANCE.md` defines the normalized spec/conformance artifact contract; Excel reference corpus at `reference\conformance\excel-worksheet-engine\` (`EXCEL_CONFORMANCE_SPEC.md`, `CONFORMANCE_REQUIREMENTS.csv`, `KNOWN_GAPS_AND_UNCERTAINTIES.md`). README confirms host ladder: TreeCalc → **PreCalc → SuperCalc → Calc** — the full grid presumably lands in those later hosts, so the grid spec should be written as a lane-owned OxCalc spec (per "canonical lane-owned spec locations").

### DnaOxIde — design mockups only
- `docs\DesignMockup` + `docs\DesignPrototype`: Figma-exported React/Vite UI mockups for the DNA OxIde IDE (editing/build-run/command-palette states). No engine, grid, or xlsx code; at most visual-design inspiration for a rendering pass.

## Gaps confirmed (nothing exists for)
- Sparse block/chunked sheet storage, 1,048,576 x 16,384 addressing, virtual cell nodes, viewport-priority *recalc* (only viewport *render* in DnaVisiCalc TUI), streaming-to-renderer protocol, and real xlsx (OOXML ZIP) import — all greenfield for OxCalc.