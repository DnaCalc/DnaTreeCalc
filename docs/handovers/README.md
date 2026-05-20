# Handovers

Cross-repo coordination. Lightweight by design — see [`../../OPERATIONS.md`](../../OPERATIONS.md) §7.

- One file per handover: `HANDOVER_<TARGET>_<short_topic>.md`.
- Request at the top; a one-word status line (`Open` / `Responded` / `Done`); the response is appended to the same file (response = completion).
- No separate response / acknowledgment / receipt docs, and no register.
- An incoming handover can become an open bead directly (`br create … --status open`).

Useful lightweight header:

```
Status: Open
Target: <repo>
Ask: <one sentence>
Context: <why this matters>
Evidence: <links / repro / spec section, only if useful>
```

Handovers this repo has authored or expects to author. Engine prerequisites (`../model/CORE_MODEL_SPEC.md` §6) are raised as **topic-specific** handovers, not one monolithic `engine_prereqs` file.

Authored:

- `HANDOVER_OXFML_constant_input.md` — adopt OxFml's §2.1A cell-entry classification on the tree-reference channel (constant vs. formula by leading `=`). Engine prereq §6 item 11.
- `HANDOVER_OXCALC_iterative_cycle_config.md` — host-config contract for circular-reference cycle profiles + iterative bounds, plus the diagnostic surface back. Engine prereq §6 item 12.

Anticipated:

- `HANDOVER_OXXLPLAY_workbook_construction.md` — request the `WorkbookConstructionSpec` + construct-and-observe capability (and UDF provisioning). See `../interop/EXCEL_EXPORT_AND_REPLAY.md` §6, §12.
- `HANDOVER_OXREPLAY_treecalc_lane.md` — register the `dna_treecalc` lane + adapter; confirm locator-keyed view pairing.
- Further `../model/CORE_MODEL_SPEC.md` §6 engine prerequisites, raised topic-by-topic as they become actionable.
