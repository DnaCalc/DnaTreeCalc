//! The Bench formatting / conditional-formatting / locale panel
//! (BENCH_SPEC §7, bead dtc-lfz.5) and its pure view-model helpers.
//!
//! This panel is the proving ground for the Calc-wide G2 format family, so its
//! helpers are TP-pure — they read `FormattingSurface` + `ArrayWindowProjection`
//! host truth and produce plain display models with **no engine types and no
//! skin-side number formatting**. Every display string it shows for a value is
//! the host's rendered projection string; the panel never turns a number into
//! text itself (the layering law).
//!
//! Authoring status (all LIVE as of bead dtc-lfz.13 — the write verbs route
//! through the OneFormula host into OxFml's publication surface, which already
//! evaluates the typed CF model + applies font/fill + renders every locale):
//! - **Number format: LIVE.** The preset gallery and code entry author through
//!   `OneFormulaIntent::SetNumberFormat` (wired in `app.rs` via
//!   [`crate::adapter::BenchHost::set_number_format`], which re-renders the
//!   result through host truth). The live preview is that re-rendered string.
//! - **Conditional formatting: LIVE (cell-value slice) + DISPLAY (typed).** The
//!   typed CF rules already on the surface (ColorScale / DataBar / IconSet /
//!   Rank / Average) render with their thresholds, and per-cell CF outcomes
//!   render off the array window's `ArrayCellFormatProjection`. Authoring the
//!   operator-driven cell-value rules (`SetConditionalFormatRule` /
//!   `RemoveConditionalFormatRule`) is LIVE; typed-visualization authoring is a
//!   follow-on editor.
//! - **Font / fill: LIVE.** Colour pickers author `SetFontAttributes` /
//!   `SetFillAttributes`; the effective colour always comes back from OxFml.
//! - **Locale: LIVE.** The locale selector + date-system toggle author
//!   `SetLocale` / `SetDate1904`; the result re-renders through host truth.
//!
//! The layering law still holds throughout: the panel only chooses which code
//! / verb to author — every display string it shows is the host's projection.

use leptos::prelude::*;

use dnacalc_skin_ir::formula::{
    ArrayWindowProjection, ConditionalFormatRuleAuthoring,
    ConditionalFormattingThresholdProjection, ConditionalFormattingTypedRuleProjection,
    FormattingSurface, FormulaResultSurface, OneFormulaProjection, RankRuleProjection,
};

// ---------------------------------------------------------------------------
// Number format — preset gallery (authoring options, NOT skin formatting)
// ---------------------------------------------------------------------------

/// One number-format preset: a human label and the format CODE it authors
/// through `SetNumberFormat`. These are authoring shortcuts (which code to
/// send the host), never a skin-side formatter — the rendered value always
/// comes back from the projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NumberFormatPreset {
    pub label: &'static str,
    pub code: &'static str,
}

impl NumberFormatPreset {
    /// The code to author, or `None` for General (an empty code).
    #[must_use]
    pub fn authored_code(&self) -> Option<String> {
        (!self.code.is_empty()).then(|| self.code.to_string())
    }
}

/// The Bench preset gallery (BENCH_SPEC §7: General, thousands, currency,
/// date, percent, scientific). The codes mirror the host's own preset table
/// so the round-trip is faithful.
pub const NUMBER_FORMAT_PRESETS: [NumberFormatPreset; 6] = [
    NumberFormatPreset {
        label: "General",
        code: "",
    },
    NumberFormatPreset {
        label: "Thousands",
        code: "#,##0.00",
    },
    NumberFormatPreset {
        label: "Currency",
        code: "$#,##0.00",
    },
    NumberFormatPreset {
        label: "Date",
        code: "yyyy-mm-dd",
    },
    NumberFormatPreset {
        label: "Percent",
        code: "0.00%",
    },
    NumberFormatPreset {
        label: "Scientific",
        code: "0.00E+00",
    },
];

/// The format code currently authored on the surface (empty string for
/// General). Pure read — never inferred from the value.
#[must_use]
pub fn active_number_format_code(surface: &FormattingSurface) -> String {
    surface.number_format_code.clone().unwrap_or_default()
}

/// The host-rendered preview string for the current result — the ONLY display
/// string the panel shows, straight off the projection. `None` when there is
/// no value yet (empty / pending), which the panel renders as an honest hint.
#[must_use]
pub fn result_preview_text(result: &FormulaResultSurface) -> Option<String> {
    match result {
        FormulaResultSurface::Display { text, .. } => Some(text.clone()),
        FormulaResultSurface::Error { code, .. } => Some(code.clone()),
        FormulaResultSurface::Array { label, .. } => Some(label.clone()),
        FormulaResultSurface::Empty | FormulaResultSurface::Pending => None,
    }
}

// ---------------------------------------------------------------------------
// Conditional formatting — typed rule display (DISPLAY-ONLY)
// ---------------------------------------------------------------------------

/// A rendered typed CF rule: its kind, a threshold-form list, and any
/// whole-rule font/fill overrides. Pure projection of one
/// `ConditionalFormattingRuleProjection`.
#[derive(Debug, Clone, PartialEq)]
pub struct CfRuleDisplay {
    pub kind_label: &'static str,
    pub operator: Option<String>,
    pub thresholds: Vec<String>,
    pub font_color: Option<String>,
    pub fill_color: Option<String>,
}

/// Render one threshold form to a short label matching its `CfThreshold`
/// shape (BENCH_SPEC §7: "threshold editors matching CfThreshold forms").
#[must_use]
pub fn threshold_label(threshold: &ConditionalFormattingThresholdProjection) -> String {
    use ConditionalFormattingThresholdProjection as T;
    match threshold {
        T::Min => "min".to_string(),
        T::Mid => "mid".to_string(),
        T::Max => "max".to_string(),
        T::Percent(value) => format!("{value}%"),
        T::Percentile(value) => format!("{value} pctl"),
        T::Number(value) => format!("{value}"),
        T::Text(text) => text.clone(),
    }
}

/// Project all typed CF rules already on the surface to display models.
/// Rules without a typed body fall back to their bounded threshold list.
#[must_use]
pub fn cf_rule_displays(surface: &FormattingSurface) -> Vec<CfRuleDisplay> {
    surface
        .conditional_formatting_rules
        .iter()
        .map(|rule| {
            let (kind_label, thresholds) = match &rule.typed_rule {
                Some(ConditionalFormattingTypedRuleProjection::ColorScale(scale)) => (
                    "Color scale",
                    scale
                        .stops
                        .iter()
                        .map(|stop| format!("{} → {}", threshold_label(&stop.position), stop.color))
                        .collect(),
                ),
                Some(ConditionalFormattingTypedRuleProjection::DataBar(bar)) => (
                    "Data bar",
                    [bar.minimum.as_ref(), bar.maximum.as_ref()]
                        .into_iter()
                        .flatten()
                        .map(threshold_label)
                        .collect(),
                ),
                Some(ConditionalFormattingTypedRuleProjection::IconSet(icons)) => (
                    "Icon set",
                    icons.thresholds.iter().map(threshold_label).collect(),
                ),
                Some(ConditionalFormattingTypedRuleProjection::Rank(rank)) => (
                    "Rank",
                    vec![match rank {
                        RankRuleProjection::Count(count) => format!("top/bottom {count}"),
                        RankRuleProjection::Percent(percent) => format!("top/bottom {percent}%"),
                    }],
                ),
                Some(ConditionalFormattingTypedRuleProjection::Average(average)) => (
                    "Average",
                    vec![match average.stddev_multiplier {
                        Some(multiplier) => format!("{multiplier}σ from mean"),
                        None => "above/below mean".to_string(),
                    }],
                ),
                None => (
                    "Predicate",
                    rule.thresholds.iter().map(threshold_label).collect(),
                ),
            };
            CfRuleDisplay {
                kind_label,
                operator: rule.operator.clone(),
                thresholds,
                font_color: rule.font_color.clone(),
                fill_color: rule.fill_color.clone(),
            }
        })
        .collect()
}

/// One per-cell CF outcome read off an array window cell's
/// `ArrayCellFormatProjection` — the resolved font/fill, a data-bar fill
/// ratio, and/or an icon. Only cells that actually carry an outcome appear.
#[derive(Debug, Clone, PartialEq)]
pub struct CfCellOutcome {
    pub row: usize,
    pub col: usize,
    pub display_text: String,
    pub font_color: Option<String>,
    pub fill_color: Option<String>,
    pub data_bar_ratio: Option<f64>,
    pub icon: Option<String>,
}

/// Extract per-cell CF outcomes from an array window (BENCH_SPEC §7: "result
/// preview on the array window; `ArrayCellFormatProjection` already carries
/// per-cell CF outcomes"). Coordinates are absolute (window offset applied).
#[must_use]
pub fn cf_cell_outcomes(window: &ArrayWindowProjection) -> Vec<CfCellOutcome> {
    let mut outcomes = Vec::new();
    for (row_index, row) in window.cells.iter().enumerate() {
        for (col_index, cell) in row.iter().enumerate() {
            let Some(format) = &cell.format else {
                continue;
            };
            let has_outcome = format.effective_font_color.is_some()
                || format.effective_fill_color.is_some()
                || format.data_bar.is_some()
                || format.icon.is_some();
            if !has_outcome {
                continue;
            }
            outcomes.push(CfCellOutcome {
                row: window.row_offset + row_index,
                col: window.col_offset + col_index,
                display_text: cell.display_text.clone(),
                font_color: format.effective_font_color.clone(),
                fill_color: format.effective_fill_color.clone(),
                data_bar_ratio: format.data_bar.as_ref().map(|bar| bar.fill_ratio),
                icon: format
                    .icon
                    .as_ref()
                    .map(|icon| format!("{}#{}", icon.set_kind, icon.icon_index)),
            });
        }
    }
    outcomes
}

/// The per-cell CF outcomes carried by the current result, when it is an
/// array. Scalar / empty results carry none.
#[must_use]
pub fn result_cf_cell_outcomes(result: &FormulaResultSurface) -> Vec<CfCellOutcome> {
    match result {
        FormulaResultSurface::Array { window, .. } => cf_cell_outcomes(window),
        _ => Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// Locale (READ-ONLY this phase)
// ---------------------------------------------------------------------------

/// The locale display model: the surface's language tag and calendar epoch.
/// `has_tag` is false when the surface carries no tag (honest absence).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocaleDisplay {
    pub language_tag: String,
    pub has_tag: bool,
    pub date1904: bool,
}

impl LocaleDisplay {
    /// The calendar-epoch label the date1904 flag names.
    #[must_use]
    pub fn epoch_label(&self) -> &'static str {
        if self.date1904 {
            "1904 date system"
        } else {
            "1900 date system"
        }
    }
}

/// Project the locale display from the surface (BENCH_SPEC §7).
#[must_use]
pub fn locale_display(surface: &FormattingSurface) -> LocaleDisplay {
    let tag = surface.locale_language_tag.trim().to_string();
    LocaleDisplay {
        has_tag: !tag.is_empty(),
        language_tag: tag,
        date1904: surface.date1904,
    }
}

// ---------------------------------------------------------------------------
// Panel component
// ---------------------------------------------------------------------------

/// A small colour swatch + hex label, or nothing when the colour is absent.
fn colour_chip(label: &'static str, colour: Option<String>) -> AnyView {
    match colour {
        Some(value) => {
            let swatch_style = format!("background:{value}");
            let hex = value.clone();
            view! {
                <span class="dna-format__swatch-row" data-colour=value>
                    <span class="dna-format__swatch-label">{label}</span>
                    <span class="dna-format__swatch" style=swatch_style></span>
                    <span class="dna-format__swatch-hex">{hex}</span>
                </span>
            }
            .into_any()
        }
        None => view! {
            <span class="dna-format__swatch-row dna-format__swatch-row--absent">
                <span class="dna-format__swatch-label">{label}</span>
                <span class="dna-format__none">"—"</span>
            </span>
        }
        .into_any(),
    }
}

/// The formatting / CF / locale panel (BENCH_SPEC §7). Renders host truth off
/// `projection`; every control authors through a real write verb (number
/// format, font/fill colour, cell-value CF rules, locale, date system) and the
/// result re-renders through host truth — never a skin-side formatter.
#[component]
pub fn FormatPanel(
    /// The live OneFormula projection — the panel reads the FormattingSurface
    /// and the result (for the host-rendered preview + per-cell CF outcomes).
    projection: RwSignal<OneFormulaProjection>,
    /// Authors a number-format code (or `None` for General). Wired in `app.rs`
    /// to `BenchHost::set_number_format`.
    on_set_number_format: Callback<Option<String>>,
    /// Authors the font colour (hex `#RRGGBB`, or `None` to clear) —
    /// `BenchHost::set_font_color`.
    on_set_font_color: Callback<Option<String>>,
    /// Authors the fill colour — `BenchHost::set_fill_color`.
    on_set_fill_color: Callback<Option<String>>,
    /// Authors a cell-value CF rule: `None` index appends, `Some(i)` replaces —
    /// `BenchHost::set_cf_rule`.
    on_set_cf_rule: Callback<(Option<usize>, ConditionalFormatRuleAuthoring)>,
    /// Removes the CF rule at the given index — `BenchHost::remove_cf_rule`.
    on_remove_cf_rule: Callback<usize>,
    /// Switches the workspace locale (BCP-47 tag) — `BenchHost::set_locale`.
    on_set_locale: Callback<String>,
    /// Flips the date system (1900 / 1904) — `BenchHost::set_date1904`.
    on_set_date1904: Callback<bool>,
) -> impl IntoView {
    view! {
        <div class="dna-format" data-testid="format-panel">
            {number_format_section(projection, on_set_number_format)}
            {conditional_formatting_section(projection, on_set_cf_rule, on_remove_cf_rule)}
            {font_fill_section(projection, on_set_font_color, on_set_fill_color)}
            {locale_section(projection, on_set_locale, on_set_date1904)}
        </div>
    }
}

fn number_format_section(
    projection: RwSignal<OneFormulaProjection>,
    on_set_number_format: Callback<Option<String>>,
) -> AnyView {
    let presets = NUMBER_FORMAT_PRESETS
        .iter()
        .map(|preset| {
            let preset = *preset;
            let active = move || {
                projection.with(|current| active_number_format_code(&current.formatting))
                    == preset.code
            };
            view! {
                <button
                    type="button"
                    class="dna-format__preset"
                    class:dna-format__preset--active=active
                    data-format-preset=preset.label
                    data-format-code=preset.code
                    on:click=move |_| on_set_number_format.run(preset.authored_code())
                >
                    {preset.label}
                </button>
            }
        })
        .collect_view();

    view! {
        <section class="dna-format__section" data-format-section="number">
            <h4 class="dna-format__title">"Number format"</h4>
            <div class="dna-format__presets">{presets}</div>
            <label class="dna-format__code-row">
                <span class="dna-format__code-label">"Code"</span>
                <input
                    type="text"
                    class="dna-format__code"
                    data-testid="format-code"
                    placeholder="General"
                    prop:value=move || {
                        projection.with(|current| active_number_format_code(&current.formatting))
                    }
                    on:change=move |ev| {
                        let raw = event_target_value(&ev);
                        let trimmed = raw.trim();
                        on_set_number_format
                            .run((!trimmed.is_empty()).then(|| trimmed.to_string()));
                    }
                />
            </label>
            <div class="dna-format__preview-row">
                <span class="dna-format__preview-label">"Preview"</span>
                {move || {
                    match projection.with(|current| result_preview_text(&current.result)) {
                        Some(text) => view! {
                            <span class="dna-format__preview" data-testid="format-preview">
                                {text}
                            </span>
                        }
                        .into_any(),
                        None => view! {
                            <span
                                class="dna-format__preview dna-format__preview--empty"
                                data-testid="format-preview"
                                data-empty="true"
                            >
                                "author a formula to preview its formatted value"
                            </span>
                        }
                        .into_any(),
                    }
                }}
            </div>
        </section>
    }
    .into_any()
}

/// The Excel cell-value CF operators OxFml evaluates today. `between`
/// takes two thresholds; every other operator takes one.
pub const CF_OPERATORS: [(&str, &str); 7] = [
    ("greater than", "greaterThan"),
    ("≥", "greaterThanOrEqual"),
    ("less than", "lessThan"),
    ("≤", "lessThanOrEqual"),
    ("equal to", "equal"),
    ("not equal to", "notEqual"),
    ("between", "between"),
];

fn conditional_formatting_section(
    projection: RwSignal<OneFormulaProjection>,
    on_set_cf_rule: Callback<(Option<usize>, ConditionalFormatRuleAuthoring)>,
    on_remove_cf_rule: Callback<usize>,
) -> AnyView {
    view! {
        <section class="dna-format__section" data-format-section="cf">
            <h4 class="dna-format__title">"Conditional formatting"</h4>
            {move || {
                let rules = projection.with(|current| cf_rule_displays(&current.formatting));
                if rules.is_empty() {
                    view! {
                        <p
                            class="dna-format__empty"
                            data-testid="cf-empty"
                        >
                            "No conditional-formatting rules on this result."
                        </p>
                    }
                    .into_any()
                } else {
                    rules
                        .into_iter()
                        .enumerate()
                        .map(|(index, rule)| cf_rule_view(index, &rule, on_remove_cf_rule))
                        .collect_view()
                        .into_any()
                }
            }}
            {move || {
                let outcomes = projection
                    .with(|current| result_cf_cell_outcomes(&current.result));
                (!outcomes.is_empty())
                    .then(|| {
                        view! {
                            <div class="dna-format__cf-cells" data-testid="cf-cells">
                                <span class="dna-format__subtitle">"Per-cell outcomes"</span>
                                {outcomes
                                    .into_iter()
                                    .map(cf_cell_view)
                                    .collect_view()}
                            </div>
                        }
                    })
            }}
            {cf_rule_editor(on_set_cf_rule)}
        </section>
    }
    .into_any()
}

/// A minimal cell-value rule editor (BENCH_SPEC §7): pick an operator,
/// enter its threshold(s), optionally a font / fill colour, and author a
/// new rule through `SetConditionalFormatRule`. Draft state lives in
/// component-local signals; on add they build the authoring payload and
/// reset. Typed-visualization rules author through a follow-on editor.
fn cf_rule_editor(
    on_set_cf_rule: Callback<(Option<usize>, ConditionalFormatRuleAuthoring)>,
) -> AnyView {
    let operator = RwSignal::new("greaterThan".to_string());
    let threshold1 = RwSignal::new(String::new());
    let threshold2 = RwSignal::new(String::new());
    let font = RwSignal::new(String::new());
    let fill = RwSignal::new(String::new());

    let is_between = move || operator.with(|op| op == "between");

    let add = move |_| {
        let op = operator.get();
        let mut thresholds = Vec::new();
        let t1 = threshold1.get();
        if !t1.trim().is_empty() {
            thresholds.push(t1.trim().to_string());
        }
        if op == "between" {
            let t2 = threshold2.get();
            if !t2.trim().is_empty() {
                thresholds.push(t2.trim().to_string());
            }
        }
        let non_empty = |value: String| (!value.trim().is_empty()).then_some(value);
        let rule = ConditionalFormatRuleAuthoring {
            rule_kind: "cell_value".to_string(),
            operator: Some(op),
            thresholds,
            font_color: non_empty(font.get()),
            fill_color: non_empty(fill.get()),
        };
        on_set_cf_rule.run((None, rule));
        threshold1.set(String::new());
        threshold2.set(String::new());
    };

    view! {
        <div class="dna-format__cf-editor" data-testid="cf-editor">
            <span class="dna-format__subtitle">"Add a cell-value rule"</span>
            <div class="dna-format__cf-editor-row">
                <select
                    class="dna-format__cf-operator"
                    data-testid="cf-operator"
                    prop:value=move || operator.get()
                    on:change=move |ev| operator.set(event_target_value(&ev))
                >
                    {CF_OPERATORS
                        .iter()
                        .map(|(label, value)| {
                            view! { <option value=*value>{*label}</option> }
                        })
                        .collect_view()}
                </select>
                <input
                    type="text"
                    class="dna-format__cf-threshold-input"
                    data-testid="cf-threshold-1"
                    placeholder="value"
                    prop:value=move || threshold1.get()
                    on:input=move |ev| threshold1.set(event_target_value(&ev))
                />
                {move || {
                    is_between()
                        .then(|| {
                            view! {
                                <input
                                    type="text"
                                    class="dna-format__cf-threshold-input"
                                    data-testid="cf-threshold-2"
                                    placeholder="and"
                                    prop:value=move || threshold2.get()
                                    on:input=move |ev| threshold2.set(event_target_value(&ev))
                                />
                            }
                        })
                }}
            </div>
            <div class="dna-format__cf-editor-row">
                <label class="dna-format__cf-colour-pick">
                    <span class="dna-format__author-label">"font"</span>
                    <input
                        type="color"
                        data-testid="cf-font-colour"
                        prop:value=move || {
                            let value = font.get();
                            if value.is_empty() { "#000000".to_string() } else { value }
                        }
                        on:change=move |ev| font.set(event_target_value(&ev))
                    />
                </label>
                <label class="dna-format__cf-colour-pick">
                    <span class="dna-format__author-label">"fill"</span>
                    <input
                        type="color"
                        data-testid="cf-fill-colour"
                        prop:value=move || {
                            let value = fill.get();
                            if value.is_empty() { "#ffffff".to_string() } else { value }
                        }
                        on:change=move |ev| fill.set(event_target_value(&ev))
                    />
                </label>
                <button
                    type="button"
                    class="dna-format__cf-add"
                    data-testid="cf-add-rule"
                    on:click=add
                >
                    "Add rule"
                </button>
            </div>
        </div>
    }
    .into_any()
}

fn cf_rule_view(index: usize, rule: &CfRuleDisplay, on_remove_cf_rule: Callback<usize>) -> AnyView {
    let thresholds = rule
        .thresholds
        .iter()
        .cloned()
        .map(|label| view! { <li class="dna-format__cf-threshold">{label}</li> })
        .collect_view();
    let operator = rule
        .operator
        .clone()
        .map(|op| view! { <span class="dna-format__cf-op">{op}</span> });
    view! {
        <div class="dna-format__cf-rule" data-cf-rule=index.to_string() data-cf-kind=rule.kind_label>
            <div class="dna-format__cf-head">
                <span class="dna-format__cf-kind">{rule.kind_label}</span>
                {operator}
                <button
                    type="button"
                    class="dna-format__cf-remove"
                    data-testid=format!("cf-remove-{index}")
                    title="Remove rule"
                    on:click=move |_| on_remove_cf_rule.run(index)
                >
                    "×"
                </button>
            </div>
            <ul class="dna-format__cf-thresholds">{thresholds}</ul>
            <div class="dna-format__cf-colours">
                {colour_chip("font", rule.font_color.clone())}
                {colour_chip("fill", rule.fill_color.clone())}
            </div>
        </div>
    }
    .into_any()
}

fn cf_cell_view(outcome: CfCellOutcome) -> AnyView {
    let coord = format!("r{}c{}", outcome.row, outcome.col);
    let cell_style = outcome
        .fill_color
        .as_ref()
        .map(|fill| format!("background:{fill}"))
        .unwrap_or_default();
    let font_style = outcome
        .font_color
        .as_ref()
        .map(|font| format!("color:{font}"))
        .unwrap_or_default();
    let data_bar = outcome.data_bar_ratio.map(|ratio| {
        let width = format!("width:{}%", (ratio.clamp(0.0, 1.0) * 100.0).round());
        view! { <span class="dna-format__cf-bar" style=width data-bar-ratio=ratio.to_string()></span> }
    });
    let icon = outcome.icon.clone().map(|icon| {
        let icon_label = icon.clone();
        view! { <span class="dna-format__cf-icon" data-cf-icon=icon>{icon_label}</span> }
    });
    view! {
        <span
            class="dna-format__cf-cell"
            data-cf-cell=coord
            data-cf-fill=outcome.fill_color.clone().unwrap_or_default()
            data-cf-font=outcome.font_color.clone().unwrap_or_default()
            style=cell_style
        >
            <span class="dna-format__cf-cell-text" style=font_style>{outcome.display_text}</span>
            {data_bar}
            {icon}
        </span>
    }
    .into_any()
}

fn font_fill_section(
    projection: RwSignal<OneFormulaProjection>,
    on_set_font_color: Callback<Option<String>>,
    on_set_fill_color: Callback<Option<String>>,
) -> AnyView {
    view! {
        <section class="dna-format__section" data-format-section="font-fill">
            <h4 class="dna-format__title">"Font & fill"</h4>
            {move || {
                let (font, fill) = projection.with(|current| {
                    (
                        current.formatting.font_color.clone(),
                        current.formatting.fill_color.clone(),
                    )
                });
                view! {
                    <div class="dna-format__swatches" data-testid="font-fill">
                        {colour_chip("font", font)}
                        {colour_chip("fill", fill)}
                    </div>
                }
            }}
            {colour_authoring_row("font", "font-colour", projection, on_set_font_color, |surface| {
                surface.font_color.clone()
            })}
            {colour_authoring_row("fill", "fill-colour", projection, on_set_fill_color, |surface| {
                surface.fill_color.clone()
            })}
        </section>
    }
    .into_any()
}

/// A live colour-authoring row: a `type=color` picker seeded from the
/// current surface value plus a Clear button. `read` pulls the current
/// colour off the FormattingSurface; the picker authors `Some(hex)` on
/// change and Clear authors `None`. `default_hex` seeds the picker when
/// the colour is currently absent (the picker widget has no empty state).
fn colour_authoring_row(
    label: &'static str,
    testid: &'static str,
    projection: RwSignal<OneFormulaProjection>,
    on_set: Callback<Option<String>>,
    read: fn(&FormattingSurface) -> Option<String>,
) -> AnyView {
    view! {
        <div class="dna-format__author-row" data-author-row=testid>
            <span class="dna-format__author-label">{label}</span>
            <input
                type="color"
                class="dna-format__colour-input"
                data-testid=testid
                prop:value=move || {
                    projection
                        .with(|current| read(&current.formatting))
                        .unwrap_or_else(|| "#000000".to_string())
                }
                on:change=move |ev| on_set.run(Some(event_target_value(&ev)))
            />
            <button
                type="button"
                class="dna-format__clear"
                data-testid=format!("{testid}-clear")
                on:click=move |_| on_set.run(None)
            >
                "Clear"
            </button>
        </div>
    }
    .into_any()
}

/// The locale presets the selector offers — all carry a real OxFml
/// locale profile, so switching genuinely re-renders month / weekday
/// names, separators, and `General`/date values through host truth.
pub const LOCALE_PRESETS: [(&str, &str); 8] = [
    ("English (US)", "en-US"),
    ("English (UK)", "en-GB"),
    ("German", "de-DE"),
    ("French", "fr-FR"),
    ("Spanish", "es-ES"),
    ("Italian", "it-IT"),
    ("Dutch", "nl-NL"),
    ("Russian", "ru-RU"),
];

fn locale_section(
    projection: RwSignal<OneFormulaProjection>,
    on_set_locale: Callback<String>,
    on_set_date1904: Callback<bool>,
) -> AnyView {
    view! {
        <section class="dna-format__section" data-format-section="locale">
            <h4 class="dna-format__title">"Locale"</h4>
            {move || {
                let locale = projection.with(|current| locale_display(&current.formatting));
                let tag_view = if locale.has_tag {
                    view! {
                        <span
                            class="dna-format__locale-tag"
                            data-testid="format-locale"
                            data-locale-tag=locale.language_tag.clone()
                        >
                            {locale.language_tag.clone()}
                        </span>
                    }
                    .into_any()
                } else {
                    view! {
                        <span
                            class="dna-format__locale-tag dna-format__locale-tag--absent"
                            data-testid="format-locale"
                            data-locale-tag=""
                        >
                            "locale not set on this document"
                        </span>
                    }
                    .into_any()
                };
                view! {
                    <div class="dna-format__locale-row">
                        {tag_view}
                        <span class="dna-format__locale-epoch" data-date1904=locale.date1904.to_string()>
                            {locale.epoch_label()}
                        </span>
                    </div>
                }
            }}
            <div class="dna-format__author-row">
                <span class="dna-format__author-label">"locale"</span>
                <select
                    class="dna-format__locale-select"
                    data-testid="locale-select"
                    prop:value=move || {
                        projection.with(|current| current.formatting.locale_language_tag.clone())
                    }
                    on:change=move |ev| on_set_locale.run(event_target_value(&ev))
                >
                    {LOCALE_PRESETS
                        .iter()
                        .map(|(label, tag)| {
                            view! { <option value=*tag>{*label}</option> }
                        })
                        .collect_view()}
                </select>
            </div>
            <label class="dna-format__date-toggle">
                <input
                    type="checkbox"
                    data-testid="date1904-toggle"
                    prop:checked=move || projection.with(|current| current.formatting.date1904)
                    on:change=move |ev| on_set_date1904.run(event_target_checked(&ev))
                />
                <span>"1904 date system"</span>
            </label>
        </section>
    }
    .into_any()
}

/// The panel's component-scoped styles (strand `--dna-*` tokens only).
pub const FORMAT_PANEL_CSS: &str = "\
.dna-format{display:flex;flex-direction:column;gap:var(--dna-gap-4);font-size:12px}
.dna-format__section{display:flex;flex-direction:column;gap:var(--dna-gap-2)}
.dna-format__title{margin:0;font-size:12px;font-weight:600;color:var(--dna-ink)}
.dna-format__subtitle{font-size:11px;color:var(--dna-ink-3);text-transform:uppercase;letter-spacing:0.04em}
.dna-format__presets{display:flex;flex-wrap:wrap;gap:var(--dna-gap-1)}
.dna-format__preset{font:inherit;font-size:11px;color:var(--dna-ink-2);background:var(--dna-paper-2);border:1px solid var(--dna-line);border-radius:var(--dna-radius-chip);padding:2px var(--dna-gap-2);cursor:pointer}
.dna-format__preset--active{background:var(--dna-accent-soft);color:var(--dna-accent-ink);border-color:var(--dna-accent)}
.dna-format__code-row,.dna-format__preview-row{display:flex;align-items:baseline;gap:var(--dna-gap-2)}
.dna-format__code-label,.dna-format__preview-label{color:var(--dna-ink-3);min-width:3.5em}
.dna-format__code{flex:1;font:inherit;font-family:'Recursive Mono','Cascadia Code',Consolas,ui-monospace,monospace;font-size:11px;color:var(--dna-ink);background:var(--dna-paper);border:1px solid var(--dna-line);border-radius:var(--dna-radius-chip);padding:2px var(--dna-gap-2)}
.dna-format__preview{font-family:'Recursive Mono','Cascadia Code',Consolas,ui-monospace,monospace;color:var(--dna-value-ink);font-weight:600}
.dna-format__preview--empty{color:var(--dna-ink-3);font-style:italic;font-weight:400}
.dna-format__empty{margin:0;color:var(--dna-ink-3);font-style:italic}
.dna-format__cf-rule{display:flex;flex-direction:column;gap:var(--dna-gap-1);padding:var(--dna-gap-2);background:var(--dna-paper-2);border-radius:var(--dna-radius-chip)}
.dna-format__cf-head{display:flex;align-items:baseline;gap:var(--dna-gap-2)}
.dna-format__cf-kind{font-weight:600;color:var(--dna-accent-ink)}
.dna-format__cf-op{color:var(--dna-ink-3)}
.dna-format__cf-thresholds{margin:0;padding-left:var(--dna-gap-4);color:var(--dna-ink-2)}
.dna-format__cf-threshold{font-family:'Recursive Mono','Cascadia Code',Consolas,ui-monospace,monospace}
.dna-format__cf-colours{display:flex;gap:var(--dna-gap-3)}
.dna-format__cf-cells{display:flex;flex-wrap:wrap;align-items:center;gap:var(--dna-gap-2)}
.dna-format__cf-cell{display:inline-flex;align-items:center;gap:2px;padding:0 var(--dna-gap-2);border-radius:var(--dna-radius-chip);border:1px solid var(--dna-line)}
.dna-format__cf-cell-text{font-family:'Recursive Mono','Cascadia Code',Consolas,ui-monospace,monospace}
.dna-format__cf-bar{display:inline-block;height:8px;min-width:2px;background:var(--dna-accent);border-radius:2px}
.dna-format__cf-icon{font-size:10px;color:var(--dna-ink-3)}
.dna-format__swatches{display:flex;gap:var(--dna-gap-4)}
.dna-format__swatch-row{display:inline-flex;align-items:center;gap:var(--dna-gap-1)}
.dna-format__swatch-label{color:var(--dna-ink-3)}
.dna-format__swatch{display:inline-block;width:12px;height:12px;border-radius:3px;border:1px solid var(--dna-line)}
.dna-format__swatch-hex{font-family:'Recursive Mono','Cascadia Code',Consolas,ui-monospace,monospace;color:var(--dna-ink-2)}
.dna-format__none{color:var(--dna-ink-3)}
.dna-format__locale-row{display:flex;align-items:baseline;gap:var(--dna-gap-3)}
.dna-format__locale-tag{font-family:'Recursive Mono','Cascadia Code',Consolas,ui-monospace,monospace;color:var(--dna-ink)}
.dna-format__locale-tag--absent{color:var(--dna-ink-3);font-style:italic;font-family:inherit}
.dna-format__locale-epoch{color:var(--dna-ink-2)}
.dna-format__author-row{display:flex;align-items:center;gap:var(--dna-gap-2)}
.dna-format__author-label{color:var(--dna-ink-3);min-width:3.5em}
.dna-format__colour-input{width:28px;height:20px;padding:0;border:1px solid var(--dna-line);border-radius:var(--dna-radius-chip);background:var(--dna-paper);cursor:pointer}
.dna-format__clear{font:inherit;font-size:11px;color:var(--dna-ink-2);background:var(--dna-paper-2);border:1px solid var(--dna-line);border-radius:var(--dna-radius-chip);padding:1px var(--dna-gap-2);cursor:pointer}
.dna-format__locale-select,.dna-format__cf-operator{font:inherit;font-size:11px;color:var(--dna-ink);background:var(--dna-paper);border:1px solid var(--dna-line);border-radius:var(--dna-radius-chip);padding:2px var(--dna-gap-2)}
.dna-format__date-toggle{display:flex;align-items:center;gap:var(--dna-gap-2);color:var(--dna-ink-2)}
.dna-format__cf-editor{display:flex;flex-direction:column;gap:var(--dna-gap-2);padding:var(--dna-gap-2);background:var(--dna-paper-2);border-radius:var(--dna-radius-chip)}
.dna-format__cf-editor-row{display:flex;flex-wrap:wrap;align-items:center;gap:var(--dna-gap-2)}
.dna-format__cf-threshold-input{width:5em;font:inherit;font-family:'Recursive Mono','Cascadia Code',Consolas,ui-monospace,monospace;font-size:11px;color:var(--dna-ink);background:var(--dna-paper);border:1px solid var(--dna-line);border-radius:var(--dna-radius-chip);padding:2px var(--dna-gap-2)}
.dna-format__cf-colour-pick{display:inline-flex;align-items:center;gap:var(--dna-gap-1)}
.dna-format__cf-colour-pick input{width:24px;height:18px;padding:0;border:1px solid var(--dna-line);border-radius:var(--dna-radius-chip);cursor:pointer}
.dna-format__cf-add{font:inherit;font-size:11px;color:var(--dna-accent-ink);background:var(--dna-accent-soft);border:1px solid var(--dna-accent);border-radius:var(--dna-radius-chip);padding:2px var(--dna-gap-3);cursor:pointer}
.dna-format__cf-remove{margin-left:auto;font:inherit;font-size:13px;line-height:1;color:var(--dna-ink-3);background:transparent;border:none;cursor:pointer;padding:0 var(--dna-gap-1)}
.dna-format__cf-remove:hover{color:var(--dna-danger-ink,var(--dna-ink))}
";

#[cfg(test)]
mod tests {
    use super::*;
    use dnacalc_skin_ir::formula::{
        ArrayCellFormatProjection, ArrayWindowCellProjection, ColorScaleRuleProjection,
        ColorScaleStopProjection, ConditionalFormattingRuleProjection, DataBarDirectionProjection,
        DataBarFillProjection,
    };

    #[test]
    fn presets_carry_the_bench_gallery_and_general_authors_none() {
        let labels: Vec<_> = NUMBER_FORMAT_PRESETS.iter().map(|p| p.label).collect();
        assert_eq!(
            labels,
            ["General", "Thousands", "Currency", "Date", "Percent", "Scientific"]
        );
        // General authors an absent code (the host's General default).
        assert_eq!(NUMBER_FORMAT_PRESETS[0].authored_code(), None);
        // A non-empty preset authors its exact code.
        assert_eq!(
            NUMBER_FORMAT_PRESETS[1].authored_code().as_deref(),
            Some("#,##0.00")
        );
    }

    #[test]
    fn active_code_and_preview_read_host_truth_only() {
        // Preview is the host's rendered string, never a skin formatter.
        let display = FormulaResultSurface::Display {
            text: "1,234.50".to_string(),
            value: Default::default(),
            applied_font_color: None,
            applied_fill_color: None,
        };
        assert_eq!(result_preview_text(&display).as_deref(), Some("1,234.50"));
        // Empty / pending carry no preview (honest hint, not a fake zero).
        assert_eq!(result_preview_text(&FormulaResultSurface::Empty), None);
        assert_eq!(result_preview_text(&FormulaResultSurface::Pending), None);

        let mut surface = FormattingSurface::default();
        assert_eq!(active_number_format_code(&surface), "");
        surface.number_format_code = Some("0.00%".to_string());
        assert_eq!(active_number_format_code(&surface), "0.00%");
    }

    #[test]
    fn color_scale_rule_projects_kind_and_thresholds() {
        let mut surface = FormattingSurface::default();
        surface
            .conditional_formatting_rules
            .push(ConditionalFormattingRuleProjection {
                operator: None,
                thresholds: Vec::new(),
                font_color: None,
                fill_color: None,
                typed_rule: Some(ConditionalFormattingTypedRuleProjection::ColorScale(
                    ColorScaleRuleProjection {
                        stops: vec![
                            ColorScaleStopProjection {
                                position: ConditionalFormattingThresholdProjection::Min,
                                color: "#FF0000".to_string(),
                            },
                            ColorScaleStopProjection {
                                position: ConditionalFormattingThresholdProjection::Max,
                                color: "#00FF00".to_string(),
                            },
                        ],
                    },
                )),
            });

        let displays = cf_rule_displays(&surface);
        assert_eq!(displays.len(), 1);
        assert_eq!(displays[0].kind_label, "Color scale");
        assert_eq!(
            displays[0].thresholds,
            vec!["min → #FF0000".to_string(), "max → #00FF00".to_string()]
        );
    }

    /// A CF color-scale rule shows per-cell outcomes on the array window
    /// (bead dtc-lfz.5, BENCH_SPEC §7): the panel reads the resolved font/fill
    /// off each cell's `ArrayCellFormatProjection`. Only cells carrying an
    /// outcome appear, and coordinates are absolute (window offset applied).
    #[test]
    fn cf_cell_outcomes_read_per_cell_projection_with_absolute_coords() {
        let scaled_cell = ArrayWindowCellProjection {
            display_text: "9".to_string(),
            value: None,
            format: Some(ArrayCellFormatProjection {
                effective_font_color: Some("#FFFFFF".to_string()),
                effective_fill_color: Some("#006600".to_string()),
                data_bar: Some(DataBarFillProjection {
                    fill_ratio: 0.9,
                    bar_color: "#006600".to_string(),
                    direction: DataBarDirectionProjection::Right,
                    show_bar_only: false,
                }),
                icon: None,
            }),
        };
        let plain_cell = ArrayWindowCellProjection {
            display_text: "1".to_string(),
            value: None,
            format: None,
        };
        let window = ArrayWindowProjection {
            total_rows: 2,
            total_cols: 1,
            row_offset: 1,
            col_offset: 0,
            cells: vec![vec![plain_cell], vec![scaled_cell]],
        };

        let outcomes = cf_cell_outcomes(&window);
        assert_eq!(
            outcomes.len(),
            1,
            "only the cell with a CF outcome is surfaced (the plain cell is skipped)"
        );
        let outcome = &outcomes[0];
        // row_offset 1 + local row 1 => absolute row 2.
        assert_eq!((outcome.row, outcome.col), (2, 0));
        assert_eq!(outcome.fill_color.as_deref(), Some("#006600"));
        assert_eq!(outcome.font_color.as_deref(), Some("#FFFFFF"));
        assert_eq!(outcome.data_bar_ratio, Some(0.9));
        assert_eq!(outcome.display_text, "9");

        // The same window surfaces through the result-surface helper.
        let result = FormulaResultSurface::Array {
            total_rows: 2,
            total_cols: 1,
            label: "2×1".to_string(),
            window,
            truncated: false,
        };
        assert_eq!(result_cf_cell_outcomes(&result), outcomes);
        // A scalar result carries no per-cell outcomes.
        assert!(result_cf_cell_outcomes(&FormulaResultSurface::Empty).is_empty());
    }

    /// Locale display reflects the surface verbatim (bead dtc-lfz.5): the tag
    /// and the date1904 epoch, with honest absence when no tag is set.
    #[test]
    fn locale_display_reflects_the_surface() {
        let mut surface = FormattingSurface {
            locale_language_tag: "af-ZA".to_string(),
            date1904: true,
            ..FormattingSurface::default()
        };
        let locale = locale_display(&surface);
        assert!(locale.has_tag);
        assert_eq!(locale.language_tag, "af-ZA");
        assert!(locale.date1904);
        assert_eq!(locale.epoch_label(), "1904 date system");

        // No tag => honest absence, 1900 epoch by default.
        surface.locale_language_tag = "   ".to_string();
        surface.date1904 = false;
        let absent = locale_display(&surface);
        assert!(!absent.has_tag);
        assert!(absent.language_tag.is_empty());
        assert_eq!(absent.epoch_label(), "1900 date system");
    }
}
