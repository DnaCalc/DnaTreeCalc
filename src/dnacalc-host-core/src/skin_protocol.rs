use dnacalc_skin_ir::{
    ExtensionPlacementProjection, HostCapabilityProjection, HostKindProjection,
    RuntimeProfileProjection, SkinIntent, SkinIntentDiagnostic, SkinIntentEnvelope,
    SkinIntentReceipt, SkinShellProjection, SkinSnapshot,
};

use crate::DocumentSession;

/// Shared Skin IR boundary over an in-process DNA Calc document session.
pub struct SkinProtocolSession {
    document_id: String,
    runtime_profile: RuntimeProfileProjection,
    extension_placement: ExtensionPlacementProjection,
    document: DocumentSession,
}

impl SkinProtocolSession {
    #[must_use]
    pub fn new(
        document_id: impl Into<String>,
        runtime_profile: RuntimeProfileProjection,
        extension_placement: ExtensionPlacementProjection,
        document: DocumentSession,
    ) -> Result<Self, dnacalc_skin_ir::SkinProtocolError> {
        HostCapabilityProjection::treecalc_references(runtime_profile, extension_placement)
            .validate()?;
        Ok(Self {
            document_id: document_id.into(),
            runtime_profile,
            extension_placement,
            document,
        })
    }

    #[must_use]
    pub fn snapshot(&self) -> SkinSnapshot {
        SkinSnapshot::tree_workspace(
            SkinShellProjection {
                host_kind: HostKindProjection::TreeCalc,
                title: "DNA TreeCalc".into(),
                active_document_id: Some(self.document_id.clone()),
                status_text: None,
                command_palette_open: false,
                persistence: Default::default(),
            },
            HostCapabilityProjection::treecalc_references(
                self.runtime_profile,
                self.extension_placement,
            ),
            self.document.snapshot(),
        )
    }

    pub fn dispatch(&mut self, envelope: SkinIntentEnvelope) -> SkinIntentReceipt {
        if let Err(error) = envelope.validate() {
            return rejected(
                envelope.intent_id,
                "skin_protocol_invalid",
                format!("{error:?}"),
            );
        }
        if envelope
            .document_id
            .as_deref()
            .is_some_and(|id| id != self.document_id)
        {
            return rejected(
                envelope.intent_id,
                "document_mismatch",
                "intent targets another document".into(),
            );
        }
        match envelope.intent {
            SkinIntent::TreeWorkspace(intent) => {
                let receipt = self.document.dispatch(intent);
                if receipt.accepted {
                    SkinIntentReceipt::Applied {
                        intent_id: envelope.intent_id,
                        snapshot_revision: self.document.snapshot().projection_seq,
                    }
                } else {
                    rejected(
                        envelope.intent_id,
                        "workspace_intent_rejected",
                        receipt
                            .error
                            .map_or_else(|| "intent rejected".into(), |error| error.to_string()),
                    )
                }
            }
            SkinIntent::Shell(_) | SkinIntent::OneFormula(_) => rejected(
                envelope.intent_id,
                "unsupported_intent_family",
                "intent family is not handled by this document session".into(),
            ),
        }
    }
}

fn rejected(intent_id: String, code: &str, message: String) -> SkinIntentReceipt {
    SkinIntentReceipt::Rejected {
        intent_id,
        diagnostic: SkinIntentDiagnostic {
            code: code.into(),
            message,
            recoverable: true,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::WorkbookSession;
    use dnacalc_skin_ir::WorkspaceIntent;

    #[test]
    fn validates_envelope_routes_workspace_and_rejects_skew() {
        let mut workbook = WorkbookSession::create("w").unwrap();
        workbook.add_sheet("Sheet1").unwrap();
        let mut session = SkinProtocolSession::new(
            "d",
            RuntimeProfileProjection::NullTest,
            ExtensionPlacementProjection::Unavailable,
            DocumentSession::Workbook(workbook),
        )
        .unwrap();
        let receipt = session.dispatch(SkinIntentEnvelope::new(
            "i1",
            Some("d".into()),
            SkinIntent::TreeWorkspace(WorkspaceIntent::Recalculate),
        ));
        assert!(matches!(receipt, SkinIntentReceipt::Applied { .. }));
        let mut skewed = SkinIntentEnvelope::new(
            "i2",
            Some("d".into()),
            SkinIntent::TreeWorkspace(WorkspaceIntent::Recalculate),
        );
        skewed.schema_version += 1;
        assert!(
            matches!(session.dispatch(skewed), SkinIntentReceipt::Rejected { diagnostic, .. } if diagnostic.code == "skin_protocol_invalid")
        );
        assert_eq!(session.snapshot().validate(), Ok(()));
        let mismatch = SkinIntentEnvelope::new(
            "i3",
            Some("other".into()),
            SkinIntent::TreeWorkspace(WorkspaceIntent::Recalculate),
        );
        assert!(
            matches!(session.dispatch(mismatch), SkinIntentReceipt::Rejected { diagnostic, .. } if diagnostic.code == "document_mismatch")
        );
        let unsupported = SkinIntentEnvelope::new(
            "i4",
            Some("d".into()),
            SkinIntent::Shell(dnacalc_skin_ir::SkinShellIntent::Save),
        );
        assert!(
            matches!(session.dispatch(unsupported), SkinIntentReceipt::Rejected { diagnostic, .. } if diagnostic.code == "unsupported_intent_family")
        );
        let mut bad_schema = SkinIntentEnvelope::new(
            "i5",
            Some("d".into()),
            SkinIntent::TreeWorkspace(WorkspaceIntent::Recalculate),
        );
        bad_schema.schema_id = "future".into();
        assert!(
            matches!(session.dispatch(bad_schema), SkinIntentReceipt::Rejected { diagnostic, .. } if diagnostic.code == "skin_protocol_invalid")
        );
        assert!(
            SkinProtocolSession::new(
                "bad",
                RuntimeProfileProjection::BrowserWasm,
                ExtensionPlacementProjection::InProcess,
                DocumentSession::RichTree(crate::RichTreeSession::new())
            )
            .is_err()
        );
    }
}
