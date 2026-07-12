*Posted by Codex agent on behalf of @govert*

# Handoff: Excel Oracle Harness For VBA UDF Type Behavior

Date: 2026-05-16
Source repo: DnaOneCalc
Target repos: OxXlPlay, OxReplay

## Summary

DnaOneCalc needs an Excel-backed oracle harness for VBA UDF invocation and type
coercion behavior. The goal is to compare DnaOneCalc + OxVba UDF execution
against real Excel VBA UDF execution through retained OxXlPlay observations and
OxReplay comparison views.

## Symptom In DnaOneCalc

The DnaOneCalc VBA integration cannot safely widen beyond a tiny scalar UDF
slice until it knows how Excel handles the same VBA UDF signatures and worksheet
formula arguments. Existing OxXlPlay OneCalc observation families cover direct
cell values, formula text, and some SpreadsheetML formatting/display surfaces,
but they do not yet provide a VBA-project/UDF oracle family.

## Requested OxXlPlay Expansion

Add a Windows-only live Excel capture family that can:
1. create/open a workbook containing VBA module code,
2. insert public VBA UDFs into a standard module,
3. write formulas that call those UDFs with controlled argument cells/literals,
4. calculate Excel,
5. capture:
   - formula text,
   - raw cell value,
   - error state,
   - displayed text,
   - argument cell values where relevant,
   - VBA project/module/function identity,
   - declared VBA signature metadata when available,
   - Excel build/provenance and macro/security state,
   - capture-loss markers.
6. retain the workbook and observation bundle with replay-ready sidecars.

## Requested OxReplay Expansion

Add comparison-view support for VBA UDF oracle cases, tentatively:
1. `vba_udf_signature`
2. `vba_udf_argument_values`
3. `vba_udf_result_value`
4. `vba_udf_error_state`
5. `vba_udf_display_text`
6. `vba_udf_coercion_observation`

The comparison view should make projection gaps explicit. If the Excel side can
observe a behavior but DnaOneCalc/OxVba cannot yet represent it, the result
should be a coverage/projection gap rather than a false semantic mismatch.

## Minimal First Matrix

Start with a standard-module function:

```vb
Public Function AddThem(val1 As Double, val2 As Double) As Double
    AddThem = val1 + val2
End Function
```

Initial worksheet formulas:
1. `=AddThem(2,3)`
2. `=AddThem(TRUE,3)`
3. `=AddThem(FALSE,3)`
4. `=AddThem("2",3)`
5. `=AddThem("",3)`
6. `=AddThem(A1,3)` where `A1` is blank
7. `=AddThem(A1,3)` where `A1` contains an Excel error

Then add typed signature variants for `Integer`, `Long`, `Currency`, `String`,
`Boolean`, and `Variant` once the first retained bundle shape is stable.

## What Not To Do

Do not make DnaOneCalc infer Excel VBA UDF coercion behavior from ordinary
worksheet function coercion. The point of this harness is to observe the VBA UDF
call boundary directly.

## Coordination Checklist

1. OxXlPlay adds a retained VBA UDF observation family.
2. OxReplay accepts the retained family through declared comparison views.
3. DnaOneCalc adds matching local cases that run the same UDF signatures through
   OxVba.
4. Diff/explain output distinguishes value mismatch, error mismatch, display
   mismatch, and projection coverage gap.
5. The first admitted DnaOneCalc VBA UDF type matrix rows cite retained Excel
   oracle evidence.
