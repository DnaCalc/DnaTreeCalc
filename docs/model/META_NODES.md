# DNA TreeCalc — Meta-Nodes

A meta-node is a tree node marked with the `is_meta = true` attribute. Meta-flagged nodes — and their entire descendant subtree — are invisible to formula reference resolution and are not bound or evaluated by the engine.

The concept is intentionally narrow:

- **No formula-language syntax** for meta-nodes. Formulas can't reach them.
- **No parallel namespace**. Meta-flagged nodes live in the same tree as regular nodes; they just have the flag set.
- **Host manipulates them directly.** Templates, formatting, configuration, and other host-data uses operate via the host's tree API, not through formula references.

## Per-node attributes

```
is_meta        bool, default false
hidden         bool, default false (auto-true when is_meta is set)
calc_excluded  bool, default false  (not in scope for v1; slot reserved)
```

Only `is_meta` is mandatory for the meta-node concept. `hidden` is a UI presentation flag. `calc_excluded` is reserved for future use cases (regular-but-not-calculated drafts), not yet exercised.

## Behavior of `is_meta`

When `is_meta` is true on any node:

- That node is excluded from formula reference resolution.
- The entire descendant subtree of that node is excluded from formula reference resolution (the flag is contagious downward; a regular-flagged descendant inside a meta subtree is still effectively meta because of its ancestor).
- The node's formula text is stored but not bound or evaluated.
- The node does not appear in the engine's dependency graph.
- The node still occupies its position in the parent's child list, but the bind layer skips it during reference resolution, and positional operators on regular siblings (like `@PREV` / `@NEXT` / `@INDEX` / `.*`) skip meta neighbors.

Effective-meta status is computed as `self.is_meta OR any-ancestor.is_meta`. The flag can be set anywhere in a subtree; once set, everything below is meta-effective.

## Canonical uses

**Templates.** A template is a meta-flagged subtree. Its formulas are stored as text patterns; the engine never binds them. The host's instantiation operation reads the template's structure, generates regular (non-meta) content nodes elsewhere with the same shape and formula text, and those instances get bound and evaluated normally. Copied content nodes get `is_meta = false`; template-internal meta children that should remain host data on the rollout, such as `Format`, stay meta. Rollouts may also carry hidden meta-node bookkeeping tags for template id/version/source-node mapping.

**Formatting.** A content node `Foo` may have a child `Format` with `is_meta = true`. The Format meta-node can have its own internal structure (children for `NumberFormat`, `Font`, `Fill`, etc.) — all effectively meta because their ancestor is meta. The host reads format data when rendering. Formulas cannot reach `Foo.Format` from any regular formula. Format-inheritance walks (workspace-level format defaults flowing into sub-trees) are host-level operations. Older notes and mockups sometimes used a double-colon display shorthand for this; treat that shorthand as retired, not syntax or a namespace.

**Future host-data uses** that fit the same pattern: workspace configuration (alias manifest, UI preferences), reusable named-lambda library, annotations, draft formulas, test scenarios. All addressed by direct host access, not formula syntax.

## Engine ask

A single per-node attribute:

```rust
struct NodeAttributes {
    is_meta: bool,    // default false
    hidden: bool,     // host-level UI, default false (auto-true when is_meta is set)
    // ... room for future flags
}
```

Bind-time behavior:
- When resolving a reference, the bind layer skips any node whose effective-meta is true. Failed resolutions produce `Unresolved`.
- Meta-flagged nodes are not bound; their formula text is not parsed for dependency tracking.

Runtime behavior:
- Meta-flagged nodes do not appear in dependency graph.
- Positional operators on regular siblings filter out meta neighbors.

Storage and filter strategy are OxCalc's choice — the spec mandates behavior, not representation.

## Open questions

1. **Reverse: can a regular node be retroactively flagged meta?** Yes, in principle — flipping `is_meta` to true takes the node out of the calculation surface. Any references to it from regular formulas re-bind to `Unresolved`. Standard rebind-on-structural-edit semantics apply.
2. **Role taxonomy.** A free-form `role` string (or closed enum) per meta-node could help the host render them differently (template icon, format gear, draft pencil, etc.). Not engine-relevant; pure host concern. Defer until UI work.
3. **Computed values inside meta-nodes.** Currently impossible — meta-nodes' formulas are not evaluated. If we later want computed format (e.g., color depending on data), the host can invoke the engine on a format expression at render time without putting it in the live dependency graph. That's a future host-side capability; the engine remains unaware.

## What this concept does NOT do (intentional)

- It does not introduce `::` or any other meta-syntax in the formula language.
- It does not reserve a parallel double-colon namespace for format, template, or skin data. Those are ordinary meta-flagged nodes by convention: `Format`, `Templates`, `skins`, etc.
- It does not require two child collections per node — there's just one child list, with a flag per child.
- It does not introduce a "parallel meta-system" for cross-meta references, meta-meta typing, or any richer machinery.
- It does not require OxCalc to model "meta-tree" as a distinct concept beyond honoring the flag.

The intent is **the smallest possible structural mechanism** that lets host-data sit in the workspace tree alongside computational content. Future evolution remains open (the `hidden` / `calc_excluded` slots, plus possible bound-meta-nodes for computed format, etc.), but the v1 footprint is just one boolean.

## Status

Adopted in this simpler form 2026-05-18, replacing the earlier `::` namespace concept which proved over-engineered for the actual needs. The use cases (templates, formatting, etc.) all work with a single `is_meta` flag and host-side direct access.
