# PARITY_TRUST_UX — Excel-parity evidence in the interface

Status: design traces only · 2026-07-13 · **explicitly outside the S0–S5 work lists and
acceptance criteria** (owner direction 2026-07-13). This document exists so the parity story
returns *uniformly* later, and so the current phase reserves the right seams without building
any of it. No bead may cite this doc as in-scope work; specs may cite it only as "seam
reserved, renders nothing this phase."

## 1. Why this exists

The stack's differentiator is evidence: OxXlPlay captures observed Excel behavior, OxReplay
turns runs into replayable witnesses and diffs, U-ORACLE makes differential-vs-Excel the CI
gate. Users should eventually get the same deal CI gets: **don't take our word — see the
evidence.** Trust is a product feature, and it must feel like one system everywhere (Bench,
Sheet, Notebook, Atlas), not a OneCalc gadget.

## 2. Parity state vocabulary (uniform, five states)

| State | Meaning | Presentation |
|---|---|---|
| **Match** | value/type/format agree with retained Excel evidence for this case | quiet ✓ mark |
| **Divergence** | evidence exists and disagrees; diff available | red mark + diff entry point |
| **No evidence** | case not covered by any capture | grey "no evidence" — silence is never implied coverage |
| **Evidence stale** | evidence predates engine/Excel/locale version in play | signal-tinted age note |
| **Out of scope** | deliberate, documented divergence (novel tree surface, post-Excel semantics) | neutral "by design" with pointer |

Laws: parity is **evidence, not enforcement** — it never blocks editing, never gates calc.
And *out-of-scope* is first-class: the Rich Tree novel surface must read as "beyond Excel, by
design," never as a failed comparison.

## 3. Visual law (Strand addition, ratify with this doc later)

Parity marks reuse the standard semantic colors (green-ink / red / grey / signal) but carry a
**distinct glyph family — the twin block ◫** (two paired blocks, from the mark) — so
calc-state, provenance, and parity-state can never be conflated. Parity gets **no hue of its
own**, ever. Amber remains calc-attention only.

## 4. Granularity ladder (where evidence attaches)

1. **Value** — this result matches its captured Excel counterpart (axes: value, type, format
   render, precision).
2. **Function/formula** — the functions this formula uses are parity-covered (coverage from
   corpus, per profile + locale).
3. **Document** — the parity report: N formulas, coverage %, divergence list, out-of-scope
   list. Natural home: **Atlas Health lens**.
4. **Run** — this recalc is witness-backed (OxReplay): reproducible, diffable against a prior
   run or an Excel observation.
5. **Program** — the public conformance story (dashboards, corpus stats). Out of product UI
   scope entirely; noted for continuity.

## 5. Reserved seams (what the current phase keeps, all render nothing)

| Surface | Seam | This phase |
|---|---|---|
| Strip (shell) | one parity readout slot, rightmost group | slot reserved in layout math; renders nothing |
| Inspector | "Evidence" block as a named typed slot | slot enum carries the variant; never populated |
| Atlas · Health | parity report section | section id reserved in the lens layout |
| Bench | oracle detail overlay (the fullest instrument: per-axis diff, evidence provenance, witness link) | **not built in S1** — mission text keeps the Twin Oracle identity; UI deferred |
| Notebook · published | "verified against evidence" footer badge for readers | deferred; the highest-leverage trust moment for non-modelers |
| Capability | `HostCapabilityProjection.replay_or_comparison` | already exists — the one live seam; stays false-capable until the return |

## 6. Interaction traces (design intent for the return, not commitments)

- **Diff anatomy**: divergence opens a four-axis diff (value · type · format render ·
  precision) with evidence provenance (capture id, Excel build, locale, date). One layout for
  Bench and Inspector.
- **Witness hop**: from any divergence, one step to its OxReplay witness reference (copyable
  id; deep tooling stays in the appliance lane, not the product UI).
- **Coverage honesty**: function-level coverage shows numerator and denominator ("31 of 34
  functions in this document have evidence"), never a bare percentage.
- **Request capture**: a divergence or no-evidence case can emit a capture request artifact
  (routes into the OxXlPlay corpus workflow) — users grow the corpus by using the product.
- **Agent parity**: the same verdicts over MCP so agents can cite evidence in their outputs
  (mechanism 20 extension; verdicts are receipts-shaped, addressable).
- **Uniform entry**: one command-deck verb family (`parity: …`) across all surfaces; no
  per-stage bespoke entry points.

## 7. Future IR asks — the P-register (NOT filed this phase)

Held here, deliberately out of `HOST_AND_SKIN_IR_REQUIREMENTS.md`, until the owner re-opens
the lane:

- **P1** `ParityEvidenceProjection` — per-scope verdict {state ×5, axes, evidence ref,
  capture/build/locale metadata}.
- **P2** `DocumentParityReport` — ladder rung 3 (coverage, divergences, out-of-scope).
- **P3** Witness reference surface (run-level; OxReplay id + reproduce affordance).
- **P4** Capture-request emission verb (corpus growth loop).
- **P5** Coverage-by-function projection (rung 2), profile- and locale-aware.

## 8. What the current phase must NOT do

No parity UI beads · no P-register filings · no acceptance criteria referencing oracle
comparison (BENCH_SPEC amended accordingly) · no fake or placeholder parity marks anywhere —
an absent feature renders as nothing, not as "no evidence" (that state belongs to the built
feature). The only permitted footprint: the reserved slots in §5 and the visual law in §3.
