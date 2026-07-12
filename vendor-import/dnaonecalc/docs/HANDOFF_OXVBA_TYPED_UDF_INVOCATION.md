*Posted by Codex agent on behalf of @govert*

# Handoff: OxVba Typed UDF Invocation Contract

Date: 2026-05-16
Source repo: DnaOneCalc
Target repo: OxVba

## Summary

DnaOneCalc wants to integrate OxVba-hosted VBA UDFs as formula-callable
functions, but the invocation boundary should be typed from the start and
validated against Excel-observed VBA UDF behavior. The current OxVba
`HostUdfCatalog` / `invoke_host_udf_with_variants` surface is a useful start,
but may be too weak for exact Excel-compatible coercion, error, array, object,
optional, `Variant`, and ByRef behavior.

## Symptom In DnaOneCalc

DnaOneCalc must bridge three type systems:
1. OxFml formula argument/result surfaces,
2. OxFunc value and error carriers,
3. OxVba VBA parameter and return types.

If DnaOneCalc treats the current OxVba host-UDF invocation as only a generic
`Variant` call, it risks inventing a private coercion layer that diverges from
Excel. The intended product target for this lane is exact Excel behavior for VBA
UDF worksheet calls.

## Current Upstream Reading

From the current OxVba source and worksets:
1. `HostUdfCatalog` exposes public procedural functions and basic descriptors.
2. `HostUdfCallContext` carries caller, locale, dependency tokens, and volatile
   request shape.
3. `Engine::invoke_host_udf_with_variants` invokes by stable host-call id.
4. `WORKSET_2026-05-10_HOST_PROGRAM_DESIGN_AND_UDF_REWORK.md` notes that the
   host-UDF call frame/context path still needs refinement and that descriptor
   source-of-truth and function-only behavior remain design questions.

## Requested OxVba Surface

Please consider a typed host-UDF invocation contract that exposes:
1. canonical VBA UDF signature descriptors:
   - module/function identity,
   - public formula-visible name,
   - parameter names,
   - declared VBA types,
   - optional/default information,
   - ByVal/ByRef,
   - `ParamArray`,
   - return type,
   - whether the procedure is function-only and procedural-module scoped.
2. typed host-call arguments that can represent Excel worksheet-call inputs
   before lossy conversion into generic `Variant`.
3. typed invocation results that distinguish:
   - VBA scalar values,
   - Excel/VBA error results,
   - `Empty` / `Null`,
   - arrays/ranges when admitted,
   - object values when admitted,
   - conversion/runtime diagnostics.
4. a context path that actual VBA runtime execution can observe for
   `Application.Caller`, volatile behavior, dependency registration, and
   worksheet-call policy where supported.
5. a way to state which mappings are implemented/evidenced versus rejected or
   provisional.

## Minimal Reproduction Path

Use a `.basproj` `HostModule` project with:

```vb
Public Function AddThem(val1 As Double, val2 As Double) As Double
    AddThem = val1 + val2
End Function
```

Then exercise:
1. `=AddThem(2,3)`
2. `=AddThem(TRUE,3)`
3. `=AddThem("2",3)`
4. `=AddThem("",3)`
5. `=AddThem(#DIV/0!,3)`

The target behavior for each row should come from Excel-observed VBA UDF
execution, not from DnaOneCalc guesswork.

## What Not To Do

DnaOneCalc should not paper over this by creating a broad local coercion table
that OxVba cannot represent or test. That would hide the actual engine/host
contract and make later Excel parity work harder to verify.

## Coordination Checklist

1. OxVba defines or refines the typed host-UDF signature and invocation contract.
2. OxVba tests the contract at least for scalar Excel-style UDF calls.
3. DnaOneCalc updates its `vba_host` adapter to consume the typed contract.
4. OxXlPlay/OxReplay retained Excel oracle cases are linked to the same signature
   ids and comparison-view families.
5. DnaOneCalc removes any provisional local conversion assumptions once the
   OxVba contract lands.
