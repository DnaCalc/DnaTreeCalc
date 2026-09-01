# W011 Fable run status

Single running status note for the Fable Ultracode W011 campaign
(`Foundation/notes/FABLE_ULTRACODE_WORK_INSTRUCTIONS_W011_2026-09-01.md`).
Updated only at campaign milestones (phase-0-done, ingest-green, edit-green,
save-green, or blocker), per the pack's coordination protocol.

## 2026-09-01 — Phase 0 complete (Campaign A); session handover to Fable 5.1

**Product status:** no product code changed yet. The bead graph is reconciled
with HEAD and the successor epic `dtc-j7n8` (18 children, label `w011-fable`)
is the sole execution state for the campaign.

**Evidence:** commits `e80444b` and `4ed3f15` on `main` (pushed to
`origin/main`); `br dep cycles` empty; `cargo test -p dnacalc-host-core
--offline` = 51 passed (baseline, also the close evidence for `dtc-hj2.3/.4`);
`br ready` at P0/P1 offers only `dtc-j7n8.1` plus the deliberately-untouched
`dtc-c0wf.33`.

**Still open:** all of Wave 1 (`dtc-j7n8.1` – `.9`), then Waves 1.5/2/3.

### What Phase 0 did

- Read the pack, `PROGRAM_INVESTIGATION_2026-08-30.md` §§2, 5–7, 15–16, 19,
  the W011 proof doc, TreeCalc `AGENTS.md`/`OPERATIONS.md`/`CHARTER.md`, and
  `HOUSEKEEPING_2026-08-31.md`.
- Installed the three granted skills: `planning-workflow`, `beads-bv`,
  `testing-conformance-harnesses` (already present: `beads-workflow`,
  `agent-fungibility-philosophy`).
- Verified seam ground truth in code with three read-only sweeps (OxDoc,
  OxCalc, DnaTreeCalc). Everything load-bearing is written INTO the beads —
  exact signatures, `LoadProfile::full()` requirement, the `book:{workspace_id}`
  address-token trap (three host-side construction sites; `grid_authored_view`
  returns blanks — never an error — on token mismatch), OxDoc's surgical-path
  rejections, calcChain drop policy, the Manual-mode stale-cache caveat.
- Reconciled the stale graph: closed `dtc-hj2.3`/`.4` (landed, evidence cited),
  `dtc-hj2.5` (superseded by OxCalc W062 R5/R6 native verbs), the four empty
  sample beads (`dtc-pg91`, `dtc-juyd`, `dtc-ga2e`, `dtc-5u1s`); parked
  `dtc-hj2.6`–`.10` behind `dtc-j7n8` with blocked-on-successor edges and
  PARKED notes. `dtc-hj2` itself stays OPEN (its closure needs browser UI,
  multi-skin layout, strict-profile lanes — parked children).
- Authored `dtc-j7n8.1`–`.18` and polished four rounds: self-review, the
  agent-ergonomic rumination (single `workbook_token` authority; three-truths
  doc comment), an adversarial fresh-context review (15 findings applied —
  wasm blast radius of `oxdoc-xlsx`, corrected fixture-template facts, the
  silent GridRect token failure, the `dnacalc-app` visibility compile blocker,
  GridChanged split out as `.18`, fmt/clippy floor on every code bead), and
  final validation.

### Orchestration facts for the successor session

- **Wave 1 is a serial relay, not a swarm**: `.1 → .2 → .3 → .4 → .5 → .6 →
  .7 → .8` over three single-writer files (`command.rs`, `lib.rs`,
  `workbook.rs`). Run ONE implementer until `.7` closes; fan out only after.
- **Fable personally reviews the `dtc-j7n8.7` diff before close** (the
  cached-B1=30 save trap). Its failure playbook is pre-registered in the bead.
- Implementers claim ONLY beads labeled `w011-fable`; the subagent init prompt
  is pack §9.1 verbatim. `dtc-c0wf.33`, `dtc-yo3`, Bench/frontend beads stay
  untouched.
- Beads are self-contained by design: implement from `br show <id>`, not from
  the markdown pack. The pack remains binding for guardrails (§0, §11).
- Commit per closed bead (`br sync --flush-only`; `git add` sources +
  `.beads/issues.jsonl`); push `main` when an unblocking bead closes. No
  force-push, no amending pushed commits, never bare `bv`.

### Session archaeology (only if something seems missing)

Fable 5 session conversation log:
`C:\Users\GovertvanDrimmelen\.claude\projects\C--Work-DnaCalc\5a57b5cb-b665-438e-8e5a-97016334b694.jsonl`
(~1.5 MB JSONL). The durable state is the bead graph + git, not the chat;
consult the log only for provenance questions the beads and close reasons
cannot answer.

## 2026-09-01 — Ingest-green: `dtc-j7n8.1`–`.4` landed; save-seam blocker filed (Fable 5.1 session)

**Product status:** a DnaTreeCalc host opens the committed W011 fixture
(`fixtures/w011/a1_times_three.xlsx`; readable parts under
`fixtures/w011/a1_times_three/parts/`) through OxDoc with `LoadProfile::full()`,
owns the `HostOwnedXlsxSource` next to the live `OxCalcDocumentContext`,
ingests it via `load_workbook_model`, and reports A1 = 7 and B1 `=A1*3`
published 21 (Calculated, Automatic). `enter_grid_cell` on the loaded fixture
recalculates B1 to 30. Grid address tokens route through one host-side
authority (`book:{workspace}` for ingested grids, bare id for seeded grids),
pinned against the engine by a two-origin test. `HostCommand::OpenXlsxBytes`
exists with typed OxDoc errors and a fallible `DocumentSession::execute`.
Not yet: Skin IR projection verification (.5), the dispatch-level edit proof
(.6), save (.7), app wiring (.8).

**Evidence:** commits `5b8e73b` (.1 deps; F-gate widened to oxdoc), `bd82a59`
(.2 fixture + OxDoc open acceptance test), `63a5af4` (workspace rustfmt,
`dtc-xwa3`), `e7018e5` (.3 OpenXlsxBytes), `64b92fe` (.4 ingest) — all on
`origin/main`. Each bead was closed with its test names and independently
verified by two refuters (acceptance/tests, ownership/guardrails) before push.
`cargo test -p dnacalc-host-core --offline` = 61 passed (baseline 51);
`cargo test -p dnacalc-arch-gates --offline` green incl. the new
`inverted_control_host_core_contains_oxdoc`; wasm32 `cargo check` of
`dnacalc-app` and `dnatreecalc-web` green before and after the oxdoc edge (no
cfg-gate needed). Sibling repos untouched (OxDoc `786ef0c`, OxCalc `752a269d`
at the time of these commits).

**Blocker (typed, pre-registered before `.7` starts):** an out-of-repo probe by
the orchestrator showed `oxdoc_xlsx::write_save_request` rejecting OxCalc's
`project_workbook_model_output` stream of this exact fixture — with and
without the edit — as
`UnsupportedRoundTripFeature("changing differential style metadata during round-trip is not supported yet (DifferentialStyles Workbook)")`.
Root cause: a full-profile OxDoc load always emits an (empty)
`DifferentialStyleTable` and marks the surface Materialized; OxCalc ingests it
but re-emits it only when non-empty. With that one event replayed the same
probe saves cleanly (no `Dropped` ledger entries) and reopens with B1 formula
text preserved and **cached 30** — so the cached-value trap itself does not
bite. Fix owned by OxCalc bead `calc-5kqg.70` (presence-aware store, replay
when present); DnaTreeCalc blocker `dtc-rpdy` blocks `dtc-j7n8.7`; handover
`HANDOVER_OXCALC_w011_projection_replays_empty_dxf_table.md`. The OxCalc fix
runs serialized ahead of `.5`–`.7` because OxCalc is a path dependency the
relay compiles.

**Still open:** `.5` projection verification, `.6` edit proof, `.7` save
(blocked on `dtc-rpdy`), `.8` app wiring, `.9` charter footnote; then Waves
1.5–3. Housekeeping filed in passing: `dtc-g43s` (pre-existing
`walking_skeleton` flake under multi-package `cargo test`; test
`dnatreecalc-host` solo), `dtc-lxz9` closed as duplicate of `dtc-xwa3`.

## 2026-09-01 — Save-green: `dtc-j7n8.5`–`.7` landed; the W011 outcome is evidenced at host-core level

**Product status:** the W011 outcome holds end to end in `dnacalc-host-core`
on the committed fixture: open through OxDoc → ingest → the Skin IR projection
carries authored metadata and provenance (A1 Literal 7, B1 Formula `=A1*3` =
21, Calculated) → `WorkspaceIntent::EnterGridCell` A1 "10" through the real
dispatch path publishes B1 = 30 with a `GridCellEntered` receipt →
`HostCommand::SaveActiveXlsx` returns the package bytes plus OxDoc's ledger →
the **reopened saved bytes carry B1 formula text `A1*3` with cached 30** (raw
OxDoc events, no engine). Refusals are typed end to end (no backing source,
RichTree session, out-of-policy cell add). Not yet: app-level wiring (`.8`),
the charter footnote (`.9`), GridChanged deltas (`.18`), Waves 1.5–3.

**Evidence:** commits `c813272` (.5 projection verification), `9e3a9fe` (.6
edit proof, incl. engine-accepted formula text and the Recalculate no-op),
`0e7d60a` (.7 save), all on `main`. `cargo test -p dnacalc-host-core
--offline` = 74 passed, 0 failed, 2 ignored (generators). The .7 diff was
reviewed personally by the orchestrator, re-run, and attacked by three
adversarial refuters (a mutation probe flipping the expected cache to 21 made
both cached-30 tests fail loudly; files restored byte-exact). OxDoc's verbatim
cell-add rejection is pinned in the tests. Upstream: OxCalc `60c6af72`
(calc-5kqg.70) — the present-but-empty DifferentialStyleTable now replays, so
the pre-registered rejection never fired.

**Still open:** `.8` app wiring, `.9` charter footnote, `.18` GridChanged
(after .8), Wave 1.5 `.10`, Wave 2 `.11`/`.12` (Excel compare of this fixture;
the artifact `target/w011/a1_times_three_saved.xlsx` is generated by the
ignored test), Wave 3 `.13`–`.15`. Follow-up filed: `dtc-o4t1` (ledger loss
assertion hardening). Dirty tracking and source rebasing after save remain
future beads, not W011 exit criteria.
