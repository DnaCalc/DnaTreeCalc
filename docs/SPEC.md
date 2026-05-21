# DNA TreeCalc — Spec Index

This file is the index for **the Spec**: the combined requirements + design + top-level-planning document set. It is referenced from [`../CHARTER.md`](../CHARTER.md) and [`../OPERATIONS.md`](../OPERATIONS.md) (§3) as the entrypoint to design truth. Start with the [Charter](../CHARTER.md) for mission and context; this set covers the design detail.

The Spec is a structured **set**, not one monolithic file — grouped into three areas: **model** (the calculation/language model), **interop** (Excel import/export/verification), and **ux** (the user experience — requirements, technical plan, skin architecture, prototypes). Edit it freely as the design evolves; beads that change behavior update the Spec documents they touch.

The v1 boundary — what's in scope, what's deferred (with an architectural hook), and what's parked for later — is drawn in one place: [`SCOPE.md`](SCOPE.md). Consult it before treating something as a gap.

For *how we work* (worksets, beads, handovers), see [`../OPERATIONS.md`](../OPERATIONS.md) and [`WORKSET_REGISTER.md`](WORKSET_REGISTER.md).

---

## Reading order

For someone new to the project:

1. [`../CHARTER.md`](../CHARTER.md) — mission, north star, place in the DNA Calc program, position among sibling repos.
2. [`model/CORE_MODEL_SPEC.md`](model/CORE_MODEL_SPEC.md) — the core: tree model, reference syntax, capability profile, value model, templates, Excel import. The foundational spec.
3. [`model/META_NODES.md`](model/META_NODES.md) — the `is_meta` mechanism for host-managed tree data.
4. [`ux/REQUIREMENTS.md`](ux/REQUIREMENTS.md) — what the application must present and let users do.
5. [`ux/SKINS.md`](ux/SKINS.md) — the skin architecture (the load-bearing UI design decision).
6. [`ux/prototypes/index.html`](ux/prototypes/index.html) — the visual mockups (the eight skins, rendered).
7. [`ux/TRACEABILITY.md`](ux/TRACEABILITY.md) — maps prototype affordances to skins, primitives, host state, DnaTreeCalc services, and OxFml/OxCalc flows.
8. [`ux/IMPLEMENTATION_MATRIX.md`](ux/IMPLEMENTATION_MATRIX.md) — UX-side implementation driver: trace IDs, scenario cards, contracts, harness expectations, and workset entry criteria.
9. [`ux/TECHNICAL.md`](ux/TECHNICAL.md) — how to build it.
10. [`interop/EXCEL_EXPORT_AND_REPLAY.md`](interop/EXCEL_EXPORT_AND_REPLAY.md) — export and verification against Excel.

---

## Model

The calculation and language model — what TreeCalc *is* underneath the UI.

| Document | Covers |
|---|---|
| [`model/CORE_MODEL_SPEC.md`](model/CORE_MODEL_SPEC.md) | Core tree model; reference syntax (`.` separator, `^` ancestors, `[]`/`[ws]` workspace anchors, bracket-escape, walk-up scope, set-producing operators); capability profile (`treecalc-v1` vs `strict-excel`); Excel-alignment principle; values & arrays; engine prerequisites; ownership boundary (§5.1); recalculation, calc-state & diagnostics (§7); templates; **tables** (§7c); structural editing & **undo/redo** (§8a); **node-as-function** (§3.8); **Excel defined-name import** (§10); compact grammar. The authoritative spec. |
| [`model/META_NODES.md`](model/META_NODES.md) | The `is_meta` per-node flag — host-managed tree data (templates, formatting, per-skin UI state) invisible to formulas. Single-flag design; behavior; canonical uses; engine ask. |

## Interop

Excel interoperation and verification.

| Document | Covers |
|---|---|
| [`interop/EXCEL_EXPORT_AND_REPLAY.md`](interop/EXCEL_EXPORT_AND_REPLAY.md) | Converting a workspace to Excel and verifying it against Excel as canonical truth. Repo partitioning (TreeCalc converts; OxXlPlay builds+observes; OxReplay compares+governs); the `WorkbookConstructionSpec` contract; export strategies (defined-names primary + grid-cell promotion); bake/mangle passes; export manifest; UDF provisioning; verification commands; end-to-end flow; handover docs to author. (Excel *import* is in `model/CORE_MODEL_SPEC.md` §10.) |

## UX

The user experience.

| Document | Covers |
|---|---|
| [`ux/REQUIREMENTS.md`](ux/REQUIREMENTS.md) | Conceptual UX requirements: personas, presentation areas, editing actions, keybindings, skin surfaces, interaction patterns, adaptive behaviors, cross-cutting concerns, coverage check. |
| [`ux/SKINS.md`](ux/SKINS.md) | The Winamp-style skin architecture: layered design, `WorkspaceSkin` trait, `SkinContext`, intent dispatch, per-skin meta-namespaces, formatting-vs-skin-styling boundary, call traces, phasing. The load-bearing UI architecture. |
| [`ux/TECHNICAL.md`](ux/TECHNICAL.md) | Implementation/integration plan: tech stack (Leptos/WASM, extends DnaOneCalc), crate layout, state model, OxCalc bridge, persistence format, per-component plan, UDF hosting, performance, build phasing, verification host. |
| [`ux/prototypes/index.html`](ux/prototypes/index.html) | Eight visual HTML mockups — the first eight skins: workspace shell, array value, template editor, outline-table, format editor, Excel-style cell, nodes-across, canvas flow. Open `index.html` to navigate. |
| [`ux/TRACEABILITY.md`](ux/TRACEABILITY.md) | Fine-detail UX traceability: prototype-to-skin mapping, feature ownership, state/intent/engine boundaries, and concrete flows from editor text to OxCalc publication and invalidation/resize back to display updates. |
| [`ux/IMPLEMENTATION_MATRIX.md`](ux/IMPLEMENTATION_MATRIX.md) | UX-side work driver built from traceability: stable trace IDs, implementation slices, component/service contracts, scenario cards, trace events, harness expectations, and workset entry criteria. |
| [`ux/design-references/`](ux/design-references/) | Loose visual references and mood-board notes for possible future skins or prototype directions. Not locked product design. |

---

## Verification corpus

The executable, spec-derived test corpus lives in [`test-corpus/`](test-corpus/) — declarative JSON cases that pin TreeCalc's **novel surface** (path-syntax → engine-reference mapping, walk-up resolution, profile gating, import mapping), with a pwsh validator (`test-corpus/tools/validate-corpus.ps1`) as the local-tier check until the OxCalc-bridge runner lands (W002). It deliberately does **not** retest engine / Excel-aligned behavior (function semantics, values, formats, dates, error codes) — that stays Excel-anchored in OxFunc/OxFml/OxCalc + OxReplay and is never reimplemented here. See [`test-corpus/README.md`](test-corpus/README.md) for the TreeCalc-vs-OxCalc boundary and the gate-tier mapping ([`../OPERATIONS.md`](../OPERATIONS.md) §6).

---

## Cross-cutting notes

- The **capability profile** model (`treecalc-v1` / `strict-excel`) in `model/CORE_MODEL_SPEC.md` §4 gates every TreeCalc-specific extension; grid hosts default to strict Excel.
- **Engine prerequisites** for OxCalc / OxFml / OxFunc are listed in `model/CORE_MODEL_SPEC.md` §6 and raised through `docs/handovers/` when cross-repo work is needed.
- **Meta-nodes** (`model/META_NODES.md`) are the substrate for three features that recur across the docs: templates, formatting, and per-skin UI state.
- The relationship to Excel runs in both directions: **import** (`model/CORE_MODEL_SPEC.md` §10) and **export/verify** (`interop/EXCEL_EXPORT_AND_REPLAY.md`).
- UX implementation is anchored from the other side by `ux/IMPLEMENTATION_MATRIX.md`: prototype affordances are assigned trace IDs and scenario checks so W003/W005/W006 can drive from visible behavior into host services and engine boundaries.

## Status

These documents capture the design exploration to date. They are living documents — edited in place as decisions evolve, with no separate "status" or "proposal" layering. Where the design depends on engine work in the sibling repos, that dependency is stated as content (an engine prerequisite or a handover request), not as a TreeCalc-side status marker.
