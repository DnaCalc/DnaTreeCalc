//! W011 `.xlsx` fixture access for host-core tests — the committed
//! `a1_times_three` workbook (`Sheet1`: `A1 = 7`, `B1 = =A1*3`, cached `21`).
//!
//! The readable XML parts under `<repo>/fixtures/w011/a1_times_three/parts/`
//! are the source of truth. Tests zip them in memory through
//! [`oxdoc_conformance::read_fixture_parts_as_xlsx`] — the committed-parts
//! pattern OxDoc's own conformance crate uses — so host-core never touches zip
//! or xlsx bytes itself and this repo never takes a zip crate. The committed
//! binary `<repo>/fixtures/w011/a1_times_three.xlsx` exists for the app
//! click-through (open dialog / drag-drop); it is regenerated from the parts by
//! the `#[ignore]`d generator `regenerate_w011_fixture_binary_from_parts` and
//! pinned to the parts by the acceptance test through **event-stream
//! equality**, not zip byte equality (zip metadata may legitimately differ).
//!
//! Fixture constraints the W011 save round-trip (cached `B1 = 30` on reopen)
//! depends on — see `fixtures/w011/a1_times_three/README.md`:
//!
//! - both cells already exist and their start tags carry only `r` (OxDoc's
//!   conservative round-trip save rejects cell add/remove, formula add/remove,
//!   and value edits to cells carrying any attribute beyond `r`/`t`);
//! - exactly five parts: no styles, no shared strings, no calc chain, no
//!   drawings;
//! - 1900 date system, `calcMode="auto"`, `B1` a Normal formula stored
//!   without the leading `=`.
//!
//! The Wave 3a Manual twin (dtc-j7n8.13) lives beside it:
//! `<repo>/fixtures/w011/a1_times_three_manual/{parts/,README.md}` plus the
//! committed `a1_times_three_manual.xlsx`, byte-identical to the auto fixture
//! except for `<calcPr calcMode="manual"/>`. Same zipping, same generator
//! pattern (`regenerate_w011_manual_fixture_binary_from_parts`), same
//! event-stream pin to the parts — and additionally pinned to the AUTO
//! twin's stream with only the header's calc mode differing, so the two
//! fixtures can never drift apart in anything but the mode.
//!
//! The Wave 3b cross-sheet fixture (dtc-j7n8.14) lives beside them too:
//! `<repo>/fixtures/w011/cross_sheet/{parts/,README.md}` plus the committed
//! `cross_sheet.xlsx` — two sheets, one unstyled cell each, `Sheet1!A1 = 2`
//! and `Sheet2!A1 = =Sheet1!A1*5` cached 10, six parts (a second
//! worksheet), `calcMode="auto"`. Same zipping, same generator pattern
//! (`regenerate_w011_cross_sheet_fixture_binary_from_parts`), same
//! event-stream pin to the parts. Single-cell cross-sheet references only:
//! the cross-sheet RANGE gap is calc-5kqg.67 (OxCalc), not this fixture's.
//!
//! Compiled only under `cfg(test)`: `oxdoc_conformance` is a dev-dependency
//! and must never become a normal one.
//!
//! The raw-reopen helpers below (`open_xlsx_raw`, `raw_sheet_cells`, …) are
//! the save tests' FILE-truth probe (dtc-j7n8.7): they walk OxDoc's own
//! events of a package with no engine involvement, because an engine readout
//! after a reload would recalculate and mask a stale cached formula value.

use std::io::Cursor;
use std::path::{Path, PathBuf};

use oxdoc_model::{
    CellPayload, DocumentEvent, DocumentFidelityLedger, DocumentFidelityLedgerEntry,
    FidelityDisposition, PackedCellAddr,
};
use oxdoc_xlsx::{HostOwnedXlsxSource, LoadProfile, open_host_owned_xlsx_source};

/// Walk from this crate's manifest dir (`src/dnacalc-host-core`) to the repo
/// root. Relative on purpose — no absolute paths in tests.
const REPO_ROOT_FROM_MANIFEST: &str = "../..";

/// Repo-relative location of the readable fixture parts (source of truth).
pub(crate) const W011_FIXTURE_PARTS_REL: &str = "fixtures/w011/a1_times_three/parts";

/// Repo-relative location of the committed binary fixture (app click-through).
pub(crate) const W011_FIXTURE_XLSX_REL: &str = "fixtures/w011/a1_times_three.xlsx";

/// Repo-relative location of the Manual twin's readable parts (dtc-j7n8.13).
pub(crate) const W011_MANUAL_FIXTURE_PARTS_REL: &str = "fixtures/w011/a1_times_three_manual/parts";

/// Repo-relative location of the Manual twin's committed binary.
pub(crate) const W011_MANUAL_FIXTURE_XLSX_REL: &str = "fixtures/w011/a1_times_three_manual.xlsx";

/// Repo-relative location of the Wave 3b cross-sheet fixture's readable
/// parts (dtc-j7n8.14): `Sheet1!A1 = 2`, `Sheet2!A1 = =Sheet1!A1*5` cached
/// `10` — six parts (a second worksheet), otherwise the same constraints as
/// `a1_times_three`.
pub(crate) const W011_CROSS_SHEET_FIXTURE_PARTS_REL: &str = "fixtures/w011/cross_sheet/parts";

/// Repo-relative location of the cross-sheet fixture's committed binary.
pub(crate) const W011_CROSS_SHEET_FIXTURE_XLSX_REL: &str = "fixtures/w011/cross_sheet.xlsx";

/// Repo-relative location of the POST-EDIT saved bytes (`A1 = 10`, `B1`
/// cached 30) the `#[ignore]`d generator `emit_saved_fixture_for_excel_compare`
/// (`workbook.rs`, dtc-j7n8.7) writes for the Wave 2 Excel comparison
/// (dtc-j7n8.11): under the BUILD dir, never the repo — a derived artifact,
/// regenerated on demand.
pub(crate) const W011_SAVED_FIXTURE_TARGET_REL: &str = "target/w011/a1_times_three_saved.xlsx";

/// The exact `XlsxError::UnsupportedRoundTripFeature` message OxDoc emits
/// when a save projects a cell the opened package does not have (the W011
/// `C1 = 5` cell add), observed 2026-09-01 against OxDoc `786ef0c`: the
/// surgical worksheet merge (`worksheet_xml_with_surgical_cell_replacements`,
/// oxdoc-xlsx `lib.rs`) compares the original and projected cell KEY SETS
/// and refuses without naming the cell. dtc-j7n8.7 pre-registered this
/// branch ("if the observed message does not name C1, widen the assertion to
/// the exact observed text — never delete it"), so the refusal tests pin this
/// text verbatim; an upstream wording change fails them loudly, on purpose.
pub(crate) const OXDOC_CELL_ADD_REJECTION: &str =
    "adding or removing cells in metadata-aware round-trip is not supported yet";

/// The exact part names the fixture consists of (zip entry names, sorted).
pub(crate) const W011_FIXTURE_PART_NAMES: [&str; 5] = [
    "[Content_Types].xml",
    "_rels/.rels",
    "xl/_rels/workbook.xml.rels",
    "xl/workbook.xml",
    "xl/worksheets/sheet1.xml",
];

/// The exact part names the cross-sheet fixture consists of (zip entry
/// names, sorted): the five of `a1_times_three` plus `sheet2.xml`.
pub(crate) const W011_CROSS_SHEET_FIXTURE_PART_NAMES: [&str; 6] = [
    "[Content_Types].xml",
    "_rels/.rels",
    "xl/_rels/workbook.xml.rels",
    "xl/workbook.xml",
    "xl/worksheets/sheet1.xml",
    "xl/worksheets/sheet2.xml",
];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(REPO_ROOT_FROM_MANIFEST)
}

/// Directory holding the readable fixture parts.
pub(crate) fn w011_fixture_parts_dir() -> PathBuf {
    repo_root().join(W011_FIXTURE_PARTS_REL)
}

/// Path of the committed binary fixture.
pub(crate) fn w011_fixture_xlsx_path() -> PathBuf {
    repo_root().join(W011_FIXTURE_XLSX_REL)
}

/// Path of the generated post-edit saved fixture (build dir).
pub(crate) fn w011_saved_fixture_target_path() -> PathBuf {
    repo_root().join(W011_SAVED_FIXTURE_TARGET_REL)
}

/// Directory holding the Manual twin's readable parts (dtc-j7n8.13).
pub(crate) fn w011_manual_fixture_parts_dir() -> PathBuf {
    repo_root().join(W011_MANUAL_FIXTURE_PARTS_REL)
}

/// Path of the Manual twin's committed binary.
pub(crate) fn w011_manual_fixture_xlsx_path() -> PathBuf {
    repo_root().join(W011_MANUAL_FIXTURE_XLSX_REL)
}

/// Directory holding the cross-sheet fixture's readable parts (dtc-j7n8.14).
pub(crate) fn w011_cross_sheet_fixture_parts_dir() -> PathBuf {
    repo_root().join(W011_CROSS_SHEET_FIXTURE_PARTS_REL)
}

/// Path of the cross-sheet fixture's committed binary.
pub(crate) fn w011_cross_sheet_fixture_xlsx_path() -> PathBuf {
    repo_root().join(W011_CROSS_SHEET_FIXTURE_XLSX_REL)
}

/// Zip a fixture's readable parts into `.xlsx` bytes in memory through
/// OxDoc's own conformance zipper — the one byte source every W011 fixture
/// (auto and Manual) is opened from.
fn fixture_bytes_from_parts(parts_dir: &Path) -> Vec<u8> {
    oxdoc_conformance::read_fixture_parts_as_xlsx(parts_dir).unwrap_or_else(|err| {
        panic!(
            "failed to zip the W011 fixture parts under {}: {err}",
            parts_dir.display()
        )
    })
}

/// A committed binary fixture's bytes, read from disk.
fn committed_xlsx_bytes(path: &Path) -> Vec<u8> {
    std::fs::read(path).unwrap_or_else(|err| {
        panic!(
            "failed to read the committed W011 fixture {}: {err}",
            path.display()
        )
    })
}

/// The `a1_times_three` workbook as `.xlsx` bytes, zipped in memory from the
/// committed parts. This is the byte source every W011 host-core test opens;
/// later W011 beads (open command, ingest, edit/recalc, save/reopen) reuse it.
pub(crate) fn w011_fixture_bytes() -> Vec<u8> {
    fixture_bytes_from_parts(&w011_fixture_parts_dir())
}

/// The committed binary fixture's bytes, read from disk.
pub(crate) fn w011_committed_xlsx_bytes() -> Vec<u8> {
    committed_xlsx_bytes(&w011_fixture_xlsx_path())
}

/// The Manual twin (`calcMode="manual"`, otherwise `a1_times_three`) as
/// `.xlsx` bytes, zipped in memory from its committed parts — the byte
/// source of the Wave 3a Manual calc-mode lane (dtc-j7n8.13).
pub(crate) fn w011_manual_fixture_bytes() -> Vec<u8> {
    fixture_bytes_from_parts(&w011_manual_fixture_parts_dir())
}

/// The Manual twin's committed binary bytes, read from disk.
pub(crate) fn w011_manual_committed_xlsx_bytes() -> Vec<u8> {
    committed_xlsx_bytes(&w011_manual_fixture_xlsx_path())
}

/// The Wave 3b cross-sheet fixture (`Sheet1!A1 = 2`; `Sheet2!A1 =
/// =Sheet1!A1*5` cached 10) as `.xlsx` bytes, zipped in memory from its
/// committed parts — the byte source of the cross-sheet lane (dtc-j7n8.14).
pub(crate) fn w011_cross_sheet_fixture_bytes() -> Vec<u8> {
    fixture_bytes_from_parts(&w011_cross_sheet_fixture_parts_dir())
}

/// The cross-sheet fixture's committed binary bytes, read from disk.
pub(crate) fn w011_cross_sheet_committed_xlsx_bytes() -> Vec<u8> {
    committed_xlsx_bytes(&w011_cross_sheet_fixture_xlsx_path())
}

/// Open `.xlsx` bytes through OxDoc under [`LoadProfile::full()`] — the
/// profile the W011 host lifecycle uses, because only `full()` materializes
/// the `FormulaTopology` a later save needs — with NO engine involvement: the
/// returned source's `source_context.events()` are the file's raw truth. This
/// is the reopen every save test walks; an engine readout after a reload
/// would recalculate and mask a stale cached value.
pub(crate) fn open_xlsx_raw(bytes: &[u8]) -> HostOwnedXlsxSource {
    open_host_owned_xlsx_source(Cursor::new(bytes), LoadProfile::full())
        .unwrap_or_else(|err| panic!("OxDoc rejected the xlsx bytes: {err} / {err:?}"))
}

/// Every `(address, payload)` pair the `CellChunk` events of the sheet named
/// `sheet_name` carry, in stream order — one sheet's raw cell truth, straight
/// from OxDoc's events. Fails naming the sheet when the package has none of
/// that name.
pub(crate) fn raw_sheet_cells(
    source: &HostOwnedXlsxSource,
    sheet_name: &str,
) -> Vec<(PackedCellAddr, CellPayload)> {
    let events = source.source_context.events();
    let mut in_sheet = false;
    let mut found = false;
    let mut cells = Vec::new();
    for event in events {
        match event {
            DocumentEvent::SheetBegin(sheet) => {
                in_sheet = sheet.name == sheet_name;
                found |= in_sheet;
            }
            DocumentEvent::SheetEnd { .. } => in_sheet = false,
            DocumentEvent::CellChunk(chunk) if in_sheet => {
                cells.extend(chunk.cells.iter().cloned());
            }
            _ => {}
        }
    }
    assert!(found, "no sheet named {sheet_name:?} in {events:#?}");
    cells
}

/// The raw payload at 1-based `(row, col)`, failing with the whole cell list
/// when the cell is absent.
pub(crate) fn raw_cell_payload(
    cells: &[(PackedCellAddr, CellPayload)],
    row: u32,
    col: u32,
) -> &CellPayload {
    let addr = PackedCellAddr::from_one_based(row, col)
        .unwrap_or_else(|| panic!("({row}, {col}) is not a 1-based cell address"));
    cells
        .iter()
        .find(|(cell_addr, _)| *cell_addr == addr)
        .map(|(_, payload)| payload)
        .unwrap_or_else(|| panic!("no cell at ({row}, {col}) in {cells:?}"))
}

/// Print every ledger entry under `label` (`--nocapture` evidence).
pub(crate) fn log_ledger(label: &str, ledger: &DocumentFidelityLedger) {
    println!("{label}: {} entries", ledger.entries.len());
    for entry in &ledger.entries {
        println!("{label}: {} -> {:?}", entry.subject, entry.disposition);
    }
}

/// The ledger's `Dropped` entries — the visible-loss signal a save test
/// asserts empty (never silenced, never filtered away).
pub(crate) fn dropped_entries(
    ledger: &DocumentFidelityLedger,
) -> Vec<&DocumentFidelityLedgerEntry> {
    ledger
        .entries
        .iter()
        .filter(|entry| matches!(entry.disposition, FidelityDisposition::Dropped { .. }))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxdoc_model::{
        CachedValueProvenance, CalcMode, CellChunk, DateSystem, FormulaCachedValueState,
        FormulaRecordKind, FormulaTextKind, FormulaTopology, ProjectionStatus, SheetRef,
        WorkbookHeader,
    };

    fn a1() -> PackedCellAddr {
        PackedCellAddr::from_one_based(1, 1).unwrap()
    }

    fn b1() -> PackedCellAddr {
        PackedCellAddr::from_one_based(1, 2).unwrap()
    }

    /// Open the W011 fixture bytes through OxDoc under the full profile (the
    /// shared raw-reopen helper, named for the fixture in these tests).
    fn open_full(bytes: &[u8]) -> HostOwnedXlsxSource {
        open_xlsx_raw(bytes)
    }

    fn cell_payload(cells: &[(PackedCellAddr, CellPayload)], addr: PackedCellAddr) -> &CellPayload {
        cells
            .iter()
            .find(|(cell_addr, _)| *cell_addr == addr)
            .map(|(_, payload)| payload)
            .unwrap_or_else(|| panic!("no cell at {addr:?} in {cells:?}"))
    }

    /// Acceptance (dtc-j7n8.2): the committed W011 fixture opens through
    /// OxDoc under `LoadProfile::full()` and exposes exactly the two expected
    /// cells — `A1 = Number(7)`, `B1 = Formula { text: "A1*3", cached:
    /// Number(21) }` — plus the `FormulaTopology` record for `B1` the later
    /// save round-trip needs; and the committed binary yields the same event
    /// stream as the parts it was generated from.
    #[test]
    fn w011_fixture_opens_through_oxdoc_with_two_cells() {
        let parts_dir = w011_fixture_parts_dir();
        println!("W011 fixture parts: {}", parts_dir.display());

        let bytes = w011_fixture_bytes();
        let source = open_full(&bytes);
        let events = source.source_context.events();

        // Workbook header: the template's `date1904` / `manual` settings were
        // deliberately dropped — W011 wants the 1900 date system and
        // automatic calculation.
        let headers: Vec<&WorkbookHeader> = events
            .iter()
            .filter_map(|event| match event {
                DocumentEvent::WorkbookHeader(header) => Some(header),
                _ => None,
            })
            .collect();
        assert_eq!(
            headers.len(),
            1,
            "exactly one WorkbookHeader in {events:#?}"
        );
        assert_eq!(headers[0].date_system, DateSystem::Date1900);
        assert_eq!(headers[0].calc_mode, CalcMode::Automatic);

        // Exactly one sheet, named Sheet1.
        let sheets: Vec<&SheetRef> = events
            .iter()
            .filter_map(|event| match event {
                DocumentEvent::SheetBegin(sheet) => Some(sheet),
                _ => None,
            })
            .collect();
        assert_eq!(sheets.len(), 1, "exactly one SheetBegin in {events:#?}");
        assert_eq!(sheets[0].name, "Sheet1");
        let sheet_id = sheets[0].sheet_id;

        // One CellChunk carrying exactly the two cells — no extra cells "for
        // later", which the conservative round-trip save would not tolerate.
        let chunks: Vec<&CellChunk> = events
            .iter()
            .filter_map(|event| match event {
                DocumentEvent::CellChunk(chunk) => Some(chunk),
                _ => None,
            })
            .collect();
        assert_eq!(chunks.len(), 1, "exactly one CellChunk in {events:#?}");
        let cells = &chunks[0].cells;
        assert_eq!(cells.len(), 2, "exactly two cells, got {cells:?}");

        let a1_payload = cell_payload(cells, a1());
        let b1_payload = cell_payload(cells, b1());
        assert_eq!(a1_payload, &CellPayload::Number(7.0), "A1 is the literal 7");
        assert_eq!(
            b1_payload,
            &CellPayload::Formula {
                region: None,
                text: Some("A1*3".to_string()),
                cached: Some(Box::new(CellPayload::Number(21.0))),
            },
            "B1 is the Normal formula A1*3 with file-cached 21"
        );

        println!("A1 payload: {a1_payload:?}");
        let CellPayload::Formula { text, cached, .. } = b1_payload else {
            unreachable!("B1 asserted to be a formula above");
        };
        println!("B1 formula text: {text:?}");
        println!("B1 cached value: {cached:?}");

        // `full()` materializes the formula topology; the W011 save needs the
        // B1 record present, Normal, with its text and a file-cached value.
        let topologies: Vec<&FormulaTopology> = events
            .iter()
            .filter_map(|event| match event {
                DocumentEvent::FormulaTopology(topology) => Some(topology),
                _ => None,
            })
            .collect();
        assert_eq!(
            topologies.len(),
            1,
            "exactly one FormulaTopology under LoadProfile::full() in {events:#?}"
        );
        let topology = topologies[0];
        assert_eq!(topology.sheet_id, sheet_id);
        assert!(
            topology.unsupported_fragments.is_empty(),
            "no unsupported formula fragments: {:?}",
            topology.unsupported_fragments
        );
        assert_eq!(
            topology.records.len(),
            1,
            "only B1 carries a formula: {:?}",
            topology.records
        );
        let record = &topology.records[0];
        assert_eq!(record.sheet_id, sheet_id);
        assert_eq!(record.address, b1(), "the formula record is B1's");
        assert_eq!(record.kind, FormulaRecordKind::Normal);
        assert_eq!(record.text.as_deref(), Some("A1*3"));
        assert_eq!(record.text_kind, FormulaTextKind::SpreadsheetMlA1);
        assert_eq!(
            record.cached_value,
            FormulaCachedValueState::Present {
                provenance: CachedValueProvenance::FileCached,
            }
        );
        assert!(record.unsupported_fragments.is_empty());

        // Nothing in the package was dropped or lossily projected.
        for entry in &source.load_ledger.entries {
            assert!(
                matches!(
                    entry.disposition,
                    FidelityDisposition::Projected {
                        status: ProjectionStatus::Direct,
                        loss: None,
                    }
                ),
                "ledger entry {} is not a direct lossless projection: {:?}",
                entry.subject,
                entry.disposition
            );
        }

        // The committed binary is the same workbook: identical event stream
        // (the contract) and identical load ledger. Zip byte equality is not
        // required — zip metadata may differ between materializations.
        let xlsx_path = w011_fixture_xlsx_path();
        println!("W011 committed fixture: {}", xlsx_path.display());
        let committed = open_full(&w011_committed_xlsx_bytes());
        assert_eq!(
            committed.source_context.events(),
            events,
            "committed {} is out of sync with its parts; rerun \
             regenerate_w011_fixture_binary_from_parts with --ignored",
            xlsx_path.display()
        );
        assert_eq!(committed.load_ledger, source.load_ledger);
    }

    /// The fixture is exactly the five parts the W011 save round-trip relies
    /// on (no styles, shared strings, calc chain, or drawings), and the
    /// committed binary stays tiny.
    #[test]
    fn w011_fixture_is_exactly_five_parts_and_a_tiny_binary() {
        let parts_dir = w011_fixture_parts_dir();
        let mut part_names = Vec::new();
        collect_part_names(&parts_dir, &parts_dir, &mut part_names);
        part_names.sort();
        assert_eq!(part_names, W011_FIXTURE_PART_NAMES);

        let committed_len = w011_committed_xlsx_bytes().len();
        println!("W011 committed fixture size: {committed_len} bytes");
        assert!(
            committed_len < 5 * 1024,
            "committed fixture must stay under 5 KB, got {committed_len} bytes"
        );
    }

    /// The calc mode of the one `WorkbookHeader` in an event stream.
    fn header_calc_mode(events: &[DocumentEvent]) -> CalcMode {
        let headers: Vec<&WorkbookHeader> = events
            .iter()
            .filter_map(|event| match event {
                DocumentEvent::WorkbookHeader(header) => Some(header),
                _ => None,
            })
            .collect();
        assert_eq!(
            headers.len(),
            1,
            "exactly one WorkbookHeader in {events:#?}"
        );
        headers[0].calc_mode
    }

    /// Acceptance (dtc-j7n8.13): the Manual twin is `a1_times_three` with
    /// exactly one difference — `calcMode="manual"`. Pinned at three levels
    /// so the two fixtures can never drift apart in anything but the mode:
    /// (1) the PARTS: same five names, every part byte-identical except
    /// `xl/workbook.xml`, whose only textual difference is the `calcMode`
    /// attribute value; (2) the OxDoc EVENT STREAM: the Manual twin's
    /// header says `CalcMode::Manual`, and swapping that one field back to
    /// `Automatic` makes the streams equal (so the two cells, the formula
    /// topology, and every other event are identical); (3) the committed
    /// BINARY: the same event stream and load ledger as its parts, under 5
    /// KB, with no lossy ledger entry.
    #[test]
    fn w011_manual_fixture_is_the_auto_twin_with_calc_mode_manual() {
        let auto_dir = w011_fixture_parts_dir();
        let manual_dir = w011_manual_fixture_parts_dir();
        println!("W011 auto fixture parts:   {}", auto_dir.display());
        println!("W011 manual fixture parts: {}", manual_dir.display());

        // (1) Parts: same names; byte-identical except workbook.xml, which
        // differs only in the calcMode attribute value.
        let mut manual_names = Vec::new();
        collect_part_names(&manual_dir, &manual_dir, &mut manual_names);
        manual_names.sort();
        assert_eq!(manual_names, W011_FIXTURE_PART_NAMES);
        for name in W011_FIXTURE_PART_NAMES {
            let auto_part = std::fs::read_to_string(auto_dir.join(name)).unwrap();
            let manual_part = std::fs::read_to_string(manual_dir.join(name)).unwrap();
            if name == "xl/workbook.xml" {
                assert!(
                    auto_part.contains("<calcPr calcMode=\"auto\"/>"),
                    "the auto fixture pins calcMode=\"auto\": {auto_part}"
                );
                assert!(
                    manual_part.contains("<calcPr calcMode=\"manual\"/>"),
                    "the Manual twin pins calcMode=\"manual\": {manual_part}"
                );
                assert_eq!(
                    manual_part.replace("calcMode=\"manual\"", "calcMode=\"auto\""),
                    auto_part,
                    "xl/workbook.xml differs from the auto twin's in nothing but the calcMode value"
                );
            } else {
                assert_eq!(
                    manual_part, auto_part,
                    "part {name} must be byte-identical to the auto twin's"
                );
            }
        }

        // (2) Event stream: Manual header; otherwise the auto stream.
        let manual = open_full(&w011_manual_fixture_bytes());
        let manual_events = manual.source_context.events();
        assert_eq!(
            header_calc_mode(manual_events),
            CalcMode::Manual,
            "OxDoc reads calcMode=\"manual\" into the WorkbookHeader"
        );
        let auto = open_full(&w011_fixture_bytes());
        let auto_events = auto.source_context.events();
        assert_eq!(header_calc_mode(auto_events), CalcMode::Automatic);
        let manual_as_auto: Vec<DocumentEvent> = manual_events
            .iter()
            .cloned()
            .map(|event| match event {
                DocumentEvent::WorkbookHeader(mut header) => {
                    header.calc_mode = CalcMode::Automatic;
                    DocumentEvent::WorkbookHeader(header)
                }
                other => other,
            })
            .collect();
        assert_eq!(
            manual_as_auto.as_slice(),
            auto_events,
            "with the header's calc mode swapped back, the Manual twin's event stream IS the \
             auto fixture's: same two cells, same B1 topology record, nothing else"
        );
        assert_eq!(manual.load_ledger, auto.load_ledger);
        for entry in &manual.load_ledger.entries {
            assert!(
                matches!(
                    entry.disposition,
                    FidelityDisposition::Projected {
                        status: ProjectionStatus::Direct,
                        loss: None,
                    }
                ),
                "ledger entry {} is not a direct lossless projection: {:?}",
                entry.subject,
                entry.disposition
            );
        }

        // (3) The committed binary is the same workbook as its parts.
        let xlsx_path = w011_manual_fixture_xlsx_path();
        println!("W011 manual committed fixture: {}", xlsx_path.display());
        let committed_bytes = w011_manual_committed_xlsx_bytes();
        println!(
            "W011 manual committed fixture size: {} bytes",
            committed_bytes.len()
        );
        assert!(
            committed_bytes.len() < 5 * 1024,
            "committed Manual fixture must stay under 5 KB, got {} bytes",
            committed_bytes.len()
        );
        let committed = open_full(&committed_bytes);
        assert_eq!(
            committed.source_context.events(),
            manual_events,
            "committed {} is out of sync with its parts; rerun \
             regenerate_w011_manual_fixture_binary_from_parts with --ignored",
            xlsx_path.display()
        );
        assert_eq!(committed.load_ledger, manual.load_ledger);
    }

    /// Acceptance (dtc-j7n8.14): the Wave 3b cross-sheet fixture opens
    /// through OxDoc under `LoadProfile::full()` as TWO sheets in workbook
    /// order — `Sheet1` (`sheet_id` 1) then `Sheet2` (`sheet_id` 2), each
    /// `SheetBegin` .. `SheetEnd` bracket closed before the next opens —
    /// with exactly one cell each: `Sheet1!A1 = Number(2)` and `Sheet2!A1 =
    /// Formula { text: "Sheet1!A1*5", cached: Number(10) }`; ONE
    /// `FormulaTopology` record, `Sheet2`'s, Normal, `FileCached`, no
    /// unsupported fragment (the cross-sheet reference is plain A1 text to
    /// OxDoc — it classifies nothing); a 1900/Automatic header; a lossless
    /// load ledger; and the committed binary yields the same event stream as
    /// the parts it was generated from.
    #[test]
    fn w011_cross_sheet_fixture_opens_through_oxdoc_with_two_sheets() {
        let parts_dir = w011_cross_sheet_fixture_parts_dir();
        println!("W011 cross-sheet fixture parts: {}", parts_dir.display());

        let source = open_full(&w011_cross_sheet_fixture_bytes());
        let events = source.source_context.events();

        // Header: 1900 date system, Automatic.
        let headers: Vec<&WorkbookHeader> = events
            .iter()
            .filter_map(|event| match event {
                DocumentEvent::WorkbookHeader(header) => Some(header),
                _ => None,
            })
            .collect();
        assert_eq!(
            headers.len(),
            1,
            "exactly one WorkbookHeader in {events:#?}"
        );
        assert_eq!(headers[0].date_system, DateSystem::Date1900);
        assert_eq!(headers[0].calc_mode, CalcMode::Automatic);

        // Two sheets, in workbook order, with properly nested brackets: the
        // WorkbookHeader/SheetBegin ordering the ingest sink relies on.
        let brackets: Vec<(u32, String, bool)> = events
            .iter()
            .filter_map(|event| match event {
                DocumentEvent::SheetBegin(sheet) => {
                    Some((sheet.sheet_id, sheet.name.clone(), true))
                }
                DocumentEvent::SheetEnd { sheet_id } => Some((*sheet_id, String::new(), false)),
                _ => None,
            })
            .collect();
        assert_eq!(
            brackets,
            vec![
                (1, "Sheet1".to_string(), true),
                (1, String::new(), false),
                (2, "Sheet2".to_string(), true),
                (2, String::new(), false),
            ],
            "Sheet1 opens and closes before Sheet2 opens: {events:#?}"
        );
        let header_index = events
            .iter()
            .position(|event| matches!(event, DocumentEvent::WorkbookHeader(_)))
            .unwrap();
        let first_sheet_index = events
            .iter()
            .position(|event| matches!(event, DocumentEvent::SheetBegin(_)))
            .unwrap();
        assert!(
            header_index < first_sheet_index,
            "the WorkbookHeader precedes the first SheetBegin: {events:#?}"
        );

        // Exactly one cell per sheet — the conservative round-trip save
        // tolerates no extra cell "for later".
        let sheet1_cells = raw_sheet_cells(&source, "Sheet1");
        let sheet2_cells = raw_sheet_cells(&source, "Sheet2");
        println!("W011 cross-sheet: raw Sheet1 cells = {sheet1_cells:?}");
        println!("W011 cross-sheet: raw Sheet2 cells = {sheet2_cells:?}");
        assert_eq!(
            sheet1_cells.len(),
            1,
            "Sheet1 holds only A1: {sheet1_cells:?}"
        );
        assert_eq!(
            sheet2_cells.len(),
            1,
            "Sheet2 holds only A1: {sheet2_cells:?}"
        );
        assert_eq!(
            raw_cell_payload(&sheet1_cells, 1, 1),
            &CellPayload::Number(2.0),
            "Sheet1!A1 is the literal 2"
        );
        assert_eq!(
            raw_cell_payload(&sheet2_cells, 1, 1),
            &CellPayload::Formula {
                region: None,
                text: Some("Sheet1!A1*5".to_string()),
                cached: Some(Box::new(CellPayload::Number(10.0))),
            },
            "Sheet2!A1 is the Normal cross-sheet formula Sheet1!A1*5 with file-cached 10"
        );

        // One FormulaTopology — Sheet2's — with the single A1 record.
        let topologies: Vec<&FormulaTopology> = events
            .iter()
            .filter_map(|event| match event {
                DocumentEvent::FormulaTopology(topology) => Some(topology),
                _ => None,
            })
            .collect();
        assert_eq!(
            topologies.len(),
            1,
            "exactly one FormulaTopology (Sheet2's; Sheet1 carries no formula) in {events:#?}"
        );
        let topology = topologies[0];
        assert_eq!(topology.sheet_id, 2, "the topology is Sheet2's");
        assert!(
            topology.unsupported_fragments.is_empty(),
            "no unsupported formula fragments: {:?}",
            topology.unsupported_fragments
        );
        assert_eq!(
            topology.records.len(),
            1,
            "only Sheet2!A1 carries a formula"
        );
        let record = &topology.records[0];
        assert_eq!(record.sheet_id, 2);
        assert_eq!(record.address, a1(), "the formula record is Sheet2!A1's");
        assert_eq!(record.kind, FormulaRecordKind::Normal);
        assert_eq!(record.text.as_deref(), Some("Sheet1!A1*5"));
        assert_eq!(record.text_kind, FormulaTextKind::SpreadsheetMlA1);
        assert_eq!(
            record.cached_value,
            FormulaCachedValueState::Present {
                provenance: CachedValueProvenance::FileCached,
            }
        );
        assert!(record.unsupported_fragments.is_empty());

        // Nothing in the package was dropped or lossily projected.
        log_ledger("W011 cross-sheet load ledger", &source.load_ledger);
        for entry in &source.load_ledger.entries {
            assert!(
                matches!(
                    entry.disposition,
                    FidelityDisposition::Projected {
                        status: ProjectionStatus::Direct,
                        loss: None,
                    }
                ),
                "ledger entry {} is not a direct lossless projection: {:?}",
                entry.subject,
                entry.disposition
            );
        }

        // The committed binary is the same workbook (event-stream equality).
        let xlsx_path = w011_cross_sheet_fixture_xlsx_path();
        println!(
            "W011 cross-sheet committed fixture: {}",
            xlsx_path.display()
        );
        let committed = open_full(&w011_cross_sheet_committed_xlsx_bytes());
        assert_eq!(
            committed.source_context.events(),
            events,
            "committed {} is out of sync with its parts; rerun \
             regenerate_w011_cross_sheet_fixture_binary_from_parts with --ignored",
            xlsx_path.display()
        );
        assert_eq!(committed.load_ledger, source.load_ledger);
    }

    /// The cross-sheet fixture is exactly six parts (the five of
    /// `a1_times_three` plus `sheet2.xml`; still no styles, shared strings,
    /// calc chain, or drawings), its shared parts are byte-identical to the
    /// auto fixture's, and the committed binary stays tiny.
    #[test]
    fn w011_cross_sheet_fixture_is_exactly_six_parts_and_a_tiny_binary() {
        let parts_dir = w011_cross_sheet_fixture_parts_dir();
        let mut part_names = Vec::new();
        collect_part_names(&parts_dir, &parts_dir, &mut part_names);
        part_names.sort();
        assert_eq!(part_names, W011_CROSS_SHEET_FIXTURE_PART_NAMES);

        // The one package-level part that does not enumerate sheets is the
        // auto fixture's, byte for byte — one fixture family, not a fork.
        let auto_dir = w011_fixture_parts_dir();
        assert_eq!(
            std::fs::read_to_string(parts_dir.join("_rels/.rels")).unwrap(),
            std::fs::read_to_string(auto_dir.join("_rels/.rels")).unwrap(),
            "part _rels/.rels must be byte-identical to the auto fixture's"
        );
        let workbook_xml = std::fs::read_to_string(parts_dir.join("xl/workbook.xml")).unwrap();
        assert!(
            workbook_xml.contains("<calcPr calcMode=\"auto\"/>"),
            "the cross-sheet fixture pins calcMode=\"auto\": {workbook_xml}"
        );
        assert!(
            !workbook_xml.contains("date1904"),
            "1900 date system (no workbookPr date1904): {workbook_xml}"
        );

        let committed_len = w011_cross_sheet_committed_xlsx_bytes().len();
        println!("W011 cross-sheet committed fixture size: {committed_len} bytes");
        assert!(
            committed_len < 5 * 1024,
            "committed cross-sheet fixture must stay under 5 KB, got {committed_len} bytes"
        );
    }

    fn collect_part_names(root: &Path, dir: &Path, names: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", dir.display()))
        {
            let path = entry.unwrap().path();
            if path.is_dir() {
                collect_part_names(root, &path, names);
            } else {
                let relative = path.strip_prefix(root).unwrap();
                names.push(relative.to_string_lossy().replace('\\', "/"));
            }
        }
    }

    /// Generator, not a check: rewrites the committed binary from the parts
    /// through OxDoc's conformance zipper, then re-opens what it wrote. It is
    /// `#[ignore]`d so the normal suite never writes into the repo; run it
    /// manually whenever the parts change:
    ///
    /// `cargo test -p dnacalc-host-core --offline regenerate_w011_fixture_binary_from_parts -- --ignored`
    #[test]
    #[ignore = "generator: rewrites fixtures/w011/a1_times_three.xlsx from parts/; run with --ignored when the parts change"]
    fn regenerate_w011_fixture_binary_from_parts() {
        let parts_dir = w011_fixture_parts_dir();
        let xlsx_path = w011_fixture_xlsx_path();
        oxdoc_conformance::materialize_fixture_parts_to_xlsx(&parts_dir, &xlsx_path)
            .unwrap_or_else(|err| {
                panic!(
                    "failed to materialize {} from {}: {err}",
                    xlsx_path.display(),
                    parts_dir.display()
                )
            });
        println!(
            "wrote {} ({} bytes) from {}",
            xlsx_path.display(),
            w011_committed_xlsx_bytes().len(),
            parts_dir.display()
        );
        // What we wrote must open through OxDoc exactly like the parts do.
        let written = open_full(&w011_committed_xlsx_bytes());
        let from_parts = open_full(&w011_fixture_bytes());
        assert_eq!(
            written.source_context.events(),
            from_parts.source_context.events()
        );
    }

    /// Generator for the Manual twin (dtc-j7n8.13), the same shape as
    /// `regenerate_w011_fixture_binary_from_parts`: rewrites
    /// `fixtures/w011/a1_times_three_manual.xlsx` from its parts through
    /// OxDoc's conformance zipper, then re-opens what it wrote. `#[ignore]`d
    /// so the normal suite never writes into the repo:
    ///
    /// `cargo test -p dnacalc-host-core --offline regenerate_w011_manual_fixture_binary_from_parts -- --ignored`
    #[test]
    #[ignore = "generator: rewrites fixtures/w011/a1_times_three_manual.xlsx from parts/; run with --ignored when the parts change"]
    fn regenerate_w011_manual_fixture_binary_from_parts() {
        let parts_dir = w011_manual_fixture_parts_dir();
        let xlsx_path = w011_manual_fixture_xlsx_path();
        oxdoc_conformance::materialize_fixture_parts_to_xlsx(&parts_dir, &xlsx_path)
            .unwrap_or_else(|err| {
                panic!(
                    "failed to materialize {} from {}: {err}",
                    xlsx_path.display(),
                    parts_dir.display()
                )
            });
        println!(
            "wrote {} ({} bytes) from {}",
            xlsx_path.display(),
            w011_manual_committed_xlsx_bytes().len(),
            parts_dir.display()
        );
        let written = open_full(&w011_manual_committed_xlsx_bytes());
        let from_parts = open_full(&w011_manual_fixture_bytes());
        assert_eq!(
            written.source_context.events(),
            from_parts.source_context.events()
        );
        assert_eq!(
            header_calc_mode(written.source_context.events()),
            CalcMode::Manual
        );
    }

    /// Generator for the cross-sheet fixture (dtc-j7n8.14), the same shape
    /// as `regenerate_w011_fixture_binary_from_parts`: rewrites
    /// `fixtures/w011/cross_sheet.xlsx` from its parts through OxDoc's
    /// conformance zipper, then re-opens what it wrote. `#[ignore]`d so the
    /// normal suite never writes into the repo:
    ///
    /// `cargo test -p dnacalc-host-core --offline regenerate_w011_cross_sheet_fixture_binary_from_parts -- --ignored`
    #[test]
    #[ignore = "generator: rewrites fixtures/w011/cross_sheet.xlsx from parts/; run with --ignored when the parts change"]
    fn regenerate_w011_cross_sheet_fixture_binary_from_parts() {
        let parts_dir = w011_cross_sheet_fixture_parts_dir();
        let xlsx_path = w011_cross_sheet_fixture_xlsx_path();
        oxdoc_conformance::materialize_fixture_parts_to_xlsx(&parts_dir, &xlsx_path)
            .unwrap_or_else(|err| {
                panic!(
                    "failed to materialize {} from {}: {err}",
                    xlsx_path.display(),
                    parts_dir.display()
                )
            });
        println!(
            "wrote {} ({} bytes) from {}",
            xlsx_path.display(),
            w011_cross_sheet_committed_xlsx_bytes().len(),
            parts_dir.display()
        );
        let written = open_full(&w011_cross_sheet_committed_xlsx_bytes());
        let from_parts = open_full(&w011_cross_sheet_fixture_bytes());
        assert_eq!(
            written.source_context.events(),
            from_parts.source_context.events()
        );
        assert_eq!(
            raw_sheet_cells(&written, "Sheet2").len(),
            1,
            "the written package carries Sheet2"
        );
    }
}
