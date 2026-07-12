# DnaOneCalc Beads Working Method

## 1. Purpose
This file is the complete local beads story for `DnaOneCalc`.

It covers:
1. local execution method,
2. `br` and `bv` usage,
3. the mutation rule,
4. bead quality expectations,
5. the workset -> epic -> bead rollout pattern,
6. a compact rollout template,
7. a compact worked example.

## 2. Core Model
Execution in this repo runs through:
1. `docs/WORKSET_REGISTER.md`
2. `workset -> epic -> bead`
3. `.beads/` as the detailed execution truth

Worksets are the high-level planning and scope-partition unit.
Epics are the main execution lanes under a chosen workset lineage.
Beads are the unit of executable progress.

`docs/WORKSET_REGISTER.md` and `.beads/` are the planning surfaces for
implementation work.
They are not a prompt to roll out one local document per work item.

Interpretation rule:
1. worksets do not carry ready/in-progress/blocked/closed execution status in the
   register,
2. `.beads/` is the sole owner of execution-state truth,
3. a workset is practically incomplete while any epic or bead rolled from it remains
   open.

## 3. Tool Split
`br` is the mutation tool.

Use it to:
1. inspect ready work,
2. create beads,
3. update bead status,
4. add dependencies,
5. close completed beads.

Typical commands:

```powershell
br ready
br show <id>
br create --title "..." --type task --priority 2
br update <id> --status in_progress
br close <id> --reason "Completed"
br dep add <issue> <depends-on>
```

`bv` is the graph-aware triage and analysis tool.

Use it to:
1. inspect the ready path,
2. identify blockers to clear,
3. inspect graph shape and priority pressure.

Agent rule:
1. use only non-interactive robot-style inspection calls from agent sessions,
2. for `bv` and read-only `br` inspection commands, prefer machine-readable or robot
   output modes where available,
3. do not launch blocking interactive views from unattended sessions.

## 4. Mutation Rule
Do not edit `.beads/` files directly.

Examples:

```powershell
br create --title "Roll out WS-05 child beads" --type task --priority 2
br update dno-1 --status in_progress
br close dno-1 --reason "Completed"
```

## 5. Bead Quality Bar
Every executable bead should state:
1. one reviewable implementation outcome,
2. completion evidence that proves the claimed behavior,
3. its parent epic,
4. any real dependency relationship,
5. canonical evidence or truth surfaces touched where that matters.

Bad beads:
1. vague activity,
2. ongoing theme,
3. mini-worksets disguised as one issue,
4. hidden follow-up work left only in chat or commit messages,
5. local-document-only output unless the bead is making a narrow spec correction,
   upstream seam handoff, or reference note for behavior that now exists in code.

## 6. Rollout Rule
Any workset chosen for execution should be rolled out into one or more epics.

Each epic should normally begin with a rollout bead when the child path still needs to be created or refreshed.

Rollout pattern rule:
1. some epics should be expanded into child beads immediately during initial rollout,
2. some epics should begin with a rollout bead whose job is to create or refresh the
   next child beads once enough context exists,
3. early or well-understood implementation work should default to direct child beads,
4. both patterns are normal as long as the bead graph stays explicit and reviewable.

A rollout bead is complete only when:
1. the epic has a believable ready path,
2. the next child beads exist explicitly,
3. the work no longer depends on narrative memory alone.

## 7. Closure Rule
A bead closes only when:
1. the stated outcome exists,
2. the stated evidence exists,
3. any newly discovered required work has already been added back into the bead graph.

Do not close a bead because “enough progress” happened.

Capability-bearing beads must normally close on meaningful implementation code plus
verification.
Stub commands, placeholder artifacts, and descriptive notes are not sufficient
closure evidence.

## 8. Documentation Rule
After planning is in place, default bead outputs should be:
1. implementation code,
2. test code,
3. narrowly-scoped spec corrections,
4. narrowly-scoped upstream seam handoffs,
5. necessary reference notes for behavior that now exists in code.

If a proposed bead's main output is a local note, floor, or baseline document,
rewrite it as implementation work or do not create it.

## 9. Compact Rollout Template
When a workset is chosen for rollout, capture this:

1. Workset:
   - id
   - title
   - scope
   - terminal condition
2. Execution epics:
   - rollout epic
   - first implementation lane
   - second implementation or integration lane
   - validation or evidence lane
   - cleanup or upstream-pressure lane where needed
3. First rollout bead per epic:
   - title
   - one reviewable outcome
   - completion evidence
4. First execution child beads:
   - one clear outcome each
   - explicit dependencies
   - explicit evidence

## 10. Compact Example
Example workset:
- `WS-02 Formula Editing And Language-Service Integration`

Example epic set:
1. editor shell and interaction lane
2. OxFml edit-packet integration lane
3. diagnostics projection lane
4. verification lane

Example first child beads:
1. implement formula buffer, cursor, and keyboard flow in the real shell
2. wire `FormulaEditRequest` and `FormulaEditResult` through the editor state
3. project `LiveDiagnosticSnapshot` into visible diagnostics and spans
4. add a runnable editor integration proof or test

## 11. Validator
Use:
- `scripts/check-worksets.ps1`

This is a minimal shape checker for the register, not an execution-status validator.

It should at least confirm:
1. the workset register exists,
2. workset ids are unique,
3. the register exposes a coherent workset sequence suitable for rollout into the bead
   graph.
