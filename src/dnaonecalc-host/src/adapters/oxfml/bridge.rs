use super::types::{CalcValue, EditorAnalysisStage, EditorDocument};

#[derive(Debug, Clone, PartialEq)]
pub struct FormulaEditRequest {
    pub formula_stable_id: String,
    pub entered_text: String,
    pub cursor_offset: usize,
    pub previous_green_tree_key: Option<String>,
    pub analysis_stage: EditorAnalysisStage,
    /// Optional VerificationPublicationContext driving the runtime
    /// pass's effective_display_text computation. The host populates
    /// this from the active formula's FormulaFormattingState — the
    /// number format code, font / fill colours, and CF rules.
    /// `None` skips the formatted-display lane.
    pub formatting_request: Option<FormulaFormattingRequest>,
    /// Calc-options scenario policy. Drives whether the bridge
    /// supplies fixed clock/provider inputs (Deterministic) or
    /// fresh clock/provider inputs per request (LiveRecalc).
    pub scenario_policy: ScenarioPolicyRequest,
    /// When `true`, the bridge runs parse / bind / completion /
    /// signature-help / function-help (the "interactive" lane) and
    /// **skips** the runtime-evaluation pass that produces values,
    /// walks, summaries, and effective display text. Used for
    /// caret-only navigation events (mouse click, arrow keys) where
    /// the user expects popups to refresh against the new caret
    /// position but the formula's value cannot have changed.
    /// Defaults to `false` — every text-input keystroke and every
    /// formatting-panel knob still triggers a full runtime pass.
    /// The interactive lane reuses the cached green-tree when
    /// `previous_green_tree_key` matches, so the cost of a
    /// caret-sync round-trip is bounded by `interact_at_cursor`.
    pub skip_runtime_evaluation: bool,
    /// Calc-options recalc policy. Drives whether the runtime
    /// evaluation runs at all on text-input events
    /// (`Auto` — current behaviour) or only when the user
    /// explicitly asks for it via Calculate / F9
    /// (`Manual` — quiet typing of large formulas). Independent of
    /// `skip_runtime_evaluation`, which is per-event; this is a
    /// per-formula opt-in.
    pub recalc_mode: RecalcModeRequest,
    /// Workspace locale as a BCP-47 language tag (e.g. `"en-US"`,
    /// `"de-DE"`). The bridge resolves this through
    /// `LocaleProfileId::from_bcp47_language_tag` to a profile and
    /// builds the runtime `LocaleFormatContext` accordingly. An
    /// empty string or unrecognised tag falls back to en-US so the
    /// runtime always has a usable locale. Drives month / weekday
    /// names, decimal / thousands separators, currency symbol, and
    /// the `General` rendering for the active formula.
    pub language_tag: String,
    /// Bounded OneCalc formula-input bindings. These are single-formula
    /// context facts, not workbook defined-name semantics.
    pub formal_input_bindings: Vec<FormulaInputBindingRequest>,
    /// Trace-mode request that drives whether the runtime emits
    /// per-prepared-call detail (rich walk tree, evaluation
    /// timeline, argument summaries) or only the final value. The
    /// rich mode is the input the formula-drill panel needs; the
    /// value-only mode is the cheap default OxFml landed with W075
    /// / OxFunc W096 — it skips per-step bookkeeping the user
    /// doesn't see when the drill panel is closed. The host flips
    /// this on per-formula based on `formula_drill_open`, so
    /// keystrokes on a formula whose drill is closed pay the
    /// cheap-trace cost and opening the drill gets the rich tree
    /// on the next round-trip.
    pub trace_mode: TraceModeRequest,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FormulaInputBindingRequest {
    pub label: String,
    pub reference_descriptor: String,
    pub reference_handle: Option<String>,
    pub value: CalcValue,
}

/// Mirror of `oxfml_core::eval::EvaluationTraceMode` at the
/// bridge boundary. `ValueOnly` is the cheap default the runtime
/// shipped in W075 — no per-call trace bookkeeping; the drill
/// panel falls back to a single `Formula` node. `PreparedCalls`
/// emits the per-step trace the rich walk-tree projection needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TraceModeRequest {
    /// Default: no per-call trace. Cheapest runtime path.
    #[default]
    ValueOnly,
    /// Rich per-call trace. Required for the formula-drill walk
    /// tree to surface the prepared-call breakdown.
    PreparedCalls,
}

/// Mirror of `crate::persistence::ScenarioRecalcMode` at the bridge
/// boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RecalcModeRequest {
    /// Default: every event that changes the formula text triggers
    /// a runtime pass. Caret-only events still skip runtime via
    /// `skip_runtime_evaluation`.
    #[default]
    Auto,
    /// Runtime evaluation is gated on an explicit Calculate / F9
    /// request. Text edits skip the runtime pass entirely; the
    /// editor still gets parse / bind / popups every event.
    Manual,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FormulaFormattingRequest {
    pub number_format_code: Option<String>,
    pub font_color: Option<String>,
    pub fill_color: Option<String>,
    pub style_id: Option<String>,
    pub date1904: bool,
    /// Conditional-formatting rules to evaluate on the result hero.
    /// Rendered through `VerificationPublicationContext.conditional_formatting_rules`
    /// in the runtime pass; the matching rule's
    /// `effective_display_text` (when set) replaces the base
    /// formatted display.
    pub conditional_formatting_rules: Vec<FormulaFormattingCfRule>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FormulaFormattingCfRule {
    pub rule_kind: String,
    pub operator: Option<String>,
    pub thresholds: Vec<String>,
    pub font_color: Option<String>,
    pub fill_color: Option<String>,
    /// Optional typed CF rule payload, mirroring OxFml W073's
    /// `ConditionalFormattingTypedRule`. When set, the bridge attaches
    /// the upstream `typed_rule` field on the resulting
    /// `VerificationConditionalFormattingRule`. The bounded-string
    /// `thresholds` keeps riding along as the W072 fallback so older
    /// callers continue to work.
    pub typed_rule: Option<FormulaFormattingCfTypedRule>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FormulaFormattingCfTypedRule {
    pub color_scale: Option<FormulaFormattingCfColorScaleRuleOptions>,
    pub data_bar: Option<FormulaFormattingCfDataBarRuleOptions>,
    pub icon_set: Option<FormulaFormattingCfIconSetRuleOptions>,
    pub rank: Option<FormulaFormattingCfRankRuleOptions>,
    pub average: Option<FormulaFormattingCfAverageRuleOptions>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FormulaFormattingCfColorScaleRuleOptions {
    pub stops: Vec<FormulaFormattingCfColorScaleStop>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaFormattingCfColorScaleStop {
    pub position: FormulaFormattingCfThreshold,
    pub color: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FormulaFormattingCfDataBarRuleOptions {
    pub minimum: Option<FormulaFormattingCfThreshold>,
    pub maximum: Option<FormulaFormattingCfThreshold>,
    pub bar_color: Option<String>,
    pub direction: Option<FormulaFormattingCfDataBarDirection>,
    pub show_bar_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormulaFormattingCfDataBarDirection {
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaFormattingCfIconSetRuleOptions {
    pub set_kind: String,
    pub thresholds: Vec<FormulaFormattingCfThreshold>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaFormattingCfRankRuleOptions {
    pub rank: FormulaFormattingCfRank,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FormulaFormattingCfRank {
    Count(usize),
    Percent(f64),
}

impl Eq for FormulaFormattingCfRank {}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct FormulaFormattingCfAverageRuleOptions {
    pub include_equal: bool,
    pub stddev_multiplier: Option<f64>,
}

impl Eq for FormulaFormattingCfAverageRuleOptions {}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum FormulaFormattingCfThreshold {
    #[default]
    Min,
    Mid,
    Max,
    Percent(f64),
    Percentile(f64),
    Number(f64),
}

impl Eq for FormulaFormattingCfThreshold {}

/// Mirror of `crate::persistence::ScenarioPolicy` at the bridge
/// boundary. Kept as its own type so the bridge module does not
/// take a hard dependency on `crate::persistence`. Drives volatile-
/// function seed handling. The `ManualRecalc` variant maps to
/// `RecalcModeRequest::Manual` at the bridge call site; the seed
/// behaviour for ManualRecalc matches `LiveRecalc` (fresh seeds
/// when the user explicitly asks for a recalc).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScenarioPolicyRequest {
    Deterministic,
    /// Default: matches `ScenarioPolicy::LiveRecalc`.
    #[default]
    LiveRecalc,
    /// Runtime gated on Calculate; matches
    /// `ScenarioPolicy::ManualRecalc`.
    ManualRecalc,
}

// `Eq` cannot be derived: `EditorDocument.value_presentation.published_value`
// is the upstream `CalcValue`, which contains `f64` (not `Eq`).
#[derive(Debug, Clone, PartialEq)]
pub struct FormulaEditResult {
    pub document: EditorDocument,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OxfmlHostSessionError {
    UpstreamFailure(String),
}

pub trait OxfmlHostSession {
    fn apply_formula_edit(
        &self,
        request: FormulaEditRequest,
    ) -> Result<FormulaEditResult, OxfmlHostSessionError>;
}
