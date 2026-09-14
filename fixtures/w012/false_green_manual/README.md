# false_green_manual — the Manual twin of `../false_green`

Status: committed W012 fixture (`dtc-5zet.2`; `C1` re-cut to `=Is?Valid*2`
by `dtc-5zet.20` in lock-step with the auto fixture). Byte-identical to
`../false_green/parts/**` except `xl/workbook.xml` carries
`<calcPr calcMode="manual"/>` instead of `auto`.

Purpose: pin the second half of the FileCached trap. Under `calcMode="manual"`
OxCalc runs no open-recalc, so EVERY formula cell renders the file's cache as
`FileCached` until an explicit `recalculate_workbook` (F9) — `B1 = 21` and
`C1 = #NAME?` alike. After F9, `B1` becomes engine-`Calculated` while the
rejected `C1` is STILL `FileCached`: the pin is persistent, not transient. 62
of the 174 Excel-authored workbooks in the program's corpus are
`calcMode="manual"` (Foundation note, Appendix B), so an instrument that
compared values before provenance would score all of them green without
evaluating a single cell.

Keep the twins in lock-step: the host-core test module
`src/dnacalc-host-core/src/xlsx_false_green.rs` asserts the two fixtures'
OxDoc event streams differ only in the workbook header's calc mode. See
`../false_green/README.md` for the fixture's content and constraints.
