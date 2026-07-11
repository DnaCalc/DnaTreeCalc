# Skin IR Gap Register — asks filed by the front-end redesign

Status: v0.1 · 2026-07-12 · companion to [REDESIGN_PROGRAM.md](REDESIGN_PROGRAM.md).

The layering is strict: skins speak `dnacalc-skin-ir` only. Where the redesign needs more than
the IR expresses today, the need is registered here, then filed into
`docs/ux/stack-requirements/HOST_AND_SKIN_IR_REQUIREMENTS.md` (the official ledger — several
asks below already exist there as planned-not-built; ledger ids noted) and into upstream lanes
where the substrate is engine-owned. **No shims: a stage that lands before its ask degrades to
an honest primitive.**

Baseline capability map used for the "today" claims: skin-ir/host-core at main, 2026-07-12
(see `src/dnacalc-skin-ir/src/*.rs`, `src/dnacalc-host-core/src/*.rs`).

| # | Ask | Today | Gates | Likely owner |
|---|---|---|---|---|
| G1 | **Unified formula editing** — generalize the OneFormula editor/assist/drill surfaces (token runs, staged diagnostics, completions, signature help, partial-eval drill) to any authoring context: node, grid cell, table column, defined name | Rich surfaces exist only on the OneFormula document (`formula.rs`); workspace formulas get raw text + `token_span` per reference + dry-bind preview | Bridge everywhere · mech 07/11 · S1→S2 boundary | OxFml editor services + IR + host-core |
| G2 | **Formatting family** — per-node/cell effective format (font/fill/border/align/sizes), CF rule *results*, format-resolver seam `render(value, code, locale)`, locale presentation layer | Node-level `number_format_code` only (`EffectiveFormatProjection`); grid cells carry no format (`grid_publication.rs:155`); CF/locale exist only in OneFormula `FormattingSurface` | Sheet & Model styling · mech 05 · S3/S4 | OxFml (resolver) + OxCalc/OxDoc (storage) + IR — ledger: `format-resolver-on-context`, `per-node-effective-format`, `locale-presentation-layer` |
| G3 | **Grid interaction pack** — range selection type in the IR, row/col insert/delete/hide/resize/freeze, grid fill + grid-scoped clipboard verbs, merged-region authoring, workbook Undo/Redo | `SelectionState` has node + table-cell only; no row/col ops; clipboard is node-keyed; workbook path lacks Undo/Redo (`UnsupportedByModel`) | Sheet parity · mech 13/14/19 · S3 | OxCalc verbs + IR + host-core |
| G4 | **Viewport & LOD pack** — honor `SetGridInterest` on the workbook dispatcher (currently a no-op: `workbook_dispatcher.rs:161-168`), multi-rect interest + prefetch tiles, intra-window cell diffs, windowed tree projection, server-side query | Windowed `GridProjection` + epochs exist; single rect; `GridChanged` reships whole window; tree snapshots ship all nodes | Huge grids, GPU path, agents-at-scale · mech 01 · S3 | host-core + IR — ledger: `virtualization-window-projection`, `model-query-projection` |
| G5 | **Value typing** — bounded array windows outside OneFormula, typed error codes, `Lambda`/`Rich`/`Image` value variants | Workspace arrays ship fully materialized; errors are `Error(String)` | Array-rich display everywhere · S2+ | IR + OxCalc — ledger: `richer-typed-value` |
| G6 | **Narrative blocks** — notebook block projection (blocks, order, cursor), prose stored in the manifest/annotation layer | Notebook is a convention over defined names + `_names` backing sheet; no block structure in the IR | Notebook as protocol · S2 | host-core + IR — ledger: `narrative-projection` |
| G7 | **Extension surface** — provider inventory/status/lifecycle diagnostics, RTD topic liveness/staleness, trust + quarantine states projected into the IR | `ExtensionPlacementProjection` + `unavailable_families` only; `dnacalc-extension-host-core` exists but nothing surfaces into the IR | Extensions manager, feed instruments · mech 18 · S1 (minimal slice) | extension-host-core + IR (aligned with `docs/ux/EXTENSION_ADAPTER_ARCHITECTURE.md`) |
| G8 | **Serializable command catalog + host-resolved keybindings + drag verdict protocol** | `CommandCatalogProjection`/manifests are `&'static str` (cannot cross a wire); chord resolution is Leptos-side; no drag/drop vocabulary | Command deck, keyboard atlas, grips, remote/agent skins · mech 08/10/13/20 · S0 ask, S3 for drag | IR + host-core — ledger: `keybinding-registry`, `drag-gesture-model` |
| G9 | **Grid dependency projection** — precedents/dependents (and cycle/blast-radius data) for grid cells | `DependencyGraphProjection` is tree-only | Atlas + X-Ray + error triage on Excel-strict · mech 07/17 · S3 | OxCalc + IR (relates to U-DEP lane) |
| G10 | **Shareable layout overlay** (optional) — canvas/diagram positions as overlay data traveling with the document | Positions are skin-local `SkinState` (deliberate; ledger: `facade-position-persistence`) | Model layouts across devices · S4 | host-core (overlay/manifest layer) |

Also inherited (engineering prerequisite, not a protocol gap): **RichTree session extraction**
into Leptos-free `dnacalc-host-core` — `DocumentSession::RichTree` is an empty seam today; the
real tree session lives Leptos-bound in `dnatreecalc-host`. Gates S4.

Filing discipline: each ask lands in the ledger with (a) the consuming stage + mechanism ids,
(b) a minimal-slice definition (what S-phase actually needs vs the full shape), (c) the honest
degrade the stage ships with until the ask lands.
