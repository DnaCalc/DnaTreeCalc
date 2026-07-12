use std::env;

use dnaonecalc_host::{
    launch_shell, launch_shell_with_formula, OneCalcHostProfile, RecalcContext,
    RetainedScenarioStore, RuntimeAdapter,
};

fn main() {
    match env::args().nth(1).as_deref() {
        Some("--probe") => run_probe(),
        Some("--function-surface-smoke") => run_function_surface_smoke(),
        Some("--capability-snapshot-smoke") => run_capability_snapshot_smoke(),
        Some("--capability-center-smoke") => run_capability_center_smoke(),
        Some("--extension-abi-smoke") => run_extension_abi_smoke(),
        Some("--extension-root-smoke") => run_extension_root_smoke(),
        Some("--extension-provider-smoke") => run_extension_provider_smoke(),
        Some("--extension-rtd-state-smoke") => run_extension_rtd_state_smoke(),
        Some("--windows-rtd-smoke") => run_windows_rtd_smoke(),
        Some("--linux-rtd-gate-smoke") => run_linux_rtd_gate_smoke(),
        Some("--scenario-index-smoke") => run_scenario_index_smoke(),
        Some("--scenario-library-filter-smoke") => run_scenario_library_filter_smoke(),
        Some("--scenario-selection-smoke") => run_scenario_selection_smoke(),
        Some("--acceptance-matrix-smoke") => run_acceptance_matrix_smoke(),
        Some("--scenario-regression-smoke") => run_scenario_regression_smoke(),
        Some("--upstream-pressure-smoke") => run_upstream_pressure_smoke(),
        Some("--h1-smoke") => run_h1_smoke(),
        Some("--h1-retained-smoke") => run_h1_retained_smoke(),
        Some("--h1-compare-smoke") => run_h1_compare_smoke(),
        Some("--replay-capture-smoke") => run_replay_capture_smoke(),
        Some("--xray-diff-smoke") => run_xray_diff_smoke(),
        Some("--witness-smoke") => run_witness_smoke(),
        Some("--handoff-smoke") => run_handoff_smoke(),
        Some("--document-roundtrip-smoke") => run_document_roundtrip_smoke(),
        Some("--workspace-smoke") => run_workspace_smoke(),
        Some("--windows-observation-smoke") => run_windows_observation_smoke(),
        Some("--twin-compare-smoke") => run_twin_compare_smoke(),
        Some("--widening-request-smoke") => run_widening_request_smoke(),
        Some("--scenario-capsule-smoke") => run_scenario_capsule_smoke(),
        Some("--shell-smoke") => run_shell(true),
        Some("--editor-diagnostic-smoke") => run_editor_diagnostic_smoke(),
        Some(flag) => {
            eprintln!("unknown flag: {flag}");
            eprintln!(
                "supported flags: --probe, --function-surface-smoke, --capability-snapshot-smoke, --capability-center-smoke, --extension-abi-smoke, --extension-root-smoke, --extension-provider-smoke, --extension-rtd-state-smoke, --windows-rtd-smoke, --linux-rtd-gate-smoke, --scenario-index-smoke, --scenario-library-filter-smoke, --scenario-selection-smoke, --acceptance-matrix-smoke, --scenario-regression-smoke, --upstream-pressure-smoke, --h1-smoke, --h1-retained-smoke, --h1-compare-smoke, --replay-capture-smoke, --xray-diff-smoke, --witness-smoke, --handoff-smoke, --document-roundtrip-smoke, --workspace-smoke, --windows-observation-smoke, --twin-compare-smoke, --widening-request-smoke, --scenario-capsule-smoke, --shell-smoke, --editor-diagnostic-smoke"
            );
            std::process::exit(2);
        }
        None => run_shell(false),
    }
}

fn run_probe() {
    let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH0);
    let packet_register = adapter
        .packet_kinds()
        .iter()
        .map(|packet| packet.id())
        .collect::<Vec<_>>()
        .join(",");

    match adapter.dependency_probe() {
        Ok(report) => {
            println!("dnaonecalc-host dependency probe");
            println!("host_profile={}", adapter.host_profile().id());
            println!("packet_kinds={packet_register}");
            println!("formula_token={}", report.formula_token);
            println!("parse_token_count={}", report.parse_token_count);
            println!("parse_diagnostic_count={}", report.parse_diagnostic_count);
            println!("sum_result={}", report.sum_result);
            println!("replay_ready={}", report.replay_ready);
            println!(
                "replay_registry_ref_count={}",
                report.replay_registry_ref_count
            );
        }
        Err(error) => {
            eprintln!("dependency probe failed: {error:?}");
            std::process::exit(1);
        }
    }
}

fn run_function_surface_smoke() {
    let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH0);
    let catalog = adapter.load_function_surface_catalog();
    let summary = catalog.label_summary();

    let abs = catalog
        .get("ABS")
        .expect("ABS should exist in the snapshot");
    let call = catalog
        .get("CALL")
        .expect("CALL should exist in the snapshot");
    let accrint = catalog
        .get("ACCRINT")
        .expect("ACCRINT should exist in the snapshot");
    let encodeurl = catalog
        .get("ENCODEURL")
        .expect("ENCODEURL should exist in the snapshot");

    println!("dnaonecalc-host function surface smoke");
    println!(
        "label_summary=supported:{};preview:{};experimental:{};deferred:{};catalog_only:{}",
        summary.supported,
        summary.preview,
        summary.experimental,
        summary.deferred,
        summary.catalog_only
    );
    println!(
        "ABS={} CALL={} ACCRINT={} ENCODEURL={}",
        abs.admission_category.id(),
        call.admission_category.id(),
        accrint.admission_category.id(),
        encodeurl.admission_category.id()
    );
}

fn run_capability_snapshot_smoke() {
    let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
    let snapshot = adapter
        .emit_capability_snapshot("edit_accept_recalc", None)
        .expect("capability snapshot should emit");

    println!("dnaonecalc-host capability snapshot smoke");
    println!("capability_snapshot_id={}", snapshot.capability_snapshot_id);
    println!("capability_floor={}", snapshot.capability_floor);
    println!(
        "function_surface_snapshot_ref={}",
        snapshot.function_surface_snapshot_ref
    );
    println!("packet_kinds={}", snapshot.packet_kind_register.join(","));
    for mode in &snapshot.mode_availability {
        println!(
            "mode={} state={} reason={}",
            mode.mode_id,
            mode.state,
            mode.reason.as_deref().unwrap_or("none")
        );
    }
}

fn run_capability_center_smoke() {
    let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
    let root = env::temp_dir().join("dnaonecalc-capability-center-smoke");
    let _ = std::fs::remove_dir_all(&root);
    let store = RetainedScenarioStore::new(&root);

    let left = adapter
        .persist_capability_snapshot(&store, "formula_edit", None)
        .expect("left snapshot should persist");
    let right = adapter
        .persist_capability_snapshot(
            &store,
            "edit_accept_recalc",
            Some(&left.snapshot.capability_snapshot_id),
        )
        .expect("right snapshot should persist");
    let opened = adapter
        .open_capability_snapshot(&store, &right.snapshot.capability_snapshot_id)
        .expect("current snapshot should open");
    let diff = adapter
        .diff_capability_snapshots(
            &store,
            &left.snapshot.capability_snapshot_id,
            &right.snapshot.capability_snapshot_id,
        )
        .expect("snapshot diff should open");

    println!("dnaonecalc-host capability center smoke");
    println!("snapshot_id={}", opened.capability_snapshot_id);
    println!("runtime_class={}", opened.runtime_class);
    println!("dependencies={}", opened.dependency_set.join(","));
    println!("packet_kinds={}", opened.packet_kind_register.join(","));
    println!(
        "modes={}",
        opened
            .mode_availability
            .iter()
            .map(|mode| format!("{}:{}", mode.mode_id, mode.state))
            .collect::<Vec<_>>()
            .join(",")
    );
    println!(
        "diff=left:{};right:{};mode_changes:{};policy_changed:{};runtime_class_changed:{}",
        diff.left_snapshot_id,
        diff.right_snapshot_id,
        diff.mode_changes.join("|"),
        diff.function_surface_policy_changed,
        diff.runtime_class_changed
    );
}

fn run_extension_abi_smoke() {
    let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
    let contract = adapter.extension_abi_contract();
    let admitted_manifest = dnaonecalc_host::ExtensionProviderManifest {
        provider_id: "demo.sum.provider".to_string(),
        display_name: "Demo Sum Provider".to_string(),
        abi_version: "v1".to_string(),
        host_profile_ids: vec!["OC-H1".to_string()],
        platform_gate_ids: vec!["desktop_native_only".to_string()],
        declared_capabilities: vec!["host_managed_function_registration".to_string()],
        entrypoint: "providers/demo_sum".to_string(),
    };
    let blocked_manifest = dnaonecalc_host::ExtensionProviderManifest {
        provider_id: "demo.rtd.provider".to_string(),
        display_name: "Demo RTD Provider".to_string(),
        abi_version: "v1".to_string(),
        host_profile_ids: vec!["OC-H1".to_string()],
        platform_gate_ids: vec!["desktop_native_only".to_string()],
        declared_capabilities: vec!["rtd_provider".to_string()],
        entrypoint: "providers/demo_rtd".to_string(),
    };
    let admitted = adapter.validate_extension_manifest(&admitted_manifest);
    let blocked = adapter.validate_extension_manifest(&blocked_manifest);

    println!("dnaonecalc-host extension abi smoke");
    println!(
        "abi=id:{};version:{};host:{};platform:{}",
        contract.abi_id, contract.abi_version, contract.host_profile_id, contract.platform_gate_id
    );
    println!(
        "admitted_capabilities={}",
        contract.admitted_capabilities.join(",")
    );
    println!(
        "excluded_capabilities={}",
        contract.excluded_capabilities.join(",")
    );
    println!(
        "admitted_provider={};admitted:{};caps:{}",
        admitted.provider_id,
        admitted.admitted,
        admitted.admitted_capabilities.join(",")
    );
    println!(
        "blocked_provider={};admitted:{};blocked_caps:{};reasons:{}",
        blocked.provider_id,
        blocked.admitted,
        blocked.blocked_capabilities.join(","),
        blocked.blocked_reasons.join("|")
    );
}

fn run_extension_root_smoke() {
    let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
    let root = env::temp_dir().join("dnaonecalc-extension-root-smoke");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("demo-sum")).expect("demo sum dir should create");
    std::fs::create_dir_all(root.join("demo-rtd")).expect("demo rtd dir should create");

    let admitted_manifest = dnaonecalc_host::ExtensionProviderManifest {
        provider_id: "demo.sum.provider".to_string(),
        display_name: "Demo Sum Provider".to_string(),
        abi_version: "v1".to_string(),
        host_profile_ids: vec!["OC-H1".to_string()],
        platform_gate_ids: vec!["desktop_native_only".to_string()],
        declared_capabilities: vec!["host_managed_function_registration".to_string()],
        entrypoint: "functions.json".to_string(),
    };
    let blocked_manifest = dnaonecalc_host::ExtensionProviderManifest {
        provider_id: "demo.rtd.provider".to_string(),
        display_name: "Demo RTD Provider".to_string(),
        abi_version: "v1".to_string(),
        host_profile_ids: vec!["OC-H1".to_string()],
        platform_gate_ids: vec!["desktop_native_only".to_string()],
        declared_capabilities: vec!["rtd_provider".to_string()],
        entrypoint: "functions.json".to_string(),
    };

    std::fs::write(
        root.join("demo-sum").join("provider.json"),
        serde_json::to_string_pretty(&admitted_manifest)
            .expect("admitted manifest should serialize"),
    )
    .expect("admitted manifest should write");
    std::fs::write(
        root.join("demo-sum").join("functions.json"),
        serde_json::to_string_pretty(&dnaonecalc_host::ExtensionProviderEntrypoint {
            registered_functions: vec![dnaonecalc_host::RegisteredExtensionFunction {
                function_name: "DEMOADD".to_string(),
                behavior: dnaonecalc_host::RegisteredExtensionBehavior::SumNumbers,
            }],
            rtd_topics: Vec::new(),
        })
        .expect("entrypoint should serialize"),
    )
    .expect("entrypoint should write");
    std::fs::write(
        root.join("demo-rtd").join("provider.json"),
        serde_json::to_string_pretty(&blocked_manifest).expect("blocked manifest should serialize"),
    )
    .expect("blocked manifest should write");

    let loaded = adapter
        .load_extension_root(&root)
        .expect("extension root should load");

    println!("dnaonecalc-host extension root smoke");
    println!("extension_root={}", loaded.extension_root);
    println!(
        "discovered_manifest_count={}",
        loaded.discovered_manifest_count
    );
    println!(
        "admitted_providers={}",
        loaded
            .admitted_providers
            .iter()
            .map(|provider| {
                format!(
                    "{}:{}",
                    provider.manifest.provider_id,
                    provider.validation.admitted_capabilities.join("+")
                )
            })
            .collect::<Vec<_>>()
            .join(",")
    );
    println!(
        "rejected_providers={}",
        loaded
            .rejected_providers
            .iter()
            .map(|provider| {
                format!(
                    "{}:{}",
                    provider.manifest.provider_id,
                    provider.validation.blocked_reasons.join("|")
                )
            })
            .collect::<Vec<_>>()
            .join(",")
    );
}

fn run_extension_provider_smoke() {
    let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
    let root = env::temp_dir().join("dnaonecalc-extension-provider-smoke");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("demo-sum")).expect("demo sum dir should create");
    std::fs::create_dir_all(root.join("demo-fail")).expect("demo fail dir should create");
    std::fs::create_dir_all(root.join("demo-rtd")).expect("demo rtd dir should create");

    let admitted_manifest = dnaonecalc_host::ExtensionProviderManifest {
        provider_id: "demo.sum.provider".to_string(),
        display_name: "Demo Sum Provider".to_string(),
        abi_version: "v1".to_string(),
        host_profile_ids: vec!["OC-H1".to_string()],
        platform_gate_ids: vec!["desktop_native_only".to_string()],
        declared_capabilities: vec!["host_managed_function_registration".to_string()],
        entrypoint: "functions.json".to_string(),
    };
    let failing_manifest = dnaonecalc_host::ExtensionProviderManifest {
        provider_id: "demo.fail.provider".to_string(),
        display_name: "Demo Fail Provider".to_string(),
        abi_version: "v1".to_string(),
        host_profile_ids: vec!["OC-H1".to_string()],
        platform_gate_ids: vec!["desktop_native_only".to_string()],
        declared_capabilities: vec!["host_managed_function_registration".to_string()],
        entrypoint: "functions.json".to_string(),
    };
    let blocked_manifest = dnaonecalc_host::ExtensionProviderManifest {
        provider_id: "demo.rtd.provider".to_string(),
        display_name: "Demo RTD Provider".to_string(),
        abi_version: "v1".to_string(),
        host_profile_ids: vec!["OC-H1".to_string()],
        platform_gate_ids: vec!["desktop_native_only".to_string()],
        declared_capabilities: vec!["rtd_provider".to_string()],
        entrypoint: "functions.json".to_string(),
    };

    std::fs::write(
        root.join("demo-sum").join("provider.json"),
        serde_json::to_string_pretty(&admitted_manifest)
            .expect("admitted manifest should serialize"),
    )
    .expect("admitted manifest should write");
    std::fs::write(
        root.join("demo-sum").join("functions.json"),
        serde_json::to_string_pretty(&dnaonecalc_host::ExtensionProviderEntrypoint {
            registered_functions: vec![dnaonecalc_host::RegisteredExtensionFunction {
                function_name: "DEMOADD".to_string(),
                behavior: dnaonecalc_host::RegisteredExtensionBehavior::SumNumbers,
            }],
            rtd_topics: Vec::new(),
        })
        .expect("sum entrypoint should serialize"),
    )
    .expect("sum entrypoint should write");

    std::fs::write(
        root.join("demo-fail").join("provider.json"),
        serde_json::to_string_pretty(&failing_manifest).expect("failing manifest should serialize"),
    )
    .expect("failing manifest should write");
    std::fs::write(
        root.join("demo-fail").join("functions.json"),
        serde_json::to_string_pretty(&dnaonecalc_host::ExtensionProviderEntrypoint {
            registered_functions: vec![dnaonecalc_host::RegisteredExtensionFunction {
                function_name: "DEMOFAIL".to_string(),
                behavior: dnaonecalc_host::RegisteredExtensionBehavior::AlwaysError {
                    message: "provider execution failed".to_string(),
                },
            }],
            rtd_topics: Vec::new(),
        })
        .expect("failing entrypoint should serialize"),
    )
    .expect("failing entrypoint should write");

    std::fs::write(
        root.join("demo-rtd").join("provider.json"),
        serde_json::to_string_pretty(&blocked_manifest).expect("blocked manifest should serialize"),
    )
    .expect("blocked manifest should write");

    let sum = adapter
        .invoke_extension_provider(
            &root,
            "demo.sum.provider",
            "DEMOADD",
            &[
                dnaonecalc_host::ExtensionInvocationArgument::Number(1.0),
                dnaonecalc_host::ExtensionInvocationArgument::Number(2.0),
                dnaonecalc_host::ExtensionInvocationArgument::Number(3.0),
            ],
        )
        .expect("sum provider should invoke");
    let fail = adapter
        .invoke_extension_provider(&root, "demo.fail.provider", "DEMOFAIL", &[])
        .expect("failing provider should return explicit state");
    let blocked = adapter
        .invoke_extension_provider(&root, "demo.rtd.provider", "RTDDEMO", &[])
        .expect("blocked provider should return explicit state");

    println!("dnaonecalc-host extension provider smoke");
    println!(
        "sum=provider:{};state:{};invocation:{};value:{}",
        sum.provider_id,
        sum.provider_state,
        sum.invocation_state,
        sum.value_summary.as_deref().unwrap_or("none")
    );
    println!(
        "fail=provider:{};state:{};invocation:{};reason:{}",
        fail.provider_id,
        fail.provider_state,
        fail.invocation_state,
        fail.failure_reason.as_deref().unwrap_or("none")
    );
    println!(
        "blocked=provider:{};state:{};invocation:{};reason:{}",
        blocked.provider_id,
        blocked.provider_state,
        blocked.invocation_state,
        blocked.failure_reason.as_deref().unwrap_or("none")
    );
}

fn run_extension_rtd_state_smoke() {
    let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
    let root = env::temp_dir().join("dnaonecalc-extension-rtd-state-smoke");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("demo-sum")).expect("demo sum dir should create");
    std::fs::create_dir_all(root.join("demo-rtd")).expect("demo rtd dir should create");

    let admitted_manifest = dnaonecalc_host::ExtensionProviderManifest {
        provider_id: "demo.sum.provider".to_string(),
        display_name: "Demo Sum Provider".to_string(),
        abi_version: "v1".to_string(),
        host_profile_ids: vec!["OC-H1".to_string()],
        platform_gate_ids: vec!["desktop_native_only".to_string()],
        declared_capabilities: vec!["host_managed_function_registration".to_string()],
        entrypoint: "functions.json".to_string(),
    };
    let rtd_manifest = dnaonecalc_host::ExtensionProviderManifest {
        provider_id: "demo.rtd.provider".to_string(),
        display_name: "Demo RTD Provider".to_string(),
        abi_version: "v1".to_string(),
        host_profile_ids: vec!["OC-H1".to_string()],
        platform_gate_ids: vec!["desktop_native_only".to_string()],
        declared_capabilities: vec!["rtd_provider".to_string()],
        entrypoint: "functions.json".to_string(),
    };

    std::fs::write(
        root.join("demo-sum").join("provider.json"),
        serde_json::to_string_pretty(&admitted_manifest)
            .expect("admitted manifest should serialize"),
    )
    .expect("admitted manifest should write");
    std::fs::write(
        root.join("demo-rtd").join("provider.json"),
        serde_json::to_string_pretty(&rtd_manifest).expect("rtd manifest should serialize"),
    )
    .expect("rtd manifest should write");

    let truth = adapter
        .extension_root_runtime_truth(&root)
        .expect("extension truth should load");

    println!("dnaonecalc-host extension rtd state smoke");
    println!("runtime_platform={}", truth.runtime_platform);
    for provider in &truth.provider_truths {
        println!(
            "provider={} state={} blocked_reasons={}",
            provider.provider_id,
            provider.provider_state,
            provider.blocked_reasons.join("|")
        );
        for capability in &provider.capability_truths {
            println!(
                "capability={} declaration={} runtime={} reason={}",
                capability.capability_id,
                capability.declaration_state,
                capability.runtime_state,
                capability.reason.as_deref().unwrap_or("none")
            );
        }
    }
}

fn run_windows_rtd_smoke() {
    if std::env::consts::OS != "windows" {
        eprintln!("windows rtd smoke is only admitted on Windows");
        std::process::exit(1);
    }

    let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
    let root = env::temp_dir().join("dnaonecalc-windows-rtd-smoke");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("demo-rtd")).expect("demo rtd dir should create");

    let rtd_manifest = dnaonecalc_host::ExtensionProviderManifest {
        provider_id: "demo.rtd.provider".to_string(),
        display_name: "Demo RTD Provider".to_string(),
        abi_version: "v1".to_string(),
        host_profile_ids: vec!["OC-H1".to_string()],
        platform_gate_ids: vec!["desktop_native_only".to_string()],
        declared_capabilities: vec!["rtd_provider".to_string()],
        entrypoint: "functions.json".to_string(),
    };

    std::fs::write(
        root.join("demo-rtd").join("provider.json"),
        serde_json::to_string_pretty(&rtd_manifest).expect("rtd manifest should serialize"),
    )
    .expect("rtd manifest should write");
    std::fs::write(
        root.join("demo-rtd").join("functions.json"),
        serde_json::to_string_pretty(&dnaonecalc_host::ExtensionProviderEntrypoint {
            registered_functions: Vec::new(),
            rtd_topics: vec![dnaonecalc_host::RegisteredRtdTopic {
                topic_id: "PRICE".to_string(),
                initial_value: "100.0".to_string(),
                updates: vec!["101.5".to_string(), "103.0".to_string()],
            }],
        })
        .expect("rtd entrypoint should serialize"),
    )
    .expect("rtd entrypoint should write");

    let truth = adapter
        .extension_root_runtime_truth(&root)
        .expect("extension truth should load");
    let mut session = adapter
        .activate_windows_rtd_topic(&root, "demo.rtd.provider", "PRICE")
        .expect("windows rtd topic should activate");
    let first = adapter.advance_rtd_topic(&mut session);
    let second = adapter.advance_rtd_topic(&mut session);
    let third = adapter.advance_rtd_topic(&mut session);

    println!("dnaonecalc-host windows rtd smoke");
    println!("runtime_platform={}", truth.runtime_platform);
    println!(
        "provider_state={}",
        truth
            .provider_truths
            .iter()
            .find(|provider| provider.provider_id == "demo.rtd.provider")
            .expect("rtd provider should be present")
            .capability_truths[0]
            .runtime_state
    );
    println!(
        "initial=provider:{};topic:{};state:{};value:{}",
        session.provider_id, session.topic_id, "active", "100.0"
    );
    println!(
        "update1=state:{};value:{};remaining={}",
        first.lifecycle_state, first.current_value, first.remaining_update_count
    );
    println!(
        "update2=state:{};value:{};remaining={}",
        second.lifecycle_state, second.current_value, second.remaining_update_count
    );
    println!(
        "update3=state:{};value:{};remaining={}",
        third.lifecycle_state, third.current_value, third.remaining_update_count
    );
}

fn run_linux_rtd_gate_smoke() {
    let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
    let root = env::temp_dir().join("dnaonecalc-linux-rtd-gate-smoke");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("demo-rtd")).expect("demo rtd dir should create");

    let rtd_manifest = dnaonecalc_host::ExtensionProviderManifest {
        provider_id: "demo.rtd.provider".to_string(),
        display_name: "Demo RTD Provider".to_string(),
        abi_version: "v1".to_string(),
        host_profile_ids: vec!["OC-H1".to_string()],
        platform_gate_ids: vec!["desktop_native_only".to_string()],
        declared_capabilities: vec!["rtd_provider".to_string()],
        entrypoint: "functions.json".to_string(),
    };

    std::fs::write(
        root.join("demo-rtd").join("provider.json"),
        serde_json::to_string_pretty(&rtd_manifest).expect("rtd manifest should serialize"),
    )
    .expect("rtd manifest should write");
    std::fs::write(
        root.join("demo-rtd").join("functions.json"),
        serde_json::to_string_pretty(&dnaonecalc_host::ExtensionProviderEntrypoint {
            registered_functions: Vec::new(),
            rtd_topics: vec![dnaonecalc_host::RegisteredRtdTopic {
                topic_id: "PRICE".to_string(),
                initial_value: "100.0".to_string(),
                updates: vec!["101.5".to_string()],
            }],
        })
        .expect("rtd entrypoint should serialize"),
    )
    .expect("rtd entrypoint should write");

    let summary = adapter
        .linux_rtd_registry_truth(&root)
        .expect("linux rtd registry truth should load");

    println!("dnaonecalc-host linux rtd gate smoke");
    println!("runtime_platform={}", summary.runtime_platform);
    println!("gate_state={}", summary.gate_state);
    println!("reason={}", summary.reason);
    for entry in &summary.entries {
        println!(
            "entry={} topics={}",
            entry.provider_id,
            entry.topic_ids.join(",")
        );
    }
}

fn run_scenario_index_smoke() {
    let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
    let root = env::temp_dir().join("dnaonecalc-scenario-index-smoke");
    let _ = std::fs::remove_dir_all(&root);
    let store = RetainedScenarioStore::new(&root);

    let mut host = adapter
        .new_driven_single_formula_host("onecalc.index", "=SUM(1,2,3)")
        .expect("OC-H1 should admit the driven host model");
    let recalc_context = RecalcContext::edit_accept(Some(46_000.0), Some(0.25));
    let recalc_summary = adapter
        .edit_accept_recalc(&mut host, "=SUM(1,2,3)", recalc_context.clone())
        .expect("edit-and-accept recalc should succeed");
    let persisted = adapter
        .persist_driven_scenario_run(
            &store,
            &host,
            &recalc_context,
            &recalc_summary,
            "index-smoke",
        )
        .expect("retained run should persist");
    let replay = adapter
        .emit_replay_capture_for_run(&store, &persisted.run.scenario_run_id)
        .expect("replay capture should persist");

    let index = adapter
        .build_promoted_scenario_index(&store)
        .expect("promoted scenario index should build");

    println!("dnaonecalc-host scenario index smoke");
    for row in &index.rows {
        println!(
            "row={} scenario={} run={} replay={} comparisons={} witnesses={} handoffs={}",
            row.row_id,
            row.scenario_id,
            row.latest_run_id,
            row.replay_capture_ids.join(","),
            row.comparison_ids.join(","),
            row.witness_ids.join(","),
            row.handoff_ids.join(",")
        );
    }
    println!("replay_capture_id={}", replay.capture.replay_capture_id);
}

fn run_scenario_library_filter_smoke() {
    let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
    let index = dnaonecalc_host::PromotedScenarioIndex {
        rows: vec![
            dnaonecalc_host::PromotedScenarioIndexRow {
                row_id: "promoted-scenario:one".to_string(),
                scenario_id: "scenario-one".to_string(),
                scenario_slug: "one".to_string(),
                latest_run_id: "run-one".to_string(),
                host_profile_id: "OC-H1".to_string(),
                runtime_platform: std::env::consts::OS.to_string(),
                formula_text: "=SUM(1,2,3)".to_string(),
                worksheet_value_summary: "Number(6)".to_string(),
                replay_capture_ids: vec!["replay-one".to_string()],
                comparison_ids: vec!["comparison-one".to_string()],
                witness_ids: vec!["witness-one".to_string()],
                handoff_ids: vec!["handoff-one".to_string()],
            },
            dnaonecalc_host::PromotedScenarioIndexRow {
                row_id: "promoted-scenario:two".to_string(),
                scenario_id: "scenario-two".to_string(),
                scenario_slug: "two".to_string(),
                latest_run_id: "run-two".to_string(),
                host_profile_id: "OC-H1".to_string(),
                runtime_platform: std::env::consts::OS.to_string(),
                formula_text: "=ABS(-3)".to_string(),
                worksheet_value_summary: "Number(3)".to_string(),
                replay_capture_ids: vec!["replay-two".to_string()],
                comparison_ids: Vec::new(),
                witness_ids: Vec::new(),
                handoff_ids: Vec::new(),
            },
        ],
    };
    let filter = dnaonecalc_host::ScenarioLibraryFilter {
        host_profile_ids: vec!["OC-H1".to_string()],
        runtime_platform: Some(std::env::consts::OS.to_string()),
        replay_required: Some(true),
        comparison_required: Some(true),
        witness_required: Some(true),
        handoff_required: Some(true),
    };
    let filtered = adapter.apply_scenario_library_filter(&index, &filter);
    let view_path = env::temp_dir().join("dnaonecalc-scenario-library-view-smoke.json");
    let view = dnaonecalc_host::ScenarioLibrarySavedView {
        view_id: "with-evidence".to_string(),
        display_name: "With Evidence".to_string(),
        filter,
    };
    adapter
        .save_scenario_library_view(&view_path, &view)
        .expect("saved view should persist");
    let reopened = adapter
        .read_scenario_library_view(&view_path)
        .expect("saved view should reopen");

    println!("dnaonecalc-host scenario library filter smoke");
    println!(
        "filtered_rows={}",
        filtered
            .iter()
            .map(|row| row.row_id.as_str())
            .collect::<Vec<_>>()
            .join(",")
    );
    println!("saved_view_id={}", reopened.view_id);
}

fn run_scenario_selection_smoke() {
    let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
    let row = dnaonecalc_host::PromotedScenarioIndexRow {
        row_id: "promoted-scenario:one".to_string(),
        scenario_id: "scenario-one".to_string(),
        scenario_slug: "one".to_string(),
        latest_run_id: "run-one".to_string(),
        host_profile_id: "OC-H1".to_string(),
        runtime_platform: std::env::consts::OS.to_string(),
        formula_text: "=SUM(1,2,3)".to_string(),
        worksheet_value_summary: "Number(6)".to_string(),
        replay_capture_ids: vec!["replay-one".to_string()],
        comparison_ids: vec!["comparison-one".to_string()],
        witness_ids: vec!["witness-one".to_string()],
        handoff_ids: vec!["handoff-one".to_string()],
    };
    let detail = adapter.build_scenario_selection_detail(&row);

    println!("dnaonecalc-host scenario selection smoke");
    println!("row_id={}", detail.row_id);
    println!(
        "lineage={}",
        detail
            .lineage
            .iter()
            .map(|item| format!("{}:{}", item.relation, item.artifact_id))
            .collect::<Vec<_>>()
            .join(",")
    );
    println!(
        "actions={}",
        detail
            .available_actions
            .iter()
            .map(|item| format!("{}:{}:{}", item.action_id, item.target_kind, item.target_id))
            .collect::<Vec<_>>()
            .join(",")
    );
}

fn run_acceptance_matrix_smoke() {
    let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
    let root = env::temp_dir().join("dnaonecalc-acceptance-matrix-smoke");
    let _ = std::fs::remove_dir_all(&root);
    let store = RetainedScenarioStore::new(&root);

    let mut host = adapter
        .new_driven_single_formula_host("onecalc.acceptance", "=SUM(1,2,3)")
        .expect("OC-H1 should admit the driven host model");
    let recalc_context = RecalcContext::edit_accept(Some(46_000.0), Some(0.25));
    let recalc_summary = adapter
        .edit_accept_recalc(&mut host, "=SUM(1,2,3)", recalc_context.clone())
        .expect("edit-and-accept recalc should succeed");
    let persisted = adapter
        .persist_driven_scenario_run(
            &store,
            &host,
            &recalc_context,
            &recalc_summary,
            "acceptance-smoke",
        )
        .expect("retained run should persist");
    adapter
        .emit_replay_capture_for_run(&store, &persisted.run.scenario_run_id)
        .expect("replay capture should persist");

    let matrix = adapter
        .build_acceptance_matrix(&store)
        .expect("acceptance matrix should build");

    println!("dnaonecalc-host acceptance matrix smoke");
    for row in &matrix.rows {
        println!(
            "row={} floor={} runtime={} replay={} compare={} witness={} handoff={}",
            row.row_id,
            row.capability_floor,
            row.runtime_class,
            row.replay_status,
            row.comparison_status,
            row.witness_status,
            row.handoff_status
        );
    }
}

fn run_scenario_regression_smoke() {
    let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
    let matrix = dnaonecalc_host::AcceptanceMatrix {
        rows: vec![dnaonecalc_host::AcceptanceMatrixRow {
            row_id: "promoted-scenario:one".to_string(),
            scenario_id: "scenario-one".to_string(),
            latest_run_id: "run-one".to_string(),
            capability_snapshot_id: "cap-one".to_string(),
            capability_floor: "OC-H1".to_string(),
            runtime_class: "desktop_native_only".to_string(),
            replay_status: "available".to_string(),
            comparison_status: "missing".to_string(),
            witness_status: "missing".to_string(),
            handoff_status: "missing".to_string(),
        }],
    };
    let summary = adapter.build_promoted_scenario_regression_summary(&matrix);
    let path = env::temp_dir().join("dnaonecalc-scenario-regression-smoke.json");
    adapter
        .save_promoted_scenario_regression_summary(&path, &summary)
        .expect("regression summary should persist");
    let reopened = adapter
        .read_promoted_scenario_regression_summary(&path)
        .expect("regression summary should reopen");

    println!("dnaonecalc-host scenario regression smoke");
    println!(
        "total={} replay={} compare={} witness={} handoff={}",
        reopened.total_rows,
        reopened.replay_ready_rows,
        reopened.compare_ready_rows,
        reopened.witness_ready_rows,
        reopened.handoff_ready_rows
    );
}

fn run_upstream_pressure_smoke() {
    let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
    let matrix = dnaonecalc_host::AcceptanceMatrix {
        rows: vec![dnaonecalc_host::AcceptanceMatrixRow {
            row_id: "promoted-scenario:one".to_string(),
            scenario_id: "scenario-one".to_string(),
            latest_run_id: "run-one".to_string(),
            capability_snapshot_id: "cap-one".to_string(),
            capability_floor: "OC-H1".to_string(),
            runtime_class: "desktop_native_only".to_string(),
            replay_status: "available".to_string(),
            comparison_status: "missing".to_string(),
            witness_status: "missing".to_string(),
            handoff_status: "missing".to_string(),
        }],
    };
    let packets = adapter.build_upstream_pressure_packets(&matrix);

    println!("dnaonecalc-host upstream pressure smoke");
    for packet in &packets {
        println!(
            "packet={} scenario={} lane={} blockers={}",
            packet.packet_id,
            packet.scenario_id,
            packet.target_lane,
            packet.blocker_ids.join("|")
        );
    }
}

fn run_h1_smoke() {
    let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
    let mut host = adapter
        .new_driven_single_formula_host("onecalc.h1", "=SUM(1,2,3)")
        .expect("OC-H1 should admit the driven host model");

    let edit = adapter
        .edit_accept_recalc(
            &mut host,
            "=SUM(1,2,3)",
            RecalcContext::edit_accept(Some(46_000.0), Some(0.25)),
        )
        .expect("edit-and-accept recalc should succeed");
    let manual = adapter
        .manual_recalc(&mut host, RecalcContext::manual(Some(46_000.0), Some(0.25)))
        .expect("manual recalc should succeed");
    let forced = adapter
        .forced_recalc(&mut host, RecalcContext::forced(Some(46_000.0), Some(0.25)))
        .expect("forced recalc should succeed");

    println!("dnaonecalc-host h1 smoke");
    println!("host_profile={}", edit.host_profile_id);
    println!(
        "edit_accept=trigger:{};packet_kind:{};formula_text_version:{};structure_context_version:{};worksheet_value:{}",
        edit.trigger_kind,
        edit.packet_kind,
        edit.formula_text_version,
        edit.structure_context_version,
        edit.evaluation.worksheet_value_summary
    );
    println!(
        "manual_recalc=trigger:{};packet_kind:{};formula_text_version:{};worksheet_value:{}",
        manual.trigger_kind,
        manual.packet_kind,
        manual.formula_text_version,
        manual.evaluation.worksheet_value_summary
    );
    println!(
        "forced_recalc=trigger:{};packet_kind:{};formula_text_version:{};worksheet_value:{}",
        forced.trigger_kind,
        forced.packet_kind,
        forced.formula_text_version,
        forced.evaluation.worksheet_value_summary
    );
}

fn run_h1_retained_smoke() {
    let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
    let mut host = adapter
        .new_driven_single_formula_host("onecalc.h1.retained", "=SUM(1,2,3)")
        .expect("OC-H1 should admit the driven host model");
    let edit_context = RecalcContext::edit_accept(Some(46_000.0), Some(0.25));
    let edit = adapter
        .edit_accept_recalc(&mut host, "=SUM(1,2,3)", edit_context.clone())
        .expect("edit-and-accept recalc should succeed");

    let root = env::temp_dir().join("dnaonecalc-h1-retained-smoke");
    let _ = std::fs::remove_dir_all(&root);
    let store = RetainedScenarioStore::new(&root);
    let persisted = adapter
        .persist_driven_scenario_run(&store, &host, &edit_context, &edit, "SUM retained smoke")
        .expect("retained scenario and run should persist");
    let mut reopened = adapter
        .reopen_driven_scenario_run(&store, &persisted.run.scenario_run_id)
        .expect("retained run should reopen");
    let reopened_summary = adapter
        .manual_recalc(
            &mut reopened.driven_host,
            RecalcContext::manual(Some(46_000.0), Some(0.25)),
        )
        .expect("reopened driven host should recalc");

    println!("dnaonecalc-host h1 retained smoke");
    println!("scenario_id={}", persisted.scenario.scenario_id);
    println!("scenario_run_id={}", persisted.run.scenario_run_id);
    println!("scenario_path={}", persisted.scenario_path.display());
    println!("run_path={}", persisted.run_path.display());
    println!(
        "reopened=host_profile:{};formula_text_version:{};worksheet_value:{}",
        reopened_summary.host_profile_id,
        reopened_summary.formula_text_version,
        reopened_summary.evaluation.worksheet_value_summary
    );
}

fn run_h1_compare_smoke() {
    let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
    let root = env::temp_dir().join("dnaonecalc-h1-compare-smoke");
    let _ = std::fs::remove_dir_all(&root);
    let store = RetainedScenarioStore::new(&root);
    let mut host = adapter
        .new_driven_single_formula_host("onecalc.h1.compare", "=SUM(1,2,3)")
        .expect("OC-H1 should admit the driven host model");

    let first_context = RecalcContext::edit_accept(Some(46_000.0), Some(0.25));
    let first_summary = adapter
        .edit_accept_recalc(&mut host, "=SUM(1,2,3)", first_context)
        .expect("first retained run should succeed");
    let first = adapter
        .persist_driven_scenario_run(&store, &host, &first_context, &first_summary, "SUM compare")
        .expect("first retained run should persist");

    let second_context = RecalcContext::edit_accept(Some(46_001.0), Some(0.25));
    let second_summary = adapter
        .edit_accept_recalc(&mut host, "=SUM(1,2,4)", second_context)
        .expect("second retained run should succeed");
    let second = adapter
        .persist_driven_scenario_run(
            &store,
            &host,
            &second_context,
            &second_summary,
            "SUM compare",
        )
        .expect("second retained run should persist");

    let comparison = adapter
        .compare_retained_driven_runs(
            &store,
            &first.run.scenario_run_id,
            &second.run.scenario_run_id,
        )
        .expect("retained driven runs should compare");

    println!("dnaonecalc-host h1 compare smoke");
    println!("left_run_id={}", comparison.left_run_id);
    println!("right_run_id={}", comparison.right_run_id);
    println!(
        "comparison=same_scenario:{};formula_version_changed:{};formula_text_changed:{};worksheet_value_match:{};reliability:{}",
        comparison.same_scenario,
        comparison.formula_version_changed,
        comparison.formula_text_changed,
        comparison.worksheet_value_match,
        comparison.reliability_badge
    );
}

fn run_replay_capture_smoke() {
    let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
    let root = env::temp_dir().join("dnaonecalc-replay-capture-smoke");
    let _ = std::fs::remove_dir_all(&root);
    let store = RetainedScenarioStore::new(&root);
    let mut host = adapter
        .new_driven_single_formula_host("onecalc.h1.replay", "=SUM(1,2,3)")
        .expect("OC-H1 should admit the driven host model");

    let edit_context = RecalcContext::edit_accept(Some(46_000.0), Some(0.25));
    let edit = adapter
        .edit_accept_recalc(&mut host, "=SUM(1,2,3)", edit_context)
        .expect("replay source recalc should succeed");
    let retained = adapter
        .persist_driven_scenario_run(&store, &host, &edit_context, &edit, "SUM replay")
        .expect("retained run should persist");
    let replay_capture = adapter
        .emit_replay_capture_for_run(&store, &retained.run.scenario_run_id)
        .expect("replay capture should emit");
    let opened = adapter
        .open_replay_capture(&store, &replay_capture.capture.replay_capture_id)
        .expect("replay capture should open");

    println!("dnaonecalc-host replay capture smoke");
    println!(
        "replay_capture_id={}",
        replay_capture.capture.replay_capture_id
    );
    println!("replay_path={}", replay_capture.replay_path.display());
    println!(
        "open=floor:{};ready:{};events:{};registry_refs:{};view_family:{};projection_family:{};projection_phase:{};projection_alias:{}",
        opened.replay_floor,
        opened.replay_ready,
        opened.event_count,
        opened.registry_ref_count,
        opened.view_family,
        opened.projection_source_artifact_family,
        opened.projection_phase.as_deref().unwrap_or("none"),
        opened.projection_alias.as_deref().unwrap_or("none")
    );
}

fn run_xray_diff_smoke() {
    let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
    let root = env::temp_dir().join("dnaonecalc-xray-diff-smoke");
    let _ = std::fs::remove_dir_all(&root);
    let store = RetainedScenarioStore::new(&root);
    let mut host = adapter
        .new_driven_single_formula_host("onecalc.h1.xray", "=SUM(1,2,3)")
        .expect("OC-H1 should admit the driven host model");

    let first_context = RecalcContext::edit_accept(Some(46_000.0), Some(0.25));
    let first_summary = adapter
        .edit_accept_recalc(&mut host, "=SUM(1,2,3)", first_context)
        .expect("first xray recalc should succeed");
    let first = adapter
        .persist_driven_scenario_run(&store, &host, &first_context, &first_summary, "SUM xray")
        .expect("first retained run should persist");
    let first_replay = adapter
        .emit_replay_capture_for_run(&store, &first.run.scenario_run_id)
        .expect("first replay capture should emit");

    let second_context = RecalcContext::edit_accept(Some(46_001.0), Some(0.25));
    let second_summary = adapter
        .edit_accept_recalc(&mut host, "=SUM(1,2,4)", second_context)
        .expect("second xray recalc should succeed");
    let second = adapter
        .persist_driven_scenario_run(&store, &host, &second_context, &second_summary, "SUM xray")
        .expect("second retained run should persist");
    let second_replay = adapter
        .emit_replay_capture_for_run(&store, &second.run.scenario_run_id)
        .expect("second replay capture should emit");

    let xray = adapter
        .open_retained_run_xray(&store, &first.run.scenario_run_id)
        .expect("retained X-Ray should open");
    let diff = adapter
        .diff_retained_run_xray(
            &store,
            &first.run.scenario_run_id,
            &second.run.scenario_run_id,
        )
        .expect("retained diff should open");

    println!("dnaonecalc-host xray diff smoke");
    println!(
        "xray=run:{};worksheet_value:{};capability_snapshot:{};formatting_truth:{};conditional_formatting_scope:{};blocked:{};replay_capture:{};replay_floor:{};replay_projection_family:{};replay_projection_phase:{};replay_projection_alias:{}",
        xray.provenance.scenario_run_id.as_deref().unwrap_or("none"),
        xray.evaluation.as_ref().map(|value| value.worksheet_value_summary.as_str()).unwrap_or("none"),
        xray.provenance.latest_capability_snapshot_id,
        xray.provenance.formatting_truth_plane,
        xray.provenance.conditional_formatting_scope.replace(": ", "=").replace(" ", "_"),
        xray.provenance.blocked_dimensions.join(","),
        xray.trace.as_ref().and_then(|value| value.replay_capture_id.as_deref()).unwrap_or("none"),
        xray.trace.as_ref().and_then(|value| value.replay_floor.as_deref()).unwrap_or("none"),
        xray.trace.as_ref().and_then(|value| value.replay_projection_source_artifact_family.as_deref()).unwrap_or("none"),
        xray.trace.as_ref().and_then(|value| value.replay_projection_phase.as_deref()).unwrap_or("none"),
        xray.trace.as_ref().and_then(|value| value.replay_projection_alias.as_deref()).unwrap_or("none")
    );
    println!(
        "diff=left:{};right:{};formula_text_changed:{};worksheet_value_match:{};capability_snapshot_changed:{};replay_pair_openable:{};formatting_truth:{};conditional_formatting_scope:{};blocked:{};floor:{}",
        diff.left_run_id,
        diff.right_run_id,
        diff.formula_text_changed,
        diff.worksheet_value_match,
        diff.capability_snapshot_changed,
        diff.replay_pair_openable,
        diff.formatting_truth_plane,
        diff.conditional_formatting_scope.replace(": ", "=").replace(" ", "_"),
        diff.blocked_dimensions.join(","),
        diff.diff_floor
    );
    println!(
        "replay_ids={},{}",
        first_replay.capture.replay_capture_id, second_replay.capture.replay_capture_id
    );
}

fn run_witness_smoke() {
    let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
    let root = env::temp_dir().join("dnaonecalc-witness-smoke");
    let _ = std::fs::remove_dir_all(&root);
    let store = RetainedScenarioStore::new(&root);
    let mut host = adapter
        .new_driven_single_formula_host("onecalc.h1.witness", "=SUM(1,2,3)")
        .expect("OC-H1 should admit the driven host model");

    let left_context = RecalcContext::edit_accept(Some(46_000.0), Some(0.25));
    let left_summary = adapter
        .edit_accept_recalc(&mut host, "=SUM(1,2,3)", left_context)
        .expect("left witness recalc should succeed");
    let left = adapter
        .persist_driven_scenario_run(&store, &host, &left_context, &left_summary, "SUM witness")
        .expect("left retained run should persist");
    adapter
        .emit_replay_capture_for_run(&store, &left.run.scenario_run_id)
        .expect("left replay capture should emit");

    let right_context = RecalcContext::edit_accept(Some(46_001.0), Some(0.25));
    let right_summary = adapter
        .edit_accept_recalc(&mut host, "=SUM(1,2,4)", right_context)
        .expect("right witness recalc should succeed");
    let right = adapter
        .persist_driven_scenario_run(&store, &host, &right_context, &right_summary, "SUM witness")
        .expect("right retained run should persist");
    adapter
        .emit_replay_capture_for_run(&store, &right.run.scenario_run_id)
        .expect("right replay capture should emit");

    let persisted = adapter
        .generate_retained_witness(
            &store,
            &left.run.scenario_run_id,
            &right.run.scenario_run_id,
        )
        .expect("witness should generate");
    let opened = adapter
        .open_witness(&store, &persisted.witness.witness_id)
        .expect("witness should open");

    println!("dnaonecalc-host witness smoke");
    println!("witness_id={}", opened.witness_id);
    println!("scenario_id={}", opened.scenario_id);
    println!("explain_floor={}", opened.explain_floor);
    println!("lines={}", opened.explanation_lines.join(" | "));
    println!("blocked={}", opened.blocked_dimensions.join(","));
    println!(
        "replay_projection_aliases={}",
        opened.replay_projection_aliases.join(",")
    );
}

fn run_handoff_smoke() {
    let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
    let root = env::temp_dir().join("dnaonecalc-handoff-smoke");
    let _ = std::fs::remove_dir_all(&root);
    let store = RetainedScenarioStore::new(&root);
    let mut host = adapter
        .new_driven_single_formula_host("onecalc.h1.handoff", "=SUM(1,2,3)")
        .expect("OC-H1 should admit the driven host model");

    let left_context = RecalcContext::edit_accept(Some(46_000.0), Some(0.25));
    let left_summary = adapter
        .edit_accept_recalc(&mut host, "=SUM(1,2,3)", left_context)
        .expect("left handoff recalc should succeed");
    let left = adapter
        .persist_driven_scenario_run(&store, &host, &left_context, &left_summary, "SUM handoff")
        .expect("left retained run should persist");
    adapter
        .emit_replay_capture_for_run(&store, &left.run.scenario_run_id)
        .expect("left replay capture should emit");

    let right_context = RecalcContext::edit_accept(Some(46_001.0), Some(0.25));
    let right_summary = adapter
        .edit_accept_recalc(&mut host, "=SUM(1,2,4)", right_context)
        .expect("right handoff recalc should succeed");
    let right = adapter
        .persist_driven_scenario_run(&store, &host, &right_context, &right_summary, "SUM handoff")
        .expect("right retained run should persist");
    adapter
        .emit_replay_capture_for_run(&store, &right.run.scenario_run_id)
        .expect("right replay capture should emit");

    let witness = adapter
        .generate_retained_witness(
            &store,
            &left.run.scenario_run_id,
            &right.run.scenario_run_id,
        )
        .expect("witness should generate");
    let handoff = adapter
        .generate_handoff_packet(&store, &witness.witness.witness_id)
        .expect("handoff should generate");
    let opened = adapter
        .open_handoff_packet(&store, &handoff.handoff.handoff_id)
        .expect("handoff should open");

    println!("dnaonecalc-host handoff smoke");
    println!("handoff_id={}", opened.handoff_id);
    println!("target_lane={}", opened.target_lane);
    println!("requested_action_kind={}", opened.requested_action_kind);
    println!("status={}", opened.status);
    println!("capability_snapshot_id={}", opened.capability_snapshot_id);
    println!(
        "replay_projection_alias={}",
        opened.replay_projection_alias.as_deref().unwrap_or("none")
    );
    println!(
        "replay_projection_phase={}",
        opened.replay_projection_phase.as_deref().unwrap_or("none")
    );
    println!(
        "readiness={}",
        opened
            .readiness
            .iter()
            .map(|item| format!("{}:{}", item.item_id, item.satisfied))
            .collect::<Vec<_>>()
            .join(",")
    );
}

fn run_document_roundtrip_smoke() {
    let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
    let root = env::temp_dir().join("dnaonecalc-document-roundtrip-smoke");
    let _ = std::fs::remove_dir_all(&root);
    let store = RetainedScenarioStore::new(root.join("retained"));
    let document_path = root.join("sum-document.xml");
    let mut host = adapter
        .new_driven_single_formula_host("onecalc.h1.document", "=SUM(1,2,3)")
        .expect("OC-H1 should admit the driven host model");

    let edit_context = RecalcContext::edit_accept(Some(46_000.0), Some(0.25));
    let edit = adapter
        .edit_accept_recalc(&mut host, "=SUM(1,2,3)", edit_context)
        .expect("document recalc should succeed");
    let retained = adapter
        .persist_driven_scenario_run(&store, &host, &edit_context, &edit, "SUM document")
        .expect("retained run should persist");
    let persisted_document = adapter
        .persist_isolated_document(
            &document_path,
            &host,
            &edit_context,
            &edit,
            "SUM document",
            Some(&retained),
        )
        .expect("isolated document should persist");
    let invariants = adapter
        .verify_isolated_document_roundtrip_invariants(&persisted_document)
        .expect("document invariants should survive round-trip");
    let mut reopened = adapter
        .reopen_isolated_document(&persisted_document.document_path)
        .expect("isolated document should reopen");
    let reopened_summary = adapter
        .manual_recalc(
            &mut reopened.driven_host,
            RecalcContext::manual(Some(46_000.0), Some(0.25)),
        )
        .expect("reopened document should recalc");

    println!("dnaonecalc-host document roundtrip smoke");
    println!("document_id={}", reopened.document.document_id);
    println!(
        "document_path={}",
        persisted_document.document_path.display()
    );
    println!(
        "document_scope={};format={};artifacts={}",
        reopened.document.document_scope,
        reopened.document.persistence_format_id,
        reopened.document.artifact_index.len()
    );
    println!(
        "invariants=document_id:{};formula_identity:{};structure_context:{};library_context_snapshot_ref:{};artifact_index:{};effective_display_status:{}",
        invariants.document_id_preserved,
        invariants.formula_identity_preserved,
        invariants.structure_context_preserved,
        invariants.library_context_snapshot_ref_preserved,
        invariants.artifact_index_preserved,
        invariants.effective_display_status_preserved
    );
    println!(
        "reopened=host_profile:{};formula_text_version:{};worksheet_value:{}",
        reopened_summary.host_profile_id,
        reopened_summary.formula_text_version,
        reopened_summary.evaluation.worksheet_value_summary
    );
}

fn run_scenario_capsule_smoke() {
    let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
    let root = env::temp_dir().join("dnaonecalc-scenario-capsule-smoke");
    let _ = std::fs::remove_dir_all(&root);
    let export_store = RetainedScenarioStore::new(root.join("retained-export"));
    let import_store = RetainedScenarioStore::new(root.join("retained-import"));
    let capsule_root = root.join("capsule");
    let mut host = adapter
        .new_driven_single_formula_host("onecalc.h1.capsule", "=SUM(1,2,3)")
        .expect("OC-H1 should admit the driven host model");

    let edit_context = RecalcContext::edit_accept(Some(46_000.0), Some(0.25));
    let edit = adapter
        .edit_accept_recalc(&mut host, "=SUM(1,2,3)", edit_context)
        .expect("capsule source recalc should succeed");
    let retained = adapter
        .persist_driven_scenario_run(&export_store, &host, &edit_context, &edit, "SUM capsule")
        .expect("retained run should persist");
    let exported = adapter
        .export_scenario_capsule(
            &export_store,
            &capsule_root,
            &[&retained.run.scenario_run_id],
        )
        .expect("ScenarioCapsule export should succeed");
    let imported = adapter
        .import_scenario_capsule(&import_store, &exported.capsule_root)
        .expect("ScenarioCapsule intake should succeed");

    println!("dnaonecalc-host scenario capsule smoke");
    println!("capsule_id={}", exported.manifest.capsule_id);
    println!("manifest_path={}", exported.manifest_path.display());
    println!(
        "inventory=scenario:{};runs:{};capabilities:{}",
        exported
            .manifest
            .included_artifacts
            .iter()
            .filter(|entry| entry.artifact_kind == "scenario")
            .count(),
        exported
            .manifest
            .included_artifacts
            .iter()
            .filter(|entry| entry.artifact_kind == "scenario_run")
            .count(),
        exported.manifest.capability_snapshot_refs.len()
    );
    println!(
        "intake=imported:{};deduped:{};conflicts:{};manifest_copy:{}",
        imported.imported_paths.len(),
        imported.deduped_paths.len(),
        imported.conflict_paths.len(),
        imported.manifest_copy_path.display()
    );
}

fn run_workspace_smoke() {
    let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
    let root = env::temp_dir().join("dnaonecalc-workspace-smoke");
    let _ = std::fs::remove_dir_all(&root);
    let store = RetainedScenarioStore::new(root.join("retained"));
    let workspace_path = root.join("workspace.onecalc.json");

    let mut first_host = adapter
        .new_driven_single_formula_host("onecalc.h1.workspace.left", "=SUM(1,2,3)")
        .expect("first workspace host should initialize");
    let first_context = RecalcContext::edit_accept(Some(46_000.0), Some(0.25));
    let first_summary = adapter
        .edit_accept_recalc(&mut first_host, "=SUM(1,2,3)", first_context)
        .expect("first workspace recalc should succeed");
    let first_retained = adapter
        .persist_driven_scenario_run(
            &store,
            &first_host,
            &first_context,
            &first_summary,
            "SUM workspace left",
        )
        .expect("first retained run should persist");
    let first_document = adapter
        .persist_isolated_document(
            root.join("left.xml"),
            &first_host,
            &first_context,
            &first_summary,
            "SUM workspace left",
            Some(&first_retained),
        )
        .expect("first document should persist");

    let mut second_host = adapter
        .new_driven_single_formula_host("onecalc.h1.workspace.right", "=SUM(1,2,4)")
        .expect("second workspace host should initialize");
    let second_context = RecalcContext::edit_accept(Some(46_001.0), Some(0.25));
    let second_summary = adapter
        .edit_accept_recalc(&mut second_host, "=SUM(1,2,4)", second_context)
        .expect("second workspace recalc should succeed");
    let second_retained = adapter
        .persist_driven_scenario_run(
            &store,
            &second_host,
            &second_context,
            &second_summary,
            "SUM workspace right",
        )
        .expect("second retained run should persist");
    let second_document = adapter
        .persist_isolated_document(
            root.join("right.xml"),
            &second_host,
            &second_context,
            &second_summary,
            "SUM workspace right",
            Some(&second_retained),
        )
        .expect("second document should persist");

    let persisted = adapter
        .persist_workspace_manifest(
            &workspace_path,
            "OneCalc Workspace Smoke",
            &[
                &first_document.document_path,
                &second_document.document_path,
            ],
        )
        .expect("workspace manifest should persist");
    let opened = adapter
        .open_workspace(&persisted.manifest_path)
        .expect("workspace should reopen");

    println!("dnaonecalc-host workspace smoke");
    println!("workspace_id={}", opened.manifest.workspace_id);
    println!("workspace_path={}", persisted.manifest_path.display());
    println!(
        "documents=active:{};count:{};ids:{}",
        opened.manifest.active_document_id,
        opened.reopened_documents.len(),
        opened
            .reopened_documents
            .iter()
            .map(|document| document.document.document_id.clone())
            .collect::<Vec<_>>()
            .join(",")
    );
}

fn run_windows_observation_smoke() {
    let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
    let root = env::temp_dir().join("dnaonecalc-windows-observation-smoke");
    let _ = std::fs::remove_dir_all(&root);
    let store = RetainedScenarioStore::new(root.join("retained"));
    let output_root = root.join("source-bundle");
    let persisted = adapter
        .capture_windows_observation(&store, &output_root)
        .expect("windows observation capture should succeed");

    println!("dnaonecalc-host windows observation smoke");
    println!("observation_id={}", persisted.observation.observation_id);
    println!("scenario_id={}", persisted.observation.scenario_id);
    println!(
        "capture_mode={};projection_status={};platform_scope={}",
        persisted.observation.capture_mode,
        persisted.observation.projection_status,
        persisted.observation.platform_scope
    );
    println!(
        "surfaces={}",
        persisted
            .observation
            .capture
            .surfaces
            .iter()
            .map(|surface| format!(
                "{}:{}:{}:{}",
                surface.surface.surface_id,
                surface.surface.surface_kind,
                surface.status,
                surface.capture_loss
            ))
            .collect::<Vec<_>>()
            .join(",")
    );
    println!("lossiness={}", persisted.observation.lossiness.join(","));
}

fn run_twin_compare_smoke() {
    let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
    let root = env::temp_dir().join("dnaonecalc-twin-compare-smoke");
    let _ = std::fs::remove_dir_all(&root);
    let store = RetainedScenarioStore::new(root.join("retained"));
    let output_root = root.join("source-bundle");
    let mut host = adapter
        .new_driven_single_formula_host("onecalc.h1.twin-compare", "=SUM(10,20,12)")
        .expect("compare host should initialize");
    let context = RecalcContext::edit_accept(Some(46_000.0), Some(0.25));
    let summary = adapter
        .edit_accept_recalc(&mut host, "=SUM(10,20,12)", context)
        .expect("compare recalc should succeed");
    let retained = adapter
        .persist_driven_scenario_run(&store, &host, &context, &summary, "Twin compare")
        .expect("retained run should persist");
    let observation = adapter
        .capture_windows_observation(&store, &output_root)
        .expect("windows observation should capture");
    let comparison = adapter
        .compare_run_with_observation(
            &store,
            &retained.run.scenario_run_id,
            &observation.observation.observation_id,
        )
        .expect("comparison should persist");
    let opened = adapter
        .open_twin_compare(&store, &comparison.comparison.comparison_id)
        .expect("twin compare should open");

    println!("dnaonecalc-host twin compare smoke");
    println!("comparison_id={}", opened.comparison_id);
    println!("left_run_id={}", opened.left_run_id);
    println!("observation_id={}", opened.observation_id);
    println!(
        "envelope={};reliability={}",
        opened.comparison_envelope.join(","),
        opened.reliability_badge
    );
    println!("mismatches={}", opened.mismatch_lines.join("|"));
    println!(
        "projection_limitations={}",
        opened.projection_limitations.join(",")
    );
}

fn run_widening_request_smoke() {
    let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
    let root = env::temp_dir().join("dnaonecalc-widening-request-smoke");
    let _ = std::fs::remove_dir_all(&root);
    let store = RetainedScenarioStore::new(root.join("retained"));
    let output_root = root.join("source-bundle");
    let mut host = adapter
        .new_driven_single_formula_host("onecalc.h1.widening", "=SUM(10,20,12)")
        .expect("widening host should initialize");
    let context = RecalcContext::edit_accept(Some(46_000.0), Some(0.25));
    let summary = adapter
        .edit_accept_recalc(&mut host, "=SUM(10,20,12)", context)
        .expect("widening recalc should succeed");
    let retained = adapter
        .persist_driven_scenario_run(&store, &host, &context, &summary, "Widening request")
        .expect("retained run should persist");
    let observation = adapter
        .capture_windows_observation(&store, &output_root)
        .expect("windows observation should capture");
    let comparison = adapter
        .compare_run_with_observation(
            &store,
            &retained.run.scenario_run_id,
            &observation.observation.observation_id,
        )
        .expect("comparison should persist");
    let handoff = adapter
        .generate_observation_widening_handoff(&store, &comparison.comparison.comparison_id)
        .expect("widening handoff should persist");
    let opened = adapter
        .open_handoff_packet(&store, &handoff.handoff.handoff_id)
        .expect("handoff should open");

    println!("dnaonecalc-host widening request smoke");
    println!("handoff_id={}", opened.handoff_id);
    println!("target_lane={}", opened.target_lane);
    println!("requested_action_kind={}", opened.requested_action_kind);
    println!("status={}", opened.status);
    println!("capability_snapshot_id={}", opened.capability_snapshot_id);
}

fn run_shell(smoke_mode: bool) {
    if let Err(error) = launch_shell(smoke_mode) {
        eprintln!("failed to launch dnaonecalc shell: {error}");
        std::process::exit(1);
    }
}

fn run_editor_diagnostic_smoke() {
    if let Err(error) = launch_shell_with_formula("=SUM(1,", true) {
        eprintln!("failed to launch dnaonecalc editor diagnostic smoke: {error}");
        std::process::exit(1);
    }
}
