use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtensionProviderManifest {
    pub provider_id: String,
    pub display_name: String,
    pub abi_version: String,
    pub host_profile_ids: Vec<String>,
    pub platform_gate_ids: Vec<String>,
    pub declared_capabilities: Vec<String>,
    pub entrypoint: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtensionProviderEntrypoint {
    #[serde(default)]
    pub registered_functions: Vec<RegisteredExtensionFunction>,
    #[serde(default)]
    pub rtd_topics: Vec<RegisteredRtdTopic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegisteredExtensionFunction {
    pub function_name: String,
    pub behavior: RegisteredExtensionBehavior,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RegisteredExtensionBehavior {
    SumNumbers,
    AlwaysError { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegisteredRtdTopic {
    pub topic_id: String,
    pub initial_value: String,
    #[serde(default)]
    pub updates: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionAbiContract {
    pub abi_id: String,
    pub abi_version: String,
    pub host_profile_id: String,
    pub platform_gate_id: String,
    pub admitted_capabilities: Vec<String>,
    pub excluded_capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionValidationResult {
    pub provider_id: String,
    pub admitted: bool,
    pub admitted_capabilities: Vec<String>,
    pub blocked_capabilities: Vec<String>,
    pub blocked_reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedExtensionProvider {
    pub manifest_path: String,
    pub manifest: ExtensionProviderManifest,
    pub validation: ExtensionValidationResult,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionManifestLoadFailure {
    pub manifest_path: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionRootLoadSummary {
    pub extension_root: String,
    pub discovered_manifest_count: usize,
    pub admitted_providers: Vec<LoadedExtensionProvider>,
    pub rejected_providers: Vec<LoadedExtensionProvider>,
    pub malformed_manifests: Vec<ExtensionManifestLoadFailure>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExtensionInvocationArgument {
    Number(f64),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionInvocationSummary {
    pub provider_id: String,
    pub function_name: String,
    pub provider_state: String,
    pub invocation_state: String,
    pub value_summary: Option<String>,
    pub failure_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionCapabilityTruth {
    pub capability_id: String,
    pub declaration_state: String,
    pub runtime_state: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionProviderRuntimeTruth {
    pub provider_id: String,
    pub display_name: String,
    pub manifest_path: String,
    pub provider_state: String,
    pub capability_truths: Vec<ExtensionCapabilityTruth>,
    pub blocked_reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionRootRuntimeTruthSummary {
    pub extension_root: String,
    pub runtime_platform: String,
    pub provider_truths: Vec<ExtensionProviderRuntimeTruth>,
    pub manifest_failures: Vec<ExtensionManifestLoadFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivatedRtdTopicSession {
    pub provider_id: String,
    pub topic_id: String,
    pub lifecycle_state: String,
    pub current_value: String,
    pending_updates: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtdTopicUpdateSummary {
    pub provider_id: String,
    pub topic_id: String,
    pub lifecycle_state: String,
    pub current_value: String,
    pub remaining_update_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxRtdRegistryEntry {
    pub provider_id: String,
    pub topic_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxRtdRegistrySummary {
    pub runtime_platform: String,
    pub gate_state: String,
    pub reason: String,
    pub entries: Vec<LinuxRtdRegistryEntry>,
}

pub fn admitted_extension_abi(
    host_profile_id: &str,
    platform_gate_id: &str,
) -> ExtensionAbiContract {
    ExtensionAbiContract {
        abi_id: "dnaonecalc.desktop.extension_abi".to_string(),
        abi_version: "v1".to_string(),
        host_profile_id: host_profile_id.to_string(),
        platform_gate_id: platform_gate_id.to_string(),
        admitted_capabilities: vec!["host_managed_function_registration".to_string()],
        excluded_capabilities: vec![
            "worksheet_register_call_semantics".to_string(),
            "rtd_provider".to_string(),
            "vba_bridge".to_string(),
            "browser_host".to_string(),
        ],
    }
}

pub fn validate_extension_manifest(
    manifest: &ExtensionProviderManifest,
    host_profile_id: &str,
    platform_gate_id: &str,
) -> ExtensionValidationResult {
    let contract = admitted_extension_abi(host_profile_id, platform_gate_id);
    let mut blocked_reasons = Vec::new();
    let mut admitted_capabilities = Vec::new();
    let mut blocked_capabilities = Vec::new();

    if manifest.abi_version != contract.abi_version {
        blocked_reasons.push(format!(
            "abi_version {} does not match admitted {}",
            manifest.abi_version, contract.abi_version
        ));
    }
    if !manifest
        .host_profile_ids
        .iter()
        .any(|item| item == host_profile_id)
    {
        blocked_reasons.push(format!(
            "host profile {} is not declared by provider",
            host_profile_id
        ));
    }
    if !manifest
        .platform_gate_ids
        .iter()
        .any(|item| item == platform_gate_id)
    {
        blocked_reasons.push(format!(
            "platform gate {} is not declared by provider",
            platform_gate_id
        ));
    }

    for capability in &manifest.declared_capabilities {
        if contract
            .admitted_capabilities
            .iter()
            .any(|admitted| admitted == capability)
        {
            admitted_capabilities.push(capability.clone());
        } else {
            blocked_capabilities.push(capability.clone());
            blocked_reasons.push(format!(
                "capability {} is not admitted in OneCalc v1",
                capability
            ));
        }
    }

    ExtensionValidationResult {
        provider_id: manifest.provider_id.clone(),
        admitted: blocked_reasons.is_empty() && !admitted_capabilities.is_empty(),
        admitted_capabilities,
        blocked_capabilities,
        blocked_reasons,
    }
}

pub fn load_extension_root(
    extension_root: impl AsRef<Path>,
    host_profile_id: &str,
    platform_gate_id: &str,
) -> Result<ExtensionRootLoadSummary, String> {
    let extension_root = extension_root.as_ref();
    if !extension_root.exists() {
        return Err(format!(
            "extension root {} does not exist",
            extension_root.display()
        ));
    }
    if !extension_root.is_dir() {
        return Err(format!(
            "extension root {} is not a directory",
            extension_root.display()
        ));
    }

    let manifest_paths = discover_manifest_paths(extension_root)?;
    let mut admitted_providers = Vec::new();
    let mut rejected_providers = Vec::new();
    let mut malformed_manifests = Vec::new();

    for manifest_path in &manifest_paths {
        let body = match fs::read_to_string(manifest_path) {
            Ok(body) => body,
            Err(error) => {
                malformed_manifests.push(ExtensionManifestLoadFailure {
                    manifest_path: manifest_path.display().to_string(),
                    reason: format!("read failed: {error}"),
                });
                continue;
            }
        };

        let manifest = match serde_json::from_str::<ExtensionProviderManifest>(&body) {
            Ok(manifest) => manifest,
            Err(error) => {
                malformed_manifests.push(ExtensionManifestLoadFailure {
                    manifest_path: manifest_path.display().to_string(),
                    reason: format!("json parse failed: {error}"),
                });
                continue;
            }
        };

        let validation = validate_extension_manifest(&manifest, host_profile_id, platform_gate_id);
        let loaded = LoadedExtensionProvider {
            manifest_path: manifest_path.display().to_string(),
            manifest,
            validation,
        };

        if loaded.validation.admitted {
            admitted_providers.push(loaded);
        } else {
            rejected_providers.push(loaded);
        }
    }

    Ok(ExtensionRootLoadSummary {
        extension_root: extension_root.display().to_string(),
        discovered_manifest_count: manifest_paths.len(),
        admitted_providers,
        rejected_providers,
        malformed_manifests,
    })
}

pub fn invoke_extension_provider(
    extension_root: impl AsRef<Path>,
    host_profile_id: &str,
    platform_gate_id: &str,
    provider_id: &str,
    function_name: &str,
    arguments: &[ExtensionInvocationArgument],
) -> Result<ExtensionInvocationSummary, String> {
    let loaded = load_extension_root(extension_root.as_ref(), host_profile_id, platform_gate_id)?;

    if let Some(provider) = loaded
        .admitted_providers
        .iter()
        .find(|provider| provider.manifest.provider_id == provider_id)
    {
        return invoke_loaded_provider(provider, function_name, arguments);
    }

    if let Some(provider) = loaded
        .rejected_providers
        .iter()
        .find(|provider| provider.manifest.provider_id == provider_id)
    {
        return Ok(ExtensionInvocationSummary {
            provider_id: provider_id.to_string(),
            function_name: function_name.to_string(),
            provider_state: "rejected".to_string(),
            invocation_state: "blocked".to_string(),
            value_summary: None,
            failure_reason: Some(provider.validation.blocked_reasons.join(" | ")),
        });
    }

    Ok(ExtensionInvocationSummary {
        provider_id: provider_id.to_string(),
        function_name: function_name.to_string(),
        provider_state: "missing".to_string(),
        invocation_state: "blocked".to_string(),
        value_summary: None,
        failure_reason: Some("provider not discovered under extension root".to_string()),
    })
}

pub fn extension_root_runtime_truth(
    extension_root: impl AsRef<Path>,
    host_profile_id: &str,
    platform_gate_id: &str,
    runtime_platform: &str,
) -> Result<ExtensionRootRuntimeTruthSummary, String> {
    let loaded = load_extension_root(extension_root.as_ref(), host_profile_id, platform_gate_id)?;
    let provider_truths = loaded
        .admitted_providers
        .iter()
        .chain(loaded.rejected_providers.iter())
        .map(|provider| project_provider_runtime_truth(provider, runtime_platform))
        .collect::<Vec<_>>();

    Ok(ExtensionRootRuntimeTruthSummary {
        extension_root: loaded.extension_root,
        runtime_platform: runtime_platform.to_string(),
        provider_truths,
        manifest_failures: loaded.malformed_manifests,
    })
}

pub fn activate_windows_rtd_topic(
    extension_root: impl AsRef<Path>,
    host_profile_id: &str,
    platform_gate_id: &str,
    runtime_platform: &str,
    provider_id: &str,
    topic_id: &str,
) -> Result<ActivatedRtdTopicSession, String> {
    if !runtime_platform.eq_ignore_ascii_case("windows") {
        return Err("Windows RTD activation is not admitted on this platform".to_string());
    }

    let loaded = load_extension_root(extension_root.as_ref(), host_profile_id, platform_gate_id)?;
    let provider = loaded
        .rejected_providers
        .iter()
        .find(|provider| provider.manifest.provider_id == provider_id)
        .or_else(|| {
            loaded
                .admitted_providers
                .iter()
                .find(|provider| provider.manifest.provider_id == provider_id)
        })
        .ok_or_else(|| {
            format!(
                "provider {} not discovered under extension root",
                provider_id
            )
        })?;
    let entrypoint = load_provider_entrypoint(provider)?;
    let topic = entrypoint
        .rtd_topics
        .iter()
        .find(|topic| topic.topic_id == topic_id)
        .ok_or_else(|| {
            format!(
                "RTD topic {} is not declared by provider {}",
                topic_id, provider.manifest.provider_id
            )
        })?;

    Ok(ActivatedRtdTopicSession {
        provider_id: provider.manifest.provider_id.clone(),
        topic_id: topic.topic_id.clone(),
        lifecycle_state: "active".to_string(),
        current_value: topic.initial_value.clone(),
        pending_updates: topic.updates.clone(),
    })
}

pub fn linux_rtd_registry_truth(
    extension_root: impl AsRef<Path>,
    host_profile_id: &str,
    platform_gate_id: &str,
    runtime_platform: &str,
) -> Result<LinuxRtdRegistrySummary, String> {
    let loaded = load_extension_root(extension_root.as_ref(), host_profile_id, platform_gate_id)?;
    let entries = loaded
        .admitted_providers
        .iter()
        .chain(loaded.rejected_providers.iter())
        .filter_map(|provider| {
            let entrypoint = load_provider_entrypoint(provider).ok()?;
            if entrypoint.rtd_topics.is_empty() {
                return None;
            }

            Some(LinuxRtdRegistryEntry {
                provider_id: provider.manifest.provider_id.clone(),
                topic_ids: entrypoint
                    .rtd_topics
                    .iter()
                    .map(|topic| topic.topic_id.clone())
                    .collect::<Vec<_>>(),
            })
        })
        .collect::<Vec<_>>();

    let (gate_state, reason) = if runtime_platform.eq_ignore_ascii_case("linux") {
        (
            "admitted_design_floor".to_string(),
            "Linux RTD registry truth is available; activation remains bounded to the declared subset.".to_string(),
        )
    } else {
        (
            "blocked_on_host_platform".to_string(),
            "Linux RTD registry remains explicit design truth only on this host platform."
                .to_string(),
        )
    };

    Ok(LinuxRtdRegistrySummary {
        runtime_platform: runtime_platform.to_string(),
        gate_state,
        reason,
        entries,
    })
}

pub fn advance_rtd_topic(session: &mut ActivatedRtdTopicSession) -> RtdTopicUpdateSummary {
    if let Some(next_value) = session.pending_updates.first().cloned() {
        session.current_value = next_value;
        session.pending_updates.remove(0);
        session.lifecycle_state = if session.pending_updates.is_empty() {
            "active_final_value".to_string()
        } else {
            "active".to_string()
        };
    } else {
        session.lifecycle_state = "active_no_pending_updates".to_string();
    }

    RtdTopicUpdateSummary {
        provider_id: session.provider_id.clone(),
        topic_id: session.topic_id.clone(),
        lifecycle_state: session.lifecycle_state.clone(),
        current_value: session.current_value.clone(),
        remaining_update_count: session.pending_updates.len(),
    }
}

fn discover_manifest_paths(extension_root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut manifest_paths = Vec::new();
    let root_manifest = extension_root.join("provider.json");
    if root_manifest.is_file() {
        manifest_paths.push(root_manifest);
    }

    let mut entries = fs::read_dir(extension_root)
        .map_err(|error| format!("failed to enumerate extension root: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("failed to enumerate extension root: {error}"))?;
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let manifest_path = path.join("provider.json");
        if manifest_path.is_file() {
            manifest_paths.push(manifest_path);
        }
    }

    Ok(manifest_paths)
}

fn invoke_loaded_provider(
    provider: &LoadedExtensionProvider,
    function_name: &str,
    arguments: &[ExtensionInvocationArgument],
) -> Result<ExtensionInvocationSummary, String> {
    let entrypoint = load_provider_entrypoint(provider)?;
    let Some(function) = entrypoint
        .registered_functions
        .iter()
        .find(|item| item.function_name == function_name)
    else {
        return Ok(ExtensionInvocationSummary {
            provider_id: provider.manifest.provider_id.clone(),
            function_name: function_name.to_string(),
            provider_state: "admitted".to_string(),
            invocation_state: "blocked".to_string(),
            value_summary: None,
            failure_reason: Some(format!(
                "function {} is not registered by provider {}",
                function_name, provider.manifest.provider_id
            )),
        });
    };

    let (invocation_state, value_summary, failure_reason) =
        match execute_registered_function(&function.behavior, arguments) {
            Ok(value_summary) => ("returned".to_string(), Some(value_summary), None),
            Err(reason) => ("provider_error".to_string(), None, Some(reason)),
        };

    Ok(ExtensionInvocationSummary {
        provider_id: provider.manifest.provider_id.clone(),
        function_name: function_name.to_string(),
        provider_state: "admitted".to_string(),
        invocation_state,
        value_summary,
        failure_reason,
    })
}

fn load_provider_entrypoint(
    provider: &LoadedExtensionProvider,
) -> Result<ExtensionProviderEntrypoint, String> {
    let manifest_dir = Path::new(&provider.manifest_path).parent().ok_or_else(|| {
        format!(
            "manifest {} has no parent directory",
            provider.manifest_path
        )
    })?;
    let entrypoint_path = manifest_dir.join(&provider.manifest.entrypoint);
    let body = fs::read_to_string(&entrypoint_path).map_err(|error| {
        format!(
            "failed to read entrypoint {}: {error}",
            entrypoint_path.display()
        )
    })?;
    serde_json::from_str(&body).map_err(|error| {
        format!(
            "failed to parse entrypoint {}: {error}",
            entrypoint_path.display()
        )
    })
}

fn execute_registered_function(
    behavior: &RegisteredExtensionBehavior,
    arguments: &[ExtensionInvocationArgument],
) -> Result<String, String> {
    match behavior {
        RegisteredExtensionBehavior::SumNumbers => {
            let value = arguments
                .iter()
                .map(|argument| match argument {
                    ExtensionInvocationArgument::Number(value) => *value,
                })
                .sum::<f64>();
            Ok(format!("Number({value})"))
        }
        RegisteredExtensionBehavior::AlwaysError { message } => Err(message.clone()),
    }
}

fn project_provider_runtime_truth(
    provider: &LoadedExtensionProvider,
    runtime_platform: &str,
) -> ExtensionProviderRuntimeTruth {
    let provider_state =
        if provider.validation.admitted && provider.validation.blocked_capabilities.is_empty() {
            "admitted".to_string()
        } else if provider.manifest.declared_capabilities.is_empty() {
            "blocked".to_string()
        } else {
            "declared_with_blocked_capabilities".to_string()
        };

    let capability_truths = provider
        .manifest
        .declared_capabilities
        .iter()
        .map(|capability| match capability.as_str() {
            "host_managed_function_registration" => ExtensionCapabilityTruth {
                capability_id: capability.clone(),
                declaration_state: "declared".to_string(),
                runtime_state: if provider
                    .validation
                    .admitted_capabilities
                    .iter()
                    .any(|item| item == capability)
                {
                    "admitted".to_string()
                } else {
                    "blocked".to_string()
                },
                reason: capability_reason(provider, capability),
            },
            "rtd_provider" => ExtensionCapabilityTruth {
                capability_id: capability.clone(),
                declaration_state: "declared".to_string(),
                runtime_state: {
                    let (runtime_state, _) = rtd_runtime_state(provider, runtime_platform);
                    runtime_state
                },
                reason: {
                    let (_, reason) = rtd_runtime_state(provider, runtime_platform);
                    reason
                },
            },
            _ => ExtensionCapabilityTruth {
                capability_id: capability.clone(),
                declaration_state: "declared".to_string(),
                runtime_state: "blocked".to_string(),
                reason: capability_reason(provider, capability),
            },
        })
        .collect::<Vec<_>>();

    ExtensionProviderRuntimeTruth {
        provider_id: provider.manifest.provider_id.clone(),
        display_name: provider.manifest.display_name.clone(),
        manifest_path: provider.manifest_path.clone(),
        provider_state,
        capability_truths,
        blocked_reasons: provider.validation.blocked_reasons.clone(),
    }
}

fn capability_reason(provider: &LoadedExtensionProvider, capability: &str) -> Option<String> {
    if provider
        .validation
        .admitted_capabilities
        .iter()
        .any(|item| item == capability)
    {
        None
    } else {
        let matching_reasons = provider
            .validation
            .blocked_reasons
            .iter()
            .filter(|reason| reason.contains(capability))
            .cloned()
            .collect::<Vec<_>>();
        if matching_reasons.is_empty() {
            Some(provider.validation.blocked_reasons.join(" | "))
        } else {
            Some(matching_reasons.join(" | "))
        }
    }
}

fn rtd_runtime_state(
    provider: &LoadedExtensionProvider,
    runtime_platform: &str,
) -> (String, Option<String>) {
    if !runtime_platform.eq_ignore_ascii_case("windows") {
        return (
            "blocked_by_platform".to_string(),
            Some(
                "RTD remains blocked on this platform until the Linux activation path lands."
                    .to_string(),
            ),
        );
    }

    match load_provider_entrypoint(provider) {
        Ok(entrypoint) if !entrypoint.rtd_topics.is_empty() => (
            "admitted_windows_subset".to_string(),
            Some(
                "Windows in-process RTD topic activation is available for the admitted subset."
                    .to_string(),
            ),
        ),
        _ => (
            "declared_but_not_yet_admitted".to_string(),
            Some(
                "RTD lifecycle is a later admitted Windows subset and is not executable yet."
                    .to_string(),
            ),
        ),
    }
}
