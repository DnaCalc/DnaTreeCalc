# Twenty Mechanisms — the behaviours that make DNA Calc effortless

Status: v0.1 · 2026-07-12 · companion to [REDESIGN_PROGRAM.md](REDESIGN_PROGRAM.md).
Each mechanism names its Skin IR footing: **built** (projection/verb exists) or a gap id from
[SKIN_IR_GAP_REGISTER.md](SKIN_IR_GAP_REGISTER.md). Stage designers treat these as requirements
to place, not options to consider; a stage spec that drops one must say why.

## See — structure without squinting

1. **Semantic zoom: names over blocks.** Zoom a sheet out and detail yields to structure —
   named ranges, tables and spill regions render as labeled blocks; zoom the tree out and
   subtrees become titled cards. *(grid overlays + defined names: built · LOD tiers: G4)*
2. **Collapse to summary, not ellipsis.** A folded node shows an aggregate of its children —
   total, count, sparkline — chosen per node; the rule is overlay data. *(children + series
   projections: built)*
3. **Spill topology made visible.** Origin badge + quiet extent veil on every dynamic array;
   members state their anchor; #SPILL! blockers point at the blocking cell.
   *(GridSpillOverlayDescriptor + SpillDisplay: built)*
4. **Provenance typography.** Constants, formulas, spill members and external feeds are set
   differently (weight, ink, tick) — a sheet reads like annotated source.
   *(authored kind + ValueProvenanceProjection: built)*
5. **Format provenance inspector.** "Why does this cell look like this": explicit vs inherited
   vs conditional vs locale, winning rule highlighted. *(G2)*

## Move — navigation at the speed of intent

6. **Peek cards.** Alt-hover any reference/name/node → resolved target in a transient card
   (value, shape, format) without leaving your place; pin two to compare.
   *(ReferenceResolutionProjection + detail projections: built)*
7. **Reference X-Ray.** Caret inside a formula lights its references on stage; tree references
   replay their resolution path step by step (walk-up, anchor, selector). *(tree: built via
   token_span + resolution targets · grid: G9)*
8. **The command deck.** One palette (Ctrl+K): commands, goto-anything (A1, path, name, table
   column), function docs. Every entry shows its chord and carries a stable command ID. *(G8)*
9. **Cross-stage continuity.** Switch Sheet → Notebook → Atlas and the selection survives,
   briefly haloed. Teaches the projection mapping for free. *(SharedSkinState: built)*
10. **Keyboard atlas.** Hold the leader key: the current stage annotates itself with every
    available chord, including browser-safe alternates. *(verb grammar: built · host-resolved
    bindings: G8)*

## Edit — confidence before commitment

11. **Partial-evaluation pills.** Select any subexpression → its value, type and shape in
    place; step evaluation outward one ring at a time. F9 that never destroys the formula.
    *(OneFormula drill: built · all contexts: G1)*
12. **Ghost edits.** Structural changes (move, delete, paste-block, table grow) render as
    ghosts with a legality verdict before commit; Esc is an exact revert.
    *(PreviewService + MutationImpactProjection: built)*
13. **Instrument grips.** Selection handles with live readouts: fill shows the series it will
    write, resize shows the delta, table-grow shows the rows it will absorb. *(G3 + G8)*
14. **Big-paste intelligence.** Paste a block → quiet offer to make it a table (headers
    detected, formats inferred, totals optional), preview first. *(PasteExternalClipboardText:
    partial · tableify: G3)*
15. **Rename is refactor.** Renaming a name/node/column shows every reference that will
    rewrite, as a diff, before it happens. *(reverse_references + preview seam: built for tree ·
    defined names: engine ask)*

## Trust — the model states its own health

16. **Calc HUD.** Every run reports what recalculated, in what order, where the time went;
    scrub the last run along real evaluation order. *(CalcRunProjection + phase timings: built)*
17. **Error triage.** Errors group by root cause with blast-radius counts; fix the origin,
    watch the group collapse. *(dependency graph + invalidation reasons: built for tree · grid: G9)*
18. **Feed instruments.** RTD topics and providers are instruments in the Strip: tick pulse,
    staleness age, quarantine state. External uncertainty is visible, never silent. *(G7)*
19. **The timeline.** Undo history as a browsable revision timeline with transaction summaries
    and named checkpoints; branch a candidate from any point. *(RevisionHistoryProjection DAG +
    candidates: built for tree · workbook undo: G3)*

## Drive — agents as first-class operators

20. **The surface tree.** Everything visible is addressable and stateful over MCP — the same
    intents, catalog and receipts humans generate; an agent's cursor is visible on stage; every
    step lands in the same audit log and timeline. *(intents/receipts/IntentRecord + B3 lane:
    built · serializable catalog: G8)*
