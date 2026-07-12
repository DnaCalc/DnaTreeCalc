*Posted by Codex agent on behalf of @govert*

# Handoff: OxFml/OxFunc Random Provider API Mismatch

## Symptom Observed In DnaOneCalc

While checking the DnaOneCalc VBA UDF browser-selection path, native validation now fails before DnaOneCalc code is reached.

Commands run from `C:\Work\DnaCalc\DnaOneCalc`:

```powershell
cargo check -p dnaonecalc-host
cargo test -p dnaonecalc-host view_model_projects_browser_vba_file_without_raw_source_prefix
```

Both fail in sibling repos with random-provider API mismatches.

## Compile Errors

`OxFunc`:

```text
C:\Work\DnaCalc\OxFunc\crates\oxfunc_core\src\function_call.rs:314:13
expected Option<&dyn RandomProvider>, found Option<f64>
```

`OxFml`:

```text
C:\Work\DnaCalc\OxFml\crates\oxfml_core\src\eval\mod.rs:1692:17
expected Option<&dyn oxfunc_core::functions::rand_fn::RandomProvider>, found Option<f64>
```

`OxFml` also writes the function execution context bundle's `random_provider` in several places, but the current `FunctionExecutionContextBundle` fields are now:

```text
resolver, now_serial, now_provider, random_provider, locale_ctx, ...
```

The missing field errors were reported at:

```text
OxFml/crates/oxfml_core/src/eval/mod.rs:2328
OxFml/crates/oxfml_core/src/eval/mod.rs:2350
OxFml/crates/oxfml_core/src/eval/mod.rs:5182
OxFml/crates/oxfml_core/src/eval/mod.rs:5371
```

## Likely Root Cause

`OxFunc` appears to have moved the random-number contract from a concrete `Option<f64>` seed/value to a `RandomProvider` trait object. Some `OxFunc` and `OxFml` call sites still pass or store the old `Option<f64>` shape.

This blocks DnaOneCalc validation because `dnaonecalc-host` depends on both crates.

## Proposed Upstream Change

Align `OxFunc` and `OxFml` on the new random-provider contract:

1. Replace remaining `Option<f64>` dispatch arguments with `Option<&dyn RandomProvider>`.
2. Replace stale function execution context random-provider assignments with the current `random_provider` field or an adapter that preserves deterministic seeded behavior.
3. Keep DnaOneCalc deterministic/live recalc semantics intact when `LiveOxfmlBridge` passes its scenario random seed through `TypedContextQueryBundle`.

## Minimal Reproduction

From `C:\Work\DnaCalc\DnaOneCalc`:

```powershell
cargo check -p dnaonecalc-host
```

Expected: host check reaches DnaOneCalc code and succeeds or fails on host changes.

Actual: compile stops in `OxFunc`/`OxFml` with the random-provider mismatch above.
