use serde::{Deserialize, Serialize};

/// Single-formula document projection used by DNA OneCalc and by any skin
/// that wants the formula editor/value/drill surface without a workbook tree.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct OneFormulaProjection {
    pub formula_space_id: String,
    pub formula_stable_id: String,
    pub display_name: String,
    pub raw_entered_cell_text: String,
    pub entry_mode: FormulaEntryModeProjection,
    pub editor: FormulaEditorSurface,
    pub assist: FormulaAssistSurface,
    pub result: FormulaResultSurface,
    pub formatting: FormattingSurface,
    pub comparison: ComparisonSurface,
    pub drill: FormulaDrillSurface,
    pub status: FormulaStatusSurface,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FormulaEntryModeProjection {
    Formula,
    Value,
    Text,
    #[default]
    Empty,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormulaEditorSurface {
    pub source_text: String,
    pub caret_offset: usize,
    pub selection_anchor: usize,
    pub selection_focus: usize,
    pub syntax_runs: Vec<SyntaxRunProjection>,
    pub diagnostics: Vec<FormulaDiagnosticProjection>,
    pub metrics: EditorMetricsProjection,
    pub document_is_fresh: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxRunProjection {
    pub text: String,
    pub span_start: usize,
    pub span_len: usize,
    pub role: SyntaxTokenRoleProjection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyntaxTokenRoleProjection {
    Operator,
    Function,
    Number,
    Delimiter,
    Identifier,
    Text,
    Trivia,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorMetricsProjection {
    pub token_count: usize,
    pub function_count: usize,
    pub diagnostic_count: usize,
    pub first_diagnostic_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormulaDiagnosticProjection {
    pub diagnostic_id: String,
    pub severity: DiagnosticSeverityProjection,
    pub stage: DiagnosticStageProjection,
    pub code: Option<String>,
    pub worksheet_error_class: Option<String>,
    pub message: String,
    pub span_start: usize,
    pub span_len: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticSeverityProjection {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticStageProjection {
    Syntax,
    Bind,
    SemanticPlan,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormulaAssistSurface {
    pub completion: Option<CompletionSurface>,
    pub signature_help: Option<SignatureHelpSurface>,
    pub function_help: Option<FunctionHelpSurface>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompletionSurface {
    pub anchor_left_px: usize,
    pub anchor_top_px: usize,
    pub line_height_px: usize,
    pub selected_index: usize,
    pub items: Vec<CompletionItemProjection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompletionItemProjection {
    pub proposal_id: String,
    pub display_text: String,
    pub kind: CompletionKindProjection,
    pub documentation_ref: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompletionKindProjection {
    Function,
    DefinedName,
    TableName,
    TableColumn,
    StructuredSelector,
    SyntaxAssist,
    ProfileReference,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignatureHelpSurface {
    pub callee_text: String,
    pub anchor_left_px: usize,
    pub anchor_top_px: usize,
    pub line_height_px: usize,
    pub parameters: Vec<SignatureHelpParameterProjection>,
    pub active_parameter: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignatureHelpParameterProjection {
    pub name: String,
    pub is_active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionHelpSurface {
    pub lookup_key: String,
    pub display_name: String,
    pub signature: Option<String>,
    pub short_description: Option<String>,
    pub availability_summary: Option<String>,
    pub deferred_or_profile_limited: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FormulaResultSurface {
    Empty,
    Pending,
    Display {
        text: String,
        value: CalcValueProjection,
        applied_font_color: Option<String>,
        applied_fill_color: Option<String>,
    },
    Error {
        code: String,
        surface_repr: Option<String>,
    },
    Array {
        total_rows: usize,
        total_cols: usize,
        label: String,
        window: ArrayWindowProjection,
        truncated: bool,
    },
}

impl Default for FormulaResultSurface {
    fn default() -> Self {
        Self::Empty
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CalcValueProjection {
    pub core: CoreValueProjection,
    pub display_text: String,
    pub presentation_hint: Option<PresentationHintProjection>,
    pub rich_value_kind: Option<String>,
    pub callable: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CoreValueProjection {
    Number {
        raw: String,
    },
    Text {
        text: String,
    },
    Logical {
        value: bool,
    },
    Error {
        code: String,
        display: String,
    },
    #[default]
    Empty,
    Missing,
    Reference {
        target: String,
    },
    Array {
        shape: ArrayShapeProjection,
    },
    RichValue {
        summary: String,
    },
    Callable {
        summary: String,
    },
    Other {
        summary: String,
    },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArrayShapeProjection {
    pub rows: usize,
    pub cols: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ArrayWindowProjection {
    pub total_rows: usize,
    pub total_cols: usize,
    pub row_offset: usize,
    pub col_offset: usize,
    pub cells: Vec<Vec<ArrayWindowCellProjection>>,
}

pub const MAX_ARRAY_WINDOW_CELLS: usize = 4_096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArrayWindowError {
    Empty,
    Ragged,
    OutsideArray,
    TooLarge,
    Overflow,
}

impl ArrayWindowProjection {
    pub fn validate(&self) -> Result<(), ArrayWindowError> {
        let rows = self.cells.len();
        let cols = self.cells.first().map_or(0, Vec::len);
        if rows == 0 || cols == 0 {
            return Err(ArrayWindowError::Empty);
        }
        if self.cells.iter().any(|row| row.len() != cols) {
            return Err(ArrayWindowError::Ragged);
        }
        if rows.checked_mul(cols).ok_or(ArrayWindowError::Overflow)? > MAX_ARRAY_WINDOW_CELLS {
            return Err(ArrayWindowError::TooLarge);
        }
        let row_end = self
            .row_offset
            .checked_add(rows)
            .ok_or(ArrayWindowError::Overflow)?;
        let col_end = self
            .col_offset
            .checked_add(cols)
            .ok_or(ArrayWindowError::Overflow)?;
        if row_end > self.total_rows || col_end > self.total_cols {
            return Err(ArrayWindowError::OutsideArray);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ArrayWindowCellProjection {
    pub display_text: String,
    pub value: Option<CalcValueProjection>,
    pub format: Option<ArrayCellFormatProjection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresentationHintProjection {
    pub family: String,
    pub format_code: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ArrayCellFormatProjection {
    pub effective_font_color: Option<String>,
    pub effective_fill_color: Option<String>,
    pub data_bar: Option<DataBarFillProjection>,
    pub icon: Option<CfIconProjection>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataBarFillProjection {
    pub fill_ratio: f64,
    pub bar_color: String,
    pub direction: DataBarDirectionProjection,
    pub show_bar_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataBarDirectionProjection {
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CfIconProjection {
    pub set_kind: String,
    pub icon_index: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FormattingSurface {
    pub number_format_code: Option<String>,
    pub font_color: Option<String>,
    pub fill_color: Option<String>,
    pub date1904: bool,
    pub locale_language_tag: String,
    pub scenario_policy: ScenarioPolicyProjection,
    pub conditional_formatting_rules: Vec<ConditionalFormattingRuleProjection>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioPolicyProjection {
    Deterministic,
    #[default]
    LiveRecalc,
    ManualRecalc,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ConditionalFormattingRuleProjection {
    pub operator: Option<String>,
    pub thresholds: Vec<ConditionalFormattingThresholdProjection>,
    pub font_color: Option<String>,
    pub fill_color: Option<String>,
    pub typed_rule: Option<ConditionalFormattingTypedRuleProjection>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ConditionalFormattingThresholdProjection {
    Min,
    Mid,
    Max,
    Percent(f64),
    Percentile(f64),
    Number(f64),
    Text(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "rule", rename_all = "snake_case")]
pub enum ConditionalFormattingTypedRuleProjection {
    ColorScale(ColorScaleRuleProjection),
    DataBar(DataBarRuleProjection),
    IconSet(IconSetRuleProjection),
    Rank(RankRuleProjection),
    Average(AverageRuleProjection),
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ComparisonSurface {
    pub available: bool,
    pub baseline_label: Option<String>,
    pub candidate_label: Option<String>,
    pub outcome: Option<ComparisonOutcomeProjection>,
    pub details: Vec<ComparisonDetailProjection>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonOutcomeProjection {
    Equal,
    Different,
    Incomparable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComparisonDetailProjection {
    pub path: String,
    pub baseline: String,
    pub candidate: String,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ColorScaleRuleProjection {
    pub stops: Vec<ColorScaleStopProjection>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColorScaleStopProjection {
    pub position: ConditionalFormattingThresholdProjection,
    pub color: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DataBarRuleProjection {
    pub minimum: Option<ConditionalFormattingThresholdProjection>,
    pub maximum: Option<ConditionalFormattingThresholdProjection>,
    pub bar_color: Option<String>,
    pub direction: Option<DataBarDirectionProjection>,
    pub show_bar_only: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IconSetRuleProjection {
    pub set_kind: String,
    pub thresholds: Vec<ConditionalFormattingThresholdProjection>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum RankRuleProjection {
    Count(usize),
    Percent(f64),
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AverageRuleProjection {
    pub include_equal: bool,
    pub stddev_multiplier: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormulaDrillSurface {
    pub expanded: bool,
    pub tree: Vec<FormulaDrillNodeProjection>,
    pub diagnostics: Vec<FormulaDiagnosticProjection>,
    pub phase_summaries: Vec<FormulaDrillPhaseProjection>,
    pub document_is_fresh: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormulaDrillNodeProjection {
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
    pub array_preview: Option<ArrayPreviewProjection>,
    pub state: FormulaDrillNodeStateProjection,
    pub children: Vec<FormulaDrillNodeProjection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArrayPreviewProjection {
    pub total_rows: usize,
    pub total_cols: usize,
    #[serde(default)]
    pub row_offset: usize,
    #[serde(default)]
    pub col_offset: usize,
    pub rows: Vec<Vec<String>>,
    pub truncated: bool,
}

impl ArrayPreviewProjection {
    pub fn validate(&self) -> Result<(), ArrayWindowError> {
        let rows = self.rows.len();
        let cols = self.rows.first().map_or(0, Vec::len);
        if rows == 0 || cols == 0 {
            return Err(ArrayWindowError::Empty);
        }
        if self.rows.iter().any(|row| row.len() != cols) {
            return Err(ArrayWindowError::Ragged);
        }
        if rows.checked_mul(cols).ok_or(ArrayWindowError::Overflow)? > MAX_ARRAY_WINDOW_CELLS {
            return Err(ArrayWindowError::TooLarge);
        }
        let row_end = self
            .row_offset
            .checked_add(rows)
            .ok_or(ArrayWindowError::Overflow)?;
        let col_end = self
            .col_offset
            .checked_add(cols)
            .ok_or(ArrayWindowError::Overflow)?;
        if row_end > self.total_rows || col_end > self.total_cols {
            return Err(ArrayWindowError::OutsideArray);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FormulaDrillNodeStateProjection {
    Pending,
    Evaluated,
    Bound,
    Skipped,
    Opaque,
    Blocked,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormulaDrillPhaseProjection {
    pub label: String,
    pub detail: String,
    pub state: FormulaDrillPhaseStateProjection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FormulaDrillPhaseStateProjection {
    Ok,
    Pending,
    Blocked,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormulaStatusSurface {
    pub bridge_health: BridgeHealthProjection,
    pub truth_source: String,
    pub green_tree_key: Option<String>,
    pub scenario_label: String,
    pub load_diagnostics: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BridgeHealthProjection {
    Live,
    #[default]
    Stale,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OneFormulaIntent {
    EditText {
        formula_space_id: String,
        text: String,
        caret_offset: usize,
    },
    SetSelection {
        formula_space_id: String,
        anchor: usize,
        focus: usize,
    },
    ApplyCompletion {
        formula_space_id: String,
        proposal_id: String,
    },
    SetNumberFormat {
        formula_space_id: String,
        number_format_code: Option<String>,
    },
    SetScenarioPolicy {
        formula_space_id: String,
        policy: ScenarioPolicyProjection,
    },
    ToggleFormulaDrill {
        formula_space_id: String,
    },
    Recalculate {
        formula_space_id: String,
    },
    RequestResultArrayWindow {
        formula_space_id: String,
        row_offset: usize,
        col_offset: usize,
        row_count: usize,
        col_count: usize,
    },
    RequestDrillArrayWindow {
        formula_space_id: String,
        node_id: String,
        row_offset: usize,
        col_offset: usize,
        row_count: usize,
        col_count: usize,
    },
    /// Author the font attributes on the active formula's format
    /// surface. `font_color` is a hex `#RRGGBB`; `None` (or an empty
    /// string) clears it back to inherit / default. The OneFormula
    /// format model carries font *colour* today — the read
    /// [`FormattingSurface`] mirrors only `font_color`; bold / italic /
    /// size would extend this verb and the surface together, out of
    /// this slice.
    SetFontAttributes {
        formula_space_id: String,
        font_color: Option<String>,
    },
    /// Author the fill (background) colour on the active formula's
    /// format surface. `fill_color` is a hex `#RRGGBB`; `None` (or an
    /// empty string) clears it back to inherit / default.
    SetFillAttributes {
        formula_space_id: String,
        fill_color: Option<String>,
    },
    /// Switch the locale the active formula renders under.
    /// `locale_language_tag` is a BCP-47 tag (e.g. `"en-US"`,
    /// `"de-DE"`); the host resolves it to a locale profile so month /
    /// weekday names, separators, currency symbol, and `General`
    /// rendering all flip. Workspace-scoped — it matches the read
    /// model's [`FormattingSurface::locale_language_tag`], which
    /// reflects the ambient app context; `formula_space_id` addresses
    /// the active document for uniformity even though the effect is
    /// ambient.
    SetLocale {
        formula_space_id: String,
        locale_language_tag: String,
    },
    /// Set the active formula's date system: `false` = 1900 (default),
    /// `true` = 1904 (Mac legacy). Shifts how date serials render.
    SetDate1904 {
        formula_space_id: String,
        date1904: bool,
    },
    /// Author a conditional-formatting rule on the active formula's
    /// result hero. `rule_index` `None` appends a new rule; `Some(i)`
    /// replaces the rule already at index `i`. The payload is the
    /// honest cell-value operator slice OxFml evaluates today (see
    /// [`ConditionalFormatRuleAuthoring`]); typed visualization rules
    /// (colour scale / data bar / icon set / rank / average) author
    /// through a richer follow-on editor.
    SetConditionalFormatRule {
        formula_space_id: String,
        rule_index: Option<usize>,
        rule: ConditionalFormatRuleAuthoring,
    },
    /// Remove the conditional-formatting rule at `rule_index` from the
    /// active formula's result hero. An honest no-op when the index is
    /// out of range.
    RemoveConditionalFormatRule {
        formula_space_id: String,
        rule_index: usize,
    },
}

/// The authoring payload for [`OneFormulaIntent::SetConditionalFormatRule`]
/// — the minimal honest slice OxFml evaluates today: operator-driven
/// cell-value / text / predicate rules with optional whole-rule font /
/// fill overrides. Distinct from the read-model
/// [`ConditionalFormattingRuleProjection`] (a display shape carrying
/// typed thresholds): this is the *write* shape, which the host maps
/// 1:1 onto its `FormulaConditionalFormattingRule` with `typed_rule:
/// None`. Typed visualization rules author through a richer follow-on.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ConditionalFormatRuleAuthoring {
    /// Rule-kind label OxFml matches on (`"cell_value"`, `"text"`,
    /// `"dates"`, `"blanks"`, `"errors"`, `"expression"`, …).
    pub rule_kind: String,
    /// Canonical Excel CF operator name (`"greaterThan"`, `"between"`,
    /// `"equal"`, `"containsText"`, …); `None` for predicate-only kinds.
    pub operator: Option<String>,
    /// Threshold operands in operator order (one; two for `between`);
    /// empty for predicate-only kinds.
    pub thresholds: Vec<String>,
    /// Hex `#RRGGBB` font colour applied when the rule fires.
    pub font_color: Option<String>,
    /// Hex `#RRGGBB` fill colour applied when the rule fires.
    pub fill_color: Option<String>,
}

impl OneFormulaIntent {
    pub fn validate(&self) -> Result<(), ArrayWindowError> {
        let dimensions = match self {
            Self::RequestResultArrayWindow {
                row_count,
                col_count,
                ..
            }
            | Self::RequestDrillArrayWindow {
                row_count,
                col_count,
                ..
            } => Some((*row_count, *col_count)),
            _ => None,
        };
        let Some((rows, cols)) = dimensions else {
            return Ok(());
        };
        if rows == 0 || cols == 0 {
            return Err(ArrayWindowError::Empty);
        }
        if rows.checked_mul(cols).ok_or(ArrayWindowError::Overflow)? > MAX_ARRAY_WINDOW_CELLS {
            return Err(ArrayWindowError::TooLarge);
        }
        match self {
            Self::RequestResultArrayWindow {
                row_offset,
                col_offset,
                ..
            }
            | Self::RequestDrillArrayWindow {
                row_offset,
                col_offset,
                ..
            } => {
                row_offset
                    .checked_add(rows)
                    .ok_or(ArrayWindowError::Overflow)?;
                col_offset
                    .checked_add(cols)
                    .ok_or(ArrayWindowError::Overflow)?;
            }
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_formula_projection_round_trips() {
        let projection = OneFormulaProjection {
            formula_space_id: "formula-1".to_string(),
            formula_stable_id: "stable-1".to_string(),
            display_name: "Formula 1".to_string(),
            raw_entered_cell_text: "=SUM(1,2,3)".to_string(),
            entry_mode: FormulaEntryModeProjection::Formula,
            editor: FormulaEditorSurface {
                source_text: "=SUM(1,2,3)".to_string(),
                caret_offset: 11,
                selection_anchor: 11,
                selection_focus: 11,
                syntax_runs: vec![SyntaxRunProjection {
                    text: "SUM".to_string(),
                    span_start: 1,
                    span_len: 3,
                    role: SyntaxTokenRoleProjection::Function,
                }],
                diagnostics: Vec::new(),
                metrics: EditorMetricsProjection {
                    token_count: 6,
                    function_count: 1,
                    diagnostic_count: 0,
                    first_diagnostic_message: None,
                },
                document_is_fresh: true,
            },
            assist: FormulaAssistSurface::default(),
            result: FormulaResultSurface::Display {
                text: "6".to_string(),
                value: CalcValueProjection {
                    core: CoreValueProjection::Number {
                        raw: "6".to_string(),
                    },
                    display_text: "6".to_string(),
                    presentation_hint: None,
                    rich_value_kind: None,
                    callable: false,
                },
                applied_font_color: None,
                applied_fill_color: None,
            },
            formatting: FormattingSurface::default(),
            comparison: ComparisonSurface::default(),
            drill: FormulaDrillSurface::default(),
            status: FormulaStatusSurface::default(),
        };

        let json = serde_json::to_string(&projection).expect("serialize projection");
        let restored: OneFormulaProjection =
            serde_json::from_str(&json).expect("deserialize projection");
        assert_eq!(restored, projection);
    }

    #[test]
    fn array_window_round_trips_with_cell_formatting() {
        let window = ArrayWindowProjection {
            total_rows: 2,
            total_cols: 2,
            row_offset: 0,
            col_offset: 0,
            cells: vec![vec![ArrayWindowCellProjection {
                display_text: "1".to_string(),
                value: Some(CalcValueProjection {
                    core: CoreValueProjection::Number {
                        raw: "1".to_string(),
                    },
                    display_text: "1".to_string(),
                    presentation_hint: None,
                    rich_value_kind: None,
                    callable: false,
                }),
                format: Some(ArrayCellFormatProjection {
                    effective_font_color: Some("#FF0000".to_string()),
                    effective_fill_color: None,
                    data_bar: Some(DataBarFillProjection {
                        fill_ratio: 0.5,
                        bar_color: "#00AAFF".to_string(),
                        direction: DataBarDirectionProjection::Right,
                        show_bar_only: false,
                    }),
                    icon: Some(CfIconProjection {
                        set_kind: "3Arrows".to_string(),
                        icon_index: 1,
                    }),
                }),
            }]],
        };

        let json = serde_json::to_string(&window).expect("serialize array window");
        let restored: ArrayWindowProjection =
            serde_json::from_str(&json).expect("deserialize array window");
        assert_eq!(restored, window);
    }

    #[test]
    fn bounded_array_window_request_has_stable_golden_shape() {
        let intent = OneFormulaIntent::RequestDrillArrayWindow {
            formula_space_id: "formula-1".to_string(),
            node_id: "call-2".to_string(),
            row_offset: 10,
            col_offset: 2,
            row_count: 20,
            col_count: 4,
        };
        assert_eq!(
            serde_json::to_string(&intent).expect("serialize intent"),
            r#"{"kind":"request_drill_array_window","formula_space_id":"formula-1","node_id":"call-2","row_offset":10,"col_offset":2,"row_count":20,"col_count":4}"#
        );
    }

    #[test]
    fn array_window_validation_rejects_bad_shapes_and_large_requests() {
        let cell = ArrayWindowCellProjection::default();
        let mut window = ArrayWindowProjection {
            total_rows: 2,
            total_cols: 2,
            row_offset: 0,
            col_offset: 0,
            cells: vec![vec![cell.clone(), cell.clone()], vec![cell.clone(), cell]],
        };
        assert_eq!(window.validate(), Ok(()));
        window.cells[1].pop();
        assert_eq!(window.validate(), Err(ArrayWindowError::Ragged));
        window.cells.clear();
        assert_eq!(window.validate(), Err(ArrayWindowError::Empty));
        let request = OneFormulaIntent::RequestResultArrayWindow {
            formula_space_id: "f".into(),
            row_offset: 0,
            col_offset: 0,
            row_count: 65,
            col_count: 65,
        };
        assert_eq!(request.validate(), Err(ArrayWindowError::TooLarge));
        let overflow = OneFormulaIntent::RequestResultArrayWindow {
            formula_space_id: "f".into(),
            row_offset: usize::MAX,
            col_offset: 0,
            row_count: 1,
            col_count: 1,
        };
        assert_eq!(overflow.validate(), Err(ArrayWindowError::Overflow));
        let preview = ArrayPreviewProjection {
            total_rows: 2,
            total_cols: 2,
            row_offset: 0,
            col_offset: 0,
            rows: vec![vec!["1".into(), "2".into()], vec!["3".into()]],
            truncated: true,
        };
        assert_eq!(preview.validate(), Err(ArrayWindowError::Ragged));
    }

    #[test]
    fn formatting_drill_comparison_and_array_goldens_are_stable() {
        let formatting = serde_json::to_value(FormattingSurface::default()).unwrap();
        assert_eq!(
            formatting,
            serde_json::json!({
                "number_format_code": null, "font_color": null, "fill_color": null,
                "date1904": false, "locale_language_tag": "", "scenario_policy": "live_recalc",
                "conditional_formatting_rules": []
            })
        );
        assert_eq!(
            serde_json::to_value(FormulaDrillSurface::default()).unwrap(),
            serde_json::json!({
                "expanded": false, "tree": [], "diagnostics": [], "phase_summaries": [], "document_is_fresh": false
            })
        );
        assert_eq!(
            serde_json::to_value(ComparisonSurface::default()).unwrap(),
            serde_json::json!({
                "available": false, "baseline_label": null, "candidate_label": null, "outcome": null, "details": []
            })
        );
        let window = ArrayWindowProjection {
            total_rows: 1,
            total_cols: 1,
            row_offset: 0,
            col_offset: 0,
            cells: vec![vec![ArrayWindowCellProjection::default()]],
        };
        assert_eq!(
            serde_json::to_value(window).unwrap(),
            serde_json::json!({
                "total_rows": 1, "total_cols": 1, "row_offset": 0, "col_offset": 0,
                "cells": [[{"display_text":"", "value":null, "format":null}]]
            })
        );
        let cf = ConditionalFormattingRuleProjection {
            operator: None,
            thresholds: vec![],
            font_color: None,
            fill_color: None,
            typed_rule: Some(ConditionalFormattingTypedRuleProjection::DataBar(
                DataBarRuleProjection {
                    minimum: Some(ConditionalFormattingThresholdProjection::Min),
                    maximum: Some(ConditionalFormattingThresholdProjection::Max),
                    bar_color: Some("#00AAFF".into()),
                    direction: Some(DataBarDirectionProjection::Right),
                    show_bar_only: false,
                },
            )),
        };
        assert_eq!(
            serde_json::to_value(cf).unwrap(),
            serde_json::json!({
                "operator":null,"thresholds":[],"font_color":null,"fill_color":null,
                "typed_rule":{"kind":"data_bar","rule":{"minimum":{"kind":"min"},"maximum":{"kind":"max"},"bar_color":"#00AAFF","direction":"right","show_bar_only":false}}
            })
        );
        let preview = ArrayPreviewProjection {
            total_rows: 2,
            total_cols: 2,
            row_offset: 0,
            col_offset: 0,
            rows: vec![vec!["1".into(), "2".into()]],
            truncated: true,
        };
        let node = FormulaDrillNodeProjection {
            node_id: "n1".into(),
            label: "SEQUENCE".into(),
            developer_label: None,
            expression_text: Some("SEQUENCE(2,2)".into()),
            kind: Some("call".into()),
            source_span_start: Some(1),
            source_span_len: Some(13),
            branch_disposition: None,
            argument_name: None,
            argument_role: None,
            error_message: None,
            value_preview: Some("{2x2}".into()),
            array_preview: Some(preview),
            state: FormulaDrillNodeStateProjection::Evaluated,
            children: vec![],
        };
        let value = serde_json::to_value(node).unwrap();
        assert_eq!(
            value["array_preview"]["rows"],
            serde_json::json!([["1", "2"]])
        );
        assert_eq!(value["state"], "evaluated");
    }

    #[test]
    fn format_authoring_intents_round_trip() {
        use OneFormulaIntent as I;

        // Each new write verb round-trips losslessly (fail-pre-fix:
        // these variants did not exist before dtc-r8d5).
        let cases = vec![
            I::SetFontAttributes {
                formula_space_id: "f1".into(),
                font_color: Some("#D02A23".into()),
            },
            I::SetFillAttributes {
                formula_space_id: "f1".into(),
                fill_color: None,
            },
            I::SetLocale {
                formula_space_id: "f1".into(),
                locale_language_tag: "de-DE".into(),
            },
            I::SetDate1904 {
                formula_space_id: "f1".into(),
                date1904: true,
            },
            I::SetConditionalFormatRule {
                formula_space_id: "f1".into(),
                rule_index: None,
                rule: ConditionalFormatRuleAuthoring {
                    rule_kind: "cell_value".into(),
                    operator: Some("greaterThan".into()),
                    thresholds: vec!["100".into()],
                    font_color: Some("#FFFFFF".into()),
                    fill_color: Some("#006600".into()),
                },
            },
            I::RemoveConditionalFormatRule {
                formula_space_id: "f1".into(),
                rule_index: 2,
            },
        ];
        for intent in cases {
            let json = serde_json::to_value(&intent).unwrap();
            let back: OneFormulaIntent = serde_json::from_value(json).unwrap();
            assert_eq!(intent, back);
        }

        // The `kind` tags are the stable snake_case wire contract.
        assert_eq!(
            serde_json::to_value(I::SetFontAttributes {
                formula_space_id: "f1".into(),
                font_color: Some("#D02A23".into()),
            })
            .unwrap(),
            serde_json::json!({
                "kind": "set_font_attributes",
                "formula_space_id": "f1",
                "font_color": "#D02A23"
            })
        );
        assert_eq!(
            serde_json::to_value(I::SetConditionalFormatRule {
                formula_space_id: "f1".into(),
                rule_index: Some(0),
                rule: ConditionalFormatRuleAuthoring {
                    rule_kind: "cell_value".into(),
                    operator: Some("between".into()),
                    thresholds: vec!["1".into(), "9".into()],
                    font_color: None,
                    fill_color: Some("#FFE9A8".into()),
                },
            })
            .unwrap(),
            serde_json::json!({
                "kind": "set_conditional_format_rule",
                "formula_space_id": "f1",
                "rule_index": 0,
                "rule": {
                    "rule_kind": "cell_value",
                    "operator": "between",
                    "thresholds": ["1", "9"],
                    "font_color": null,
                    "fill_color": "#FFE9A8"
                }
            })
        );
    }
}
