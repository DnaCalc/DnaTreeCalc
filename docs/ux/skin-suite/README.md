# ATLAS — One Model, Many Lenses

A **multi-perspective skin suite** for DNA TreeCalc: seven primary lenses (each an organizing
*structure*), four companion panes (compose in other slots), and four cross-cutting modes (modifiers
*within* a lens), bound into one tool by **one grammar, one styling language, one continuity, one
composition** — built on the [upgraded stack](../stack-requirements/) and using
[FLOW](../flow-skin/) as the reference lens.

> Not "the best skin" — a coherent suite that works together. A system you flow through —
> capture → structure → compute → explore → review → present — as one continuous session over one
> model, with zero re-orientation.

- **Mockup:** the flagship cockpit (Flow + Lens + Bench + Transport + Console) is openable at
  [`../prototypes/10_cockpit.html`](../prototypes/10_cockpit.html); the FLOW lens alone is
  [`../prototypes/09_flow.html`](../prototypes/09_flow.html).
- **Provenance:** the full multi-agent synthesis — every lens's signature, stack-faculty map, and the
  22 critique resolutions — is preserved verbatim at
  [`reference/ATLAS_SUITE.raw.json`](reference/ATLAS_SUITE.raw.json).
- **Requirements:** ATLAS consumes [`../stack-requirements/`](../stack-requirements/) and surfaced six
  additions, folded in there as the *Suite-surfaced additions*.
- **Built (Phase A, slice 1):** the shared spine — [`SPINE.md`](SPINE.md) (one grammar / one
  continuity / one styling, in `dnatreecalc-skin-framework`) — and the reference lens —
  [`lenses/FLOW.md`](lenses/FLOW.md) (`dnatreecalc-skins/src/flow.rs`, `Ctrl+6`). The cockpit /
  multi-slot platform (Phase B) is not built yet; the spine is built so it drops in without rework.

This README is the durable summary. (The fuller per-lens / spine / journey breakout can be expanded
into a doc set on request; the raw synthesis holds every field today.)

---

## The seven primary lenses (Main slot — each an organizing structure)

| Lens | Modality | One line | Evolves |
|---|---|---|---|
| **Capture** | input / scaffold | Structure-by-typing: type a dotted path that doesn't exist and the tree scaffolds itself (one transaction = one undo); Tab indents, Enter drops a sibling, paste a block as a dry-run | *new* (+ proto 03/04, absorbs `outline_table`) |
| **Tree** | structure / refactor | The hierarchy made spatial; reshape like clay with a live legality net so a formula never breaks | `outline_table` (re-indented) |
| **Ledger** | wide audit + cleave + bulk | Think in **populations**: cleave 100k rows to the 200 that matter by predicate, sort by real typed value, group into health cohorts, author the class at once | `outline_table` + `dependency_inspector` |
| **Sheet** | focused edit + tables | The Excel edit loop on a tree: values by default, a real formula bar, point-mode reference insert + F4 *binding* cycle, arrays/tables expand in place — no A1, no coords | proto 06 + 02 |
| **Flow** | explore / calc-theatre | The dependency-flow "sentence": F9 reading-head sweep, ]/[ trace-as-layout, recursive explain, ghost what-if | **FLOW** (the reference lens) |
| **Canvas** | wide overview + spatial zoom | The model as a constellation you arrange and keep; F9 spreads a recalc wave as igniting geometry; lasso-and-promote a region to a template | proto 08 |
| **Story** | present / narrate (read-only) | A finished model ships as a faithful, tamper-proof narrative that can **replay a walkthrough of its own reasoning** | `value_board` + the editorial reference |

### Four companion panes (other slots; compose with any primary)
- **Lens** — the inspector / multimeter: focus+context drill of the selection (recursive explain, value, format authoring, precedents/dependents). Owns **aspect / node-detail** zoom. Subsumes `dependency_inspector` + the format-editor (proto 05).
- **Transport** — the revision-DAG time scrubber: undo-as-navigation, branch points, value-shape change-pulse between any two revisions.
- **Bench** — the scenario bench / wind-tunnel: scenario rail (Base/Bull/Bear), side-by-side compare columns, goal-seek convergence, sensitivity / tornado, consequence-free ghost what-if.
- **Console** — the command + search + health + **workspace** spine: the canonical `/` Name-Box + model-query, command palette, health counters, workspace switcher, active-lens + persona indicators.

### Four cross-cutting modes (modifiers within a lens, not skins)
**Filter/Sort** (cleave; home = Ledger, predicate carries as continuity) · **Focus/Zoom** (collapse/pin are shared; viewport zoom is skin-local) · **Present** (strip chrome, read-only) · **Persona** (Author / Reviewer / ReadOnly, enforced per intent origin).

---

## The shared spine (why it's one tool, not seven)

- **One grammar.** A small *universal* verb table, defined **once** in the keybinding registry, whose
  meaning is fixed in every lens — only the visual realization differs. It is genuinely collision-free:
  **Enter** is the *sole* commit-and-advance (next sibling/row), **F9** the *sole* recalc, **Ctrl+D**
  the *sole* fill; arrows are standardized (↑↓ = sibling, ←→ = toward-parent/child) and **h/l** is the
  *sole* fold (a bare arrow never means "collapse"); ` ] / [ ` trace; `/` Name-Box; **Space** leader +
  health; **E** explain; **Ctrl+Z/Y** revision-nav; **Ctrl+1..7** switch lens; one canonical **Esc**
  ladder. `Tab`, `Ctrl+Enter`, and **drag** are explicitly *lens-local* secondary chords, badged as
  such — so a key a user thinks is universal always behaves universally.
- **One styling language.** Shared design tokens. **`calc_state` is the only saturated channel on
  nodes** in every lens; provenance tints (published / pending / speculative-ghost / scenario /
  external) are *structural*, not decorative; the SELECTED-vs-EDITING border and the identity typeface
  are identical everywhere. **Story** is the one *declared* exception — warm editorial chrome, but nodes
  keep `calc_state` + provenance + the shared identity face, so a figure reads as the same node.
- **One continuity.** Switching a lens is **re-projection, never re-load**: selection, scope, collapse,
  pins, focus, active scenario, current revision, recalc-mode, *and the cleave predicate* are host-owned
  shared truth. The honest caveat: lens-*intrinsic* geometry (Canvas x,y, Sheet column widths, Story
  block order) is **preserved** via persisted `SkinState`, not reconstructed from a model that never had
  it.
- **One composition.** The cockpit: multi-slot, capability-negotiated, focus/keybinding-arbitrated,
  persona-per-origin, fault-isolated. *(This is the W5 destination — see rollout.)*

### Eight tenets
1. One verb, one meaning, everywhere — defined once in the registry.
2. Modeless 1-bit authoring (SELECTED vs EDITING).
3. Frame-only over engine truth (no parse / no fake / no A1 / no `$`-anchoring).
4. Switching is re-projection, never re-load — for *shared* truth (intrinsic geometry is preserved).
5. `calc_state` is the only saturated signal (Story is the one sanctioned exception).
6. Provenance is never ambiguous.
7. Cross-cutting modes are lens modifiers, not skins.
8. Predict before you pay; undo is navigation.

---

## Rollout (honestly sequenced against the stack waves)

The cockpit is the **destination**, not the baseline — ATLAS ships a single-slot **mono-lens core
first**, then composes:

| Phase | What | Stack |
|---|---|---|
| **A — mono-lens core** | Re-skin Flow as reference; evolve Tree / Ledger / Sheet / Capture as **single-slot, single-select** skins; Lens + Console content lives *inside* a mono-lens | W0–W3 + the early-W5 subset (design-tokens, delta-channel, a11y, persistence) + `cleave-predicate-shared`. Most of the suite's value **without** the cockpit |
| **B — composable cockpit** | The W5 platform: multi-slot, negotiation, focus/keybinding arbitration, persona, isolation, worker-calc; promote Lens + Console to real companion slots | W5 (the *last* platform wave — not the first, as a naïve reading would place it) |
| **C — time + speculation** | Transport + Bench companions; ghost what-if, scenarios, undo-as-navigation light up everywhere | W4a/W4b/W4c (the two gating engine spikes) |
| **D — narrative + tables + spatial reuse** | Story; Canvas revision-morphing + promote-to-template; Sheet table authoring → full ATLAS | W6 |

### Six flagship cockpits (W5+ presets)
Modeling (Flow · Lens · Bench · Transport · Console) · Author (Tree · Lens · Flow) · Audit (Ledger ·
Lens · Flow · Transport) · Sheet (Sheet · Lens) · Map (Canvas · Lens · Flow/Ledger) · Story (Story ·
Lens-on-demand · Transport-as-timeline).

### Six requirements ATLAS surfaced
`cleave-predicate-shared` (W2/spine) · `shared-focus-set` (W5, *no unified zoom intent*) ·
`cockpit-preset-registry` (W5) · `facade-position-persistence` (W5, Canvas) ·
`replay-authored-artifact` (W6, Story) · `narrative-projection` (W6, Story). Folded into
[`../stack-requirements/FUNCTIONALITY_MATRIX.md`](../stack-requirements/FUNCTIONALITY_MATRIX.md#suite-surfaced-additions-atlas).
(Tables are already covered by `table-structural-ops` + `table-cell-readback`; ATLAS confirms Sheet as
the consumer.)

---

## How the existing surface maps in

| Existing | Becomes |
|---|---|
| `outline_table` skin | **Ledger** (its seed) + **Tree** (restores indentation, keeps `is_meta` visible) |
| `triple_editor` skin | **Tree** + **Lens** composed (a W5 cockpit) |
| `dependency_inspector` skin | **Lens** drill + **Ledger** typed Deps columns |
| `value_board` skin | **Story** headline strip + **Bench** compare columns |
| `formula_tree` skin | folded into **Tree** / **Sheet** |
| proto 02 array-value | **Sheet** array-expansion mode |
| proto 03/04 template/outline | **Capture** |
| proto 05 format-editor | **Lens** format-authoring mode |
| proto 06 excel-cell | **Sheet** |
| proto 07 nodes-across | **Flow** static columnar layout mode |
| proto 08 canvas-flow | **Canvas** |
| proto 09 flow | **Flow** (the reference lens) |
