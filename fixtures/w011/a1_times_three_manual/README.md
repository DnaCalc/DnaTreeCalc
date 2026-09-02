# a1_times_three_manual — the W011 Manual calc-mode twin

Status: committed W011 fixture variant (`dtc-j7n8.13`, Wave 3a). Byte-for-byte
the `../a1_times_three` fixture — one sheet `Sheet1`; `A1 = 7`; `B1 = =A1*3`
with cached value `21`; five parts — except for exactly one attribute in
`parts/xl/workbook.xml`:

    <calcPr calcMode="manual"/>

That one attribute changes what the host does on open. OxDoc reads `calcMode`
into `DocumentEvent::WorkbookHeader`, and the engine's `load_workbook_model`
takes the **Manual** recalc path (`LoadRecalcPath::Manual`): it binds `B1`
but runs **zero** engine passes (`engine_recalcs_at_load == 0`), so the
workbook renders from the file's caches — `B1` publishes `21` with provenance
`FileCached` — until an explicit `Recalculate` (F9). The `auto` twin runs
Excel's open-recalc instead and never shows `FileCached`.

`parts/` (readable XML) is the source of truth. `../a1_times_three_manual.xlsx`
is the committed binary for the app click-through. Host-core tests zip
`parts/` in memory (`src/dnacalc-host-core/src/xlsx_fixture.rs`,
`w011_manual_fixture_bytes`) and the acceptance test
`w011_manual_fixture_is_the_auto_twin_with_calc_mode_manual` pins the binary
and the parts to the same OxDoc event stream, and pins that stream to the
`auto` twin's stream with only the header's calc mode differing. After editing
the parts, regenerate the binary:

    cargo test -p dnacalc-host-core --offline regenerate_w011_manual_fixture_binary_from_parts -- --ignored

Every constraint in `../a1_times_three/README.md` applies unchanged (existing
unstyled cells only, exactly five parts, 1900 date system, Normal formula
stored without the leading `=`). Do NOT mix modes: this fixture is Manual and
nothing else; the Wave 1 slice asserts nothing against it.

What the save lane proves on this fixture (Excel's last-calculated semantics,
`manual_mode_save_before_recalc_writes_last_calculated_cache` in
`workbook.rs`): a Manual session saved after `A1 -> 10` but BEFORE
`Recalculate` writes `A1 = 10` with `B1`'s cached `<v>` still `21` — the
last calculated value, exactly what Excel writes for a manual workbook saved
without F9. After `Recalculate` the save writes cached `30`, and reopening
those bytes under Manual renders `30` `FileCached` with zero engine passes —
the cached-30 reopen the campaign is for, with no engine pass to mask it.

Two engine facts the tests pin as observed (follow-up `dtc-j7n8.24`): a
Manual load publishes `FileCached` values for formula caches only, so the
literal `A1` has no published value (and no projected cell) until the first
F9 even though its authored text `7` reads back; and the load seeds the sheet
dirty, so `workbook_calc` reports it dirty straight after the open.
