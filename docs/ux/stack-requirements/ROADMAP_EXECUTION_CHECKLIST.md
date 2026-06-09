# Stack Roadmap Execution Checklist

This checklist keeps iteration work aligned with `ROADMAP.md`. It is not a second roadmap; it is the
short operational cursor used to decide what to do next, what repo owns it, and what proof is needed
before moving on.

## Iteration Contract

Every tranche should answer these before it is committed:

- [ ] Which `ROADMAP.md` wave and requirement did this advance?
- [ ] Was the readiness tag checked against live code (`expose`, `extend`, or `new`)?
- [ ] Is the implementation in the owning layer?
- [ ] Does the host only project, dispatch, or join typed facts rather than reinterpret semantics?
- [ ] Is there a programmable Skin IR test or real-skin exercise from outside the engine?
- [ ] Were changed specs, handovers, or checklist notes updated?
- [ ] Did the final status name product scope, evidence, still-open gaps, and next roadmap item?

## Active Cursor

Active wave: **W2 - Subjects, transactions, typed errors, and safe structural authoring**.

Current objective: finish the remaining W2 transaction and closure work by routing structural
authoring receipts through real OxCalc transaction outcomes wherever the engine can own the batch,
and by naming the cases that still need new OxCalc substrate rather than host-fabricated transaction
semantics.

Do not advance to W3 authoring verbs, W4 speculation/history, or W5 platform polish as the default
next step while these W2 items remain open.

## Iteration-To-Roadmap Checklist

Use this as the per-tranche goal statement before implementation:

- [ ] `Roadmap item`: name the exact `ROADMAP.md` requirement and wave.
- [ ] `Readiness`: verify live code confirms `expose`, `extend`, or `new`; correct the roadmap note
      if reality differs.
- [ ] `Owning repo`: implement in OxFml, OxCalc, DnaTreeCalc host, or skin layer according to the
      ownership boundary.
- [ ] `Seam`: thread the result through projection, intent receipt, or Skin IR without skin-side
      semantic reconstruction.
- [ ] `Evidence`: add or update programmable Skin IR tests or real-skin checks that exercise the
      capability from outside the engine.
- [ ] `Scope`: record exactly what now works, what remains open, and which gated workstream blocks
      the next dependent feature.
- [ ] `Commit`: commit affected repos at the end of the tranche so the next iteration starts from a
      clean boundary.

## W0 / W1 Baseline Already Available

- [x] Stable `NodeKey` transition spine carried through projection.
- [x] Typed invalidation reasons, dependency kinds, run state, calc state, phase timings, and richer
      value projection.
- [x] Reference-resolution map plus reverse lookup.
- [x] Binding diagnostics, effective formatting, runtime effects, overlay detail, derivation trace
      payloads, and active-node detail.
- [x] Per-node published-value epochs.

## W2 Execution Order

- [x] Typed intent errors replace stringly structural rejection receipts for the implemented
      host/session paths.
- [x] `AuthoringScope` models node, ordered multi-node, subtree, and reference-collection subjects
      with host projection expansion.
- [x] Edit transaction ids flow through node structural receipts for current node-level operations.
- [x] OxCalc `transaction-scope` spike has a go decision and first node-edit slice.
- [x] OxCalc committed-graph recalc-plan preview supports node-level preview mutations.
- [x] Node formula edit preview joins OxFml dry-bind with OxCalc invalidation planning.
- [x] Table body/totals formula previews dry-bind through OxCalc table formula context.
- [x] New table formula-column preflight dry-binds and plans table snapshot invalidation.
- [x] Scoped content edit preview expands `AuthoringScope`, dry-binds each target, and plans combined
      invalidation.
- [x] Rename preview joins collision legality with OxCalc structural invalidation planning.
- [x] Move/drop preview joins drop validity, collision legality, and OxCalc structural invalidation
      planning.
- [x] Delete/orphan structural preview reports outside dependents and invalidation impact without
      mutation.
- [x] Add/default-content policy preview is typed and tested for empty/literal content, meta-node
      flagging, name-collision blocking, and typed unsupported blockers for inherited/template
      policies.
- [x] Broader table row/column structural preview breadth is typed and tested for add, delete,
      rename, and reorder operations with table-collision / duplicate-input blockers and OxCalc
      table-snapshot invalidation planning.
- [ ] Remaining multi-target/table transaction ids are backed by OxCalc transaction operation
      coverage rather than host batching.
  - [x] Table snapshot operations with existing node ids route through OxCalc transaction outcomes:
        row delete/rename/reorder, formula-column add/edit/delete, totals/header
        visibility/formula edits, and column delete/rename/reorder.
  - [ ] Generated-node table operations (`AddTableRow`, constant `AddTableColumn`) need an OxCalc
        transaction placeholder or result-dependent edit substrate before their receipts can carry
        real transaction ids.
  - [x] Scoped existing-node content edits carry `AuthoringScope` through Skin IR and route through
        one OxCalc batch edit transaction after host-owned projection expansion.
  - [ ] Other scoped multi-target authoring verbs still need broader OxCalc operation coverage.
- [ ] W2 closure review confirms no skin parses formulas, computes semantic values, or fabricates
      engine facts.

## Gated Workstreams

- [ ] `transaction-scope`: broaden from first node-edit and table snapshot receipt coverage to
      generated-node table operations and remaining scoped multi-target operation families with
      accumulate-publish-once semantics. Existing-node scoped content edit is covered.
- [ ] `revision-graph-retention`: implement retained parent-linked revision DAG and cursor before
      undo, redo, time travel, or history UI claims.
- [ ] `candidate-overlay-handle`: implement N addressable, layerable, non-publishing candidate
      contexts before scenarios, what-if previews, goal seek, sweeps, or comparative overlays.
- [x] `value-epoch-keying`: per-node published-value epoch is available for projection consumers.

## Next-Wave Parking Lot

Only pull these forward when their prerequisites above are met:

- W3: reference/content authoring verbs (`replicate-by-id`, F4 binding toggle, point-mode
  insertion, paste special, duplicate subtree, set membership, notes, formats).
- W4a/W4b/W4c: revision navigation, candidate overlays, speculation, scenarios, and comparative
  projections.
- W5 early subset: projection delta channel, version stamp, persistence, design tokens, a11y.
- W5+ later platform: worker calc, multi-slot composition, keybinding registry, virtualization,
  capability negotiation, error isolation, telemetry.
- W6: templates, table structural authoring, import/export, external feeds, sensitivity/goal seek,
  onboarding.
