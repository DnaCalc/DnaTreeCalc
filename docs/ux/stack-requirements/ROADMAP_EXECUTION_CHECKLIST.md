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

Active wave: **W3 - Reference and content authoring verbs**.

Current objective: assess and land the first W3 authoring verb slice while preserving ownership:
OxFml composes or rewrites formula text, OxCalc rebinds and schedules, the DnaTreeCalc host carries
ids/handles/scopes through closed intents, and skins dispatch only.

Current status: first W3 assessment is complete. The formula rewrite verbs
(`replicate-by-id`, `f4-toggle-binding`, and `reference-insertion`) are blocked on an OxFml-owned
authoring API and are recorded in `../../handovers/HANDOVER_OXFML_formula_authoring_verbs.md`.
The first landed W3 slice is `format-write` for authored number formats via canonical meta nodes and
real OxCalc transactions. The second landed W3 slice is `note-write` via canonical `Note` meta nodes
and `NodeView.note` projection. The `SetMeta` half of `meta-and-attribute-write` is landed through
an OxCalc-owned revisioned meta-membership edit. The `SetNodeAttributes` half is landed for the
current Skin IR surface as a revisioned host-owned string attribute bag stored in canonical meta
nodes and projected through `NodeView.attributes`. The first `add-node-content-policy` widening is
landed for literal formula initial content: OxCalc dry-binds prospective new-node formulas without
mutation, and DnaTreeCalc previews/rejects invalid literal formulas before add-node commit.

Do not advance to W4 speculation/history or W5 platform polish as the default next step while W3
authoring verbs remain incomplete.

## Per-Iteration Gate

Before implementation, write the tranche in this form:

| Field | Required answer |
|---|---|
| Roadmap wave | The exact `ROADMAP.md` wave and requirement id. |
| User-visible capability | What a skin or host can now do that it could not do before. |
| Owning truth | Which layer owns the semantic fact or mutation: OxFml, OxCalc, DnaTreeCalc host, or skin. |
| Readiness result | `expose`, `extend`, or `new substrate`, corrected against live code. |
| Seam change | Projection field, closed intent, receipt shape, Skin IR shape, or handoff doc. |
| Evidence | Engine test plus programmable Skin IR or real-skin exercise, unless the tranche is a spike only. |
| Exclusions | Concrete unsupported cases and the next blocked or unblocked requirement. |

After implementation, close the tranche only when the evidence row is true and the owning-truth row
has not drifted. If the owning layer lacks the needed API, record a handoff and move only to the next
roadmap item that can be implemented without fabricating semantics host-side.

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
- [x] Remaining multi-target/table transaction ids are backed by OxCalc transaction operation
      coverage rather than host batching.
  - [x] Table snapshot operations with existing node ids route through OxCalc transaction outcomes:
        row delete/rename/reorder, formula-column add/edit/delete, totals/header
        visibility/formula edits, and column delete/rename/reorder.
  - [x] Generated-node table operations (`AddTableRow`, constant `AddTableColumn`) use OxCalc
        reserved node ids so one transaction can add generated cell nodes and publish the table
        snapshot that references them.
  - [x] Scoped existing-node content edits carry `AuthoringScope` through Skin IR and route through
        one OxCalc batch edit transaction after host-owned projection expansion.
  - [x] Other scoped multi-target authoring verbs remain parked with their owning W3 authoring verbs.
- [x] W2 closure review confirms no skin parses formulas, computes semantic values, or fabricates
      engine facts.

## W3 Execution Order

- [x] Assess W3 authoring verbs against live OxFml/OxCalc APIs:
      `replicate-by-id`, `f4-toggle-binding`, `reference-insertion`, `clipboard-transfer-model`,
      `paste-special`, `duplicate-subtree`, `set-membership-write`, `meta-and-attribute-write`,
      `note-write`, `format-write`, and `add-node-content-policy` widening.
- [x] File OxFml handoff for W3 formula rewrite/composition verbs that cannot be implemented
      ownership-correctly in DnaTreeCalc with the current editor facade.
- [x] Land first ownership-correct `format-write` tranche:
      `WorkspaceIntent::SetNumberFormat { scope, number_format_code }` creates, updates, or clears
      `Format.NumberFormat` meta nodes through OxCalc transactions; receipts carry transaction ids;
      Skin IR tests assert set, clear, multi-node scope, and reserved-path rejection.
- [x] Land `note-write`:
      `WorkspaceIntent::SetNote { node, note }` creates, updates, or clears a `Note` meta node
      through OxCalc transactions; `NodeView.note` and active-node detail project it; tests assert
      set, clear, document round-trip, and reserved-path rejection.
- [x] Land the `SetMeta` half of `meta-and-attribute-write`:
      OxCalc exposes `OxCalcTreeEdit::SetNodeMeta`, meta membership enters namespace/workspace
      revision identity, DnaTreeCalc exposes `WorkspaceIntent::SetMeta`, and tests assert
      transaction receipts, revision movement, projected `is_meta`, retained addressability, and
      formula invisibility.
- [x] Land the `SetNodeAttributes` half of `meta-and-attribute-write` for the current Skin IR
      surface:
      `WorkspaceIntent::SetNodeAttributes { node, attrs }` patches path-safe string attributes via
      canonical `Attributes.<key>` meta nodes, projects `NodeView.attributes` and active-node
      attributes, carries transaction receipts, and rejects invalid keys / reserved non-meta paths
      with typed errors.
- [x] Land first `add-node-content-policy` widening:
      `InitialNodeContentProjection::Literal { content }` formula text is dry-bound by OxCalc in a
      prospective new-node context without workspace mutation; add-node preview carries typed
      syntax/bind/profile blockers; add-node commit rejects invalid literal formulas before
      mutation. Empty, literal constants, and `is_meta` remain supported. `InheritColumnFormula`
      remains blocked until the intent carries a table/column subject; `TemplateBound` remains
      blocked on the template subsystem.
- [ ] Continue W3 with the next feasible tranche: complete `add-node-content-policy`
      subject/substrate design or OxFml-unblocked formula authoring.

## Gated Workstreams

- [x] `transaction-scope`: current W2 node, table snapshot, generated-node table add, and
      existing-node scoped content receipts carry real OxCalc transaction ids. Remaining scoped
      multi-target authoring verbs belong to W3 command expansion rather than W2 closure.
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
