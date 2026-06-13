# OxFml — grid-support map

## 1. Formula AST/IR pipeline

Single crate `oxfml_core` (`C:/Work/DnaCalc/OxFml/crates/oxfml_core/src/`). Pipeline: **parse → red projection → bind → semantic plan → compiled eval plan → evaluate**, all per single formula.

- **Parse**: lossless green tree (`syntax/green.rs`, `syntax/parser.rs:1031` lines), keyed by `green_tree_key`; incremental variants `parse_formula_incremental` (`lib.rs:133-138`). Parser is **channel-agnostic** (A1 vs R1C1 decided at bind, per `docs/spec/formula-language/OXFML_R1C1_FORMULA_CHANNEL.md`).
- **Bind**: `bind_formula(BindRequest)` → `BoundFormula` (`binding/mod.rs:301-375`). `BoundExpr` (`binding/mod.rs:158-188`): literals, Binary/Unary, FunctionCall (by **name string**), Invocation, `Reference(ReferenceExpr)`, `HostReference(HostNameBindRecord)`, host structural selectors/collections, `ImplicitIntersection`.
- **What a bound formula references**: NOT node ids. `NormalizedReference` (`binding/reference.rs:175-184`): `Cell/Area/WholeRow/WholeColumn` carry **string `workbook_id` + `sheet_id` + absolute u32 coords**; `Name`, `External`, `Structured(table_id strings)`, `Error`. `BoundFormula` (`binding/mod.rs:225-241`) carries `normalized_references`, `dependency_seeds` (summary strings), `bind_hash`, `bind_context_fingerprint`. Host/tree node identity enters only via `HostNameBindRecord` resolved through the `HostNameResolver` trait (`binding/mod.rs:101-117`) — this is how OxCalc maps names→TreeNodeId (`OxCalc/src/oxcalc-core/src/consumer.rs:36-39`, `treecalc_node_host_dependency_key`).
- **Semantic plan**: `compile_semantic_plan` (`semantics/mod.rs:248`) → `SemanticPlan` keyed `semantic_plan_key` over `bind_hash` + catalog identity; per-function `FunctionPlanBinding` with volatility/thread-safety/determinism and `ExecutionProfileSummary` (`semantics/mod.rs:176-198`) — ready-made scheduler facts (serial lane, single-flight, volatile, caller-context).
- **Eval**: private `CompiledFormulaPlan` built per `EvaluationContext` by `compile_formula_for_evaluation` (`eval/mod.rs:289, 1087, 1701`); `CompiledExpr` with special forms LET/LAMBDA/IF/IFERROR/`_XLFN.SINGLE` (`eval/mod.rs:295-451`).

## 2. Reference semantics / R1C1 canonicalization

- `AddressMode {row_absolute, col_absolute}` + `caller_anchor_used` exist on every Cell/Area ref (`binding/reference.rs:11-35`).
- **R1C1 channel** (`FormulaChannelKind::WorksheetR1C1`, `source.rs:17-22`): `R[d]C[d]` relative parts are **resolved against caller anchor at bind time** into absolute coords (`binding/mod.rs:2979-3047`); `address_mode` records relative origin, `caller_anchor_used` records anchor dependence. So R1C1 is supported as entry channel, but the bound artifact is already per-cell-absolute.
- **A1 channel gap**: `parse_cell_reference` **strips `$` and discards it** — `AddressMode::default()` (both false) and `caller_anchor_used: true` unconditionally (`binding/mod.rs:2926-2956`). A1 `$A$1` vs `A1` bind identically today; A1 relative refs are NOT caller-offset (treated as absolute coords). No A1↔R1C1 round-trip fidelity yet.
- **Sharing one bound artifact per R1C1-identical region**: today impossible without change — `bind_context_fingerprint` includes `caller_row`/`caller_col` (`binding/mod.rs:302-315`), so `bind_formula_incremental` reuse (`:377-413`) fails across cells, and `bind_hash` differs because coords are pre-resolved. **Difficulty: moderate, well-contained.** The green tree is already caller-independent and shareable (`green_tree_key`). Canonicalization needs (a) a symbolic bound ref form keeping offsets instead of resolved coords (the `AddressMode`/`caller_anchor_used` provenance already exists in the R1C1 lane), (b) dropping caller anchor from the fingerprint for anchor-relative formulas, (c) resolving offsets at dereference time — eval already has `caller_row/caller_col` (`eval/mod.rs:1680-1681`) and routes all cell reads through `ReferenceSystemProvider::dereference/enumerate_values` with string targets (`eval/mod.rs:1823-1899`). The change is localized to `binding/mod.rs` ref construction + `CompiledReferenceExpr::Atom` resolution.

## 3. Plans for OxCalc; template/closure feasibility

- OxFml owns evaluation; OxCalc consumes via `consumer/runtime/mod.rs`: `RuntimeEnvironment` (caller position, defined names, `with_cell_values(BTreeMap<String, CalcValue>)` `:421`, table context) + `RuntimeFormulaRequest` → `RuntimeFormulaResult` (`:940-1027`) carrying semantic plan, execution contract, candidate/commit decision, trace, `ArtifactReuseReport` (`host/mod.rs:52-57`: green tree / red / bound / plan reuse booleans). OxCalc's `MinimalUpstreamHostPacket::build_bind_context` sets per-formula caller anchor (`OxCalc/src/oxcalc-core/src/upstream_host.rs:165-244`). `SingleFormulaHost` (`host/mod.rs:72-94`) caches artifacts **per formula slot** — one cached artifact per cell.
- **Cell values are keyed by A1 strings** (`"Sheet!A1"`): area reads parse the target string and probe `cell_values` per cell (`eval/mod.rs:1928-2033`). For a 10^6-row grid this string-keyed `BTreeMap` seam is the scaling bottleneck, but it's behind the `ReferenceSystemProvider` trait, so OxCalc can supply a block-sparse provider without OxFml changes.
- **Parameterized plan**: structurally plausible. The only caller-coordinate dependence inside `CompiledFormulaPlan` is pre-resolved coords in `CompiledReferenceExpr::Atom(NormalizedReference)` (`eval/mod.rs:362-388`); everything else (function targets, LET slots, lambdas) is coordinate-free. One compiled plan + per-cell `(caller_row, caller_col)` closure = the §2 symbolic-ref change plus constructing `EvaluationContext` without recompiling (today `EvaluationContext::new` recompiles per context, `eval/mod.rs:1697-1701` — plan caching keyed by `bind_hash` would fix this).

## 4. Ranges / arrays / aggregates

- `ReferenceExpr` supports `Range`, `Union`, `Intersection`, `Spill` (anchor `A1#`) (`binding/reference.rs:186-204`); whole-row/whole-column refs bind (`WholeRowRef/WholeColumnRef`). Array literals (`BoundExpr::ArrayLiteral`).
- Areas materialize to `CalcArray` via `enumerate_values`/`calc_array_from_local_area` (`eval/mod.rs:1928-1948`) — **dense materialization of the rectangle**, blank-filled; aggregates (SUM etc.) dispatch through OxFunc `FunctionCallTarget` over those arrays. No streaming/lazy range aggregation today.
- Spill: bind-level `Spill` ref + `EvaluationRequirement::SpillReference` (`semantics/mod.rs:147`); commit-side consequences are typed in the seam — `SpillEvent`, `SpillFact`, `ShapeDelta`, `Extent`, `Locus` (`lib.rs:110-119`, `seam/mod.rs`). Spill extent resolution is host-owned.
- **Implicit intersection exists**: `BoundExpr::ImplicitIntersection`, `_XLFN.SINGLE`/`SINGLE` special form (`eval/mod.rs:447`), `EvaluationRequirement::ImplicitIntersection` — not a gap.

## 5. Reusable authoring surfaces (DnaTreeCalc handovers)

- **LET/LAMBDA**: full special forms with slot-compiled helpers, `LambdaLiteral`, `Invocation`, `PortableCallableValue` with `captured_refs` for host storage/invalidation (`host/mod.rs:107-112`); `HelperEnvironmentProfile` (`semantics/mod.rs:227-236`). `HANDOVER_OXFML_lambda_node_invocation` (Open): the missing piece is invoking a **host-reference carrier** as callee under a TreeCalc capability profile — `BoundExpr::Invocation` + `HostReference` exist, profile gating doesn't.
- **Dry bind** (`HANDOVER_OXFML_dry_bind_preview`, Partial): `RuntimeDryBindVerdict/RuntimeDryBindInputKind/RuntimeDryBindProfileViolationKind` now consumed by OxCalc (`consumer.rs:11-13`).
- **Paste-special** (Open): computed-value→authored-input literalization shipped for scalars + array constants (`RuntimeAuthoredInputResult`); **formula rebind/recomposition per target caller context does not exist** — exactly the machinery a grid fill/region-stamp needs.
- **Conditional formatting**: CF/DV restricted sublanguages validated in `carrier.rs` (`validate_conditional_formatting_formula:43`, typed `CarrierRestrictionCode`s); typed CF rules + `ArrayCellFormatGrid` per-cell result shape in `publication` (`lib.rs:94-101`). Multi-rule ordering/Stop-If-True accumulation still open per handover.
- Editor services: `EditorEditService`, completion application (`consumer/editor/mod.rs:30-131`); `docs/spec/formula-language/OXFML_EDITOR_LANGUAGE_SERVICE_AND_HOST_INTEGRATION_PLAN.md`.

## 6. Missing for Excel-grid semantics

1. **A1 `$` absolute/relative fidelity** — dropped at bind (`binding/mod.rs:2926-2956`); blocks fill/copy rebind and R1C1 normal-form equivalence.
2. **A1 relative-to-caller semantics** — A1 refs bind as absolute coords; no caller-offset interpretation, so no R1C1 normal form derivable from A1 entry.
3. **Symbolic/offset bound references** — everything resolves to absolute coords at bind; per-region shared artifacts need a new bound form (§2).
4. **R1C1 display/composition channel** — R1C1 is entry-only floor; no whole-row/col R1C1 parity, no A1↔R1C1 text recomposition (doc "Explicit Residuals").
5. **Formula rebind API** (paste/fill/region stamping) — explicitly absent per paste-special handover.
6. **Scalable cell-value seam** — string-keyed BTreeMap fixture + dense area materialization; needs a block/sparse `ReferenceSystemProvider` and lazy aggregate iteration for 1M×16K.
7. **Grid bounds** — no 1,048,576/16,384 clamping or `#REF!` overflow semantics in ref parsing (`column_to_index` unbounded, `binding/mod.rs:3049`).
8. Cross-sheet refs parse (sheet qualifier + `CellRef.sheet_id`); cross-workbook is opaque `ExternalRef` with capability gating — resolution deferred to host.