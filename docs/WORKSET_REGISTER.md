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
| W002_engine_seam_and_bridge | `dtc-d6o` |
| W003_tree_shell_and_core_editing | `dtc-mm0` |
| W004_reference_model_and_resolution | `dtc-z0i` |
| W005_walking_skeleton_min_end_to_end | `dtc-osq` |
| W006_additional_skins | `dtc-366` |
| W007_meta_nodes_formatting_templates | `dtc-fks` |
| W008_excel_import | `dtc-xlx` |
| W009_excel_export_and_replay | `dtc-p5q` |
| W010_udf_hosting | `dtc-dht` |

---

## Default sequence

```
W001 → W002 → W005 → { W003, W004 } → W006 → W007 → W008 → W009 → W010
(boot)  (engine (walking   (flesh out:     (more     (meta/  (import)(export)(UDF)
        bridge)  skeleton)   shell+editing,  skins +   fmt/
                             reference       framework templ)
                             model)          hardening)
```

**Slice then flesh.** W005 is a deliberate **walking skeleton**: the thinnest real slice that runs end-to-end — minimal skin framework + minimal shell + two skins of different types + bare-name walk-up resolution over the W002 bridge + `.dnatree` persistence + the first end-to-end tests — proving the whole stack (engine bridge → shell → skins → tests) at the most limited scope. Everything after it *fleshes out* a working, visible, tested base: W003 deepens the shell and editing, W004 completes the reference model, W006 adds the remaining skins and hardens the framework. **W005 deliberately owns thin slices of the shell (W003), resolution (W004), and a second skin (W006); those worksets own the full depth.** This is why W005 precedes W003/W004 despite the numbering — worksets are not executed strictly by number. The v1 bar is the **minimum lovable skin set** — `triple-editor` + `outline-table` (skeleton) + `cell-view` (W006 must-have); `canvas-flow` and `nodes-across` are enrichment, not v1 gates.

**Cross-cutting from the foundation.** Two build targets are stood up early — the browser WASM shell and the native **Tauri** desktop shell (the native-code-hosting vehicle; `ux/TECHNICAL.md` §1, §1.1) — so neither is retrofitted. A **performance measurement harness** with timed stress workloads (`ux/TECHNICAL.md` §7.6; `docs/test-corpus/perf/`) is scaffolding built as soon as the bridge supports it, since iterating on engine+host speed is part of the proving-ground mission, not a late add.

Engine prerequisites in OxCalc/OxFml/OxFunc (Spec `model/CORE_MODEL_SPEC.md` §6) gate several worksets; those are coordinated via handovers, not owned here. New since the seed map: version-based undo (§6 item 13), table-node unpacking (§6 item 14 — Tables are a cross-repo build area, in scope), and node-as-function invocation (§6 item 15).

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

### W002_engine_seam_and_bridge
- **Purpose:** The `OxCalcTreeBridge` and the multi-node recalc seam — submit structural snapshot + formula catalog, get back values/dependency-graph/calc-state. Reuse the OxFml editor bridge for per-node formulas.
- **Depends on:** W001. Engine prereqs in OxCalc (handover).
- **Spec sections:** `model/CORE_MODEL_SPEC.md`; `ux/TECHNICAL.md` §4.
- **Closure condition:** a workspace of named nodes evaluates end-to-end via the bridge; per-node values and calc-state are observable.
- **Initial epic lanes:** bridge contract; live-edit orchestration; calc-state plumbing.
- **Verification:** Local (bridge + recalc unit tests over a small multi-node fixture). Excel anchor deferred to W009. Scaffolding: bridge test harness + a minimal multi-node workspace fixture.
- **Status:** OPEN

### W003_tree_shell_and_core_editing
- **Purpose:** *Flesh out* the workspace shell and structural editing from the W005 skeleton — the full nav rail (virtualized rows, search/filter, drag-reorder, context menu, breadcrumb, meta-visibility toggle), the full structural-edit service (insert/rename/move/delete), the full three-pane TripleEditor experience, and complete `.dnatree` persistence (all meta-namespaces, undo grouping).
- **Depends on:** W005 (the walking skeleton — which already proves the minimal shell, skin mount, and persistence). Collaborates with W004 on rename/move propagation.
- **Spec sections:** `ux/REQUIREMENTS.md` §2–§3; `ux/TECHNICAL.md` §6; `ux/SKINS.md`; `ux/TRACEABILITY.md`; `ux/IMPLEMENTATION_MATRIX.md`.
- **Closure condition:** a user can build, edit, save, and reopen a substantial tree of named nodes in the three-pane skin, with full node CRUD and the nav-rail affordances.
- **Initial epic lanes:** nav-rail features; structural-edit service; full persistence + undo; TripleEditor depth. Activates UX trace slices `UX-SH`, `UX-TR`, `UX-FE`, `UX-VA`, `UX-ST`.
- **Verification:** Local + a UX click-through on the running shell + save/reopen round-trip. Scaffolding: reuse the skeleton's click-through + persistence harnesses.
- **Status:** OPEN

### W004_reference_model_and_resolution
- **Purpose:** *Flesh out* the reference surface from the skeleton's bare-name walk-up — `^`/`^^` ancestors, `[]`/`[ws]` anchors + cross-workspace, bracket-escaping, sibling-offsets, set-producing operators (`.*`/`@CHILDREN`/`@ANCESTORS`/`**`), `INDIRECT`/dynamic (CTRO) references, capability-profile gating, and rename/move propagation prompts.
- **Depends on:** W005 (the skeleton's bridge + walk-up base). Engine prereqs (SelfNode, set-membership deps, profile selectors, dynamic refs) via handover.
- **Spec sections:** `model/CORE_MODEL_SPEC.md` §3, §4, §6.
- **Closure condition:** references resolve per spec across the full surface; rename/move propagation prompt works; editor hover shows resolved bindings.
- **Initial epic lanes:** anchors + sibling-offsets; set-membership + recursive descent; cross-workspace + aliases; profile gating; INDIRECT/dynamic; propagation UX.
- **Verification:** Local — **progressively activate** the authored corpus themes (`references/*`, `profiles/`, `dynamic-references/`, `structural-edits/`) from `pending` to `active` against the bridge as each sub-surface lands; Excel anchor for name-scope behavior where it maps. Scaffolding: the test corpus (`docs/test-corpus/`, already authored) + the skeleton's corpus runner.
- **Status:** OPEN

### W005_walking_skeleton_min_end_to_end
- **Purpose:** The **walking skeleton** — the thinnest real slice that runs end-to-end through the skin types. Minimal skin framework (`WorkspaceSkin`, `SkinContext`, `Dispatcher` + a minimal closed `WorkspaceIntent`, registry + switcher, `SkinStateHandle`); a minimal shell (context strip, nav rail, one main mount slot, status foot); **two minimal skins of different categories** (`triple-editor` + `outline-table`) to prove runtime switching; minimal bare-name walk-up resolution wired through the W002 bridge; minimal `.dnatree` persistence; and the **first end-to-end test surfaces** (a corpus runner + a UX click-through).
- **Depends on:** W002 (the bridge — done). This is the pivot: it owns *thin* slices of the shell (W003), resolution (W004), and a second skin (W006); those worksets own the full depth.
- **Spec sections:** `ux/SKINS.md` §1–§2, §7; `ux/IMPLEMENTATION_MATRIX.md` (`UX-SK`, thin `UX-SH`/`UX-TR`/`UX-FE`/`UX-VA`; scenario cards S1–S4); `ux/TRACEABILITY.md` (F1, F8, F9); `model/CORE_MODEL_SPEC.md` §3.2 (bare-name walk-up only).
- **Closure condition:** the skeleton runs — a tiny workspace loads; editing a node formula updates its value through the **real bridge**; switching `triple-editor` ↔ `outline-table` preserves shared selection with **no recalc**; save/reopen round-trips. The **first slice of the test corpus is activated** (`references/walkup` cases the minimal resolution supports, flipped `pending→active` and green against the bridge via the corpus runner), and a **UX click-through passes**.
- **Initial epic lanes:** minimal skin framework + registry/switcher; minimal shell + nav rail; two minimal skins; bare-name walk-up over the bridge; `.dnatree` round-trip; **corpus runner v1** (activate-a-slice) + **click-through harness v1**.
- **Verification:** Local — the corpus runner over the activated walk-up slice + reducer/projection tests — plus a UX click-through (edit→value, switch-skin→no-recalc, save/reopen). This workset *first runs* the test-corpus **activation** model and the UX-matrix harness for real. Scaffolding: corpus runner v1; click-through harness v1; a tiny end-to-end workspace fixture.
- **Status:** OPEN

### W006_additional_skins_and_framework_hardening
- **Purpose:** The remaining v1 skins on the hardened framework, plus the hardening itself. **Minimum lovable skin set:** the skeleton already ships `triple-editor` + `outline-table`; **`cell-view` is the must-have completion** (Excel-fluent data entry) — those three are the v1 bar (CHARTER's "good enough to use"). **`canvas-flow` and `nodes-across` are nice-to-have enrichment** that may land after v1 proves out. Hardening, as the skins stress the framework: the full shared primitive library lift from OneCalc, full `SkinState` (schema migration + GC), shared-state depth, multi-slot composition, the `FormatResolver` seam, and the cross-skin equivalence harness.
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
