//! Demo-workbook builder — the seed content the app shows on first mount,
//! before any file is loaded.
//!
//! [`build_demo_workbook`] stands up a small two-sheet workbook that exercises
//! the parts a front-end most wants to demonstrate immediately: plain literals,
//! same-sheet formulas that recompute off them, and a cross-sheet formula. It
//! is authored entirely through the universal entry verb
//! ([`WorkbookSession::enter_grid_cell`]) — the same path a user's keystrokes
//! take — so the demo content is indistinguishable from hand-entered content
//! and recalculates live through the engine's normal seed path.

use crate::workbook::{WorkbookSession, WorkbookSessionError};

/// Build the demo workbook: two sheets, a small numeric column with dependent
/// formulas on `Sheet1`, and a cross-sheet formula on `Sheet2`.
///
/// - `Sheet1`: `A1..A5` = literals `1..5`; `B1..B5` = `=A{r}*10` (so `B1 = 10`,
///   `B5 = 50`).
/// - `Sheet2`: `A1` = `=Sheet1!A1+Sheet1!A5` (a cross-sheet formula, `= 6`),
///   plus two literals `B1 = 100`, `B2 = 200`.
///
/// Every cell is authored through [`WorkbookSession::enter_grid_cell`], so the
/// literal/formula classification is the engine's (OxFml's) — never a
/// skin-side leading-`=` guess — and dependents recompute immediately under
/// the default Automatic calc mode.
///
/// `result_large_err`: propagates [`WorkbookSessionError`] by value, matching
/// the by-value error convention across host-core (`workbook.rs`).
#[allow(clippy::result_large_err)]
pub fn build_demo_workbook() -> Result<WorkbookSession, WorkbookSessionError> {
    let mut session = WorkbookSession::create("workbook:demo")?;

    let sheet1 = session.add_sheet("Sheet1")?;
    let sheet2 = session.add_sheet("Sheet2")?;

    // Sheet1: A1..A5 literals, B1..B5 = A{r}*10.
    for row in 1..=5u32 {
        session.enter_grid_cell(sheet1, row, 1, &row.to_string())?;
        session.enter_grid_cell(sheet1, row, 2, &format!("=A{row}*10"))?;
    }

    // Sheet2: a cross-sheet formula plus a couple of literals.
    session.enter_grid_cell(sheet2, 1, 1, "=Sheet1!A1+Sheet1!A5")?;
    session.enter_grid_cell(sheet2, 1, 2, "100")?;
    session.enter_grid_cell(sheet2, 2, 2, "200")?;

    Ok(session)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxfunc_core::value::CalcValue;

    /// The demo builder produces exactly two sheets, and the on-sheet formulas
    /// and the cross-sheet formula have all computed under Automatic mode.
    #[test]
    fn build_demo_workbook_seeds_two_sheets_with_live_formulas() {
        let session = build_demo_workbook().unwrap();
        let sheets = session.sheets().unwrap();
        assert_eq!(sheets.len(), 2, "the demo has exactly two sheets");
        assert_eq!(sheets[0].display_name, "Sheet1");
        assert_eq!(sheets[1].display_name, "Sheet2");

        let sheet1 = sheets[0].node_id;
        let sheet2 = sheets[1].node_id;

        // Sheet1 literals and dependent formulas.
        assert_eq!(
            session.grid_cell_value(sheet1, 1, 1).unwrap(),
            Some(CalcValue::number(1.0)),
            "Sheet1!A1 = 1"
        );
        assert_eq!(
            session.grid_cell_value(sheet1, 1, 2).unwrap(),
            Some(CalcValue::number(10.0)),
            "Sheet1!B1 = A1*10 = 10"
        );
        assert_eq!(
            session.grid_cell_value(sheet1, 5, 2).unwrap(),
            Some(CalcValue::number(50.0)),
            "Sheet1!B5 = A5*10 = 50"
        );

        // Sheet2 cross-sheet formula: Sheet1!A1 + Sheet1!A5 = 1 + 5 = 6.
        assert_eq!(
            session.grid_cell_value(sheet2, 1, 1).unwrap(),
            Some(CalcValue::number(6.0)),
            "Sheet2!A1 = Sheet1!A1 + Sheet1!A5 = 6"
        );
    }
}
