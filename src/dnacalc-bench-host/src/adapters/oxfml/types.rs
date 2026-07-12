//! Bridge type surface for the host.
//!
//! Path B refactor (WS-14 dno-xcq.12): the host previously hand-rolled
//! mirror copies of upstream `oxfml_core` / `oxfunc_core` types and
//! translated upstream → mirror inside `live_bridge.rs`. That boundary
//! repeatedly dropped fields (severity, stage, value-kind, span
//! attribution) and forced follow-up beads each time. We now re-export
//! upstream types directly. The host adds only a small set of bundle /
//! projection types that combine multiple upstream pieces into shapes
//! tailored to the UI surface.

// ---------------------------------------------------------------------------
// Upstream re-exports — single source of truth.
// ---------------------------------------------------------------------------

pub use oxfml_core::consumer::editor::{
    CompletionProposal, CompletionProposalKind, EditorAnalysisStage, EditorSyntaxSnapshot,
    EditorToken, FormulaEditReuseSummary, FormulaTextChangeRange, FunctionHelpPacket,
    FunctionHelpSignatureForm, LiveDiagnostic, LiveDiagnosticSeverity, LiveDiagnosticSnapshot,
    LiveDiagnosticStage, SignatureHelpContext,
};
pub use oxfml_core::publication::{
    ArrayCellFormat, ArrayCellFormatGrid, CfIcon, DataBarDirection, DataBarFill,
};
pub use oxfml_core::syntax::token::TextSpan as FormulaTextSpan;
pub use oxfunc_core::value::{CalcValue, CoreValue, NumberFormatHint, WorksheetErrorCode};

// ---------------------------------------------------------------------------
// Host-side bundle types.
// ---------------------------------------------------------------------------

/// Discriminator-only view of a `CalcValue`. Convenience for matching
/// when the payload isn't needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FormulaValueKind {
    Number,
    Text,
    Logical,
    Error,
    Empty,
    Missing,
    Array,
    Reference,
    Callable,
    RichValue,
}

impl FormulaValueKind {
    pub fn of(value: &CalcValue) -> Self {
        if value.callable_value().is_some() {
            return Self::Callable;
        }
        if value.rich().is_some() {
            return Self::RichValue;
        }
        match value.core() {
            CoreValue::Number(_) => Self::Number,
            CoreValue::Text(_) => Self::Text,
            CoreValue::Logical(_) => Self::Logical,
            CoreValue::Error(_) => Self::Error,
            CoreValue::Empty => Self::Empty,
            CoreValue::Missing => Self::Missing,
            CoreValue::Array(_) => Self::Array,
            CoreValue::Reference(_) => Self::Reference,
        }
    }
}

/// Render the canonical Excel literal for a worksheet error code
/// (e.g. `#NAME?`, `#DIV/0!`). Free-standing helper over the upstream
/// `WorksheetErrorCode` so call sites don't have to write the match
/// themselves.
pub fn worksheet_error_literal(code: WorksheetErrorCode) -> &'static str {
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

/// Walk-tree projection of a formula. Host-defined: upstream exposes a
/// richer `SemanticPlan` / `GreenTreeRoot`; this is the UX-shaped slice
/// the formula drill-down renders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaDrillNodeViewModel {
    pub node_id: String,
    pub label: String,
    pub developer_label: Option<String>,
    pub expression_text: Option<String>,
    pub kind: Option<String>,
    pub source_span_start: Option<usize>,
    pub source_span_len: Option<usize>,
    pub branch_disposition: Option<String>,
    pub argument_name: Option<String>,
    pub argument_role: Option<String>,
    pub error_message: Option<String>,
    pub value_preview: Option<String>,
    pub array_preview: Option<FormulaDrillArrayPreview>,
    pub state: FormulaDrillNodeState,
    pub children: Vec<FormulaDrillNodeViewModel>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaDrillArrayPreview {
    pub total_rows: usize,
    pub total_cols: usize,
    pub rows: Vec<Vec<String>>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormulaDrillNodeState {
    Pending,
    Evaluated,
    Bound,
    Skipped,
    Opaque,
    Blocked,
    Error,
}

/// Compact parse-phase summary. Host-bundle of upstream parse signals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseSummary {
    pub status: String,
    pub token_count: usize,
}

/// Compact bind-phase summary. Host-bundle of upstream bind signals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindSummary {
    pub variable_count: usize,
    pub reference_count: usize,
}

/// Compact eval-phase summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalSummary {
    pub step_count: usize,
    pub duration_text: String,
}

/// Provenance fragment used in the legacy three-mode shell. Will retire
/// during the WS-14 cleanup phase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvenanceSummary {
    pub profile_summary: String,
    pub blocked_reason: Option<String>,
}

/// Truncated array preview for the result block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaArrayPreview {
    pub label: String,
    pub rows: Vec<Vec<String>>,
    pub truncated: bool,
}

/// What the host shows for the cell's value. Bundles the upstream typed
/// value with display strings so the UI layer can pick the right rendering
/// without re-running OxFml.
#[derive(Debug, Clone, PartialEq)]
pub struct FormulaResultViewModel {
    pub evaluation_summary: String,
    pub effective_display_summary: Option<String>,
    pub array_preview: Option<FormulaArrayPreview>,
    pub blocked_reason: Option<String>,
    /// Typed value passed through verbatim from `RuntimeFormulaResult.published_worksheet_value`.
    /// View-models dispatch on this discriminator (Number / Text / Logical /
    /// Error / Empty / Missing / Array / Reference / Callable / RichValue)
    /// instead of parsing strings.
    pub published_value: CalcValue,
    /// Function-emitted presentation hint, lifted from
    /// `RuntimeFormulaResult.returned_value_surface.presentation_hint`.
    /// `None` for ordinary values; `Some` when a function (e.g. NOW,
    /// TODAY) returned a `CalcValue` with a presentation hint. Drives
    /// the host's "auto-format on General default" behaviour: when the
    /// formatting state is empty (General), the host applies the hint's
    /// default code so a fresh `=NOW()` reads as a date+time without
    /// the user having to pick a format manually.
    pub number_format_hint: Option<NumberFormatHint>,
    /// Effective font colour for the result hero, lifted from
    /// `verification_publication_surface.effective_font_color`. `Some`
    /// when a CF rule fired and overrode the font colour, or
    /// (post-colour-token-publication-handoff) when the format code
    /// included a `[Red]` / `[Color3]` token that selected the active
    /// section. Hex `#RRGGBB`. `None` means "render in default chrome".
    pub effective_font_color: Option<String>,
    /// Effective fill colour for the result hero. Same source as
    /// `effective_font_color` — CF-rule-fired or colour-token. Hex
    /// `#RRGGBB`. `None` means "no fill applied".
    pub effective_fill_color: Option<String>,
    /// Per-cell CF outcomes for array results. `Some` when the
    /// published value has `CoreValue::Array`; `None` for scalar
    /// values. Each cell carries its own font / fill / data-bar /
    /// icon override based on the active CF rules with the array
    /// as the implicit "selected range" (see
    /// `docs/HANDOFF_OXFML_CF_ARRAY_PER_CELL.md` and
    /// `docs/HANDOFF_OXFML_CF_AGGREGATE_VISUALIZATION_RULES.md`).
    pub array_cell_format: Option<ArrayCellFormatGrid>,
    /// OxFunc kernel/admission metadata versions carried by OxFml's
    /// prepared formula identity. These are replay and invalidation
    /// signals, not display text.
    pub semantic_kernel_metadata_version: Option<String>,
    pub arg_admission_metadata_version: Option<String>,
    /// Rich/sparse producer capability facts from OxFml's returned
    /// value surface. Empty means this run emitted no capability facts.
    pub producer_capability_set_keys: Vec<String>,
    pub exercised_capability_keys: Vec<String>,
}

/// Host's bundle of upstream editor-interaction + runtime-result. This
/// is what a single `apply_formula_edit` call produces and what the
/// host stores per formula space. Field types reference upstream types
/// directly (no information loss at the bridge boundary).
#[derive(Debug, Clone, PartialEq)]
pub struct EditorDocument {
    pub source_text: String,
    pub text_change_range: Option<FormulaTextChangeRange>,
    pub editor_syntax_snapshot: EditorSyntaxSnapshot,
    pub live_diagnostics: LiveDiagnosticSnapshot,
    pub reuse_summary: FormulaEditReuseSummary,
    pub signature_help: Option<SignatureHelpContext>,
    pub function_help: Option<FunctionHelpPacket>,
    pub completion_proposals: Vec<CompletionProposal>,
    pub formula_walk: Vec<FormulaDrillNodeViewModel>,
    pub parse_summary: Option<ParseSummary>,
    pub bind_summary: Option<BindSummary>,
    pub eval_summary: Option<EvalSummary>,
    pub provenance_summary: Option<ProvenanceSummary>,
    pub value_presentation: Option<FormulaResultViewModel>,
}

impl EditorDocument {
    pub fn green_tree_key(&self) -> &str {
        &self.editor_syntax_snapshot.green_tree_key
    }
}
