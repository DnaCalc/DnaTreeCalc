*Posted by Codex agent on behalf of @govert*

# OxFunc Handoff: Signature seeds with empty `parameters` lose argument help in editor surfaces

Status: filed
Direction: DnaOneCalc → OxFunc
Source repo / workset: DnaOneCalc / Editor regression report
Filed date: 2026-05-05
Related:
  `OxFunc/crates/oxfunc_core/src/registry_signature_seed.rs`,
  `OxFunc/crates/oxfunc_core/src/registry.rs::signature_from_seed`,
  `OxFml/crates/oxfml_core/src/consumer/editor/mod.rs::argument_help_from_signature`.

## Symptom on DnaOneCalc

After typing `=SUMIF(` (or `=COUNTIFS(`, `=DATEDIF(`, `=VLOOKUP(`,
…) into the home-shell editor, the signature-help popup renders
just `SUMIF(...)` with no parameter labels, and the function-help
card shows the same empty `(...)` placeholder. For functions like
`=SUM(` the parameter list is correct (`number1, [number2], …`).

## Root cause in OxFunc

`registry_signature_seed::SIGNATURE_SEEDS` carries 530 built-in
function entries. **244 of them** have:

```rust
SignatureSeed {
    function_id: "FUNC.SUMIF",
    signature_display: "SUMIF(...)",
    parameters: &[],
    trailing_repeats: true,
},
```

— a placeholder `(...)` `signature_display` and an empty
`parameters` array. `signature_from_seed` projects `parameters` 1:1
into `SignatureForm.parameters`, and OxFml's
`argument_help_from_signature` derives the editor's
`argument_help: Vec<String>` from those `parameters`. With an empty
seed, `argument_help` is empty, the host renders zero parameter
labels, and the user loses the per-argument hint that's the whole
point of the popup.

A representative slice of affected built-ins (alphabetic from one
grep window): `ASC`, `CALL`, `COUNTIFS`, `COUPDAYBS`, `COUPDAYS`,
`COUPDAYSNC`, `COUPNCD`, `COUPNUM`, `COUPPCD`, `COVAR`, `CRITBINOM`,
`CUMIPMT`, `CUMPRINC`, `DATEDIF`, `DATEVALUE`, `DAVERAGE`, `DAY`,
`DAYS`, `DAYS360`, `DB`, `DBCS`, `DCOUNT`, `DCOUNTA`, `DDB`, `DGET`,
`DISC`, `DMAX`, `DMIN`, `DOLLARFR`, `DPRODUCT`, … through
`SUMIF` / `SUMIFS` / `SUMPRODUCT` / `SUMX2MY2` / `SUMX2PY2` /
`SUMXMY2` / `SWITCH` / many more.

## Concrete ask

1. Author named-parameter seeds for the 244 entries that currently
   ship `parameters: &[]`. Each replacement looks like, e.g.:

   ```rust
   SignatureSeed {
       function_id: "FUNC.SUMIF",
       signature_display: "SUMIF(range, criteria, [sum_range])",
       parameters: &[
           ParameterSeed { name: "range",     optional: false, repeats: false },
           ParameterSeed { name: "criteria",  optional: false, repeats: false },
           ParameterSeed { name: "sum_range", optional: true,  repeats: false },
       ],
       trailing_repeats: false,
   },
   ```

2. Add a build-time / compile-time guard in `registry.rs` that
   refuses any `SIGNATURE_SEEDS` entry whose `signature_display` is
   the literal `(...)` placeholder shape (matches `^[A-Z0-9_.]+\(\.\.\.\)$`).
   That keeps future additions from regressing back into the
   placeholder shape, and makes the gap impossible to ship
   silently.

3. Pin a registry test that asserts every `FunctionEntry` returned
   by `builtin_registry()` has `display_signature.parameters.len()
   >= 1` whenever its `meta.arity.min >= 1` — the case where "the
   function takes at least one argument" should never project as
   "we have no labels for any argument".

## DnaOneCalc-side impact

Once the seeds land, no host change is needed:
`argument_help_from_signature` already projects them through to
`FunctionHelpPacket.argument_help`, which the host renders directly
in `SignatureHelpView.parameters` and the function-help card.

Until then DnaOneCalc carries no SEAM stub for this — the popup
silently degrades to "callee name only", which is the existing
behaviour for any function the registry doesn't know.

## Suggested split

The 244-entry batch is large enough to justify slicing. A natural
slice order is by Excel-marketshare-of-use (statistical and
financial families first — `SUMIF` / `COUNTIF` / `VLOOKUP` /
`INDEX` / `MATCH` / `IF` / `IFS` / `SWITCH` / `LOOKUP`, then date
families, then the long tail). Each slice is mechanical authoring
plus the registry test.
