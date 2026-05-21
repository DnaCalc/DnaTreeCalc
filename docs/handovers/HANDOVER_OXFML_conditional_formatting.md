# HANDOVER_OXFML_conditional_formatting

Status: Open
Target: OxFml
Ask: Confirm / extend OxFml's conditional-formatting evaluation to the Excel CF model TreeCalc's format surface needs — ordered multiple rules per node/cell, "Stop If True", action accumulation across rules, and subtree-level CF inheritance — and clarify the boundary for host-computed (formula-valued) format properties.
Context: TreeCalc's per-node `Format` meta-children (CORE_MODEL_SPEC §6 item 10, META_NODES) and the Format Editor skin (SKINS §9.2; mockup 05) present the user with an ordered CF rule list with per-rule edit/delete/reorder. The format MODEL and the inheritance walk are TreeCalc's (`FormatResolver`), but the CF rule SEMANTICS are Excel/OxFunc-aligned and must be reusable from the engine, not reconstructed in the host.
Evidence: SKINS §9.2; CORE_MODEL_SPEC §6 item 10; ux/IMPLEMENTATION_MATRIX.md (`UX-FM-004` computed-format guard); ux/TRACEABILITY.md F5.

## What TreeCalc needs

1. **Ordered multiple rules** per node/cell, with deterministic evaluation order.
2. **"Stop If True"** — a rule attribute that halts evaluation on a true match.
3. **Action accumulation** across rules — rule 1 sets font, rule 2 sets an icon; both apply unless a rule stops.
4. **Subtree-level CF** — a CF rule on an ancestor's `Format.ConditionalFormat` applies to descendants through the format-inheritance walk.
5. **Per-cell result shape** — the post-CF appearance the host renders (an `ArrayCellFormat` per array cell, or a scalar equivalent), so `FormatResolver.apply_cf` can return it.
6. **Computed (formula-valued) format properties** — confirm where these evaluate (host render-time vs. engine) and, critically, that they do **not** silently join the node dependency graph unless explicitly designed. TreeCalc guards this (`UX-FM-004`); we need the engine boundary stated so the guard is honest.

## Expected disposition

Part **confirm** (whatever CF support already exists in OxFml/OxFunc), part **design-review / coordinate** — the Excel-faithful multi-rule semantics (1–4) and the computed-format boundary (6) are the open pieces. This is engine-prerequisite §6 item 10 raised as a concrete handover rather than an inline TODO in SKINS §9.2.
