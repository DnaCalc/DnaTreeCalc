*Posted by Codex agent on behalf of @govert*

# OxFml Handoff: Completion Proposals Read From the OxFunc Registry, Not From `LibraryContextSnapshot`

Status: closed (OxFml landing acknowledged 2026-05-04)
Direction: DnaOneCalc → OxFml
Source repo / workset: DnaOneCalc / W068 follow-up
Filed date: 2026-05-04
Closed date: 2026-05-04
Related: `docs/HANDOFF_OXFML_FUNCTION_HELP_FROM_OXFUNC_REGISTRY.md`,
`OxFml/docs/handoffs/HANDOFF-DNAONECALC-003_W068_REGISTRY_BACKED_FUNCTION_HELP.md`,
`OxFml/docs/handoffs/HANDOFF-DNAONECALC-004_W068_REGISTRY_BACKED_COMPLETION_PROPOSALS.md`,
`OxFml/docs/worksets/W068_canonical_function_registry_consumption_cleanup.md`

## Closure note (2026-05-04)

OxFml landed the proposal-collector migration in
`HANDOFF-DNAONECALC-004_W068_REGISTRY_BACKED_COMPLETION_PROPOSALS.md`.
DnaOneCalc removed `library_context_snapshot_from_registry`,
`snapshot_entry_from_registry_entry`,
`library_context_provider_from_registry`, and the
`with_library_context_provider` call from
`src/dnaonecalc-host/src/adapters/oxfml/live_bridge.rs` the same day.
The bridge call is now a single
`EditorEnvironment::new(BindContext::default())`. All five
completion-popup scenario tests that exposed the gap
(`s_cmp_1` … `s_cmp_5`) pass with no host-side function-name
authoring. Workspace `cargo test` green.

## Summary

The W068 landing migrated `build_function_help_packet` from
`LibraryContextSnapshot.arity_shape_note` to the OxFunc registry —
verified working from the host. **`collect_completion_proposals` was
not migrated**: it still discovers function names exclusively through
`library_context_snapshot.entries`, with no fallback to the function
registry. This forces every host that wants a non-empty completion
popup to keep authoring a snapshot whose only purpose is to mirror
OxFunc's catalog into a list OxFml will read.

## Repro from DnaOneCalc

After landing the W068 host-side cleanup (delete
`DEFAULT_FUNCTION_NAMES`, delete the hand-rolled snapshot, drop
`with_library_context_provider` so the editor environment falls back
to its `builtin_registry()` default):

1. `cargo build` is clean.
2. Editor surface comes up.
3. `=NOW(` correctly shows `NOW()` with no synthetic args (function-
   help path is registry-backed, working as intended).
4. Typing `=SU` shows an **empty completion popup**. The proposal
   collector returned no `Function` proposals because
   `library_context_snapshot` is `None`.

DnaOneCalc test suite confirms the regression:

```
test scenarios::completion::s_cmp_1_typing_partial_function_opens_popup_with_proposals ... FAILED
test scenarios::completion::s_cmp_2_dismiss_closes_popup_and_does_not_reopen_on_same_input ... FAILED
test scenarios::completion::s_cmp_3_accept_selected_completion_returns_acceptance_and_dismisses_popup ... FAILED
test scenarios::completion::s_cmp_4_move_selection_round_trips_back_to_starting_index ... FAILED
test scenarios::completion::s_cmp_5_typing_a_non_trigger_clears_proposals_and_closes_popup ... FAILED
```

All five fail with assertions of the shape "expected popup Open
after '=SU', got Hidden" or "accept returns Some(acceptance) when
popup is Open" — i.e. no function proposals were emitted, so the
popup never opened.

## Code site

`crates/oxfml_core/src/language_service/mod.rs::collect_completion_proposals`,
around lines 427–453 (post-W068):

```rust
let library_context_snapshot = resolve_library_context_snapshot(&request);
if let Some(snapshot) = library_context_snapshot.as_ref() {
    let mut seen_functions = BTreeSet::new();
    for entry in &snapshot.entries {
        if seen_functions.insert(entry.surface_name.to_ascii_lowercase())
            && (normalized_prefix.is_empty()
                || entry.surface_name.to_ascii_lowercase().starts_with(&normalized_prefix))
        {
            insert_proposal(
                &mut proposals,
                4,
                CompletionProposal {
                    proposal_id: format!("function:{}", entry.surface_name),
                    proposal_kind: CompletionProposalKind::Function,
                    display_text: entry.surface_name.clone(),
                    insert_text: entry.surface_name.clone(),
                    replacement_span: Some(replacement_span),
                    documentation_ref: entry.interface_contract_ref.clone(),
                    requires_revalidation: true,
                },
            );
        }
    }
}
```

The branch only fires when a snapshot exists. The
`function_registry: &FunctionRegistry` field that already lives on
`EditorEnvironment` (and on `CompletionRequest` via the same plumbing
that fed `build_function_help_packet`) is not consulted here.

## Required change

`collect_completion_proposals` discovers function names from the
registry that the editor environment was built with, not from the
snapshot:

```rust
for entry in request.function_registry.iter() {
    if normalized_prefix.is_empty()
        || entry.surface_name.to_ascii_lowercase().starts_with(&normalized_prefix)
    {
        insert_proposal(
            &mut proposals,
            4,
            CompletionProposal {
                proposal_id: format!("function:{}", entry.surface_name),
                proposal_kind: CompletionProposalKind::Function,
                display_text: entry.surface_name.clone(),
                insert_text: entry.surface_name.clone(),
                replacement_span: Some(replacement_span),
                // documentation_ref now flows from the registry:
                documentation_ref: entry
                    .registry_metadata
                    .interface_contract_ref
                    .clone(),
                requires_revalidation: true,
            },
        );
    }
}
```

UDFs registered into the host-supplied registry surface in the same
loop. Capability-overlay-denied functions can either be filtered
here (via `with_capability_overlay`) or surfaced with a `documentation_ref`
hint; both fit the same code path.

The `if let Some(snapshot) = library_context_snapshot ...` block
disappears. `LibraryContextSnapshot` keeps its real job (per-run
availability / admission / provenance overlays for capability
gating), but stops being a function-name carrier.

`CompletionRequest` already carries `library_context: PinnedLibraryContextView`,
which `EditorEnvironment` populates alongside the function registry.
Either add `function_registry: &'a FunctionRegistry` to
`CompletionRequest` (mirroring the function-help path), or thread it
through `PinnedLibraryContextView`. Whichever fits OxFml's existing
design.

## Test coverage

Suggested additions inside `oxfml_core`:

1. With `EditorEnvironment::new(BindContext::default())` (default
   built-in registry, no library snapshot), prefix `=SU` →
   `CompletionResult.proposals` contains entries whose
   `proposal_kind == Function` and `display_text` includes "SUM",
   "SUMIF", "SUMIFS", "SUMPRODUCT", "SUBSTITUTE".
2. With `EditorEnvironment::new(...).with_function_registry(udf_augmented)`,
   prefix matching the UDF's surface name → the UDF appears as a
   function proposal alongside built-ins.
3. With a `CapabilityOverlay` that denies `RTD`, prefix `=R` → no
   `RTD` proposal in the result.
4. With **no** function registry handoff and **no** snapshot → no
   function proposals (parity with existing "no snapshot, no
   functions" behaviour, but now trivially true because there's no
   default-empty case to reach).

## Closure conditions

- `collect_completion_proposals` reads function names directly from
  the `FunctionRegistry` handed in via `EditorEnvironment`.
- `LibraryContextSnapshot` no longer carries function names for
  proposal purposes — its remaining duty is availability /
  admission / provenance.
- The four tests above pass.
- DnaOneCalc can drop `library_context_snapshot_from_registry` and
  the `with_library_context_provider` call from
  `src/dnaonecalc-host/src/adapters/oxfml/live_bridge.rs`, leaving
  `EditorEnvironment::new(BindContext::default())` as the only
  configuration call. The host-side helper exists today purely as a
  bridge for this gap; it should be deletable the same week the
  OxFml change lands.

## DnaOneCalc-side state in the meantime

DnaOneCalc as of `2026-05-04` keeps a thin
`library_context_snapshot_from_registry()` helper in
`src/dnaonecalc-host/src/adapters/oxfml/live_bridge.rs`. The helper:

- iterates `oxfunc_core::registry::builtin_registry()` (no hand-typed
  function list, no `DEFAULT_FUNCTION_NAMES`, no per-function
  authoring on the host side),
- maps each `FunctionEntry`'s identity / admission / provenance
  fields onto `LibraryContextSnapshotEntry`,
- sets all availability fields to `LibraryAvailabilityState::CatalogKnown`,
- carries no parameter shape information (`arity_shape_note` is
  retired upstream).

The helper is feeding OxFml's proposal-collector code path and
nothing else. When OxFml lands the change above, the helper goes
away in a single commit and the bridge call simplifies to
`EditorEnvironment::new(BindContext::default())`.
