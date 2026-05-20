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
| W005_skin_architecture_and_primitives | `dtc-osq` |
| W006_additional_skins | `dtc-366` |
| W007_meta_nodes_formatting_templates | `dtc-fks` |
| W008_excel_import | `dtc-xlx` |
| W009_excel_export_and_replay | `dtc-p5q` |
| W010_udf_hosting | `dtc-dht` |

---

## Default sequence

```
W001 → W002 → W005 → W003 → W004 → W006 → W007 → W008 → W009 → W010
(bootstrap)   (engine  (skin   (shell)  (refs)  (more   (meta/  (excel  (excel  (UDF)
              bridge)  scaffold)                 skins)  fmt/    import) export
                                                        templ)          + replay)
```

Engine prerequisites in OxCalc/OxFml/OxFunc (Spec `model/CORE_MODEL_SPEC.md` §6) gate several worksets; those are coordinated via handovers, not owned here.

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
- **Purpose:** The workspace shell and core structural editing — nav-rail tree outline, node CRUD (insert/rename/move/delete), the three-pane editor skin, persistence to `.dnatree`.
- **Depends on:** W002, W005 skin scaffold. Worksets need not be executed strictly by number; TreeCalc's first usable shell is built through the skin interface from the start, not retrofitted later.
- **Spec sections:** `ux/REQUIREMENTS.md`; `ux/TECHNICAL.md` §6; `ux/SKINS.md` (triple-editor).
- **Closure condition:** a user can build, edit, save, and reopen a tree of named nodes in the three-pane skin.
- **Initial epic lanes:** nav rail + tree rows; structural edit service; persistence; registered TripleEditor skin.
- **Verification:** Local + a UX click-through on the running shell; save/reopen round-trip test. Scaffolding: skin click-through harness; persistence round-trip fixture.
- **Status:** OPEN

### W004_reference_model_and_resolution
- **Purpose:** The reference surface — bare-name walk-up scope, `^` ancestors, `[]`/`[ws]` anchors, bracket-escape, set-producing operators, rename/move propagation prompts.
- **Depends on:** W002, W003. Engine prereqs (SelfNode, set-membership deps) via handover.
- **Spec sections:** `model/CORE_MODEL_SPEC.md` §3, §6.
- **Closure condition:** references resolve per spec; rename/move propagation prompt works; editor hover shows resolved bindings.
- **Initial epic lanes:** bind integration; walk-up resolution; propagation UX.
- **Verification:** Local case-corpus of resolution cases authored in Spec §3 (bare-name walk-up, `^`/`^^`, `[]`/`[ws]`, profile rejections — each an input→expected-resolution row); Excel anchor for the Excel-aligned name-scope behavior where it maps. Scaffolding: reference-resolution fixture corpus driven from the spec cases.
- **Status:** OPEN

### W005_skin_architecture_and_primitives
- **Purpose:** The skin framework — `WorkspaceSkin` trait, `SkinContext`, intent dispatch, shared primitive library, per-skin meta-namespaces, skin switcher.
- **Depends on:** W002. This is foundational UI infrastructure; W003 consumes it for the first shell.
- **Spec sections:** `ux/SKINS.md`.
- **Closure condition:** the skin registry/composition layer mounts TripleEditor through the skin interface, and a second minimal skin or test skin proves per-skin state persisted in the `skins/*` meta-node subtree; shared tree-state is honored across mounted skins.
- **Initial epic lanes:** skin trait + registry; primitive library lift from OneCalc; meta-namespace persistence.
- **Verification:** Local + cross-skin equivalence (the same workspace renders/edits correctly across skins) + per-skin meta-state round-trip. Scaffolding: cross-skin equivalence harness; skin-state persistence tests.
- **Status:** OPEN

### W006_additional_skins
- **Purpose:** The remaining v1 skins — cell-view, outline-table, nodes-across, canvas-flow — on the primitive library.
- **Depends on:** W003, W005.
- **Spec sections:** `ux/SKINS.md` §6; `ux/prototypes/`.
- **Closure condition:** the prototyped skins render and edit the same workspace; canvas group→template affordance works.
- **Initial epic lanes:** cell-view; outline-table; nodes-across; canvas-flow.
- **Verification:** Local + per-skin click-through; reuse the W005 cross-skin equivalence harness across the new skins.
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
