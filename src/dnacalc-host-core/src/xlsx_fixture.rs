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
//! Compiled only under `cfg(test)`: `oxdoc_conformance` is a dev-dependency
//! and must never become a normal one.

use std::path::{Path, PathBuf};

/// Walk from this crate's manifest dir (`src/dnacalc-host-core`) to the repo
/// root. Relative on purpose — no absolute paths in tests.
const REPO_ROOT_FROM_MANIFEST: &str = "../..";

/// Repo-relative location of the readable fixture parts (source of truth).
pub(crate) const W011_FIXTURE_PARTS_REL: &str = "fixtures/w011/a1_times_three/parts";

/// Repo-relative location of the committed binary fixture (app click-through).
pub(crate) const W011_FIXTURE_XLSX_REL: &str = "fixtures/w011/a1_times_three.xlsx";

/// The exact part names the fixture consists of (zip entry names, sorted).
pub(crate) const W011_FIXTURE_PART_NAMES: [&str; 5] = [
    "[Content_Types].xml",
    "_rels/.rels",
    "xl/_rels/workbook.xml.rels",
    "xl/workbook.xml",
    "xl/worksheets/sheet1.xml",
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

/// The `a1_times_three` workbook as `.xlsx` bytes, zipped in memory from the
/// committed parts. This is the byte source every W011 host-core test opens;
/// later W011 beads (open command, ingest, edit/recalc, save/reopen) reuse it.
pub(crate) fn w011_fixture_bytes() -> Vec<u8> {
    let parts_dir = w011_fixture_parts_dir();
    oxdoc_conformance::read_fixture_parts_as_xlsx(&parts_dir).unwrap_or_else(|err| {
        panic!(
            "failed to zip the W011 fixture parts under {}: {err}",
            parts_dir.display()
        )
    })
}

/// The committed binary fixture's bytes, read from disk.
pub(crate) fn w011_committed_xlsx_bytes() -> Vec<u8> {
    let path = w011_fixture_xlsx_path();
    std::fs::read(&path).unwrap_or_else(|err| {
        panic!(
            "failed to read the committed W011 fixture {}: {err}",
            path.display()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxdoc_model::{
        CachedValueProvenance, CalcMode, CellChunk, CellPayload, DateSystem, DocumentEvent,
        FidelityDisposition, FormulaCachedValueState, FormulaRecordKind, FormulaTextKind,
        FormulaTopology, PackedCellAddr, ProjectionStatus, SheetRef, WorkbookHeader,
    };
    use oxdoc_xlsx::{HostOwnedXlsxSource, LoadProfile, open_host_owned_xlsx_source};
    use std::io::Cursor;

    fn a1() -> PackedCellAddr {
        PackedCellAddr::from_one_based(1, 1).unwrap()
    }

    fn b1() -> PackedCellAddr {
        PackedCellAddr::from_one_based(1, 2).unwrap()
    }

    /// Open `.xlsx` bytes through OxDoc under the full profile — the profile
    /// the W011 host lifecycle uses, because only `full()` materializes the
    /// `FormulaTopology` the later save needs.
    fn open_full(bytes: &[u8]) -> HostOwnedXlsxSource {
        open_host_owned_xlsx_source(Cursor::new(bytes), LoadProfile::full())
            .unwrap_or_else(|err| panic!("OxDoc rejected the W011 fixture bytes: {err}"))
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
}
