# named_input — the W011 Wave 3c defined-name fixture

Status: committed W011 fixture variant (`dtc-j7n8.15`, Wave 3c). One sheet
`Sheet1`, two unstyled cells, one workbook-scoped defined name:

- `A1 = 7` (a literal).
- `D1 = =TheInput*2`, a Normal formula stored as `<f>TheInput*2</f>` with
  cached value `<v>14</v>`.
- `TheInput -> Sheet1!$A$1`: `<definedName name="TheInput">Sheet1!$A$1</definedName>`
  in `xl/workbook.xml`'s `<definedNames>`, no `localSheetId` (workbook
  scope), no metadata attributes.

`<calcPr calcMode="auto"/>`, 1900 date system. Exactly the five parts of
`../a1_times_three` — the name lives inside the workbook part. Still no
`xl/styles.xml`, no shared strings, no calc chain, no drawings.

What the lane proves on this fixture (`workbook.rs` + `lib.rs`, dtc-j7n8.15):

- open -> the loaded name SEEDS through the engine's ingest (OxCalc resolves
  `Sheet1!$A$1` to a static rect and authors the name on `Sheet1`'s grid; the
  load report carries no `name:TheInput` bind degradation); `D1` publishes
  `14` engine-`Calculated` — resolved THROUGH the name, never `#NAME?`;
  `defined_names()` (and the mount snapshot) lists exactly `TheInput` at
  workbook scope targeting the static rect `A1`.
- `EnterGridCell A1 = "10"` -> `D1` publishes `20`: the edit recalculates
  through the name. The catalog is unchanged by a cell edit.
- save -> reopen RAW through OxDoc: the `DefinedName` event is present and
  identical to the file's, still in the prelude before `SheetBegin`; `A1 =
  Number(10)`; `D1 = Formula { text: "TheInput*2", cached: Number(20) }` —
  the formula text preserved and the cached `<v>` refreshed, not the file's
  stale 14.

The name is never hand-seeded in the host. Names enter the engine only from
the file's `DefinedName` event through `load_workbook_model`; if a loaded
name ever stops seeding, `open_named_input_fixture_seeds_the_name_and_publishes_14`
fails and documents the typed ingest gap for an OxCalc handover.

Constraint specific to this fixture: the name's text is stored exactly as
OxCalc re-renders a static name on projection — absolute and sheet-qualified,
`Sheet1!$A$1`. OxDoc's round-trip save refuses a defined-name name/text/scope
change with a typed `UnsupportedRoundTripFeature`, so a sheet-qualified rect
spelled differently in the source (`Sheet1!A1`, relative anchors) would be
re-rendered as `Sheet1!$A$1` on projection and turn every save of this
fixture into a refusal. (An UNQUALIFIED text such as `$A$1` is not a rect to
OxCalc's ingest — it takes the dynamic lane and is re-emitted verbatim — so
keep the qualified static form; that is what this lane proves.) Do not add a
second name, a `localSheetId`, or metadata attributes "for later":
adding/removing names during round-trip is also refused.

Scope boundary: one workbook-scoped static name. No names-manager UI (that
is dtc-ajl.25, parked, out of campaign); no sheet-scoped or dynamic names on
this fixture.

`parts/` (readable XML) is the source of truth. `../named_input.xlsx` is the
committed binary for the app click-through. Host-core tests zip `parts/` in
memory (`src/dnacalc-host-core/src/xlsx_fixture.rs`,
`w011_named_input_fixture_bytes`) and the acceptance test
`w011_named_input_fixture_opens_through_oxdoc_with_a_defined_name` pins the
binary and the parts to the same OxDoc event stream (one `DefinedName` before
the one `SheetBegin`, two cells, one `FormulaTopology` record — `D1`'s).
After editing the parts, regenerate the binary:

    cargo test -p dnacalc-host-core --offline regenerate_w011_named_input_fixture_binary_from_parts -- --ignored

Every constraint in `../a1_times_three/README.md` applies unchanged: existing
unstyled cells only (start tags carry only `r`), no extra parts, 1900 date
system, Normal formulas stored without the leading `=`. Do NOT add cells "for
later": OxDoc's conservative round-trip save rejects cell add/remove.
