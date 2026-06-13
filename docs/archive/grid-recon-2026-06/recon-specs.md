# Spec corpus sweep — DnaTreeCalc docs (grid/sheet/Excel-extent)

Note: `SPEC.md`/`SCOPE.md` live at `docs/SPEC.md` and `docs/SCOPE.md`, not repo root.

## 1. What is already specified/promised about grid/sheet/Excel-extent

**The grid is an explicit, structural non-goal for TreeCalc — but a planned successor host.**
- `CHARTER.md:44` — "**Not a grid.** No coordinates; no value spilling between nodes. The grid arrives with PreCalc and beyond."
- `CHARTER.md:16-28` — host progression `VbCalc → OneCalc → TreeCalc → PreCalc → SuperCalc → DNA Calc`; PreCalc/SuperCalc "introduce and refine the tree-grid hybrid"; TreeCalc's role is to "stress-test OxCalc's coordinator, dependency graph, invalidation, and epoch model … proving them before the grid hosts depend on them" (`CHARTER.md:28`, success criterion 3 at `CHARTER.md:77`).
- `docs/SCOPE.md:39` — hard non-goal: "**No grid** — no coordinates, no `A1:B5` ranges, no inter-node spilling. The grid arrives with PreCalc and beyond."

**Deliberate forward hooks for the grid host already in the spec:**
- `docs/model/CORE_MODEL_SPEC.md:62` — the `!` separator allowance "preserves Excel's `Sheet!Name` and `Sheet!Cell` conventions for … future DNA Calc grid-mode use".
- `CORE_MODEL_SPEC.md:293` — `"host-capabilities:strict-excel"` profile is the "Default for grid hosts (DNA Calc and successors)"; every TreeCalc extension is parse/bind-rejected under it (`§4`).
- `CORE_MODEL_SPEC.md:306,383` — profile-aware `INDIRECT`: string parsed as tree path under treecalc-v1, "as A1/R1C1 under `strict-excel`".
- `CORE_MODEL_SPEC.md:371-377` — the reference-abstraction design area: "Excel grid references/ranges and TreeCalc opaque reference arrays should pass through OxFml/OxFunc/OxCalc as uniformly as the model can honestly support"; engine prerequisite item 1 requires tree refs to **coexist with Excel grid references/ranges** behind one `ReferenceKind` abstraction (W051 first cut).
- `docs/interop/EXCEL_EXPORT_AND_REPLAY.md:191` — the `WorkbookConstructionSpec` "is general (Excel terms only), so the DnaCalc grid host and any future host can reuse it"; grid-cell **promotion** exists on export (`:206-213`), and the host owns "what gets grid-promoted" (`:102`).
- Tables are the sanctioned grid-adjacent surface: `CORE_MODEL_SPEC.md §7c` (`:465-469`) — engine **unpacks** table nodes (W056 largely landed); `docs/ux/stack-requirements/ENGINE_REQUIREMENTS.md:137-142` (`table-structural-ops` — "still **no** grid coords"); `docs/ux/stack-requirements/ROADMAP.md:219-221` open Q9 "Table without grid coords"; answered at `docs/handovers/HANDOVER_OXCALC_engine_readiness_and_skin_w0.md:40` — "OxCalc has structured table snapshots and typed dependency facts over rows/columns/regions while TreeCalc remains non-grid."
- Sheet lens: `docs/ux/skin-suite/lenses/SHEET.md:1-3` — "The Excel edit loop … **no A1, no coords**"; ATLAS tenet 3 forbids skins faking A1 (`docs/ux/skin-suite/README.md:90`).
- Import excludes grid usage entirely (`CORE_MODEL_SPEC.md:531, 611-617` — A1 refs, ranges, Tables, grid-position functions out of scope).
- `CORE_MODEL_SPEC.md:48` — users may "simulate grids by naming children 'A1'/'A2'… the resemblance is cosmetic."

**Nothing anywhere mentions the full Excel extent (1,048,576×16,384), sparse block occupancy, R1C1-identical regions, virtual cells, or viewport-driven recalc prioritization.** A grid spec is net-new; the corpus only promises the seams above.

## 2. Performance / acceptance criteria

- **Phase B engine perf gate** (`docs/ux/skin-suite/PHASE_B.md:46-59`, B.2.0): the passivity spike found "engine recalc cost — not threading — is the scale blocker (quadratic+ cold, warm no-op runs 10–80× slower than cold)" (`:8-15`). Acceptance: "chain n=5k cold ≤ ~1 s release; warm strictly cheaper than cold; incremental cost proportional to the dirty set, not N", re-running `OxCalc tests/host_worker_passivity_spike.rs`. Named targets: `EdgeValueCacheLookup` (88.7 s @ n=200 warm), `DiagnosticSeedCollection` (29.5 s @ n=200), per-run `OxfmlPrepareFormulas`, consumer clone overhead (23 s @ n=100).
- **B.2 exit criteria** (`PHASE_B.md:95-98`): 5k-node model, edit → `Pending` immediately, published values without main-thread jank, delta-only boundary traffic, frame-budget telemetry.
- **W5 target** (`ROADMAP.md:149`): "60fps on 100k-node models with a measurable frame budget"; `frame-telemetry-hooks` "makes the 60fps goal falsifiable" (`stack-requirements/HOST_AND_SKIN_IR_REQUIREMENTS.md:85`).
- **Host policy** (`docs/ux/TECHNICAL.md:550-558` §7.6): "**No clock-time success gates now**" — named stress workloads (`docs/test-corpus/perf/`: deep tree, wide tree, large-array, edit storm, RTD churn), timed runs through the real stack, **Excel comparison includes timing**; "Engine-internal targets remain OxCalc's; TreeCalc owns the workloads and the measurement surface."
- Budgets: ~1 KB/node, 100K nodes ≈ 100 MB (`TECHNICAL.md:540-542`); arrays >~1M cells need OxCalc range queries (`:530`); size tiers up to ">1M nodes … stricter recalc batching, optional read-only" (`docs/ux/REQUIREMENTS.md:497-500`).
- Spike datum: 1k nodes ≈ 4 min release cold (`ROADMAP.md:202-210`, open Q5).

## 3. Spec formalization style (to match)

`CORE_MODEL_SPEC.md` pattern: §1 purpose + **layering/ownership table** (`:9-14`); numbered sections with letter-suffixed insertions (§7a/§7b/§7c, §8a); example blocks; **surface→engine-variant mapping table** (§3.7 `:222-238`); capability-profile gating section (§4); Excel-alignment two-column "novel vs Excel-aligned" table (§5 `:329-338`); **ownership boundary table** of what the spec does NOT specify (§5.1 `:342-357`); numbered **engine prerequisites list** (§6 items 1–15) each cross-linked to a handover; "What's settled vs. open" paragraphs (§7c `:469`); illustrative compact grammar with "canonical grammar lives in OxFml" disclaimer (§9 `:496-527`). Conventions: mandate behavior not representation (`docs/model/META_NODES.md:61`); "what does Excel do?" as the default answer (`CORE_MODEL_SPEC.md:340`); engine dependencies stated as content/handovers, never status markers (`docs/SPEC.md:79`); living docs edited in place. Verification style: declarative JSON corpus pinning only the novel surface, **pending→active activation** when a real engine runner exercises a case (`docs/SPEC.md:65`, `WORKSET_REGISTER.md:88,98`); retained replay artifacts under `docs/test-runs/<workset-slug>-NNN/`. Stack-requirements style: kebab-case requirement ids with readiness tags `expose`/`extend`/`new`, effort S/M/L/XL, "Shape (illustrative Rust) / Unlocks / Note-risk" blocks (`ENGINE_REQUIREMENTS.md:8-11`). Formal-methods direction (Lean/TLA+/conformance corpus) is chartered as a growth direction (`CHARTER.md:30`) — precedent for a simple-correct-vs-optimized pair is the cycle profiles (`excel_match_iterative` vs `iterative_deterministic_v0`, §7a).

## 4. Documented responsibility split (OxCalc vs host vs skins)

- **OxCalc owns:** tree-model custody, coordinator, dependency graph, invalidation closure, atomic publication, epochs, value caching, cycle handling, recalc model — host must "lean on; never reconstruct host-side" (`CORE_MODEL_SPEC.md:9-14, 342-357`). Engine is **synchronous and passive; the host pumps every tick** ("sans-executor", `TECHNICAL.md:281-287`; `ROADMAP.md:37-39`; concurrency = host worker, B.2.2).
- **Host owns:** persistence, structural-edit orchestration, intents/dispatcher, meta-nodes/format/templates, selection — values/calc-state are "projected from the engine result … never authored independently" (`TECHNICAL.md:203`).
- **Skins own layout/viewport:** the six-lane ownership table at `docs/ux/TRACEABILITY.md:19-26` — per-skin state (canvas positions/zoom, column widths, array viewport/scroll) lives in `skins.<skin_id>` meta; "Skins never call OxCalc directly" (`:28`). **Resize/viewport never recalcs:** flow F10 (`TRACEABILITY.md:191-198`) — "No OxCalc call occurs for resize. If resize exposes a new array/canvas/table viewport range, the host may ask for more **already-published value slices**; this is data access, not recalculation." Large-array visible-range queries are "host/OxCalc-mediated" (F4.7 `:139`; `TECHNICAL.md:530`). "Viewport zoom is skin-local" (`skin-suite/README.md:56`). ⇒ **Viewport-driven recalc prioritization is currently unspecified and mildly counter-doctrinal** — today's doctrine is viewport = pure view state; the nearest hook is `virtualization-window-projection` (W5; deferred "only meaningful after B.2.0", `PHASE_B.md:140-141`).
- Interop split: "TreeCalc converts; OxXlPlay builds+observes; OxReplay compares+governs" (`EXCEL_EXPORT_AND_REPLAY.md:102`, `docs/SPEC.md:45`).

## 5. Open prerequisites intersecting grid work

- **Engine perf workstream `calc-ekq3`** (B.2.0) — quadratic+ recalc; gates everything at scale (`PHASE_B.md:46-59`).
- **CORE_MODEL §6 items still open:** #1 unified `ReferenceKind` carrying Excel grid ranges + tree collections (W051); #3 set-membership dependency edges; #7 profile-aware INDIRECT (A1/R1C1); #8 transactional batch editing (= `transaction-scope`, per-node publish today, `ENGINE_REQUIREMENTS.md:20-29`); #12 cycle profiles (W048/W055); #13 version-based undo (= `revision-graph-retention`); #14 table unpacking (W056 — bracket-escaped table paths remain an "OxFml packet blocker", `CORE_MODEL_SPEC.md:469`).
- **Gating substrates** with first slices landed: `candidate-overlay-handle` (copy-at-open layering; `HANDOVER_OXCALC_engine_readiness_and_skin_w0.md:87-160`), `value-epoch-keying` (no per-node published-value epoch yet, Q2), delta-only/resync mode (Q8: "At 100k nodes skins will want delta-*only*"), per-edge cache evidence (Q6), table-without-grid-coords (Q9).
- **OxFml:** dry-bind preview, conditional formatting semantics (`HANDOVER_OXFML_conditional_formatting.md`; CF authoring deferred to research W7, `ROADMAP.md:169-178`), formula authoring verbs/paste-special (rebind machinery a grid fill/region story will need).

## 6. Naming/ID conventions for new docs

- **Spec docs:** `UPPER_SNAKE.md` under `docs/model/` | `docs/interop/` | `docs/ux/` (+ suite subfolders), registered in the `docs/SPEC.md` index tables; living, edited in place, no status layering (`docs/SPEC.md:5, 77-79`).
- **Handovers:** `docs/handovers/HANDOVER_<TARGET>_<short_topic>.md`, header `Status/Target/Ask/Context/Evidence`, one-word status, response appended to the same file, topic-specific not monolithic (`docs/handovers/README.md:1-20`).
- **Worksets:** `W###_short_name` sequential in `docs/WORKSET_REGISTER.md` with paired epic beads (`dtc-*` host, `calc-*` OxCalc); OxCalc-side worksets are W0xx (W048/W051/W054/W055/W056) with specs at `OxCalc/docs/spec/core-engine/CORE_ENGINE_<TOPIC>.md` (+ `w048-cycles/` style subfolders).
- **Requirement ids:** backticked kebab-case (`virtualization-window-projection`) with readiness/effort/band tags; UX trace IDs `UX-XX-###` + scenario cards `S#` (`IMPLEMENTATION_MATRIX.md`); corpus cases `docs/test-corpus/<theme>/*.json` with kebab ids; retained runs `docs/test-runs/<topic>-NNN/`.