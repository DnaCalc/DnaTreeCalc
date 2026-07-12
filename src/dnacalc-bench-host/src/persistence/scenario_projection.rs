//! Pure projection between `FormulaSpaceState` and the persisted
//! `Scenario` shape. No I/O — those live in `browser_file_io.rs` and
//! the (eventual) Tauri equivalent.
//!
//! For slice 1b, this layer maps the live state's actually-populated
//! fields into the schema. Fields that exist in the schema but are
//! not yet wired in the host (most of `Context.publication_context`,
//! the host_profile detail, locale id, etc.) round-trip as their
//! defaults — slice 2 (Excel-native fidelity) and the eventual
//! formatting-controls work fill them in. The schema slot is
//! reserved either way.

use crate::persistence::formula_file::{
    CfAverageRuleOptions, CfColorScaleRuleOptions, CfColorScaleStop, CfDataBarDirection,
    CfDataBarRuleOptions, CfIconSetRuleOptions, CfRank, CfRankRuleOptions, CfRule, CfThreshold,
    CfTypedRule, Context, Entry, EntryMode, HostProfile, Identity, Locale, PublicationContext,
    Scenario, UiPreferences,
};
use crate::state::{
    FormulaAverageRuleOptions, FormulaColorScaleRuleOptions, FormulaColorScaleStop,
    FormulaConditionalFormattingRank, FormulaConditionalFormattingRule,
    FormulaConditionalFormattingThreshold, FormulaConditionalFormattingTypedRule,
    FormulaDataBarDirection, FormulaDataBarRuleOptions, FormulaIconSetRuleOptions,
    FormulaRankRuleOptions, FormulaSpaceState,
};
use crate::ui::editor::state::EditorEntryMode;

/// Project the live `FormulaSpaceState` into a `Scenario` ready for
/// serialisation. The caller is responsible for supplying the
/// timestamps (which depend on platform clock — see browser/tauri
/// adapters).
///
/// `created_at` should be threaded through from the formula's
/// existing identity when one exists, and supplied as "now" only on
/// the very first save. `modified_at` is "now" on every save.
pub fn formula_space_to_scenario(
    formula_space: &FormulaSpaceState,
    created_at_iso8601_utc: String,
    modified_at_iso8601_utc: String,
) -> Scenario {
    let entry_mode = match EditorEntryMode::classify(&formula_space.raw_entered_cell_text) {
        EditorEntryMode::Formula => EntryMode::Formula,
        EditorEntryMode::Value => EntryMode::Value,
        EditorEntryMode::Text => EntryMode::Text,
        EditorEntryMode::Empty => EntryMode::Empty,
    };

    let synthetic_default_label =
        formula_space.context.scenario_label == formula_space.formula_space_id.as_str();
    let display_name = if synthetic_default_label {
        // Synthetic labels (the `untitled-N` ids the host auto-generates
        // when a formula has no user-given name) are not user-meaningful;
        // empty `name` is more honest in the persisted file. The Identity
        // `id` still carries the synthetic id for stability.
        String::new()
    } else {
        formula_space.context.scenario_label.clone()
    };

    let formatting = &formula_space.formatting;
    Scenario {
        identity: Identity {
            id: formula_space.formula_space_id.as_str().to_string(),
            name: display_name,
            created_at: created_at_iso8601_utc,
            modified_at: modified_at_iso8601_utc,
        },
        entry: Entry {
            mode: entry_mode,
            text: formula_space.raw_entered_cell_text.clone(),
        },
        // Slice 5 — formatting fields drive PublicationContext +
        // Locale. Other Context fields (host_profile,
        // scenario_policy beyond default) remain reserved schema
        // slots until their UI controls land.
        context: Context {
            host_profile: HostProfile::default(),
            locale: Locale {
                id: String::new(),
                date1904: formatting.date1904,
            },
            publication_context: PublicationContext {
                format_profile: String::new(),
                number_format_code: formatting.number_format_code.clone(),
                style_id: String::new(),
                font_color: formatting.font_color.clone(),
                fill_color: formatting.fill_color.clone(),
                style_hierarchy: Vec::new(),
                cf_rules: formatting
                    .conditional_formatting_rules
                    .iter()
                    .map(host_cf_rule_to_persisted)
                    .collect(),
            },
            scenario_policy: formatting.scenario_policy,
        },
        ui_preferences: UiPreferences {
            formula_drill_expanded: formula_space.formula_drill_open,
            // `result_drill_open` is not yet a state field; reserve.
            result_drill_expanded: false,
            expanded_editor: formula_space.expanded_editor,
        },
        // Compare bundles aren't yet attached to FormulaSpaceState
        // (no in-memory home for them yet — they live on the
        // persisted file). Slice 4 ships the format slot; the
        // workspace state field that mirrors them is a follow-up
        // alongside the Compare-with-Excel UI surface.
        bundles: Vec::new(),
        // Unknown-element preservation: not yet plumbed into the
        // host state. When the host opens a file via slice 1b's
        // file picker the full LoadedFormula (including the
        // unknowns vec) is dropped — first save loses them. A
        // follow-up bead lifts them onto FormulaSpaceState so
        // they survive the open→edit→save round-trip.
        unknown_root_xml: Vec::new(),
    }
}

/// Apply a loaded `Scenario` to an existing `FormulaSpaceState`,
/// overwriting the live fields. The caller decides whether to apply
/// to the active formula space (replacing it) or to insert a new
/// space and switch to it; this helper just mutates a given target.
///
/// Post-condition:
/// - `raw_entered_cell_text` is the loaded entry text.
/// - `committed_cell_text` is set equal to `raw_entered_cell_text`,
///   so the breadcrumb's dirty marker reads `false` immediately
///   after loading (the user has not yet edited).
/// - `context.scenario_label` is the loaded display name when the
///   loaded `name` is non-empty; otherwise the `id` is used as the
///   label so the breadcrumb has something to render.
/// - UI prefs follow the loaded scenario.
/// - `load_diagnostics` carries any loader warnings (slice 3).
pub fn apply_loaded_scenario_to_formula_space(
    formula_space: &mut FormulaSpaceState,
    scenario: Scenario,
) {
    apply_loaded_scenario_with_diagnostics(formula_space, scenario, Vec::new());
}

/// Variant that also stamps the loader's diagnostics into the
/// formula-space state. The status-foot renders a warning chip
/// while `load_diagnostics` is non-empty; cleared on save.
pub fn apply_loaded_scenario_with_diagnostics(
    formula_space: &mut FormulaSpaceState,
    scenario: Scenario,
    diagnostics: Vec<crate::persistence::LoadDiagnostic>,
) {
    formula_space.raw_entered_cell_text = scenario.entry.text.clone();
    formula_space.committed_cell_text = Some(scenario.entry.text.clone());
    formula_space.proofed_cell_text = Some(scenario.entry.text.clone());
    formula_space.editor_surface_state =
        crate::ui::editor::state::EditorSurfaceState::for_text(&scenario.entry.text);
    formula_space.editor_document = None;
    formula_space.completion_help = crate::state::CompletionHelpState::default();
    formula_space.completion_popup =
        crate::services::completion_popup::CompletionPopupState::default();
    formula_space.completion_popup_suppressed_until_next_input = false;
    formula_space.array_preview = None;
    formula_space.latest_evaluation_summary = None;
    formula_space.effective_display_summary = None;

    let label = if scenario.identity.name.is_empty() {
        scenario.identity.id.clone()
    } else {
        scenario.identity.name.clone()
    };
    formula_space.context.scenario_label = label;

    formula_space.formula_drill_open = scenario.ui_preferences.formula_drill_expanded;
    formula_space.expanded_editor = scenario.ui_preferences.expanded_editor;
    formula_space.load_diagnostics = diagnostics;

    // Slice 5: formatting state mirrors the persisted PublicationContext
    // + Locale so the UI's formatting-controls row reflects what was
    // saved.
    let conditional_formatting_rules: Vec<FormulaConditionalFormattingRule> = scenario
        .context
        .publication_context
        .cf_rules
        .iter()
        .filter_map(persisted_cf_rule_to_host)
        .collect();
    formula_space.formatting = crate::state::FormulaFormattingState {
        number_format_code: scenario.context.publication_context.number_format_code,
        font_color: scenario.context.publication_context.font_color,
        fill_color: scenario.context.publication_context.fill_color,
        date1904: scenario.context.locale.date1904,
        scenario_policy: scenario.context.scenario_policy,
        conditional_formatting_rules,
    };
}

/// Map a host CF rule (result-hero) to the persisted shape. `range`
/// stays empty — that's the marker downstream uses to recognise the
/// rule belongs to the formula display, not a worksheet range.
fn host_cf_rule_to_persisted(rule: &FormulaConditionalFormattingRule) -> CfRule {
    CfRule {
        range: String::new(),
        formula: None,
        rule_kind: Some(rule.rule_kind.clone()),
        operator: rule.operator.clone(),
        thresholds: rule.thresholds.clone(),
        font_color: rule.font_color.clone(),
        fill_color: rule.fill_color.clone(),
        typed_rule: rule.typed_rule.as_ref().map(host_typed_rule_to_persisted),
    }
}

/// Lift a persisted `CfRule` back to the host's result-hero shape.
/// Returns `None` when the rule is the *worksheet-range* flavour
/// (`range` populated) — those persist for full SpreadsheetML
/// fidelity but aren't owned by the formula's CF panel; the
/// host-side panel only round-trips its own rules.
fn persisted_cf_rule_to_host(rule: &CfRule) -> Option<FormulaConditionalFormattingRule> {
    if !rule.range.is_empty() {
        return None;
    }
    let rule_kind = rule
        .rule_kind
        .clone()
        .unwrap_or_else(|| "cell_value".to_string());
    // OxFml W073 (`HANDOFF-DNAONECALC-012`, 2026-05-04 update)
    // ignores bounded `thresholds` for the seven typed families, so
    // stale entries in older saved files would just sit on the rule
    // unread. Drop them at load time so the in-memory state stays
    // canonical and a subsequent save no longer carries them.
    let thresholds = if is_w073_typed_kind(&rule_kind) {
        Vec::new()
    } else {
        rule.thresholds.clone()
    };
    Some(FormulaConditionalFormattingRule {
        rule_kind,
        operator: rule.operator.clone(),
        thresholds,
        font_color: rule.font_color.clone(),
        fill_color: rule.fill_color.clone(),
        typed_rule: rule.typed_rule.as_ref().map(persisted_typed_rule_to_host),
    })
}

/// Whether a CF rule kind is one of OxFml W073's seven typed-only
/// families (`colorScale`, `dataBar`, `iconSet`, `top`, `bottom`,
/// `aboveAverage`, `belowAverage`).
fn is_w073_typed_kind(rule_kind: &str) -> bool {
    matches!(
        rule_kind.to_ascii_lowercase().as_str(),
        "colorscale" | "databar" | "iconset" | "top" | "bottom" | "aboveaverage" | "belowaverage"
    )
}

fn host_typed_rule_to_persisted(rule: &FormulaConditionalFormattingTypedRule) -> CfTypedRule {
    CfTypedRule {
        color_scale: rule
            .color_scale
            .as_ref()
            .map(|options| CfColorScaleRuleOptions {
                stops: options
                    .stops
                    .iter()
                    .map(|stop| CfColorScaleStop {
                        position: host_threshold_to_persisted(&stop.position),
                        color: stop.color.clone(),
                    })
                    .collect(),
            }),
        data_bar: rule.data_bar.as_ref().map(|options| CfDataBarRuleOptions {
            minimum: options.minimum.as_ref().map(host_threshold_to_persisted),
            maximum: options.maximum.as_ref().map(host_threshold_to_persisted),
            bar_color: options.bar_color.clone(),
            direction: options.direction.map(|direction| match direction {
                FormulaDataBarDirection::Left => CfDataBarDirection::Left,
                FormulaDataBarDirection::Right => CfDataBarDirection::Right,
            }),
            show_bar_only: options.show_bar_only,
        }),
        icon_set: rule.icon_set.as_ref().map(|options| CfIconSetRuleOptions {
            set_kind: options.set_kind.clone(),
            thresholds: options
                .thresholds
                .iter()
                .map(host_threshold_to_persisted)
                .collect(),
        }),
        rank: rule.rank.as_ref().map(|options| CfRankRuleOptions {
            rank: match &options.rank {
                FormulaConditionalFormattingRank::Count(count) => CfRank::Count(*count),
                FormulaConditionalFormattingRank::Percent(value) => CfRank::Percent(*value),
            },
        }),
        average: rule.average.as_ref().map(|options| CfAverageRuleOptions {
            include_equal: options.include_equal,
            stddev_multiplier: options.stddev_multiplier,
        }),
    }
}

fn persisted_typed_rule_to_host(rule: &CfTypedRule) -> FormulaConditionalFormattingTypedRule {
    FormulaConditionalFormattingTypedRule {
        color_scale: rule
            .color_scale
            .as_ref()
            .map(|options| FormulaColorScaleRuleOptions {
                stops: options
                    .stops
                    .iter()
                    .map(|stop| FormulaColorScaleStop {
                        position: persisted_threshold_to_host(&stop.position),
                        color: stop.color.clone(),
                    })
                    .collect(),
            }),
        data_bar: rule
            .data_bar
            .as_ref()
            .map(|options| FormulaDataBarRuleOptions {
                minimum: options.minimum.as_ref().map(persisted_threshold_to_host),
                maximum: options.maximum.as_ref().map(persisted_threshold_to_host),
                bar_color: options.bar_color.clone(),
                direction: options.direction.map(|direction| match direction {
                    CfDataBarDirection::Left => FormulaDataBarDirection::Left,
                    CfDataBarDirection::Right => FormulaDataBarDirection::Right,
                }),
                show_bar_only: options.show_bar_only,
            }),
        icon_set: rule
            .icon_set
            .as_ref()
            .map(|options| FormulaIconSetRuleOptions {
                set_kind: options.set_kind.clone(),
                thresholds: options
                    .thresholds
                    .iter()
                    .map(persisted_threshold_to_host)
                    .collect(),
            }),
        rank: rule.rank.as_ref().map(|options| FormulaRankRuleOptions {
            rank: match &options.rank {
                CfRank::Count(count) => FormulaConditionalFormattingRank::Count(*count),
                CfRank::Percent(value) => FormulaConditionalFormattingRank::Percent(*value),
            },
        }),
        average: rule
            .average
            .as_ref()
            .map(|options| FormulaAverageRuleOptions {
                include_equal: options.include_equal,
                stddev_multiplier: options.stddev_multiplier,
            }),
    }
}

fn host_threshold_to_persisted(threshold: &FormulaConditionalFormattingThreshold) -> CfThreshold {
    match threshold {
        FormulaConditionalFormattingThreshold::Min => CfThreshold::Min,
        FormulaConditionalFormattingThreshold::Mid => CfThreshold::Mid,
        FormulaConditionalFormattingThreshold::Max => CfThreshold::Max,
        FormulaConditionalFormattingThreshold::Percent(value) => CfThreshold::Percent(*value),
        FormulaConditionalFormattingThreshold::Percentile(value) => CfThreshold::Percentile(*value),
        FormulaConditionalFormattingThreshold::Number(value) => CfThreshold::Number(*value),
    }
}

fn persisted_threshold_to_host(threshold: &CfThreshold) -> FormulaConditionalFormattingThreshold {
    match threshold {
        CfThreshold::Min => FormulaConditionalFormattingThreshold::Min,
        CfThreshold::Mid => FormulaConditionalFormattingThreshold::Mid,
        CfThreshold::Max => FormulaConditionalFormattingThreshold::Max,
        CfThreshold::Percent(value) => FormulaConditionalFormattingThreshold::Percent(*value),
        CfThreshold::Percentile(value) => FormulaConditionalFormattingThreshold::Percentile(*value),
        CfThreshold::Number(value) => FormulaConditionalFormattingThreshold::Number(*value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ids::FormulaSpaceId;
    use crate::state::FormulaSpaceState;

    #[test]
    fn formula_space_with_user_label_round_trips_into_identity_name() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=SUM(1,2,3)");
        formula_space.context.scenario_label = "invoice-eu-tax".to_string();

        let scenario = formula_space_to_scenario(
            &formula_space,
            "2026-04-22T10:14:22Z".to_string(),
            "2026-04-22T10:14:22Z".to_string(),
        );

        assert_eq!(scenario.identity.id, "untitled-1");
        assert_eq!(scenario.identity.name, "invoice-eu-tax");
        assert_eq!(scenario.entry.text, "=SUM(1,2,3)");
        assert_eq!(scenario.entry.mode, EntryMode::Formula);
    }

    #[test]
    fn synthetic_default_label_projects_to_empty_name() {
        // FormulaSpaceState::new auto-sets scenario_label = formula_space_id.
        // The persisted file should NOT carry the synthetic id as the
        // name — empty is more honest, and the breadcrumb fallback to
        // `id` happens at apply-time.
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "=1");
        let scenario =
            formula_space_to_scenario(&formula_space, "now".to_string(), "now".to_string());
        assert_eq!(scenario.identity.id, "untitled-1");
        assert_eq!(scenario.identity.name, "");
    }

    #[test]
    fn apply_loaded_scenario_clears_dirty_marker() {
        let mut formula_space =
            FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "live edit text");
        // Simulate the user having committed an earlier text.
        formula_space.committed_cell_text = Some("live edit text".to_string());

        let scenario = Scenario {
            identity: Identity {
                id: "loaded-id".to_string(),
                name: "loaded-name".to_string(),
                created_at: "2026-04-22T10:14:22Z".to_string(),
                modified_at: "2026-04-22T10:14:22Z".to_string(),
            },
            entry: Entry {
                mode: EntryMode::Formula,
                text: "=A1+B1".to_string(),
            },
            ui_preferences: UiPreferences {
                formula_drill_expanded: true,
                result_drill_expanded: false,
                expanded_editor: true,
            },
            ..Scenario::default()
        };
        apply_loaded_scenario_to_formula_space(&mut formula_space, scenario);

        // Loaded text replaces both raw and committed → dirty=false
        // immediately after loading.
        assert_eq!(formula_space.raw_entered_cell_text, "=A1+B1");
        assert_eq!(formula_space.committed_cell_text.as_deref(), Some("=A1+B1"),);
        assert_eq!(formula_space.context.scenario_label, "loaded-name");
        assert!(formula_space.formula_drill_open);
        assert!(formula_space.expanded_editor);
        assert!(formula_space.editor_document.is_none());
    }

    #[test]
    fn apply_loaded_scenario_with_empty_name_falls_back_to_id_label() {
        let mut formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), "");
        let scenario = Scenario {
            identity: Identity {
                id: "imported-from-disk".to_string(),
                ..Identity::default()
            },
            entry: Entry {
                mode: EntryMode::Empty,
                text: String::new(),
            },
            ..Scenario::default()
        };
        apply_loaded_scenario_to_formula_space(&mut formula_space, scenario);
        assert_eq!(formula_space.context.scenario_label, "imported-from-disk",);
    }

    #[test]
    fn formatting_state_round_trips_through_publication_context_and_locale() {
        // Slice 5: FormulaFormattingState mutations must travel into
        // the persisted Scenario's PublicationContext + Locale, then
        // come back unchanged on load.
        let mut formula_space = FormulaSpaceState::new(FormulaSpaceId::new("f-1"), "=A1");
        formula_space.formatting = crate::state::FormulaFormattingState {
            number_format_code: "$#,##0.00".to_string(),
            font_color: "#112233".to_string(),
            fill_color: "#445566".to_string(),
            date1904: true,
            scenario_policy: crate::persistence::ScenarioPolicy::Deterministic,
            conditional_formatting_rules: Vec::new(),
        };

        let scenario =
            formula_space_to_scenario(&formula_space, "now".to_string(), "now".to_string());
        assert_eq!(
            scenario.context.publication_context.number_format_code,
            "$#,##0.00",
        );
        assert_eq!(scenario.context.publication_context.font_color, "#112233");
        assert_eq!(scenario.context.publication_context.fill_color, "#445566");
        assert!(scenario.context.locale.date1904);

        // Apply the same scenario back into a fresh formula space —
        // the formatting state must round-trip verbatim.
        let mut destination = FormulaSpaceState::new(FormulaSpaceId::new("f-2"), "");
        apply_loaded_scenario_to_formula_space(&mut destination, scenario);
        assert_eq!(destination.formatting.number_format_code, "$#,##0.00");
        assert_eq!(destination.formatting.font_color, "#112233");
        assert_eq!(destination.formatting.fill_color, "#445566");
        assert!(destination.formatting.date1904);
    }

    /// CF rules + ScenarioPolicy round-trip through the projection.
    /// Worksheet-range CF rules in the persisted shape (those with
    /// `range != ""`) are deliberately filtered out on load — they
    /// belong to SpreadsheetML's worksheet block, not the host's
    /// result-hero panel.
    #[test]
    fn cf_rules_and_scenario_policy_round_trip_through_projection() {
        let mut formula_space = FormulaSpaceState::new(FormulaSpaceId::new("f-cf"), "=A1");
        formula_space.formatting = crate::state::FormulaFormattingState {
            number_format_code: "0.00".to_string(),
            font_color: String::new(),
            fill_color: String::new(),
            date1904: false,
            scenario_policy: crate::persistence::ScenarioPolicy::LiveRecalc,
            conditional_formatting_rules: vec![
                crate::state::FormulaConditionalFormattingRule {
                    rule_kind: "cell_value".to_string(),
                    operator: Some("greaterThan".to_string()),
                    thresholds: vec!["0".to_string()],
                    font_color: Some("#205F2A".to_string()),
                    fill_color: Some("#E6F2D9".to_string()),
                    typed_rule: None,
                },
                crate::state::FormulaConditionalFormattingRule {
                    rule_kind: "cell_value".to_string(),
                    operator: Some("lessThan".to_string()),
                    thresholds: vec!["0".to_string()],
                    font_color: Some("#882020".to_string()),
                    fill_color: Some("#F8D9D9".to_string()),
                    typed_rule: None,
                },
            ],
        };

        let scenario =
            formula_space_to_scenario(&formula_space, "now".to_string(), "now".to_string());
        assert_eq!(
            scenario.context.scenario_policy,
            crate::persistence::ScenarioPolicy::LiveRecalc,
        );
        assert_eq!(scenario.context.publication_context.cf_rules.len(), 2);
        assert_eq!(
            scenario.context.publication_context.cf_rules[0].rule_kind,
            Some("cell_value".to_string()),
        );
        assert_eq!(
            scenario.context.publication_context.cf_rules[0].operator,
            Some("greaterThan".to_string()),
        );
        assert_eq!(
            scenario.context.publication_context.cf_rules[0].thresholds,
            vec!["0".to_string()],
        );
        // Result-hero rules persist with empty `range`.
        assert!(scenario.context.publication_context.cf_rules[0]
            .range
            .is_empty());

        // Round-trip back. Add a worksheet-range CF rule into the
        // scenario before re-applying — the host's panel must NOT pick
        // that one up, so the destination's `conditional_formatting_rules`
        // stays at the two host-authored rules.
        let mut scenario_with_extra = scenario.clone();
        scenario_with_extra
            .context
            .publication_context
            .cf_rules
            .push(crate::persistence::CfRule {
                range: "A1:A10".to_string(),
                formula: Some("=A1>5".to_string()),
                rule_kind: Some("CellIs".to_string()),
                operator: None,
                thresholds: Vec::new(),
                font_color: None,
                fill_color: None,
                typed_rule: None,
            });

        let mut destination = FormulaSpaceState::new(FormulaSpaceId::new("f-cf-back"), "");
        apply_loaded_scenario_to_formula_space(&mut destination, scenario_with_extra);

        assert_eq!(
            destination.formatting.scenario_policy,
            crate::persistence::ScenarioPolicy::LiveRecalc,
        );
        assert_eq!(destination.formatting.conditional_formatting_rules.len(), 2);
        let first = &destination.formatting.conditional_formatting_rules[0];
        assert_eq!(first.rule_kind, "cell_value");
        assert_eq!(first.operator.as_deref(), Some("greaterThan"));
        assert_eq!(first.thresholds, vec!["0".to_string()]);
        assert_eq!(first.font_color.as_deref(), Some("#205F2A"));
        assert_eq!(first.fill_color.as_deref(), Some("#E6F2D9"));
        let second = &destination.formatting.conditional_formatting_rules[1];
        assert_eq!(second.operator.as_deref(), Some("lessThan"));
    }

    #[test]
    fn entry_mode_classifier_drives_projection() {
        let cases = [
            ("=SUM(1)", EntryMode::Formula),
            ("'hello", EntryMode::Text),
            ("42", EntryMode::Value),
            ("", EntryMode::Empty),
        ];
        for (text, expected_mode) in cases {
            let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("untitled-1"), text);
            let scenario =
                formula_space_to_scenario(&formula_space, "now".to_string(), "now".to_string());
            assert_eq!(
                scenario.entry.mode, expected_mode,
                "text {text:?} should classify to {expected_mode:?}",
            );
            assert_eq!(scenario.entry.text, text);
        }
    }
}
