//! WS-14 home-shell view-model.
//!
//! Pure projection function that reads the active formula space out of
//! `OneCalcHostState` and produces a small `HomeShellViewModel` describing
//! what the home shell renders: textarea text + caret, a five-way
//! `ResultView`, and a `StatusView` for the status-foot dot + green-tree key.
//!
//! The result projection dispatches **typed**: it reads the bridge's typed
//! `published_value: CalcValue` from `editor_document.value_presentation`,
//! the live diagnostics list from `editor_document.live_diagnostics`, the
//! provenance blocked reason from `editor_document.provenance_summary`, and
//! the host-derived `context.blocked_reason` — never re-parsing the
//! `latest_evaluation_summary` string. Pre-bridge text/number cells (input
//! that does not start with `=`) are hand-evaluated inline against the raw
//! source text. Array results flow through `formula_space.array_preview`.
//!
//! Reference: `docs/WS14_PRE_MVP_PATH.md` §4 Step 2.

use crate::adapters::oxfml::{
    worksheet_error_literal, CalcValue, CoreValue, LiveDiagnosticSeverity,
};
use crate::services::capability_snapshot::build_capability_ledger_snapshot;
use crate::services::completion_popup::{CompletionPopupKind, CompletionPopupState};
use crate::services::function_semantic_profile::{
    project_function_semantic_profile, FunctionSemanticProfileRow,
};
use crate::state::{
    CapabilityLedgerSnapshot, VbaHostAssociationSourceKind, VbaHostAssociationState,
};
use crate::state::{FormulaSpaceState, OneCalcHostState, ProjectionTruthSource, ViewMode};
use crate::ui::editor::geometry::caret_box_for_offset;
use crate::ui::editor::render_projection::{syntax_runs_from_snapshot, SyntaxRun, SyntaxTokenRole};
use crate::ui::editor::state::{EditorEntryMode, EditorSurfaceState};

/// Top-level home-shell projection.
///
/// Built freshly per render via `build_home_shell_view_model`. Returns
/// `None` when there is no active formula space (the home shell renders an
/// empty state in that case).
// `Eq` cannot be derived: `ResultView::Array.cell_format` carries
// `DataBarFillView { fill_ratio: f64, ... }` (W072 visualization
// payload) and `f64` does not implement `Eq`.
#[derive(Debug, Clone, PartialEq)]
pub struct HomeShellViewModel {
    /// Canonical shared DNA Calc skin IR snapshot for the active formula
    /// surface. The legacy fields below remain while the renderer is migrated,
    /// but this snapshot is the long-term UX contract shared with TreeCalc.
    pub skin_snapshot: dnacalc_skin_ir::SkinSnapshot,
    pub raw_entered_cell_text: String,
    pub editor_surface_state: EditorSurfaceState,
    /// Pill rendered above the editor textarea (Formula / Value / Text /
    /// Empty), classified from `raw_entered_cell_text`.
    pub entry_mode_pill: EntryModePill,
    /// Pill rendered above the result hero (Number / Text / Logical /
    /// Error / Array / Other). `None` when there is no result to label —
    /// `ResultView::Empty` and `ResultView::Pending` suppress the pill.
    pub result_class_pill: Option<ResultClassPill>,
    /// Coloured-token runs for the syntax overlay rendered behind the
    /// textarea. Empty when the editor document is missing or stale (its
    /// source text does not match `raw_entered_cell_text`); the home shell
    /// renders the raw text uncoloured in that case.
    pub syntax_runs: Vec<SyntaxRun>,
    /// Diagnostic squiggles to overlay on top of the textarea, one entry
    /// per upstream `LiveDiagnostic`. Sorted by `span_start` ascending and
    /// pruned of entries that overlap with an earlier one (the upstream
    /// list is already non-overlapping in practice; the prune is a
    /// belt-and-braces guard).
    pub diagnostic_squiggles: Vec<DiagnosticSquiggle>,
    /// Live counts strip rendered at the editor-foot chip:
    /// `tokens N · functions M · diagnostics K`. Counts come straight off
    /// the editor document (zeros when there is no document yet).
    pub editor_metrics: EditorMetricsChip,
    /// Active-context summary rendered at the result-foot chip:
    /// `format · policy`. `None` when the formula is in the
    /// default state (General number format, `LiveRecalc` policy,
    /// no CF rules) — the result-foot collapses entirely on
    /// "nothing to say", freeing vertical space for the result
    /// hero. `Some` whenever any of the three diverges from
    /// default OR when the policy is `ManualRecalc` (always
    /// surfaced so the user has a visible reminder that typing
    /// isn't triggering recalc). Per WS-14 §5 ("result-foot
    /// rethink"): rather than always-on chrome, the foot is
    /// progressive — visible only when it carries information.
    pub result_context: Option<ResultContextChip>,
    /// Completion popup overlay. `None` when the popup is `Hidden` OR
    /// when the editor-box metrics have not yet been measured (the
    /// browser adapter populates them on the first input event; until
    /// then the popup cannot be positioned and is suppressed).
    pub completion_popup: Option<CompletionPopupView>,
    /// Signature-help line rendered ABOVE the caret while the caret
    /// sits inside an open function call. `None` when:
    ///   * the editor document does not carry a `signature_help` from
    ///     the bridge,
    ///   * the document is stale (its `source_text` does not match
    ///     `raw_entered_cell_text`), or
    ///   * the completion popup is already `Open` at the same caret
    ///     (popup wins to avoid double-stacking; signature help
    ///     re-appears the moment the popup dismisses).
    pub signature_help: Option<SignatureHelpView>,
    /// Function-help card rendered as a hover tooltip on the matching
    /// function-token in the syntax overlay. `None` when:
    ///   * the editor document does not carry a `function_help` from
    ///     the bridge (no function context for the current caret), or
    ///   * the document is stale.
    /// Visibility is gated by component-local hover state — the
    /// view-model only carries the *content*; the actual tooltip is
    /// shown after a 400 ms hover over the matching `.syn-fn` span.
    pub function_help_card: Option<FunctionHelpCardView>,
    /// Developer/X-Ray capability context for the active formula
    /// and current workspace. User mode renders only summaries;
    /// Developer mode can render raw version strings and keys.
    pub capability_context: CapabilityContextView,
    /// First progressive-disclosure drill-down: the formula
    /// walk-tree panel rendered between the editor-foot and the
    /// result-caption when the user toggles it open with Ctrl+D.
    /// Always present (so the toggle row is rendered consistently
    /// whether the panel is open or closed); the `expanded` flag
    /// drives whether the panel body is visible.
    pub formula_drill: FormulaDrillView,
    /// Workspace-level reading-audience preference. Components
    /// branch their rendering based on this — User mode hides
    /// phase chips, state slugs, and SEAM markers; Developer mode
    /// surfaces them.
    pub view_mode: ViewMode,
    pub result_view: ResultView,
    pub status: StatusView,
    /// Titlebar scenario breadcrumb + dropdown projection. The
    /// breadcrumb is always rendered when there is an active
    /// formula space; the dropdown's `is_open` flag drives
    /// visibility of the menu body.
    pub scenario_breadcrumb: ScenarioBreadcrumbView,
    /// Tab strip surfacing every formula in
    /// `workspace_shell.open_formula_space_order`. WS-14 §1's
    /// minimum-viable surface: one chip per open formula with a
    /// click-to-switch + close affordance. The active chip is
    /// styled distinctly. Empty `chips` vec hides the strip
    /// entirely (no need to render chrome when only one formula
    /// is open and the breadcrumb already names it).
    pub formula_tab_strip: FormulaTabStripView,
    /// Command-palette overlay projection. `is_open == false`
    /// means the palette is hidden and the renderer skips it
    /// entirely; `true` carries the filter query + filtered
    /// command list + selected index for the active match. Per
    /// `SEAM-ONECALC-COMMAND-PALETTE`, the palette aggregates
    /// scenario actions, recent / pinned formulas, workspace
    /// settings, and a future function-reference lookup into one
    /// keyboard-driven launcher.
    pub command_palette: CommandPaletteView,
    /// Slice 5 — formatting controls row rendered under the
    /// result section. Mirrors the active formula's
    /// `FormulaFormattingState`; the renderer's `on:input` handlers
    /// dispatch the matching reducer setters.
    pub formatting_controls: FormattingControlsView,
    pub vba_host_context: VbaHostContextView,
    /// Manage-formulas overlay projection. `is_open == false`
    /// hides the overlay entirely; `true` carries the search
    /// query + filtered row list + per-row metadata so the user
    /// can browse / search / rename / pin / clone / close /
    /// forget every formula in one place.
    pub manage_formulas: ManageFormulasView,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityContextView {
    pub snapshot: CapabilityLedgerSnapshot,
    pub function_profiles: Vec<FunctionSemanticProfileRow>,
    pub value_capability_facts: Vec<ValueCapabilityFact>,
    pub formula_inputs: Vec<FormulaInputBindingView>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueCapabilityFact {
    pub fact_kind: ValueCapabilityFactKind,
    pub key: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueCapabilityFactKind {
    ProducerCanProvide,
    ExercisedThisRun,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaInputBindingView {
    pub label: String,
    pub reference_descriptor: String,
    pub reference_handle: Option<String>,
    pub value_preview: String,
}

/// Live-edited formatting fields for the active formula. Pure
/// projection of `FormulaFormattingState` plus a small set of
/// preset chips the UI offers as quick-buttons for the number
/// format code (matches the WS-14 mockup's `code presets` row).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormattingControlsView {
    pub number_format_code: String,
    pub font_color: String,
    pub fill_color: String,
    pub date1904: bool,
    pub number_format_presets: Vec<NumberFormatPreset>,
    /// Collapsible state from `FormulaSpaceState.formatting_panel_open`.
    /// When `false`, the renderer surfaces the one-line summary chip
    /// and hides the full controls row; when `true`, the full row is
    /// visible. Toggled via the summary-chip button (and by Ctrl+,
    /// once the keyboard wiring lands).
    pub is_open: bool,
    /// One-line summary rendered inside the collapsed-state chip.
    /// Reads e.g. `"format ▸ Currency · Date1904"` when both fields
    /// are set, or `"format ▸ General"` when nothing is overridden.
    /// Built deterministically so the summary string round-trips
    /// through pin-tests.
    pub summary: String,
    /// Calc-options scenario policy: Deterministic (stable seeds for
    /// `=NOW()` / `=RAND()`) or LiveRecalc (fresh values per
    /// keystroke). The renderer surfaces this as a two-state segmented
    /// control inside the formatting panel.
    pub scenario_policy: ScenarioPolicyView,
    /// Conditional-formatting rules (one row per rule). The renderer
    /// shows a "+ add rule" affordance plus per-rule editor cards.
    pub conditional_formatting_rules: Vec<ConditionalFormattingRuleView>,
    /// Workspace locale display, lifted from the active
    /// `AmbientAppContext`. Currently a heuristic label derived from
    /// the date-format-code shape; replaced by a `LocaleProfileId`
    /// label once OxFml's locale tables land.
    pub locale_label: String,
    /// BCP-47 language tag of the active locale preset (the one the
    /// user picked or the platform-detected default). Used to drive
    /// the dropdown's "currently-selected" indicator.
    pub locale_language_tag: String,
    /// Curated list of selectable locale presets surfaced in the
    /// formatting-panel dropdown. `(language_tag, label)` pairs.
    pub locale_presets: Vec<(&'static str, &'static str)>,
    /// Optional SEAM id surfaced next to the locale picker. The
    /// runtime locale chain landed (OxFunc W094 +
    /// `live_bridge::build_runtime_locale_context`), so the default
    /// for the standard preset list is `None`. The field is kept so
    /// hosts can still flag *additional* locale-related gaps
    /// (e.g. an as-yet-unmapped Excel locale id) without code
    /// changes here.
    pub locale_seam_id: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VbaHostContextView {
    pub pending_project_path: String,
    pub associations: Vec<VbaHostAssociationView>,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VbaHostAssociationView {
    pub association_id: String,
    pub display_name: String,
    pub source_ref: String,
    pub source_kind: String,
    pub enabled: bool,
    pub status_label: String,
    pub admitted_udf_count: usize,
    pub rejected_candidate_count: usize,
    pub admitted_udfs: Vec<String>,
    pub rejected_candidates: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumberFormatPreset {
    /// Human-readable family label rendered on the preset chip.
    pub label: &'static str,
    /// Excel format-string-text the chip applies when clicked.
    /// Empty string means the General (default) family.
    pub format_code: &'static str,
    /// Optional SEAM id when the family's *renderer* is not yet
    /// implemented in OxFml. The chip is still clickable — clicking
    /// stores the right format intent — but the result hero falls
    /// back to OxFml's hint-default rendering until the engine work
    /// lands. The renderer surfaces this with a small `<NOT IMPL>`
    /// badge and an `aria-describedby` referencing the seam.
    pub seam_id: Option<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScenarioPolicyView {
    Deterministic,
    LiveRecalc,
    /// Runtime evaluation gated on Calculate / F9. Mirrors
    /// `ScenarioPolicy::ManualRecalc`. Surfaced as a third
    /// segmented-control button alongside Deterministic / Live.
    ManualRecalc,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionalFormattingRuleView {
    pub rule_kind: String,
    pub operator: Option<String>,
    pub thresholds: Vec<String>,
    pub font_color: Option<String>,
    pub fill_color: Option<String>,
    /// Optional typed CF rule payload mirrored from the host state.
    /// When present the per-kind sub-form authors directly into this
    /// shape; the bounded-string `thresholds` continues to ride along
    /// as the W072 fallback. `None` means the rule is authored only
    /// through the bounded payload.
    pub typed_rule: Option<crate::state::FormulaConditionalFormattingTypedRule>,
    /// Optional SEAM id for rule kinds OxFml has not yet evaluated
    /// (color scales, data bars, icon sets, rank, average, unique,
    /// text, dates, blanks, errors). `None` means OxFml supports
    /// this kind today (cell_value comparisons + threshold).
    pub seam_id: Option<&'static str>,
}

impl FormattingControlsView {
    pub fn from_state(
        formatting: &crate::state::FormulaFormattingState,
        is_open: bool,
        ambient: &crate::state::AmbientAppContext,
    ) -> Self {
        let presets = number_format_presets();
        let summary = build_formatting_summary(formatting, &presets);
        let scenario_policy = match formatting.scenario_policy {
            crate::persistence::ScenarioPolicy::Deterministic => ScenarioPolicyView::Deterministic,
            crate::persistence::ScenarioPolicy::LiveRecalc => ScenarioPolicyView::LiveRecalc,
            crate::persistence::ScenarioPolicy::ManualRecalc => ScenarioPolicyView::ManualRecalc,
        };
        let conditional_formatting_rules = formatting
            .conditional_formatting_rules
            .iter()
            .map(|rule| ConditionalFormattingRuleView {
                rule_kind: rule.rule_kind.clone(),
                operator: rule.operator.clone(),
                thresholds: rule.thresholds.clone(),
                font_color: rule.font_color.clone(),
                fill_color: rule.fill_color.clone(),
                typed_rule: rule.typed_rule.clone(),
                seam_id: cf_seam_id_for_kind(&rule.rule_kind),
            })
            .collect();
        Self {
            number_format_code: formatting.number_format_code.clone(),
            font_color: formatting.font_color.clone(),
            fill_color: formatting.fill_color.clone(),
            date1904: formatting.date1904,
            number_format_presets: presets,
            is_open,
            summary,
            scenario_policy,
            conditional_formatting_rules,
            locale_label: ambient_locale_label(ambient),
            locale_language_tag: ambient.language_tag.clone(),
            locale_presets: crate::services::ambient_app_context::supported_locale_presets()
                .to_vec(),
            // Locale chain is live: OxFunc W094 ships
            // `LocaleProfileId::from_bcp47_language_tag`, OxFml exposes
            // `oxfml_locale_context`, and `live_bridge` plumbs the
            // workspace's selected tag through every edit. No SEAM
            // shim required for the curated preset list any more.
            locale_seam_id: None,
        }
    }
}

/// The full format-family preset row exposed in the panel.
///
/// Coverage tracks OxFml's published format space after W069:
/// - **General / Number / Currency / Percent / Scientific** —
///   `oxfml_core::format::number::render_with_number_format_code`.
/// - **Date / Date (long)** — `oxfml_core::format::datetime`.
/// - **Time / Time (12h) / Datetime** — W069 time tokens
///   (`h`/`hh`/`m`/`mm`/`s`/`ss`/`AM/PM`/datetime composites).
/// - **Fraction** — W069 fraction grammar (`?/?` / `??/??` /
///   `# ?/?` / `# ??/??` / `0/0`).
/// - **Accounting** — W069 common accounting parens.
/// - **Text** — `@` placeholder rendering.
///
/// All preset chips are now live; SEAM markers retired. Note that
/// the live family coverage is "what OxFml renders correctly when
/// the user types these codes" — the `applied colour` and
/// locale-prefix sub-grammar remain pending behind
/// `docs/HANDOFF_OXFML_CUSTOM_FORMAT_GRAMMAR.md`, but those are
/// only reachable via custom-typed codes, not via these presets.
fn number_format_presets() -> Vec<NumberFormatPreset> {
    vec![
        NumberFormatPreset {
            label: "General",
            format_code: "",
            seam_id: None,
        },
        NumberFormatPreset {
            label: "Number",
            format_code: "0.00",
            seam_id: None,
        },
        NumberFormatPreset {
            label: "Number (with separators)",
            format_code: "#,##0.00",
            seam_id: None,
        },
        NumberFormatPreset {
            label: "Currency",
            format_code: "$#,##0.00",
            seam_id: None,
        },
        NumberFormatPreset {
            label: "Accounting",
            format_code: "_($* #,##0.00_);_($* (#,##0.00);_($* \"-\"??_);_(@_)",
            seam_id: None,
        },
        NumberFormatPreset {
            label: "Percent",
            format_code: "0.00%",
            seam_id: None,
        },
        NumberFormatPreset {
            label: "Fraction",
            format_code: "# ?/?",
            seam_id: None,
        },
        NumberFormatPreset {
            label: "Scientific",
            format_code: "0.00E+00",
            seam_id: None,
        },
        NumberFormatPreset {
            label: "Date",
            format_code: "yyyy-mm-dd",
            seam_id: None,
        },
        NumberFormatPreset {
            label: "Date (long)",
            format_code: "dddd, mmmm d, yyyy",
            seam_id: None,
        },
        NumberFormatPreset {
            label: "Time",
            format_code: "HH:mm:ss",
            seam_id: None,
        },
        NumberFormatPreset {
            label: "Time (12h)",
            format_code: "h:mm:ss AM/PM",
            seam_id: None,
        },
        NumberFormatPreset {
            label: "Datetime",
            format_code: "yyyy-mm-dd HH:mm:ss",
            seam_id: None,
        },
        NumberFormatPreset {
            label: "Text",
            format_code: "@",
            seam_id: None,
        },
    ]
}

/// Map a CF rule_kind string to its SEAM id. Returns `None` when
/// OxFml's `evaluate_conditional_formatting_rule` already handles
/// that kind today.
///
/// Live after W070 / W071 / W072:
///
/// **Per-cell (operator / predicate) rules:**
/// - `cell_value` — operator-driven comparisons (greaterThan,
///   greaterThanOrEqual, lessThan, lessThanOrEqual, equal,
///   notEqual, between, notBetween).
/// - `text` — operator-driven text predicates (containsText,
///   notContainsText, beginsWith, endsWith).
/// - `expression` — user-supplied formula evaluated as boolean.
/// - `dates` — relative-date predicates evaluated against the
///   bridge's `now_serial` (today / yesterday / tomorrow /
///   last7Days / thisWeek / lastWeek / nextWeek / thisMonth /
///   lastMonth / nextMonth).
/// - `blanks` / `noBlanks` — blank-value predicates.
/// - `errors` / `noErrors` — error-value predicates.
///
/// **Aggregate (array-as-range) typed-payload kinds (W073, only
/// `typed_rule` accepted upstream; bounded `thresholds` ignored):**
/// - `aboveAverage` / `belowAverage` — `typed_rule.average`
///   (`include_equal`, optional `stddev_multiplier`).
/// - `top` / `bottom` — `typed_rule.rank`
///   (`Count(usize)` or `Percent(f64)`).
/// - `colorScale` — `typed_rule.color_scale.stops`
///   (ordered `(position, color)` pairs).
/// - `dataBar` — `typed_rule.data_bar`
///   (`bar_color`, optional `minimum`/`maximum`, `direction`,
///   `show_bar_only`).
/// - `iconSet` — `typed_rule.icon_set`
///   (`set_kind` plus per-icon thresholds).
///
/// **Aggregate predicate kinds (no typed payload, kind-only):**
/// - `uniqueValues` / `duplicateValues` — count occurrences
///   within the array; the kind itself is the predicate.
///
/// Unknown kinds get a SEAM marker as a defensive fallback —
/// scenarios saved against an older host version may carry a
/// rule_kind we no longer expose, and the marker makes the
/// "this rule won't fire" state visible rather than silent.
fn cf_seam_id_for_kind(rule_kind: &str) -> Option<&'static str> {
    match rule_kind.to_ascii_lowercase().as_str() {
        "cell_value" | "" | "text" | "expression" => None,
        "dates" | "blanks" | "noblanks" | "errors" | "noerrors" => None,
        "colorscale" | "databar" | "iconset" | "aboveaverage" | "belowaverage" | "top"
        | "bottom" | "uniquevalues" | "duplicatevalues" => None,
        _ => Some("SEAM-ONECALC-CF-UNKNOWN-RULE-KIND"),
    }
}

/// Build the one-line summary that the collapsed formatting chip
/// shows. When nothing is overridden the label is `"General"`.
/// Otherwise, list the overrides in a stable order: number-format
/// (preset label when matched, else the raw code), font colour,
/// fill colour, and the 1904-dates flag. The output is plain text
/// — formatting is the renderer's job — and is prefixed by the
/// renderer with `"format ▸ "` as a caption.
fn build_formatting_summary(
    formatting: &crate::state::FormulaFormattingState,
    presets: &[NumberFormatPreset],
) -> String {
    let mut parts: Vec<String> = Vec::new();
    let format_code = formatting.number_format_code.trim();
    if format_code.is_empty() {
        parts.push("General".to_string());
    } else if let Some(preset) = presets
        .iter()
        .find(|p| !p.format_code.is_empty() && p.format_code == format_code)
    {
        parts.push(preset.label.to_string());
    } else {
        parts.push(format_code.to_string());
    }
    let font = formatting.font_color.trim();
    if !font.is_empty() {
        parts.push(format!("font {}", font));
    }
    let fill = formatting.fill_color.trim();
    if !fill.is_empty() {
        parts.push(format!("fill {}", fill));
    }
    if formatting.date1904 {
        parts.push("Date1904".to_string());
    }
    if formatting.scenario_policy == crate::persistence::ScenarioPolicy::LiveRecalc {
        parts.push("Live".to_string());
    }
    let cf_count = formatting.conditional_formatting_rules.len();
    if cf_count > 0 {
        parts.push(format!(
            "{} CF rule{}",
            cf_count,
            if cf_count == 1 { "" } else { "s" }
        ));
    }
    parts.join(" · ")
}

/// Command-palette overlay projection. Closed by default; opens
/// via Ctrl+K / Ctrl+Shift+P. Each command has a stable
/// identifier the renderer dispatches on click / Enter. The
/// palette is a single ranked list (sections are flattened) so
/// keyboard navigation stays linear; section labels render as
/// non-selectable separators.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandPaletteView {
    pub is_open: bool,
    pub query: String,
    /// Filtered + ranked command list. Empty when nothing
    /// matches the query.
    pub commands: Vec<CommandPaletteEntry>,
    /// Index into `commands` that is currently highlighted. `0`
    /// when the command list is empty (no-op on Enter).
    pub selected_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandPaletteEntry {
    pub kind: CommandPaletteEntryKind,
    /// Human-readable label rendered as the row's main text.
    pub label: String,
    /// Section label (`"Formulas"`, `"Actions"`, …) for the
    /// renderer to group rows visually. The view-model already
    /// flattens commands into a single list — section is just
    /// metadata.
    pub section: &'static str,
    /// Optional secondary line rendered below the label
    /// (formula path, scenario name, etc.).
    pub detail: Option<String>,
    /// Optional keyboard chord rendered to the right of the row.
    pub chord: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandPaletteEntryKind {
    /// Run a scenario action by id (e.g. `NewScenario`,
    /// `Duplicate`, `PinActive`). Maps onto the same dispatcher
    /// the breadcrumb dropdown uses.
    ScenarioAction(ScenarioBreadcrumbActionId),
    /// Switch to an open / pinned / recent formula by id.
    SwitchFormula(String),
    /// Toggle the formatting-panel collapse state on the active
    /// formula.
    ToggleFormattingPanel,
    /// Toggle the formula drill-down on the active formula.
    ToggleFormulaDrill,
    /// Force a runtime recalc on the active formula (F9 alias).
    ForceRecalc,
}

/// Tab strip projection for the open-formulas surface (WS-14 §1).
/// Every entry in `workspace_shell.open_formula_space_order` becomes
/// a chip; the active formula's chip is highlighted via
/// `is_active`. The strip itself collapses (`is_visible == false`)
/// when only one formula is open — the breadcrumb already names it,
/// and the extra row of chrome would be visual noise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaTabStripView {
    pub is_visible: bool,
    pub chips: Vec<FormulaTabChip>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaTabChip {
    pub formula_space_id: String,
    pub display_name: String,
    pub is_active: bool,
    pub is_pinned: bool,
    /// `true` when the formula has uncommitted changes (raw text
    /// differs from `committed_cell_text`). Surfaced as a small
    /// dirty marker in the chip; close-with-dirty uses the same
    /// signal to decide whether to confirm.
    pub is_dirty: bool,
    /// `true` when the user has begun an inline rename on this
    /// chip. The renderer swaps the chip-name `<span>` for a
    /// text input bound to `pending_rename_text`. Driven by
    /// `WorkspaceShellState.renaming_formula_space_id`.
    pub is_renaming: bool,
    /// Buffered rename text — the value shown in the inline
    /// input while `is_renaming == true`. Empty when the chip
    /// is not being renamed.
    pub rename_buffer: String,
}

/// Manage-formulas overlay projection. The overlay is the
/// answer to "I have 30 saved formulas, find the one with
/// `=XLOOKUP` in it" — a searchable list of every formula in
/// the workspace (open + recent + pinned, deduped) with per-row
/// quick actions.
///
/// Projected from `WorkspaceShellState.open_formula_space_order`
/// + `recent_formula_space_order` + `pinned_formula_space_ids` +
/// `recent_formula_spaces` + `formula_spaces`. Rows are
/// pre-filtered against
/// `global_ui_chrome.manage_formulas_search_query`; the search
/// matches against display name AND raw entered text
/// (case-insensitive substring), so a search for `xlookup`
/// finds every formula that contains `=XLOOKUP(...)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManageFormulasView {
    /// `false` hides the overlay entirely; the renderer skips it.
    pub is_open: bool,
    /// Current search query. Echoed in the overlay's input.
    pub search_query: String,
    /// Total formula count BEFORE filtering, surfaced in the
    /// overlay's title row ("Manage formulas · 12").
    pub total_count: usize,
    /// Filtered + sorted rows: pinned first (in pin order), then
    /// open (in open order), then recent (most-recent first). A
    /// formula is shown at most once even if it appears in
    /// multiple lists. Empty list when nothing matches the search.
    pub rows: Vec<ManageFormulasRow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManageFormulasRow {
    pub formula_space_id: String,
    pub display_name: String,
    /// First ~80 chars of the formula's raw entered text, with
    /// newlines collapsed to spaces. Empty when the formula has
    /// no text (a fresh untitled). Surfaced as a muted preview
    /// under the display name.
    pub formula_preview: String,
    pub is_pinned: bool,
    /// `true` when this formula is currently in
    /// `open_formula_space_order` (so it has a tab chip / can be
    /// switched to without reopening). `false` means it's a
    /// recent (the row's "Open" action has to reopen it from the
    /// `ClosedFormulaSpaceRecord`).
    pub is_open: bool,
    /// `true` when this formula is the workspace's currently
    /// active one. The row gets a subtle highlight + the "Open"
    /// action reads "Active" instead.
    pub is_active: bool,
    /// `true` when raw text differs from `committed_cell_text`.
    pub is_dirty: bool,
}

/// View-model shape for the titlebar scenario breadcrumb + its
/// dropdown menu. Built from the active formula space (for the
/// label + dirty marker) and from `WorkspaceShellState` for the
/// Recent / Pinned lists. Actions are a fixed shape — handlers
/// are wired in the renderer.
///
/// Phase A: the dropdown surfaces the data already living in
/// state. File I/O for `.dnascenario` is out of scope here and
/// will land behind `SEAM-ONECALC-SCENARIO-PERSIST`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioBreadcrumbView {
    /// Human-readable breadcrumb label. `"unsaved"` when the
    /// scenario has not been named (the default `scenario_label`
    /// equals the synthetic `formula_space_id.as_str()`); the
    /// stored label otherwise. Renderer puts this inside the
    /// breadcrumb button next to the dropdown caret.
    pub active_label: String,
    /// True when the live editor text has uncommitted changes
    /// (live state is `EditingLive`). Renderer surfaces this
    /// with a small dot before the label per the WS-14 mockup
    /// (`data-dirty="true"` selector).
    pub is_dirty: bool,
    /// Lifecycle for the dropdown menu. Mirrors
    /// `global_ui_chrome.scenario_breadcrumb_open`.
    pub is_open: bool,
    /// Top-N most recently active formula spaces. Includes the
    /// active formula (first entry) followed by the workspace
    /// shell's `recent_formula_space_order`, deduplicated. Empty
    /// when there is no active space and no recents.
    pub recent: Vec<ScenarioBreadcrumbEntry>,
    /// Pinned formula spaces, in stable id order from the
    /// underlying `BTreeSet`. Empty when no pins exist.
    pub pinned: Vec<ScenarioBreadcrumbEntry>,
    /// Fixed action list rendered under the dropdown's
    /// `Actions` heading. Each action has a stable identifier
    /// the renderer dispatches on.
    pub actions: Vec<ScenarioBreadcrumbAction>,
}

/// One row in the Recent or Pinned section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioBreadcrumbEntry {
    pub formula_space_id: String,
    pub display_name: String,
    /// One-line meta string rendered to the right of the name.
    /// Today: `"active"` for the active row, `"pinned"` for
    /// pinned rows, otherwise the recent entry's last-active
    /// mode label. Future bead: timestamp once persistence
    /// lands.
    pub meta: String,
    pub is_active: bool,
    pub is_pinned: bool,
}

/// Stable identifiers for the dropdown's Actions section. The
/// renderer maps each to a click handler; today most are no-ops
/// gated behind `SEAM-ONECALC-SCENARIO-PERSIST`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScenarioBreadcrumbActionId {
    NewScenario,
    SaveAs,
    Open,
    /// Renamed from "Duplicate" in the renderer — the user-visible
    /// label is "Clone" per WS-14 §1's "Clone vs. duplicate vs.
    /// save-as" decision. The action id keeps the legacy name for
    /// stability in tests and persisted shells.
    Duplicate,
    /// Begin an inline rename of the active formula's display
    /// label. Surfaces the same input the tab-strip's
    /// double-click-to-rename path uses; the breadcrumb is the
    /// keyboard-discoverable entry point. Available for any
    /// active formula — pinned, unpinned, or unsaved.
    RenameActive,
    /// Pin the active formula. Only surfaced when the active
    /// formula is *not* already pinned.
    PinActive,
    /// Unpin the active formula. Only surfaced when the active
    /// formula *is* pinned.
    UnpinActive,
    ManageScenarios,
}

impl ScenarioBreadcrumbActionId {
    pub fn slug(self) -> &'static str {
        match self {
            Self::NewScenario => "new-scenario",
            Self::SaveAs => "save-as",
            Self::Open => "open",
            Self::Duplicate => "duplicate",
            Self::RenameActive => "rename-active",
            Self::PinActive => "pin-active",
            Self::UnpinActive => "unpin-active",
            Self::ManageScenarios => "manage-scenarios",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioBreadcrumbAction {
    pub action_id: ScenarioBreadcrumbActionId,
    pub label: &'static str,
    /// Keyboard chord rendered to the right of the action label
    /// in the dropdown. Empty string when no chord is bound today.
    pub chord_label: &'static str,
    /// Optional SEAM identifier surfaced in the renderer's
    /// `aria-describedby` so unimplemented actions are honest
    /// about the missing backend. `None` for actions that work
    /// fully today (just opening the dropdown / etc.).
    pub seam_id: Option<&'static str>,
}

/// View-model shape for the formula walk-tree drill-down. Always
/// emitted — the toggle row in the editor-foot needs to render
/// regardless of expansion state, so the `expanded` flag drives
/// visibility of the panel body and the chevron rotation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaDrillView {
    pub expanded: bool,
    /// Walk tree from `editor_document.formula_walk`, projected
    /// into nested render-friendly nodes (each node carries its
    /// own children — the renderer uses `<details>` to collapse
    /// per-node, mirroring how a user reads the tree). Empty
    /// when the document is missing or stale.
    pub tree: Vec<FormulaDrillNode>,
    /// Live diagnostics surfaced from `editor_document.live_diagnostics`.
    /// Empty when the formula is empty (the host suppresses the
    /// upstream `unexpected token Eof` for empty input) or when
    /// no diagnostics fired. Renders as a list inside the
    /// drill-down panel so the user can read full diagnostic
    /// messages — the editor-foot metrics chip just shows the
    /// count.
    pub diagnostics: Vec<FormulaDrillDiagnosticRow>,
    /// Bottom-strip phase chips: `parse: <status> · bind: <vars>
    /// vars · eval: <steps> steps`. Pulled from the document's
    /// parse_summary / bind_summary / eval_summary fields. Empty
    /// when the document is missing or stale.
    pub phase_summaries: Vec<FormulaDrillPhaseChip>,
    /// True iff the document is present and matches
    /// `raw_entered_cell_text`. The component shows a "(loading)"
    /// indicator when `expanded` is true but `document_is_fresh`
    /// is false — gives the user feedback during the brief stale
    /// window between keystroke and bridge round-trip.
    pub document_is_fresh: bool,
}

/// One row in the formula walk-tree panel. Mirrors
/// [`crate::adapters::oxfml::FormulaDrillNodeViewModel`] with nested
/// children preserved so the renderer can use `<details>`
/// elements for click-to-collapse per-node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaDrillNode {
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
    pub array_preview: Option<crate::adapters::oxfml::FormulaDrillArrayPreview>,
    pub state: crate::adapters::oxfml::FormulaDrillNodeState,
    pub children: Vec<FormulaDrillNode>,
}

/// One diagnostic surfaced inside the drill-down panel. Mirrors
/// the editor's `LiveDiagnostic` with the fields the user wants
/// at a glance: severity slug, message text, and the formula
/// span it points at (so a click could later scroll the editor
/// to that span — wired separately when the click handler lands).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaDrillDiagnosticRow {
    pub diagnostic_id: String,
    pub severity: SquiggleSeverity,
    pub stage: DiagnosticStage,
    pub code: Option<String>,
    pub message: String,
    pub span_start: usize,
    pub span_len: usize,
}

/// Phase-strip chip. Renders `label · detail` with a state
/// attribute (`ok` / `pending` / `blocked`) so the corpus can
/// pin the colour and content separately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaDrillPhaseChip {
    pub label: &'static str,
    pub detail: String,
    pub state: FormulaDrillPhaseState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormulaDrillPhaseState {
    Ok,
    Pending,
    Blocked,
}

impl FormulaDrillPhaseState {
    pub fn slug(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Pending => "pending",
            Self::Blocked => "blocked",
        }
    }
}

/// View-model shape for the function-help hover tooltip.
///
/// Sourced from `editor_document.function_help: FunctionHelpPacket`.
/// The bridge populates this packet for the caret-adjacent function;
/// hover help on an arbitrary token in the formula would require a
/// separate bridge call and is deferred to a future bead. For now,
/// the hover only fires when the user hovers over a function token
/// whose name matches `lookup_key` (case-insensitive).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionHelpCardView {
    /// The bridge's lookup key for the function (uppercase canonical
    /// name, e.g. "SUM"). The renderer uses this to gate which
    /// `.syn-fn` span can trigger the tooltip.
    pub lookup_key: String,
    /// Display name for the heading line (typically the same as
    /// `lookup_key` but may differ for localised function catalogues).
    pub display_name: String,
    /// First signature form's `display_signature`, e.g.
    /// `SUM(number1, number2, ...)`. Multi-form / overload navigation
    /// is a future bead — first form only here.
    pub signature: Option<String>,
    /// One-line description from the function-help packet. Optional
    /// because not every catalogue entry carries a description.
    pub short_description: Option<String>,
    /// Availability summary from the packet, surfaced when present so
    /// users can see why a deferred / profile-limited function might
    /// not be evaluating.
    pub availability_summary: Option<String>,
    /// True when the function-help packet flags the function as
    /// deferred or restricted by the active capability profile. The
    /// renderer styles this state so the user knows the help is for
    /// a function that won't fully evaluate today.
    pub deferred_or_profile_limited: bool,
}

/// View-model shape for the signature-help line. Mirrors the
/// completion-popup geometry primitives so the renderer uses one
/// shared positioning convention; the difference is purely that
/// the help line sits ABOVE the caret instead of below.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureHelpView {
    /// Display text of the function being called (e.g. `SUM`). Comes
    /// straight from the upstream `SignatureHelpContext`.
    pub callee_text: String,
    /// Pixel anchor of the caret-box top-left, relative to the editor
    /// frame's origin. The renderer offsets `top_px` UPWARD by the
    /// signature-help line's own height so the help sits above the
    /// line the caret occupies, not on top of it.
    pub anchor_left_px: usize,
    pub anchor_top_px: usize,
    /// Caret line height in pixels — used for the BELOW-caret fallback
    /// when the help line would clip the top of the editor frame.
    pub line_height_px: usize,
    /// Parameter list rendered with the active argument bolded. Built
    /// from `function_help.argument_help`; an empty vec is rendered
    /// as just the callee name with bare parens.
    pub parameters: Vec<SignatureHelpParameter>,
    /// Active-parameter index, clamped to `parameters.len()`. `None`
    /// when the bridge's index is out of range (caret is past the
    /// last parameter, e.g. one extra trailing comma) — the renderer
    /// shows the parameter list with no bolded entry in that case.
    pub active_parameter: Option<usize>,
}

/// One parameter in the signature-help line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureHelpParameter {
    pub name: String,
    pub is_active: bool,
}

/// View-model shape for the completion popup. Carries the anchor pixel
/// position computed from the bridge's caret offset + the browser-
/// measured char-box metrics, plus the rendered item list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionPopupView {
    /// Pixel anchor of the popup's top-left corner, relative to the
    /// editor frame's origin. The popup itself sits below the line the
    /// caret occupies, so the renderer offsets `top_px` by
    /// `line_height_px` when positioning.
    pub anchor_left_px: usize,
    pub anchor_top_px: usize,
    pub line_height_px: usize,
    pub items: Vec<CompletionPopupItemView>,
    /// Index into `items` of the row to highlight. Always in
    /// `0..items.len()`.
    pub selected_index: usize,
}

/// One row of the popup. Carries the rendering payload (display text,
/// kind glyph, is_selected) and the proposal id so click handlers can
/// look up the original proposal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionPopupItemView {
    pub proposal_id: String,
    pub display_text: String,
    pub kind_glyph: char,
    pub kind_label: &'static str,
    pub is_selected: bool,
    pub documentation_ref: Option<String>,
}

impl CompletionPopupItemView {
    fn glyph_for_kind(kind: CompletionPopupKind) -> char {
        match kind {
            CompletionPopupKind::Function => 'ƒ',
            CompletionPopupKind::DefinedName => 'N',
            CompletionPopupKind::TableName => 'T',
            CompletionPopupKind::TableColumn => '⫶',
            CompletionPopupKind::StructuredSelector => '#',
            CompletionPopupKind::SyntaxAssist => '·',
            CompletionPopupKind::ProfileReference => 'R',
        }
    }

    fn label_for_kind(kind: CompletionPopupKind) -> &'static str {
        match kind {
            CompletionPopupKind::Function => "Function",
            CompletionPopupKind::DefinedName => "Defined name",
            CompletionPopupKind::TableName => "Table",
            CompletionPopupKind::TableColumn => "Column",
            CompletionPopupKind::StructuredSelector => "Selector",
            CompletionPopupKind::SyntaxAssist => "Syntax",
            CompletionPopupKind::ProfileReference => "Reference",
        }
    }
}

/// Editor-foot live-metrics chip projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorMetricsChip {
    pub token_count: usize,
    pub function_count: usize,
    pub diagnostic_count: usize,
    /// First diagnostic's user-facing message, when one exists.
    /// Surfaced on the User-mode editor-foot chip so the user sees
    /// "1 issue: unmatched '('" instead of the developer-mode raw
    /// counts. None when `diagnostic_count == 0`.
    pub first_diagnostic_message: Option<String>,
}

/// Result-foot active-context chip projection. Each field carries either a
/// live value or a SEAM-pending placeholder; the renderer composes the
/// dot-separated string `locale · format · policy`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultContextChip {
    /// Active format-family label for the current formula.
    /// `Live("General")` when no format code is set; `Live(family)`
    /// for a matched preset; `Live("Custom · <code>")` for a user-
    /// typed code that doesn't match a preset.
    pub format: ContextChipField,
    /// `live-recalc` or `deterministic`, lifted from the formula's
    /// `formatting.scenario_policy`. Always `Live(...)`; defaults
    /// to `live-recalc` (Excel's default).
    pub policy: ContextChipField,
}

/// One field inside the result-context chip. SEAM-pending fields carry the
/// canonical SEAM id from WS-14 plan §11 so the renderer can attach
/// `data-seam-id` and `aria-describedby` and tooltips can surface it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContextChipField {
    Live(String),
    SeamPending { value: String, seam_id: String },
}

impl ContextChipField {
    pub fn value(&self) -> &str {
        match self {
            Self::Live(value) => value.as_str(),
            Self::SeamPending { value, .. } => value.as_str(),
        }
    }

    pub fn seam_id(&self) -> Option<&str> {
        match self {
            Self::Live(_) => None,
            Self::SeamPending { seam_id, .. } => Some(seam_id.as_str()),
        }
    }
}

/// A single underline overlay positioned at a diagnostic's source span.
/// Carries enough information to render the squiggle, drive its colour by
/// severity, and supply a hover tooltip via `title` attribute.
///
/// Per OxFml W067 (`docs/upstream/NOTES_FOR_DNAONECALC.md` §10), this
/// view-model passes through three additional upstream-owned fields:
///
/// * `code` — stable classification key (`unknown_function`,
///   `unknown_name`, `function_arity_mismatch`,
///   `known_symbol_not_callable`, `function_gated_or_unavailable`,
///   etc.). Used by the test corpus and by the eventual UI grouping
///   surface; never inferred host-side.
/// * `stage` — `Syntax` / `Bind` / `SemanticPlan`, the upstream phase
///   that emitted the diagnostic. Surfaced as a data attribute so the
///   corpus and X-Ray can read it without collapsing the distinction.
/// * `worksheet_error_class` — the worksheet-visible error consequence
///   when OxFml already knows it (e.g. `"#NAME?"`). Lets the UI render
///   the squiggle alongside the same error glyph Excel would show; the
///   host never derives it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticSquiggle {
    pub diagnostic_id: String,
    pub message: String,
    pub severity: SquiggleSeverity,
    pub stage: DiagnosticStage,
    pub code: Option<String>,
    pub worksheet_error_class: Option<String>,
    pub span_start: usize,
    pub span_len: usize,
}

/// Mirror of `LiveDiagnosticStage` lifted to the view-model layer so
/// the renderer doesn't depend on the upstream enum directly. Three
/// values: `Syntax`, `Bind`, `SemanticPlan`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticStage {
    Syntax,
    Bind,
    SemanticPlan,
}

impl DiagnosticStage {
    pub fn slug(self) -> &'static str {
        match self {
            Self::Syntax => "syntax",
            Self::Bind => "bind",
            Self::SemanticPlan => "semantic-plan",
        }
    }

    fn from_upstream(stage: crate::adapters::oxfml::LiveDiagnosticStage) -> Self {
        match stage {
            crate::adapters::oxfml::LiveDiagnosticStage::Syntax => Self::Syntax,
            crate::adapters::oxfml::LiveDiagnosticStage::Bind => Self::Bind,
            crate::adapters::oxfml::LiveDiagnosticStage::SemanticPlan => Self::SemanticPlan,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SquiggleSeverity {
    Error,
    Warning,
    Info,
}

impl SquiggleSeverity {
    pub fn slug(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
        }
    }

    fn from_upstream(severity: LiveDiagnosticSeverity) -> Self {
        match severity {
            LiveDiagnosticSeverity::Error => Self::Error,
            LiveDiagnosticSeverity::Warning => Self::Warning,
            LiveDiagnosticSeverity::Info => Self::Info,
        }
    }
}

/// Editor-caption pill mirroring `EditorEntryMode` but lifted into the
/// view-model so the component never re-classifies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryModePill {
    Formula,
    Value,
    Text,
    Empty,
}

impl EntryModePill {
    pub fn label(self) -> &'static str {
        match self {
            Self::Formula => "Formula",
            Self::Value => "Value",
            Self::Text => "Text",
            Self::Empty => "Empty",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            Self::Formula => "formula",
            Self::Value => "value",
            Self::Text => "text",
            Self::Empty => "empty",
        }
    }

    fn from_entry_mode(mode: EditorEntryMode) -> Self {
        match mode {
            EditorEntryMode::Formula => Self::Formula,
            EditorEntryMode::Value => Self::Value,
            EditorEntryMode::Text => Self::Text,
            EditorEntryMode::Empty => Self::Empty,
        }
    }
}

/// Result-caption pill labelling the value class shown in the result hero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultClassPill {
    Number,
    Text,
    Logical,
    Error,
    Array,
    Other,
}

impl ResultClassPill {
    pub fn label(self) -> &'static str {
        match self {
            Self::Number => "Number",
            Self::Text => "Text",
            Self::Logical => "Logical",
            Self::Error => "Error",
            Self::Array => "Array",
            Self::Other => "Other",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            Self::Number => "number",
            Self::Text => "text",
            Self::Logical => "logical",
            Self::Error => "error",
            Self::Array => "array",
            Self::Other => "other",
        }
    }

    fn from_result_view(view: &ResultView) -> Option<Self> {
        match view {
            ResultView::Empty | ResultView::Pending => None,
            ResultView::Error { .. } => Some(Self::Error),
            ResultView::Array { .. } => Some(Self::Array),
            ResultView::Display { kind, .. } => Some(match kind {
                ResultKind::Number => Self::Number,
                ResultKind::Text => Self::Text,
                ResultKind::Logical => Self::Logical,
                ResultKind::RichValue => Self::Other,
                ResultKind::Other => Self::Other,
            }),
        }
    }
}

/// What the result block should render. Mirrors the shape called out in the
/// path doc and matches the five mutually-exclusive UI states.
// `Eq` cannot be derived: `Array.cell_format` carries
// `DataBarFillView { fill_ratio: f64, ... }` and `f64` is not `Eq`.
#[derive(Debug, Clone, PartialEq)]
pub enum ResultView {
    /// Editor is empty — show a muted placeholder.
    Empty,
    /// Editor has text but no eval result yet — show "..." muted.
    Pending,
    /// A scalar / text / logical result we can render. The
    /// optional colour fields carry CF-applied font / fill colours
    /// from `verification_publication_surface.effective_*` —
    /// today populated only when a CF rule fires; will also carry
    /// custom-format colour-token output once the OxFml colour-
    /// token publication handoff lands.
    Display {
        text: String,
        kind: ResultKind,
        applied_font_color: Option<String>,
        applied_fill_color: Option<String>,
    },
    /// A diagnostic, blocked-lane, or error code state.
    Error {
        code: String,
        surface_repr: Option<String>,
    },
    /// An array result. `total_rows` / `total_cols` are the full
    /// shape from the upstream `CalcValue::Array`; `cells` is a
    /// (possibly-truncated) preview window the bridge ships back.
    /// `truncated` is `true` when either dimension was clamped —
    /// the renderer surfaces a `+N rows / +M cols hidden` chip in
    /// that case. The browser-grid component renders the cells
    /// with `overflow: auto; resize: both` so the user can scroll
    /// and resize the result panel.
    ///
    /// `cell_format` carries OxFml's per-cell CF outcomes (W071 +
    /// W072) when the formula has CF rules attached. Each entry
    /// has a font / fill colour, a data-bar fill, and / or an
    /// icon. `None` when no CF rules fired or the formula has
    /// none. The grid is row-major and indexed parallel to
    /// `cells` (a cell at `cells[r][c]` has its formatting at
    /// `cell_format[r][c]` when both are present).
    Array {
        total_rows: usize,
        total_cols: usize,
        label: String,
        cells: Vec<Vec<String>>,
        cell_format: Option<Vec<Vec<ArrayCellFormatView>>>,
        truncated: bool,
    },
}

/// Per-cell CF outcome lifted onto the view-model. Mirrors the
/// host adapter's `ArrayCellFormat` minus the `effective_display_text`
/// (which the renderer reads from `cells` directly when no CF rule
/// supplied a per-cell text override; when one did, the bridge
/// already substituted it into the string-based preview).
#[derive(Debug, Clone, PartialEq)]
pub struct ArrayCellFormatView {
    pub effective_font_color: Option<String>,
    pub effective_fill_color: Option<String>,
    pub data_bar: Option<DataBarFillView>,
    pub icon: Option<CfIconView>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DataBarFillView {
    pub fill_ratio: f64,
    pub bar_color: String,
    pub direction: DataBarDirectionView,
    pub show_bar_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataBarDirectionView {
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CfIconView {
    pub set_kind: String,
    pub icon_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultKind {
    Number,
    Text,
    Logical,
    RichValue,
    Other,
}

/// Status-foot projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusView {
    pub bridge_health: BridgeHealth,
    pub truth_source: ProjectionTruthSource,
    pub green_tree_key: Option<String>,
    /// Scenario label rendered in the status-foot's centre band
    /// (`scenario · <label>`). Same fallback rule as the
    /// breadcrumb: synthetic-default labels render as `"unsaved"`
    /// rather than leaking the synthetic id. Always populated when
    /// there is an active formula space; the renderer always shows
    /// the chip so the user always knows which scenario they are
    /// editing.
    pub scenario_label: String,
    /// Persistence-loader warnings for the active formula space.
    /// Empty when the file was loaded with full fidelity (or no
    /// file has been loaded). Slice 3 of the persistence ladder
    /// surfaces a warning chip while this is non-empty; the next
    /// save clears it.
    pub load_diagnostics: Vec<crate::persistence::LoadDiagnostic>,
}

/// Coarse bridge health for the status-foot dot. `Live` is sage; `Stale`
/// is amber.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeHealth {
    Live,
    Stale,
}

/// Resolve the active formula space and project its view-model. Returns
/// `None` when no formula space is active or the active id has no entry.
pub fn build_home_shell_view_model(state: &OneCalcHostState) -> Option<HomeShellViewModel> {
    let active_id = state.workspace_shell.active_formula_space_id.as_ref()?;
    let formula_space = state.formula_spaces.get(active_id)?;
    Some(project_formula_space(formula_space, state))
}

fn project_formula_space(
    formula_space: &FormulaSpaceState,
    state: &OneCalcHostState,
) -> HomeShellViewModel {
    let view_mode = state.view_mode;
    let result_view = project_result_view(formula_space);
    let entry_mode_pill = EntryModePill::from_entry_mode(EditorEntryMode::classify(
        &formula_space.raw_entered_cell_text,
    ));
    let result_class_pill = ResultClassPill::from_result_view(&result_view);
    let syntax_runs = project_syntax_runs(formula_space);
    let diagnostic_squiggles = project_diagnostic_squiggles(formula_space);
    let editor_metrics = project_editor_metrics(formula_space, &syntax_runs);
    let result_context = project_result_context(formula_space, &state.ambient_app_context);
    let completion_popup = project_completion_popup(formula_space);
    let signature_help = project_signature_help(formula_space, completion_popup.is_some());
    let function_help_card = project_function_help_card(formula_space);
    let capability_context = project_capability_context(formula_space, state);
    let formula_drill = project_formula_drill(formula_space);
    let scenario_breadcrumb = project_scenario_breadcrumb(formula_space, state);
    let formatting_controls = FormattingControlsView::from_state(
        &formula_space.formatting,
        formula_space.formatting_panel_open,
        &state.ambient_app_context,
    );
    let formula_tab_strip = project_formula_tab_strip(state);
    let command_palette = project_command_palette(state);
    let manage_formulas = project_manage_formulas(state);
    let vba_host_context = project_vba_host_context(state);
    let status = project_status_view(formula_space);
    let skin_snapshot = project_skin_snapshot(
        formula_space,
        state,
        &entry_mode_pill,
        &syntax_runs,
        &diagnostic_squiggles,
        &editor_metrics,
        completion_popup.as_ref(),
        signature_help.as_ref(),
        function_help_card.as_ref(),
        &result_view,
        &formatting_controls,
        &formula_drill,
        &status,
    );
    HomeShellViewModel {
        skin_snapshot,
        raw_entered_cell_text: formula_space.raw_entered_cell_text.clone(),
        editor_surface_state: formula_space.editor_surface_state.clone(),
        entry_mode_pill,
        result_class_pill,
        syntax_runs,
        diagnostic_squiggles,
        editor_metrics,
        result_context,
        completion_popup,
        signature_help,
        function_help_card,
        capability_context,
        formula_drill,
        view_mode,
        result_view,
        status,
        scenario_breadcrumb,
        formula_tab_strip,
        command_palette,
        formatting_controls,
        vba_host_context,
        manage_formulas,
    }
}

fn project_vba_host_context(state: &OneCalcHostState) -> VbaHostContextView {
    let associations = state
        .vba_host_context
        .associations
        .iter()
        .map(|association| VbaHostAssociationView {
            association_id: association.association_id.clone(),
            display_name: association.display_name.clone(),
            source_ref: vba_source_ref_label(association),
            source_kind: vba_source_kind_label(association).to_string(),
            enabled: association.enabled,
            status_label: vba_load_status_label(association),
            admitted_udf_count: association.admitted_udf_count,
            rejected_candidate_count: association.rejected_candidate_count,
            admitted_udfs: association.admitted_udfs.clone(),
            rejected_candidates: association.rejected_candidates.clone(),
        })
        .collect::<Vec<_>>();
    let admitted_total = associations
        .iter()
        .map(|association| association.admitted_udf_count)
        .sum::<usize>();
    let rejected_total = associations
        .iter()
        .map(|association| association.rejected_candidate_count)
        .sum::<usize>();
    let summary = if associations.is_empty() {
        "no VBA sources".to_string()
    } else {
        format!(
            "{} source(s) · {} UDF(s) · {} rejected",
            associations.len(),
            admitted_total,
            rejected_total
        )
    };
    VbaHostContextView {
        pending_project_path: state.vba_host_context.pending_project_path.clone(),
        associations,
        summary,
    }
}

fn vba_source_kind_label(association: &VbaHostAssociationState) -> &'static str {
    if association.source_kind == VbaHostAssociationSourceKind::ModuleSource
        && association.source_ref.starts_with("browser-file:")
    {
        "browser .bas"
    } else {
        association.source_kind.label()
    }
}

fn vba_source_ref_label(association: &VbaHostAssociationState) -> String {
    association
        .source_ref
        .strip_prefix("browser-file:")
        .unwrap_or(&association.source_ref)
        .to_string()
}

fn vba_load_status_label(association: &VbaHostAssociationState) -> String {
    match &association.load_status {
        crate::state::VbaHostAssociationLoadStatus::PendingLoad => "pending load".to_string(),
        crate::state::VbaHostAssociationLoadStatus::Loaded => "loaded".to_string(),
        crate::state::VbaHostAssociationLoadStatus::Failed(reason) => {
            format!("failed: {reason}")
        }
    }
}

fn project_capability_context(
    formula_space: &FormulaSpaceState,
    state: &OneCalcHostState,
) -> CapabilityContextView {
    let document = formula_space.editor_document.as_ref();
    let function_profiles =
        project_function_semantic_profile(&formula_space.raw_entered_cell_text, document);
    let mut value_capability_facts = Vec::new();
    if let Some(value_presentation) = document.and_then(|doc| doc.value_presentation.as_ref()) {
        value_capability_facts.extend(value_presentation.producer_capability_set_keys.iter().map(
            |key| ValueCapabilityFact {
                fact_kind: ValueCapabilityFactKind::ProducerCanProvide,
                key: key.clone(),
            },
        ));
        value_capability_facts.extend(value_presentation.exercised_capability_keys.iter().map(
            |key| ValueCapabilityFact {
                fact_kind: ValueCapabilityFactKind::ExercisedThisRun,
                key: key.clone(),
            },
        ));
    }

    CapabilityContextView {
        snapshot: build_capability_ledger_snapshot(state),
        function_profiles,
        value_capability_facts,
        formula_inputs: formula_space
            .formula_input_bindings
            .iter()
            .map(|binding| FormulaInputBindingView {
                label: binding.label.clone(),
                reference_descriptor: binding.reference_descriptor.clone(),
                reference_handle: binding.reference_handle.clone(),
                value_preview: eval_value_preview(&binding.value),
            })
            .collect(),
    }
}

fn eval_value_preview(value: &CalcValue) -> String {
    if value.callable_value().is_some() {
        return "Callable value".to_string();
    }
    if value.rich().is_some() {
        return "Rich value".to_string();
    }
    match value.core() {
        CoreValue::Number(number) => format!("{number}"),
        CoreValue::Text(text) => text.to_string_lossy(),
        CoreValue::Logical(value) => {
            if *value {
                "TRUE".to_string()
            } else {
                "FALSE".to_string()
            }
        }
        CoreValue::Error(code) => worksheet_error_literal(*code).to_string(),
        CoreValue::Empty | CoreValue::Missing => String::new(),
        CoreValue::Array(array) => {
            let shape = array.shape();
            format!("Array[{} x {}]", shape.rows, shape.cols)
        }
        CoreValue::Reference(reference) => reference.target().to_string(),
    }
}

/// Project the manage-formulas overlay. Closed shape returns an
/// empty list with `is_open: false` so the renderer can short-
/// circuit. Open shape walks `pinned`, `open`, then `recent` in
/// that priority order, deduping by id; each surviving id becomes
/// a `ManageFormulasRow`. The result is filtered by the search
/// query (case-insensitive substring match against the row's
/// display name AND its formula preview).
fn project_manage_formulas(state: &OneCalcHostState) -> ManageFormulasView {
    let chrome = &state.global_ui_chrome;
    if !chrome.manage_formulas_open {
        return ManageFormulasView {
            is_open: false,
            search_query: String::new(),
            total_count: 0,
            rows: Vec::new(),
        };
    }
    let active_id = state.workspace_shell.active_formula_space_id.as_ref();
    let pinned_set = &state.workspace_shell.pinned_formula_space_ids;

    // Walk in priority order — pinned first (in their stored
    // BTreeSet order, which is the canonical id-string order),
    // then open (in user's tab order), then recent (most-recent
    // first). Dedup as we go.
    let mut visited: std::collections::HashSet<crate::domain::ids::FormulaSpaceId> =
        std::collections::HashSet::new();
    let mut id_order: Vec<crate::domain::ids::FormulaSpaceId> = Vec::new();
    for id in pinned_set.iter() {
        if visited.insert(id.clone()) {
            id_order.push(id.clone());
        }
    }
    for id in &state.workspace_shell.open_formula_space_order {
        if visited.insert(id.clone()) {
            id_order.push(id.clone());
        }
    }
    for id in &state.workspace_shell.recent_formula_space_order {
        if visited.insert(id.clone()) {
            id_order.push(id.clone());
        }
    }

    let total_count = id_order.len();
    let needle = chrome.manage_formulas_search_query.to_lowercase();

    let rows: Vec<ManageFormulasRow> = id_order
        .into_iter()
        .filter_map(|id| {
            let is_open_space = state.workspace_shell.open_formula_space_order.contains(&id);
            // For an open space we read the live FormulaSpaceState;
            // for a recent we read the closed-record snapshot. Both
            // surfaces have the fields we need (`scenario_label`,
            // `raw_entered_cell_text`, `committed_cell_text`).
            let (display_name, raw_text, committed_text) = if is_open_space {
                let space = state.formula_spaces.get(&id)?;
                (
                    space.context.scenario_label.clone(),
                    space.raw_entered_cell_text.clone(),
                    space.committed_cell_text.clone(),
                )
            } else {
                let record = state.workspace_shell.recent_formula_spaces.get(&id)?;
                (
                    record.formula_space.context.scenario_label.clone(),
                    record.formula_space.raw_entered_cell_text.clone(),
                    record.formula_space.committed_cell_text.clone(),
                )
            };
            let display_name = if display_name.is_empty() || display_name == id.as_str() {
                id.as_str().to_string()
            } else {
                display_name
            };
            let formula_preview = build_formula_preview(&raw_text);
            // Search match: name OR preview (each lowercased once).
            if !needle.is_empty() {
                let name_lc = display_name.to_lowercase();
                let preview_lc = formula_preview.to_lowercase();
                if !name_lc.contains(&needle) && !preview_lc.contains(&needle) {
                    return None;
                }
            }
            let is_dirty = match committed_text.as_deref() {
                Some(committed) => committed != raw_text.as_str(),
                None => !raw_text.is_empty(),
            };
            Some(ManageFormulasRow {
                formula_space_id: id.as_str().to_string(),
                display_name,
                formula_preview,
                is_pinned: pinned_set.contains(&id),
                is_open: is_open_space,
                is_active: active_id.is_some_and(|active| active == &id),
                is_dirty,
            })
        })
        .collect();

    ManageFormulasView {
        is_open: true,
        search_query: chrome.manage_formulas_search_query.clone(),
        total_count,
        rows,
    }
}

/// Build the muted preview text the manage-formulas overlay
/// surfaces under each formula's display name. Collapses every
/// run of whitespace (newlines, tabs, multi-spaces) to a single
/// space so multi-line formulas don't break the row layout, and
/// truncates at ~80 chars with an ellipsis. An empty raw text
/// returns an empty string — the renderer hides the preview line
/// when this is empty.
fn build_formula_preview(raw_text: &str) -> String {
    let collapsed: String = raw_text.split_whitespace().collect::<Vec<&str>>().join(" ");
    const PREVIEW_BUDGET: usize = 80;
    if collapsed.chars().count() <= PREVIEW_BUDGET {
        return collapsed;
    }
    let truncated: String = collapsed.chars().take(PREVIEW_BUDGET).collect();
    format!("{truncated}…")
}

/// Convenience for the renderer's keyboard `Enter` handler:
/// resolve the currently-selected command's kind by re-running
/// the palette projection. Returns `None` when the palette is
/// closed or the filtered list is empty.
pub fn project_command_palette_entry_for_dispatch(
    state: &OneCalcHostState,
) -> Option<CommandPaletteEntryKind> {
    let palette = project_command_palette(state);
    if !palette.is_open {
        return None;
    }
    palette
        .commands
        .into_iter()
        .nth(palette.selected_index)
        .map(|entry| entry.kind)
}

/// Project the command-palette overlay state. When closed,
/// returns an empty `CommandPaletteView` with `is_open: false`;
/// the renderer skips rendering entirely. When open, builds the
/// full command set, filters by the user's query (case-
/// insensitive substring match against `label`/`detail`), and
/// clamps the selected index into range.
fn project_command_palette(state: &OneCalcHostState) -> CommandPaletteView {
    let chrome = &state.global_ui_chrome;
    if !chrome.command_palette_open {
        return CommandPaletteView {
            is_open: false,
            query: String::new(),
            commands: Vec::new(),
            selected_index: 0,
        };
    }
    let query = chrome.command_palette_query.clone();
    let needle = query.to_lowercase();
    let mut commands: Vec<CommandPaletteEntry> = Vec::new();

    // Section: open formulas (so Ctrl+K → type → Enter is a fast
    // switcher even when the tab strip is hidden because only one
    // formula is open).
    let active_id = state.workspace_shell.active_formula_space_id.as_ref();
    for id in &state.workspace_shell.open_formula_space_order {
        let Some(space) = state.formula_spaces.get(id) else {
            continue;
        };
        let label = if space.context.scenario_label.is_empty()
            || space.context.scenario_label == id.as_str()
        {
            id.as_str().to_string()
        } else {
            space.context.scenario_label.clone()
        };
        let is_active = active_id.is_some_and(|active| active == id);
        let detail = Some(if is_active {
            "Open · active".to_string()
        } else {
            "Open".to_string()
        });
        commands.push(CommandPaletteEntry {
            kind: CommandPaletteEntryKind::SwitchFormula(id.as_str().to_string()),
            label,
            section: "Formulas",
            detail,
            chord: "",
        });
    }
    // Section: pinned (excluding any already in the open list).
    for id in &state.workspace_shell.pinned_formula_space_ids {
        if state
            .workspace_shell
            .open_formula_space_order
            .iter()
            .any(|open| open == id)
        {
            continue;
        }
        let label = state
            .workspace_shell
            .recent_formula_spaces
            .get(id)
            .map(|record| record.formula_space.context.scenario_label.clone())
            .unwrap_or_else(|| id.as_str().to_string());
        commands.push(CommandPaletteEntry {
            kind: CommandPaletteEntryKind::SwitchFormula(id.as_str().to_string()),
            label,
            section: "Pinned",
            detail: Some("Pinned".to_string()),
            chord: "",
        });
    }
    // Section: recent (closed) formulas.
    for id in &state.workspace_shell.recent_formula_space_order {
        if state
            .workspace_shell
            .open_formula_space_order
            .iter()
            .any(|open| open == id)
        {
            continue;
        }
        let Some(record) = state.workspace_shell.recent_formula_spaces.get(id) else {
            continue;
        };
        let label = if record.formula_space.context.scenario_label.is_empty() {
            id.as_str().to_string()
        } else {
            record.formula_space.context.scenario_label.clone()
        };
        commands.push(CommandPaletteEntry {
            kind: CommandPaletteEntryKind::SwitchFormula(id.as_str().to_string()),
            label,
            section: "Recent",
            detail: Some("Closed".to_string()),
            chord: "",
        });
    }
    // Section: actions. Mirror the breadcrumb dropdown so the
    // palette is a complete keyboard alternative.
    let active_is_pinned =
        active_id.is_some_and(|id| state.workspace_shell.pinned_formula_space_ids.contains(id));
    let pin_action = if active_is_pinned {
        (
            ScenarioBreadcrumbActionId::UnpinActive,
            "Unpin active formula",
        )
    } else {
        (ScenarioBreadcrumbActionId::PinActive, "Pin active formula")
    };
    let scenario_actions: &[(ScenarioBreadcrumbActionId, &'static str, &'static str)] = &[
        (
            ScenarioBreadcrumbActionId::NewScenario,
            "New formula",
            "Ctrl+N",
        ),
        (
            ScenarioBreadcrumbActionId::SaveAs,
            "Save formula…",
            "Ctrl+Shift+S",
        ),
        (ScenarioBreadcrumbActionId::Open, "Open formula…", "Ctrl+O"),
        (
            ScenarioBreadcrumbActionId::Duplicate,
            "Clone active formula",
            "",
        ),
        (
            ScenarioBreadcrumbActionId::RenameActive,
            "Rename active formula…",
            "",
        ),
        (pin_action.0, pin_action.1, ""),
        (
            ScenarioBreadcrumbActionId::ManageScenarios,
            "Manage formulas…",
            "",
        ),
    ];
    for (action_id, label, chord) in scenario_actions {
        commands.push(CommandPaletteEntry {
            kind: CommandPaletteEntryKind::ScenarioAction(*action_id),
            label: label.to_string(),
            section: "Actions",
            detail: None,
            chord,
        });
    }
    // Section: workspace settings (toggles + force-recalc).
    commands.push(CommandPaletteEntry {
        kind: CommandPaletteEntryKind::ToggleFormattingPanel,
        label: "Toggle formatting panel".to_string(),
        section: "Settings",
        detail: None,
        chord: "",
    });
    commands.push(CommandPaletteEntry {
        kind: CommandPaletteEntryKind::ToggleFormulaDrill,
        label: "Toggle formula drill-down".to_string(),
        section: "Settings",
        detail: None,
        chord: "Ctrl+D",
    });
    commands.push(CommandPaletteEntry {
        kind: CommandPaletteEntryKind::ForceRecalc,
        label: "Force recalc".to_string(),
        section: "Settings",
        detail: None,
        chord: "F9",
    });

    // Filter by query. Empty query passes everything through;
    // non-empty does a case-insensitive substring match against
    // both label and detail.
    let filtered: Vec<CommandPaletteEntry> = if needle.trim().is_empty() {
        commands
    } else {
        commands
            .into_iter()
            .filter(|cmd| {
                let label_match = cmd.label.to_lowercase().contains(&needle);
                let detail_match = cmd
                    .detail
                    .as_deref()
                    .map(|d| d.to_lowercase().contains(&needle))
                    .unwrap_or(false);
                label_match || detail_match
            })
            .collect()
    };

    let selected_index = if filtered.is_empty() {
        0
    } else {
        chrome
            .command_palette_selected_index
            .min(filtered.len().saturating_sub(1))
    };

    CommandPaletteView {
        is_open: true,
        query,
        commands: filtered,
        selected_index,
    }
}

/// Project the open-formula list into the tab-strip view-model.
/// One chip per `workspace_shell.open_formula_space_order` entry,
/// in stable order. The strip hides itself (`is_visible == false`)
/// when only one formula is open — the breadcrumb already names
/// it; an extra row of chrome would just take vertical space.
fn project_formula_tab_strip(state: &OneCalcHostState) -> FormulaTabStripView {
    let active_id = state.workspace_shell.active_formula_space_id.as_ref();
    let renaming_id = state.workspace_shell.renaming_formula_space_id.as_ref();
    let pending_rename_text = state.workspace_shell.pending_rename_text.clone();
    let chips: Vec<FormulaTabChip> = state
        .workspace_shell
        .open_formula_space_order
        .iter()
        .filter_map(|id| {
            let space = state.formula_spaces.get(id)?;
            let is_active = active_id.is_some_and(|active| active == id);
            let is_pinned = state.workspace_shell.pinned_formula_space_ids.contains(id);
            let is_renaming = renaming_id.is_some_and(|target| target == id);
            // The chip's display name follows the same rule as the
            // breadcrumb: prefer the user's `scenario_label`,
            // falling back to the synthetic id when the label is
            // empty or matches the id verbatim.
            let display_name = if space.context.scenario_label.is_empty()
                || space.context.scenario_label == id.as_str()
            {
                id.as_str().to_string()
            } else {
                space.context.scenario_label.clone()
            };
            // Dirty marker: raw text differs from the last
            // committed text. `committed_cell_text == None`
            // counts as clean only when the raw text is also
            // empty (the fresh-formula case); a non-empty raw
            // text with no commit is dirty.
            let is_dirty = match space.committed_cell_text.as_deref() {
                Some(committed) => committed != space.raw_entered_cell_text.as_str(),
                None => !space.raw_entered_cell_text.is_empty(),
            };
            let rename_buffer = if is_renaming {
                pending_rename_text.clone()
            } else {
                String::new()
            };
            Some(FormulaTabChip {
                formula_space_id: id.as_str().to_string(),
                display_name,
                is_active,
                is_pinned,
                is_dirty,
                is_renaming,
                rename_buffer,
            })
        })
        .collect();
    // Strip stays visible when a rename is in progress even if
    // there's only one tab — the user needs to see the input.
    let is_visible = chips.len() > 1 || renaming_id.is_some();
    FormulaTabStripView { is_visible, chips }
}

/// Project the formula walk tree + phase summaries into the
/// drill-down view model. Always returns a `FormulaDrillView` —
/// the `expanded` flag follows the formula space's
/// `formula_drill_open`, and the `tree` / `phase_summaries`
/// vectors are empty when the document is missing or stale.
///
/// `document_is_fresh` lets the component distinguish "panel
/// open but bridge round-trip pending" (show a loading state)
/// from "panel open and tree ready".
fn project_formula_drill(formula_space: &FormulaSpaceState) -> FormulaDrillView {
    let document = formula_space.editor_document.as_ref();
    let document_is_fresh = document
        .map(|doc| doc.source_text == formula_space.raw_entered_cell_text)
        .unwrap_or(false);

    let mut tree = Vec::new();
    if document_is_fresh {
        if let Some(document) = document {
            for node in &document.formula_walk {
                tree.push(project_walk_node(node));
            }
        }
    }

    // Suppress diagnostics for empty input — same rule as
    // `project_diagnostic_squiggles` / `project_editor_metrics`.
    let suppress_diagnostics = formula_space.raw_entered_cell_text.is_empty();
    let diagnostics: Vec<FormulaDrillDiagnosticRow> = if suppress_diagnostics {
        Vec::new()
    } else if document_is_fresh {
        document
            .map(|document| {
                document
                    .live_diagnostics
                    .diagnostics
                    .iter()
                    .map(|diag| FormulaDrillDiagnosticRow {
                        diagnostic_id: diag.diagnostic_id.clone(),
                        severity: SquiggleSeverity::from_upstream(diag.severity),
                        stage: DiagnosticStage::from_upstream(diag.stage),
                        code: diag.code.clone(),
                        message: diag.message.clone(),
                        span_start: diag.primary_span.start,
                        span_len: diag.primary_span.len,
                    })
                    .collect()
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let mut phase_summaries = Vec::new();
    if document_is_fresh {
        if let Some(document) = document {
            if let Some(parse) = &document.parse_summary {
                let state = if parse.status == "Valid" {
                    FormulaDrillPhaseState::Ok
                } else {
                    FormulaDrillPhaseState::Pending
                };
                phase_summaries.push(FormulaDrillPhaseChip {
                    label: "parse",
                    detail: format!("{} ({} tokens)", parse.status, parse.token_count),
                    state,
                });
            }
            if let Some(bind) = &document.bind_summary {
                phase_summaries.push(FormulaDrillPhaseChip {
                    label: "bind",
                    detail: format!(
                        "{} vars · {} refs",
                        bind.variable_count, bind.reference_count
                    ),
                    state: FormulaDrillPhaseState::Ok,
                });
            }
            if let Some(eval) = &document.eval_summary {
                let blocked = document
                    .provenance_summary
                    .as_ref()
                    .and_then(|p| p.blocked_reason.as_ref())
                    .is_some();
                phase_summaries.push(FormulaDrillPhaseChip {
                    label: "eval",
                    detail: format!(
                        "{} step{} · {}",
                        eval.step_count,
                        if eval.step_count == 1 { "" } else { "s" },
                        eval.duration_text
                    ),
                    state: if blocked {
                        FormulaDrillPhaseState::Blocked
                    } else {
                        FormulaDrillPhaseState::Ok
                    },
                });
            }
        }
    }

    FormulaDrillView {
        expanded: formula_space.formula_drill_open,
        tree,
        diagnostics,
        phase_summaries,
        document_is_fresh,
    }
}

/// Project a `FormulaDrillNodeViewModel` into the view-model shape, recursing
/// into children so the tree stays nested. The renderer uses
/// `<details>` elements to give the user click-to-collapse on each
/// node — matching how the eye reads a function-call structure.
fn project_walk_node(node: &crate::adapters::oxfml::FormulaDrillNodeViewModel) -> FormulaDrillNode {
    FormulaDrillNode {
        node_id: node.node_id.clone(),
        label: node.label.clone(),
        developer_label: node.developer_label.clone(),
        expression_text: node.expression_text.clone(),
        kind: node.kind.clone(),
        source_span_start: node.source_span_start,
        source_span_len: node.source_span_len,
        branch_disposition: node.branch_disposition.clone(),
        argument_name: node.argument_name.clone(),
        argument_role: node.argument_role.clone(),
        error_message: node.error_message.clone(),
        value_preview: node.value_preview.clone(),
        array_preview: node.array_preview.clone(),
        state: node.state,
        children: node.children.iter().map(project_walk_node).collect(),
    }
}

/// Project the bridge's `FunctionHelpPacket` into the view-model
/// shape consumed by the hover-help tooltip. Returns `None` when:
///   * the editor document is missing or stale, or
///   * the bridge did not produce a function_help (no function
///     context for the current caret position).
///
/// The component decides WHEN to render the tooltip (based on hover
/// state); the view-model only carries the *content* and the
/// `lookup_key` that gates which `.syn-fn` span is eligible.
fn project_function_help_card(formula_space: &FormulaSpaceState) -> Option<FunctionHelpCardView> {
    let document = formula_space.editor_document.as_ref()?;
    if document.source_text != formula_space.raw_entered_cell_text {
        return None;
    }
    let packet = document.function_help.as_ref()?;
    let signature = packet.signature_forms.first().and_then(|form| {
        non_placeholder_signature(&packet.display_name, &form.display_signature).map(str::to_string)
    });
    Some(FunctionHelpCardView {
        lookup_key: packet.lookup_key.clone(),
        display_name: packet.display_name.clone(),
        signature,
        short_description: packet.short_description.clone(),
        availability_summary: packet.availability_summary.clone(),
        deferred_or_profile_limited: packet.deferred_or_profile_limited,
    })
}

fn non_placeholder_signature<'a>(display_name: &str, signature: &'a str) -> Option<&'a str> {
    let trimmed = signature.trim();
    if trimmed.is_empty() {
        return None;
    }
    let display_name = display_name.trim();
    if !display_name.is_empty()
        && trimmed.eq_ignore_ascii_case(&format!("{}(...)", display_name.to_ascii_uppercase()))
    {
        return None;
    }
    if !display_name.is_empty() && trimmed.eq_ignore_ascii_case(&format!("{display_name}(...)")) {
        return None;
    }
    Some(trimmed)
}

/// Project the bridge's signature-help context into the home shell's
/// renderable view-model.
///
/// Returns `None` when:
///   * the editor document is missing or stale (`source_text !=
///     raw_entered_cell_text`),
///   * the bridge did not produce a `signature_help` for the current
///     caret position (the user is not inside an open function call),
///   * the caret-box metrics have not yet been measured (without
///     metrics the anchor cannot be placed; same gate as the
///     completion popup), or
///   * the completion popup is already `Open` at the same caret —
///     popup wins to avoid stacking two overlays at the same spot.
///
/// The parameter list is sourced from the matching function-help
/// packet's `argument_help` rather than parsing the formatted
/// `signature_form.display_signature` string. If the function-help
/// packet is missing, fall back to a single empty parameter list so
/// the callee name still renders.
fn project_signature_help(
    formula_space: &FormulaSpaceState,
    completion_popup_open: bool,
) -> Option<SignatureHelpView> {
    if completion_popup_open {
        return None;
    }

    let document = formula_space.editor_document.as_ref()?;
    if document.source_text != formula_space.raw_entered_cell_text {
        return None;
    }

    let signature_help_context = document.signature_help.as_ref()?;
    let metrics = formula_space.editor_box_metrics?;

    let parameters: Vec<SignatureHelpParameter> = document
        .function_help
        .as_ref()
        .map(|packet| {
            packet
                .argument_help
                .iter()
                .enumerate()
                .map(|(index, name)| SignatureHelpParameter {
                    name: name.clone(),
                    is_active: index == signature_help_context.active_argument_index,
                })
                .collect()
        })
        .unwrap_or_default();

    let active_parameter = if signature_help_context.active_argument_index < parameters.len() {
        Some(signature_help_context.active_argument_index)
    } else {
        None
    };

    let caret_offset = formula_space.editor_surface_state.caret.offset;
    let anchor = caret_box_for_offset(&formula_space.raw_entered_cell_text, caret_offset, metrics);

    Some(SignatureHelpView {
        callee_text: signature_help_context.callee_text.clone(),
        anchor_left_px: anchor.left_px,
        anchor_top_px: anchor.top_px,
        line_height_px: metrics.line_height_px.max(1),
        parameters,
        active_parameter,
    })
}

/// Project the completion popup state into a renderable view-model.
/// Returns `None` when:
///   * the popup is in `Hidden` state, or
///   * `editor_box_metrics` is `None` (the browser adapter has not yet
///     measured the textarea — without metrics the anchor cannot be
///     placed, so the popup is suppressed for one frame).
///
/// When both gates pass, the anchor is computed via
/// [`caret_box_for_offset`] from the popup's `anchor_offset` (which the
/// reducer sourced from the proposal's `replacement_span.start` or the
/// caret offset). Each item maps to a `CompletionPopupItemView` with
/// `is_selected` set on the popup's `selected_index`.
fn project_completion_popup(formula_space: &FormulaSpaceState) -> Option<CompletionPopupView> {
    let CompletionPopupState::Open {
        anchor_offset,
        items,
        selected_index,
    } = &formula_space.completion_popup
    else {
        return None;
    };
    let metrics = formula_space.editor_box_metrics?;
    let anchor = caret_box_for_offset(
        &formula_space.raw_entered_cell_text,
        *anchor_offset,
        metrics,
    );
    let item_views: Vec<CompletionPopupItemView> = items
        .iter()
        .enumerate()
        .map(|(index, item)| CompletionPopupItemView {
            proposal_id: item.proposal_id.clone(),
            display_text: item.display_text.clone(),
            kind_glyph: CompletionPopupItemView::glyph_for_kind(item.kind),
            kind_label: CompletionPopupItemView::label_for_kind(item.kind),
            is_selected: index == *selected_index,
            documentation_ref: item.documentation_ref.clone(),
        })
        .collect();
    Some(CompletionPopupView {
        anchor_left_px: anchor.left_px,
        anchor_top_px: anchor.top_px,
        line_height_px: metrics.line_height_px.max(1),
        items: item_views,
        selected_index: *selected_index,
    })
}

/// Build the editor-foot live-metrics chip. Counts come from the editor
/// document where present (token_count, diagnostic_count) and from the
/// projected syntax runs (function_count = run with role == Function).
/// All zeros when there is no document.
///
/// Empty-input guard: when the user has typed then cleared the formula,
/// upstream OxFml's parser still runs against the empty source and emits
/// "unexpected token Eof" diagnostics. Those are correct for a
/// `parse(empty)` API call but wrong for the editor surface — an empty
/// formula is the *initial state*, not a syntax error. The host
/// suppresses every diagnostic surface (squiggles + metrics chip) when
/// `raw_entered_cell_text` is empty so the editor reads as quiet again
/// after a delete-to-empty.
fn project_editor_metrics(
    formula_space: &FormulaSpaceState,
    syntax_runs: &[SyntaxRun],
) -> EditorMetricsChip {
    let document = match formula_space.editor_document.as_ref() {
        Some(document) => document,
        None => {
            return EditorMetricsChip {
                token_count: 0,
                function_count: 0,
                diagnostic_count: 0,
                first_diagnostic_message: None,
            }
        }
    };
    let function_count = syntax_runs
        .iter()
        .filter(|run| run.role == SyntaxTokenRole::Function)
        .count();
    let suppress_diagnostics = formula_space.raw_entered_cell_text.is_empty();
    let first_diagnostic_message = if suppress_diagnostics {
        None
    } else {
        document
            .live_diagnostics
            .diagnostics
            .first()
            .map(|diagnostic| diagnostic.message.clone())
    };
    let diagnostic_count = if suppress_diagnostics {
        0
    } else {
        document.live_diagnostics.diagnostics.len()
    };
    EditorMetricsChip {
        token_count: document.editor_syntax_snapshot.tokens.len(),
        function_count,
        diagnostic_count,
        first_diagnostic_message,
    }
}

/// Build the result-foot active-context chip. The chip is
/// progressive (per WS-14 §5 "result-foot rethink"): the foot
/// collapses entirely when the formula is in default state
/// (General format, `LiveRecalc` policy, no CF rules), and
/// surfaces only when the user has authored something worth
/// glancing at — or when policy is `ManualRecalc` (always shown
/// because the user needs a visual reminder that typing isn't
/// triggering recalc).
fn project_result_context(
    formula_space: &FormulaSpaceState,
    _ambient: &crate::state::AmbientAppContext,
) -> Option<ResultContextChip> {
    let format_code = formula_space.formatting.number_format_code.trim();
    let has_format = !format_code.is_empty();
    let has_cf_rules = !formula_space
        .formatting
        .conditional_formatting_rules
        .is_empty();
    let policy = formula_space.formatting.scenario_policy;
    let is_manual_recalc = matches!(policy, crate::persistence::ScenarioPolicy::ManualRecalc,);
    let is_default_policy = matches!(policy, crate::persistence::ScenarioPolicy::LiveRecalc);
    // Collapse the chip when nothing is set and the policy is at
    // its default. ManualRecalc forces visibility regardless,
    // because the user needs to see "your typing isn't running
    // the formula" while it's in effect.
    if !has_format && !has_cf_rules && is_default_policy && !is_manual_recalc {
        return None;
    }
    let format_label = if has_format {
        classify_format_family_label(format_code)
    } else {
        "General".to_string()
    };
    let policy_label = match policy {
        crate::persistence::ScenarioPolicy::Deterministic => "deterministic",
        crate::persistence::ScenarioPolicy::LiveRecalc => "live-recalc",
        crate::persistence::ScenarioPolicy::ManualRecalc => "manual-recalc",
    };
    Some(ResultContextChip {
        format: ContextChipField::Live(format_label),
        policy: ContextChipField::Live(policy_label.to_string()),
    })
}

/// Crude inverse of the format-preset table: pick the closest
/// matching family label for a given number-format code so the
/// result-context chip reads as a human family name rather than
/// the raw code. Falls back to a `Custom` label for codes the
/// user authored that don't match any preset.
fn classify_format_family_label(format_code: &str) -> String {
    for preset in number_format_presets() {
        if !preset.format_code.is_empty() && preset.format_code == format_code {
            return preset.label.to_string();
        }
    }
    format!("Custom · {format_code}")
}

/// Best-effort label for the workspace's ambient locale. Reads
/// the date format code as a heuristic for the locale family
/// (since the AmbientAppContext doesn't yet carry a locale id —
/// it derives format codes from `navigator.language` directly).
/// Replaced by a live `LocaleProfileId` label when OxFml's
/// locale tables grow per the locale-expansion handoff.
fn ambient_locale_label(ambient: &crate::state::AmbientAppContext) -> String {
    let date = ambient.date_format_code.as_str();
    if date.starts_with("yyyy") {
        "yyyy-mm-dd locale".to_string()
    } else if date.starts_with("dd.") {
        "dd.mm.yyyy locale".to_string()
    } else if date.starts_with("dd/") {
        "dd/mm/yyyy locale".to_string()
    } else if date.starts_with("m/") {
        "m/d/yyyy locale".to_string()
    } else {
        format!("custom · {date}")
    }
}

/// Build the coloured-token runs for the syntax overlay. Returns an empty
/// vector when the editor document is missing, or when its `source_text`
/// does not match the current `raw_entered_cell_text` (a stale snapshot
/// from a prior keystroke); the home shell falls back to uncoloured raw
/// text in that case so the overlay never shows misaligned colours.
fn project_syntax_runs(formula_space: &FormulaSpaceState) -> Vec<SyntaxRun> {
    let document = match formula_space.editor_document.as_ref() {
        Some(document) => document,
        None => return Vec::new(),
    };
    if document.source_text != formula_space.raw_entered_cell_text {
        return Vec::new();
    }
    syntax_runs_from_snapshot(&document.editor_syntax_snapshot)
}

/// Build the diagnostic squiggle list. Pulls `LiveDiagnostic`s out of the
/// editor document, sorts them by `span_start`, and prunes any entry whose
/// span overlaps with the previously kept entry — the renderer relies on
/// non-overlapping spans for clean text segmentation. Returns empty when
/// the document is missing or stale, so squiggles never sit at offsets
/// that don't match the textarea contents.
fn project_diagnostic_squiggles(formula_space: &FormulaSpaceState) -> Vec<DiagnosticSquiggle> {
    let document = match formula_space.editor_document.as_ref() {
        Some(document) => document,
        None => return Vec::new(),
    };
    if document.source_text != formula_space.raw_entered_cell_text {
        return Vec::new();
    }
    // See `project_editor_metrics`: empty input is the editor's idle
    // state, not a syntax error. Suppress the parse-level "unexpected
    // token Eof" stream upstream OxFml emits for empty source.
    if formula_space.raw_entered_cell_text.is_empty() {
        return Vec::new();
    }
    let mut squiggles: Vec<DiagnosticSquiggle> = document
        .live_diagnostics
        .diagnostics
        .iter()
        .map(|diag| DiagnosticSquiggle {
            diagnostic_id: diag.diagnostic_id.clone(),
            message: diag.message.clone(),
            severity: SquiggleSeverity::from_upstream(diag.severity),
            stage: DiagnosticStage::from_upstream(diag.stage),
            code: diag.code.clone(),
            worksheet_error_class: diag.worksheet_error_class.clone(),
            span_start: diag.primary_span.start,
            span_len: diag.primary_span.len,
        })
        .collect();
    squiggles.sort_by_key(|s| s.span_start);
    let mut deduped = Vec::with_capacity(squiggles.len());
    let mut last_end: Option<usize> = None;
    for squiggle in squiggles {
        let start = squiggle.span_start;
        if last_end.is_some_and(|end| start < end) {
            continue;
        }
        last_end = Some(squiggle.span_start.saturating_add(squiggle.span_len));
        deduped.push(squiggle);
    }
    deduped
}

/// Project the host adapter's `ArrayCellFormatGrid` onto the
/// view-model shape, clamped to the preview window so the
/// renderer doesn't index past `cells`. The carrier upstream
/// can be larger than the preview when the bridge truncated
/// — we keep view-model and renderer in lock-step.
fn project_array_cell_format_grid(
    grid: &crate::adapters::oxfml::ArrayCellFormatGrid,
    preview_rows: usize,
    preview_cols: usize,
) -> Vec<Vec<ArrayCellFormatView>> {
    grid.rows
        .iter()
        .take(preview_rows)
        .map(|row| {
            row.iter()
                .take(preview_cols)
                .map(project_array_cell_format)
                .collect()
        })
        .collect()
}

fn project_array_cell_format(
    cell: &crate::adapters::oxfml::ArrayCellFormat,
) -> ArrayCellFormatView {
    ArrayCellFormatView {
        effective_font_color: cell.effective_font_color.clone(),
        effective_fill_color: cell.effective_fill_color.clone(),
        data_bar: cell.data_bar.as_ref().map(|fill| DataBarFillView {
            fill_ratio: fill.fill_ratio,
            bar_color: fill.bar_color.clone(),
            direction: match fill.direction {
                crate::adapters::oxfml::DataBarDirection::Left => DataBarDirectionView::Left,
                crate::adapters::oxfml::DataBarDirection::Right => DataBarDirectionView::Right,
            },
            show_bar_only: fill.show_bar_only,
        }),
        icon: cell.icon.as_ref().map(|icon| CfIconView {
            set_kind: icon.set_kind.clone(),
            icon_index: icon.icon_index,
        }),
    }
}

/// Parse the upstream `FormulaArrayPreview.label` shape prefix back
/// to `(rows, cols)`. The bridge constructs the label as
/// `"<rows>x<cols> spill preview"`; this helper takes the rows / cols
/// numbers from the prefix so the result-hero browser can surface
/// the *full* shape (not just the preview-window shape) regardless
/// of whether the cells were truncated. Returns `None` for any label
/// shape that doesn't parse — the caller falls back to the preview-
/// window dimensions in that case.
fn parse_shape_from_array_label(label: &str) -> Option<(usize, usize)> {
    let prefix = label.split_whitespace().next()?;
    let mut parts = prefix.split('x');
    let rows: usize = parts.next()?.parse().ok()?;
    let cols: usize = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((rows, cols))
}

fn project_result_view(formula_space: &FormulaSpaceState) -> ResultView {
    let raw_text = formula_space.raw_entered_cell_text.as_str();

    // Empty editor wins regardless of stale residuals.
    if raw_text.is_empty() {
        return ResultView::Empty;
    }

    // Array result projects to a real browser. The bridge ships back
    // a (possibly-truncated) preview window + a label that already
    // carries the *full* shape (e.g. `"100x80 spill preview"` even
    // when only 30×20 cells are populated). Parse the shape from the
    // label so the renderer can surface "+N rows hidden" chips when
    // `truncated` is true.
    if let Some(preview) = formula_space.array_preview.as_ref() {
        let preview_rows = preview.rows.len();
        let preview_cols = preview.rows.first().map(|row| row.len()).unwrap_or(0);
        let (total_rows, total_cols) =
            parse_shape_from_array_label(&preview.label).unwrap_or((preview_rows, preview_cols));
        // W071 + W072: per-cell CF carrier. The bridge populates
        // `array_cell_format` on `value_presentation` when the
        // active formula has CF rules attached; rendering applies
        // the per-cell font / fill / data-bar / icon overrides.
        // The carrier dimensions come from OxFml's full shape
        // (matching `CalcValue::Array`), which can exceed the
        // preview-window dimensions — in that case the renderer
        // only consumes the first `preview_rows × preview_cols`
        // entries to stay aligned with `cells`.
        let cell_format = formula_space
            .editor_document
            .as_ref()
            .and_then(|doc| doc.value_presentation.as_ref())
            .and_then(|presentation| presentation.array_cell_format.as_ref())
            .map(|grid| project_array_cell_format_grid(grid, preview_rows, preview_cols));
        return ResultView::Array {
            total_rows,
            total_cols,
            label: preview.label.clone(),
            cells: preview.rows.clone(),
            cell_format,
            truncated: preview.truncated,
        };
    }

    // Host-derived blocked reason wins regardless of value type.
    if let Some(reason) = formula_space.context.blocked_reason.as_deref() {
        return ResultView::Error {
            code: "BLOCKED".to_string(),
            surface_repr: Some(reason.to_string()),
        };
    }

    // Bridge-side blocked-reason on the editor document (provenance summary
    // populated by the live bridge for capability-denied lanes).
    if let Some(reason) = formula_space
        .editor_document
        .as_ref()
        .and_then(|doc| doc.provenance_summary.as_ref())
        .and_then(|prov| prov.blocked_reason.clone())
    {
        return ResultView::Error {
            code: "BLOCKED".to_string(),
            surface_repr: Some(reason),
        };
    }

    // Live diagnostic on the editor document. The bridge does not produce a
    // typed value when it stops at a parse / bind diagnostic, so the
    // diagnostic itself is the visible result. We surface the first one;
    // the drill-down is responsible for the full list.
    if !has_published_value(formula_space) {
        if let Some(message) = formula_space
            .editor_document
            .as_ref()
            .and_then(|doc| doc.live_diagnostics.diagnostics.first())
            .map(|diag| diag.message.clone())
        {
            return ResultView::Error {
                code: "DIAGNOSTIC".to_string(),
                surface_repr: Some(message),
            };
        }
    }

    // Typed dispatch on the bridge's published `CalcValue`.
    if let Some(published_value) = bridge_published_value(formula_space) {
        return project_typed_value(formula_space, published_value);
    }

    // Pre-bridge hand-evaluation for raw text / number cells: anything that
    // doesn't start with `=` is a literal cell entry. The live bridge
    // doesn't run for these; the home shell evaluates them inline against
    // the raw text. Forced-text cells (`'1.5`) keep the leading apostrophe
    // out of the rendered display.
    if let Some(forced_text) = raw_text.strip_prefix('\'') {
        return ResultView::Display {
            text: forced_text.to_string(),
            kind: ResultKind::Text,
            applied_font_color: None,
            applied_fill_color: None,
        };
    }
    if !raw_text.starts_with('=') {
        if let Ok(number) = raw_text.parse::<f64>() {
            return ResultView::Display {
                text: format_literal_number(number),
                kind: ResultKind::Number,
                applied_font_color: None,
                applied_fill_color: None,
            };
        }
        return ResultView::Display {
            text: raw_text.to_string(),
            kind: ResultKind::Text,
            applied_font_color: None,
            applied_fill_color: None,
        };
    }

    ResultView::Pending
}

fn has_published_value(formula_space: &FormulaSpaceState) -> bool {
    bridge_published_value(formula_space).is_some()
}

fn bridge_published_value(formula_space: &FormulaSpaceState) -> Option<&CalcValue> {
    formula_space
        .editor_document
        .as_ref()
        .and_then(|doc| doc.value_presentation.as_ref())
        .map(|vp| &vp.published_value)
}

fn project_typed_value(formula_space: &FormulaSpaceState, value: &CalcValue) -> ResultView {
    let display_text = || {
        formula_space
            .effective_display_summary
            .clone()
            .unwrap_or_default()
    };
    let (applied_font_color, applied_fill_color) = applied_cf_colours(formula_space);
    if value.callable_value().is_some() || value.rich().is_some() {
        return ResultView::Display {
            text: display_text(),
            kind: ResultKind::Other,
            applied_font_color,
            applied_fill_color,
        };
    }
    match value.core() {
        CoreValue::Number(_) => ResultView::Display {
            text: display_text(),
            kind: ResultKind::Number,
            applied_font_color,
            applied_fill_color,
        },
        CoreValue::Text(_) => ResultView::Display {
            text: display_text(),
            kind: ResultKind::Text,
            applied_font_color,
            applied_fill_color,
        },
        CoreValue::Logical(_) => ResultView::Display {
            text: display_text(),
            kind: ResultKind::Logical,
            applied_font_color,
            applied_fill_color,
        },
        CoreValue::Error(code) => ResultView::Error {
            code: worksheet_error_literal(*code).to_string(),
            surface_repr: None,
        },
        CoreValue::Empty | CoreValue::Missing => ResultView::Display {
            text: display_text(),
            kind: ResultKind::Other,
            applied_font_color,
            applied_fill_color,
        },
        CoreValue::Array(_) => {
            // Array path normally goes through `formula_space.array_preview`
            // (handled above). If we reach here without a preview, surface
            // the effective display string. CF colours apply per-cell on
            // arrays (see `docs/HANDOFF_OXFML_CF_ARRAY_PER_CELL.md`); the
            // single overall colour the bridge currently emits for an
            // array is intentionally dropped on the floor for the array
            // path because it's the misleading "compare-the-stringified-
            // array-to-the-threshold" output.
            ResultView::Display {
                text: display_text(),
                kind: ResultKind::Other,
                applied_font_color: None,
                applied_fill_color: None,
            }
        }
        CoreValue::Reference(_) => ResultView::Display {
            text: display_text(),
            kind: ResultKind::Other,
            applied_font_color,
            applied_fill_color,
        },
    }
}

/// Pull the CF-applied font / fill colours off the editor
/// document's value presentation. `(None, None)` when no CF rule
/// fired (the publication surface emitted no override) or when
/// the bridge hasn't returned a value presentation yet.
fn applied_cf_colours(formula_space: &FormulaSpaceState) -> (Option<String>, Option<String>) {
    let Some(presentation) = formula_space
        .editor_document
        .as_ref()
        .and_then(|doc| doc.value_presentation.as_ref())
    else {
        return (None, None);
    };
    (
        presentation.effective_font_color.clone(),
        presentation.effective_fill_color.clone(),
    )
}

fn format_literal_number(value: f64) -> String {
    if (value.fract()).abs() < f64::EPSILON {
        format!("{}", value as i64)
    } else {
        value.to_string()
    }
}

/// Project the active scenario's titlebar breadcrumb + dropdown.
///
/// The label falls back to `"unsaved"` when the scenario carries
/// the synthetic default label (the `formula_space_id.as_str()`
/// auto-set by `FormulaSpaceState::new`); a user-named scenario
/// shows its name verbatim. The dirty marker tracks live edits
/// against the last commit point. Recent lists the active formula,
/// then other open formulas, then closed recents, deduplicated
/// against the active id. Pinned reads from the workspace's stable
/// id-ordered set.
///
/// Actions are a fixed list. Most carry a `SEAM-ONECALC-SCENARIO-PERSIST`
/// id today — the dropdown surfaces them as honest stubs until
/// file I/O lands. `NewScenario` is wired to a real reducer
/// helper so the slice is not entirely no-op.
fn project_scenario_breadcrumb(
    formula_space: &FormulaSpaceState,
    state: &OneCalcHostState,
) -> ScenarioBreadcrumbView {
    let synthetic_default_label =
        formula_space.context.scenario_label == formula_space.formula_space_id.as_str();
    let active_label = if synthetic_default_label {
        "unsaved".to_string()
    } else {
        formula_space.context.scenario_label.clone()
    };
    let is_dirty = matches!(
        formula_space.live_state(),
        crate::ui::editor::state::EditorLiveState::EditingLive
            | crate::ui::editor::state::EditorLiveState::ProofedScratch
    );
    let active_entry = ScenarioBreadcrumbEntry {
        formula_space_id: formula_space.formula_space_id.as_str().to_string(),
        display_name: active_label.clone(),
        meta: "active".to_string(),
        is_active: true,
        is_pinned: state
            .workspace_shell
            .pinned_formula_space_ids
            .contains(&formula_space.formula_space_id),
    };
    let active_id = formula_space.formula_space_id.clone();
    let mut recent = vec![active_entry];
    for open_id in &state.workspace_shell.open_formula_space_order {
        if recent.len() >= 5 {
            break;
        }
        if open_id == &active_id {
            continue;
        }
        let Some(open_space) = state.formula_spaces.get(open_id) else {
            continue;
        };
        let display_name = open_space.context.scenario_label.clone();
        let synthetic_default = display_name == open_id.as_str();
        recent.push(ScenarioBreadcrumbEntry {
            formula_space_id: open_id.as_str().to_string(),
            display_name: if synthetic_default {
                "unsaved".to_string()
            } else {
                display_name
            },
            meta: "open".to_string(),
            is_active: false,
            is_pinned: state
                .workspace_shell
                .pinned_formula_space_ids
                .contains(open_id),
        });
    }
    for recent_id in &state.workspace_shell.recent_formula_space_order {
        if recent.len() >= 5 {
            break;
        }
        if recent_id == &active_id {
            continue;
        }
        if recent
            .iter()
            .any(|entry| entry.formula_space_id == recent_id.as_str())
        {
            continue;
        }
        let display_name = state
            .workspace_shell
            .recent_formula_spaces
            .get(recent_id)
            .map(|record| record.formula_space.context.scenario_label.clone())
            .unwrap_or_else(|| recent_id.as_str().to_string());
        let synthetic_default = display_name == recent_id.as_str();
        recent.push(ScenarioBreadcrumbEntry {
            formula_space_id: recent_id.as_str().to_string(),
            display_name: if synthetic_default {
                "unsaved".to_string()
            } else {
                display_name
            },
            meta: "recent".to_string(),
            is_active: false,
            is_pinned: state
                .workspace_shell
                .pinned_formula_space_ids
                .contains(recent_id),
        });
    }
    let pinned: Vec<ScenarioBreadcrumbEntry> = state
        .workspace_shell
        .pinned_formula_space_ids
        .iter()
        .map(|pinned_id| {
            let display_name = state
                .formula_spaces
                .get(pinned_id)
                .map(|space| space.context.scenario_label.clone())
                .or_else(|| {
                    state
                        .workspace_shell
                        .recent_formula_spaces
                        .get(pinned_id)
                        .map(|record| record.formula_space.context.scenario_label.clone())
                })
                .unwrap_or_else(|| pinned_id.as_str().to_string());
            let synthetic_default = display_name == pinned_id.as_str();
            ScenarioBreadcrumbEntry {
                formula_space_id: pinned_id.as_str().to_string(),
                display_name: if synthetic_default {
                    "unsaved".to_string()
                } else {
                    display_name
                },
                meta: "pinned".to_string(),
                is_active: pinned_id == &active_id,
                is_pinned: true,
            }
        })
        .collect();

    // User-facing labels say "formula" (per docs/APP_UX_BRIEF.md
    // §1A); internal type / action-id slugs continue to say
    // `scenario`.
    //
    // Seam policy: NewScenario / Duplicate use existing in-memory
    // reducers; SaveAs and Open are wired through the browser-host
    // download / file-input adapter (slice 1b — see
    // `persistence/browser_file_io.rs`). ManageScenarios still
    // pends its full-screen page UI, hence the SEAM. Tauri-host
    // file IO is a separate later slice; the per-host adapter
    // lives below the action layer, so the breadcrumb sees the
    // same dispatch surface either way.
    // Browser-host save uses the browser's download dialog, not a
    // file-overwrite save; users see a download confirmation, not a
    // "save back to the same path" flow. The label says "Download
    // Formula File" on wasm so the action's effect matches the
    // user's mental model. On the desktop (Tauri) host the action
    // is a real "Save as…" with a native file picker, which is the
    // label there.
    #[cfg(target_arch = "wasm32")]
    let save_as_label = "Download formula XML";
    #[cfg(not(target_arch = "wasm32"))]
    let save_as_label = "Save as…";

    // Pin / Unpin is conditional on whether the active formula is
    // already pinned. Pinning is workspace-level (survives close);
    // pinned ids persist into `workspace.json` once persistence
    // ships. Until then the in-memory toggle is the visible
    // affordance.
    let active_is_pinned = state
        .workspace_shell
        .active_formula_space_id
        .as_ref()
        .is_some_and(|id| state.workspace_shell.pinned_formula_space_ids.contains(id));
    let pin_action = if active_is_pinned {
        ScenarioBreadcrumbAction {
            action_id: ScenarioBreadcrumbActionId::UnpinActive,
            label: "Unpin",
            chord_label: "",
            seam_id: None,
        }
    } else {
        ScenarioBreadcrumbAction {
            action_id: ScenarioBreadcrumbActionId::PinActive,
            label: "Pin",
            chord_label: "",
            seam_id: None,
        }
    };
    let actions = vec![
        ScenarioBreadcrumbAction {
            action_id: ScenarioBreadcrumbActionId::NewScenario,
            label: "New formula",
            chord_label: "Ctrl+N",
            seam_id: None,
        },
        ScenarioBreadcrumbAction {
            action_id: ScenarioBreadcrumbActionId::SaveAs,
            label: save_as_label,
            chord_label: "Ctrl+Shift+S",
            seam_id: None,
        },
        ScenarioBreadcrumbAction {
            action_id: ScenarioBreadcrumbActionId::Open,
            label: "Open…",
            chord_label: "Ctrl+O",
            seam_id: None,
        },
        ScenarioBreadcrumbAction {
            action_id: ScenarioBreadcrumbActionId::Duplicate,
            // User-visible label per WS-14 §1; the action id keeps
            // the legacy `Duplicate` for stability.
            label: "Clone",
            chord_label: "",
            seam_id: None,
        },
        ScenarioBreadcrumbAction {
            action_id: ScenarioBreadcrumbActionId::RenameActive,
            label: "Rename…",
            chord_label: "",
            seam_id: None,
        },
        pin_action,
        ScenarioBreadcrumbAction {
            action_id: ScenarioBreadcrumbActionId::ManageScenarios,
            label: "Manage formulas…",
            chord_label: "",
            // The manage-formulas overlay is the v1 surface — no
            // longer a SEAM stub. Bulk operations + drag-reorder
            // are open follow-ups but the row-by-row actions and
            // search are functional today.
            seam_id: None,
        },
    ];

    ScenarioBreadcrumbView {
        active_label,
        is_dirty,
        is_open: state.global_ui_chrome.scenario_breadcrumb_open,
        recent,
        pinned,
        actions,
    }
}

fn project_status_view(formula_space: &FormulaSpaceState) -> StatusView {
    let truth_source = formula_space.context.truth_source.clone();
    let green_tree_key = formula_space
        .editor_document
        .as_ref()
        .map(|document| document.editor_syntax_snapshot.green_tree_key.clone())
        .filter(|key| !key.is_empty());

    let bridge_health = match (&truth_source, &green_tree_key) {
        (ProjectionTruthSource::LiveBacked, Some(_)) => BridgeHealth::Live,
        _ => BridgeHealth::Stale,
    };

    let synthetic_default_label =
        formula_space.context.scenario_label == formula_space.formula_space_id.as_str();
    let scenario_label = if synthetic_default_label {
        "unsaved".to_string()
    } else {
        formula_space.context.scenario_label.clone()
    };

    StatusView {
        bridge_health,
        truth_source,
        green_tree_key,
        scenario_label,
        load_diagnostics: formula_space.load_diagnostics.clone(),
    }
}

fn project_skin_snapshot(
    formula_space: &FormulaSpaceState,
    state: &OneCalcHostState,
    entry_mode_pill: &EntryModePill,
    syntax_runs: &[SyntaxRun],
    diagnostic_squiggles: &[DiagnosticSquiggle],
    editor_metrics: &EditorMetricsChip,
    completion_popup: Option<&CompletionPopupView>,
    signature_help: Option<&SignatureHelpView>,
    function_help_card: Option<&FunctionHelpCardView>,
    result_view: &ResultView,
    formatting_controls: &FormattingControlsView,
    formula_drill: &FormulaDrillView,
    status: &StatusView,
) -> dnacalc_skin_ir::SkinSnapshot {
    let formula_space_id = formula_space.formula_space_id.as_str().to_string();
    let formula_stable_id = formula_space
        .editor_document
        .as_ref()
        .map(|document| document.editor_syntax_snapshot.formula_stable_id.clone())
        .filter(|id| !id.is_empty())
        .unwrap_or_else(|| formula_space_id.clone());
    let display_name = status.scenario_label.clone();
    let formula = dnacalc_skin_ir::OneFormulaProjection {
        formula_space_id: formula_space_id.clone(),
        formula_stable_id,
        display_name: display_name.clone(),
        raw_entered_cell_text: formula_space.raw_entered_cell_text.clone(),
        entry_mode: project_skin_entry_mode(*entry_mode_pill),
        editor: dnacalc_skin_ir::FormulaEditorSurface {
            source_text: formula_space.raw_entered_cell_text.clone(),
            caret_offset: formula_space.editor_surface_state.caret.offset,
            selection_anchor: formula_space.editor_surface_state.selection.anchor,
            selection_focus: formula_space.editor_surface_state.selection.focus,
            syntax_runs: syntax_runs.iter().map(project_skin_syntax_run).collect(),
            diagnostics: diagnostic_squiggles
                .iter()
                .map(project_skin_diagnostic_from_squiggle)
                .collect(),
            metrics: dnacalc_skin_ir::EditorMetricsProjection {
                token_count: editor_metrics.token_count,
                function_count: editor_metrics.function_count,
                diagnostic_count: editor_metrics.diagnostic_count,
                first_diagnostic_message: editor_metrics.first_diagnostic_message.clone(),
            },
            document_is_fresh: editor_document_is_fresh(formula_space),
        },
        assist: dnacalc_skin_ir::FormulaAssistSurface {
            completion: completion_popup.map(project_skin_completion),
            signature_help: signature_help.map(project_skin_signature_help),
            function_help: function_help_card.map(project_skin_function_help),
        },
        result: project_skin_result_surface(formula_space, result_view),
        comparison: dnacalc_skin_ir::ComparisonSurface::default(),
        formatting: project_skin_formatting_surface(formatting_controls),
        drill: project_skin_drill_surface(formula_drill),
        status: project_skin_status_surface(status),
    };

    dnacalc_skin_ir::SkinSnapshot::one_formula(
        dnacalc_skin_ir::SkinShellProjection {
            host_kind: dnacalc_skin_ir::HostKindProjection::OneCalc,
            title: display_name,
            active_document_id: Some(formula_space_id),
            status_text: Some(
                match status.bridge_health {
                    BridgeHealth::Live => "live",
                    BridgeHealth::Stale => "stale",
                }
                .to_string(),
            ),
            command_palette_open: state.global_ui_chrome.command_palette_open,
            persistence: dnacalc_skin_ir::PersistenceProjection {
                can_save: !cfg!(target_arch = "wasm32"),
                can_open: !cfg!(target_arch = "wasm32"),
                dirty: matches!(
                    formula_space.live_state(),
                    crate::ui::editor::state::EditorLiveState::EditingLive
                        | crate::ui::editor::state::EditorLiveState::ProofedScratch
                ),
                current_path: state.workspace_shell.current_workspace_path.clone(),
                recent_documents: state
                    .workspace_shell
                    .recent_formula_space_order
                    .iter()
                    .filter_map(|id| {
                        state
                            .workspace_shell
                            .recent_formula_spaces
                            .get(id)
                            .map(|record| (id, record))
                    })
                    .map(|(id, record)| dnacalc_skin_ir::RecentDocumentProjection {
                        document_id: id.as_str().to_string(),
                        display_name: record.formula_space.context.scenario_label.clone(),
                        path: None,
                        last_opened_unix_ms: None,
                        available: true,
                    })
                    .collect(),
            },
        },
        dnacalc_skin_ir::HostCapabilityProjection::onecalc_null_references(
            state.runtime_profile_override.unwrap_or_else(|| {
                if cfg!(target_arch = "wasm32") {
                    dnacalc_skin_ir::RuntimeProfileProjection::BrowserWasm
                } else if cfg!(target_os = "windows") {
                    dnacalc_skin_ir::RuntimeProfileProjection::WindowsDesktop
                } else {
                    dnacalc_skin_ir::RuntimeProfileProjection::NativeUnix
                }
            }),
            dnacalc_skin_ir::ExtensionPlacementProjection::Unavailable,
        ),
        formula,
    )
}

fn editor_document_is_fresh(formula_space: &FormulaSpaceState) -> bool {
    formula_space
        .editor_document
        .as_ref()
        .is_some_and(|document| document.source_text == formula_space.raw_entered_cell_text)
}

fn project_skin_entry_mode(mode: EntryModePill) -> dnacalc_skin_ir::FormulaEntryModeProjection {
    match mode {
        EntryModePill::Formula => dnacalc_skin_ir::FormulaEntryModeProjection::Formula,
        EntryModePill::Value => dnacalc_skin_ir::FormulaEntryModeProjection::Value,
        EntryModePill::Text => dnacalc_skin_ir::FormulaEntryModeProjection::Text,
        EntryModePill::Empty => dnacalc_skin_ir::FormulaEntryModeProjection::Empty,
    }
}

fn project_skin_syntax_run(run: &SyntaxRun) -> dnacalc_skin_ir::SyntaxRunProjection {
    dnacalc_skin_ir::SyntaxRunProjection {
        text: run.text.clone(),
        span_start: run.span_start,
        span_len: run.span_len,
        role: match run.role {
            SyntaxTokenRole::Operator => dnacalc_skin_ir::SyntaxTokenRoleProjection::Operator,
            SyntaxTokenRole::Function => dnacalc_skin_ir::SyntaxTokenRoleProjection::Function,
            SyntaxTokenRole::Number => dnacalc_skin_ir::SyntaxTokenRoleProjection::Number,
            SyntaxTokenRole::Delimiter => dnacalc_skin_ir::SyntaxTokenRoleProjection::Delimiter,
            SyntaxTokenRole::Identifier => dnacalc_skin_ir::SyntaxTokenRoleProjection::Identifier,
            SyntaxTokenRole::Text => dnacalc_skin_ir::SyntaxTokenRoleProjection::Text,
            SyntaxTokenRole::Trivia => dnacalc_skin_ir::SyntaxTokenRoleProjection::Trivia,
        },
    }
}

fn project_skin_diagnostic_from_squiggle(
    diagnostic: &DiagnosticSquiggle,
) -> dnacalc_skin_ir::FormulaDiagnosticProjection {
    dnacalc_skin_ir::FormulaDiagnosticProjection {
        diagnostic_id: diagnostic.diagnostic_id.clone(),
        severity: project_skin_severity(diagnostic.severity),
        stage: project_skin_stage(diagnostic.stage),
        code: diagnostic.code.clone(),
        worksheet_error_class: diagnostic.worksheet_error_class.clone(),
        message: diagnostic.message.clone(),
        span_start: diagnostic.span_start,
        span_len: diagnostic.span_len,
    }
}

fn project_skin_diagnostic_from_drill(
    diagnostic: &FormulaDrillDiagnosticRow,
) -> dnacalc_skin_ir::FormulaDiagnosticProjection {
    dnacalc_skin_ir::FormulaDiagnosticProjection {
        diagnostic_id: diagnostic.diagnostic_id.clone(),
        severity: project_skin_severity(diagnostic.severity),
        stage: project_skin_stage(diagnostic.stage),
        code: diagnostic.code.clone(),
        worksheet_error_class: None,
        message: diagnostic.message.clone(),
        span_start: diagnostic.span_start,
        span_len: diagnostic.span_len,
    }
}

fn project_skin_severity(
    severity: SquiggleSeverity,
) -> dnacalc_skin_ir::DiagnosticSeverityProjection {
    match severity {
        SquiggleSeverity::Error => dnacalc_skin_ir::DiagnosticSeverityProjection::Error,
        SquiggleSeverity::Warning => dnacalc_skin_ir::DiagnosticSeverityProjection::Warning,
        SquiggleSeverity::Info => dnacalc_skin_ir::DiagnosticSeverityProjection::Info,
    }
}

fn project_skin_stage(stage: DiagnosticStage) -> dnacalc_skin_ir::DiagnosticStageProjection {
    match stage {
        DiagnosticStage::Syntax => dnacalc_skin_ir::DiagnosticStageProjection::Syntax,
        DiagnosticStage::Bind => dnacalc_skin_ir::DiagnosticStageProjection::Bind,
        DiagnosticStage::SemanticPlan => dnacalc_skin_ir::DiagnosticStageProjection::SemanticPlan,
    }
}

fn project_skin_completion(view: &CompletionPopupView) -> dnacalc_skin_ir::CompletionSurface {
    dnacalc_skin_ir::CompletionSurface {
        anchor_left_px: view.anchor_left_px,
        anchor_top_px: view.anchor_top_px,
        line_height_px: view.line_height_px,
        selected_index: view.selected_index,
        items: view
            .items
            .iter()
            .map(|item| dnacalc_skin_ir::CompletionItemProjection {
                proposal_id: item.proposal_id.clone(),
                display_text: item.display_text.clone(),
                kind: project_skin_completion_kind(item.kind_label),
                documentation_ref: item.documentation_ref.clone(),
            })
            .collect(),
    }
}

fn project_skin_completion_kind(label: &str) -> dnacalc_skin_ir::CompletionKindProjection {
    match label {
        "Function" => dnacalc_skin_ir::CompletionKindProjection::Function,
        "Defined name" => dnacalc_skin_ir::CompletionKindProjection::DefinedName,
        "Table" => dnacalc_skin_ir::CompletionKindProjection::TableName,
        "Column" => dnacalc_skin_ir::CompletionKindProjection::TableColumn,
        "Selector" => dnacalc_skin_ir::CompletionKindProjection::StructuredSelector,
        "Reference" => dnacalc_skin_ir::CompletionKindProjection::ProfileReference,
        _ => dnacalc_skin_ir::CompletionKindProjection::SyntaxAssist,
    }
}

fn project_skin_signature_help(view: &SignatureHelpView) -> dnacalc_skin_ir::SignatureHelpSurface {
    dnacalc_skin_ir::SignatureHelpSurface {
        callee_text: view.callee_text.clone(),
        anchor_left_px: view.anchor_left_px,
        anchor_top_px: view.anchor_top_px,
        line_height_px: view.line_height_px,
        parameters: view
            .parameters
            .iter()
            .map(
                |parameter| dnacalc_skin_ir::SignatureHelpParameterProjection {
                    name: parameter.name.clone(),
                    is_active: parameter.is_active,
                },
            )
            .collect(),
        active_parameter: view.active_parameter,
    }
}

fn project_skin_function_help(view: &FunctionHelpCardView) -> dnacalc_skin_ir::FunctionHelpSurface {
    dnacalc_skin_ir::FunctionHelpSurface {
        lookup_key: view.lookup_key.clone(),
        display_name: view.display_name.clone(),
        signature: view.signature.clone(),
        short_description: view.short_description.clone(),
        availability_summary: view.availability_summary.clone(),
        deferred_or_profile_limited: view.deferred_or_profile_limited,
    }
}

fn project_skin_result_surface(
    formula_space: &FormulaSpaceState,
    view: &ResultView,
) -> dnacalc_skin_ir::FormulaResultSurface {
    match view {
        ResultView::Empty => dnacalc_skin_ir::FormulaResultSurface::Empty,
        ResultView::Pending => dnacalc_skin_ir::FormulaResultSurface::Pending,
        ResultView::Error { code, surface_repr } => dnacalc_skin_ir::FormulaResultSurface::Error {
            code: code.clone(),
            surface_repr: surface_repr.clone(),
        },
        ResultView::Display {
            text,
            kind,
            applied_font_color,
            applied_fill_color,
        } => dnacalc_skin_ir::FormulaResultSurface::Display {
            text: text.clone(),
            value: bridge_published_value(formula_space)
                .map(|value| project_skin_calc_value(value, text.clone()))
                .unwrap_or_else(|| project_skin_display_value(*kind, text.clone())),
            applied_font_color: applied_font_color.clone(),
            applied_fill_color: applied_fill_color.clone(),
        },
        ResultView::Array {
            total_rows,
            total_cols,
            label,
            cells,
            cell_format,
            truncated,
        } => dnacalc_skin_ir::FormulaResultSurface::Array {
            total_rows: *total_rows,
            total_cols: *total_cols,
            label: label.clone(),
            window: project_skin_array_window(
                *total_rows,
                *total_cols,
                cells,
                cell_format.as_ref(),
            ),
            truncated: *truncated,
        },
    }
}

fn project_skin_display_value(
    kind: ResultKind,
    display_text: String,
) -> dnacalc_skin_ir::CalcValueProjection {
    let core = match kind {
        ResultKind::Number => dnacalc_skin_ir::CoreValueProjection::Number {
            raw: display_text.clone(),
        },
        ResultKind::Text => dnacalc_skin_ir::CoreValueProjection::Text {
            text: display_text.clone(),
        },
        ResultKind::Logical => dnacalc_skin_ir::CoreValueProjection::Logical {
            value: display_text.eq_ignore_ascii_case("true"),
        },
        ResultKind::RichValue => dnacalc_skin_ir::CoreValueProjection::RichValue {
            summary: display_text.clone(),
        },
        ResultKind::Other => dnacalc_skin_ir::CoreValueProjection::Other {
            summary: display_text.clone(),
        },
    };
    dnacalc_skin_ir::CalcValueProjection {
        core,
        display_text,
        presentation_hint: None,
        rich_value_kind: None,
        callable: false,
    }
}

fn project_skin_calc_value(
    value: &CalcValue,
    display_text: String,
) -> dnacalc_skin_ir::CalcValueProjection {
    let mut projected = dnacalc_formula_ux_core::project_calc_value(value);
    // Effective display is host policy (number format and conditional-format
    // context); the typed value/presentation projection remains shared.
    projected.display_text = display_text;
    projected
}

fn project_skin_array_window(
    total_rows: usize,
    total_cols: usize,
    cells: &[Vec<String>],
    cell_format: Option<&Vec<Vec<ArrayCellFormatView>>>,
) -> dnacalc_skin_ir::ArrayWindowProjection {
    dnacalc_skin_ir::ArrayWindowProjection {
        total_rows,
        total_cols,
        row_offset: 0,
        col_offset: 0,
        cells: cells
            .iter()
            .enumerate()
            .map(|(row_index, row)| {
                row.iter()
                    .enumerate()
                    .map(
                        |(col_index, display_text)| dnacalc_skin_ir::ArrayWindowCellProjection {
                            display_text: display_text.clone(),
                            value: Some(dnacalc_skin_ir::CalcValueProjection {
                                core: dnacalc_skin_ir::CoreValueProjection::Text {
                                    text: display_text.clone(),
                                },
                                display_text: display_text.clone(),
                                presentation_hint: None,
                                rich_value_kind: None,
                                callable: false,
                            }),
                            format: cell_format
                                .and_then(|grid| grid.get(row_index))
                                .and_then(|row| row.get(col_index))
                                .map(project_skin_array_cell_format),
                        },
                    )
                    .collect()
            })
            .collect(),
    }
}

fn project_skin_array_cell_format(
    format: &ArrayCellFormatView,
) -> dnacalc_skin_ir::ArrayCellFormatProjection {
    dnacalc_skin_ir::ArrayCellFormatProjection {
        effective_font_color: format.effective_font_color.clone(),
        effective_fill_color: format.effective_fill_color.clone(),
        data_bar: format
            .data_bar
            .as_ref()
            .map(|bar| dnacalc_skin_ir::DataBarFillProjection {
                fill_ratio: bar.fill_ratio,
                bar_color: bar.bar_color.clone(),
                direction: match bar.direction {
                    DataBarDirectionView::Left => dnacalc_skin_ir::DataBarDirectionProjection::Left,
                    DataBarDirectionView::Right => {
                        dnacalc_skin_ir::DataBarDirectionProjection::Right
                    }
                },
                show_bar_only: bar.show_bar_only,
            }),
        icon: format
            .icon
            .as_ref()
            .map(|icon| dnacalc_skin_ir::CfIconProjection {
                set_kind: icon.set_kind.clone(),
                icon_index: icon.icon_index,
            }),
    }
}

fn project_skin_formatting_surface(
    view: &FormattingControlsView,
) -> dnacalc_skin_ir::FormattingSurface {
    dnacalc_skin_ir::FormattingSurface {
        number_format_code: non_empty_string(view.number_format_code.clone()),
        font_color: non_empty_string(view.font_color.clone()),
        fill_color: non_empty_string(view.fill_color.clone()),
        date1904: view.date1904,
        locale_language_tag: view.locale_language_tag.clone(),
        scenario_policy: match view.scenario_policy {
            ScenarioPolicyView::Deterministic => {
                dnacalc_skin_ir::ScenarioPolicyProjection::Deterministic
            }
            ScenarioPolicyView::LiveRecalc => dnacalc_skin_ir::ScenarioPolicyProjection::LiveRecalc,
            ScenarioPolicyView::ManualRecalc => {
                dnacalc_skin_ir::ScenarioPolicyProjection::ManualRecalc
            }
        },
        conditional_formatting_rules: view
            .conditional_formatting_rules
            .iter()
            .map(project_skin_cf_rule)
            .collect(),
    }
}

fn non_empty_string(value: String) -> Option<String> {
    (!value.is_empty()).then_some(value)
}

fn project_skin_cf_rule(
    rule: &ConditionalFormattingRuleView,
) -> dnacalc_skin_ir::ConditionalFormattingRuleProjection {
    dnacalc_skin_ir::ConditionalFormattingRuleProjection {
        operator: rule.operator.clone(),
        thresholds: rule
            .thresholds
            .iter()
            .cloned()
            .map(dnacalc_skin_ir::ConditionalFormattingThresholdProjection::Text)
            .collect(),
        font_color: rule.font_color.clone(),
        fill_color: rule.fill_color.clone(),
        typed_rule: rule
            .typed_rule
            .as_ref()
            .and_then(project_skin_cf_typed_rule),
    }
}

fn project_skin_cf_typed_rule(
    rule: &crate::state::FormulaConditionalFormattingTypedRule,
) -> Option<dnacalc_skin_ir::ConditionalFormattingTypedRuleProjection> {
    if let Some(options) = &rule.color_scale {
        return Some(
            dnacalc_skin_ir::ConditionalFormattingTypedRuleProjection::ColorScale(
                dnacalc_skin_ir::ColorScaleRuleProjection {
                    stops: options
                        .stops
                        .iter()
                        .map(|stop| dnacalc_skin_ir::ColorScaleStopProjection {
                            position: project_skin_cf_threshold(&stop.position),
                            color: stop.color.clone(),
                        })
                        .collect(),
                },
            ),
        );
    }
    if let Some(options) = &rule.data_bar {
        return Some(
            dnacalc_skin_ir::ConditionalFormattingTypedRuleProjection::DataBar(
                dnacalc_skin_ir::DataBarRuleProjection {
                    minimum: options.minimum.as_ref().map(project_skin_cf_threshold),
                    maximum: options.maximum.as_ref().map(project_skin_cf_threshold),
                    bar_color: options.bar_color.clone(),
                    direction: options.direction.map(|direction| match direction {
                        crate::state::FormulaDataBarDirection::Left => {
                            dnacalc_skin_ir::DataBarDirectionProjection::Left
                        }
                        crate::state::FormulaDataBarDirection::Right => {
                            dnacalc_skin_ir::DataBarDirectionProjection::Right
                        }
                    }),
                    show_bar_only: options.show_bar_only,
                },
            ),
        );
    }
    if let Some(options) = &rule.icon_set {
        return Some(
            dnacalc_skin_ir::ConditionalFormattingTypedRuleProjection::IconSet(
                dnacalc_skin_ir::IconSetRuleProjection {
                    set_kind: options.set_kind.clone(),
                    thresholds: options
                        .thresholds
                        .iter()
                        .map(project_skin_cf_threshold)
                        .collect(),
                },
            ),
        );
    }
    if let Some(options) = &rule.rank {
        return Some(
            dnacalc_skin_ir::ConditionalFormattingTypedRuleProjection::Rank(match options.rank {
                crate::state::FormulaConditionalFormattingRank::Count(count) => {
                    dnacalc_skin_ir::RankRuleProjection::Count(count)
                }
                crate::state::FormulaConditionalFormattingRank::Percent(percent) => {
                    dnacalc_skin_ir::RankRuleProjection::Percent(percent)
                }
            }),
        );
    }
    rule.average.as_ref().map(|options| {
        dnacalc_skin_ir::ConditionalFormattingTypedRuleProjection::Average(
            dnacalc_skin_ir::AverageRuleProjection {
                include_equal: options.include_equal,
                stddev_multiplier: options.stddev_multiplier,
            },
        )
    })
}

fn project_skin_cf_threshold(
    threshold: &crate::state::FormulaConditionalFormattingThreshold,
) -> dnacalc_skin_ir::ConditionalFormattingThresholdProjection {
    match threshold {
        crate::state::FormulaConditionalFormattingThreshold::Min => {
            dnacalc_skin_ir::ConditionalFormattingThresholdProjection::Min
        }
        crate::state::FormulaConditionalFormattingThreshold::Mid => {
            dnacalc_skin_ir::ConditionalFormattingThresholdProjection::Mid
        }
        crate::state::FormulaConditionalFormattingThreshold::Max => {
            dnacalc_skin_ir::ConditionalFormattingThresholdProjection::Max
        }
        crate::state::FormulaConditionalFormattingThreshold::Percent(value) => {
            dnacalc_skin_ir::ConditionalFormattingThresholdProjection::Percent(*value)
        }
        crate::state::FormulaConditionalFormattingThreshold::Percentile(value) => {
            dnacalc_skin_ir::ConditionalFormattingThresholdProjection::Percentile(*value)
        }
        crate::state::FormulaConditionalFormattingThreshold::Number(value) => {
            dnacalc_skin_ir::ConditionalFormattingThresholdProjection::Number(*value)
        }
    }
}

fn project_skin_drill_surface(view: &FormulaDrillView) -> dnacalc_skin_ir::FormulaDrillSurface {
    dnacalc_skin_ir::FormulaDrillSurface {
        expanded: view.expanded,
        tree: view.tree.iter().map(project_skin_drill_node).collect(),
        diagnostics: view
            .diagnostics
            .iter()
            .map(project_skin_diagnostic_from_drill)
            .collect(),
        phase_summaries: view
            .phase_summaries
            .iter()
            .map(|phase| dnacalc_skin_ir::FormulaDrillPhaseProjection {
                label: phase.label.to_string(),
                detail: phase.detail.clone(),
                state: match phase.state {
                    FormulaDrillPhaseState::Ok => {
                        dnacalc_skin_ir::FormulaDrillPhaseStateProjection::Ok
                    }
                    FormulaDrillPhaseState::Pending => {
                        dnacalc_skin_ir::FormulaDrillPhaseStateProjection::Pending
                    }
                    FormulaDrillPhaseState::Blocked => {
                        dnacalc_skin_ir::FormulaDrillPhaseStateProjection::Blocked
                    }
                },
            })
            .collect(),
        document_is_fresh: view.document_is_fresh,
    }
}

fn project_skin_drill_node(node: &FormulaDrillNode) -> dnacalc_skin_ir::FormulaDrillNodeProjection {
    dnacalc_skin_ir::FormulaDrillNodeProjection {
        node_id: node.node_id.clone(),
        label: node.label.clone(),
        developer_label: node.developer_label.clone(),
        expression_text: node.expression_text.clone(),
        kind: node.kind.clone(),
        source_span_start: node.source_span_start,
        source_span_len: node.source_span_len,
        branch_disposition: node.branch_disposition.clone(),
        argument_name: node.argument_name.clone(),
        argument_role: node.argument_role.clone(),
        error_message: node.error_message.clone(),
        value_preview: node.value_preview.clone(),
        array_preview: node.array_preview.as_ref().map(|preview| {
            dnacalc_skin_ir::ArrayPreviewProjection {
                row_offset: 0,
                col_offset: 0,
                total_rows: preview.total_rows,
                total_cols: preview.total_cols,
                rows: preview.rows.clone(),
                truncated: preview.truncated,
            }
        }),
        state: match node.state {
            crate::adapters::oxfml::FormulaDrillNodeState::Pending => {
                dnacalc_skin_ir::FormulaDrillNodeStateProjection::Pending
            }
            crate::adapters::oxfml::FormulaDrillNodeState::Evaluated => {
                dnacalc_skin_ir::FormulaDrillNodeStateProjection::Evaluated
            }
            crate::adapters::oxfml::FormulaDrillNodeState::Bound => {
                dnacalc_skin_ir::FormulaDrillNodeStateProjection::Bound
            }
            crate::adapters::oxfml::FormulaDrillNodeState::Skipped => {
                dnacalc_skin_ir::FormulaDrillNodeStateProjection::Skipped
            }
            crate::adapters::oxfml::FormulaDrillNodeState::Opaque => {
                dnacalc_skin_ir::FormulaDrillNodeStateProjection::Opaque
            }
            crate::adapters::oxfml::FormulaDrillNodeState::Blocked => {
                dnacalc_skin_ir::FormulaDrillNodeStateProjection::Blocked
            }
            crate::adapters::oxfml::FormulaDrillNodeState::Error => {
                dnacalc_skin_ir::FormulaDrillNodeStateProjection::Error
            }
        },
        children: node.children.iter().map(project_skin_drill_node).collect(),
    }
}

fn project_skin_status_surface(status: &StatusView) -> dnacalc_skin_ir::FormulaStatusSurface {
    dnacalc_skin_ir::FormulaStatusSurface {
        bridge_health: match status.bridge_health {
            BridgeHealth::Live => dnacalc_skin_ir::BridgeHealthProjection::Live,
            BridgeHealth::Stale => dnacalc_skin_ir::BridgeHealthProjection::Stale,
        },
        truth_source: status.truth_source.label().to_string(),
        green_tree_key: status.green_tree_key.clone(),
        scenario_label: status.scenario_label.clone(),
        load_diagnostics: status
            .load_diagnostics
            .iter()
            .map(|diagnostic| format!("{diagnostic:?}"))
            .collect(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::oxfml::{FormulaResultViewModel, ProvenanceSummary};
    use crate::domain::ids::FormulaSpaceId;
    use crate::state::{
        AppMode, ClosedFormulaSpaceRecord, FormulaArrayPreviewState, FormulaSpaceState,
        VbaHostAssociationLoadStatus, VbaHostAssociationSourceKind, VbaHostAssociationState,
    };
    use crate::test_support::{
        array_editor_document, blocked_editor_document, diagnostic_editor_document,
        sample_editor_document,
    };

    /// Build a one-formula-space host state with the given (optional)
    /// editor_document attached. Helper for the test cases below.
    fn host_state_with(formula_space: FormulaSpaceState) -> OneCalcHostState {
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id =
            Some(formula_space.formula_space_id.clone());
        state
            .workspace_shell
            .open_formula_space_order
            .push(formula_space.formula_space_id.clone());
        state.formula_spaces.insert(formula_space);
        state
    }

    /// Attach a typed Number `value_presentation` to the document so the
    /// home view-model dispatches via the typed path.
    fn attach_number_value_presentation(
        document: &mut crate::adapters::oxfml::EditorDocument,
        number: f64,
        display: &str,
    ) {
        document.value_presentation = Some(FormulaResultViewModel {
            evaluation_summary: format!("Number · {display}"),
            effective_display_summary: Some(display.to_string()),
            array_preview: None,
            blocked_reason: None,
            published_value: CalcValue::number(number),
            number_format_hint: None,
            effective_font_color: None,
            effective_fill_color: None,
            array_cell_format: None,
            semantic_kernel_metadata_version: None,
            arg_admission_metadata_version: None,
            producer_capability_set_keys: Vec::new(),
            exercised_capability_keys: Vec::new(),
        });
    }

    #[test]
    fn view_model_projects_vba_host_context_associations() {
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("space-1"), "=AddThem(2,3)");
        let mut state = host_state_with(formula_space);
        state.vba_host_context.pending_project_path = "fixtures/vba_udf/multi_module".to_string();
        state
            .vba_host_context
            .associations
            .push(VbaHostAssociationState {
                association_id: "vba-assoc-1".to_string(),
                display_name: "MathOne".to_string(),
                source_ref: "fixtures/vba_udf/multi_module".to_string(),
                source_kind: VbaHostAssociationSourceKind::ProjectPath,
                enabled: true,
                load_status: VbaHostAssociationLoadStatus::Loaded,
                admitted_udf_count: 2,
                rejected_candidate_count: 1,
                admitted_udfs: vec!["AddThem".to_string(), "MultiplyThree".to_string()],
                rejected_candidates: vec!["EchoText".to_string()],
            });

        let vm = build_home_shell_view_model(&state).expect("active formula space");

        assert_eq!(
            vm.vba_host_context.pending_project_path,
            "fixtures/vba_udf/multi_module"
        );
        assert_eq!(
            vm.vba_host_context.summary,
            "1 source(s) · 2 UDF(s) · 1 rejected"
        );
        assert_eq!(vm.vba_host_context.associations[0].source_kind, "project");
        assert_eq!(
            vm.vba_host_context.associations[0].admitted_udfs,
            vec!["AddThem".to_string(), "MultiplyThree".to_string()]
        );
    }

    #[test]
    fn view_model_projects_browser_vba_file_without_raw_source_prefix() {
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("space-1"), "=AddThem(2,3)");
        let mut state = host_state_with(formula_space);
        state
            .vba_host_context
            .associations
            .push(VbaHostAssociationState {
                association_id: "vba-assoc-1".to_string(),
                display_name: "SimpleVba.bas".to_string(),
                source_ref: "browser-file:SimpleVba.bas".to_string(),
                source_kind: VbaHostAssociationSourceKind::ModuleSource,
                enabled: true,
                load_status: VbaHostAssociationLoadStatus::Loaded,
                admitted_udf_count: 1,
                rejected_candidate_count: 0,
                admitted_udfs: vec!["AddThem".to_string()],
                rejected_candidates: Vec::new(),
            });

        let vm = build_home_shell_view_model(&state).expect("active formula space");
        let association = &vm.vba_host_context.associations[0];

        assert_eq!(association.source_kind, "browser .bas");
        assert_eq!(association.source_ref, "SimpleVba.bas");
        assert_eq!(association.status_label, "loaded");
    }

    #[test]
    fn returns_none_when_no_active_formula_space() {
        let state = OneCalcHostState::default();
        assert!(build_home_shell_view_model(&state).is_none());
    }

    #[test]
    fn empty_text_projects_to_result_view_empty() {
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert_eq!(vm.raw_entered_cell_text, "");
        assert_eq!(vm.result_view, ResultView::Empty);
    }

    #[test]
    fn happy_sum_projects_to_result_view_display_number() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2)");
        let mut document = sample_editor_document("=SUM(1,2)");
        attach_number_value_presentation(&mut document, 3.0, "3");
        formula_space.editor_document = Some(document);
        formula_space.effective_display_summary = Some("3".to_string());
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        match vm.result_view {
            ResultView::Display { text, kind, .. } => {
                assert_eq!(text, "3");
                assert_eq!(kind, ResultKind::Number);
            }
            other => panic!("expected Display(Number, '3'), got {other:?}"),
        }
    }

    #[test]
    fn happy_sum_projects_to_shared_one_formula_skin_snapshot() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2)");
        let mut document = sample_editor_document("=SUM(1,2)");
        attach_number_value_presentation(&mut document, 3.0, "3");
        formula_space.editor_document = Some(document);
        formula_space.effective_display_summary = Some("3".to_string());
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");

        assert_eq!(
            vm.skin_snapshot.host_capabilities.references,
            dnacalc_skin_ir::ReferenceCapabilityProjection::Absent
        );
        match &vm.skin_snapshot.document {
            dnacalc_skin_ir::SkinDocumentProjection::OneFormula(formula) => {
                assert_eq!(formula.raw_entered_cell_text, "=SUM(1,2)");
                assert_eq!(
                    formula.entry_mode,
                    dnacalc_skin_ir::FormulaEntryModeProjection::Formula
                );
                match &formula.result {
                    dnacalc_skin_ir::FormulaResultSurface::Display { text, value, .. } => {
                        assert_eq!(text, "3");
                        assert!(matches!(
                            value.core,
                            dnacalc_skin_ir::CoreValueProjection::Number { .. }
                        ));
                    }
                    other => panic!("expected shared display result, got {other:?}"),
                }
            }
            other => panic!("expected OneFormula snapshot, got {other:?}"),
        }

        let json = serde_json::to_string(&vm.skin_snapshot).expect("serialize skin snapshot");
        let restored: dnacalc_skin_ir::SkinSnapshot =
            serde_json::from_str(&json).expect("deserialize skin snapshot");
        assert_eq!(restored, vm.skin_snapshot);
    }

    #[test]
    fn diagnostic_in_editor_document_projects_to_result_view_error() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,)");
        formula_space.editor_document = Some(diagnostic_editor_document("=SUM(1,)"));
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        match vm.result_view {
            ResultView::Error { code, surface_repr } => {
                assert_eq!(code, "DIAGNOSTIC");
                assert_eq!(surface_repr.as_deref(), Some("Missing trailing argument"));
            }
            other => panic!("expected Error(DIAGNOSTIC, ...), got {other:?}"),
        }
    }

    #[test]
    fn host_blocked_reason_projects_to_result_view_error() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=XLOOKUP(...)");
        formula_space.editor_document = Some(blocked_editor_document("=XLOOKUP(...)"));
        formula_space.context.blocked_reason =
            Some("XLOOKUP not admitted on this host".to_string());
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        match vm.result_view {
            ResultView::Error { code, surface_repr } => {
                assert_eq!(code, "BLOCKED");
                assert_eq!(
                    surface_repr.as_deref(),
                    Some("XLOOKUP not admitted on this host")
                );
            }
            other => panic!("expected Error(BLOCKED, ...), got {other:?}"),
        }
    }

    #[test]
    fn bridge_blocked_provenance_projects_to_result_view_error() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=XLOOKUP(...)");
        let mut document = blocked_editor_document("=XLOOKUP(...)");
        // `blocked_editor_document` already sets a provenance blocked reason.
        document.provenance_summary = Some(ProvenanceSummary {
            profile_summary: "OxFml blocked lane".to_string(),
            blocked_reason: Some("Excel comparison lane unavailable".to_string()),
        });
        formula_space.editor_document = Some(document);
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        match vm.result_view {
            ResultView::Error { code, surface_repr } => {
                assert_eq!(code, "BLOCKED");
                assert_eq!(
                    surface_repr.as_deref(),
                    Some("Excel comparison lane unavailable")
                );
            }
            other => panic!("expected Error(BLOCKED, ...), got {other:?}"),
        }
    }

    #[test]
    fn array_preview_projects_to_result_view_array_with_shape() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SEQUENCE(2,3)");
        formula_space.editor_document = Some(array_editor_document("=SEQUENCE(2,3)"));
        formula_space.array_preview = Some(FormulaArrayPreviewState {
            label: "Array[2 × 3]".to_string(),
            rows: vec![
                vec!["1".to_string(), "2".to_string(), "3".to_string()],
                vec!["4".to_string(), "5".to_string(), "6".to_string()],
            ],
            truncated: false,
        });
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        match vm.result_view {
            ResultView::Array {
                total_rows,
                total_cols,
                label,
                cells,
                cell_format,
                truncated,
            } => {
                assert_eq!(total_rows, 2);
                assert_eq!(total_cols, 3);
                assert_eq!(label, "Array[2 × 3]");
                assert_eq!(cells.len(), 2);
                assert_eq!(cells[0], vec!["1", "2", "3"]);
                assert!(!truncated);
                assert!(cell_format.is_none(), "no CF rules → no per-cell carrier");
            }
            other => panic!("expected Array(2 × 3), got {other:?}"),
        }
    }

    #[test]
    fn array_preview_projects_to_shared_skin_array_window() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SEQUENCE(2,3)");
        formula_space.editor_document = Some(array_editor_document("=SEQUENCE(2,3)"));
        formula_space.array_preview = Some(FormulaArrayPreviewState {
            label: "Array[2 × 3]".to_string(),
            rows: vec![
                vec!["1".to_string(), "2".to_string(), "3".to_string()],
                vec!["4".to_string(), "5".to_string(), "6".to_string()],
            ],
            truncated: false,
        });
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");

        match &vm.skin_snapshot.document {
            dnacalc_skin_ir::SkinDocumentProjection::OneFormula(formula) => match &formula.result {
                dnacalc_skin_ir::FormulaResultSurface::Array {
                    total_rows,
                    total_cols,
                    window,
                    truncated,
                    ..
                } => {
                    assert_eq!((*total_rows, *total_cols), (2, 3));
                    assert_eq!((window.total_rows, window.total_cols), (2, 3));
                    assert_eq!(window.cells[0][0].display_text, "1");
                    assert!(!truncated);
                }
                other => panic!("expected shared array result, got {other:?}"),
            },
            other => panic!("expected OneFormula snapshot, got {other:?}"),
        }
    }

    #[test]
    fn pending_text_with_no_summary_projects_to_pending() {
        // `=SU` starts with `=`, so the pre-bridge hand-eval doesn't fire;
        // there's also no published_value or diagnostics → Pending.
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SU");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert_eq!(vm.result_view, ResultView::Pending);
    }

    #[test]
    fn literal_number_input_renders_inline_as_display_number() {
        // `1.5` is a literal number cell entry; the bridge doesn't run for
        // these. The home shell evaluates inline.
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "1.5");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        match vm.result_view {
            ResultView::Display { text, kind, .. } => {
                assert_eq!(text, "1.5");
                assert_eq!(kind, ResultKind::Number);
            }
            other => panic!("expected Display(Number, '1.5'), got {other:?}"),
        }
    }

    #[test]
    fn literal_text_input_renders_inline_as_display_text() {
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "hello");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        match vm.result_view {
            ResultView::Display { text, kind, .. } => {
                assert_eq!(text, "hello");
                assert_eq!(kind, ResultKind::Text);
            }
            other => panic!("expected Display(Text, 'hello'), got {other:?}"),
        }
    }

    #[test]
    fn forced_text_input_strips_leading_apostrophe() {
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "'123");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        match vm.result_view {
            ResultView::Display { text, kind, .. } => {
                assert_eq!(text, "123");
                assert_eq!(kind, ResultKind::Text);
            }
            other => panic!("expected Display(Text, '123'), got {other:?}"),
        }
    }

    // -----------------------------------------------------------------
    // Caption pills
    // -----------------------------------------------------------------

    #[test]
    fn entry_mode_pill_is_empty_for_blank_input() {
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert_eq!(vm.entry_mode_pill, EntryModePill::Empty);
    }

    #[test]
    fn entry_mode_pill_is_formula_for_leading_equals() {
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2)");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert_eq!(vm.entry_mode_pill, EntryModePill::Formula);
    }

    #[test]
    fn entry_mode_pill_is_text_for_leading_apostrophe() {
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "'42");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert_eq!(vm.entry_mode_pill, EntryModePill::Text);
    }

    #[test]
    fn entry_mode_pill_is_value_for_literal_cell_entry() {
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "42.5");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert_eq!(vm.entry_mode_pill, EntryModePill::Value);
    }

    #[test]
    fn result_class_pill_is_none_for_empty_and_pending() {
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert!(vm.result_class_pill.is_none());

        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SU");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert!(vm.result_class_pill.is_none());
    }

    #[test]
    fn result_class_pill_is_number_for_number_display() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2)");
        let mut document = sample_editor_document("=SUM(1,2)");
        attach_number_value_presentation(&mut document, 3.0, "3");
        formula_space.editor_document = Some(document);
        formula_space.effective_display_summary = Some("3".to_string());
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert_eq!(vm.result_class_pill, Some(ResultClassPill::Number));
    }

    #[test]
    fn result_class_pill_is_error_for_diagnostic_or_blocked() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,)");
        formula_space.editor_document = Some(diagnostic_editor_document("=SUM(1,)"));
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert_eq!(vm.result_class_pill, Some(ResultClassPill::Error));
    }

    #[test]
    fn result_class_pill_is_array_for_array_result() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SEQUENCE(2,3)");
        formula_space.editor_document = Some(array_editor_document("=SEQUENCE(2,3)"));
        formula_space.array_preview = Some(FormulaArrayPreviewState {
            label: "Array[2 × 3]".to_string(),
            rows: vec![
                vec!["1".to_string(), "2".to_string(), "3".to_string()],
                vec!["4".to_string(), "5".to_string(), "6".to_string()],
            ],
            truncated: false,
        });
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert_eq!(vm.result_class_pill, Some(ResultClassPill::Array));
    }

    #[test]
    fn result_class_pill_is_text_for_literal_text_input() {
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "hello");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert_eq!(vm.result_class_pill, Some(ResultClassPill::Text));
    }

    #[test]
    fn result_class_pill_is_number_for_literal_number_input() {
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "42");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert_eq!(vm.result_class_pill, Some(ResultClassPill::Number));
    }

    // -----------------------------------------------------------------
    // Syntax overlay runs
    // -----------------------------------------------------------------

    #[test]
    fn syntax_runs_empty_without_editor_document() {
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2)");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert!(vm.syntax_runs.is_empty());
    }

    #[test]
    fn syntax_runs_populated_when_document_matches_raw_text() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2)");
        formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert!(!vm.syntax_runs.is_empty());
        assert_eq!(
            vm.syntax_runs.first().map(|run| run.text.as_str()),
            Some("=")
        );
    }

    #[test]
    fn syntax_runs_empty_when_document_is_stale() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2,3)");
        // Document carries a different (older) source text — stale snapshot.
        formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert!(vm.syntax_runs.is_empty());
    }

    // -----------------------------------------------------------------
    // Diagnostic squiggles
    // -----------------------------------------------------------------

    #[test]
    fn diagnostic_squiggles_empty_without_editor_document() {
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert!(vm.diagnostic_squiggles.is_empty());
    }

    #[test]
    fn diagnostic_squiggles_empty_when_document_is_stale() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2,3)");
        formula_space.editor_document = Some(diagnostic_editor_document("=SUM(1,)"));
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert!(vm.diagnostic_squiggles.is_empty());
    }

    #[test]
    fn diagnostic_squiggles_carry_message_severity_and_span() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,)");
        formula_space.editor_document = Some(diagnostic_editor_document("=SUM(1,)"));
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert_eq!(vm.diagnostic_squiggles.len(), 1);
        let squiggle = &vm.diagnostic_squiggles[0];
        assert_eq!(squiggle.message, "Missing trailing argument");
        assert_eq!(squiggle.severity, SquiggleSeverity::Error);
        assert!(squiggle.span_len >= 1);
    }

    #[test]
    fn diagnostic_squiggles_sort_and_dedup_overlaps() {
        use crate::adapters::oxfml::LiveDiagnosticSnapshot;
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2,3)");
        let mut document = sample_editor_document("=SUM(1,2,3)");
        document.live_diagnostics = LiveDiagnosticSnapshot {
            formula_stable_id: "f1".into(),
            formula_token: "f1".into(),
            diagnostics: vec![
                // Out of order: late then early.
                make_diag("d-late", "later", 8, 2, LiveDiagnosticSeverity::Warning),
                make_diag("d-early", "earlier", 1, 3, LiveDiagnosticSeverity::Error),
                // Overlaps with d-early — should be dropped.
                make_diag("d-overlap", "overlap", 2, 2, LiveDiagnosticSeverity::Error),
            ],
        };
        formula_space.editor_document = Some(document);
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        // Sorted by span_start ascending; overlap dropped, leaving 2.
        assert_eq!(vm.diagnostic_squiggles.len(), 2);
        assert_eq!(vm.diagnostic_squiggles[0].diagnostic_id, "d-early");
        assert_eq!(vm.diagnostic_squiggles[0].severity, SquiggleSeverity::Error);
        assert_eq!(vm.diagnostic_squiggles[1].diagnostic_id, "d-late");
        assert_eq!(
            vm.diagnostic_squiggles[1].severity,
            SquiggleSeverity::Warning
        );
    }

    fn make_diag(
        id: &str,
        message: &str,
        start: usize,
        len: usize,
        severity: LiveDiagnosticSeverity,
    ) -> crate::adapters::oxfml::LiveDiagnostic {
        use crate::adapters::oxfml::{FormulaTextSpan, LiveDiagnostic, LiveDiagnosticStage};
        LiveDiagnostic {
            diagnostic_id: id.to_string(),
            severity,
            stage: LiveDiagnosticStage::Bind,
            message: message.to_string(),
            primary_span: FormulaTextSpan { start, len },
            related_spans: Vec::new(),
            code: None,
            worksheet_error_class: None,
            suggested_fix_kind: None,
        }
    }

    // -----------------------------------------------------------------
    // Foot chips
    // -----------------------------------------------------------------

    #[test]
    fn editor_metrics_zero_without_document() {
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2)");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert_eq!(vm.editor_metrics.token_count, 0);
        assert_eq!(vm.editor_metrics.function_count, 0);
        assert_eq!(vm.editor_metrics.diagnostic_count, 0);
    }

    #[test]
    fn editor_metrics_count_tokens_functions_and_diagnostics() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2)");
        formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        // sample_editor_document for "=SUM(1,2)" emits 7 tokens, has 1
        // diagnostic ('sample diagnostic'), and SUM is a function token.
        assert_eq!(vm.editor_metrics.token_count, 7);
        assert!(vm.editor_metrics.function_count >= 1);
        assert_eq!(vm.editor_metrics.diagnostic_count, 1);
    }

    #[test]
    fn result_context_collapses_when_formula_is_at_default() {
        // Per WS-14 §5 ("result-foot rethink"): when nothing is
        // authored beyond defaults (no format code, no CF rules,
        // policy at `LiveRecalc`), the result-foot collapses
        // entirely. The view-model returns `None` so the renderer
        // skips the chrome.
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert!(
            vm.result_context.is_none(),
            "default formula must collapse the result-foot",
        );
        // Locale moved to the formatting panel. The runtime locale
        // chain is live (OxFunc W094 + `build_runtime_locale_context`
        // in `live_bridge`), so the picker no longer carries the
        // `SEAM-OXFML-LOCALE-EXPAND` marker for the curated preset
        // list — the field is `None`.
        assert_eq!(vm.formatting_controls.locale_seam_id, None);
        assert!(!vm.formatting_controls.locale_label.is_empty());
    }

    #[test]
    fn result_context_format_chip_reads_active_formula_format_code() {
        let mut formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=A1");
        formula_space.formatting.number_format_code = "$#,##0.00".to_string();
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        // The chip shows the matched preset family label, not the raw code.
        let context = vm
            .result_context
            .as_ref()
            .expect("non-default formatting surfaces the chip");
        assert_eq!(context.format.value(), "Currency");
        assert_eq!(context.format.seam_id(), None);
    }

    #[test]
    fn result_context_always_shows_when_manual_recalc() {
        // ManualRecalc forces the chip to be visible regardless
        // of format code state — the user needs the visual
        // reminder that typing isn't running the runtime pass.
        let mut formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=NOW()");
        formula_space.formatting.scenario_policy = crate::persistence::ScenarioPolicy::ManualRecalc;
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        let context = vm
            .result_context
            .as_ref()
            .expect("ManualRecalc always shows the chip");
        assert_eq!(context.policy.value(), "manual-recalc");
    }

    // -----------------------------------------------------------------
    // Completion popup view-model projection (bead dno-xcq.24)
    // -----------------------------------------------------------------

    fn open_popup_state() -> CompletionPopupState {
        use crate::adapters::oxfml::FormulaTextSpan;
        use crate::services::completion_popup::{CompletionPopupItem, CompletionPopupKind};
        CompletionPopupState::Open {
            anchor_offset: 1,
            items: vec![
                CompletionPopupItem {
                    proposal_id: "p-1".to_string(),
                    display_text: "SUM".to_string(),
                    insert_text: "SUM(".to_string(),
                    kind: CompletionPopupKind::Function,
                    replacement_span: Some(FormulaTextSpan { start: 1, len: 2 }),
                    documentation_ref: Some("doc:sum".to_string()),
                },
                CompletionPopupItem {
                    proposal_id: "p-2".to_string(),
                    display_text: "SUMIF".to_string(),
                    insert_text: "SUMIF(".to_string(),
                    kind: CompletionPopupKind::Function,
                    replacement_span: Some(FormulaTextSpan { start: 1, len: 2 }),
                    documentation_ref: None,
                },
            ],
            selected_index: 1,
        }
    }

    fn synthetic_metrics() -> crate::adapters::oxfml::FormulaTextSpan {
        // Returning a span isn't quite right; replace below.
        crate::adapters::oxfml::FormulaTextSpan { start: 0, len: 0 }
    }

    #[test]
    fn completion_popup_view_is_none_when_state_hidden() {
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SU");
        // popup defaults to Hidden, even when metrics are populated.
        let mut formula_space = formula_space;
        formula_space.editor_box_metrics =
            Some(crate::ui::editor::geometry::TextareaMeasurementMetrics {
                char_width_px: 9,
                line_height_px: 22,
                scroll_top_px: 0,
                scroll_left_px: 0,
            });
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert!(vm.completion_popup.is_none());
        let _ = synthetic_metrics(); // silence unused-helper warning
    }

    #[test]
    fn completion_popup_view_is_none_when_metrics_unmeasured() {
        let mut formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SU");
        formula_space.completion_popup = open_popup_state();
        // Metrics deliberately None — adapter hasn't run yet.
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert!(
            vm.completion_popup.is_none(),
            "popup view suppressed until measurement lands",
        );
    }

    #[test]
    fn completion_popup_view_is_some_when_open_and_measured() {
        let mut formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SU");
        formula_space.completion_popup = open_popup_state();
        formula_space.editor_box_metrics =
            Some(crate::ui::editor::geometry::TextareaMeasurementMetrics {
                char_width_px: 9,
                line_height_px: 22,
                scroll_top_px: 0,
                scroll_left_px: 0,
            });
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        let popup = vm.completion_popup.expect("popup view present");
        // Anchor at offset 1 with char_width 9 -> left = 9.
        assert_eq!(popup.anchor_left_px, 9);
        assert_eq!(popup.anchor_top_px, 0);
        assert_eq!(popup.line_height_px, 22);
        assert_eq!(popup.items.len(), 2);
        assert_eq!(popup.selected_index, 1);
    }

    #[test]
    fn completion_popup_projects_to_shared_formula_assist_surface() {
        let mut formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SU");
        formula_space.completion_popup = open_popup_state();
        formula_space.editor_box_metrics =
            Some(crate::ui::editor::geometry::TextareaMeasurementMetrics {
                char_width_px: 9,
                line_height_px: 22,
                scroll_top_px: 0,
                scroll_left_px: 0,
            });
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");

        match &vm.skin_snapshot.document {
            dnacalc_skin_ir::SkinDocumentProjection::OneFormula(formula) => {
                let completion = formula
                    .assist
                    .completion
                    .as_ref()
                    .expect("shared completion surface");
                assert_eq!(completion.selected_index, 1);
                assert_eq!(completion.items[0].display_text, "SUM");
                assert_eq!(
                    completion.items[0].kind,
                    dnacalc_skin_ir::CompletionKindProjection::Function
                );
            }
            other => panic!("expected OneFormula snapshot, got {other:?}"),
        }
    }

    #[test]
    fn completion_popup_view_marks_selected_item_only() {
        let mut formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SU");
        formula_space.completion_popup = open_popup_state();
        formula_space.editor_box_metrics =
            Some(crate::ui::editor::geometry::TextareaMeasurementMetrics {
                char_width_px: 9,
                line_height_px: 22,
                scroll_top_px: 0,
                scroll_left_px: 0,
            });
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        let popup = vm.completion_popup.expect("popup view present");
        assert_eq!(popup.items[0].is_selected, false);
        assert_eq!(popup.items[1].is_selected, true);
    }

    #[test]
    fn completion_popup_view_kind_glyph_and_label_cover_all_variants() {
        use crate::services::completion_popup::CompletionPopupKind as Kind;
        for (kind, expected_glyph, expected_label) in [
            (Kind::Function, 'ƒ', "Function"),
            (Kind::DefinedName, 'N', "Defined name"),
            (Kind::TableName, 'T', "Table"),
            (Kind::TableColumn, '⫶', "Column"),
            (Kind::StructuredSelector, '#', "Selector"),
            (Kind::SyntaxAssist, '·', "Syntax"),
        ] {
            assert_eq!(
                CompletionPopupItemView::glyph_for_kind(kind),
                expected_glyph
            );
            assert_eq!(
                CompletionPopupItemView::label_for_kind(kind),
                expected_label
            );
        }
    }

    #[test]
    fn completion_popup_view_anchor_uses_replacement_span_start_via_state_offset() {
        // The reducer auto-sync sets anchor_offset to the proposal's
        // replacement_span.start; here we set it explicitly to verify
        // the projector consumes that field rather than the caret.
        let mut formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM");
        // Pretend the popup anchored at offset 1 (start of "SUM") even
        // though the caret has advanced to offset 4 (end of text).
        formula_space.editor_surface_state.caret.offset = 4;
        formula_space.completion_popup = open_popup_state();
        formula_space.editor_box_metrics =
            Some(crate::ui::editor::geometry::TextareaMeasurementMetrics {
                char_width_px: 9,
                line_height_px: 22,
                scroll_top_px: 0,
                scroll_left_px: 0,
            });
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        let popup = vm.completion_popup.expect("popup view present");
        // Anchor offset = 1; left_px = 1 * 9 = 9. NOT 4 * 9 = 36.
        assert_eq!(popup.anchor_left_px, 9);
    }

    // -----------------------------------------------------------------
    // Signature help
    // -----------------------------------------------------------------

    fn metrics_9x22() -> crate::ui::editor::geometry::TextareaMeasurementMetrics {
        crate::ui::editor::geometry::TextareaMeasurementMetrics {
            char_width_px: 9,
            line_height_px: 22,
            scroll_top_px: 0,
            scroll_left_px: 0,
        }
    }

    fn document_with_signature_help_for_sum(
        source_text: &str,
        active_argument_index: usize,
    ) -> crate::adapters::oxfml::EditorDocument {
        use oxfml_core::syntax::green::SyntaxKind;
        let mut document = sample_editor_document(source_text);
        document.signature_help = Some(crate::adapters::oxfml::SignatureHelpContext {
            callee_text: "SUM".to_string(),
            call_span: crate::adapters::oxfml::FormulaTextSpan {
                start: 1,
                len: source_text.chars().count().saturating_sub(1),
            },
            active_argument_index,
            invocation_kind: SyntaxKind::CallExpr,
        });
        document.function_help = Some(crate::adapters::oxfml::FunctionHelpPacket {
            lookup_key: "SUM".to_string(),
            library_context_snapshot_ref: None,
            display_name: "SUM".to_string(),
            signature_forms: vec![crate::adapters::oxfml::FunctionHelpSignatureForm {
                display_signature: "SUM(number1, number2, ...)".to_string(),
                min_arity: 1,
                max_arity: None,
            }],
            argument_help: vec![
                "number1".to_string(),
                "number2".to_string(),
                "additional_numbers".to_string(),
            ],
            short_description: Some("Adds numbers together.".to_string()),
            availability_summary: Some("supported".to_string()),
            deferred_or_profile_limited: false,
        });
        document
    }

    #[test]
    fn signature_help_view_built_from_editor_document() {
        let mut formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(");
        formula_space.editor_document = Some(document_with_signature_help_for_sum("=SUM(", 0));
        formula_space.editor_box_metrics = Some(metrics_9x22());
        formula_space.editor_surface_state.caret.offset = 5;

        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("vm");
        let help = vm.signature_help.expect("signature help projected");
        assert_eq!(help.callee_text, "SUM");
        assert_eq!(help.parameters.len(), 3);
        assert_eq!(help.parameters[0].name, "number1");
        assert!(help.parameters[0].is_active);
        assert!(!help.parameters[1].is_active);
        assert_eq!(help.active_parameter, Some(0));
        // Anchor at caret offset 5 with char_width 9 → left 45.
        assert_eq!(help.anchor_left_px, 45);
        assert_eq!(help.line_height_px, 22);
    }

    #[test]
    fn signature_help_view_active_argument_advances_after_comma() {
        // After typing `=SUM(1,` the bridge bumps active_argument_index
        // to 1 — the second parameter is now the active one.
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,");
        formula_space.editor_document = Some(document_with_signature_help_for_sum("=SUM(1,", 1));
        formula_space.editor_box_metrics = Some(metrics_9x22());
        formula_space.editor_surface_state.caret.offset = 7;

        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("vm");
        let help = vm.signature_help.expect("signature help projected");
        assert_eq!(help.active_parameter, Some(1));
        assert!(!help.parameters[0].is_active);
        assert!(help.parameters[1].is_active);
        assert!(!help.parameters[2].is_active);
    }

    #[test]
    fn signature_help_view_active_argument_clamps_when_out_of_range() {
        // Bridge reports active_argument_index = 5 but argument_help
        // has 3 entries. Clamp to None (no parameter bolded) rather
        // than panic or wrap.
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2,3,4,5,");
        formula_space.editor_document =
            Some(document_with_signature_help_for_sum("=SUM(1,2,3,4,5,", 5));
        formula_space.editor_box_metrics = Some(metrics_9x22());
        formula_space.editor_surface_state.caret.offset = 15;

        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("vm");
        let help = vm.signature_help.expect("signature help projected");
        assert_eq!(help.active_parameter, None);
        assert!(help.parameters.iter().all(|p| !p.is_active));
    }

    #[test]
    fn signature_help_view_empty_when_document_is_stale() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2");
        // Document still reflects the pre-`,2` state — stale by one keystroke.
        formula_space.editor_document = Some(document_with_signature_help_for_sum("=SUM(", 0));
        formula_space.editor_box_metrics = Some(metrics_9x22());

        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("vm");
        assert!(vm.signature_help.is_none());
    }

    #[test]
    fn signature_help_view_empty_when_no_signature_help_in_document() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2)");
        // Plain sample document carries function_help but no
        // signature_help (sample_editor_document populates the help
        // packet but signature_help only when explicitly attached).
        let mut document = sample_editor_document("=SUM(1,2)");
        document.signature_help = None;
        formula_space.editor_document = Some(document);
        formula_space.editor_box_metrics = Some(metrics_9x22());

        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("vm");
        assert!(vm.signature_help.is_none());
    }

    #[test]
    fn signature_help_view_empty_when_metrics_unmeasured() {
        let mut formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(");
        formula_space.editor_document = Some(document_with_signature_help_for_sum("=SUM(", 0));
        // editor_box_metrics deliberately None — geometry can't anchor yet.

        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("vm");
        assert!(vm.signature_help.is_none());
    }

    #[test]
    fn signature_help_view_suppressed_when_completion_popup_open() {
        // Popup wins; signature help hides until the popup dismisses.
        let mut formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(s");
        formula_space.editor_document = Some(document_with_signature_help_for_sum("=SUM(s", 0));
        formula_space.editor_box_metrics = Some(metrics_9x22());
        formula_space.completion_popup = open_popup_state();
        formula_space.editor_surface_state.caret.offset = 6;

        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("vm");
        assert!(vm.completion_popup.is_some());
        assert!(
            vm.signature_help.is_none(),
            "signature help must be suppressed while the completion popup is open",
        );
    }

    #[test]
    fn signature_help_view_renders_callee_only_when_function_help_packet_missing() {
        // Defensive: bridge gives signature_help but no function_help
        // (theoretically possible during a brief stale-document tick).
        // The view-model still renders the callee — empty parameter list.
        let mut formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(");
        let mut document = document_with_signature_help_for_sum("=SUM(", 0);
        document.function_help = None;
        formula_space.editor_document = Some(document);
        formula_space.editor_box_metrics = Some(metrics_9x22());
        formula_space.editor_surface_state.caret.offset = 5;

        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("vm");
        let help = vm.signature_help.expect("signature help projected");
        assert_eq!(help.callee_text, "SUM");
        assert!(help.parameters.is_empty());
        assert_eq!(help.active_parameter, None);
    }

    // -----------------------------------------------------------------
    // Function-help card
    // -----------------------------------------------------------------

    #[test]
    fn function_help_card_built_from_editor_document_packet() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2)");
        // sample_editor_document populates a function_help packet
        // for SUM with three arg names + short description.
        formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("vm");
        let card = vm.function_help_card.expect("card projected");
        assert_eq!(card.lookup_key, "SUM");
        assert_eq!(card.display_name, "SUM");
        assert_eq!(
            card.signature.as_deref(),
            Some("SUM(number1, number2, ...)")
        );
        assert_eq!(
            card.short_description.as_deref(),
            Some("Adds numbers together.")
        );
        assert_eq!(card.availability_summary.as_deref(), Some("supported"));
        assert!(!card.deferred_or_profile_limited);
    }

    #[test]
    fn function_help_card_is_none_when_packet_absent() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2)");
        let mut document = sample_editor_document("=SUM(1,2)");
        document.function_help = None;
        formula_space.editor_document = Some(document);
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("vm");
        assert!(vm.function_help_card.is_none());
    }

    #[test]
    fn function_help_card_is_none_when_document_is_stale() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2,3)");
        // Document still reflects the pre-`,3` state.
        formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("vm");
        assert!(vm.function_help_card.is_none());
    }

    #[test]
    fn function_help_card_signature_is_none_when_signature_forms_empty() {
        // Defensive: bridge populates function_help but the packet has
        // no signature forms. The card still renders display_name and
        // description; the signature line is just absent.
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2)");
        let mut document = sample_editor_document("=SUM(1,2)");
        if let Some(ref mut packet) = document.function_help {
            packet.signature_forms.clear();
        }
        formula_space.editor_document = Some(document);
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("vm");
        let card = vm.function_help_card.expect("card projected");
        assert_eq!(card.lookup_key, "SUM");
        assert!(card.signature.is_none());
    }

    #[test]
    fn function_help_card_suppresses_placeholder_signature() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2)");
        let mut document = sample_editor_document("=SUM(1,2)");
        if let Some(ref mut packet) = document.function_help {
            packet.signature_forms[0].display_signature = "SUM(...)".to_string();
        }
        formula_space.editor_document = Some(document);
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("vm");
        let card = vm.function_help_card.expect("card projected");
        assert_eq!(card.lookup_key, "SUM");
        assert!(card.signature.is_none());
    }

    // -----------------------------------------------------------------
    // View mode
    // -----------------------------------------------------------------

    #[test]
    fn view_mode_defaults_to_user_in_view_model() {
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("vm");
        assert_eq!(vm.view_mode, ViewMode::User);
    }

    #[test]
    fn view_mode_developer_propagates_into_view_model() {
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "");
        let mut state = host_state_with(formula_space);
        state.view_mode = ViewMode::Developer;
        let vm = build_home_shell_view_model(&state).expect("vm");
        assert_eq!(vm.view_mode, ViewMode::Developer);
    }

    // -----------------------------------------------------------------
    // Formula drill-down
    // -----------------------------------------------------------------

    #[test]
    fn formula_drill_default_collapsed_with_empty_tree() {
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("vm");
        assert!(!vm.formula_drill.expanded);
        assert!(vm.formula_drill.tree.is_empty());
        assert!(vm.formula_drill.phase_summaries.is_empty());
        assert!(!vm.formula_drill.document_is_fresh);
    }

    #[test]
    fn formula_drill_expanded_flag_follows_state_field() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2)");
        formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
        formula_space.formula_drill_open = true;
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("vm");
        assert!(vm.formula_drill.expanded);
    }

    #[test]
    fn formula_drill_flattens_walk_tree_in_preorder_with_depth() {
        use crate::adapters::oxfml::{
            FormulaDrillArrayPreview, FormulaDrillNodeState, FormulaDrillNodeViewModel,
        };
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=LET(x,1,x)");
        let mut document = sample_editor_document("=LET(x,1,x)");
        document.formula_walk = vec![FormulaDrillNodeViewModel {
            node_id: "let".to_string(),
            label: "LET".to_string(),
            developer_label: None,
            expression_text: None,
            kind: None,
            source_span_start: None,
            source_span_len: None,
            branch_disposition: None,
            argument_name: None,
            argument_role: None,
            error_message: None,
            value_preview: Some("1".to_string()),
            array_preview: Some(FormulaDrillArrayPreview {
                total_rows: 2,
                total_cols: 2,
                rows: vec![
                    vec!["1".to_string(), "2".to_string()],
                    vec!["3".to_string(), "4".to_string()],
                ],
                truncated: false,
            }),
            state: FormulaDrillNodeState::Evaluated,
            children: vec![
                FormulaDrillNodeViewModel {
                    node_id: "x-bind".to_string(),
                    label: "x".to_string(),
                    developer_label: None,
                    expression_text: None,
                    kind: None,
                    source_span_start: None,
                    source_span_len: None,
                    branch_disposition: None,
                    argument_name: None,
                    argument_role: None,
                    error_message: None,
                    value_preview: Some("1".to_string()),
                    array_preview: None,
                    state: FormulaDrillNodeState::Bound,
                    children: vec![],
                },
                FormulaDrillNodeViewModel {
                    node_id: "x-use".to_string(),
                    label: "x".to_string(),
                    developer_label: None,
                    expression_text: None,
                    kind: None,
                    source_span_start: None,
                    source_span_len: None,
                    branch_disposition: None,
                    argument_name: None,
                    argument_role: None,
                    error_message: None,
                    value_preview: Some("1".to_string()),
                    array_preview: None,
                    state: FormulaDrillNodeState::Evaluated,
                    children: vec![],
                },
            ],
        }];
        formula_space.editor_document = Some(document);
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("vm");
        // Tree is now nested rather than flat-with-depth: the root
        // is a single LET node carrying the two child rows.
        let nodes = &vm.formula_drill.tree;
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].node_id, "let");
        assert_eq!(
            nodes[0].array_preview.as_ref().map(|preview| (
                preview.total_rows,
                preview.total_cols,
                preview.rows.len()
            )),
            Some((2, 2, 2))
        );
        assert_eq!(nodes[0].children.len(), 2);
        assert_eq!(nodes[0].children[0].node_id, "x-bind");
        assert!(nodes[0].children[0].children.is_empty());
        assert_eq!(nodes[0].children[1].node_id, "x-use");
    }

    #[test]
    fn formula_drill_phase_summaries_emit_parse_bind_eval() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2)");
        formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("vm");
        let labels: Vec<&str> = vm
            .formula_drill
            .phase_summaries
            .iter()
            .map(|p| p.label)
            .collect();
        assert_eq!(labels, vec!["parse", "bind", "eval"]);
        assert!(vm
            .formula_drill
            .phase_summaries
            .iter()
            .all(|p| p.state == FormulaDrillPhaseState::Ok));
    }

    #[test]
    fn formula_drill_eval_phase_blocked_when_provenance_carries_blocked_reason() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=XLOOKUP(...)");
        formula_space.editor_document = Some(blocked_editor_document("=XLOOKUP(...)"));
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("vm");
        let eval = vm
            .formula_drill
            .phase_summaries
            .iter()
            .find(|p| p.label == "eval")
            .expect("eval chip emitted");
        assert_eq!(eval.state, FormulaDrillPhaseState::Blocked);
    }

    #[test]
    fn formula_drill_tree_empty_when_document_is_stale() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2,3)");
        // Document still reflects the pre-`,3` state.
        formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
        formula_space.formula_drill_open = true;
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("vm");
        assert!(vm.formula_drill.expanded);
        assert!(vm.formula_drill.tree.is_empty());
        assert!(vm.formula_drill.phase_summaries.is_empty());
        assert!(!vm.formula_drill.document_is_fresh);
    }

    // -----------------------------------------------------------------
    // Status foot
    // -----------------------------------------------------------------

    #[test]
    fn status_live_when_live_backed_with_green_tree_key() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2)");
        formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
        formula_space.context.truth_source = ProjectionTruthSource::LiveBacked;
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert_eq!(vm.status.bridge_health, BridgeHealth::Live);
        assert_eq!(vm.status.green_tree_key.as_deref(), Some("green-1"));
    }

    #[test]
    fn status_stale_when_local_fallback() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2)");
        formula_space.editor_document = Some(sample_editor_document("=SUM(1,2)"));
        formula_space.context.truth_source = ProjectionTruthSource::LocalFallback;
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert_eq!(vm.status.bridge_health, BridgeHealth::Stale);
    }

    #[test]
    fn status_stale_when_live_backed_but_no_green_tree_key() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2)");
        // LiveBacked but no editor_document => no green-tree key => stale.
        formula_space.context.truth_source = ProjectionTruthSource::LiveBacked;
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert_eq!(vm.status.bridge_health, BridgeHealth::Stale);
        assert!(vm.status.green_tree_key.is_none());
    }

    // -----------------------------------------------------------------------
    // Scenario breadcrumb projection
    // -----------------------------------------------------------------------

    #[test]
    fn breadcrumb_label_falls_back_to_unsaved_for_synthetic_scenario_label() {
        // FormulaSpaceState::new auto-sets scenario_label = formula_space_id;
        // the breadcrumb projects that to "unsaved" rather than leaking the
        // synthetic id into the titlebar.
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active");
        assert_eq!(vm.scenario_breadcrumb.active_label, "unsaved");
    }

    #[test]
    fn breadcrumb_label_uses_user_assigned_scenario_label() {
        let mut formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "");
        formula_space.context.scenario_label = "invoice-eu-tax".to_string();
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active");
        assert_eq!(vm.scenario_breadcrumb.active_label, "invoice-eu-tax");
    }

    #[test]
    fn breadcrumb_dirty_when_live_text_differs_from_committed() {
        let mut formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=1");
        formula_space.committed_cell_text = Some("=2".to_string());
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active");
        assert!(vm.scenario_breadcrumb.is_dirty);
    }

    #[test]
    fn breadcrumb_clean_when_live_text_matches_committed() {
        let mut formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=1");
        formula_space.committed_cell_text = Some("=1".to_string());
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active");
        assert!(!vm.scenario_breadcrumb.is_dirty);
    }

    #[test]
    fn breadcrumb_dropdown_open_mirrors_global_chrome_state() {
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "");
        let mut state = host_state_with(formula_space);
        assert!(
            !build_home_shell_view_model(&state)
                .unwrap()
                .scenario_breadcrumb
                .is_open
        );
        state.global_ui_chrome.scenario_breadcrumb_open = true;
        assert!(
            build_home_shell_view_model(&state)
                .unwrap()
                .scenario_breadcrumb
                .is_open
        );
    }

    #[test]
    fn breadcrumb_recent_starts_with_active_scenario_marked_active() {
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("space-active"), "");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active");
        let recent = &vm.scenario_breadcrumb.recent;
        assert!(
            !recent.is_empty(),
            "recent must include the active scenario"
        );
        assert!(recent[0].is_active);
        assert_eq!(recent[0].formula_space_id, "space-active");
        assert_eq!(recent[0].meta, "active");
    }

    #[test]
    fn breadcrumb_recent_caps_at_five_entries() {
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("space-active"), "");
        let mut state = host_state_with(formula_space);
        // Seed 6 closed records; only the first 4 (plus the active one)
        // should make it into the breadcrumb's Recent list.
        for i in 0..6 {
            let id = FormulaSpaceId::new(format!("recent-{i}"));
            state
                .workspace_shell
                .recent_formula_space_order
                .push(id.clone());
            state.workspace_shell.recent_formula_spaces.insert(
                id.clone(),
                ClosedFormulaSpaceRecord {
                    formula_space: FormulaSpaceState::new(id, ""),
                    last_active_mode: AppMode::Explore,
                },
            );
        }
        let vm = build_home_shell_view_model(&state).expect("active");
        assert_eq!(vm.scenario_breadcrumb.recent.len(), 5);
        // First entry is the active one; remaining four are recent-0..recent-3.
        assert!(vm.scenario_breadcrumb.recent[0].is_active);
        for entry in &vm.scenario_breadcrumb.recent[1..] {
            assert!(!entry.is_active);
            assert_eq!(entry.meta, "recent");
        }
    }

    #[test]
    fn breadcrumb_recent_includes_other_open_formula_spaces_before_closed_recents() {
        let active = FormulaSpaceState::new(FormulaSpaceId::new("space-active"), "");
        let mut state = host_state_with(active);
        let open_id = FormulaSpaceId::new("space-open");
        let mut open_space = FormulaSpaceState::new(open_id.clone(), "");
        open_space.context.scenario_label = "Open friend".to_string();
        state.formula_spaces.insert(open_space);
        state
            .workspace_shell
            .open_formula_space_order
            .push(open_id.clone());
        let closed_id = FormulaSpaceId::new("space-closed");
        let mut closed_space = FormulaSpaceState::new(closed_id.clone(), "");
        closed_space.context.scenario_label = "Closed recent".to_string();
        state
            .workspace_shell
            .recent_formula_space_order
            .push(closed_id.clone());
        state.workspace_shell.recent_formula_spaces.insert(
            closed_id,
            ClosedFormulaSpaceRecord {
                formula_space: closed_space,
                last_active_mode: AppMode::Explore,
            },
        );

        let vm = build_home_shell_view_model(&state).expect("active");
        let recent = &vm.scenario_breadcrumb.recent;
        assert_eq!(recent[0].formula_space_id, "space-active");
        assert_eq!(recent[1].formula_space_id, "space-open");
        assert_eq!(recent[1].meta, "open");
        assert_eq!(recent[2].formula_space_id, "space-closed");
        assert_eq!(recent[2].meta, "recent");
    }

    #[test]
    fn breadcrumb_recent_dedupes_active_scenario_from_recent_list() {
        // If the workspace has the active id ALSO listed in
        // recent_formula_space_order (e.g. it was previously closed
        // and re-opened), the active entry must appear once, not
        // twice.
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("space-active"), "");
        let mut state = host_state_with(formula_space);
        state
            .workspace_shell
            .recent_formula_space_order
            .push(FormulaSpaceId::new("space-active"));
        let vm = build_home_shell_view_model(&state).expect("active");
        let active_count = vm
            .scenario_breadcrumb
            .recent
            .iter()
            .filter(|entry| entry.formula_space_id == "space-active")
            .count();
        assert_eq!(active_count, 1, "active scenario must appear only once");
    }

    #[test]
    fn breadcrumb_pinned_lists_pinned_ids_and_marks_active_when_active_is_pinned() {
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("space-active"), "");
        let mut state = host_state_with(formula_space);
        state
            .workspace_shell
            .pinned_formula_space_ids
            .insert(FormulaSpaceId::new("space-active"));
        state
            .workspace_shell
            .pinned_formula_space_ids
            .insert(FormulaSpaceId::new("space-other-pin"));
        let vm = build_home_shell_view_model(&state).expect("active");
        let pinned = &vm.scenario_breadcrumb.pinned;
        assert_eq!(pinned.len(), 2);
        let active_pin = pinned
            .iter()
            .find(|entry| entry.formula_space_id == "space-active")
            .expect("active pin row");
        assert!(active_pin.is_active);
        assert!(active_pin.is_pinned);
        let other_pin = pinned
            .iter()
            .find(|entry| entry.formula_space_id == "space-other-pin")
            .expect("other pin row");
        assert!(!other_pin.is_active);
        assert!(other_pin.is_pinned);
    }

    #[test]
    fn breadcrumb_actions_carry_stable_ids_and_seam_markers() {
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active");
        let action_ids: Vec<_> = vm
            .scenario_breadcrumb
            .actions
            .iter()
            .map(|action| action.action_id)
            .collect();
        // The active formula on a fresh state is unpinned, so the
        // dropdown surfaces `PinActive` (toggling to `UnpinActive`
        // when pinned).
        assert_eq!(
            action_ids,
            vec![
                ScenarioBreadcrumbActionId::NewScenario,
                ScenarioBreadcrumbActionId::SaveAs,
                ScenarioBreadcrumbActionId::Open,
                ScenarioBreadcrumbActionId::Duplicate,
                ScenarioBreadcrumbActionId::RenameActive,
                ScenarioBreadcrumbActionId::PinActive,
                ScenarioBreadcrumbActionId::ManageScenarios,
            ],
        );
        // After slice 1b, the only action that still carries a SEAM
        // marker is `ManageScenarios` (the manage-formulas page is
        // not yet built). Other actions (`NewScenario`, `Duplicate`,
        // `PinActive` / `UnpinActive`) use existing in-memory
        // reducers; SaveAs / Open are wired through
        // `persistence/browser_file_io.rs`.
        for action in &vm.scenario_breadcrumb.actions {
            // Every action is wired today — `ManageScenarios` opens
            // the new manage-formulas overlay and the others use the
            // case-lifecycle / persistence reducers directly.
            assert_eq!(action.seam_id, None, "{:?}", action.action_id);
        }
    }

    /// The manage-formulas view-model is `is_open: false` when the
    /// chrome flag is off — no projection cost on the closed path.
    #[test]
    fn manage_formulas_is_closed_by_default() {
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("space-1"), "=A1");
        let state = host_state_with(formula_space);
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert!(!vm.manage_formulas.is_open);
        assert_eq!(vm.manage_formulas.total_count, 0);
        assert!(vm.manage_formulas.rows.is_empty());
    }

    /// When opened, the overlay surfaces every formula in the
    /// workspace once each (deduped across pinned / open / recent).
    #[test]
    fn manage_formulas_open_lists_every_formula_once() {
        let mut state = OneCalcHostState::default();
        // Two open formulas, one pinned.
        let space_a = FormulaSpaceState::new(FormulaSpaceId::new("space-a"), "=SUM(1,2)");
        let space_b = {
            let mut space = FormulaSpaceState::new(FormulaSpaceId::new("space-b"), "=NOW()");
            space.context.scenario_label = "now-time".to_string();
            space
        };
        state.workspace_shell.active_formula_space_id = Some(space_a.formula_space_id.clone());
        state
            .workspace_shell
            .open_formula_space_order
            .push(space_a.formula_space_id.clone());
        state
            .workspace_shell
            .open_formula_space_order
            .push(space_b.formula_space_id.clone());
        state
            .workspace_shell
            .pinned_formula_space_ids
            .insert(space_b.formula_space_id.clone());
        state.formula_spaces.insert(space_a);
        state.formula_spaces.insert(space_b);
        state.global_ui_chrome.manage_formulas_open = true;

        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert!(vm.manage_formulas.is_open);
        assert_eq!(vm.manage_formulas.total_count, 2);
        assert_eq!(vm.manage_formulas.rows.len(), 2);
        // Pinned comes first.
        assert_eq!(vm.manage_formulas.rows[0].formula_space_id, "space-b");
        assert!(vm.manage_formulas.rows[0].is_pinned);
        assert_eq!(vm.manage_formulas.rows[0].display_name, "now-time");
        assert_eq!(vm.manage_formulas.rows[1].formula_space_id, "space-a");
        assert!(vm.manage_formulas.rows[1].is_active);
        assert!(!vm.manage_formulas.rows[1].is_pinned);
    }

    /// Search filter narrows by name AND formula text — case-insensitive.
    #[test]
    fn manage_formulas_search_matches_name_or_formula_text() {
        let mut state = OneCalcHostState::default();
        let space_a = {
            let mut space =
                FormulaSpaceState::new(FormulaSpaceId::new("space-a"), "=XLOOKUP(A1,B:B,C:C)");
            space.context.scenario_label = "lookups".to_string();
            space
        };
        let space_b = {
            let mut space = FormulaSpaceState::new(FormulaSpaceId::new("space-b"), "=SUM(1,2)");
            space.context.scenario_label = "totals".to_string();
            space
        };
        state.workspace_shell.active_formula_space_id = Some(space_a.formula_space_id.clone());
        state
            .workspace_shell
            .open_formula_space_order
            .push(space_a.formula_space_id.clone());
        state
            .workspace_shell
            .open_formula_space_order
            .push(space_b.formula_space_id.clone());
        state.formula_spaces.insert(space_a);
        state.formula_spaces.insert(space_b);
        state.global_ui_chrome.manage_formulas_open = true;
        // Match by formula text (case-insensitive).
        state.global_ui_chrome.manage_formulas_search_query = "xlookup".to_string();

        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert_eq!(vm.manage_formulas.rows.len(), 1);
        assert_eq!(vm.manage_formulas.rows[0].formula_space_id, "space-a");
        // The total count reflects pre-filter cardinality.
        assert_eq!(vm.manage_formulas.total_count, 2);

        // Match by display name.
        state.global_ui_chrome.manage_formulas_search_query = "totals".to_string();
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert_eq!(vm.manage_formulas.rows.len(), 1);
        assert_eq!(vm.manage_formulas.rows[0].formula_space_id, "space-b");

        // Empty query → all rows.
        state.global_ui_chrome.manage_formulas_search_query = "".to_string();
        let vm = build_home_shell_view_model(&state).expect("active formula space");
        assert_eq!(vm.manage_formulas.rows.len(), 2);
    }

    #[test]
    fn manage_formulas_preview_collapses_whitespace_and_truncates() {
        // 90-character formula text -> the preview should truncate at
        // ~80 chars with an ellipsis. Multiline whitespace collapses.
        let multiline =
            "=LET(\n  x, 1,\n  y, 2,\n  z, x + y + 100 + 200 + 300 + 400 + 500 + 600,\n  z\n)";
        let mut formula_space = FormulaSpaceState::new(FormulaSpaceId::new("multi"), multiline);
        formula_space.context.scenario_label = "multi".to_string();
        let mut state = host_state_with(formula_space);
        state.global_ui_chrome.manage_formulas_open = true;

        let vm = build_home_shell_view_model(&state).expect("active formula space");
        let row = vm.manage_formulas.rows.first().expect("at least one row");
        // No newlines survive.
        assert!(!row.formula_preview.contains('\n'));
        // Truncation marker appears on long inputs.
        if multiline.chars().count() > 80 {
            assert!(row.formula_preview.ends_with('…'));
        }
    }

    #[test]
    fn capability_context_projects_sum_semantic_profile() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("sum-profile"), "=SUM(1,2,3)");
        let mut document = sample_editor_document("=SUM(1,2,3)");
        document.editor_syntax_snapshot = crate::test_support::make_editor_syntax_snapshot(
            "sum-profile",
            "green-sum",
            vec![
                crate::test_support::make_editor_token("=", 0),
                crate::test_support::make_editor_token("SUM", 1),
            ],
        );
        attach_number_value_presentation(&mut document, 6.0, "6");
        formula_space.editor_document = Some(document);
        let state = host_state_with(formula_space);

        let vm = build_home_shell_view_model(&state).expect("active formula space");
        let sum = vm
            .capability_context
            .function_profiles
            .iter()
            .find(|row| row.surface_name == "SUM")
            .expect("SUM profile row");

        assert!(sum.reduction_sensitive);
        assert_eq!(
            sum.numerical_reduction_policy.as_deref(),
            Some("SequentialLeftFold")
        );
        assert!(!vm
            .capability_context
            .snapshot
            .oxfunc_metadata
            .semantic_kernel_metadata_versions
            .is_empty());
    }

    #[test]
    fn capability_context_projects_formula_inputs() {
        let mut formula_space = FormulaSpaceState::new(FormulaSpaceId::new("inputs"), "=Rate*10");
        formula_space.formula_input_bindings.push(
            crate::state::FormulaInputBindingState::scalar_number("Rate", 0.2),
        );
        let state = host_state_with(formula_space);

        let vm = build_home_shell_view_model(&state).expect("active formula space");

        assert_eq!(vm.capability_context.formula_inputs.len(), 1);
        assert_eq!(vm.capability_context.formula_inputs[0].label, "Rate");
        assert_eq!(
            vm.capability_context.formula_inputs[0].reference_descriptor,
            "name:Rate"
        );
        assert_eq!(vm.capability_context.formula_inputs[0].value_preview, "0.2");
    }
}
