# false_green — the W012 FileCached-pin fixture

Status: committed W012 fixture (`dtc-5zet.2`; re-cut by `dtc-5zet.20` on
2026-09-15). One sheet `Sheet1`:

| cell | file content | what the engine does with it |
|---|---|---|
| `A1` | `<v>7</v>` | literal |
| `B1` | `<f>A1*3</f><v>21</v>` | binds and evaluates (the control) |
| `C1` | `<c r="C1" t="e"><f>Is?Valid*2</f><v>#NAME?</v></c>` | OxFml REJECTS the text; OxCalc publishes the file's cache as a persistent `FileCached` pin |

`C1` is the point of the fixture. `=Is?Valid*2` is an Excel-legal formula:
a question mark is a legal character inside a defined name by the file
format's own grammar ([MS-XLSX] §2.2.2 Formulas, `name-character =
name-start-character / decimal-digit / full-stop / questionmark`), and
because no name `Is?Valid` is defined in this workbook Excel evaluates the
cell to `#NAME?` and stores an error-typed cell. The `C1` element above is
**byte-for-byte what Excel wrote**: observed 2026-09-15 on Excel 16.0
build 20326 through COM (`Range("C1").Formula = "=Is?Valid*2"` accepted,
`.Text = "#NAME?"`, saved as `.xlsx`, unzipped: `<c r="C1" t="e"><f>Is?Valid*2</f><v>#NAME?</v></c>`,
no `<definedNames>` in `xl/workbook.xml`). The surrounding five parts are
the minimal hand-authored envelope, not Excel's. OxFml's lexer tokenizes
`?` as `Unknown`, so on load OxCalc never binds the cell: it publishes the
file's own `#NAME?` with `PublishedValueProvenance::FileCached` and records
a `BindDegradation` for `R1C3` (OxCalc `consumer.rs`, the
`FormulaBindRejected` arm of the loader). A comparator that reads values
without provenance therefore scores `C1` as a MATCH with Excel although the
engine never evaluated it — the false green the W012 instrument exists to
avoid. See `Foundation/notes/PROGRAM_INVESTIGATION_2026-09-10.md` §1 and the
host-core test module `src/dnacalc-host-core/src/xlsx_false_green.rs`.

History of the rejected text. The fixture was cut on 2026-09-14 with
`C1 = =Größe*2` (a non-ASCII name), which OxFml `0bf8266` rejected at the
lexer. OxFml fml-kt8.2 (`cbb3099`, 2026-09-15) made non-ASCII identifiers
lex, parse, bind and evaluate, so that text now BINDS and the engine
publishes `#VALUE!` `Calculated` for it (a real divergence from Excel's
`#NAME?`, recorded on `dtc-5zet.18` / OxCalc `calc-1q03.17`) — the pin was
gone and five tests went red as designed (host-core at its verbatim
diagnostics pin and `formulas_bound == 1`, dnacalc-conform at the verdict
and coverage 0.5). `dtc-5zet.20` probed the current OxFml through the
product's open path with about a hundred candidate texts and re-cut `C1`
on one it still rejects. Still rejected at bind on that tree: a `?` inside
a name (`my?name*2`, `Grö?e*2`), a backslash after the first character of
a name (`my\name*2`, `\a\*2`), and a trailing dot (`a.*2`, `_.*2`) — all
six accepted by live Excel in the same COM probe (2026-09-15), each
evaluating `#NAME?`. Not usable: `=#REF!A1` is rejected by OxFml but Excel
REFUSES it when typed and writes `#REF!*2` (which OxFml binds) for a
deleted-sheet reference. When
OxFml learns `?` in names the pin fails loudly again; the remaining
candidates above are the next re-cut, and if none is left the pin needs a
different producer (an external-reference pin `[1]Sheet1!A1`, which
already publishes `FileCached`, or a `NotCalcModeled` cell).

Why the cache is `#NAME?` and not a number: declaring the name
(`<definedName name="Is?Valid">Sheet1!$A$1</definedName>`) makes the open
FAIL outright — OxCalc's defined-name key validator accepts only ASCII
alphanumerics, `_` and `.` after the first character
(`InvalidDefinedName`, `grid/reference_engine/parse.rs`
`excel_grid_defined_name_key`), so an Excel-faithful numeric cache for a
rejected name is not openable today (filed as dtc-5zet.5, which already
notes `?`). The undefined-name form is what Excel wrote here.

`parts/` (readable XML) is the source of truth; there is no committed binary
(no app click-through needs one). Host-core tests zip `parts/` in memory
through `oxdoc_conformance::read_fixture_parts_as_xlsx` (OxDoc's own
committed-parts pattern; no zip crate in this repo).

Constraints, inherited from `fixtures/w011/a1_times_three` (see its README):

- Exactly five parts: no `xl/styles.xml`, `xl/sharedStrings.xml`,
  `xl/calcChain.xml`, drawings.
- 1900 date system; `<calcPr calcMode="auto"/>`.
- `A1`/`B1` start tags carry only `r`; `C1` carries `r` and `t="e"` — the
  attribute Excel writes for every error-cached cell, and one of the two
  (`r`/`t`) OxDoc's surgical save path accepts.
- `B1`/`C1` are Normal formulas (no `t="shared"`/`"array"`, no `si`/`ref`);
  text stored without the leading `=`.

The Manual twin `../false_green_manual/` is byte-identical except for
`<calcPr calcMode="manual"/>`.
