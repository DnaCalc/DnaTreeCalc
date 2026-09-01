# a1_times_three — the W011 fixture

Status: committed W011 fixture (`dtc-j7n8.2`). One sheet `Sheet1`; `A1 = 7`;
`B1 = =A1*3` with cached value `21`. Nothing else.

`parts/` (readable XML) is the source of truth. `../a1_times_three.xlsx` is
the committed binary for the app click-through. Host-core tests never read the
binary as their byte source: `src/dnacalc-host-core/src/xlsx_fixture.rs` zips
`parts/` in memory through `oxdoc_conformance::read_fixture_parts_as_xlsx`
(OxDoc's own committed-parts pattern; no zip crate in this repo), and the
acceptance test `w011_fixture_opens_through_oxdoc_with_two_cells` pins the
binary and the parts to the same OxDoc event stream. After editing the parts,
regenerate the binary:

    cargo test -p dnacalc-host-core --offline regenerate_w011_fixture_binary_from_parts -- --ignored

Constraints — OxDoc's conservative round-trip save (the W011 save/reopen with
cached `B1 = 30`) depends on every one of them, so do not "improve" the
fixture:

- Both cells already exist and their start tags carry only `r`. OxDoc rejects
  cell add/remove, formula add/remove, and value edits to cells whose start
  tag carries any attribute beyond `r`/`t`; the formula forces the surgical
  save path, so every cell the W011 edit touches must already be present and
  unstyled.
- Exactly five parts: no `xl/styles.xml`, no `xl/sharedStrings.xml`, no
  `xl/calcChain.xml`, no drawings. A dangling override or relationship is a
  broken package.
- 1900 date system (no `<workbookPr date1904>`); `<calcPr calcMode="auto"/>`
  (`calcMode` is the only `calcPr` attribute OxDoc reads).
- `B1` is a Normal formula (no `t="shared"`/`"array"`, no `si`/`ref`); the
  text is stored without the leading `=` (xlsx convention).

Trimmed from OxDoc's `crates/oxdoc-conformance/fixtures/minimal-values` parts.
