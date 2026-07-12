use oxfml_core::consumer::editor::{EditorEditService, EditorEnvironment};
use oxfml_core::{
    validate_conditional_formatting_formula, BindContext, CarrierRestrictionCode,
    CarrierValidationDisposition, ConditionalFormattingCarrierSpec, FormulaChannelKind,
    FormulaSourceRecord, StructureContextVersion,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IsolatedConditionalFormattingCarrier {
    pub formula_stable_id: String,
    pub formula_text: String,
    pub target_ranges: Vec<String>,
    pub rule_kind: String,
    pub operator: Option<String>,
    pub threshold_fields: Vec<String>,
    pub admitted_consequence_kinds: Vec<String>,
    pub blocked_scope_kinds: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionalFormattingCarrierSummary {
    pub disposition: String,
    pub restriction_profile_id: String,
    pub restriction_codes: Vec<String>,
    pub host_field_facts: Vec<String>,
    pub admitted_consequence_kinds: Vec<String>,
    pub blocked_scope_kinds: Vec<String>,
}

impl IsolatedConditionalFormattingCarrier {
    pub fn admitted_expression_rule(formula_text: impl Into<String>) -> Self {
        Self {
            formula_stable_id: "onecalc.cf.rule".to_string(),
            formula_text: formula_text.into(),
            target_ranges: vec!["A1:A10".to_string()],
            rule_kind: "Expression".to_string(),
            operator: None,
            threshold_fields: Vec::new(),
            admitted_consequence_kinds: vec![
                "fill_color".to_string(),
                "font_color".to_string(),
                "bold".to_string(),
                "italic".to_string(),
                "underline".to_string(),
                "simple_border".to_string(),
                "number_format_override".to_string(),
                "local_icon_set".to_string(),
            ],
            blocked_scope_kinds: vec![
                "data_bars".to_string(),
                "two_color_scale".to_string(),
                "three_color_scale".to_string(),
                "rich_icon_sets".to_string(),
                "multi_range_priority_graph".to_string(),
                "stop_if_true_graph".to_string(),
                "workbook_global_scope".to_string(),
            ],
        }
    }

    pub fn policy_text() -> String {
        let carrier = Self::admitted_expression_rule("=A1>0");
        format!(
            "Conditional Formatting: admitted={} blocked={}",
            carrier.admitted_consequence_kinds.join("|"),
            carrier.blocked_scope_kinds.join("|")
        )
    }
}

pub fn validate_isolated_conditional_formatting_carrier(
    carrier: &IsolatedConditionalFormattingCarrier,
) -> Result<ConditionalFormattingCarrierSummary, String> {
    let source = FormulaSourceRecord::new(
        carrier.formula_stable_id.clone(),
        1,
        carrier.formula_text.clone(),
    )
    .with_formula_channel_kind(FormulaChannelKind::ConditionalFormatting);
    let document = build_cf_validation_service().open_document(source, None);
    let bound_formula = document.bound_formula.as_ref().ok_or_else(|| {
        "conditional-formatting carrier validation requires a bound formula".to_string()
    })?;
    // Until OxFml exposes a dedicated consumer CF validator, OneCalc treats this as an
    // explicit editor-adjacent semantic operation rather than general host-side seam drilling.
    let validation = validate_conditional_formatting_formula(
        bound_formula,
        &ConditionalFormattingCarrierSpec {
            target_ranges: carrier.target_ranges.clone(),
            rule_kind: carrier.rule_kind.clone(),
            operator: carrier.operator.clone(),
            threshold_fields: carrier.threshold_fields.clone(),
        },
    );

    Ok(ConditionalFormattingCarrierSummary {
        disposition: match validation.disposition {
            CarrierValidationDisposition::Admitted => "admitted".to_string(),
            CarrierValidationDisposition::Rejected => "rejected".to_string(),
        },
        restriction_profile_id: validation.restriction_profile_id,
        restriction_codes: validation
            .restriction_codes
            .into_iter()
            .map(restriction_code_id)
            .collect(),
        host_field_facts: validation.host_field_facts,
        admitted_consequence_kinds: carrier.admitted_consequence_kinds.clone(),
        blocked_scope_kinds: carrier.blocked_scope_kinds.clone(),
    })
}

fn build_cf_validation_service() -> EditorEditService<'static> {
    let bind_context = BindContext {
        caller_row: 1,
        caller_col: 1,
        structure_context_version: StructureContextVersion("onecalc:cf:isolation:v1".to_string()),
        ..BindContext::default()
    };
    EditorEditService::new(EditorEnvironment::new(bind_context))
}

fn restriction_code_id(code: CarrierRestrictionCode) -> String {
    match code {
        CarrierRestrictionCode::UnionReferenceOperatorNotAdmitted => {
            "union_reference_operator_not_admitted".to_string()
        }
        CarrierRestrictionCode::IntersectionReferenceOperatorNotAdmitted => {
            "intersection_reference_operator_not_admitted".to_string()
        }
        CarrierRestrictionCode::SpillReferenceOperatorNotAdmitted => {
            "spill_reference_operator_not_admitted".to_string()
        }
        CarrierRestrictionCode::ExternalReferenceNotAdmitted => {
            "external_reference_not_admitted".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admitted_isolated_cf_subset_validates_an_expression_rule_through_oxfml() {
        let carrier = IsolatedConditionalFormattingCarrier::admitted_expression_rule("=A1>0");

        let summary = validate_isolated_conditional_formatting_carrier(&carrier)
            .expect("CF carrier should validate");

        assert_eq!(summary.disposition, "admitted");
        assert_eq!(
            summary.restriction_profile_id,
            "cf_restricted_not_equal_to_dv"
        );
        assert!(summary.restriction_codes.is_empty());
        assert!(summary
            .host_field_facts
            .contains(&"target_ranges=A1:A10".to_string()));
        assert!(summary
            .admitted_consequence_kinds
            .contains(&"fill_color".to_string()));
        assert!(summary
            .blocked_scope_kinds
            .contains(&"data_bars".to_string()));
    }

    #[test]
    fn admitted_isolated_cf_subset_keeps_restricted_reference_families_blocked() {
        let carrier = IsolatedConditionalFormattingCarrier::admitted_expression_rule("=A1,B1");

        let summary = validate_isolated_conditional_formatting_carrier(&carrier)
            .expect("CF carrier should validate");

        assert_eq!(summary.disposition, "rejected");
        assert_eq!(
            summary.restriction_codes,
            vec!["union_reference_operator_not_admitted".to_string()]
        );
    }
}
