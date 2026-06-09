# HANDOVER_OXCALC_set_membership_write

Status: Open
Target: OxCalc
Ask: Add an OxCalc-owned W3 `set-membership-write` substrate for authored reference collections, with transaction-backed membership/order edits and published dependency/version updates.
Context: DNA TreeCalc W3 needs a Skin IR intent equivalent to `SetCollectionMembership { owner, source_reference_handle, members, order }` so a skin can author multi-value reference collections without grid coordinates or formula-text rewriting. DnaTreeCalc can currently expand `AuthoringScope::Collection` from engine-published reference-resolution facts, but that is a read projection only. The host must not mutate collection membership by editing formula strings, editing tree structure as a proxy, or manufacturing membership/order versions.
Evidence: `docs/ux/stack-requirements/ENGINE_REQUIREMENTS.md` defines `set-membership-write` as an OxCalc-owned `extend` item that bumps `membership_version` / `order_version`. Current OxCalc `OxCalcTreeEdit` exposes node, table, meta, rename/move/reorder/delete edits, but no collection-membership edit. Current OxCalc dependency facts expose `TreeReferenceCollectionDependency { family, host_ref_handle, base_node_id, membership_version, order_version, member_node_ids }`; DnaTreeCalc projects those into `ReferenceTargetProjection::Collection` and uses them for `AuthoringScope::Collection` expansion.

Update: OxCalc now exposes a first `SetReferenceCollectionMembership` transaction edit slice that
validates `owner_node_id`, `source_reference_handle`, and requested member node ids against current
OxCalc dependency descriptors. The slice returns typed `UnknownReferenceCollection` and
`ReferenceCollectionNotEditable` errors for current derived collection families such as
`@CHILDREN`. It does not yet store authored membership/order, bump collection versions, invalidate
dependents, or publish updated descriptors, so this handoff remains open for the positive mutation
substrate.

Update 2026-06-09: A direct OxCalc spike tested the tempting first positive substrate for
`ReferenceLiteralArrayV1`: store an edited member/order list keyed by `(owner_node_id,
source_reference_handle)` and apply it while building dependency descriptors. The spike is a no-go
as a standalone OxCalc implementation. It can make descriptors republish edited membership/order,
but OxFml runtime invocation still evaluates the original authored formula source, for example
`=SUM({A,C,A})` continues to compute from `A,C,A` after the descriptor membership is changed to
`C,A`. Shipping that would split dependency truth from runtime value truth. Positive
set-membership-write therefore needs an owning seam that changes both together: either an
OxFml-owned formula rewrite/bound-formula invocation API, or an OxCalc/OxFml runtime packet that
lets an edited reference collection replace the bound/evaluated collection value without changing
displayed authored text.

## Required Shape

The DnaTreeCalc host needs an OxCalc transaction API along these lines:

```rust
OxCalcTreeEdit::SetReferenceCollectionMembership {
    owner_node_id: TreeNodeId,
    source_reference_handle: String,
    member_node_ids: Vec<TreeNodeId>,
    order: TreeReferenceCollectionMemberOrder,
}
```

or an equivalent API that:

1. validates that `source_reference_handle` belongs to `owner_node_id`,
2. validates that the referenced descriptor is an authored/editable collection, not a derived structural collection such as ordinary `@CHILDREN` unless that family is explicitly declared editable,
3. stores the authored membership/order in OxCalc-owned model state,
4. bumps membership and order identity deterministically,
5. invalidates membership and member-value dependencies with typed reasons,
6. runs inside `OxCalcTreeEditTransaction` and returns a real transaction id,
7. republishes reference-resolution and dependency descriptors so DnaTreeCalc can project the updated collection without reinterpretation.
8. proves that the same edited membership/order drives formula evaluation, not only dependency
   descriptor projection. Descriptor-only overrides are explicitly rejected.

## Boundary

- OxCalc owns reference-collection membership, order identity, invalidation, dependency descriptors, transaction semantics, and publication.
- OxFml owns parse/bind handles and any formula syntax that names or creates an authored collection.
- DnaTreeCalc host owns closed intent dispatch, `NodeKey` to `TreeNodeId` lookup, scope expansion for request payloads, and projection of OxCalc-published facts.
- Skins render collections and dispatch typed membership changes only.

Until this substrate exists, DnaTreeCalc should keep `SetCollectionMembership` out of the supported Skin IR surface. It can continue to use `AuthoringScope::Collection` for read/selection/scoped authoring over already-published collection members, but not for editing collection membership itself.
