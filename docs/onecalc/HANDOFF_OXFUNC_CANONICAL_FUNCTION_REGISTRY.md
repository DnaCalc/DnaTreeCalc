*Posted by Codex agent on behalf of @govert*

# OxFunc Handoff: Canonical, Runtime-Mutable Function Registry

## Architectural principle

There is **one** function list across the OxCalc stack, and it lives in
`OxFunc`. Every other layer (`OxFml`, `OxReplay`, `OxXlPlay`, `OxIde`,
`OxCalc`, `OxVba`, host applications including `DnaOneCalc`) reads from
that registry; nobody re-declares it, mirrors it, or paraphrases it
into a string.

Two consequences of this principle:

1. **No "default function names" lists in hosts.** A host today (e.g.
   `DnaOneCalc/src/dnaonecalc-host/src/adapters/oxfml/live_bridge.rs::DEFAULT_FUNCTION_NAMES`)
   ships a hand-typed list of 67 function names just to make the
   completion popup work. That list must go away — the host should
   ask the registry instead.
2. **The list is runtime-mutable.** UDF registration adds entries.
   Capability tweaks (host says "this RTD provider is unavailable in
   this run") flip availability state on existing entries. Both must
   be expressible without rebuilding the registry from scratch.

The current architecture violates this principle in two places:
- `OxFunc` has the *data* (`FunctionMeta` records, one per function)
  but does not yet expose a *registry API* with iteration, surface-
  name lookup, UDF registration, and capability overlays as first-
  class operations.
- `OxFml` currently consumes function arity through a *string*
  channel (`LibraryContextSnapshotEntry.arity_shape_note`) populated
  by hosts, instead of querying the OxFunc registry. That string-
  channel is the proximate cause of the symptom that triggered this
  handoff (see "Symptom" below).

The OxFml side is captured separately in
`docs/HANDOFF_OXFML_FUNCTION_HELP_FROM_OXFUNC_REGISTRY.md`. This note
is the OxFunc-side ask.

## Symptom that triggered this work

In `DnaOneCalc`, typing `=NOW(` opens a signature-help popup reading:

```
NOW(*arg1, arg2, arg3, additional_args)
```

`NOW` is a zero-argument function; `NOW_META.arity = Arity::exact(0)`
in OxFunc today. The wrong signature is showing because the host's
`LibraryContextSnapshot` hardcodes
`arity_shape_note: Some("variadic".to_string())` for every function
it wants to advertise — there is currently no way for the host to
say "ask OxFunc, you already know."

## What OxFunc has today

Per-function metadata, one constant per function file:

```rust
// crates/oxfunc_core/src/functions/now_fn.rs
pub const NOW_META: FunctionMeta = FunctionMeta {
    function_id: "FUNC.NOW",
    arity: Arity::exact(0),
    determinism: DeterminismClass::TimeDependent,
    volatility: VolatilityClass::VolatileFull,
    host_interaction: HostInteractionClass::ApplicationState,
    thread_safety: ThreadSafetyClass::HostSerialized,
    arg_preparation_profile: ArgPreparationProfile::ValuesOnlyPreAdapter,
    coercion_lift_profile: CoercionLiftProfile::None,
    kernel_signature_class: KernelSignatureClass::Custom,
    fec_dependency_profile: FecDependencyProfile::TimeProvider,
    surface_fec_dependency_profile: FecDependencyProfile::TimeProvider,
};
```

A static catalog accessor at the XLL layer:

```rust
// crates/oxfunc_core/src/xll_export_specs.rs
pub fn function_catalog() -> &'static [FunctionMeta] { ... }
pub fn lookup_function_meta(name_or_id: &str) -> Option<FunctionMeta> { ... }
pub fn lookup_function_meta_by_surface_name(...) -> Option<FunctionMeta> { ... }
pub fn lookup_function_meta_by_id(...) -> Option<FunctionMeta> { ... }
```

The data is all there. What is **missing** is a first-class registry
abstraction with iteration, parameter-name metadata, runtime
mutability, and a capability-overlay model.

## What OxFunc needs to become

### 1. A registry crate-public API

A single module (`oxfunc_core::registry`) exposes the registry as a
type and a default static instance:

```rust
pub struct FunctionRegistry { /* ... */ }

impl FunctionRegistry {
    /// Iterate every entry in registration order. UDFs follow the
    /// built-in catalog.
    pub fn iter(&self) -> impl Iterator<Item = &FunctionEntry>;

    pub fn lookup_by_surface_name(&self, name: &str) -> Option<&FunctionEntry>;
    pub fn lookup_by_id(&self, function_id: &str) -> Option<&FunctionEntry>;

    /// Add a UDF. Returns Err on collision with an existing surface
    /// name, unless the entry is marked as `replaces_builtin = true`.
    pub fn register_udf(&mut self, entry: FunctionEntry) -> Result<(), RegistryError>;

    pub fn unregister_udf(&mut self, function_id: &str) -> Result<(), RegistryError>;

    /// Apply a capability overlay (provider deny-listing, feature
    /// flag gating). Returns a new registry view; the underlying
    /// catalog is not mutated.
    pub fn with_capability_overlay<'a>(
        &'a self,
        overlay: &'a CapabilityOverlay,
    ) -> CapabilityScopedRegistry<'a>;
}

/// Built-in default registry — every function whose `*_META`
/// constant is currently linked into oxfunc_core. Pre-populated at
/// crate init time. Consumers that want pure built-in semantics can
/// use this directly; consumers that need UDFs clone it and mutate.
pub fn builtin_registry() -> &'static FunctionRegistry;
```

### 2. A richer `FunctionEntry` shape

`FunctionMeta` stays as the runtime-execution metadata. `FunctionEntry`
wraps it with the *editor-facing* metadata that today is missing:

```rust
pub struct FunctionEntry {
    pub meta: FunctionMeta,
    pub surface_name: String,                   // "NOW", "SUM", etc.
    pub display_signature: SignatureForm,       // see below
    pub short_description: Option<String>,      // 1-line tooltip text
    pub long_description: Option<String>,       // multi-line help body
    pub source: FunctionSource,                 // BuiltIn | Udf { provenance }
}

pub struct SignatureForm {
    /// Ordered list of parameter descriptors.
    pub parameters: Vec<ParameterDescriptor>,
    /// True when the trailing parameter repeats (variadic).
    pub trailing_repeats: bool,
}

pub struct ParameterDescriptor {
    pub name: String,            // "value", "test", "lookup_value", ...
    pub optional: bool,
    pub repeats: bool,           // for the trailing variadic parameter
    pub short_description: Option<String>,
}
```

Today the editor renders `SUM(arg1, arg2, ...)` because nobody knows
the parameters are called `(value1, [value2], ...)`. Adding
`ParameterDescriptor` fixes that for every function in one place.

**Every function carries real parameter descriptors. No partial
landing.** All ~250 functions currently linked into `oxfunc_core`
must populate `display_signature.parameters` with their canonical
parameter names, optional flags, and trailing-variadic flag before
this work closes. There is no "common functions first, long tail
later" tier and no synthesised `arg1, arg2, …` fallback — those
fallbacks would create a quiet mode where the editor still lies
about parameters for the long-tail functions, and would entrench a
synthesis path in OxFml that someone has to remember to delete
later. Better to do it once, completely.

Practically: every `*_META` constant currently sitting in
`oxfunc_core/src/functions/<fn>.rs` gains a sibling
`*_SIGNATURE: SignatureForm` constant (or the data is folded into
an extended meta), and the registry's built-in init wires both
together. Excel's published parameter-name catalogue is the source
of truth; deviations must be deliberate and documented in the
function file.

### 3. Runtime mutability via UDF registration

UDFs come from two directions:

- **Host registration**, e.g. `DnaOneCalc` hosting an OxVba module
  that exposes `MYUDF(x, y)`. The host calls
  `registry.register_udf(entry)` after acquiring a clone of the
  built-in registry.
- **Capability overlay**, where a function exists in the registry
  but is hidden / rejected for this run because the host cannot
  provide its dependency (e.g. `RTD` with no provider). The
  registry stays whole; the overlay flips state.

The two are deliberately separate. UDF registration *changes the
list*; capability overlays *project a view of the list*.

### 4. Stable, queryable from the wasm32 target

`OxFunc` is consumed from `wasm32-unknown-unknown` builds (DnaOneCalc
publishes a wasm bundle). The registry API must compile cleanly on
wasm: no `std::sync::Mutex` for the hot path (use `OnceLock` /
`LazyLock` with immutable views and copy-on-write for UDFs), no
`std::time` or `std::thread` requirements.

## Migration of existing call sites

After this lands, every consumer that currently mirrors / paraphrases
the function list switches to the registry:

| Site | Today | Goal |
|---|---|---|
| `DnaOneCalc/src/dnaonecalc-host/src/adapters/oxfml/live_bridge.rs::DEFAULT_FUNCTION_NAMES` | hand-typed `&[&str]` of 67 names | deleted; the host iterates `OxFunc::builtin_registry()` and forwards entries to OxFml |
| `DnaOneCalc/.../default_function_library_snapshot()` | hand-builds `LibraryContextSnapshot` with `arity_shape_note: "variadic"` for every entry | deleted entirely; OxFml reads from registry directly |
| `OxFml::semantics::lookup_function_meta` | already calls `oxfunc_core::xll_export_specs::lookup_function_meta` | switches to `oxfunc_core::registry::builtin_registry().lookup_by_surface_name(...)`; same data, plus parameter names |
| `OxFml::consumer::editor::build_function_help_packet` | reads `LibraryContextSnapshotEntry.arity_shape_note` string | reads parameter list directly from `FunctionEntry.display_signature`; covered by `HANDOFF_OXFML_FUNCTION_HELP_FROM_OXFUNC_REGISTRY.md` |
| `OxReplay`, `OxXlPlay`, `OxIde`, `OxCalc`, `OxVba` | various ad-hoc consumers | follow the same migration; out of scope for this note but tracked once OxFunc and OxFml land |

## Test coverage

Within `oxfunc_core`:

1. `builtin_registry().iter().count()` equals the number of `*_META`
   constants linked in (catches drift if a new function is added but
   not registered).
2. Round-trip: every entry's `surface_name` resolves back to the same
   entry via `lookup_by_surface_name` and via `lookup_by_id`.
3. UDF registration succeeds, then `iter()` includes the UDF after
   the built-ins; `unregister_udf` removes it; double-register on
   the same id without `replaces_builtin` errors with `RegistryError::Collision`.
4. Capability overlay on a built-in function flips its
   availability state in the *view* without mutating the underlying
   registry (a second overlay on the same registry sees the
   un-flipped baseline).
5. `wasm32-unknown-unknown` build of the registry compiles and the
   four tests above pass under `wasm-bindgen-test`.

## Why this is worth doing now

Today the symptom is one popup showing wrong signature text. The
cost of the workaround (have the host populate the right `arity_shape_note`
strings from the OxFunc catalog) is small — but it would entrench
the wrong architecture: hosts owning function-list mirrors,
serialised through string fields, with parameter names still
synthesised from arity. Any future surface that wants to render
function help (the formula drill-down, the seam-board, the OxIde
language service, OxCalc's calc engine) would either bend through
the same string channel or hand-roll a third copy.

The registry is the right shape now. It also unblocks the UDF story
(which is unavoidable for OxVba host integration) and the capability
story (which `DnaOneCalc` already needs for `SEAM-OXFUNC-LOCALE-EXPAND`,
`SEAM-OXFUNC-FORMAT-*`, the RTD-provider gating, and a dozen other
places).

## Closure conditions

- `oxfunc_core::registry` is a public module with the API sketched
  above.
- **Every function** linked into `oxfunc_core` (today: ~250 entries
  under `crates/oxfunc_core/src/functions/`) carries a
  `FunctionEntry.display_signature` with canonical parameter names,
  optional flags, and the trailing-variadic flag where applicable.
  No synthesised `arg1, arg2, …` fallback is left in the codebase.
  A test enforces this: every entry returned from
  `builtin_registry().iter()` has `display_signature.parameters`
  consistent with its `meta.arity` (parameter count covers
  `arity.min`, optional flags cover the gap to `arity.max`,
  trailing repeats covers unbounded `max`).
- UDF registration / unregistration round-trips with explicit
  collision handling.
- Capability overlay produces a `CapabilityScopedRegistry` view that
  the wasm-target editor adapter can consume.
- `oxfunc_core` lib-tests pass on host and on `wasm32-unknown-unknown`.
