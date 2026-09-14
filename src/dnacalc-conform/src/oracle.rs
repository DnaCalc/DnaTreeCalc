//! The oracle side: Excel's own answers, read from OxDoc's `DocumentEvent`
//! stream and nothing else.
//!
//! Every formula cell an Excel-saved workbook carries also carries Excel's
//! computed result as `<v>` (`Foundation/notes/PROGRAM_INVESTIGATION_2026-09-10.md`
//! §1, "The decisive discovery"). OxDoc surfaces that as
//! `CellPayload::Formula { text, cached }` inside `CellChunk` events. This
//! module walks the stream once and produces, per sheet, every formula cell
//! with its cached value and the `ca="1"` (`calculate_cell`) marker from the
//! matching `FormulaTopology` record.
//!
//! The denominator rule (dtc-5zet.6): `total_formula_cells` is the number of
//! `CellPayload::Formula` payloads in the stream — **never** the engine's
//! authored view, where a rejected formula reads back as `Empty`.

use std::collections::BTreeMap;

use oxcalc_core::oxdoc_ingest::map_biff_error_code;
use oxdoc_model::{CalcMode, CellPayload, DocumentEvent, FormulaRecordKind, PackedCellAddr};
use oxfunc_core::value::{CalcValue, ExcelText, WorksheetErrorCode};

/// The file-level formula kind of a cell, from its `FormulaTopology` record
/// (`<f t="...">`). Drives the `NotCalcModeled` refinement of a `FileCached`
/// pin: OxCalc's ingest routes `DataTable` and `Unknown` records around the
/// calc graph and publishes their cache only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OracleFormulaKind {
    Normal,
    Shared,
    Array,
    DataTable,
    Unknown,
    /// The `CellChunk` carried a formula payload but no `FormulaTopology`
    /// record named the address (a stream the reader emitted without
    /// topology, or a reader gap). Treated like `Normal` by the ladder.
    NoTopologyRecord,
}

/// One formula cell exactly as the file carries it.
#[derive(Debug, Clone, PartialEq)]
pub struct OracleFormulaCell {
    /// 1-based row.
    pub row: u32,
    /// 1-based column.
    pub col: u32,
    /// Shared-formula region id when the cell is a region member/anchor.
    pub region: Option<u32>,
    /// The formula text as the file stores it (no leading `=`); `None` for a
    /// shared-region follower, which carries only its cache.
    pub text: Option<String>,
    /// Excel's cached result (`<v>`), as OxDoc decoded it. `None` when the
    /// file carried no `<v>` for the cell — which is also how OxDoc reports
    /// an empty-string result (`<v></v>` decodes to no cache).
    pub cached: Option<CellPayload>,
    /// `ca="1"` on the `<f>` element: Excel marks the cell as always
    /// calculated (volatile).
    pub calculate_cell: bool,
    pub kind: OracleFormulaKind,
}

impl OracleFormulaCell {
    /// The address in OxCalc's `R{row}C{col}` notation — the key
    /// `WorkbookLoadReport::bind_degradations` / `external_reference_pins`
    /// use.
    #[must_use]
    pub fn r1c1(&self) -> String {
        format!("R{}C{}", self.row, self.col)
    }

    /// The address in A1 notation, for people reading the artifact.
    #[must_use]
    pub fn a1(&self) -> String {
        format!("{}{}", column_letters(self.col), self.row)
    }
}

/// One sheet's oracle: its formula cells plus the cell census.
#[derive(Debug, Clone, PartialEq)]
pub struct OracleSheet {
    pub sheet_id: u32,
    pub name: String,
    /// Every non-empty cell the `CellChunk` events carried (literals and
    /// formulas alike).
    pub cells: u64,
    pub formula_cells: Vec<OracleFormulaCell>,
}

/// The whole workbook's oracle.
#[derive(Debug, Clone, PartialEq)]
pub struct Oracle {
    /// The `<calcPr calcMode>` the header carried, when a header was seen.
    pub calc_mode: Option<CalcMode>,
    /// The shared-string table, for `CellPayload::SharedText` caches.
    pub string_table: Vec<String>,
    pub sheets: Vec<OracleSheet>,
}

/// Why a cached payload could not be resolved to a typed value. Each is an
/// oracle-side reader ceiling, never an engine fact: the cell is `Excluded`
/// with the reason, not scored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OracleValueGap {
    /// OxDoc decodes only the seven classic error literals; anything else
    /// (`#SPILL!`, `#CALC!`, `#GETTING_DATA`, ...) arrives as byte `255`,
    /// which OxCalc's ingest maps to `#VALUE!` **flagged unknown**. Comparing
    /// against that would launder a modern error into `#VALUE!`.
    UnknownErrorByte(u8),
    /// A `SharedText` index outside the string table.
    SharedStringOutOfRange(u32),
    /// A payload shape that is not a cached value (`Empty`, a nested
    /// `Formula`, `RichStub`).
    UnsupportedPayload(&'static str),
}

impl Oracle {
    /// Walk the event stream once. Sheet order is stream order (the same
    /// order OxCalc's ingest creates sheet nodes in).
    #[must_use]
    pub fn from_events(events: &[DocumentEvent]) -> Self {
        let mut calc_mode = None;
        let mut string_table = Vec::new();
        let mut sheets: Vec<OracleSheet> = Vec::new();
        let mut open_sheet: Option<usize> = None;
        // (sheet_id, address) -> (calculate_cell, kind), from FormulaTopology.
        let mut topology: BTreeMap<(u32, PackedCellAddr), (bool, OracleFormulaKind)> =
            BTreeMap::new();

        for event in events {
            match event {
                DocumentEvent::WorkbookHeader(header) => calc_mode = Some(header.calc_mode),
                DocumentEvent::StringTable(entries) => {
                    string_table = entries.iter().map(|entry| entry.text.clone()).collect();
                }
                DocumentEvent::SheetBegin(sheet) => {
                    open_sheet = Some(sheets.len());
                    sheets.push(OracleSheet {
                        sheet_id: sheet.sheet_id,
                        name: sheet.name.clone(),
                        cells: 0,
                        formula_cells: Vec::new(),
                    });
                }
                DocumentEvent::SheetEnd { .. } => open_sheet = None,
                DocumentEvent::FormulaTopology(topo) => {
                    for record in &topo.records {
                        let kind = match &record.kind {
                            FormulaRecordKind::Normal => OracleFormulaKind::Normal,
                            FormulaRecordKind::Shared(_) => OracleFormulaKind::Shared,
                            FormulaRecordKind::Array(_) => OracleFormulaKind::Array,
                            FormulaRecordKind::DataTable(_) => OracleFormulaKind::DataTable,
                            FormulaRecordKind::Unknown { .. } => OracleFormulaKind::Unknown,
                        };
                        topology.insert(
                            (record.sheet_id, record.address),
                            (record.attrs.calculate_cell == Some(true), kind),
                        );
                    }
                }
                DocumentEvent::CellChunk(chunk) => {
                    let Some(index) = open_sheet else {
                        // A chunk outside any sheet is a malformed stream; OxCalc's
                        // ingest rejects it. Nothing to attribute it to here.
                        continue;
                    };
                    let sheet = &mut sheets[index];
                    for (addr, payload) in &chunk.cells {
                        if matches!(payload, CellPayload::Empty) {
                            continue;
                        }
                        sheet.cells += 1;
                        if let CellPayload::Formula {
                            region,
                            text,
                            cached,
                        } = payload
                        {
                            sheet.formula_cells.push(OracleFormulaCell {
                                row: addr.row_one_based(),
                                col: addr.col_one_based(),
                                region: *region,
                                text: text.clone(),
                                cached: cached.as_deref().cloned(),
                                // Filled from the topology map below.
                                calculate_cell: false,
                                kind: OracleFormulaKind::NoTopologyRecord,
                            });
                        }
                    }
                }
                _ => {}
            }
        }

        for sheet in &mut sheets {
            for cell in &mut sheet.formula_cells {
                let key = (
                    sheet.sheet_id,
                    PackedCellAddr::from_zero_based(cell.row - 1, cell.col - 1),
                );
                if let Some((calculate_cell, kind)) = topology.get(&key) {
                    cell.calculate_cell = *calculate_cell;
                    cell.kind = *kind;
                }
            }
        }

        Self {
            calc_mode,
            string_table,
            sheets,
        }
    }

    /// The coverage denominator: every `CellPayload::Formula` in the stream.
    #[must_use]
    pub fn formula_cell_count(&self) -> u64 {
        self.sheets
            .iter()
            .map(|sheet| sheet.formula_cells.len() as u64)
            .sum()
    }

    /// Every non-empty cell in the stream.
    #[must_use]
    pub fn cell_count(&self) -> u64 {
        self.sheets.iter().map(|sheet| sheet.cells).sum()
    }

    /// Resolve Excel's cached payload to the typed value the engine's own
    /// ingest would have published for it — the same `map_biff_error_code`
    /// table and the same shared-string lookup — or name the gap.
    pub fn resolve_cached(&self, payload: &CellPayload) -> Result<CalcValue, OracleValueGap> {
        match payload {
            CellPayload::Number(n) => Ok(CalcValue::number(*n)),
            CellPayload::Bool(b) => Ok(CalcValue::logical(*b)),
            CellPayload::Error(code) => {
                let (mapped, known) = map_biff_error_code(*code);
                if known {
                    Ok(CalcValue::error(mapped))
                } else {
                    Err(OracleValueGap::UnknownErrorByte(*code))
                }
            }
            CellPayload::SharedText(index) => self
                .string_table
                .get(*index as usize)
                .map(|text| CalcValue::text(ExcelText::from_interop_assignment(text)))
                .ok_or(OracleValueGap::SharedStringOutOfRange(*index)),
            CellPayload::InlineText(text) => {
                Ok(CalcValue::text(ExcelText::from_interop_assignment(text)))
            }
            CellPayload::Empty => Err(OracleValueGap::UnsupportedPayload("Empty")),
            CellPayload::Formula { .. } => Err(OracleValueGap::UnsupportedPayload("Formula")),
            CellPayload::RichStub(_) => Err(OracleValueGap::UnsupportedPayload("RichStub")),
        }
    }
}

/// Excel's display literal for an error code — for the artifact's
/// human-readable value column only; comparison is by code.
#[must_use]
pub fn error_literal(code: WorksheetErrorCode) -> &'static str {
    match code {
        WorksheetErrorCode::Null => "#NULL!",
        WorksheetErrorCode::Div0 => "#DIV/0!",
        WorksheetErrorCode::Value => "#VALUE!",
        WorksheetErrorCode::Ref => "#REF!",
        WorksheetErrorCode::Name => "#NAME?",
        WorksheetErrorCode::Num => "#NUM!",
        WorksheetErrorCode::NA => "#N/A",
        WorksheetErrorCode::Busy => "#BUSY!",
        WorksheetErrorCode::GettingData => "#GETTING_DATA",
        WorksheetErrorCode::Spill => "#SPILL!",
        WorksheetErrorCode::Calc => "#CALC!",
        WorksheetErrorCode::Field => "#FIELD!",
        WorksheetErrorCode::Blocked => "#BLOCKED!",
        WorksheetErrorCode::Connect => "#CONNECT!",
    }
}

/// 1-based column index to letters (`1 -> A`, `27 -> AA`).
#[must_use]
pub fn column_letters(mut col: u32) -> String {
    let mut letters = Vec::new();
    while col > 0 {
        let rem = (col - 1) % 26;
        letters.push((b'A' + rem as u8) as char);
        col = (col - 1) / 26;
    }
    letters.iter().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxdoc_model::{CellChunk, DateSystem, SharedStringEntry, SheetRef, WorkbookHeader};

    #[test]
    fn column_letters_cover_the_single_and_double_letter_ranges() {
        assert_eq!(column_letters(1), "A");
        assert_eq!(column_letters(26), "Z");
        assert_eq!(column_letters(27), "AA");
        assert_eq!(column_letters(52), "AZ");
        assert_eq!(column_letters(53), "BA");
        assert_eq!(column_letters(16_384), "XFD");
    }

    /// A hand-built stream: two sheets, one literal + two formulas (one with
    /// a cache, one without) on the first, one formula on the second. The
    /// denominator counts every `Formula` payload; the literal is a cell
    /// but not a formula cell.
    #[test]
    fn from_events_counts_every_formula_payload_and_keeps_stream_order() {
        let events = vec![
            DocumentEvent::WorkbookHeader(WorkbookHeader::new(
                DateSystem::Date1900,
                CalcMode::Manual,
            )),
            DocumentEvent::StringTable(vec![SharedStringEntry {
                text: "hello".to_string(),
            }]),
            DocumentEvent::SheetBegin(SheetRef {
                sheet_id: 1,
                name: "First".to_string(),
            }),
            DocumentEvent::CellChunk(CellChunk {
                row_band: 0,
                cells: vec![
                    (
                        PackedCellAddr::from_one_based(1, 1).unwrap(),
                        CellPayload::Number(7.0),
                    ),
                    (
                        PackedCellAddr::from_one_based(1, 2).unwrap(),
                        CellPayload::Formula {
                            region: None,
                            text: Some("A1*3".to_string()),
                            cached: Some(Box::new(CellPayload::Number(21.0))),
                        },
                    ),
                    (
                        PackedCellAddr::from_one_based(2, 2).unwrap(),
                        CellPayload::Formula {
                            region: None,
                            text: Some("\"\"".to_string()),
                            cached: None,
                        },
                    ),
                ],
            }),
            DocumentEvent::SheetEnd { sheet_id: 1 },
            DocumentEvent::SheetBegin(SheetRef {
                sheet_id: 2,
                name: "Second".to_string(),
            }),
            DocumentEvent::CellChunk(CellChunk {
                row_band: 0,
                cells: vec![(
                    PackedCellAddr::from_one_based(3, 1).unwrap(),
                    CellPayload::Formula {
                        region: None,
                        text: Some("First!B1".to_string()),
                        cached: Some(Box::new(CellPayload::SharedText(0))),
                    },
                )],
            }),
            DocumentEvent::SheetEnd { sheet_id: 2 },
        ];
        let oracle = Oracle::from_events(&events);
        assert_eq!(oracle.calc_mode, Some(CalcMode::Manual));
        assert_eq!(oracle.cell_count(), 4);
        assert_eq!(oracle.formula_cell_count(), 3);
        assert_eq!(oracle.sheets.len(), 2);
        assert_eq!(oracle.sheets[0].name, "First");
        assert_eq!(oracle.sheets[0].cells, 3);
        assert_eq!(oracle.sheets[0].formula_cells.len(), 2);
        let b1 = &oracle.sheets[0].formula_cells[0];
        assert_eq!((b1.row, b1.col), (1, 2));
        assert_eq!(b1.a1(), "B1");
        assert_eq!(b1.r1c1(), "R1C2");
        assert_eq!(b1.kind, OracleFormulaKind::NoTopologyRecord);
        assert!(!b1.calculate_cell);
        assert_eq!(oracle.sheets[0].formula_cells[1].cached, None);
        let second = &oracle.sheets[1].formula_cells[0];
        assert_eq!(second.a1(), "A3");
        assert_eq!(
            oracle.resolve_cached(second.cached.as_ref().unwrap()),
            Ok(CalcValue::text(ExcelText::from_interop_assignment("hello")))
        );
    }

    #[test]
    fn resolve_cached_maps_known_error_bytes_and_refuses_unknown_ones() {
        let oracle = Oracle::from_events(&[]);
        assert_eq!(
            oracle.resolve_cached(&CellPayload::Error(29)),
            Ok(CalcValue::error(WorksheetErrorCode::Name))
        );
        assert_eq!(
            oracle.resolve_cached(&CellPayload::Error(7)),
            Ok(CalcValue::error(WorksheetErrorCode::Div0))
        );
        // OxDoc's byte for every non-classic literal (#SPILL!, #CALC!, ...).
        assert_eq!(
            oracle.resolve_cached(&CellPayload::Error(255)),
            Err(OracleValueGap::UnknownErrorByte(255))
        );
        assert_eq!(
            oracle.resolve_cached(&CellPayload::SharedText(3)),
            Err(OracleValueGap::SharedStringOutOfRange(3))
        );
        assert_eq!(
            oracle.resolve_cached(&CellPayload::Empty),
            Err(OracleValueGap::UnsupportedPayload("Empty"))
        );
        assert_eq!(
            oracle.resolve_cached(&CellPayload::Bool(true)),
            Ok(CalcValue::logical(true))
        );
    }
}
