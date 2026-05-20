# DNA TreeCalc — Excel Export and Replay Verification

This document specifies how a DNA TreeCalc workspace is converted to an Excel-equivalent form and verified against Excel as canonical truth, via the OxXlPlay (Excel interop) and OxReplay (comparison/governance) sibling repos.

The driving principle (per the user, 2026-05-20): **put the right responsibility in the right repo. No seams, shims, or conversions scattered across repos. Direct API calls to the right surfaces in the right repos.** DNA TreeCalc's role is deliberately limited — convert its model to an OxXlPlay-friendly form and emit its own value bundle. OxXlPlay owns Excel construction and observation. OxReplay owns comparison, record-keeping, and verdict governance.

This document also records what must be **added or improved in the sibling repos** to make this clean, so the work lands where it belongs.

Cross-references: the import direction is in [`CORE_MODEL_SPEC.md`](../model/CORE_MODEL_SPEC.md) §10. The export-strategy discussion (defined names + grid-cell promotion) is folded in here.

---

## 1. Current state of the sibling repos

Grounded in repo survey 2026-05-20.

### 1.1 OxXlPlay (`C:\Work\DnaCalc\OxXlPlay`)

**What it is today:** an Excel *observation* harness. It captures observable Excel behavior through controlled runs and emits replay-ready evidence bundles. It is explicitly NOT a workbook automation/build library today.

- 7-crate Rust workspace: `oxxlplay-abstractions`, `-scenario`, `-capture`, `-provenance`, `-bridge`, `-bundle`, `-cli`.
- Excel automation is via a **PowerShell COM bridge** (`scripts\invoke-excel-observation.ps1`), invoked as a subprocess. The Rust side validates scenarios and assembles evidence; PowerShell drives Excel COM.
- Scenario kinds: `FileBacked` (open an existing `.xlsx`, recalc, observe), `ProgrammaticFormula` (write one formula into `Sheet1!A1`, recalc, observe), `ProgrammaticVbaUdf`.
- Observable surface kinds include `CellValue`, `FormulaText`, `EffectiveDisplayText`, `NumberFormatCode`, `FontColor`, `FillColor`, `ConditionalFormattingRules`, `DefinedNameValue`, `ErrorValue`.
- CLI: `dna-xl-play capture-run --scenario <path>` / `capture-run-batch --manifest <path>`.

**Gaps for TreeCalc:**
- ✗ No multi-cell / multi-sheet programmatic workbook construction.
- ✗ No defined-name *write* API (can observe `DefinedNameValue`, can't create names).
- ✗ No Excel Table creation.
- ✗ No batch-cell-write API.
- Read-back of `DefinedNameValue` exists as a surface kind but is described as untested via the live driver.

### 1.2 OxReplay (`C:\Work\DnaCalc\OxReplay`)

**What it is today:** the shared comparison/governance substrate. A library plus the `DNA ReCalc` reference CLI. It compares replay scenarios at the typed comparison-view family level, explains divergences, distills witnesses, and governs the witness lifecycle. It does NOT drive Excel and does NOT own final host verdict policy.

- Crates: `oxreplay-abstractions`, `-bundle`, `-core`, `-diff`, `-explain`, `-distill`, `-governance`, `-conformance`, `-dnarecalc-cli`.
- Comparison-view families: `comparison_value`, `effective_display_text`, `visible_value_text`, `execution_outcome`, `formatting_view`, `conditional_formatting_view`.
- Mismatch kinds: `ComparisonValue`, `EffectiveDisplayText`, `OutcomeValue`, `FormattingView`, `ConditionalFormattingView`, `ProjectionCoverageGap`, etc.
- Numeric comparison is exact via JSON; near-match labels (`near_equal_last_bit`, `near_zero_residue`) available; tolerance policy is lane-declared, not built-in.
- A host emits replay bundles (manifest + scenarios with comparison views); calls `oxreplay_diff::diff_summary()`, `oxreplay_explain::explain_diff()`, etc.; retains witness records via `oxreplay_governance`.
- DNA ReCalc CLI: `validate-bundle`, `replay`, `diff`, `explain`, `distill`, `validate-adapter`, `witness-state`, `pack-export`.

**Key boundary:** OxReplay owns the comparison mechanics and witness lifecycle; the host owns final verdict policy and product UX. It compares *given* bundles — it does not orchestrate Excel runs.

### 1.3 DnaOneCalc verification (the pattern to follow)

DnaOneCalc's verification is **architecturally complete, semantically incomplete**:

- Orchestration lives in OneCalc (`services/verification_bundle.rs`). It runs the batch, invokes OxXlPlay CLI (`capture-run-batch`) and OxReplay CLI (`validate-bundle`, `diff`, `explain`) as subprocesses (hard-coded sibling-repo paths).
- Verification config XML declares host-profile + capabilities (`requires-excel-observation`, `excel-observation-available`).
- CLI: `verify-formula`, `verify-xml-cell`, `verify-batch`, `verify-vba-udf`, `audit-formula-drill`.
- Retained artifacts under `target/onecalc-verification/bundle-<timestamp>/` — the same artifact families the interactive Workbench/Inspect UI consumes (no parallel format).
- **Three handover docs** request upstream capabilities: OxFml richer publication, OxXlPlay richer observation (effective display text, formatting), OxReplay finer comparison-view families. A real April-2026 run showed OxFml="6" vs Excel="$6.00" being classified as a coarse mismatch instead of "value matches, display differs."

**The pattern for TreeCalc to follow:** keep orchestration thin in the host; push evaluation + replay surfaces to OxFml; push Excel construction + observation to OxXlPlay; push display-aware comparison to OxReplay. Do not invent local formatting interpretation or Excel-construction logic in TreeCalc.

---

## 2. The clean partitioning

```
+---------------------------------------------------------------+
|  DNA TreeCalc  (limited role)                                 |
|  - Convert workspace + bind results → WorkbookConstructionSpec|
|  - Emit own replay bundle (per-node computed values as        |
|    comparison_value / effective_display_text views)           |
|  - Thin verify-workspace command that ties the pieces together|
|  - Owns: tree→Excel mapping decisions (bake, mangle, promote) |
+---------------------------------------------------------------+
                 |                                  |
                 | WorkbookConstructionSpec         | TreeCalc replay bundle
                 v                                  |
+-------------------------------------------+       |
|  OxXlPlay  (Excel interop)                |       |
|  - NEW: build a workbook from a Spec      |       |
|    (sheets, defined names, cells, tables) |       |
|  - Drive Excel COM, recalc                |       |
|  - Observe per-name / per-cell values     |       |
|  - Emit Excel-side replay bundle          |       |
|  - Owns: all Excel COM + workbook build   |       |
+-------------------------------------------+       |
                 |                                  |
                 | Excel-side replay bundle         |
                 v                                  v
+---------------------------------------------------------------+
|  OxReplay  (comparison + governance)                          |
|  - Compare TreeCalc bundle vs Excel bundle (view families)    |
|  - Explain divergences                                        |
|  - Stamp witness records, lifecycle, provenance               |
|  - Produce typed diff verdict                                 |
|  - Owns: comparison mechanics, witness governance             |
+---------------------------------------------------------------+
                 |
                 v
            verdict + retained artifacts
```

**One-line ownership statements:**

- **DNA TreeCalc** owns the *semantics of mapping a tree to Excel* — what becomes a defined name, what gets grid-promoted, how relative refs bake to absolute, how names mangle. Nothing about Excel COM or comparison.
- **OxXlPlay** owns *everything about talking to Excel* — building a workbook from a declarative spec, driving recalc, observing results. The construction spec is its input contract and should be general (usable by any DnaCalc host, not TreeCalc-specific).
- **OxReplay** owns *comparison and evidence* — diffing two bundles at the view-family level, explaining, governing the witness lifecycle, stamping records.

No repo reaches into another's domain. TreeCalc does not build `.xlsx`. OxXlPlay does not know what a TreeCalc tree is. OxReplay does not drive Excel.

---

## 3. The Workbook Construction Spec (new contract, owned by OxXlPlay)

This is the central new artifact. OxXlPlay must gain the ability to **build a workbook from a declarative spec**. The spec type lives in OxXlPlay (`oxxlplay-scenario` or a new `oxxlplay-construct` crate) as its input contract. TreeCalc depends on this type and populates it.

The spec is **general** — it describes an Excel workbook in Excel terms, with no TreeCalc concepts. Any DnaCalc host that needs to construct a workbook uses the same spec.

### 3.1 Spec shape (proposed, lives in OxXlPlay)

```rust
pub struct WorkbookConstructionSpec {
    pub sheets: Vec<SheetSpec>,
    pub defined_names: Vec<DefinedNameSpec>,
    pub tables: Vec<TableSpec>,
    pub external_links: Vec<ExternalLinkSpec>,
    pub date_system: DateSystem,                // 1900 | 1904
    pub calc_mode: CalcMode,                    // Automatic | Manual
}

pub struct SheetSpec {
    pub name: String,
    pub cells: Vec<CellSpec>,
}

pub struct CellSpec {
    pub address: CellAddress,                   // e.g., A1, B2
    pub content: CellContent,                   // Formula(String) | Literal(LiteralValue) | Empty
    pub number_format: Option<String>,          // format code
    pub style: Option<StyleSpec>,               // font, fill, etc.
    pub conditional_format: Vec<CfRuleSpec>,    // ordered CF rules
}

pub struct DefinedNameSpec {
    pub name: String,
    pub scope: NameScope,                       // Workbook | Sheet(SheetName)
    pub refers_to: NameTarget,                  // Formula(String) | Range(CellRange) | TableColumn(...)
    pub hidden: bool,
}

pub struct TableSpec {
    pub name: String,
    pub sheet: String,
    pub range: CellRange,                       // including header row
    pub columns: Vec<TableColumnSpec>,          // column names + optional column formula
    pub has_totals_row: bool,
}

pub struct CfRuleSpec {
    pub condition: CfCondition,                 // CellValue(op, operand) | Formula(String) | ...
    pub action: CfAction,                       // font color, fill, data bar, icon set
    pub stop_if_true: bool,
}

pub struct ExternalLinkSpec {
    pub alias_filename: String,                 // e.g., "reports.xlsx"
    pub real_path: PathBuf,                     // resolved path for COM to find it
}
```

### 3.2 Read-back contract

After OxXlPlay builds the workbook and Excel recalculates, it observes results. The read-back targets are declared in the construction request (or a paired observation scenario):

```rust
pub struct ConstructAndObserveRequest {
    pub workbook: WorkbookConstructionSpec,
    pub observe: Vec<ObservationTarget>,        // which defined names / cells to read back
    pub trigger: RecalcTrigger,                 // open_then_recalc, full_recalc, etc.
}

pub enum ObservationTarget {
    DefinedName { name: String, scope: NameScope },
    Cell { sheet: String, address: CellAddress },
    TableColumn { table: String, column: String },
}
```

The output is an OxXlPlay replay bundle (the existing bundle format) carrying the observed `comparison_value`, `effective_display_text`, `formatting_view`, etc. per observed target.

### 3.3 Why the spec lives in OxXlPlay

- OxXlPlay owns Excel COM. Building a workbook is an Excel-COM operation. The build logic — translating a `CellSpec` into a `Range.Formula` write, a `DefinedNameSpec` into a `Names.Add`, a `TableSpec` into a `ListObjects.Add` — is COM code. It belongs in OxXlPlay's bridge.
- The spec is general (Excel terms only), so the DnaCalc grid host and any future host can reuse it. Putting it in OxXlPlay keeps it host-neutral.
- TreeCalc depends on OxXlPlay's spec type and populates it — a direct API dependency, no shim.

---

## 4. DNA TreeCalc's converter: workspace → WorkbookConstructionSpec

This is TreeCalc's core contribution. It lives in TreeCalc (`services/excel_export.rs`). It reads the workspace + the bind results (from the OxCalc bridge) and produces a `WorkbookConstructionSpec`.

### 4.1 Export strategies

**Strategy 1 — defined-names primary.** Each node with non-empty content (a `=`-formula or a literal constant) becomes a `DefinedNameSpec` whose `refers_to` is the baked formula. No cells used. This is the default.

**Strategy 2 — grid-cell promotion** for nodes that need it. A promoted node's value lands in cells (`CellSpec`s on a sheet); a `DefinedNameSpec` aliases the cell range. Triggers:

1. **Table-typed nodes** → `TableSpec` (grid range + named Excel Table).
2. **Arrays with per-cell formatting / CF** → cells (formats attach to cells, not names).
3. **Arrays exceeding inline-literal practicality** (~100–500 cells; Excel formula length limit) → cells.
4. **Explicit user request** (a host hint) → cells.

Most of the workspace stays defined-names. Grid-promotion is the exception.

### 4.2 The bake-relatives-to-absolute pass

Every TreeCalc relative reference resolves to an absolute name at export time, using the bind result:

| TreeCalc source (in `.Accounts.2026.Q2.Income.Sales`) | Baked Excel reference |
|---|---|
| `Margin` (walk-up) | `Accounts.2026.Q2.Income.Margin` |
| `^.Margin` | `Accounts.2026.Q2.Income.Margin` |
| `^^.Total` | `Accounts.2026.Q2.Total` |
| `@PREV.Net` (from Q2) | `Accounts.2026.Q1.Net` |
| `@PREV.Net` (from Q1) | `#REF!` / `NA()` (no previous sibling) |
| `[].Foo` | `Foo` (workbook-scope) |
| `[ws]Branch.Item` | `[ws.xlsx]Branch.Item` (external link) |
| `Q1.*` in `SUM(...)` | static-expand: `SUM(Q1.Income, Q1.Expenses, Q1.Net, Q1.Variance)` |
| `Foo.**.Bar` | static-expand over all current matches (formula-length permitting) |

The same source text in different positions becomes different baked formulas. Templates expand to instances; each instance bakes its relatives differently.

### 4.3 Name-mangling pass

Excel defined names disallow spaces, hyphens, leading digits, `[`, `]`, `#`, `@`, `'`, `\`, and others. TreeCalc bracket-escaped names allow all of these. The converter:

1. Detects names with disallowed characters.
2. Applies a deterministic transform (spaces → `_`, leading digit → `_NNNN` prefix, other disallowed → `_`).
3. Records the mapping (`TreeCalc name → Excel name`) in the export manifest.
4. Suffixes on collision.
5. Emits a warning per transformation.

### 4.4 Table layout

For table-typed nodes:
- Allocate a sheet (or a region) for the table.
- Header row from column names; data rows from the table's rows.
- Column formulas → Excel structured-ref column formulas (`=[@Col1] * [@Col2]`); the structured-ref syntax is already Excel-compatible (TreeCalc adopted it verbatim).
- References from the column formula to outside the table bake to absolute per §4.2.
- Emit a `TableSpec`. The table's name is the (mangled) TreeCalc path; references elsewhere use `TablePath[ColName]`.

### 4.5 The export manifest

A sidecar produced by TreeCalc (NOT by OxXlPlay or OxReplay), recording per node:

```rust
pub struct ExportManifest {
    pub entries: Vec<ExportEntry>,
    pub name_mangling: Vec<(TreeCalcName, ExcelName)>,
    pub set_expansions: Vec<SetExpansionRecord>,    // where .* / ** were statically expanded
    pub unmapped: Vec<UnmappedNode>,                // nodes that couldn't export (with reason)
}

pub struct ExportEntry {
    pub node_id: TreeNodeId,
    pub tree_path: String,
    pub excel_target: ExcelTarget,                  // DefinedName(name) | Cell(sheet, addr) | Table(name)
    pub treecalc_computed_value: EvalValue,         // what TreeCalc computed (the comparison left side)
}
```

The manifest is the bridge for: (a) re-import round-trip, (b) replay-divergence triage (which Excel target maps to which node), (c) re-export stability checking.

---

## 5. DNA TreeCalc's replay bundle emission

For OxReplay to compare, it needs two bundles: TreeCalc's and Excel's (via OxXlPlay). TreeCalc emits **its own** bundle using OxReplay's bundle schema, populated with per-node computed values as comparison views.

```rust
// TreeCalc populates an OxReplay ReplayBundleManifest:
ReplayBundleManifest {
    lane_id: LaneId("dna_treecalc"),
    adapter_id: AdapterId("dna_treecalc_v1"),
    bundle_schema: "replay.bundle.v1",
    source_schema: "dna_treecalc.workspace.v1",
    capture_mode: "model_projection",
    registry_refs: [ /* pinned comparison-family versions */ ],
    views: [ /* per-node comparison views */ ],
    // ...
}
```

For each node, TreeCalc emits comparison views keyed by the node's Excel target (the manifest's `excel_target`):

- `comparison_value` — TreeCalc's computed `EvalValue` as the canonical comparison family.
- `effective_display_text` — TreeCalc's formatted display string (locale-aware), for display-faithful comparison.
- `execution_outcome` — for nodes that error, the typed outcome (`#REF!`, `#VALUE!`, etc.).
- Optionally `formatting_view` / `conditional_formatting_view` if TreeCalc preserves the format metadata (these come from OxFml's format machinery, which TreeCalc reuses).

The view key must match the OxXlPlay-side bundle's locator so OxReplay can pair them. The shared key is the Excel target (defined-name or sheet!cell) recorded in the export manifest.

---

## 6. What OxXlPlay must add

The substantial sibling-repo work. These are capabilities OxXlPlay does not have today.

| Capability | Detail | Crate |
|---|---|---|
| **Workbook construction from `WorkbookConstructionSpec`** | Build a multi-sheet, multi-cell workbook with defined names and tables via Excel COM. The PowerShell bridge gains a `construct-workbook` operation that consumes the spec JSON and issues `Range.Formula`, `Names.Add`, `ListObjects.Add` COM calls. | new `oxxlplay-construct` + bridge script extension |
| **Defined-name write** | `Names.Add` for workbook- and sheet-scoped names; names with dots. | bridge |
| **Excel Table creation** | `ListObjects.Add` over a range; set column names; column formulas. | bridge |
| **Multi-cell batch write** | Write many cells / formulas in one workbook-construction pass. | bridge |
| **Defined-name value read-back** | Observe `DefinedNameValue` for a list of names (the surface kind exists; the live driver path needs completing). | `oxxlplay-capture` + bridge |
| **Table column read-back** | Observe a Table column's computed values. | `oxxlplay-capture` |
| **Construct-and-observe in one run** | Build the workbook, recalc, observe, emit bundle — single CLI invocation `dna-xl-play construct-and-observe --request <path>`. | `oxxlplay-cli` |
| **External-link workbook handling** | Construct workbooks that reference other workbooks (`[other.xlsx]Name`), resolving paths so Excel can find them. | bridge |
| **UDF provisioning into Excel** | Load VBA modules/projects AND `.xll` native add-ins into the Excel instance before recalc, so UDF-using workspaces verify faithfully. VBA path partially exists (`ProgrammaticVbaUdf`); `.xll` path is new. | bridge |

A handover doc to OxXlPlay should request these.

The `WorkbookConstructionSpec` type itself lives in OxXlPlay (it's the Excel-construction input contract). TreeCalc imports it as a direct dependency.

---

## 7. What OxReplay must add (or confirm)

OxReplay is largely ready. Items to confirm or add:

| Item | Status |
|---|---|
| Accept a `dna_treecalc` lane id + `dna_treecalc_v1` adapter id | New lane registration; adapter manifest. |
| Pair views by Excel-target locator | Confirm the comparison can key views by an arbitrary locator string (defined-name or `Sheet!Cell`). |
| Numeric tolerance policy for TreeCalc lane | TreeCalc declares its tolerance (exact, or last-bit) via adapter manifest. |
| `effective_display_text` family populated on both sides | Same gap OneCalc flagged; OxReplay supports the family, both sides must emit it. |
| CF / formatting view families | Same upstream gap OneCalc identified; needed only when comparing formatting, not values. |

No new OxReplay mechanics needed for value comparison — it's a matter of TreeCalc emitting a valid bundle and registering the lane. The display-faithful comparison shares the same upstream gap OneCalc already documented (so both products benefit from one fix).

---

## 8. Orchestration: who drives

Two viable models:

**Model A — host-orchestrated (follows DnaOneCalc).** TreeCalc has a thin `verify-workspace` command that:
1. Calls its own converter → `WorkbookConstructionSpec` + manifest + TreeCalc replay bundle.
2. Invokes OxXlPlay (`construct-and-observe`) as a subprocess → Excel-side bundle.
3. Invokes OxReplay (`diff`, `explain`) as a subprocess → diff report.
4. Applies verdict, retains artifacts.

**Model B — OxReplay/DNA-ReCalc-driven.** DNA ReCalc gains a "two-sided Excel verification" mode that, given a TreeCalc bundle + a construction spec, calls OxXlPlay and compares. This puts orchestration in OxReplay's CLI.

**Recommendation: Model A.** It matches the established DnaOneCalc pattern, keeps TreeCalc-specific knowledge (the converter) in TreeCalc, keeps OxReplay a comparison library that doesn't know about TreeCalc, and keeps OxXlPlay a workbook builder that doesn't know about TreeCalc. The orchestration is thin glue in TreeCalc that calls two general-purpose tools. This respects "right stuff in the right repo" better than Model B, which would teach OxReplay's CLI about TreeCalc workspaces.

The user's intuition that "OxReplay coordinates" is honored in the sense that **OxReplay owns the comparison authority, the verdict mechanics, and the record-keeping** — it just isn't the process that launches Excel. The thin TreeCalc command sequences the calls; OxReplay decides equivalence and stamps the witness.

---

## 9. End-to-end verification flow

```
DNA TreeCalc                    OxXlPlay                    OxReplay
-----------                     --------                    --------
verify-workspace foo.dnatree
   |
   |-- load workspace + bind via OxCalc bridge
   |-- converter: bake, mangle, grid-promote
   |       |
   |       +--> WorkbookConstructionSpec (JSON)
   |       +--> ExportManifest (JSON)
   |       +--> TreeCalc replay bundle (per-node comparison_value views)
   |
   |-- invoke OxXlPlay construct-and-observe --request spec.json
   |                              |
   |                              |-- PowerShell COM bridge:
   |                              |     build workbook (cells, names, tables)
   |                              |     recalc
   |                              |     observe declared targets
   |                              |
   |                              +--> Excel-side replay bundle (observed values)
   |
   |<-----------------------------+
   |
   |-- invoke OxReplay diff --left treecalc.bundle --right excel.bundle --kind dna_treecalc
   |                                                          |
   |                                                          |-- diff_summary() per view family
   |                                                          |-- explain_diff() on mismatches
   |                                                          |-- witness lifecycle record
   |                                                          |
   |<---------------------------------------------------------+ ReplayDiffReport
   |
   |-- apply verdict policy (value-equiv within declared tolerance)
   |-- retain artifacts (manifest, both bundles, diff, explain)
   |-- exit code: 0 matched / 1 mismatched / 4 blocked
```

Per-node comparison: OxReplay pairs the TreeCalc bundle's `comparison_value` for node N (keyed by N's Excel target from the manifest) against the Excel-side bundle's observed value for the same target. Divergences become `ComparisonValue` (or `EffectiveDisplayText`) mismatches; the manifest maps the Excel target back to the TreeCalc node path for triage.

---

## 10. Verification commands (DNA TreeCalc)

Following DnaOneCalc's CLI shape:

```
verify-workspace --workspace <path> [--config-xml <path>] [--output-root <path>]
    Full workspace: convert, construct via OxXlPlay, observe, diff via OxReplay, verdict.

verify-node --workspace <path> --node-path <path> [...]
    Single-node verification — convert just the node's dependency closure, compare.

export-excel --workspace <path> --out <path.xlsx> [--strategy defined-names|mixed]
    Produce the .xlsx (via OxXlPlay construction) without the comparison step.
    For sharing / inspection / manual verification.

verify-roundtrip --workspace <path>
    Export to Excel, re-import, compare structure + values to the original.
    Tests export+import fidelity (not value-vs-Excel).
```

Verification config XML mirrors OneCalc's (host-profile, capabilities, excel-observation-available), so the same harness conventions apply.

---

## 11. Repo-by-repo work summary

| Repo | What it owns here | What must be added |
|---|---|---|
| **DNA TreeCalc** | model → spec converter; bake/mangle/promote passes; export manifest; own replay bundle emission; thin verify-workspace orchestration | `services/excel_export.rs`, `verify-workspace`/`export-excel`/`verify-roundtrip` commands, replay-bundle emitter |
| **OxXlPlay** | Excel COM: build workbook from spec, recalc, observe | `WorkbookConstructionSpec` type; `construct-workbook` + `construct-and-observe` bridge ops; defined-name write; table creation; multi-cell write; defined-name + table read-back; external-link handling |
| **OxReplay** | comparison, explain, witness governance, verdict mechanics | register `dna_treecalc` lane + adapter manifest; confirm locator-keyed view pairing; (shared with OneCalc) display-faithful view families |
| **OxFml** | per-node eval + format publication (reused) | (shared with OneCalc) richer publication surfaces for display/format comparison |
| **OxCalc** | multi-node recalc, bind results TreeCalc bakes from | the engine prerequisites in [`CORE_MODEL_SPEC.md`](../model/CORE_MODEL_SPEC.md) §6 |

---

## 12. Handover documents to author

TreeCalc should author:

1. **`HANDOVER_OXXLPLAY_workbook_construction.md`** — the big one. Requests the `WorkbookConstructionSpec` type and the construct-and-observe capability (multi-cell, defined names, tables, read-back). This is net-new capability for OxXlPlay.
2. **`HANDOVER_OXREPLAY_treecalc_lane.md`** — requests `dna_treecalc` lane registration, adapter manifest, locator-keyed view pairing. Mostly registration + confirmation.
3. **Shared with OneCalc:** the existing display-faithful-comparison requests benefit both products; TreeCalc co-signs rather than duplicates.

---

## 12b. UDF provisioning for verification (VBA and .xll)

When a workspace uses user-defined functions, faithful Excel verification requires the **same UDFs to be available to Excel** during the OxXlPlay observation run. Otherwise Excel returns `#NAME?` for the UDF calls while TreeCalc computed real values, producing spurious divergences.

TreeCalc supports two UDF mechanisms (see UX technical plan §1.1), both via a shared UDF-hosting core extracted from DnaOneCalc-first work:

- **VBA UDFs** — provisioned into Excel by injecting the VBA module/project before recalc. OxXlPlay already has a `ProgrammaticVbaUdf` scenario path for this in the single-formula case; the workbook-construction request must carry the VBA source so it loads alongside the constructed workbook.
- **.xll native add-ins** — native code add-ins using the Excel C API. Excel must load the `.xll` before recalc. OxXlPlay needs an analogous provisioning capability: register the `.xll` add-in during the construct-and-observe run.

Repo placement:
- **The shared UDF-hosting core** (OneCalc-first, then extracted) owns compiling/loading the UDF definitions for the *DnaCalc-side* evaluation. TreeCalc consumes it; doesn't reimplement.
- **OxXlPlay** owns provisioning the *Excel-side* UDFs (loading VBA modules and `.xll` add-ins into the Excel instance during the run). The VBA path partially exists; the `.xll` path is new. Both belong in OxXlPlay's bridge, not in TreeCalc.
- The `WorkbookConstructionSpec` (or its paired observe request) gains fields for UDF provisioning: VBA module/project sources and `.xll` add-in paths to load before recalc.

This is a handover item to OxXlPlay alongside the workbook-construction capability (§6, §12).

## 13. Known gaps and limitations

- **Literal-dot-in-name round-trip** — `[My.Region]` (literal dot) exports to Excel `My.Region`, re-imports as a path. Mitigated by the export manifest sidecar recording original structure; or by disallowing literal dots in TreeCalc names.
- **Meta-nodes** — don't survive to Excel natively. Templates expand to instances; format meta bakes into cells (mixed strategy) or is lost (defined-names-only). Optional: stash the meta-tree in a custom XML part of the `.xlsx` for round-trip.
- **Set-producing operators in non-function contexts** — `.*` in a function argument expands cleanly; in a MAP expression the translation may be lossy.
- **`[]` standalone (root reference)** — no Excel analog; skip with warning.
- **INDIRECT with dynamic strings** — fragile in both directions.
- **Display-faithful comparison** — blocked on the same upstream gap OneCalc flagged (OxFml publication + OxXlPlay observation + OxReplay view families). Value comparison works now; format comparison waits on the shared fix.
- **Cross-workspace bundling** — each referenced workspace needs its own export; path coordination required.

---

## 14. Status

The partitioning is settled: TreeCalc converts and emits; OxXlPlay builds and observes; OxReplay compares and governs. The single biggest piece of new work is **OxXlPlay gaining workbook-construction capability** — it is currently an observation harness, not a builder, and TreeCalc's verification depends on the builder. This is correctly placed in OxXlPlay (it's Excel-COM work) rather than shimmed into TreeCalc.

Value-equivalence verification is achievable once OxXlPlay can construct workbooks; display-faithful verification shares OneCalc's pending upstream gap. The verification command shape and artifact conventions follow DnaOneCalc directly, so the two products share harness patterns and retained-artifact families.
