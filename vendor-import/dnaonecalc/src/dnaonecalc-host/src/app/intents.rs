use crate::adapters::oxfml::{
    EditorAnalysisStage, FormulaFormattingRequest, RecalcModeRequest, ScenarioPolicyRequest,
    TraceModeRequest,
};
use crate::domain::ids::FormulaSpaceId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppIntent {
    ApplyFormulaEdit(ApplyFormulaEditIntent),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyFormulaEditIntent {
    pub formula_space_id: FormulaSpaceId,
    pub formula_stable_id: String,
    pub entered_text: String,
    pub cursor_offset: usize,
    pub analysis_stage: EditorAnalysisStage,
    /// Live formatting state for the active formula. The host
    /// derives this from `FormulaSpaceState.formatting` and forwards
    /// it through the bridge so the runtime pass populates the
    /// `verification_publication_surface.effective_display_text`
    /// the result hero renders. `None` skips the formatted-display
    /// lane (the hero falls back to the raw value).
    pub formatting_request: Option<FormulaFormattingRequest>,
    /// Calc-options scenario policy lifted from the active
    /// formula's `FormulaFormattingState.scenario_policy`. Drives
    /// clock and random-provider selection in the bridge.
    pub scenario_policy: ScenarioPolicyRequest,
    /// When `true`, skip the runtime-evaluation pass and run only
    /// parse / bind / popup / signature-help / function-help. The
    /// resulting `EditorDocument` carries the prior runtime fields
    /// from cache (or `None` for fresh formulas) — popups + caret
    /// state refresh, value / walk / display do not change. Set on
    /// caret-only navigation (mouse click, arrow keys, Home / End,
    /// PageUp / PageDown).
    pub skip_runtime_evaluation: bool,
    /// Per-formula recalc mode. `Auto` runs the runtime on every
    /// text-input event (subject to `skip_runtime_evaluation`);
    /// `Manual` gates the runtime on an explicit Calculate / F9
    /// request, decoupling typing latency from formula complexity.
    pub recalc_mode: RecalcModeRequest,
    /// Workspace locale as a BCP-47 language tag (e.g. `"en-US"`,
    /// `"de-DE"`). The host lifts this from
    /// `OneCalcHostState.ambient_app_context.language_tag` and
    /// forwards it through the bridge so the runtime pass binds the
    /// matching `LocaleProfileId` for month / weekday / separator /
    /// currency rendering. An empty string falls back to en-US.
    pub language_tag: String,
    /// Trace-mode request — `ValueOnly` for the cheap default, or
    /// `PreparedCalls` when the formula-drill panel is open and the
    /// rich walk tree needs the per-step trace. The host flips this
    /// on per-formula based on `formula_drill_open`.
    pub trace_mode: TraceModeRequest,
}
