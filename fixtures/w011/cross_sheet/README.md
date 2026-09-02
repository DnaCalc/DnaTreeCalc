# cross_sheet — the W011 Wave 3b cross-sheet fixture

Status: committed W011 fixture variant (`dtc-j7n8.14`, Wave 3b). Two sheets,
one cell each, unstyled, pre-populated:

- `Sheet1` (`sheetId="1"`): `A1 = 2` (a literal; the sheet carries no formula).
- `Sheet2` (`sheetId="2"`): `A1 = =Sheet1!A1*5`, a Normal formula stored as
  `<f>Sheet1!A1*5</f>` with cached value `<v>10</v>`.

`<calcPr calcMode="auto"/>`, 1900 date system. Six parts: the five of
`../a1_times_three` plus `xl/worksheets/sheet2.xml` (with its content-type
override and workbook relationship). Still no `xl/styles.xml`, no shared
strings, no calc chain, no drawings.

What the lane proves on this fixture (`workbook.rs`, dtc-j7n8.14):

- open -> `Sheet2!A1` publishes `10` engine-`Calculated` under Automatic: the
  cross-sheet reference EVALUATES on a freshly-loaded workbook (the engine
  prerequisite calc-5kqg.65). `engine_recalcs_at_load == 1`: the counter
  counts DRAINED sheets and the literal-only `Sheet1` is never drained (its
  `A1` publishes `2` `FileCached` at staging) — never assert `== 2`.
- `EnterGridCell Sheet1!A1 = "4"` -> `Sheet2!A1` publishes `20`: cross-sheet
  dirty PROPAGATION on a loaded workbook. At the command surface (`lib.rs`)
  the accepted receipt carries `GridChanged` for `Sheet1` AND `Sheet2`
  (dtc-j7n8.18), so a retained mirror patches the dependent sheet without a
  snapshot.
- save -> reopen RAW through OxDoc, per sheet: `Sheet1!A1 = Number(4)`,
  `Sheet2!A1 = Formula { text: "Sheet1!A1*5", cached: Number(20) }` — the
  formula text preserved and the cached `<v>` on the OTHER sheet refreshed,
  not the file's stale 10.

Scope boundary: single-cell cross-sheet references only. The general
cross-sheet RANGE gap is calc-5kqg.67 (OxCalc); do not extend this fixture
with `Sheet1!A1:A3`-style references.

`parts/` (readable XML) is the source of truth. `../cross_sheet.xlsx` is the
committed binary for the app click-through. Host-core tests zip `parts/` in
memory (`src/dnacalc-host-core/src/xlsx_fixture.rs`,
`w011_cross_sheet_fixture_bytes`) and the acceptance test
`w011_cross_sheet_fixture_opens_through_oxdoc_with_two_sheets` pins the
binary and the parts to the same OxDoc event stream (two `SheetBegin` ..
`SheetEnd` brackets in workbook order, one `FormulaTopology` — `Sheet2`'s).
After editing the parts, regenerate the binary:

    cargo test -p dnacalc-host-core --offline regenerate_w011_cross_sheet_fixture_binary_from_parts -- --ignored

Every constraint in `../a1_times_three/README.md` applies unchanged: existing
unstyled cells only (start tags carry only `r`), no extra parts, 1900 date
system, Normal formulas stored without the leading `=`. Do NOT add cells "for
later": OxDoc's conservative round-trip save rejects cell add/remove.
