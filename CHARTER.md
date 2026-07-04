# DNA TreeCalc — Charter

## Mission (north star)

**DNA TreeCalc is the calculation environment where a model is a tree of named formulas — as natural and Excel-faithful as a spreadsheet, but freed from the grid.**

It is the first serious multi-node calculation host built on the OxCalc engine. Where a spreadsheet organizes calculation into a grid of cells, TreeCalc organizes it into a tree of named nodes — each with its own formula, value, and formatting — with references that resolve by name and lexical scope, nesting deeper than sheet-and-cell, and reusable subtree templates. Everything outside that tree/reference surface stays faithful to Excel: function semantics, the value model, number formats, dates, LAMBDA.

We want to end with a tool that is **good enough to use** for real hierarchical models (financial plans, scientific computations, scenario trees, data pipelines) and **rigorous enough to graduate the OxCalc engine** toward the grid hosts that follow it.

## Place in the DNA Calc program

DNA Calc is a near-formal, Excel-faithful spreadsheet platform — rigorously specified, verifiable, and built to evolve. It advances through a progression of hosts, each proving more of the system on shared infrastructure:

```
VbCalc  →  OneCalc  →  TreeCalc  →  PreCalc  →  SuperCalc  →  DNA Calc
(VBA)      (single     (THIS:       (Round 1   (Round 2     (Round 3
           formula)    tree-only    tree-grid  refinement)  product)
                       multi-node)  hybrid)
```

- **VbCalc** proved VBA hosting.
- **OneCalc** proves single-formula evaluation — one cell or defined name, no references, no scheduling.
- **TreeCalc (this project)** is the first *multi-node* host: many named nodes, references between them, dependency-driven recalculation, on the OxCalc engine. It proves tree-only calculation before the grid arrives.
- **PreCalc / SuperCalc** introduce and refine the tree-grid hybrid.
- **DNA Calc** is the synthesized, long-term Excel-parity product.

TreeCalc's role in the progression is twofold: be a genuinely useful product for hierarchical models that don't want a grid, **and** stress-test OxCalc's coordinator, dependency graph, invalidation, and epoch model with a real multi-node workload — proving them before the grid hosts depend on them.

A standing aim of the whole program shapes how we build. Much of Excel's behaviour is implicit; **DNA Calc exists in large part to make it explicit** — an implementation anchored, where possible, in formal descriptions (Lean, TLA+, the conformance corpus) so that the calculation stack — engine, formula evaluation, function calls — can be *changed while staying provably correct*. This is not a checkbox we tick once; it is a direction the implementation grows toward, and it carries real weight even where it does not gate day-to-day progress. TreeCalc serves it by keeping its own semantics — reference resolution, scope, set operators, recalculation — explicit and formally describable rather than buried in ad-hoc code, so the formal layer can grow to meet them.

## What TreeCalc is

- A **tree of named nodes**. Each node has a formula, a value (scalar, array, error, reference, lambda), and per-node formatting.
- **Excel-faithful outside the novel surface.** Function semantics, value model, number formats, dates, LAMBDA, error codes — all align with Excel. The novelty is confined to tree structure and reference identifiers.
- **A reference model** with lexical walk-up name resolution, relative ancestor/sibling navigation, cross-workspace references, and set-producing operators that compose with FILTER / INDEX / MAP.
- **Templates** — reusable subtree definitions, instantiated across the tree.
- **Meta-nodes** — host-managed data (templates, formatting, per-skin UI state) hung on the tree, invisible to formulas.
- **A skinnable UI** — multiple parallel front-ends (cell view, canvas, outline-table, three-pane editor, …) over one core, switchable at runtime.
- **Excel interop** — imports workbooks restricted to sheets + defined names with no formula rewriting; exports and verifies against Excel as canonical truth.

## What TreeCalc is not

- **Not a grid.** No coordinates; no value spilling between nodes. The grid arrives with PreCalc and beyond. *(Mark, 2026-07-02: this clause describes the TreeCalc **model**, not the repo's hosting scope. The owner-approved dual-profile decision (treecalc-v1 + strict-excel-grid) and the W011 DnaCalc-host pivot sanction hosting `.xlsx` grid workbooks from this repo through `dnacalc-host-core` — see `docs/WORKSET_REGISTER.md` "W011 pivot" and `docs/ux/DNACALC_HOST_CORE_XLSX_NOTEBOOK_PROOF.md`. A full charter amendment is pending.)*
- **Not an engine.** Calculation is OxCalc; the formula language is OxFml; functions and value semantics are OxFunc. TreeCalc is the *host*.
- **Not a reimplementation of Excel semantics.** Those live in the Ox\* libraries and are consumed, never duplicated.

## Position among the repositories

DNA TreeCalc consumes the **infrastructure (Ox\*) lanes** and parallels its **sibling hosts (Dna\*)**, all under **Foundation** doctrine.

**Consumes (infrastructure):**
- **OxCalc** — the multi-node calculation engine. TreeCalc's substrate (dependency graph, invalidation, coordinator scheduling, epochs).
- **OxFml** — the formula language: grammar, parse, bind, single-node evaluation.
- **OxFunc** — function semantics and the value universe.
- **OxXlPlay** — the Excel observation/interop harness (constructing and observing workbooks for verification).
- **OxReplay** — the comparison and verification appliance (diff, explain, witness governance).
- **OxVba** + a future shared UDF-hosting core — VBA UDFs and `.xll` native add-ins (developed first in OneCalc, then shared).

**Parallels (hosts):**
- **DnaOneCalc** — the single-formula host. TreeCalc reuses its formula-editor surface, direct engine integration conventions, drill panel, and verification harness conventions.
- **DnaVisiCalc** — the Round-0 grid pathfinder.

**Inherits (doctrine):**
- **Foundation** — architecture, capability profiles, conformance, replay governance.

**TreeCalc owns:** the tree/reference model surface, host UX (the skin architecture), structural editing, templates, and the model→Excel mapping. It owns no engine, no Excel-COM, and no comparison machinery — those belong to the Ox\* lanes. The guiding rule is *right responsibility in the right repo* — direct API calls to the right surfaces, no shims scattered across repos.

Put plainly, **TreeCalc is the UX-interaction and verification surface for the OxCalc / OxFml / OxFunc + replay stack.** It consumes engine types directly — no wrapper types, no re-interpretation seams, no parallel libraries — and where the engine carries a deeper model than the host displays (immutable structure-tree versioning, value caching, calc tracing), TreeCalc *leans on it* rather than reconstructing it host-side. Undoing a node edit, for instance, is the engine's version model surfacing through the host, not a host-side inverse-edit replay.

## Where we want to end (success criteria)

TreeCalc succeeds when:

1. A user can build a substantial hierarchical model — thousands of named nodes, templated subtrees, dynamic arrays — and calculate it interactively with confidence.
2. Any such model **verifies against Excel** as canonical truth, with divergences surfaced as durable evidence.
3. The OxCalc engine has been exercised hard enough that its coordinator, dependency, invalidation, and epoch semantics are **proven for the grid hosts that follow**.
4. The skin architecture has demonstrated that **one core carries many genuinely different front-ends**, so new ways of seeing a model are new skins, not rewrites.

TreeCalc is at once a product and a proving ground. It must be good enough to use, and rigorous enough to move OxCalc one host closer to the grid.

---

*The Spec (requirements + design + planning): see [`docs/SPEC.md`](docs/SPEC.md). How we work: see [`OPERATIONS.md`](OPERATIONS.md).*
