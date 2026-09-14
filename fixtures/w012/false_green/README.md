# false_green — the W012 FileCached-pin fixture

Status: committed W012 fixture (`dtc-5zet.2`). One sheet `Sheet1`:

| cell | file content | what the engine does with it |
|---|---|---|
| `A1` | `<v>7</v>` | literal |
| `B1` | `<f>A1*3</f><v>21</v>` | binds and evaluates (the control) |
| `C1` | `<c r="C1" t="e"><f>Größe*2</f><v>#NAME?</v></c>` | OxFml REJECTS the text; OxCalc publishes the file's cache as a persistent `FileCached` pin |

`C1` is the point of the fixture. `=Größe*2` is an Excel-legal formula (a
defined name may contain non-ASCII letters), and because no name `Größe` is
defined in this workbook Excel would evaluate it to `#NAME?` and, per the
SpreadsheetML cell rules, store an error-typed cell like the one above
(`t="e"` plus `<v>#NAME?</v>`). This fixture is hand-authored, not
Excel-authored: the bytes follow the documented format, and were not
observed from a live Excel save. OxFml's lexer does not accept
non-ASCII letters outside a string, so on load OxCalc never binds the cell:
it publishes the file's own `#NAME?` with `PublishedValueProvenance::
FileCached` and records a `BindDegradation` for `R1C3` (OxCalc
`consumer.rs`, the `FormulaBindRejected` arm of the loader). A comparator
that reads values without provenance therefore scores `C1` as a MATCH with
Excel although the engine never evaluated it — the false green the W012
instrument exists to avoid. See `Foundation/notes/PROGRAM_INVESTIGATION_2026-09-10.md`
§1 and the host-core test module `src/dnacalc-host-core/src/xlsx_false_green.rs`.

Why the cache is `#NAME?` and not a number: declaring the name
(`<definedName name="Größe">Sheet1!$A$1</definedName>`) makes the open FAIL
outright — OxCalc's defined-name key validator is ASCII-only
(`InvalidDefinedName { name: "Größe" }`), so an Excel-faithful numeric cache
for a rejected non-ASCII name is not openable today (filed as dtc-5zet.5).
The undefined-name form is what the documented format says Excel writes for
such a workbook; it has not been captured from live Excel here.

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
