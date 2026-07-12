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
use dnacalc_bench_host::app::reducer::{
    apply_editor_command_to_active_formula_space, apply_skin_intent_to_host_state,
};
use dnacalc_bench_host::services::home_shell_view_model::build_home_shell_view_model;
use dnacalc_bench_host::services::live_edit::{
    apply_live_editor_input, flush_pending_runtime_recalc, refresh_active_formula_space,
};
use dnacalc_bench_host::state::OneCalcHostState;
use dnacalc_bench_host::ui::editor::commands::{EditorCommand, EditorInputEvent, EditorInputKind};

use dnacalc_bridge::BridgeEvent;
use dnacalc_skin_ir::formula::{
    ConditionalFormatRuleAuthoring, OneFormulaIntent, OneFormulaProjection,
};
use dnacalc_skin_ir::protocol::{
    HostCapabilityProjection, PersistenceProjection, SkinDocumentProjection, SkinIntent,
    SkinShellIntent,
};

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

    /// The host's real `HostCapabilityProjection` (bead dtc-lfz.6, G7
    /// minimal slice) — `runtime_profile` × `extension_placement` is the
    /// legality matrix the extensions overlay/Strip instrument look up
    /// (never re-derived skin-side); `home_shell_view_model` now projects
    /// `extension_placement` honestly from `dnacalc-extension-host-core`'s
    /// real per-runtime capability gate instead of a hardcoded value (see
    /// `dnacalc_bench_host::extensions::honest_extension_placement_for`).
    /// Falls back to the honest `NullTest`/`Unavailable` pair only if the
    /// host has no active formula space (never in normal operation —
    /// mirrors `projection()`'s and `persistence()`'s fallback).
    #[must_use]
    pub fn host_capabilities(&self) -> HostCapabilityProjection {
        build_home_shell_view_model(&self.state)
            .map(|view_model| view_model.skin_snapshot.host_capabilities)
            .unwrap_or_else(|| {
                HostCapabilityProjection::onecalc_null_references(
                    dnacalc_skin_ir::RuntimeProfileProjection::NullTest,
                    dnacalc_skin_ir::ExtensionPlacementProjection::Unavailable,
                )
            })
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
            // CommitRequested → mark the active formula space committed and
            // flush any pending (debounced) runtime pass so the result is
            // final. The commit mark goes through the SAME reducer path the
            // legacy `home_shell` UI drives (`EditorCommand::CommitEntry`), so
            // `FormulaSpaceState.committed_cell_text`/`proofed_cell_text` are
            // set to the just-committed `raw_entered_cell_text`. Without this,
            // `FormulaSpaceState::live_state()` could never resolve to
            // `Committed` in the Bench flow, leaving the persistence-projected
            // `dirty` bit (read by the Shell's mast dirty-dot, bead dtc-lfz.3)
            // stuck `true` for the rest of the session after the first edit.
            // `committed_text` still mirrors it for `RevertRequested`.
            BridgeEvent::CommitRequested => {
                self.committed_text = self.projection().raw_entered_cell_text;
                apply_editor_command_to_active_formula_space(
                    &mut self.state,
                    EditorCommand::CommitEntry,
                );
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

    /// Author the result hero's font colour (bead dtc-5xiw,
    /// `SetFontAttributes`). `color` is a hex `#RRGGBB`; `None` clears it
    /// back to inherit. Same two-step contract as
    /// [`Self::set_number_format`] — the effective colour comes back from
    /// OxFml's publication surface, never a skin-side decision.
    pub fn set_font_color(&mut self, color: Option<String>) -> bool {
        let formula_space_id = self.formula_space_id();
        self.author_format(OneFormulaIntent::SetFontAttributes {
            formula_space_id,
            font_color: color,
        })
    }

    /// Author the result hero's fill (background) colour
    /// (`SetFillAttributes`). `color` is a hex `#RRGGBB`; `None` clears it.
    pub fn set_fill_color(&mut self, color: Option<String>) -> bool {
        let formula_space_id = self.formula_space_id();
        self.author_format(OneFormulaIntent::SetFillAttributes {
            formula_space_id,
            fill_color: color,
        })
    }

    /// Switch the workspace locale (`SetLocale`); the result re-renders
    /// its `General`/date value through OxFml under the new profile.
    /// `language_tag` is a BCP-47 tag (`"en-US"`, `"de-DE"`, …).
    pub fn set_locale(&mut self, language_tag: String) -> bool {
        let formula_space_id = self.formula_space_id();
        self.author_format(OneFormulaIntent::SetLocale {
            formula_space_id,
            locale_language_tag: language_tag,
        })
    }

    /// Flip the active formula's date system (`SetDate1904`): `false` =
    /// 1900 (default), `true` = 1904 (Mac legacy).
    pub fn set_date1904(&mut self, date1904: bool) -> bool {
        let formula_space_id = self.formula_space_id();
        self.author_format(OneFormulaIntent::SetDate1904 {
            formula_space_id,
            date1904,
        })
    }

    /// Author a conditional-formatting rule on the result hero
    /// (`SetConditionalFormatRule`). `index` `None` appends; `Some(i)`
    /// replaces the rule at `i`. OxFml evaluates the rule on the next
    /// pass and re-renders the effective display / colours.
    pub fn set_cf_rule(
        &mut self,
        index: Option<usize>,
        rule: ConditionalFormatRuleAuthoring,
    ) -> bool {
        let formula_space_id = self.formula_space_id();
        self.author_format(OneFormulaIntent::SetConditionalFormatRule {
            formula_space_id,
            rule_index: index,
            rule,
        })
    }

    /// Remove the conditional-formatting rule at `index` from the result
    /// hero (`RemoveConditionalFormatRule`). Honest no-op out of range.
    pub fn remove_cf_rule(&mut self, index: usize) -> bool {
        let formula_space_id = self.formula_space_id();
        self.author_format(OneFormulaIntent::RemoveConditionalFormatRule {
            formula_space_id,
            rule_index: index,
        })
    }

    /// Shared authoring path for the format-write verbs: dispatch the
    /// `OneFormulaIntent` and, when it changed host state, re-run the
    /// OxFml pass so `effective_display_summary` (and any CF-applied
    /// colours) recompute. The same two-step no-skin-side-formatting
    /// contract [`Self::set_number_format`] documents; returns `true`
    /// when the value actually changed (so the caller re-projects).
    fn author_format(&mut self, intent: OneFormulaIntent) -> bool {
        let changed = apply_skin_intent_to_host_state(
            &mut self.state,
            SkinIntent::OneFormula(intent),
        );
        if changed {
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

    /// The font / fill / locale / date-system / CF authoring methods each
    /// record on the FormattingSurface (and re-render through host truth,
    /// like `set_number_format`). Fail-pre-fix: none of these adapter
    /// methods existed before bead dtc-5xiw.
    #[test]
    fn format_authoring_methods_record_on_the_surface() {
        let mut host = BenchHost::new();
        host.apply(BridgeEvent::TextEdited {
            text: "=1234.5".to_string(),
            caret: 7,
        });

        assert!(host.set_font_color(Some("#D02A23".to_string())));
        assert!(host.set_fill_color(Some("#FFE9A8".to_string())));
        assert!(host.set_date1904(true));
        {
            let p = host.projection();
            assert_eq!(p.formatting.font_color.as_deref(), Some("#D02A23"));
            assert_eq!(p.formatting.fill_color.as_deref(), Some("#FFE9A8"));
            assert!(p.formatting.date1904);
        }
        // Unchanged re-apply is an honest no-op.
        assert!(!host.set_font_color(Some("#D02A23".to_string())));

        // Locale switches the workspace tag; re-applying is a no-op.
        assert!(host.set_locale("de-DE".to_string()));
        assert!(!host.set_locale("de-DE".to_string()));
        assert!(!host.projection().formatting.locale_language_tag.is_empty());

        // CF: add a cell_value rule, confirm it lands, then remove it.
        assert!(host.set_cf_rule(
            None,
            ConditionalFormatRuleAuthoring {
                rule_kind: "cell_value".to_string(),
                operator: Some("greaterThan".to_string()),
                thresholds: vec!["100".to_string()],
                font_color: None,
                fill_color: Some("#006600".to_string()),
            },
        ));
        {
            let rules = host.projection().formatting.conditional_formatting_rules;
            assert_eq!(rules.len(), 1);
            assert_eq!(rules[0].operator.as_deref(), Some("greaterThan"));
        }
        assert!(host.remove_cf_rule(0));
        assert!(
            host.projection()
                .formatting
                .conditional_formatting_rules
                .is_empty()
        );
        // Out-of-range removal is an honest no-op.
        assert!(!host.remove_cf_rule(5));
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
    /// hardcoded constant, across the full edit → commit lifecycle: a fresh
    /// empty formula space is not dirty, an uncommitted edit makes it dirty,
    /// and a commit clears it again. That last leg exercises the fix where
    /// `BenchHost::apply(CommitRequested)` marks the active formula space's
    /// `committed_cell_text`/`proofed_cell_text` (via the same reducer path
    /// the legacy home_shell `EditorCommand::CommitEntry` drives) so
    /// `FormulaSpaceState::live_state()` resolves to `Committed`. On native,
    /// `can_save`/`can_open` are real (a real file-persistence seam exists —
    /// see the save-coverage NOTE at the foot of this module).
    #[test]
    fn persistence_tracks_real_dirty_state_across_edit_and_commit() {
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

        // Committing marks the active formula space committed (raw ==
        // committed == proofed), so `live_state()` resolves to `Committed`
        // and the persistence-projected `dirty` bit clears.
        host.apply(BridgeEvent::CommitRequested);
        assert!(
            !host.persistence().dirty,
            "a commit must clear the persistence-projected dirty bit"
        );
    }

    /// bead dtc-lfz.6 (G7 minimal slice), desktop-runtime coverage: a real
    /// `BenchHost` compiled and run natively (this test's own compile
    /// target) reports its `HostCapabilityProjection.extension_placement`
    /// as `InProcess` — genuine in-process catalog data, not the prior
    /// hardcoded `Unavailable` that lied about desktop's real capability.
    /// `native_unix`/`windows_desktop` both resolve to `InProcess` per
    /// `dnacalc_extension_host_core::RuntimeProfile::capabilities()`, so
    /// this assertion holds regardless of which native OS runs the suite —
    /// what it proves is "native compile is NOT the browser-unavailable
    /// case", which only the fix makes true (pre-fix this asserted
    /// `Unavailable` unconditionally).
    #[test]
    fn desktop_runtime_host_capabilities_report_real_in_process_placement() {
        use dnacalc_skin_ir::protocol::ExtensionPlacementProjection;

        let host = BenchHost::new();
        let capabilities = host.host_capabilities();
        assert_eq!(
            capabilities.extension_placement,
            ExtensionPlacementProjection::InProcess,
            "a native test run must report real in-process catalog placement, not Unavailable"
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
