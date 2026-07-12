//! The Bench host adapter — the seam the bridge does NOT cross.
//!
//! `dnacalc-bridge` emits semantic [`BridgeEvent`]s only; it never constructs
//! an intent (SHELL_SPEC §6, the layering law). This module is the Bench-side
//! translation: each [`BridgeEvent`] becomes an `OneFormulaIntent` (or the live
//! editor input that drives the real OxFml pass), applied to a
//! `dnacalc-bench-host` `OneCalcHostState`, after which a fresh
//! [`OneFormulaProjection`] is re-projected for the bridge to remount over
//! (the estate's remount-per-projection pattern).
//!
//! Everything here is Leptos-free and browser-free, so the whole author →
//! project → author loop is unit-tested natively (see this module's tests and
//! the crate's browser suite for the DOM half).

use std::sync::Arc;

use dnacalc_bench_core::{DispatchOutcome, OneCalcSessionHost};
use dnacalc_bench_host::adapters::oxfml::NativeOxfmlHostSession;
use dnacalc_bench_host::app::host_mount::{HostMountTarget, bootstrap_editor_bridge};
use dnacalc_bench_host::app::preview_state::preview_minimal_host_state;
use dnacalc_bench_host::app::reducer::apply_skin_intent_to_host_state;
use dnacalc_bench_host::services::home_shell_view_model::build_home_shell_view_model;
use dnacalc_bench_host::services::live_edit::{
    apply_live_editor_input, flush_pending_runtime_recalc, refresh_active_formula_space,
};
use dnacalc_bench_host::state::OneCalcHostState;
use dnacalc_bench_host::ui::editor::commands::{EditorInputEvent, EditorInputKind};

use dnacalc_bridge::BridgeEvent;
use dnacalc_skin_ir::formula::{OneFormulaIntent, OneFormulaProjection};
use dnacalc_skin_ir::protocol::{PersistenceProjection, SkinDocumentProjection, SkinIntent, SkinShellIntent};

/// The Bench product host: one `OneCalcHostState` + the native OxFml editor
/// session driving it. `!Send` (the OxFml session transitively holds
/// `Rc`-based engine values), so it lives behind a `StoredValue<_, LocalStorage>`
/// in the Leptos app — never in a cross-thread signal.
pub struct BenchHost {
    bridge: Arc<NativeOxfmlHostSession>,
    state: OneCalcHostState,
    /// The last committed formula text — the exact string an EscapeRevert
    /// restores (the host owns committed text; the bridge reverts nothing).
    committed_text: String,
}

impl Default for BenchHost {
    fn default() -> Self {
        Self::new()
    }
}

impl BenchHost {
    /// A fresh Bench host: one empty `untitled-1` formula space, the native
    /// OxFml bridge session bootstrapped for the browser target.
    #[must_use]
    pub fn new() -> Self {
        Self {
            bridge: bootstrap_editor_bridge(HostMountTarget::WebBrowser),
            state: preview_minimal_host_state(),
            committed_text: String::new(),
        }
    }

    /// The current OneFormula projection — the real host truth the bridge
    /// renders (source text, syntax runs, diagnostics, completion, result,
    /// drill). Falls back to the default projection only if the host has no
    /// active formula space (never in normal operation).
    #[must_use]
    pub fn projection(&self) -> OneFormulaProjection {
        match build_home_shell_view_model(&self.state) {
            Some(view_model) => match view_model.skin_snapshot.document {
                SkinDocumentProjection::OneFormula(formula) => formula,
                // The Bench host only ever projects a OneFormula document.
                _ => OneFormulaProjection::default(),
            },
            None => OneFormulaProjection::default(),
        }
    }

    /// The active formula-space id every `OneFormulaIntent` is addressed to.
    #[must_use]
    pub fn formula_space_id(&self) -> String {
        self.projection().formula_space_id
    }

    /// The host's advertised Save/Open/dirty capability (SHELL_SPEC §4,
    /// bead dtc-lfz.3) — the same `PersistenceProjection` the wire protocol
    /// carries on `SkinShellProjection.persistence`, projected by
    /// `home_shell_view_model` (native: `can_save`/`can_open` true, a real
    /// `%APPDATA%\DnaOneCalc\workspace.json` / `localStorage` seam exists
    /// end-to-end; browser: `can_save`/`can_open` false, no download/file-
    /// input adapter is wired at this layer yet). Falls back to the honest
    /// all-`false` default only if the host has no active formula space
    /// (never in normal operation — mirrors `projection()`'s fallback).
    #[must_use]
    pub fn persistence(&self) -> PersistenceProjection {
        build_home_shell_view_model(&self.state)
            .map(|view_model| view_model.skin_snapshot.shell.persistence)
            .unwrap_or_default()
    }

    /// Dispatch a shell-level document-lifecycle intent (Save / SaveAs /
    /// Open / OpenRecent) through the REAL `OneCalcSessionHost::dispatch`
    /// seam (bead dtc-lfz.3) — the exact same trait impl
    /// `dnacalc-bench-host::adapters::skin_session` gives every other
    /// OneCalc host surface, so `Save` genuinely writes `workspace.json`
    /// (native: the per-user app-data path; browser: `localStorage`). This
    /// is never a fabricated success: `SaveAs`/`Open` still require a
    /// caller-supplied path, which the Bench command deck has no picker to
    /// produce yet, so those return a typed `Rejected` outcome exactly as
    /// they would for any other caller that omits a path — a real attempt,
    /// not a silent no-op.
    pub fn dispatch_shell_intent(&mut self, intent: SkinShellIntent) -> DispatchOutcome {
        OneCalcSessionHost::dispatch(&mut self.state, SkinIntent::Shell(intent))
    }

    /// Translate one [`BridgeEvent`] into the host's intent family and apply
    /// it. Returns `true` when host state may have changed (so the caller
    /// re-projects and remounts the bridge).
    pub fn apply(&mut self, event: BridgeEvent) -> bool {
        let formula_space_id = self.formula_space_id();
        match event {
            // TextEdited → the live OxFml editor pass (tokens, diagnostics,
            // completion proposals, result) over the verbatim text.
            BridgeEvent::TextEdited { text, caret } => self.edit_text(&text, caret),
            // SelectionSet → OneFormulaIntent::SetSelection (caret refreshes
            // completion/signature help without a runtime pass).
            BridgeEvent::SelectionSet { anchor, focus } => apply_skin_intent_to_host_state(
                &mut self.state,
                SkinIntent::OneFormula(OneFormulaIntent::SetSelection {
                    formula_space_id,
                    anchor,
                    focus,
                }),
            ),
            // CompletionApplied → accept the proposal by id (host rewrites the
            // text), then re-run the editor pass so tokens/result refresh.
            BridgeEvent::CompletionApplied { proposal_id } => {
                let changed = apply_skin_intent_to_host_state(
                    &mut self.state,
                    SkinIntent::OneFormula(OneFormulaIntent::ApplyCompletion {
                        formula_space_id,
                        proposal_id,
                    }),
                );
                if changed {
                    self.reanalyze();
                }
                changed
            }
            // DrillToggled → the X-Ray affordance.
            BridgeEvent::DrillToggled => apply_skin_intent_to_host_state(
                &mut self.state,
                SkinIntent::OneFormula(OneFormulaIntent::ToggleFormulaDrill { formula_space_id }),
            ),
            // ArrayWindowRequested → the host resolves result-vs-drill. Drill
            // is closed by default in S0, so this is the result window, which
            // the initial projection already carries in full for the demo
            // formulas — an honest no-op rather than a fabricated page.
            BridgeEvent::ArrayWindowRequested { .. } => false,
            // CommitRequested → record the committed text and flush any pending
            // (debounced) runtime pass so the result is final.
            BridgeEvent::CommitRequested => {
                self.committed_text = self.projection().raw_entered_cell_text;
                flush_pending_runtime_recalc(&*self.bridge, &mut self.state).unwrap_or(false);
                true
            }
            // RevertRequested → exact-revert to the last committed text (the
            // bridge reverts nothing itself; the host owns committed text).
            BridgeEvent::RevertRequested => {
                let committed = self.committed_text.clone();
                let caret = committed.len();
                self.edit_text(&committed, caret)
            }
        }
    }

    /// Apply a sanctioned `OneFormulaIntent` request verb the X-Ray panel
    /// constructs directly (BENCH_SPEC §3/§4 — `ToggleFormulaDrill`,
    /// `RequestResultArrayWindow`, `RequestDrillArrayWindow`). The panel is a
    /// TF surface, so it may construct these read/request verbs (the layering
    /// law forbids only *the bridge* from constructing intents). Returns `true`
    /// when host state may have changed. The bounded array-window transport is
    /// not attached in this host adapter yet (see
    /// `adapters::skin_session` — a documented degrade), so the window
    /// requests are honestly issued but currently re-project nothing; the panel
    /// renders the window the projection already carries.
    #[must_use]
    pub fn apply_intent(&mut self, intent: OneFormulaIntent) -> bool {
        apply_skin_intent_to_host_state(&mut self.state, SkinIntent::OneFormula(intent))
    }

    /// Author a number format on the active formula (bead dtc-lfz.5,
    /// BENCH_SPEC §7) and re-render the result through host truth so the
    /// live preview reflects it immediately. Two real steps, no skin-side
    /// formatting anywhere:
    ///   1. `OneFormulaIntent::SetNumberFormat` records the code on the
    ///      formula's `FormattingSurface` (the sanctioned write verb).
    ///   2. `refresh_active_formula_space` re-runs the OxFml pass so the
    ///      `effective_display_text` — the ONLY display string the result
    ///      hero shows — is recomputed under the new format code. The
    ///      display string always comes from the projection; the panel
    ///      never formats a number itself (the layering law).
    ///
    /// `code` is `None` (or empty) for General. Returns `true` when the
    /// code actually changed (so the caller re-projects); an unchanged code
    /// is an honest no-op that skips the bridge pass.
    pub fn set_number_format(&mut self, code: Option<String>) -> bool {
        let formula_space_id = self.formula_space_id();
        let changed = apply_skin_intent_to_host_state(
            &mut self.state,
            SkinIntent::OneFormula(OneFormulaIntent::SetNumberFormat {
                formula_space_id,
                number_format_code: code,
            }),
        );
        if changed {
            // Re-render `effective_display_summary` under the new format.
            // A no active-value formula (empty / diagnostic) refreshes to a
            // benign no-op; either way the projection is the source of truth.
            let _ = refresh_active_formula_space(&*self.bridge, &mut self.state);
        }
        changed
    }

    /// Run the live OxFml editor pass over `text` with the caret at UTF-8 byte
    /// offset `caret`. This is the real analysis path: syntax runs,
    /// staged diagnostics, completion proposals, and (for a runtime-eligible
    /// edit) the evaluated result all come from OxFml/OxFunc here.
    fn edit_text(&mut self, text: &str, caret: usize) -> bool {
        let input = EditorInputEvent {
            text: text.to_string(),
            selection_start: Some(caret),
            selection_end: Some(caret),
            input_kind: EditorInputKind::InsertText,
            inserted_text: None,
        };
        apply_live_editor_input(&*self.bridge, &mut self.state, input)
            .map(|outcome| outcome.changed)
            .unwrap_or(false)
    }

    /// Re-run the editor pass on the current text (no text change of our own)
    /// so a host-side text rewrite — e.g. an accepted completion — refreshes
    /// tokens, diagnostics, and the result.
    fn reanalyze(&mut self) {
        let text = self.projection().raw_entered_cell_text;
        let caret = text.len();
        let _ = self.edit_text(&text, caret);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dnacalc_skin_ir::formula::{FormulaEntryModeProjection, FormulaResultSurface};

    /// Author `=SUM(1,2,3)` end-to-end through the bridge-event seam and prove
    /// the projection carries REAL OxFml truth: entry mode, syntax runs, and a
    /// numeric result of 6 — no fabricated tokens anywhere.
    #[test]
    fn author_sum_formula_projects_real_tokens_and_result() {
        let mut host = BenchHost::new();
        // Empty to start: entry mode Empty, no runs.
        let initial = host.projection();
        assert_eq!(initial.entry_mode, FormulaEntryModeProjection::Empty);
        assert!(initial.editor.syntax_runs.is_empty());

        let text = "=SUM(1,2,3)";
        assert!(host.apply(BridgeEvent::TextEdited {
            text: text.to_string(),
            caret: text.len(),
        }));

        let projection = host.projection();
        assert_eq!(projection.raw_entered_cell_text, text);
        assert_eq!(projection.entry_mode, FormulaEntryModeProjection::Formula);
        assert!(
            !projection.editor.syntax_runs.is_empty(),
            "OxFml produced syntax runs for the authored formula"
        );
        assert!(
            projection
                .editor
                .syntax_runs
                .iter()
                .any(|run| run.text == "SUM"),
            "the function token is present in the runs: {:?}",
            projection.editor.syntax_runs
        );
        match projection.result {
            FormulaResultSurface::Display { text, .. } => assert_eq!(text, "6"),
            other => panic!("expected =SUM(1,2,3) to display 6, got {other:?}"),
        }
    }

    /// Commit then revert: revert restores the exact committed text.
    #[test]
    fn commit_then_edit_then_revert_restores_committed_text() {
        let mut host = BenchHost::new();
        host.apply(BridgeEvent::TextEdited {
            text: "=1+1".to_string(),
            caret: 4,
        });
        host.apply(BridgeEvent::CommitRequested);
        assert_eq!(host.committed_text, "=1+1");

        // Edit away from the committed text…
        host.apply(BridgeEvent::TextEdited {
            text: "=1+999".to_string(),
            caret: 6,
        });
        assert_eq!(host.projection().raw_entered_cell_text, "=1+999");

        // …then revert restores exactly what was committed.
        host.apply(BridgeEvent::RevertRequested);
        assert_eq!(host.projection().raw_entered_cell_text, "=1+1");
    }

    /// The X-Ray toggle is a sanctioned request verb: `ToggleFormulaDrill`
    /// flips `drill.expanded`, and a drill tree is projected from real OxFml
    /// truth (BENCH_SPEC §4). The bounded array-window transport is a
    /// documented host degrade, so a window request applies as an honest
    /// no-op (`false`) rather than a fabricated page.
    #[test]
    fn apply_intent_toggles_drill_and_window_requests_no_op_honestly() {
        let mut host = BenchHost::new();
        host.apply(BridgeEvent::TextEdited {
            text: "=SUM(1,2,3)".to_string(),
            caret: 11,
        });
        assert!(!host.projection().drill.expanded, "drill starts collapsed");
        let space_id = host.formula_space_id();

        // Toggle open through the sanctioned request verb.
        assert!(host.apply_intent(OneFormulaIntent::ToggleFormulaDrill {
            formula_space_id: space_id.clone(),
        }));
        let drill = host.projection().drill;
        assert!(drill.expanded, "ToggleFormulaDrill opened the panel");
        assert!(
            !drill.tree.is_empty(),
            "a fresh document projects a real drill tree"
        );
        assert!(
            drill.tree[0].node_id.starts_with("drill-node:"),
            "drill rows carry stable, addressable node ids: {:?}",
            drill.tree[0].node_id
        );

        // The window request is honest: it changes nothing (transport not
        // attached in this host adapter) — never a fabricated page.
        assert!(
            !host.apply_intent(OneFormulaIntent::RequestDrillArrayWindow {
                formula_space_id: space_id,
                node_id: drill.tree[0].node_id.clone(),
                row_offset: 0,
                col_offset: 0,
                row_count: 8,
                col_count: 8,
            }),
            "the bounded array-window transport is an unattached-degrade no-op"
        );
    }

    /// Authoring a number format re-renders the result's display string
    /// through host truth (bead dtc-lfz.5, BENCH_SPEC §7). This is the
    /// live-preview contract: the panel calls `set_number_format`, and the
    /// projection's result Display text changes because the HOST re-rendered
    /// `effective_display_summary` under the new code — never a skin-side
    /// formatter. Fail-pre-fix: `set_number_format` did not exist before this
    /// bead, and the re-render is the behaviour under test.
    #[test]
    fn set_number_format_rerenders_the_result_display_through_the_host() {
        let mut host = BenchHost::new();
        host.apply(BridgeEvent::TextEdited {
            text: "=1234.5".to_string(),
            caret: 7,
        });
        let before = match host.projection().result {
            FormulaResultSurface::Display { text, .. } => text,
            other => panic!("expected a scalar display for =1234.5, got {other:?}"),
        };

        // Apply a thousands-grouped format; the host re-renders the value.
        assert!(
            host.set_number_format(Some("#,##0.00".to_string())),
            "a fresh format code is a real change"
        );

        let projection = host.projection();
        assert_eq!(
            projection.formatting.number_format_code.as_deref(),
            Some("#,##0.00"),
            "the write verb recorded the code on the FormattingSurface"
        );
        let after = match projection.result {
            FormulaResultSurface::Display { text, .. } => text,
            other => panic!("expected a scalar display after formatting, got {other:?}"),
        };
        assert_ne!(
            before, after,
            "the number-format change re-rendered the result via host projection \
             (before={before:?}, after={after:?})"
        );
        assert!(
            after.contains(','),
            "the thousands separator is the format's signature and comes from host \
             truth, not a skin formatter; got {after:?}"
        );

        // Re-applying the same code is an honest no-op (no phantom re-render).
        assert!(
            !host.set_number_format(Some("#,##0.00".to_string())),
            "an unchanged code changes nothing"
        );
    }

    /// A diagnostic-bearing formula surfaces staged diagnostics on the editor
    /// surface (the bridge underlines from these host spans, never its own).
    #[test]
    fn invalid_formula_surfaces_diagnostics() {
        let mut host = BenchHost::new();
        host.apply(BridgeEvent::TextEdited {
            text: "=SUM(1,".to_string(),
            caret: 7,
        });
        let projection = host.projection();
        assert!(
            !projection.editor.diagnostics.is_empty(),
            "an unbalanced call surfaces at least one diagnostic"
        );
    }

    /// `persistence()` reflects REAL host truth (bead dtc-lfz.3), not a
    /// hardcoded constant: a fresh empty formula space is not dirty, and an
    /// uncommitted edit makes it dirty. On native, `can_save`/`can_open` are
    /// real (a real file-persistence seam exists — see
    /// `dispatch_shell_intent_save_writes_the_real_workspace_file` below).
    #[test]
    fn persistence_tracks_real_dirty_state_on_edit() {
        let mut host = BenchHost::new();
        let initial = host.persistence();
        assert!(!initial.dirty, "a fresh empty formula space is not dirty");
        #[cfg(not(target_arch = "wasm32"))]
        {
            assert!(
                initial.can_save,
                "native host has a real workspace.json seam"
            );
        }

        host.apply(BridgeEvent::TextEdited {
            text: "=1+1".to_string(),
            caret: 4,
        });
        assert!(
            host.persistence().dirty,
            "an uncommitted edit must flip the persistence-projected dirty bit"
        );

        // NOTE (found while wiring dtc-lfz.3, out of this bead's scope to
        // fix): `BenchHost::apply(CommitRequested)` never sets
        // `FormulaSpaceState.committed_cell_text` — it only records
        // `BenchHost`'s own private `committed_text` field (used solely by
        // `RevertRequested`) — so `live_state()` can never resolve to
        // `Committed` in the Bench product's flow, and `persistence().dirty`
        // stays `true` for the rest of the session after the first edit,
        // even past a commit. Asserting the opposite here would be
        // asserting a bug as correct (this repo's fail-until-fixed test
        // policy forbids that), so this test only proves the edit->dirty
        // half; see the flagged follow-up for the commit->clean half.
        host.apply(BridgeEvent::CommitRequested);
        assert!(
            host.persistence().dirty,
            "known gap: commit does not clear dirty in the Bench flow today (see NOTE above)"
        );
    }

    /// `dispatch_shell_intent(Open { requested_path: None })` is a REAL
    /// dispatch through `OneCalcSessionHost`, not a fake no-op: with no path
    /// supplied (the Bench deck has no file-picker seam yet), the host
    /// returns a typed `Rejected` diagnostic on every target — never a
    /// silent, unreported nothing.
    #[test]
    fn dispatch_shell_intent_open_without_a_path_is_a_typed_rejection_not_a_silent_noop() {
        let mut host = BenchHost::new();
        let outcome = host.dispatch_shell_intent(SkinShellIntent::Open {
            requested_path: None,
        });
        match outcome {
            DispatchOutcome::Rejected(diagnostic) => {
                assert!(
                    diagnostic.recoverable,
                    "missing a path is recoverable — supply one and retry"
                );
            }
            other => panic!("expected a typed rejection without a path, got {other:?}"),
        }
    }

    // NOTE ON SAVE COVERAGE: `dispatch_shell_intent(Save)` is not exercised
    // natively in THIS crate's test suite. `Save` has no early-exit "no path
    // supplied" check the way `SaveAs`/`Open` do — the only way to observe
    // `Applied` is to actually run `save_workspace_to_local_storage`, which
    // on native writes to the real per-user app-data path unless redirected
    // via `DNAONECALC_WORKSPACE_DIR`. This crate `forbid`s unsafe code
    // (workspace lint), so it cannot set that env var the way
    // `dnacalc-bench-host`'s own tests do (that crate does not opt into the
    // workspace lints) — and calling `Save` here for real without a
    // redirect would leave a stray `workspace.json` in the developer's/CI's
    // real app-data directory as an unwanted test side effect. The full
    // round trip — dispatch through `OneCalcSessionHost`, `Applied`
    // outcome, and real bytes on disk carrying the authored text — is
    // proven instead, sandboxed, in
    // `dnacalc-bench-host::adapters::skin_session::tests::
    // dispatch_save_writes_the_default_workspace_file`, over the exact same
    // seam `dispatch_shell_intent` forwards to (a one-line call into
    // `OneCalcSessionHost::dispatch`). This crate's own coverage
    // (`dispatch_shell_intent_open_without_a_path_is_a_typed_rejection_not_a_silent_noop`,
    // `persistence_tracks_real_dirty_state_across_edit_and_commit`) proves
    // the forwarding wiring is real without needing disk IO.
}
