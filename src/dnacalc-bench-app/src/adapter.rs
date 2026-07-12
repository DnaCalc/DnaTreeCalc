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

use dnacalc_bench_host::adapters::oxfml::NativeOxfmlHostSession;
use dnacalc_bench_host::app::host_mount::{HostMountTarget, bootstrap_editor_bridge};
use dnacalc_bench_host::app::preview_state::preview_minimal_host_state;
use dnacalc_bench_host::app::reducer::apply_skin_intent_to_host_state;
use dnacalc_bench_host::services::home_shell_view_model::build_home_shell_view_model;
use dnacalc_bench_host::services::live_edit::{apply_live_editor_input, flush_pending_runtime_recalc};
use dnacalc_bench_host::state::OneCalcHostState;
use dnacalc_bench_host::ui::editor::commands::{EditorInputEvent, EditorInputKind};

use dnacalc_bridge::BridgeEvent;
use dnacalc_skin_ir::formula::{OneFormulaIntent, OneFormulaProjection};
use dnacalc_skin_ir::protocol::{SkinDocumentProjection, SkinIntent};

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
}
