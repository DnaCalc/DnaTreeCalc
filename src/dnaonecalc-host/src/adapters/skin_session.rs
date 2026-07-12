//! Adapter from native OneCalc state to the Leptos-free shared session core.

impl dnaonecalc_core::OneCalcSessionHost for crate::state::OneCalcHostState {
    fn set_runtime_profile(&mut self, profile: dnacalc_skin_ir::RuntimeProfileProjection) {
        self.runtime_profile_override = Some(profile);
    }
    fn snapshot(&self) -> dnacalc_skin_ir::SkinSnapshot {
        crate::services::home_shell_view_model::build_home_shell_view_model(self)
            .expect("OneCalc session requires an active formula space")
            .skin_snapshot
    }

    #[allow(unreachable_code, unused_variables)]
    fn dispatch(
        &mut self,
        intent: dnacalc_skin_ir::SkinIntent,
    ) -> dnaonecalc_core::DispatchOutcome {
        if matches!(intent, dnacalc_skin_ir::SkinIntent::TreeWorkspace(_)) {
            return dnaonecalc_core::DispatchOutcome::Rejected(
                dnacalc_skin_ir::SkinIntentDiagnostic {
                    code: "tree_workspace_unsupported".into(),
                    message: "OneCalc has no tree workspace surface".into(),
                    recoverable: false,
                },
            );
        }
        if let dnacalc_skin_ir::SkinIntent::OneFormula(formula_intent) = &intent {
            let formula_space_id = match formula_intent {
                dnacalc_skin_ir::OneFormulaIntent::EditText {
                    formula_space_id, ..
                }
                | dnacalc_skin_ir::OneFormulaIntent::SetSelection {
                    formula_space_id, ..
                }
                | dnacalc_skin_ir::OneFormulaIntent::ApplyCompletion {
                    formula_space_id, ..
                }
                | dnacalc_skin_ir::OneFormulaIntent::SetNumberFormat {
                    formula_space_id, ..
                }
                | dnacalc_skin_ir::OneFormulaIntent::SetScenarioPolicy {
                    formula_space_id, ..
                }
                | dnacalc_skin_ir::OneFormulaIntent::ToggleFormulaDrill { formula_space_id }
                | dnacalc_skin_ir::OneFormulaIntent::Recalculate { formula_space_id }
                | dnacalc_skin_ir::OneFormulaIntent::RequestResultArrayWindow {
                    formula_space_id,
                    ..
                }
                | dnacalc_skin_ir::OneFormulaIntent::RequestDrillArrayWindow {
                    formula_space_id,
                    ..
                } => formula_space_id,
            };
            let id = crate::domain::ids::FormulaSpaceId::new(formula_space_id.clone());
            if !self.formula_spaces.spaces.contains_key(&id) {
                return dnaonecalc_core::DispatchOutcome::Rejected(
                    dnacalc_skin_ir::SkinIntentDiagnostic {
                        code: "unknown_formula_space".into(),
                        message: format!("unknown formula space `{formula_space_id}`"),
                        recoverable: true,
                    },
                );
            }
            if matches!(
                formula_intent,
                dnacalc_skin_ir::OneFormulaIntent::RequestResultArrayWindow { .. }
                    | dnacalc_skin_ir::OneFormulaIntent::RequestDrillArrayWindow { .. }
            ) {
                return dnaonecalc_core::DispatchOutcome::Rejected(
                    dnacalc_skin_ir::SkinIntentDiagnostic {
                        code: "array_window_adapter_unavailable".into(),
                        message:
                            "bounded array-window transport is not attached in this host adapter"
                                .into(),
                        recoverable: true,
                    },
                );
            }
        }
        if let dnacalc_skin_ir::SkinIntent::Shell(shell) = &intent {
            match shell {
                dnacalc_skin_ir::SkinShellIntent::Save => {
                    crate::persistence::save_workspace_to_local_storage(self);
                    self.workspace_shell.pending_persistence_intent = None;
                    return dnaonecalc_core::DispatchOutcome::Applied;
                }
                dnacalc_skin_ir::SkinShellIntent::SaveAs { suggested_path } => {
                    #[cfg(target_arch = "wasm32")]
                    return dnaonecalc_core::DispatchOutcome::Rejected(
                        dnacalc_skin_ir::SkinIntentDiagnostic {
                            code: "browser_file_picker_required".into(),
                            message:
                                "browser SaveAs requires the asynchronous file-download adapter"
                                    .into(),
                            recoverable: true,
                        },
                    );
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        let Some(path) = suggested_path.as_deref() else {
                            return dnaonecalc_core::DispatchOutcome::Rejected(
                                dnacalc_skin_ir::SkinIntentDiagnostic {
                                    code: "save_as_path_required".into(),
                                    message: "native SaveAs requires a path".into(),
                                    recoverable: true,
                                },
                            );
                        };
                        if let Err(message) = crate::persistence::save_workspace_to_path(self, path)
                        {
                            return dnaonecalc_core::DispatchOutcome::Rejected(
                                dnacalc_skin_ir::SkinIntentDiagnostic {
                                    code: "save_as_failed".into(),
                                    message,
                                    recoverable: true,
                                },
                            );
                        }
                    }
                    self.workspace_shell.current_workspace_path = suggested_path.clone();
                    self.workspace_shell.pending_persistence_intent = None;
                    return dnaonecalc_core::DispatchOutcome::Applied;
                }
                dnacalc_skin_ir::SkinShellIntent::Open { requested_path } => {
                    #[cfg(target_arch = "wasm32")]
                    return dnaonecalc_core::DispatchOutcome::Rejected(
                        dnacalc_skin_ir::SkinIntentDiagnostic {
                            code: "browser_file_picker_required".into(),
                            message: "browser Open requires the asynchronous file-input adapter"
                                .into(),
                            recoverable: true,
                        },
                    );
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        let Some(path) = requested_path.as_deref() else {
                            return dnaonecalc_core::DispatchOutcome::Rejected(
                                dnacalc_skin_ir::SkinIntentDiagnostic {
                                    code: "open_path_required".into(),
                                    message: "native Open requires a path".into(),
                                    recoverable: true,
                                },
                            );
                        };
                        if let Err(message) =
                            crate::persistence::open_workspace_from_path(self, path)
                        {
                            return dnaonecalc_core::DispatchOutcome::Rejected(
                                dnacalc_skin_ir::SkinIntentDiagnostic {
                                    code: "open_failed".into(),
                                    message,
                                    recoverable: true,
                                },
                            );
                        }
                    }
                    self.workspace_shell.current_workspace_path = requested_path.clone();
                    self.workspace_shell.pending_persistence_intent = None;
                    return dnaonecalc_core::DispatchOutcome::Applied;
                }
                _ => {}
            }
        }
        if crate::app::reducer::apply_skin_intent_to_host_state(self, intent) {
            dnaonecalc_core::DispatchOutcome::Applied
        } else {
            dnaonecalc_core::DispatchOutcome::NoChange
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn host_state_round_trips_snapshot_and_serialized_intent_receipt() {
        let state = crate::app::preview_state::preview_minimal_host_state();
        let formula_space_id = state
            .workspace_shell
            .active_formula_space_id
            .as_ref()
            .unwrap()
            .as_str()
            .to_string();
        let mut session = dnaonecalc_core::OneCalcSession::new(state);
        let envelope = dnacalc_skin_ir::SkinIntentEnvelope::new(
            "edit-1",
            None,
            dnacalc_skin_ir::SkinIntent::OneFormula(dnacalc_skin_ir::OneFormulaIntent::EditText {
                formula_space_id,
                text: "=SUM(1,2,3)".into(),
                caret_offset: 11,
            }),
        );
        let json = serde_json::to_string(&envelope).unwrap();
        let receipt_json = session.handle_json(&json).unwrap();
        let receipt: dnacalc_skin_ir::SkinIntentReceipt =
            serde_json::from_str(&receipt_json).unwrap();
        assert!(matches!(
            receipt,
            dnacalc_skin_ir::SkinIntentReceipt::Applied {
                snapshot_revision: 1,
                ..
            }
        ));
        match session.snapshot().document {
            dnacalc_skin_ir::SkinDocumentProjection::OneFormula(formula) => {
                assert_eq!(formula.raw_entered_cell_text, "=SUM(1,2,3)")
            }
            _ => panic!("expected OneFormula snapshot"),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn native_save_as_and_open_use_the_requested_path() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/ws16/native-path-workspace.json");
        let path_text = path.to_string_lossy().to_string();
        let mut save = dnaonecalc_core::OneCalcSession::new(
            crate::app::preview_state::preview_minimal_host_state(),
        );
        let receipt = save.handle(dnacalc_skin_ir::SkinIntentEnvelope::new(
            "save-as",
            None,
            dnacalc_skin_ir::SkinIntent::Shell(dnacalc_skin_ir::SkinShellIntent::SaveAs {
                suggested_path: Some(path_text.clone()),
            }),
        ));
        assert!(matches!(
            receipt,
            dnacalc_skin_ir::SkinIntentReceipt::Applied { .. }
        ));
        assert!(path.exists());

        let mut open = dnaonecalc_core::OneCalcSession::new(
            crate::app::preview_state::preview_minimal_host_state(),
        );
        let receipt = open.handle(dnacalc_skin_ir::SkinIntentEnvelope::new(
            "open",
            None,
            dnacalc_skin_ir::SkinIntent::Shell(dnacalc_skin_ir::SkinShellIntent::Open {
                requested_path: Some(path_text.clone()),
            }),
        ));
        assert!(matches!(
            receipt,
            dnacalc_skin_ir::SkinIntentReceipt::Applied { .. }
        ));
        assert_eq!(
            open.host()
                .workspace_shell
                .current_workspace_path
                .as_deref(),
            Some(path_text.as_str())
        );
    }

    #[test]
    fn all_six_profiles_execute_the_same_serialized_session_contract() {
        let targets = [
            dnaonecalc_core::RuntimeTarget::BrowserWasm,
            dnaonecalc_core::RuntimeTarget::HostedWeb,
            dnaonecalc_core::RuntimeTarget::WindowsDesktop,
            dnaonecalc_core::RuntimeTarget::WindowsHeadless,
            dnaonecalc_core::RuntimeTarget::NativeUnix,
            dnaonecalc_core::RuntimeTarget::NullTest,
        ];
        for target in targets {
            let state = crate::app::preview_state::preview_minimal_host_state();
            let id = state
                .workspace_shell
                .active_formula_space_id
                .as_ref()
                .unwrap()
                .as_str()
                .to_string();
            let mut session = dnaonecalc_core::ProfiledSession::new(target, state);
            let json = serde_json::to_string(&dnacalc_skin_ir::SkinIntentEnvelope::new(
                format!("edit-{target:?}"),
                None,
                dnacalc_skin_ir::SkinIntent::OneFormula(
                    dnacalc_skin_ir::OneFormulaIntent::EditText {
                        formula_space_id: id,
                        text: "=SUM(4,5)".into(),
                        caret_offset: 9,
                    },
                ),
            ))
            .unwrap();
            let receipt: dnacalc_skin_ir::SkinIntentReceipt =
                serde_json::from_str(&session.handle_json(&json).unwrap()).unwrap();
            assert!(matches!(
                receipt,
                dnacalc_skin_ir::SkinIntentReceipt::Applied {
                    snapshot_revision: 1,
                    ..
                }
            ));
            assert_eq!(
                session.snapshot().host_capabilities.runtime_profile,
                target.profile()
            );
        }
    }

    #[test]
    fn invalid_formula_id_is_a_typed_rejection() {
        let state = crate::app::preview_state::preview_minimal_host_state();
        let mut session = dnaonecalc_core::OneCalcSession::new(state);
        let receipt = session.handle(dnacalc_skin_ir::SkinIntentEnvelope::new(
            "bad-id",
            None,
            dnacalc_skin_ir::SkinIntent::OneFormula(
                dnacalc_skin_ir::OneFormulaIntent::Recalculate {
                    formula_space_id: "missing".into(),
                },
            ),
        ));
        match receipt {
            dnacalc_skin_ir::SkinIntentReceipt::Rejected { diagnostic, .. } => {
                assert_eq!(diagnostic.code, "unknown_formula_space")
            }
            _ => panic!("expected typed rejection"),
        }
    }
}
