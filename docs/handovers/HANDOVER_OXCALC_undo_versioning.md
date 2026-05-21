# HANDOVER_OXCALC_undo_versioning

Status: Open
Target: OxCalc
Ask: Confirm / define the host-facing interface and semantics for **version-based undo/redo** — immutable structure-tree versioning so the host can undo a node add/move/delete/formula edit and view calc results from before a structural change, without faking the inverse host-side.
Context: DNA TreeCalc undo/redo is layered (CORE_MODEL_SPEC §8a). App-level interaction undo (keystrokes, view/mode state) is the host's. But model-affecting edits must lean on the engine's deeper model: TreeCalc wants to undo by moving between engine versions, not by reconstructing inverse edits in the host. This is the honest way to keep prior calc results viewable.
Evidence: CORE_MODEL_SPEC §8a (undo model), §6 engine prerequisites; the existing transactional-batch-edit prereq (§6 item 8) is the grouping primitive.

## What TreeCalc needs

1. **Version handles.** A stable handle per published structural version the host can name, retain, and return to (undo/redo as version navigation).
2. **Edit → version mapping.** Which structural edits produce a new version, and how a grouped batch (one user action = N engine edits) maps to one undoable version step.
3. **Prior-result visibility.** Whether a prior version's published values remain observable after later edits (so "undo" shows the old calc result, not a recompute-from-scratch).
4. **OxFml caching role.** How OxFml's bind/value caching (green/black tree) participates — what is retained vs. recomputed when navigating versions.
5. **What is *not* versioned.** Confirm calc runs (F9 / RTD / future scripting) are not undo steps; any calc-trace history is engine-internal, not a host undo surface.

## Expected disposition

Part **coordinate** (the production version-handle interface and the edit→version mapping), part **confirm** (prior-result visibility and the OxFml caching boundary). This is an ongoing area built out as TreeCalc's command taxonomy grows; the first ask is the minimal version-navigation contract.
