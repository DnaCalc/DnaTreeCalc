//! `dnacalc-conform` — the W012 measured-Excel conformance instrument
//! (`Foundation/notes/PROGRAM_INVESTIGATION_2026-09-10.md` §3 W1, Appendix A;
//! bead dtc-5zet.8).
//!
//! Excel already wrote its answers into the files: every formula cell an
//! Excel-saved workbook carries also carries Excel's computed result as
//! `<v>`. So a cell-by-cell Excel-vs-DNA differential needs no live Excel —
//! open → ingest → recalc → compare against the cache. This crate runs that
//! differential over a corpus directory **through the path the product
//! runs** (`dnacalc_host_core::WorkbookSession::open_xlsx_bytes` +
//! `recalculate`), never a harness path inside the engine.
//!
//! The trap it exists to avoid (proven in
//! `dnacalc-host-core/src/xlsx_false_green.rs`): when OxFml rejects a
//! formula, OxCalc publishes Excel's own cached value as a persistent
//! `FileCached` pin, so a value-only comparator scores every refusal as a
//! pass. Hence the rule that shapes every module here:
//!
//! > Provenance decides before value does. Only `Calculated { tick_id }`
//! > may be scored `Match` / `Mismatch`. The primary number is coverage —
//! > `engine_calculated / formula_cells` — not the mismatch count.
//!
//! Modules: [`oracle`] (Excel's answers from OxDoc's event stream — the
//! denominator), [`verdict`] (the ladder's types and the comparator),
//! [`volatile`] (nondeterministic-formula detection), [`diff`] (the runner
//! and the artifact assembly), [`report`] (the `divergence.v1.json`
//! schema), [`cli`] (argument parsing for the binary).
//!
//! TC tier: this crate may depend on `dnacalc-host-core`, `oxcalc_core`,
//! `oxdoc_model`, `oxdoc_xlsx`; it is never a dependency of any other
//! workspace crate (positive control in `dnacalc-arch-gates`).

pub mod cli;
pub mod diff;
pub mod oracle;
pub mod report;
pub mod verdict;
pub mod volatile;

#[cfg(test)]
mod fixture_tests;
