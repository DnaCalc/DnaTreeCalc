use crate::state::FormulaInputBindingState;

pub fn runtime_formal_input_bindings(
    bindings: &[FormulaInputBindingState],
) -> Vec<oxfml_core::consumer::runtime::RuntimeFormalInputBinding> {
    bindings
        .iter()
        .map(
            |binding| oxfml_core::consumer::runtime::RuntimeFormalInputBinding {
                reference_handle: binding.reference_handle.clone(),
                reference_descriptor: binding.reference_descriptor.clone(),
                binding: oxfml_core::DefinedNameBinding::Value(binding.value.clone()),
            },
        )
        .collect()
}

pub fn validate_formula_input_bindings(
    bindings: &[FormulaInputBindingState],
) -> Result<(), String> {
    for binding in bindings {
        if binding.label.trim().is_empty() {
            return Err("formula input label must not be empty".to_string());
        }
        if !binding.reference_descriptor.starts_with("name:") {
            return Err(format!(
                "formula input {} must use a name: descriptor",
                binding.label
            ));
        }
        match binding.value.core() {
            crate::adapters::oxfml::CoreValue::Number(_)
            | crate::adapters::oxfml::CoreValue::Text(_)
            | crate::adapters::oxfml::CoreValue::Logical(_)
            | crate::adapters::oxfml::CoreValue::Error(_) => {}
            _ => {
                return Err(format!(
                    "formula input {} uses a non-scalar value; v1 admits scalar inputs only",
                    binding.label
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::FormulaInputBindingState;

    #[test]
    fn scalar_formula_input_maps_to_runtime_formal_binding() {
        let bindings =
            runtime_formal_input_bindings(&[FormulaInputBindingState::scalar_number("Rate", 0.2)]);

        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].reference_descriptor, "name:Rate");
        assert!(matches!(
            bindings[0].binding,
            oxfml_core::DefinedNameBinding::Value(ref value)
                if value.as_number() == Some(0.2)
        ));
    }
}
