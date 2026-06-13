# Design Proposal — Dual-Profile Code Structure & Repo/Crate Organization

## 1. The profile axis: kernel vs profile modules, and where code goes

**Profiles are runtime data, never cargo features.** A workspace's profile is persisted state (`CapabilityProfileId`, DnaTreeCalc `workspace.rs:43-56`) and the owner wants both lineages alive in one product line — a feature-flag build matrix would forbid mixed-profile processes (point 5) and split the test surface. Every binary compiles both profiles; gating is data-driven at parse/bind/dispatch chokepoints.

**Kernel (profile-neutral, never branches on profile):** values (`oxfunc_core` CalcValue + the aligned JSON wire), OxFunc function dispatch, dependency graph + invalidation closure (`dependency.rs`), recalc states (`recalc.rs:12`), revisions/candidate overlays/undo (`workspace_revision.rs`, `consumer.rs:789-801`), edge algebra/cycle machinery, `sparse_reader.rs`, value epochs, the coordinator/recalc-wave pipeline, TraceCalc oracle.

**Profile modules:** reference *forms* (`TreeReference` in `formula.rs:26-144` vs grid A1/R1C1), addressing (paths vs `PackedCellAddr`), container kinds (tree children vs sheet grid facet), syntax token tables, INDIRECT resolution, per-profile function overlays.

**Crate structure: do NOT split `oxcalc-core` now.** It is one 68k-LOC crate with treecalc.rs↔dependency.rs↔formula.rs tightly coupled, and the storage lens correctly notes the grid store must interleave with revisions/overlays (designs-storage.md Q1). Instead:

- `oxcalc-core/src/calc_target.rs` — the shared identity spine (below).
- `oxcalc-core/src/grid/` module family (`store.rs`, `template.rs`, `region.rs`, `axis_layout.rs`) — sibling of `structured_table.rs`, same facet pattern (`structural.rs:144`).
- `oxcalc-core/src/profile.rs` — typed `ProfileDescriptor` replacing the ≥6 scattered string comparisons (consumer.rs:4910; treecalc.rs:5543; session.rs:6995-7022).
- *Later, optional:* extract a leaf `oxcalc-grid-store` crate (PackedCellAddr, CompactCell codec, block store, AxisLayout — zero engine deps) once stable, purely to enforce kernel purity. Do not pre-create it. The file-I/O repo never imports it — its contract is `WorkbookConstructionSpec`, not our store.

**The placement rule:** *if code must know how a calc unit is addressed or contained, it is profile-module code; if it only needs identity/equality/ordering/serde of an opaque target, it is kernel code.* Mechanically: kernel code may `match` on `CalcTarget` only inside `calc_target.rs`-exported total functions; profile modules own all other matches. Use a **closed enum, not a dyn-trait container abstraction** — the engine is passive, single-threaded, serde-everywhere; enums keep Ord/Hash/serde trivial and match house style. Traits appear only at genuinely pluggable seams (`SparseRangeReader`, `TableBacking` §3, OxFml `ReferenceSystemProvider`).

## 2. CalcTarget is the shared spine — and the shared-debt refactors to pull forward

Yes: adopt designs-calcgraph.md's `CalcTarget { Node, GridRect, GridCell }` as the keying type for `DependencyDescriptor.target/owner` (`dependency.rs:36-40`), `InvalidationSeed`/`NodeInvalidationRecord` (`dependency.rs:450`), `Stage1RecalcTracker` (`recalc.rs:80-92`), scheduling plans (`treecalc.rs:454-460`), and value epochs. Tree profile uses `Node` only; the refactor is behavior-neutral for the existing lineage.

**Pull-forward refactors that serve BOTH lineages (sequence with calc-perf `calc-ekq3`):**
1. **CalcTarget newtype pass** — mechanical, Node-only, zero behavior change, full suite green. Land it first so calc-perf and grid both build on it.
2. **String-id interning** — `descriptor_id`/`edge_id` heap strings cloned into reverse_edges (`dependency.rs:420-427`), `FormulaArtifactId(String)` (`structural.rs:30`), descriptor versions embedding member-id lists (`dependency.rs:88-96`). This is also today's warm-run pathology fuel (`value_cache.rs:173`).
3. **Persistent incremental dependency graph** — per-run rebuild (`treecalc.rs:845`, double-build at :851-857) is the calc-perf target AND a grid prerequisite. One implementation; calc-perf lands it, grid consumes it.
4. **Typed profile/policy ids** — `CapabilityProfileId` end-to-end; replace `runtime_policy_id.contains(...)` dispatch (`treecalc.rs:1035,1656-1690`) with a typed engine enum; collapse the duplicate `treecalc_host_reference_syntax_profile()` (consumer.rs:5658 vs treecalc.rs:7797) and the two inconsistently-normalizing `RuntimeHostFormulaContext` constructors (treecalc.rs:5540 vs consumer.rs:5689); hard-error unknown persisted profiles (delete the `Box::leak` tolerance, session.rs:7017-7022).
5. **Template/plan caching** — kill per-node `prepare_oxfml_formula` (`treecalc.rs:798-811`); grid needs it for regions, tree benefits for repeated formulas immediately.

## 3. Tables: one lowering seam, two backings

Split `structured_table.rs` into **table-semantics core** (sections `#Headers/#Data/#Totals` at :1875, column-formula-per-row evaluation at :3501, totals, structured-ref dependency-descriptor emission) and a **`TableBacking` seam** supplying cell reads + row identity + placement:

- **Tree backing** (existing): `TreeCalcTableNodeSnapshot` (:132-151), optional `body_cell_nodes` materialization, `virtual_anchor` optional/virtual.
- **Grid-overlay backing** (new, in `grid/`): the table is a claimed rect on the SheetStore; `virtual_anchor` (:51) becomes the *actual* placement; cells ARE grid cells; overlay collisions error per Excel.

The seam's contract: semantics core reads only through `SparseRangeReader` (tree backing already does — `TreeCalcTableSparseReader` :2973) and emits descriptors targeting `CalcTarget` — tree backing emits node/table-region targets, grid backing emits `GridRect` targets. OxFml stays unchanged: `NormalizedReference::Structured(table_id)` resolves through the host resolver to whichever backing owns the id. The export `TableSpec` (EXCEL_EXPORT_AND_REPLAY.md:118-166) is the grid-overlay shape already — both backings lower to it.

## 4. Host/UX: grid host stays in DnaTreeCalc; repo split is a later Cargo move

**Recommend: no new sibling host repo now.** The skin framework already provides everything a grid host needs — `WorkspaceSkin` mount (skin-framework `skin.rs:92`), dispatcher/intent log/persona gating, preview seam (`preview.rs:34`), and the anticipated per-skin event channel (`skin.rs:62-66`) for the tile-stream that bypasses `WorkspaceState`. Duplicating `session.rs` (~7k lines of persistence/undo/intents) into a fresh repo forks the part that should stay shared.

Structure: the grid host is a **profile-switched shell mode** — workspace profile `strict-excel-grid` selects a grid-first lens set and a (possibly separate) web entry beside `dnatreecalc-web/src/lib.rs:188`. Charter tension (`CHARTER.md:44` "not a grid"; PreCalc is the grid host) resolves as: the *repo* hosts a host suite; the grid mode is the PreCalc proving line living in the same workspace. The crate boundaries (host/shell/skin-framework/skins/web, all web-sys-free except `dnatreecalc-web`) mean a future PreCalc repo split is a Cargo.toml relocation, not a rewrite — defer it until grid UX divergence forces framework changes that are grid-only.

**Migration story / Phase B non-blocking:** grid lens registers as a normal Main-slot skin; tile streaming is a new read-side channel, so B.2.2 worker and B.2.3 delta-protocol work proceed untouched; grid intents enter the closed `WorkspaceIntent` enum (replay/persona free); everything is gated on workspace profile so the atlas line never sees it.

## 5. Cross-profile doors (open, not designed)

Profile is **per-workspace** (already persisted per workspace; `LocalTreeCalcEnvironmentContext` carries it per context). Mixed-profile processes therefore come for free. Cross-profile references reduce to cross-workspace edges (`workspace_reverse_edges`, `dependency.rs:246-254`) whose targets are `CalcTarget`s — kernel-representable today. Doctrine to encode now, at a **single edge-admission chokepoint** in descriptor lowering: strict-excel formulas may never reference tree targets (Excel conformance); tree→grid is permitted-in-principle, deferred; hybrid is PreCalc's future profile. Values cross on the profile-neutral CalcValue wire.

## 6. OxFml gating seams (concrete)

Retire the text scan (`strict_excel_unsupported_profile_diagnostic`, OxCalc consumer.rs:4900-4953) in favor of the three real axes:
- **Parse:** `HostReferenceSyntaxProfile` (parser.rs:14-93) is already the right shape — strict-excel-grid supplies empty TreeCalc token tables; make the five `treecalc_host_reference_syntax_profile()` call sites profile-conditional. Fix the non-default-profile incremental-reuse penalty (host/mod.rs:430-447) before grid uses it everywhere.
- **Bind:** introduce a typed `BindProfile { reference_syntax, a1_relative_to_caller: bool, grid_bounds: Option<GridBounds>, … }` on `BindContext` (binding/mod.rs:258) — carrying the A1 `$`-fidelity fix (binding/mod.rs:2926-2956), caller-relative A1, bounds→`#REF!` (column_to_index unbounded, :3049), and the symbolic-ref form. Keep `FormulaChannelKind` orthogonal (WorksheetA1/R1C1 serve the grid as-is).
- **Capability:** per-profile OxFunc `CapabilityOverlay` (registry.rs:450) composed by the host — deny tree-only surface under strict-excel; existing `HostProfileUnavailable` + dry-bind violation plumbing (consumer/runtime/mod.rs:272-370) works unchanged.
- **Identity:** replace `dialect_id`/`capability_profile_id` free strings (consumer/runtime/mod.rs:2524-2534) with a typed descriptor that serializes to today's strings (cache identity preserved); un-conflate `FenceSnapshot.profile_version` from locale (host/mod.rs:803-814).
- **INDIRECT:** profile-selected `ReferenceSystemProvider`; `a1_style` flag exists (resolver.rs:103-108).

## Hardest problems + derisking

1. **CalcTarget refactor collides with in-flight calc-perf.** Same files, same hot types. Derisk: land the mechanical Node-only newtype pass first, coordinate explicitly with the calc-perf worktree (workflow w77t18wyu), and let calc-perf's persistent graph be built target-typed from day one.
2. **Replacing the string scan without gating regressions.** Derisk: keep the scan as a secondary assert during transition; expand `tests/active_profile_gating_corpus.rs` + `docs/test-corpus/profiles/gating.json` into the conformance contract; differential-run every case old-vs-new with the invariant "new never accepts what old rejected."
3. **structured_table.rs semantics extraction.** Large, tree-assumptions interleaved. Derisk: pin characterization tests from `active_table_corpus.rs` first; extract by parameterizing *reads only* (`TableBacking` with the existing sparse reader as first impl); never relocate evaluation logic in the same change.

## Build-first vs defer

**First:** CalcTarget newtype + interning; typed profile ids + dedup/normalization fixes + unknown-profile hard error; profile-conditional syntax tables with corpus differential; `BindProfile` + A1 `$` fidelity; `grid/` module skeleton behind profile; `TableBacking` extraction. **Defer:** `oxcalc-grid-store` crate extraction; PreCalc repo split; TreeCalc `FormulaChannelKind`; cross-profile reference semantics (reserve the chokepoint only); OxFunc `gating_profile_ref` catalog variance beyond the strict-excel overlay; any cargo-feature gating (rejected permanently).

## Open questions for the owner

1. Confirm profiles-as-runtime-data: both profiles in every binary, no feature flags — acceptable binary/test stance?
2. Does the grid shell mode ship branded "PreCalc" inside this repo (charter amendment) or as an unnamed TreeCalc mode until the repo split?
3. Who composes the strict-excel `CapabilityOverlay` — OxFunc seed catalog (make `gating_profile_ref` real) or host-side lists in `session.rs`? I recommend OxFunc-owned, host-selected.
4. Is `TableBacking` extraction a prerequisite (my recommendation) or do we tolerate a temporary parallel grid-table implementation?
5. Ratify the cross-profile invariant "strict-excel formulas never reference tree targets" as spec text (CORE_MODEL_SPEC §4 amendment).