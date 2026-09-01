//! Defined-names projection — the host-core fill of the Skin IR's defined-name
//! catalog (H4, `FRONTEND_UI_DESIGN_AND_ROUTEMAP.md` §A.2/§A.3) from OxCalc's
//! `document_defined_names` readout-all verb, plus the `SetDefinedName`/
//! `RenameDefinedName`/`DeleteDefinedName` intent-level writes (§A.2's verb
//! map: `set_workbook_defined_name`/`set_sheet_defined_name`/the dynamic
//! variants, `rename_defined_name`, `delete_defined_name`).
//!
//! `WorkspaceState::defined_names` (skin-IR `workspace.rs`) is a serde-clean
//! mirror of OxCalc's `DefinedNameReadout` (`consumer.rs`): scope, name text,
//! target (a static rect or a dynamic formula's source text), and the
//! `is_dynamic` bit. `document_defined_names` reads one grid node's authored
//! names (both workbook- and sheet-scoped names authored *on that node*), so
//! the workbook-wide catalog is the union across every sheet's grid,
//! deduplicated by `(scope, name)` — a workbook-scoped name authored on one
//! sheet is visible from every other sheet's grid too (R3.5 precedence), so a
//! naive per-sheet concatenation would double-list it.

use dnacalc_skin_ir::{
    DefinedNameProjection, DefinedNameScopeProjection, DefinedNameTargetProjection,
    DefinedNamesProjection, GridRectProjection,
};
use oxcalc_core::consumer::{
    DefinedNameReadout, DefinedNameTargetReadout, OxCalcDocumentError, OxCalcTreeDefinedNameScope,
};
use oxcalc_core::grid::geometry::GridRect;
use oxcalc_core::structural::TreeNodeId;

use crate::workbook::{WorkbookSession, WorkbookSessionError, sheet_grid_node_id};

/// The `_names` backing sheet's display name (B.7/F.4 DECIDED, owner-ratified
/// 2026-07-05): a hidden-in-notebook, append-only sheet, column A, one cell
/// per static name — created lazily on first name creation. It is an
/// **ordinary sheet to the engine** (B.7's rationale: zero new engine
/// features, round-trips as a perfectly normal sheet+names workbook); "hidden"
/// is a notebook-UI convention by the `_` prefix, not an engine visibility bit
/// (none exists, `K8`'s "`_names` visible-in-K check" is the only place this
/// sheet is ever meant to surface, by design).
pub const NAMES_BACKING_SHEET: &str = "_names";

// `result_large_err`: same rationale as `workbook.rs`'s impl block — this
// module's errors wrap the same `WorkbookSessionError`/engine error by value,
// so the allow is kept consistent across the crate's single-session native/
// wasm calls rather than boxing here alone.
#[allow(clippy::result_large_err)]
impl WorkbookSession {
    /// The complete defined-name catalog (H4, §A.3): the union of
    /// `document_defined_names` across every sheet's grid, deduplicated by
    /// `(scope, name)` (a workbook-scoped name is authored on exactly one
    /// sheet's grid but is visible workbook-wide, so it is listed once).
    pub fn defined_names(&self) -> Result<DefinedNamesProjection, WorkbookSessionError> {
        // `OxCalcTreeDefinedNameScope` is not `Ord`, so the de-dup key is the
        // scope's own string discriminant (`"workbook"` or `"sheet:{id}"`)
        // paired with the name text, rather than the scope value itself.
        let mut seen = std::collections::BTreeSet::new();
        let mut entries = Vec::new();
        for row in self.sheets()? {
            for readout in self.document_defined_names_on(row.node_id)? {
                let key = (scope_dedup_key(&readout.scope), readout.name.clone());
                if seen.insert(key) {
                    entries.push(defined_name_projection(&readout));
                }
            }
        }
        Ok(DefinedNamesProjection { entries })
    }

    /// Allocate the next free backing cell for a new static name on the
    /// `_names` sheet (N3, B.7's host-core-policy convention): finds (or
    /// lazily creates) the `_names` sheet, then returns the next append-only
    /// row in column A — one past the highest row any name currently targets
    /// on that sheet, or row 1 if the sheet is fresh. **Re-derivable, no side
    /// ledger** (B.7): the "next free row" is computed fresh from
    /// [`WorkbookSession::defined_names`] every call, by scanning the already-
    /// allocated static targets whose scope's sheet is `_names` — never a
    /// counter this session persists on its own, so a session rehydrated from
    /// a snapshot allocates correctly on its very first call.
    ///
    /// This is the one function the bead's "host-core allocation helper"
    /// clause names; N3's name-creation flow calls it once per `+ name`
    /// commit, then dispatches `EnterGridCell` at the returned cell followed
    /// by `SetDefinedName{Static}` over it (the two-intent creation, §B.3's
    /// Decision paragraph — two undo steps, accepted v1).
    pub fn allocate_next_names_backing_cell(
        &mut self,
    ) -> Result<(TreeNodeId, u32), WorkbookSessionError> {
        let names_sheet = self.names_backing_sheet()?;
        let next_row = self
            .document_defined_names_on(names_sheet)?
            .iter()
            .filter_map(|readout| match &readout.target {
                DefinedNameTargetReadout::Static(rect) => Some(rect.bottom_row),
                DefinedNameTargetReadout::Dynamic { .. } => None,
            })
            .max()
            .map_or(1, |highest_row| highest_row + 1);
        Ok((names_sheet, next_row))
    }

    /// Create a workbook-scoped named value atomically (N3's `+ name`
    /// affordance, e.g. `rate = 0.065`) — the whole named-value creation
    /// host-core owns so a skin never guesses the engine's real backing-cell
    /// address (a skin has no session handle to resolve the lazily-created
    /// `_names` sheet's opaque `sheet:{node}` grid id, which is why the older
    /// skin-side two-intent `EnterGridCell{grid:"_names"}` + `SetDefinedName`
    /// flow failed). Three steps, in order:
    ///
    /// 1. [`WorkbookSession::allocate_next_names_backing_cell`] — find-or-lazily-
    ///    create the `_names` sheet and pick the next append row in column A.
    /// 2. [`WorkbookSession::enter_grid_cell`] — write `value_text` into that
    ///    cell through the universal entry verb (OxFml interprets a literal vs a
    ///    leading-`=` formula; no host-side classification).
    /// 3. [`WorkbookSession::set_defined_name`] at **Workbook** scope, anchored
    ///    on the `_names` sheet, targeting the single `(row, 1)` cell.
    ///
    /// The engine's no-mutation-on-error contract holds **per verb**: a
    /// mid-sequence failure (e.g. step 3 rejecting after step 2's write landed)
    /// is possible but unlikely, and propagates to the caller — the receipt
    /// surfaces the rejection rather than silently swallowing it.
    pub fn create_named_value(
        &mut self,
        name: &str,
        value_text: &str,
    ) -> Result<(), WorkbookSessionError> {
        let (names_sheet, row) = self.allocate_next_names_backing_cell()?;
        self.enter_grid_cell(names_sheet, row, 1, value_text)?;
        let target = DefinedNameTargetIntentInput::Static(GridRectProjection {
            top_row: row,
            left_col: 1,
            bottom_row: row,
            right_col: 1,
        });
        self.set_defined_name(
            names_sheet,
            DefinedNameScopeProjection::Workbook,
            name,
            target,
        )?;
        Ok(())
    }

    /// Find the `_names` sheet by its display name, creating it (grid-backed,
    /// via [`WorkbookSession::add_sheet`]) the first time a name is created in
    /// this workbook (B.7: "created lazily"). An ordinary sheet by every
    /// engine measure — no hidden/system flag exists or is invented here.
    fn names_backing_sheet(&mut self) -> Result<TreeNodeId, WorkbookSessionError> {
        if let Some(existing) = self
            .sheets()?
            .into_iter()
            .find(|row| row.display_name == NAMES_BACKING_SHEET)
        {
            return Ok(existing.node_id);
        }
        self.add_sheet(NAMES_BACKING_SHEET)
    }

    /// One grid node's own `document_defined_names` readout, `dnacalc-host-core`-internal
    /// (the public catalog is [`WorkbookSession::defined_names`], the workbook-wide union).
    fn document_defined_names_on(
        &self,
        sheet: TreeNodeId,
    ) -> Result<Vec<DefinedNameReadout>, WorkbookSessionError> {
        Ok(self
            .context()
            .document_defined_names(self.workspace_id(), sheet)?)
    }

    /// Define (or redefine) a defined name at workbook or sheet scope (§A.2).
    /// `scope`/`target` are the skin-IR intent payload shapes; this resolves
    /// them to the engine's own scope/verb split
    /// (`set_workbook_defined_name`/`set_sheet_defined_name`/the dynamic
    /// variants) and always authors on `anchor_sheet`'s grid (the sheet a
    /// skin's names-manager UI is currently viewing; a workbook-scoped name is
    /// visible from every sheet regardless of which one authored it).
    pub fn set_defined_name(
        &mut self,
        anchor_sheet: TreeNodeId,
        scope: DefinedNameScopeProjection,
        name: impl Into<String>,
        target: DefinedNameTargetIntentInput,
    ) -> Result<(), WorkbookSessionError> {
        let name = name.into();
        let workspace_id = self.workspace_id().clone();
        match (scope, target) {
            (DefinedNameScopeProjection::Workbook, DefinedNameTargetIntentInput::Static(rect)) => {
                let rect = self.grid_rect_for(anchor_sheet, rect)?;
                self.context_mut().set_workbook_defined_name(
                    &workspace_id,
                    anchor_sheet,
                    name,
                    rect,
                )?;
            }
            (DefinedNameScopeProjection::Sheet(_), DefinedNameTargetIntentInput::Static(rect)) => {
                let rect = self.grid_rect_for(anchor_sheet, rect)?;
                self.context_mut().set_sheet_defined_name(
                    &workspace_id,
                    anchor_sheet,
                    name,
                    rect,
                )?;
            }
            (
                DefinedNameScopeProjection::Workbook,
                DefinedNameTargetIntentInput::Dynamic { source_text },
            ) => {
                self.context_mut().set_workbook_dynamic_defined_name(
                    &workspace_id,
                    anchor_sheet,
                    name,
                    &source_text,
                )?;
            }
            (
                DefinedNameScopeProjection::Sheet(_),
                DefinedNameTargetIntentInput::Dynamic { source_text },
            ) => {
                self.context_mut().set_sheet_dynamic_defined_name(
                    &workspace_id,
                    anchor_sheet,
                    name,
                    &source_text,
                )?;
            }
        }
        Ok(())
    }

    /// Rename a defined name in place (§A.2: `rename_defined_name`).
    pub fn rename_defined_name(
        &mut self,
        anchor_sheet: TreeNodeId,
        scope: DefinedNameScopeProjection,
        old_name: impl AsRef<str>,
        new_name: impl Into<String>,
    ) -> Result<(), WorkbookSessionError> {
        let engine_scope = self.engine_scope_for(anchor_sheet, &scope)?;
        let workspace_id = self.workspace_id().clone();
        self.context_mut().rename_defined_name(
            &workspace_id,
            anchor_sheet,
            engine_scope,
            old_name,
            new_name,
        )?;
        Ok(())
    }

    /// Delete a defined name (§A.2: `delete_defined_name`).
    pub fn delete_defined_name(
        &mut self,
        anchor_sheet: TreeNodeId,
        scope: DefinedNameScopeProjection,
        name: impl AsRef<str>,
    ) -> Result<(), WorkbookSessionError> {
        let engine_scope = self.engine_scope_for(anchor_sheet, &scope)?;
        let workspace_id = self.workspace_id().clone();
        self.context_mut()
            .delete_defined_name(&workspace_id, anchor_sheet, engine_scope, name)?;
        Ok(())
    }

    /// Resolve a skin-IR [`DefinedNameScopeProjection`] to the engine's own
    /// [`OxCalcTreeDefinedNameScope`]. `Workbook` maps directly; `Sheet` reuses
    /// `anchor_sheet`'s own scope (§A.2: skins never compose the raw
    /// engine `sheet_id` string themselves — the projection's `Sheet(NodeId)`
    /// is the same stable grid address `EnterGridCell` already uses, and the
    /// engine's `sheet_scope_of_node`-equivalent is re-derived here from that
    /// node).
    fn engine_scope_for(
        &self,
        anchor_sheet: TreeNodeId,
        scope: &DefinedNameScopeProjection,
    ) -> Result<OxCalcTreeDefinedNameScope, WorkbookSessionError> {
        match scope {
            DefinedNameScopeProjection::Workbook => Ok(OxCalcTreeDefinedNameScope::Workbook),
            DefinedNameScopeProjection::Sheet(_) => {
                // Re-derive from the authored grid the same way the engine's
                // own `sheet_scope_of_node` does: the sheet's own `sheet_id`,
                // which is exactly this stable grid node id's string.
                Ok(OxCalcTreeDefinedNameScope::Sheet(
                    sheet_grid_node_id(anchor_sheet).as_str().to_string(),
                ))
            }
        }
    }

    /// Build a [`GridRect`] over `anchor_sheet`'s own grid namespace from a
    /// projected 1-based rectangle (§A.2: the skin never composes the raw
    /// engine `GridRect` itself). The workbook half comes from the session's
    /// token authority (`workbook_token`, dtc-j7n8.4): on an opened workbook
    /// the engine keys grids under `book:{workspace}`, and a rect built from
    /// the bare workspace id would resolve to nothing — silently.
    fn grid_rect_for(
        &self,
        anchor_sheet: TreeNodeId,
        rect: GridRectProjection,
    ) -> Result<GridRect, WorkbookSessionError> {
        GridRect::new(
            self.workbook_token(),
            sheet_grid_node_id(anchor_sheet).as_str().to_string(),
            rect.top_row,
            rect.left_col,
            rect.bottom_row,
            rect.right_col,
            self.bounds(),
        )
        .map_err(|_| WorkbookSessionError::SheetNotGridBacked { node: anchor_sheet })
    }
}

/// The target payload [`WorkbookSession::set_defined_name`] accepts —
/// host-core's own shape (a projected [`GridRectProjection`] rather than the
/// intent-layer [`dnacalc_skin_ir::DefinedNameTargetIntent`]) so this module
/// does not have to depend on `intent.rs`'s dispatch-only type. `lib.rs`'s
/// dispatcher converts between the two.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DefinedNameTargetIntentInput {
    Static(GridRectProjection),
    Dynamic { source_text: String },
}

/// A `BTreeSet`-usable dedup key for [`OxCalcTreeDefinedNameScope`] (which is
/// not itself `Ord`): the scope's own discriminant string, distinguishing
/// `Workbook` from each individually-scoped sheet.
fn scope_dedup_key(scope: &OxCalcTreeDefinedNameScope) -> String {
    match scope {
        OxCalcTreeDefinedNameScope::Workbook => "workbook".to_string(),
        OxCalcTreeDefinedNameScope::Sheet(sheet_id) => format!("sheet:{sheet_id}"),
    }
}

fn defined_name_projection(readout: &DefinedNameReadout) -> DefinedNameProjection {
    DefinedNameProjection {
        scope: defined_name_scope_projection(&readout.scope),
        name: readout.name.clone(),
        target: defined_name_target_projection(&readout.target),
        is_dynamic: readout.is_dynamic,
    }
}

fn defined_name_scope_projection(scope: &OxCalcTreeDefinedNameScope) -> DefinedNameScopeProjection {
    match scope {
        OxCalcTreeDefinedNameScope::Workbook => DefinedNameScopeProjection::Workbook,
        // The engine's `sheet_id` string is exactly the same stable grid
        // address `sheet_grid_node_id`/`parse_sheet_grid_node_id` already use
        // (`WorkbookSession::add_sheet` derives it as `sheet:{node_id}`), so
        // it round-trips into a `NodeId` by identity — never a second lookup.
        OxCalcTreeDefinedNameScope::Sheet(sheet_id) => {
            DefinedNameScopeProjection::Sheet(dnacalc_skin_ir::NodeId::new(sheet_id.clone()))
        }
    }
}

fn defined_name_target_projection(
    target: &DefinedNameTargetReadout,
) -> DefinedNameTargetProjection {
    match target {
        DefinedNameTargetReadout::Static(rect) => {
            DefinedNameTargetProjection::Static(GridRectProjection {
                top_row: rect.top_row,
                left_col: rect.left_col,
                bottom_row: rect.bottom_row,
                right_col: rect.right_col,
            })
        }
        DefinedNameTargetReadout::Dynamic { source_text, .. } => {
            DefinedNameTargetProjection::Dynamic {
                source_text: source_text.clone(),
            }
        }
    }
}

/// Map a rejected defined-name write to its typed [`dnacalc_skin_ir::IntentError`]
/// (§A.4): `DefinedNameCollidesWithTreeNode` is the sole typed duplicate-name
/// rejection this engine surface has (a workbook-scope name colliding with a
/// root tree node's symbol, D2 §4.3 rule 4 / V8) — plain redefinition of an
/// existing name (same scope+text) is the engine's own last-write-wins
/// semantics and is never a rejection, so this map does not fabricate one.
/// Everything else falls through to the documented "unknown/unmapped"
/// decision, never a panic.
#[must_use]
pub fn present_defined_name_rejection(
    error: &WorkbookSessionError,
) -> dnacalc_skin_ir::IntentError {
    match error {
        WorkbookSessionError::OxCalc(OxCalcDocumentError::DefinedNameCollidesWithTreeNode {
            name,
            ..
        }) => dnacalc_skin_ir::IntentError::DefinedNameCollision { name: name.clone() },
        other => dnacalc_skin_ir::IntentError::GenericEngineRejection {
            debug: format!("{other:?}"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxfunc_core::value::CalcValue;

    fn workbook_with_sheet(workspace_id: &str) -> (WorkbookSession, TreeNodeId) {
        let mut session = WorkbookSession::create(workspace_id).unwrap();
        let sheet = session.add_sheet("Sheet1").unwrap();
        (session, sheet)
    }

    fn rect(top_row: u32, left_col: u32, bottom_row: u32, right_col: u32) -> GridRectProjection {
        GridRectProjection {
            top_row,
            left_col,
            bottom_row,
            right_col,
        }
    }

    /// H4 acceptance (2), define half: `SetDefinedName` (static, workbook
    /// scope) → the catalog lists it with the exact scope + rect.
    #[test]
    fn set_defined_name_static_workbook_scope_lists_scope_and_rect() {
        let (mut session, sheet) = workbook_with_sheet("workbook:h4-set-static");
        session
            .set_grid_cell_value(sheet, 2, 2, CalcValue::number(5.0))
            .unwrap();

        session
            .set_defined_name(
                sheet,
                DefinedNameScopeProjection::Workbook,
                "Rate",
                DefinedNameTargetIntentInput::Static(rect(2, 2, 2, 2)),
            )
            .unwrap();

        let catalog = session.defined_names().unwrap();
        assert_eq!(catalog.entries.len(), 1);
        let entry = &catalog.entries[0];
        assert_eq!(entry.name, "Rate");
        assert_eq!(entry.scope, DefinedNameScopeProjection::Workbook);
        assert!(!entry.is_dynamic);
        assert_eq!(
            entry.target,
            DefinedNameTargetProjection::Static(rect(2, 2, 2, 2))
        );
    }

    /// H4 acceptance (2), rename half: rename -> the old name is gone and the
    /// new name is present (same target).
    #[test]
    fn rename_defined_name_removes_old_and_adds_new_preserving_target() {
        let (mut session, sheet) = workbook_with_sheet("workbook:h4-rename");
        session
            .set_defined_name(
                sheet,
                DefinedNameScopeProjection::Workbook,
                "Rate",
                DefinedNameTargetIntentInput::Static(rect(1, 1, 1, 1)),
            )
            .unwrap();

        session
            .rename_defined_name(
                sheet,
                DefinedNameScopeProjection::Workbook,
                "Rate",
                "TaxRate",
            )
            .unwrap();

        let catalog = session.defined_names().unwrap();
        assert_eq!(
            catalog.entries.len(),
            1,
            "rename does not add a second entry"
        );
        let entry = &catalog.entries[0];
        assert_eq!(entry.name, "TaxRate");
        assert_eq!(
            entry.target,
            DefinedNameTargetProjection::Static(rect(1, 1, 1, 1)),
            "rename preserves the target"
        );
        assert!(
            !catalog.entries.iter().any(|e| e.name == "Rate"),
            "the old name is gone"
        );
    }

    /// H4 acceptance (2), delete + recreate half: delete removes the name
    /// from the catalog; a formula referencing it shows #NAME? (surfaced as
    /// the grid's own error rendering, not duplicated here); recreating the
    /// same name self-heals the dependent on the next tick.
    #[test]
    fn delete_defined_name_removes_it_and_recreate_self_heals_dependents() {
        let (mut session, sheet) = workbook_with_sheet("workbook:h4-delete-recreate");
        session
            .set_grid_cell_value(sheet, 1, 1, CalcValue::number(7.0))
            .unwrap();
        session
            .set_defined_name(
                sheet,
                DefinedNameScopeProjection::Workbook,
                "Total",
                DefinedNameTargetIntentInput::Static(rect(1, 1, 1, 1)),
            )
            .unwrap();
        session.enter_grid_cell(sheet, 2, 1, "=Total").unwrap();
        assert_eq!(
            session
                .grid_cell_value(sheet, 2, 1)
                .unwrap()
                .and_then(|v| v.as_number()),
            Some(7.0),
            "A2 resolves Total to 7 before the delete"
        );

        session
            .delete_defined_name(sheet, DefinedNameScopeProjection::Workbook, "Total")
            .unwrap();
        assert!(
            session.defined_names().unwrap().entries.is_empty(),
            "the catalog no longer lists Total"
        );
        let after_delete = session.grid_cell_value(sheet, 2, 1).unwrap().unwrap();
        assert!(
            after_delete.as_number().is_none(),
            "A2 no longer resolves to a plain number once Total is deleted (expected #NAME?-shaped error, got {after_delete:?})"
        );

        // Recreate the same name: the dependent self-heals on the next tick.
        session
            .set_defined_name(
                sheet,
                DefinedNameScopeProjection::Workbook,
                "Total",
                DefinedNameTargetIntentInput::Static(rect(1, 1, 1, 1)),
            )
            .unwrap();
        assert_eq!(
            session
                .grid_cell_value(sheet, 2, 1)
                .unwrap()
                .and_then(|v| v.as_number()),
            Some(7.0),
            "recreating Total heals the dependent back to 7"
        );
    }

    /// H4 acceptance (3): a duplicate name — a workbook-scope name colliding
    /// with a root tree node's symbol — is a typed rejection, and the catalog
    /// is unchanged.
    #[test]
    fn set_defined_name_colliding_with_tree_node_is_typed_rejection_and_catalog_unchanged() {
        let (mut session, sheet) = workbook_with_sheet("workbook:h4-collision");
        session.add_root_calc_node_for_test("Rate", "5");

        let error = session
            .set_defined_name(
                sheet,
                DefinedNameScopeProjection::Workbook,
                "Rate",
                DefinedNameTargetIntentInput::Static(rect(3, 3, 3, 3)),
            )
            .unwrap_err();
        assert!(
            matches!(
                error,
                WorkbookSessionError::OxCalc(
                    OxCalcDocumentError::DefinedNameCollidesWithTreeNode { ref name, .. }
                ) if name == "Rate"
            ),
            "expected DefinedNameCollidesWithTreeNode, got {error:?}"
        );
        assert!(
            session.defined_names().unwrap().entries.is_empty(),
            "the rejected write leaves the catalog unchanged"
        );

        match present_defined_name_rejection(&error) {
            dnacalc_skin_ir::IntentError::DefinedNameCollision { name } => {
                assert_eq!(name, "Rate");
            }
            other => panic!("expected DefinedNameCollision, got {other:?}"),
        }
    }

    // ------------------------------------------------------------------
    // N3 acceptance: `_names` backing-cell allocation (host-core policy,
    // §B.7/F.4).
    // ------------------------------------------------------------------

    /// N3: the first allocation on a fresh workbook lazily creates the
    /// `_names` sheet and returns row 1 (append-only, column A convention).
    #[test]
    fn allocate_next_names_backing_cell_creates_sheet_lazily_at_row_one() {
        let (mut session, _sheet) = workbook_with_sheet("workbook:n3-alloc-fresh");
        assert!(
            session
                .sheets()
                .unwrap()
                .iter()
                .all(|row| row.display_name != NAMES_BACKING_SHEET),
            "the _names sheet does not exist before the first allocation"
        );

        let (names_sheet, row) = session.allocate_next_names_backing_cell().unwrap();

        assert_eq!(row, 1, "the first allocation lands on row 1");
        let sheets = session.sheets().unwrap();
        let names_row = sheets
            .iter()
            .find(|row| row.node_id == names_sheet)
            .expect("the _names sheet now exists");
        assert_eq!(names_row.display_name, NAMES_BACKING_SHEET);
        assert!(
            names_row.grid_backed,
            "the lazily-created _names sheet is grid-backed like any sheet"
        );
    }

    /// N3: a second allocation appends past the first name's target row —
    /// re-derived fresh from the catalog each call, no side ledger (B.7).
    #[test]
    fn allocate_next_names_backing_cell_appends_past_existing_static_names() {
        let (mut session, _sheet) = workbook_with_sheet("workbook:n3-alloc-append");

        let (names_sheet, first_row) = session.allocate_next_names_backing_cell().unwrap();
        assert_eq!(first_row, 1);
        session
            .set_grid_cell_value(names_sheet, first_row, 1, CalcValue::number(0.065))
            .unwrap();
        session
            .set_defined_name(
                names_sheet,
                DefinedNameScopeProjection::Workbook,
                "rate",
                DefinedNameTargetIntentInput::Static(rect(first_row, 1, first_row, 1)),
            )
            .unwrap();

        let (second_sheet, second_row) = session.allocate_next_names_backing_cell().unwrap();
        assert_eq!(
            second_sheet, names_sheet,
            "the second allocation reuses the same _names sheet, not a new one"
        );
        assert_eq!(second_row, 2, "the second allocation appends at row 2");
    }

    /// N3: re-deriving the allocation with two names already on `_names`
    /// still allocates past both — the "no side ledger" guarantee (B.7):
    /// nothing but `document_defined_names` on the `_names` sheet is
    /// consulted, so a session rehydrated from a snapshot allocates
    /// correctly on its very first call.
    #[test]
    fn allocate_next_names_backing_cell_is_re_derivable_with_no_side_ledger() {
        let (mut session, _sheet) = workbook_with_sheet("workbook:n3-alloc-re-derive");

        let (names_sheet, row_a) = session.allocate_next_names_backing_cell().unwrap();
        session
            .set_defined_name(
                names_sheet,
                DefinedNameScopeProjection::Workbook,
                "a",
                DefinedNameTargetIntentInput::Static(rect(row_a, 1, row_a, 1)),
            )
            .unwrap();
        let (_, row_b) = session.allocate_next_names_backing_cell().unwrap();
        session
            .set_defined_name(
                names_sheet,
                DefinedNameScopeProjection::Workbook,
                "b",
                DefinedNameTargetIntentInput::Static(rect(row_b, 1, row_b, 1)),
            )
            .unwrap();

        // A third call (no counter state involved beyond the catalog itself)
        // allocates row 3 — one past the highest of the two existing rows.
        let (_, row_c) = session.allocate_next_names_backing_cell().unwrap();
        assert_eq!(row_c, 3);
    }

    // ------------------------------------------------------------------
    // N3 acceptance: atomic `create_named_value` (host-core owns the whole
    // `+ name` named-value creation).
    // ------------------------------------------------------------------

    /// The `_names`-static backing row a name targets (its single cell's row).
    fn named_static_row(catalog: &DefinedNamesProjection, name: &str) -> u32 {
        match &catalog
            .entries
            .iter()
            .find(|entry| entry.name == name)
            .expect("name is in the catalog")
            .target
        {
            DefinedNameTargetProjection::Static(rect) => rect.bottom_row,
            other => panic!("expected a static target for {name}, got {other:?}"),
        }
    }

    /// N3: `create_named_value` lazily creates the `_names` sheet, writes the
    /// value into it, and defines the name workbook-wide — a formula on another
    /// sheet referencing the name resolves (the end-to-end `+ name` contract).
    #[test]
    fn create_named_value_defines_workbook_name_resolvable_from_another_sheet() {
        let (mut session, sheet1) = workbook_with_sheet("workbook:n3-create-named-value");
        assert!(
            session
                .sheets()
                .unwrap()
                .iter()
                .all(|row| row.display_name != NAMES_BACKING_SHEET),
            "the _names sheet does not exist before the first named value"
        );

        session.create_named_value("rate", "0.065").unwrap();

        // The _names sheet is created lazily, and the catalog lists `rate` at
        // Workbook scope targeting a single static cell.
        assert!(
            session
                .sheets()
                .unwrap()
                .iter()
                .any(|row| row.display_name == NAMES_BACKING_SHEET),
            "the _names sheet is created lazily on the first named value"
        );
        let catalog = session.defined_names().unwrap();
        let rate = catalog
            .entries
            .iter()
            .find(|entry| entry.name == "rate")
            .expect("rate is in the catalog");
        assert_eq!(rate.scope, DefinedNameScopeProjection::Workbook);
        assert!(!rate.is_dynamic);

        // A formula on Sheet1 referencing the workbook-wide name resolves.
        session.enter_grid_cell(sheet1, 1, 1, "=rate*2").unwrap();
        let got = session
            .grid_cell_value(sheet1, 1, 1)
            .unwrap()
            .and_then(|v| v.as_number())
            .expect("=rate*2 resolves to a number");
        assert!(
            (got - 0.13).abs() < 1e-9,
            "=rate*2 resolves to 0.065*2 = 0.13, got {got}"
        );
    }

    /// N3: a second `create_named_value` appends to the next `_names` row
    /// rather than colliding with the first — both names resolve independently.
    #[test]
    fn create_named_value_second_appends_next_row_without_collision() {
        let (mut session, sheet1) = workbook_with_sheet("workbook:n3-create-named-append");

        session.create_named_value("rate", "0.065").unwrap();
        session.create_named_value("years", "30").unwrap();

        // Both names are in the catalog at distinct backing rows.
        let catalog = session.defined_names().unwrap();
        assert_ne!(
            named_static_row(&catalog, "rate"),
            named_static_row(&catalog, "years"),
            "the second named value appends to a fresh row, not the first's"
        );

        // Both resolve from Sheet1 (proving neither write clobbered the other).
        session.enter_grid_cell(sheet1, 1, 1, "=rate").unwrap();
        session.enter_grid_cell(sheet1, 2, 1, "=years").unwrap();
        let rate = session
            .grid_cell_value(sheet1, 1, 1)
            .unwrap()
            .and_then(|v| v.as_number())
            .expect("=rate resolves to a number");
        assert!(
            (rate - 0.065).abs() < 1e-9,
            "=rate resolves to 0.065, got {rate}"
        );
        assert_eq!(
            session
                .grid_cell_value(sheet1, 2, 1)
                .unwrap()
                .and_then(|v| v.as_number()),
            Some(30.0),
            "=years resolves to 30"
        );
    }
}
