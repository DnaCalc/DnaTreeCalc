//! Adapter from native OneCalc state to the Leptos-free shared session core.

impl dnacalc_bench_core::OneCalcSessionHost for crate::state::OneCalcHostState {
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
    ) -> dnacalc_bench_core::DispatchOutcome {
        if matches!(intent, dnacalc_skin_ir::SkinIntent::TreeWorkspace(_)) {
            return dnacalc_bench_core::DispatchOutcome::Rejected(
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
                }
                | dnacalc_skin_ir::OneFormulaIntent::SetFontAttributes {
                    formula_space_id, ..
                }
                | dnacalc_skin_ir::OneFormulaIntent::SetFillAttributes {
                    formula_space_id, ..
                }
                | dnacalc_skin_ir::OneFormulaIntent::SetLocale {
                    formula_space_id, ..
                }
                | dnacalc_skin_ir::OneFormulaIntent::SetDate1904 {
                    formula_space_id, ..
                }
                | dnacalc_skin_ir::OneFormulaIntent::SetConditionalFormatRule {
                    formula_space_id,
                    ..
                }
                | dnacalc_skin_ir::OneFormulaIntent::RemoveConditionalFormatRule {
                    formula_space_id,
                    ..
                } => formula_space_id,
            };
            let id = crate::domain::ids::FormulaSpaceId::new(formula_space_id.clone());
            if !self.formula_spaces.spaces.contains_key(&id) {
                return dnacalc_bench_core::DispatchOutcome::Rejected(
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
                return dnacalc_bench_core::DispatchOutcome::Rejected(
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
                    return dnacalc_bench_core::DispatchOutcome::Applied;
                }
                dnacalc_skin_ir::SkinShellIntent::SaveAs { suggested_path } => {
                    #[cfg(target_arch = "wasm32")]
                    return dnacalc_bench_core::DispatchOutcome::Rejected(
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
                            return dnacalc_bench_core::DispatchOutcome::Rejected(
                                dnacalc_skin_ir::SkinIntentDiagnostic {
                                    code: "save_as_path_required".into(),
                                    message: "native SaveAs requires a path".into(),
                                    recoverable: true,
                                },
                            );
                        };
                        if let Err(message) = crate::persistence::save_workspace_to_path(self, path)
                        {
                            return dnacalc_bench_core::DispatchOutcome::Rejected(
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
                    return dnacalc_bench_core::DispatchOutcome::Applied;
                }
                dnacalc_skin_ir::SkinShellIntent::Open { requested_path } => {
                    #[cfg(target_arch = "wasm32")]
                    return dnacalc_bench_core::DispatchOutcome::Rejected(
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
                            return dnacalc_bench_core::DispatchOutcome::Rejected(
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
                            return dnacalc_bench_core::DispatchOutcome::Rejected(
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
                    return dnacalc_bench_core::DispatchOutcome::Applied;
                }
                _ => {}
            }
        }
        if crate::app::reducer::apply_skin_intent_to_host_state(self, intent) {
            dnacalc_bench_core::DispatchOutcome::Applied
        } else {
            dnacalc_bench_core::DispatchOutcome::NoChange
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
        let mut session = dnacalc_bench_core::OneCalcSession::new(state);
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
        let mut save = dnacalc_bench_core::OneCalcSession::new(
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

        let mut open = dnacalc_bench_core::OneCalcSession::new(
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

    /// Plain `Save` (no caller-supplied path — bead dtc-lfz.3) genuinely
    /// writes the default per-user `workspace.json`, carrying the real
    /// authored formula text, not a fabricated `Applied` receipt. This is
    /// the exact `OneCalcSessionHost::dispatch` seam
    /// `dnacalc-bench-app::adapter::BenchHost::dispatch_shell_intent` calls
    /// through for the Bench product's command-deck Save entry, but proven
    /// here at the host layer (which — unlike `dnacalc-bench-app` — does
    /// not `forbid(unsafe_code)`, so it can use the same
    /// `DNAONECALC_WORKSPACE_DIR` scratch-dir override
    /// `persistence::workspace_storage`'s own tests use).
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn dispatch_save_writes_the_default_workspace_file() {
        // The override env var is process-global; serialize against EVERY
        // other test in the crate that touches it via the one shared lock
        // (see its doc comment — two independent per-module locks do not
        // actually serialize against each other).
        let _guard = crate::persistence::WORKSPACE_DIR_ENV_LOCK.lock().unwrap();

        let scratch = std::env::temp_dir().join(format!(
            "dnacalc-bench-host-skin-session-save-test-{}",
            std::process::id()
        ));
        if scratch.exists() {
            let _ = std::fs::remove_dir_all(&scratch);
        }
        std::env::set_var("DNAONECALC_WORKSPACE_DIR", &scratch);

        let mut session = dnacalc_bench_core::OneCalcSession::new(
            crate::app::preview_state::preview_minimal_host_state(),
        );
        let edit_receipt = session.handle(dnacalc_skin_ir::SkinIntentEnvelope::new(
            "edit-1",
            None,
            dnacalc_skin_ir::SkinIntent::OneFormula(dnacalc_skin_ir::OneFormulaIntent::EditText {
                formula_space_id: "untitled-1".to_string(),
                text: "=42".to_string(),
                caret_offset: 3,
            }),
        ));
        assert!(matches!(
            edit_receipt,
            dnacalc_skin_ir::SkinIntentReceipt::Applied { .. }
        ));

        let save_receipt = session.handle(dnacalc_skin_ir::SkinIntentEnvelope::new(
            "save-1",
            None,
            dnacalc_skin_ir::SkinIntent::Shell(dnacalc_skin_ir::SkinShellIntent::Save),
        ));
        assert!(
            matches!(
                save_receipt,
                dnacalc_skin_ir::SkinIntentReceipt::Applied { .. }
            ),
            "Save must be Applied, got {save_receipt:?}"
        );

        let path = crate::persistence::workspace_storage::workspace_storage_path()
            .expect("scratch dir override resolves a path");
        assert!(
            path.exists(),
            "Save must write the real workspace.json at {path:?}, not a fake receipt"
        );
        let written = std::fs::read_to_string(&path).expect("read back the saved file");
        assert!(
            written.contains("=42"),
            "the saved file carries the authored formula text: {written}"
        );

        let _ = std::fs::remove_dir_all(&scratch);
        std::env::remove_var("DNAONECALC_WORKSPACE_DIR");
    }

    #[test]
    fn all_six_profiles_execute_the_same_serialized_session_contract() {
        let targets = [
            dnacalc_bench_core::RuntimeTarget::BrowserWasm,
            dnacalc_bench_core::RuntimeTarget::HostedWeb,
            dnacalc_bench_core::RuntimeTarget::WindowsDesktop,
            dnacalc_bench_core::RuntimeTarget::WindowsHeadless,
            dnacalc_bench_core::RuntimeTarget::NativeUnix,
            dnacalc_bench_core::RuntimeTarget::NullTest,
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
            let mut session = dnacalc_bench_core::ProfiledSession::new(target, state);
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
        let mut session = dnacalc_bench_core::OneCalcSession::new(state);
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
