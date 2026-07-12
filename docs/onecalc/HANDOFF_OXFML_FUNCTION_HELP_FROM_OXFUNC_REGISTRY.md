*Posted by Codex agent on behalf of @govert*

# OxFml Handoff: Editor Function Help Reads From the OxFunc Registry, Not From a Host-Supplied Snapshot String

## Architectural principle

The function list — its members, arities, parameter names,
descriptions, and source (built-in vs. UDF) — is owned by `OxFunc`.
`OxFml` reads from that registry. `OxFml` does **not** carry a
parallel function list, does **not** ask the host to paraphrase one
through string fields, and does **not** synthesise signatures from
arity counts.

The OxFunc-side ask (the registry itself, with parameter-name
metadata and runtime mutability) is captured in
`docs/HANDOFF_OXFUNC_CANONICAL_FUNCTION_REGISTRY.md`. **This note
depends on that one** — it can be drafted now, but
implementation must wait until `oxfunc_core::registry` exists.

## Symptom that triggered this work

In `DnaOneCalc`, typing `=NOW(` opens a signature-help popup reading:

```
NOW(*arg1, arg2, arg3, additional_args)
```

`NOW_META.arity = Arity::exact(0)` in OxFunc. The wrong signature is
showing because OxFml's `build_function_help_packet` reads arity from
`LibraryContextSnapshotEntry.arity_shape_note: Option<String>`, which
the host (`DnaOneCalc/src/dnaonecalc-host/src/adapters/oxfml/live_bridge.rs::default_function_library_snapshot`)
hardcodes to `"variadic"` for every function it advertises. The
fault is *not* the host's wrong string; the fault is OxFml accepting
arity through a string-typed channel from the host at all when the
real data lives in OxFunc.

## What OxFml does today

Two relevant paths:

### Path A — `OxFml::semantics::lookup_function_meta`

```rust
// crates/oxfml_core/src/semantics/mod.rs
pub fn lookup_function_meta(function_name: &str) -> Option<FunctionMeta> {
    lookup_oxfunc_function_meta(function_name)
}
```

This already delegates to `oxfunc_core::xll_export_specs::lookup_function_meta`.
This path is correct; it gets us the *runtime* metadata (arity,
determinism, threading) for binding and semantic planning.

### Path B — `OxFml::consumer::editor::build_function_help_packet`

```rust
// crates/oxfml_core/src/consumer/editor/mod.rs
let (min_arity, max_arity, signature_suffix) = snapshot_entry
    .as_ref()
    .and_then(|entry| entry.arity_shape_note.as_deref())
    .map(parse_arity_shape_note)
    .unwrap_or((0, None, "...".to_string()));
// ...
argument_help: build_argument_help(min_arity, max_arity, signature_help_context),
```

`parse_arity_shape_note` understands strings like `"variadic"`,
`"3+"`, `"2..3"`, `"0"`. `build_argument_help` then synthesises
`arg1, arg2, …` and prepends `*` to the active argument.

This path is the bug. It pulls arity out of a host-supplied string
and synthesises parameter names from arity counts because no real
parameter names are available. Both should come from the OxFunc
registry.

## What OxFml needs to become

### 1. Editor help packet reads from the OxFunc registry

`build_function_help_packet` (and any peer that needs arity /
parameter names for editor surfaces) calls into the OxFunc registry
directly:

```rust
let entry = registry.lookup_by_surface_name(callee);
//          ^ &FunctionRegistry handed in via EditorEnvironment
//            (see "Plumbing" below). Built-in or UDF-augmented;
//            either way, the registry is authoritative.
```

When `entry` is `Some`, `FunctionHelpPacket` is built from
`entry.surface_name` and `entry.display_signature.parameters`
directly — no `signature_suffix`, no `build_argument_help`, no
synthesis from arity. When `entry` is `None`, no help packet is
built at all: the editor surfaces nothing for an unresolved callee
(the binder has already flagged the name as unknown through its own
diagnostic channel; the editor does not need to compensate by
inventing a signature).

There is **no fallback** that synthesises `arg1, arg2, …` names
from arity counts. The OxFunc registry — per
`HANDOFF_OXFUNC_CANONICAL_FUNCTION_REGISTRY.md` — guarantees that
every function it contains carries real parameter descriptors. If
the registry has the entry, OxFml has the names. If the registry
does not have the entry, OxFml shows nothing. Those are the only
two states.

### 2. Retire the string arity channel from `LibraryContextSnapshotEntry`

`LibraryContextSnapshotEntry.arity_shape_note: Option<String>` exists
solely to feed `parse_arity_shape_note`. With the OxFunc registry
landed, that field is dead weight that mostly carries wrong values.

Two options at the API level:

- **Hard removal** (cleaner, but breaking). Drop the field and the
  parser; every consumer is forced to migrate.
- **Deprecate then remove** (kinder during migration). Mark the
  field `#[deprecated]` with a pointer to
  `oxfunc_core::registry`; OxFml ignores it in
  `build_function_help_packet`; remove in a follow-up.

Either is fine; pick whichever fits OxFml's release discipline.

### 3. The snapshot keeps its real job

`LibraryContextSnapshot` is **not** retired. It still carries
host-specific information that the OxFunc registry cannot know about:

- **Capability state per function** for *this run*: e.g.
  `LibraryAvailabilityState::CapabilityDenied` for `RTD` because
  the host didn't wire an RTD provider, or `Deferred` for a
  function gated behind a feature flag.
- **UDF-side identity** when the host has registered UDFs into a
  cloned `FunctionRegistry` — the snapshot can carry the UDF entries
  the host wants admitted for this run.
- **Source attestation** — `registration_source_kind`,
  `metadata_status`, etc. — that documents *why* this run admits
  what it admits. These remain.

So the snapshot becomes a thin overlay describing host-specific
admission decisions, not a reduplicated function catalog.

### 4. Plumbing — how does OxFml see the registry?

Two reasonable shapes:

**Shape A: implicit static reference.** `OxFml` calls
`oxfunc_core::registry::builtin_registry()` directly. Simplest;
works as long as no UDFs are involved.

**Shape B: registry handle on `EditorEnvironment`.** `OxFml`'s
editor environment grows a `&FunctionRegistry` (or
`&CapabilityScopedRegistry`) field. The host hands it in along with
the rest of the environment. UDF-aware hosts pass a registry that
includes their UDFs; pure built-in hosts pass `builtin_registry()`.

Shape B is the correct long-term answer (UDFs have to flow somehow);
Shape A is acceptable for the first pass while UDF wiring is being
designed.

## DnaOneCalc-side impact (informational)

After both this OxFml change and the OxFunc registry land,
DnaOneCalc deletes:

- `DEFAULT_FUNCTION_NAMES` constant (67 hand-typed names).
- `default_function_library_snapshot()` builder.
- The `InMemoryLibraryContextProvider` wiring whose only job was to
  feed those entries to OxFml.

DnaOneCalc replaces them with:

- One call to obtain the OxFunc registry (or a host-augmented clone
  of it once UDFs land).
- A thin `LibraryContextSnapshot` carrying *only* host capability
  decisions (today: nothing; after `SEAM-OXFUNC-LOCALE-EXPAND`,
  `SEAM-OXFML-PARTIAL-EVAL`, etc. land: a few entries).

This DnaOneCalc-side cleanup is **not** done in this handoff. It
follows once OxFunc and OxFml are green, and is tracked under the
existing `SEAM-ONECALC-LIBRARY-CONTEXT-FROM-OXFUNC-CATALOG` seam id.

## Test coverage

In `oxfml_core`:

1. `=NOW(` at the open paren → `FunctionHelpPacket.signature_forms[0].display_signature == "NOW()"`,
   `argument_help == []`.
2. `=SUM(` → `display_signature` matches `SUM(value1, [value2], ...)`
   exactly (or whatever the canonical parameter names settle on
   in the OxFunc registry); `argument_help[0] == "value1"` (or with
   `*` prefix when active).
3. `=IF(test, ` (caret in second arg) →
   `argument_help == ["test", "*value_if_true", "value_if_false"]`.
4. Unknown function `=ZZZNOTAFUNCTION(` →
   no `FunctionHelpPacket` is produced. The editor surface is empty.
   The binder's own `unknown_function` diagnostic is the only
   signal the user sees; no `(*arg1, …)` synthesis path exists in
   the codebase.
5. UDF registered through the registry → its signature renders the
   same way as a built-in's, with the host-supplied parameter names.
6. **Synthesis-path absence test.** A grep / structural test in
   `oxfml_core` asserts that the strings `"arg1"`, `"additional_args"`,
   and the helpers `parse_arity_shape_note`, `signature_suffix`,
   and `build_argument_help` are not present anywhere in the editor
   builder hot path (or only in retired test fixtures). This catches
   the synthesis sneaking back in during a future refactor.

## Why depend on the OxFunc note

Without the OxFunc registry carrying parameter-name metadata,
`build_function_help_packet` would still have to synthesise names.
The whole point is to stop synthesising — so OxFunc must land first
(or the two land together). This note is implementable the moment
`oxfunc_core::registry` exists with `FunctionEntry.display_signature`
populated for the initial set of common functions.

## Closure conditions

- `OxFml::consumer::editor::build_function_help_packet` reads
  `display_signature` and parameter names directly from the OxFunc
  registry handle in its environment.
- `LibraryContextSnapshotEntry.arity_shape_note` is removed (not
  deprecated). The string-typed arity channel does not survive this
  change.
- `parse_arity_shape_note`, `signature_suffix`, and
  `build_argument_help` are deleted entirely from `oxfml_core`.
  No "fallback" synthesis path remains in the source tree.
- The six tests above pass — including test 6, the structural
  no-synthesis assertion.
- A short note in `OxFml/docs/` records the architectural rule
  ("function metadata flows OxFunc → OxFml, never host →
  OxFml; OxFml does not synthesise function metadata it lacks")
  so the regression doesn't re-emerge.
