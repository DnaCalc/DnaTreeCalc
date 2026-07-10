use serde::{Deserialize, Serialize};

use crate::formula::{OneFormulaIntent, OneFormulaProjection};
use crate::intent::WorkspaceIntent;
use crate::workspace::WorkspaceState;

pub const SKIN_IR_SCHEMA_VERSION: u32 = 1;

/// Top-level DNA Calc skin snapshot. It is the UX contract a frontend skin
/// consumes, with host-specific document shape carried by
/// [`SkinDocumentProjection`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkinSnapshot {
    pub schema_id: String,
    pub schema_version: u32,
    pub shell: SkinShellProjection,
    pub host_capabilities: HostCapabilityProjection,
    pub document: SkinDocumentProjection,
}

impl SkinSnapshot {
    #[must_use]
    pub fn one_formula(
        shell: SkinShellProjection,
        host_capabilities: HostCapabilityProjection,
        formula: OneFormulaProjection,
    ) -> Self {
        Self {
            schema_id: crate::SKIN_IR_PROTOCOL_SCHEMA.to_string(),
            schema_version: SKIN_IR_SCHEMA_VERSION,
            shell,
            host_capabilities,
            document: SkinDocumentProjection::OneFormula(formula),
        }
    }

    #[must_use]
    pub fn tree_workspace(
        shell: SkinShellProjection,
        host_capabilities: HostCapabilityProjection,
        workspace: WorkspaceState,
    ) -> Self {
        Self {
            schema_id: crate::SKIN_IR_PROTOCOL_SCHEMA.to_string(),
            schema_version: SKIN_IR_SCHEMA_VERSION,
            shell,
            host_capabilities,
            document: SkinDocumentProjection::TreeWorkspace(workspace),
        }
    }

    pub fn validate(&self) -> Result<(), SkinProtocolError> {
        validate_schema(&self.schema_id, self.schema_version)?;
        self.host_capabilities.validate()?;
        if self.shell.host_kind != self.host_capabilities.host_kind {
            return Err(SkinProtocolError::HostKindMismatch);
        }
        match (
            &self.document,
            self.host_capabilities.host_kind,
            self.host_capabilities.references,
        ) {
            (
                SkinDocumentProjection::OneFormula(_),
                HostKindProjection::OneCalc,
                ReferenceCapabilityProjection::Absent,
            )
            | (
                SkinDocumentProjection::TreeWorkspace(_),
                HostKindProjection::TreeCalc,
                ReferenceCapabilityProjection::TreeWorkspace,
            ) => Ok(()),
            _ => Err(SkinProtocolError::DocumentCapabilityMismatch),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SkinProtocolError {
    UnsupportedSchema { expected: String, actual: String },
    UnsupportedVersion { expected: u32, actual: u32 },
    HostKindMismatch,
    DocumentCapabilityMismatch,
    InvalidIntent { message: String },
    InvalidCapabilityPlacement,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "document_kind", content = "document", rename_all = "snake_case")]
pub enum SkinDocumentProjection {
    OneFormula(OneFormulaProjection),
    TreeWorkspace(WorkspaceState),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "intent_kind", content = "intent", rename_all = "snake_case")]
pub enum SkinIntent {
    Shell(SkinShellIntent),
    OneFormula(OneFormulaIntent),
    TreeWorkspace(WorkspaceIntent),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SkinShellIntent {
    OpenCommandPalette,
    CloseCommandPalette,
    SetActiveDocument { document_id: String },
    Save,
    SaveAs { suggested_path: Option<String> },
    Open { requested_path: Option<String> },
    OpenRecent { document_id: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkinIntentEnvelope {
    pub schema_id: String,
    pub schema_version: u32,
    pub intent_id: String,
    pub document_id: Option<String>,
    pub intent: SkinIntent,
}

impl SkinIntentEnvelope {
    #[must_use]
    pub fn new(
        intent_id: impl Into<String>,
        document_id: Option<String>,
        intent: SkinIntent,
    ) -> Self {
        Self {
            schema_id: crate::SKIN_IR_PROTOCOL_SCHEMA.into(),
            schema_version: SKIN_IR_SCHEMA_VERSION,
            intent_id: intent_id.into(),
            document_id,
            intent,
        }
    }

    pub fn validate(&self) -> Result<(), SkinProtocolError> {
        validate_schema(&self.schema_id, self.schema_version)?;
        if let SkinIntent::OneFormula(intent) = &self.intent {
            intent
                .validate()
                .map_err(|error| SkinProtocolError::InvalidIntent {
                    message: format!("{error:?}"),
                })?;
        }
        Ok(())
    }
}

fn validate_schema(schema_id: &str, schema_version: u32) -> Result<(), SkinProtocolError> {
    if schema_id != crate::SKIN_IR_PROTOCOL_SCHEMA {
        return Err(SkinProtocolError::UnsupportedSchema {
            expected: crate::SKIN_IR_PROTOCOL_SCHEMA.into(),
            actual: schema_id.into(),
        });
    }
    if schema_version != SKIN_IR_SCHEMA_VERSION {
        return Err(SkinProtocolError::UnsupportedVersion {
            expected: SKIN_IR_SCHEMA_VERSION,
            actual: schema_version,
        });
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SkinIntentReceipt {
    Applied {
        intent_id: String,
        snapshot_revision: u64,
    },
    Rejected {
        intent_id: String,
        diagnostic: SkinIntentDiagnostic,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkinIntentDiagnostic {
    pub code: String,
    pub message: String,
    pub recoverable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkinShellProjection {
    pub host_kind: HostKindProjection,
    pub title: String,
    pub active_document_id: Option<String>,
    pub status_text: Option<String>,
    pub command_palette_open: bool,
    pub persistence: PersistenceProjection,
}

impl Default for SkinShellProjection {
    fn default() -> Self {
        Self {
            host_kind: HostKindProjection::OneCalc,
            title: String::new(),
            active_document_id: None,
            status_text: None,
            command_palette_open: false,
            persistence: PersistenceProjection::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostKindProjection {
    OneCalc,
    TreeCalc,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistenceProjection {
    pub can_save: bool,
    pub can_open: bool,
    pub dirty: bool,
    pub current_path: Option<String>,
    pub recent_documents: Vec<RecentDocumentProjection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentDocumentProjection {
    pub document_id: String,
    pub display_name: String,
    pub path: Option<String>,
    pub last_opened_unix_ms: Option<u64>,
    pub available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostCapabilityProjection {
    pub host_kind: HostKindProjection,
    pub formula_editor: bool,
    pub formula_assist: bool,
    pub formula_drill: bool,
    pub formatting: bool,
    pub conditional_formatting: bool,
    pub replay_or_comparison: bool,
    pub references: ReferenceCapabilityProjection,
    pub runtime_profile: RuntimeProfileProjection,
    pub extension_placement: ExtensionPlacementProjection,
    pub unavailable_families: Vec<String>,
}

impl HostCapabilityProjection {
    #[must_use]
    pub fn onecalc_null_references(
        runtime_profile: RuntimeProfileProjection,
        extension_placement: ExtensionPlacementProjection,
    ) -> Self {
        Self {
            host_kind: HostKindProjection::OneCalc,
            formula_editor: true,
            formula_assist: true,
            formula_drill: true,
            formatting: true,
            conditional_formatting: true,
            replay_or_comparison: true,
            references: ReferenceCapabilityProjection::Absent,
            runtime_profile,
            extension_placement,
            unavailable_families: Vec::new(),
        }
    }

    #[must_use]
    pub fn treecalc_references(
        runtime_profile: RuntimeProfileProjection,
        extension_placement: ExtensionPlacementProjection,
    ) -> Self {
        Self {
            host_kind: HostKindProjection::TreeCalc,
            formula_editor: true,
            formula_assist: true,
            formula_drill: true,
            formatting: true,
            conditional_formatting: true,
            replay_or_comparison: false,
            references: ReferenceCapabilityProjection::TreeWorkspace,
            runtime_profile,
            extension_placement,
            unavailable_families: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeProfileProjection {
    BrowserWasm,
    HostedWeb,
    WindowsDesktop,
    WindowsHeadless,
    NativeUnix,
    NullTest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionPlacementProjection {
    Unavailable,
    InProcess,
    NativeCompanion,
}

impl HostCapabilityProjection {
    pub fn validate(&self) -> Result<(), SkinProtocolError> {
        use ExtensionPlacementProjection::{InProcess, NativeCompanion, Unavailable};
        use RuntimeProfileProjection::{
            BrowserWasm, HostedWeb, NativeUnix, NullTest, WindowsDesktop, WindowsHeadless,
        };
        match (self.runtime_profile, self.extension_placement) {
            (BrowserWasm | NullTest, Unavailable)
            | (HostedWeb, Unavailable | NativeCompanion)
            | (WindowsDesktop, Unavailable | InProcess | NativeCompanion)
            | (WindowsHeadless | NativeUnix, Unavailable | InProcess) => Ok(()),
            _ => Err(SkinProtocolError::InvalidCapabilityPlacement),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceCapabilityProjection {
    Absent,
    TreeWorkspace,
    ExternalProvider,
    Unsupported,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formula::{FormulaEntryModeProjection, OneFormulaProjection};

    #[test]
    fn top_level_one_formula_snapshot_round_trips() {
        let snapshot = SkinSnapshot::one_formula(
            SkinShellProjection {
                host_kind: HostKindProjection::OneCalc,
                title: "OneCalc".to_string(),
                active_document_id: Some("formula-1".to_string()),
                status_text: Some("ready".to_string()),
                command_palette_open: false,
                persistence: PersistenceProjection {
                    can_save: true,
                    can_open: true,
                    dirty: false,
                    current_path: None,
                    recent_documents: vec![RecentDocumentProjection {
                        document_id: "formula-1".to_string(),
                        display_name: "Formula 1".to_string(),
                        path: None,
                        last_opened_unix_ms: Some(1),
                        available: true,
                    }],
                },
            },
            HostCapabilityProjection::onecalc_null_references(
                RuntimeProfileProjection::NullTest,
                ExtensionPlacementProjection::Unavailable,
            ),
            OneFormulaProjection {
                formula_space_id: "formula-1".to_string(),
                formula_stable_id: "stable-1".to_string(),
                display_name: "Formula 1".to_string(),
                raw_entered_cell_text: "=SUM(1,2,3)".to_string(),
                entry_mode: FormulaEntryModeProjection::Formula,
                ..OneFormulaProjection::default()
            },
        );

        let json = serde_json::to_string(&snapshot).expect("serialize snapshot");
        assert_eq!(json, include_str!("../fixtures/one_formula.json").trim());
        let restored: SkinSnapshot = serde_json::from_str(&json).expect("deserialize snapshot");
        assert_eq!(restored, snapshot);
    }

    #[test]
    fn top_level_tree_workspace_snapshot_round_trips() {
        let snapshot = SkinSnapshot::tree_workspace(
            SkinShellProjection {
                host_kind: HostKindProjection::TreeCalc,
                title: "TreeCalc".to_string(),
                active_document_id: Some("workspace-1".to_string()),
                status_text: None,
                command_palette_open: false,
                persistence: PersistenceProjection::default(),
            },
            HostCapabilityProjection::treecalc_references(
                RuntimeProfileProjection::NullTest,
                ExtensionPlacementProjection::Unavailable,
            ),
            WorkspaceState {
                workspace_id: "workspace-1".to_string(),
                ..WorkspaceState::default()
            },
        );

        let json = serde_json::to_string(&snapshot).expect("serialize snapshot");
        assert_eq!(json, include_str!("../fixtures/tree_workspace.json").trim());
        let restored: SkinSnapshot = serde_json::from_str(&json).expect("deserialize snapshot");
        assert_eq!(restored, snapshot);
    }

    #[test]
    fn validation_rejects_schema_and_version_skew() {
        let mut snapshot = SkinSnapshot::tree_workspace(
            SkinShellProjection {
                host_kind: HostKindProjection::TreeCalc,
                ..Default::default()
            },
            HostCapabilityProjection::treecalc_references(
                RuntimeProfileProjection::NullTest,
                ExtensionPlacementProjection::Unavailable,
            ),
            WorkspaceState::default(),
        );
        assert_eq!(snapshot.validate(), Ok(()));
        snapshot.schema_version += 1;
        assert!(matches!(
            snapshot.validate(),
            Err(SkinProtocolError::UnsupportedVersion { .. })
        ));
        snapshot.schema_version = SKIN_IR_SCHEMA_VERSION;
        snapshot.schema_id = "other.protocol".to_string();
        assert!(matches!(
            snapshot.validate(),
            Err(SkinProtocolError::UnsupportedSchema { .. })
        ));
    }

    #[test]
    fn unknown_intent_variant_is_rejected() {
        let json = r#"{"intent_kind":"shell","intent":{"kind":"future_command"}}"#;
        assert!(serde_json::from_str::<SkinIntent>(json).is_err());
    }

    #[test]
    fn shell_intent_golden_shape_is_stable() {
        let intent = SkinIntent::Shell(SkinShellIntent::OpenRecent {
            document_id: "recent-7".to_string(),
        });
        assert_eq!(
            serde_json::to_string(&intent).expect("serialize intent"),
            r#"{"intent_kind":"shell","intent":{"kind":"open_recent","document_id":"recent-7"}}"#
        );
    }

    #[test]
    fn intent_envelope_stamps_and_validates_schema() {
        let mut envelope =
            SkinIntentEnvelope::new("intent-1", None, SkinIntent::Shell(SkinShellIntent::Save));
        assert_eq!(envelope.validate(), Ok(()));
        envelope.schema_version += 1;
        assert!(matches!(
            envelope.validate(),
            Err(SkinProtocolError::UnsupportedVersion { .. })
        ));
        envelope.schema_version = SKIN_IR_SCHEMA_VERSION;
        envelope.schema_id = "future".into();
        assert!(matches!(
            envelope.validate(),
            Err(SkinProtocolError::UnsupportedSchema { .. })
        ));
    }

    #[test]
    fn snapshot_rejects_cross_host_capability_mismatch() {
        let mut snapshot = SkinSnapshot::tree_workspace(
            SkinShellProjection {
                host_kind: HostKindProjection::TreeCalc,
                ..Default::default()
            },
            HostCapabilityProjection::treecalc_references(
                RuntimeProfileProjection::NullTest,
                ExtensionPlacementProjection::Unavailable,
            ),
            WorkspaceState::default(),
        );
        assert_eq!(snapshot.validate(), Ok(()));
        snapshot.host_capabilities.host_kind = HostKindProjection::OneCalc;
        assert_eq!(
            snapshot.validate(),
            Err(SkinProtocolError::HostKindMismatch)
        );
    }

    #[test]
    fn persistence_and_capability_goldens_are_stable() {
        let persistence = PersistenceProjection {
            can_save: true,
            can_open: true,
            dirty: true,
            current_path: Some("model.dnatree".into()),
            recent_documents: vec![RecentDocumentProjection {
                document_id: "d1".into(),
                display_name: "Model".into(),
                path: Some("model.dnatree".into()),
                last_opened_unix_ms: Some(42),
                available: true,
            }],
        };
        assert_eq!(
            serde_json::to_value(persistence).unwrap(),
            serde_json::json!({
                "can_save":true,"can_open":true,"dirty":true,"current_path":"model.dnatree",
                "recent_documents":[{"document_id":"d1","display_name":"Model","path":"model.dnatree","last_opened_unix_ms":42,"available":true}]
            })
        );
        let capabilities = HostCapabilityProjection::onecalc_null_references(
            RuntimeProfileProjection::WindowsDesktop,
            ExtensionPlacementProjection::InProcess,
        );
        assert_eq!(
            serde_json::to_value(capabilities).unwrap(),
            serde_json::json!({
                "host_kind":"one_calc","formula_editor":true,"formula_assist":true,"formula_drill":true,
                "formatting":true,"conditional_formatting":true,"replay_or_comparison":true,
                "references":"absent","runtime_profile":"windows_desktop","extension_placement":"in_process","unavailable_families":[]
            })
        );
    }

    #[test]
    fn browser_capabilities_reject_native_extension_placement() {
        let capabilities = HostCapabilityProjection::onecalc_null_references(
            RuntimeProfileProjection::BrowserWasm,
            ExtensionPlacementProjection::InProcess,
        );
        assert_eq!(
            capabilities.validate(),
            Err(SkinProtocolError::InvalidCapabilityPlacement)
        );
    }
}
