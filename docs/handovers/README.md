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

Known handovers this repo expects to author (per the Spec):

- `HANDOVER_OXXLPLAY_workbook_construction.md` — request the `WorkbookConstructionSpec` + construct-and-observe capability (and UDF provisioning). See `../interop/EXCEL_EXPORT_AND_REPLAY.md` §6, §12.
- `HANDOVER_OXREPLAY_treecalc_lane.md` — register the `dna_treecalc` lane + adapter; confirm locator-keyed view pairing.
- `HANDOVER_OXCALC_engine_prereqs.md` — the engine prerequisites in `../model/CORE_MODEL_SPEC.md` §6.
