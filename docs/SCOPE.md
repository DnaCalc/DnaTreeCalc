# DNA TreeCalc — Scope Boundary

One canonical place that draws the v1 boundary, so scope questions don't get re-litigated and deferred ideas don't quietly grow. It consolidates the non-goals scattered across [`../CHARTER.md`](../CHARTER.md) and [`ux/REQUIREMENTS.md`](ux/REQUIREMENTS.md) §1.3/§8 and adds the long-term parking lot. Three tiers: **in scope**, **deferred with an architectural hook**, **later (parking lot)**.

## In scope (and being specified)

- The tree / reference model, walk-up scope, set-producing operators, capability profiles — the novel surface ([`model/CORE_MODEL_SPEC.md`](model/CORE_MODEL_SPEC.md)).
- The skin architecture and the minimum-lovable skin set (`triple-editor` + `outline-table` + `cell-view`).
- Templates; per-node formatting + conditional formatting; meta-nodes.
- Excel **import** and **export + replay verification** against Excel as canonical truth.
- **Dependency-graph view and navigation** (local map + on-demand subtree graph).
- **Table model** — a first-class TreeCalc node concept the engine unpacks (CORE_MODEL §7c); not an OxFunc value.
- **Cross-workspace references** — the obvious Excel-aligned policy: references into *currently-loaded* workspaces resolve; references into workspaces that are not loaded fail (CORE_MODEL §3.3).
- **Undo / redo** — layered, leaning on OxCalc's version model (CORE_MODEL §8a).
- **Node-as-function** invocation of lambda-valued nodes (CORE_MODEL §3.8).
- **Two build targets from the start:** the browser WASM shell and a **native Tauri desktop build** that can host native code in-process (`ux/TECHNICAL.md` §1, §1.1).

## Deferred — architecture leaves the door open (v2 / v3)

These are not v1 work, but the design already carries the hook so they are additive later, not rewrites:

- **Multi-pane / split composition** — the skin composition contract already allows it ([`ux/SKINS.md`](ux/SKINS.md) §7).
- **User-authored skins / extension model** — declarative or programmatic skin path ([`ux/SKINS.md`](ux/SKINS.md) §8).
- **Multi-user collaboration** — stable `TreeNodeId` identity gives a future merge basis (`ux/REQUIREMENTS.md` §7.3).

## Later — long-term parking lot (obvious, low-weight, no ceremony)

Recorded so they're not forgotten; they add no design weight now and we get to them when they matter:

- Installers and auto-updaters for the native build.
- UI-string internationalization / localization (the locale UI exists as in DnaOneCalc; number/date locale is OxFml's; we build the English app now).
- Mobile / touch-primary UX (a future Presentation-category skin is the natural home).
- Publish modes beyond Excel (static HTML, PDF, embeddable widget, public read-only sharing).
- Telemetry / anonymized usage and operational logging.
- Security hardening of native-code hosting (`.xll` / VBA in-process) — the posture is clear; hardening is later.

## Hard non-goals (structural, not "later")

- **No grid** — no coordinates, no `A1:B5` ranges, no inter-node spilling. The grid arrives with PreCalc and beyond (CHARTER).
- **No engine / no Excel-COM / no comparison machinery** owned here — consumed from the Ox\* lanes, never duplicated.
- **Charting / visualization** beyond basic value-shape rendering (`ux/REQUIREMENTS.md` §1.3).
