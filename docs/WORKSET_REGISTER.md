# Workset Register — DNA TreeCalc

One living register of the large planned work areas. Worksets are big pushes, not atomic tasks (those are beads — see [`../OPERATIONS.md`](../OPERATIONS.md) §5). Format and lifecycle are defined in [`../OPERATIONS.md`](../OPERATIONS.md) §4.

**Lifecycle:** `OPEN → IN PROGRESS → CLOSED`. The status here is a coarse human scan, not the live execution board. Once the bead store exists, every workset recorded here has a matching epic bead; `OPEN` means that epic exists but is not yet underway. The bootstrap workset is the only exception because it creates the bead store. Use `.beads/` for truth (`br epic status`, `br ready`, `br list --status in_progress`). Worksets are re-scoped and re-sequenced freely. **Closing a workset includes a housekeeping pass** ([`../OPERATIONS.md`](../OPERATIONS.md) §9) — bring touched docs/artifacts to a known, in-its-place state (keep / mark / delete); delete confidently-dead things (git backs you), but mark or defer rather than sweep out anything you genuinely doubt.

**Naming:** `W###_short_name`, sequential.

**Verification line:** every workset names which gate tiers apply (local / Excel-anchor / formal — see [`../OPERATIONS.md`](../OPERATIONS.md) §6) and any obvious scaffolding needed. The point is to make real checks easy to build early, not to create a paperwork lane.

This is the initial seed map, derived from the Spec (`SPEC.md`) and the UX technical-plan phasing (`ux/TECHNICAL.md` §10). Worksets are refined as beads roll out.

---

## Epic bead pairing

These epic beads were created during W001 bootstrap. Use this table only to pair register worksets with bead epics; use `br epic status`, `br ready`, and `br show <id>` for live execution state.

| Workset | Epic bead |
|---|---|
| W001_repo_bootstrap_and_doctrine | `dtc-jfv` |
| W002_engine_seam_and_context | `dtc-d6o` |
| W003_tree_shell_and_core_editing | `dtc-mm0` |
| W004_reference_model_and_resolution | `dtc-z0i` |
| W005_walking_skeleton_min_end_to_end | `dtc-osq` |
| W006_additional_skins | `dtc-366` |
| W007_meta_nodes_formatting_templates | `dtc-fks` |
| W008_excel_import | `dtc-xlx` |
| W009_excel_export_and_replay | `dtc-p5q` |
| W010_udf_hosting | `dtc-dht` |
| W011_dnacalc_host_core_xlsx_notebook_proof | `dtc-hj2` |
| W012_measured_excel_conformance_instrument | `dtc-5zet` |

---

## Default sequence

```
W001 → W002 → W005 → { W003, W004 } → W006 → W007 → W008 → W009 → W010
(boot)  (engine (walking   (flesh out:     (more     (meta/  (import)(export)(UDF)
        context) skeleton)   shell+editing,  skins +   fmt/
                             reference       framework templ)
                             model)          hardening)
```

**Slice then flesh.** W005 is a deliberate **walking skeleton**: the thinnest real slice that runs end-to-end — minimal skin framework + minimal shell + two skins of different types + bare-name walk-up resolution through direct OxCalc context execution + `.dnatree` persistence + the first end-to-end tests — proving the whole stack (OxCalc context → shell → skins → tests) at the most limited scope. Everything after it *fleshes out* a working, visible, tested base: W003 deepens the shell and editing, W004 completes the reference model, W006 adds the remaining skins and hardens the framework. **W005 deliberately owns thin slices of the shell (W003), resolution (W004), and a second skin (W006); those worksets own the full depth.** This is why W005 precedes W003/W004 despite the numbering — worksets are not executed strictly by number. The v1 skin bar is `triple-editor` + `outline-table` (skeleton) + `cell-view` (W006 must-have); `canvas-flow` and `nodes-across` are enrichment, not v1 gates.

**Cross-cutting from the foundation.** Two build targets are stood up early — the browser WASM shell and the native **Tauri** desktop shell (the native-code-hosting vehicle; `ux/TECHNICAL.md` §1, §1.1) — so neither is retrofitted. A **performance measurement harness** with timed stress workloads (`ux/TECHNICAL.md` §7.6; `docs/test-corpus/perf/`) is scaffolding built as soon as the OxCalc context supports it, since iterating on engine+host speed is part of the proving-ground mission, not a late add.

Engine prerequisites in OxCalc/OxFml/OxFunc (Spec `model/CORE_MODEL_SPEC.md` §6) gate several worksets; those are coordinated via handovers, not owned here. New since the seed map: version-based undo (§6 item 13), table-node unpacking (§6 item 14 — Tables are a cross-repo build area, in scope), and node-as-function invocation (§6 item 15).

**W011 pivot.** W011 is an explicit near-term integration pivot, not an item at the tail of the original tree-only sequence. It proves the DnaCalc host pattern for `.xlsx` workbooks while reusing and cleaning up the skin architecture that TreeCalc has already grown. It may proceed alongside existing open worksets where dependency edges allow; live execution truth stays in `br`. The dual-profile hosting scope (tree workspaces + `.xlsx` grid workbooks through `dnacalc-host-core`) is now settled in `CHARTER.md` "Not a grid", evidenced by the host-core save/reopen tests (`dtc-j7n8.7`, cached `B1 = 30`).

---

## Worksets

### W001_repo_bootstrap_and_doctrine
- **Purpose:** Stand up the repo as a conformant project: doctrine docs (CHARTER/OPERATIONS/AGENTS/README), Spec set + index, this register, `.beads/` init, `.gitignore`, `git init`.
- **Depends on:** —
- **Spec sections:** all of `SPEC.md` (the migrated design set).
- **Closure condition:** repo has the doctrine doc set, an initialized bead store, and a first epic+beads created; `br ready` returns work.
- **Initial epic lanes:** doctrine docs; bead-store bootstrap; spec-set indexing.
- **Verification:** Local only — doc set present, `git status` works, bead store is initialized, and `br ready` returns work. No Excel/formal tier.
- **Status:** CLOSED

### W002_engine_seam_and_context
- **Purpose:** Establish direct use of OxCalc's exported TreeCalc engine context for multi-node recalc. DnaTreeCalc creates and edits workspaces through `OxCalcTreeContext`, then reads values, dependency graph, calc state, and diagnostics from OxCalc views/outcomes. Reuse the OxFml editor surface for editing text, but not for owning tree semantics.
- **Depends on:** W001. Engine prereqs in OxCalc (handover).
- **Spec sections:** `model/CORE_MODEL_SPEC.md`; `ux/TECHNICAL.md` §4.
- **Closure condition:** a workspace of named nodes evaluates end-to-end through direct OxCalc context calls; per-node values and calc-state are observable.
- **Initial epic lanes:** context contract; live-edit orchestration; calc-state plumbing.
- **Verification:** Local recalc unit tests over a small multi-node fixture. Excel anchor deferred to W009. Scaffolding: direct-context test harness + a minimal multi-node workspace fixture.
- **Current closure note:** W002's original one-shot proof is migration history. The active product boundary is now direct `OxCalcTreeContext`; DnaTreeCalc must not submit semantic request/result DTOs, formula catalogs, prepared carriers, or host-side reference resolutions. The W002 corpus validator remains green with zero active W002 corpus cases by design: `constants/entry-classification` is pending on the OxFml TreeCalc entry-classification API, and `cycles/cycle-profiles` is pending on executable typed cycle configuration/outcome fields in the current OxCalc context surface. Those are exact successor blockers, not hidden W002 closure claims.
- **Status:** CLOSED

### W003_tree_shell_and_core_editing
- **Purpose:** *Flesh out* the workspace shell and structural editing from the W005 skeleton — the full nav rail (virtualized rows, search/filter, drag-reorder, context menu, breadcrumb, meta-visibility toggle), the full structural-edit service (insert/rename/move/delete), the full three-pane TripleEditor experience, and complete `.dnatree` persistence (all meta-namespaces, undo grouping).
- **Depends on:** W005 (the walking skeleton — which already proves the minimal shell, skin mount, and persistence). Collaborates with W004 on rename/move propagation.
- **Spec sections:** `ux/REQUIREMENTS.md` §2–§3; `ux/TECHNICAL.md` §6; `ux/SKINS.md`; `ux/TRACEABILITY.md`; `ux/IMPLEMENTATION_MATRIX.md`.
- **Closure condition:** a user can build, edit, save, and reopen a substantial tree of named nodes in the three-pane skin, with full node CRUD and the nav-rail affordances.
- **Initial epic lanes:** nav-rail features; structural-edit service; full persistence + undo; TripleEditor depth. Activates UX trace slices `UX-SH`, `UX-TR`, `UX-FE`, `UX-VA`, `UX-ST`.
- **Verification:** Local + a UX click-through on the running shell + save/reopen round-trip. Scaffolding: reuse the skeleton's click-through + persistence harnesses.
- **Status:** OPEN

### W004_reference_model_and_resolution
- **Purpose:** *Flesh out* the reference surface from the skeleton's bare-name walk-up into the full TreeCalc v1 reference suite: bare walk-up and dotted descent; `^`/`^^` ancestors; `[]`/`[ws]` current/cross-workspace anchors; workspace aliases and `!` sheet-position aliasing; bracket escaping; case-insensitive lookup with canonical display-path reporting; meta-node invisibility and `@` accessors; sibling navigation (`@PREV`/`@NEXT`); ordered collections (`@CHILDREN`/`.*`, `@PRECEDING`, `@FOLLOWING`, `@ANCESTORS`); recursive descent `**`; reference-array literals and mixed-array rejection; profile gating; dynamic `INDIRECT` / CTRO references; node-as-function invocation for lambda-valued nodes; structural-edit rebind/propagation prompts; and full table structured references.
- **Depends on:** the closed W002 direct context baseline plus the already-proven raw children/ordered-selector migration slices. W005 still owns the product walking-skeleton UX/persistence/click-through closure, but W004 successor reference beads may execute independently once their engine-facing packet is available. Engine prereqs (SelfNode, set-membership deps, profile selectors, dynamic refs, reference-array carriers, node-as-function binding, table structured-ref lowering) via handover.
- **Spec sections:** `model/CORE_MODEL_SPEC.md` §3, §4, §6, §7c, §8.
- **Closure condition:** references resolve per spec across the full surface through direct OxCalc context execution; rename/move propagation prompt works; editor hover shows resolved bindings/canonical target paths; table structured references lower through the agreed engine contract rather than host parsing.
- **Initial epic lanes:** anchors + sibling-offsets; set-membership + ordered collections + recursive descent; reference literals; cross-workspace + aliases + `!`; case/canonicalization + escaping; profile gating; INDIRECT/dynamic; node-as-function; table structured refs; propagation UX.
- **Verification:** Local — **progressively activate** the authored corpus themes (`references/*`, `profiles/`, `dynamic-references/`, `structural-edits/`, `arrays/`, `tables/`) from `pending` to `active` only when the Rust corpus runner exercises them through the real OxCalc context. Excel anchor applies where name-scope, `INDIRECT`, LAMBDA, and structured-reference behavior maps to Excel; TreeCalc-only path/set behavior is pinned by corpus cases. Scaffolding: the test corpus (`docs/test-corpus/`, already authored/expanded) + the skeleton's corpus runner.
- **Status:** IN PROGRESS

### W005_walking_skeleton_min_end_to_end
- **Purpose:** The **walking skeleton** — the thinnest real slice that runs end-to-end through the skin types. Minimal skin framework (`WorkspaceSkin`, `SkinContext`, `Dispatcher` + a minimal closed `WorkspaceIntent`, registry + switcher, `SkinStateHandle`); a minimal shell (context strip, nav rail, one main mount slot, status foot); **two minimal skins of different categories** (`triple-editor` + `outline-table`) to prove runtime switching; minimal bare-name walk-up plus dotted descent resolution through direct OxCalc context execution; minimal `.dnatree` persistence; and the **first end-to-end test surfaces** (a corpus runner + a UX click-through).
- **Depends on:** W002 (direct context baseline — done). This is the pivot: it owns *thin* slices of the shell (W003), resolution (W004), and a second skin (W006); those worksets own the full depth.
- **Spec sections:** `ux/SKINS.md` §1–§2, §7; `ux/IMPLEMENTATION_MATRIX.md` (`UX-SK`, thin `UX-SH`/`UX-TR`/`UX-FE`/`UX-VA`; scenario cards S1–S4); `ux/TRACEABILITY.md` (F1, F8, F9); `model/CORE_MODEL_SPEC.md` §3.2 (bare-name walk-up only).
- **Closure condition:** the skeleton runs — a tiny workspace loads; editing a node formula updates its value through the **real OxCalc context**; switching `triple-editor` ↔ `outline-table` preserves shared selection with **no recalc**; save/reopen round-trips using OxCalc-owned stable identities. The **first slice of the test corpus is activated** (`references/walkup-raw-active` bare-name and dotted-descent cases the minimal resolution supports, green against direct context via the corpus runner), and a **UX click-through passes**.
- **Initial epic lanes:** minimal skin framework + registry/switcher; minimal shell + nav rail; two minimal skins; bare-name walk-up + dotted descent over direct context; `.dnatree` round-trip; **corpus runner v1** (activate-a-slice through OxCalc, not a local parser) + **click-through harness v1**.
- **Verification:** Local — the corpus runner over the activated walk-up/dotted-descent slice + reducer/projection tests — plus a UX click-through (edit->value, switch-skin->no-recalc, save/reopen). This workset *first runs* the test-corpus **activation** model and the UX-matrix harness for real. Scaffolding: corpus runner v1; click-through harness v1; a tiny end-to-end workspace fixture.
- **Current direct-context note:** earlier one-shot evidence is migration history only. Current and future active slices must construct an `OxCalcTreeContext`, add workspaces/nodes/tables/formula text through OxCalc APIs, run `recalculate`, and assert values/diagnostics/dependency facts from OxCalc views/outcomes. DnaTreeCalc must not construct formula catalogs, prepared carriers, path/member packets, or table classifier inputs. Raw children, ordered selectors, sibling offsets, reference-only literal arrays, focused walk-up/dotted names, focused ancestor anchors, focused bracket-escaped paths, meta-node invisibility, structural-edit propagation consequences, and tables now have direct-context active slices; raw dynamic/cross-workspace formulas, node-as-function calls, full walk-up name/cell precedence cases, workspace-root/sheet-alias anchors, alias/base-token variants outside the focused packets, profile-gated syntax, and other not-yet-admitted families remain typed pending/exclusion lanes until OxCalc/OxFml exposes direct context support.
- **Status:** OPEN

### W006_additional_skins_and_framework_hardening
- **Purpose:** The remaining v1 skins on the hardened framework, plus the hardening itself. **v1 skin bar:** the skeleton already ships `triple-editor` + `outline-table`; **`cell-view` is the must-have completion** (Excel-fluent data entry) — those three are the v1 bar (CHARTER's "good enough to use"). **`canvas-flow` and `nodes-across` are nice-to-have enrichment** that may land after v1 proves out. Hardening, as the skins stress the framework: the full shared primitive library lift from OneCalc, full `SkinState` (schema migration + GC), shared-state depth, multi-slot composition, the `FormatResolver` seam, and the cross-skin equivalence harness.
- **Depends on:** W005 (skeleton), W003 (shell affordances the skins reuse).
- **Spec sections:** `ux/SKINS.md` §3, §6, §7; `ux/TRACEABILITY.md`; `ux/IMPLEMENTATION_MATRIX.md` (`UX-CV`, `UX-GR`, remaining `UX-VA`/`UX-SK`); `ux/prototypes/`.
- **Closure condition:** the must-have `cell-view` renders/edits the same workspace and switches cleanly with the skeleton's skins (no recalc); the nice-to-have skins (`canvas-flow`, `nodes-across`) land as enrichment, with the canvas group→template affordance.
- **Initial epic lanes:** primitive library lift; skin-state hardening (migration/GC); **cell-view (must-have)**; **canvas-flow + nodes-across (nice-to-have)**; cross-skin equivalence harness.
- **Verification:** Local + per-skin click-through + cross-skin equivalence. Scaffolding: cross-skin equivalence harness (reused thereafter).
- **Status:** OPEN

### W007_meta_nodes_formatting_templates
- **Purpose:** Meta-nodes (`is_meta`), per-node formatting via `Format` meta-children with inheritance, and host-level templates (instantiate, per-leaf override, sync).
- **Depends on:** W004, W005. Engine prereqs (`is_meta` flag, transactional batch edit) via handover.
- **Spec sections:** `model/META_NODES.md`; `model/CORE_MODEL_SPEC.md` §7b; `ux/REQUIREMENTS.md` §2.8.
- **Closure condition:** templates instantiate and sync; format editor reads/writes `Format` meta-children; inheritance walk works.
- **Initial epic lanes:** is_meta plumbing; format editor + inheritance; template index/bookkeeping + sync.
- **Verification:** Local for template instantiate/sync/override semantics; Excel anchor for number-format and conditional-format **rendering** via OxReplay display-faithful views (where the format model is Excel-aligned). Scaffolding: format/CF fixture corpus; template-sync test fixtures.
- **Status:** OPEN

### W008_excel_import
- **Purpose:** Import Excel workbooks restricted to sheets + defined names with zero formula rewriting (dots unroll as paths; `!`-after-sheet; stub intermediates).
- **Depends on:** W004.
- **Spec sections:** `model/CORE_MODEL_SPEC.md` §10.
- **Closure condition:** a defined-names-only workbook imports and recomputes equivalently; import preview + manifest produced.
- **Initial epic lanes:** importer; stub handling; import preview UX.
- **Verification:** **Excel anchor primary** — imported defined-name workbooks recompute equal to Excel (value-equivalence per node). Scaffolding: a defined-name workbook corpus + an import verify harness driving OxXlPlay (observe Excel) and OxReplay (diff).
- **Status:** OPEN

### W009_excel_export_and_replay
- **Purpose:** Convert a workspace to a `WorkbookConstructionSpec`, emit the TreeCalc replay bundle, verify against Excel via OxXlPlay + OxReplay. Heavy on cross-repo handovers.
- **Depends on:** W004, W007. Major handovers: OxXlPlay workbook construction; OxReplay TreeCalc lane.
- **Spec sections:** `interop/EXCEL_EXPORT_AND_REPLAY.md`.
- **Closure condition:** `verify-workspace` runs end-to-end (export → construct → observe → diff) for a value-equivalence case.
- **Initial epic lanes:** model→spec converter; replay-bundle emitter; verification orchestration.
- **Verification:** **Excel anchor primary** — `verify-workspace` value-equivalence end-to-end. This workset *builds* the shared verification scaffolding the family reuses: the model→spec converter, the replay-bundle emitter, the `verify-workspace` command, and the export manifest. Treat the scaffolding lanes as early/blocking within the workset.
- **Status:** OPEN

### W010_udf_hosting
- **Purpose:** Integrate the shared UDF-hosting core (VBA + `.xll`) once extracted from DnaOneCalc. Off the critical path.
- **Depends on:** shared UDF core (external; handover).
- **Spec sections:** `ux/TECHNICAL.md` §1.1; `interop/EXCEL_EXPORT_AND_REPLAY.md` §12b.
- **Closure condition:** UDF-using workspaces evaluate and verify with UDFs provisioned into Excel.
- **Initial epic lanes:** shared-core consumption; Excel-side provisioning handover follow-through.
- **Verification:** Excel anchor with UDFs provisioned into Excel (OxXlPlay VBA + `.xll` provisioning), reusing the W009 verify-workspace path. Scaffolding: a UDF fixture set (VBA module + a sample `.xll`).
- **Status:** OPEN

### W011_dnacalc_host_core_xlsx_notebook_proof
- **Purpose:** Create the first clean DnaCalc reference host that marries OxDoc and OxCalc end-to-end: open a small `.xlsx` through OxDoc, have the host own the OxDoc source/model context and the OxCalc workbook context, pass the neutral model into OxCalc, render workbook grids through Skin IR in a B1 Pluto-style notebook, edit a cell, recalculate dependents, and save/download a round-tripped workbook.
- **Depends on:** Existing skin/host skeleton code in this repo. Upstream OxCalc/OxDoc work is coordinated via W011 handovers, not direct writes from this repo. W010 remains separate and off the critical path.
- **Spec sections:** `ux/DNACALC_HOST_CORE_XLSX_NOTEBOOK_PROOF.md`; `ux/SKINS.md`; `ux/THREE_FRONTENDS_PLAN.md`; `interop/UPSTREAM_OX_LANES.md`.
- **Closure condition:** fixture workbook `A1 = 7`, `B1 = =A1*3` opens in B1; editing `A1` to `10` through `WorkspaceIntent::EnterGridCell` makes `B1` show `30` through OxCalc's three-way literal/formula/clear branch (OxFml is the sole interpretation authority; formula text is accepted, not rejected — proven on the loaded fixture by `dtc-j7n8.6`), the receipt carrying `GridCellEntered` plus the edited sheet's complete `GridChanged` — and one per other sheet the edit's cross-sheet recalc moved — so a retained mirror patches in place without a snapshot under Automatic calc mode (`dtc-j7n8.18`; proven on the single-sheet fixture, the two-sheet demo workbook, and the loaded two-sheet `cross_sheet` fixture whose save reopens with `Sheet2!A1` cached `20` — `dtc-j7n8.14`; the Manual-mode `workbook_calc` lag is `dtc-j7n8.20`); save/download reopens through OxDoc with `A1` changed, `B1` formula text preserved, **and `B1` cached value updated to `30`**; the notebook uses only Skin IR; `dnacalc-skin-ir` and the host core compile/test without Leptos (dev-deps included); the same document mounts notebook-only and notebook plus companion skin; full/strict/values profile behavior is covered by the first fixture lane. The save proof is native (byte buffers); the browser download click-through completes it. Worker-boundary alignment (`dtc-hj2.13`) does not gate closure.
- **Initial epic lanes:** workset/register anchoring; Skin IR split (`dnacalc-skin-ir` + `dnacalc-skin-leptos`); Leptos-free `dnacalc-host-core` (the THREE_FRONTENDS Gap-4 SessionEngine crate) + oxdoc wasm spike; model-neutral sessions (`DocumentSession` enum: `RichTreeSession`, `WorkbookSession`); OxCalc/OxDoc handovers + `[U-INGEST]` lane registration; `.xlsx` open/ingest (host-side `oxdoc-model`→`GridBackingSeed` translation + fixture + authored-metadata IR); read-only B1; `EnterGridCell` edit/recalc loop (engine three-way branch; `dtc-j7n8.6`); browser open/download; save/reopen (host-assembled whole-model projection); multi-skin layout (host-unioned grid interest); strict-grid profile lane; worker-boundary alignment.
- **Verification:** Local build/test/lint/format for touched crates; no-Leptos dependency checks for pure core crates; Skin IR protocol tests; host-core open/edit/recalc/save tests; browser click-through for open/edit/recalc/download; OxDoc reopen assertions for saved bytes; Excel anchor for workbook round-trip and formula/value preservation. Formal tier remains a design aim but does not gate this first host proof.
- **Status:** OPEN

### W012_measured_excel_conformance_instrument
- **Purpose:** The DnaTreeCalc-owned slice of the program's "Measured Excel" campaign (`../Foundation/notes/PROGRAM_INVESTIGATION_2026-09-10.md` §3, approved 2026-09-10): campaign W0 (decide the harness's home, prove the FileCached false green in a committed test, register the orchestration constraints) and W1 (the instrument). The instrument is a new TC-tier crate `src/dnacalc-conform` that opens every `.xlsx` in a corpus directory through the same path the product runs (`HostCommand::OpenXlsxBytes` -> ingest -> `recalculate_workbook`, forced regardless of the file's `calcMode`), applies a provenance-first verdict ladder to every cell carrying both `<f>` and `<v>` (only `PublishedValueProvenance::Calculated` may score Match/Mismatch; `FileCached`/`Stale`/`Degraded`/`NotCalcModeled` are `NotEvaluated`; volatiles are `Excluded`), and retains a `divergence.v1.json` whose primary number is coverage (`engine_calculated_cells / total_formula_cells`), with per-class mismatch counts, escalation counters, a reader-ceiling report, and the first retained grid-engine wall-clock baseline beside it.
- **Depends on:** W011 (host-core owns both the OxDoc source and the OxCalc context; `HostCommand::OpenXlsxBytes` / `SaveActiveXlsx` exist). Sibling-repo items are referenced, not owned: OxCalc `calc-9ouy.3` (delete the fixture-faked iterative surface), the OxCalc `CHARTER.md` §8 escalation amendment (handover), injection hooks for the reference-twin check (handover if needed). Corpus admission (campaign W2) is a later workset.
- **Spec sections:** `../Foundation/notes/PROGRAM_INVESTIGATION_2026-09-10.md` §1 (the mechanism), §3 W0-W1, Appendix A (harness-home decision), Appendix B (corpus census); `interop/UPSTREAM_OX_LANES.md`.
- **Closure condition:** one command, run from a path it was not compiled with, opens the 174 in-tree Excel-authored workbooks and produces a retained baseline carrying coverage %, per-class red, per-workbook denominators, the corpus hash set and `escalation_rate`; a committed host-core test demonstrates the FileCached false green; injected wrong values at the three blind sites named in the note fail the reference-twin check; wall-clock baselines are retained with explicit bands. No bit-exactness claim anywhere.
- **Initial epic lanes:** W0.1 harness-home decision (closed 2026-09-14); W0.2 false-green test (closed 2026-09-14: `src/dnacalc-host-core/src/xlsx_false_green.rs` over `fixtures/w012/false_green{,_manual}/` — a rejected `=Größe*2` publishes the file's `#NAME?` as a persistent `FileCached` pin in both calc modes; the open-fails-on-non-ASCII-name ceiling and the authored-view gap it surfaced are `dtc-5zet.5`/`.6`); W0.5 orchestration constraints; W1 rollout bead authoring 1.1-1.7 (crate + runner + verdict ladder; reader ceiling + census; manual-mode coverage test; calcChain oracle; twin fails loudly; wall-clock baseline; escalation counters). W1.1 (closed 2026-09-14, `dtc-5zet.8`): `src/dnacalc-conform` lib + bin, `diff --corpus <dir> --out <dir>`, provenance-first ladder read at the OxCalc level (`WorkbookSession::grid_view`), `divergence.v1.json` with coverage first and every verdict broken down by reason/class (definitions in the crate README). First baseline over the 109 in-tree Excel-authored workbooks: **coverage 0.2151 (91/423 formula cells)**; 43 of 109 workbooks fail to OPEN on the product path (`OxfmlEvaluationFailed` at the open-recalc — 40 on `_xlfn.*` callable invocation, 3 on the missing `locale_format_context`; 325 cells `not_evaluated:open_failed`); 7 external-reference pins; 7 excluded (NOW/TODAY and Excel's `ca="1"` closure); 69 match; 15 mismatch (structured-table refs, the intersection operator, RTD, `IF("0",1,2)`); 0 unclassified. W1.6 (`dtc-5zet.13`, 2026-09-15): `bench --corpus <dir> --out <dir>` — the first retained grid-engine wall-clock baseline through the product's own verbs, `docs/test-runs/w012/BASELINE_2026-09-15.md` (machine identity, every number, a band per number). Headlines: the three largest openable corpus files are 10-cell probes (product open 13-18 ms, single-cell edit 0.4-4.2 ms); the synthetic 2,000-row fill-down costs 8.6 ms per `enter_grid_cell` to author (every verb is O(authored cells): the bead's 10,000-row fill-down took 19.9 min to author at 59.6 ms/verb, 81 s cold F9, 0.65-0.76 s per single-cell edit, 2.1 GiB peak working set), 5.3 s cold F9, 147-170 ms per single-cell Automatic edit; the 500-row two-sheet workbook's single-cell edit costs 535-623 ms because cross-sheet propagation mark-alls the dependent AND re-enters the origin (measured by the origin's F9 counter); `GridSeededLaneOutcome`, the M-2 phase timers and the rebuild's mark-all are unobservable from the host (`docs/handovers/HANDOVER_OXCALC_lane_outcome_and_phase_timers_accessor.md`). Findings filed as `dtc-5zet.20`-`.24`; `.20` records that OxFml fml-kt8.2 (non-ASCII identifiers bind) landed mid-bead and took the W0.2 false-green fixture's premise with it (5 tests red across host-core and dnacalc-conform, untouched). `.20` (closed 2026-09-15): the fixture's `C1` re-cut to `=Is?Valid*2` — a `?` inside a defined name is legal by [MS-XLSX] §2.2.2, live Excel 16.0 accepted it and wrote the cell exactly as the fixture carries it (`t="e"`, `<v>#NAME?</v>`), and OxFml still rejects it at the lexer — so the pin lands on rung 1 again with every assertion unchanged (host-core 99 passed, dnacalc-conform 35 passed); the divergence the landing exposed (an unresolved NON-ASCII name evaluates `#VALUE!` where Excel gives `#NAME?`; ASCII names already give `#NAME?`) is catalogued on `dtc-5zet.18`, sibling fix OxCalc `calc-1q03.17`.
- **Verification:** local build/test/lint/format for touched crates; `dnacalc-arch-gates` positive control for the new TC crate; the runner's own retained artifact; two independent refuters per bead before close. Excel anchor is the cached `<v>` the files already carry (offline); live OxXlPlay observation is not required for this workset. Formal tier does not block this workset.
- **Status:** OPEN
