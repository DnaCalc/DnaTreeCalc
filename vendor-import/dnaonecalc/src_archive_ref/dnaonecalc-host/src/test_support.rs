use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::{
    DrivenRecalcSummary, DrivenSingleFormulaHost, OneCalcHostProfile, PersistedObservation,
    PersistedReplayCapture, PersistedScenarioRun, RecalcContext, RetainedScenarioStore,
    RuntimeAdapter,
};

static FIXTURE_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug)]
pub(crate) struct FixtureRoot {
    root: PathBuf,
}

impl FixtureRoot {
    pub(crate) fn new(label: &str) -> Self {
        let fixture_id = FIXTURE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = env::temp_dir().join(format!(
            "dnaonecalc-fixture-{label}-{}-{fixture_id}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);

        Self { root }
    }

    pub(crate) fn join(&self, child: impl AsRef<Path>) -> PathBuf {
        self.root.join(child)
    }

    pub(crate) fn retained_store(&self) -> RetainedScenarioStore {
        RetainedScenarioStore::new(self.join("retained"))
    }
}

impl Drop for FixtureRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FormulaScenarioFamily {
    ExplorerSum,
    ExplorerInvalid,
    ExplorerCompletionStem,
    ExplorerSequence23,
    RetainedSumBaseline,
    RetainedSumShifted,
    ObservationCompareSum,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PromotedFormulaScenario {
    pub case_id: &'static str,
    pub family: FormulaScenarioFamily,
    pub host_profile: OneCalcHostProfile,
    pub plane_tags: &'static [&'static str],
    pub formula: &'static str,
    pub stable_id: &'static str,
    pub scenario_slug: &'static str,
    pub expected_value_summary: &'static str,
}

pub(crate) const PROMOTED_FORMULA_SCENARIOS: [PromotedFormulaScenario; 7] = [
    PromotedFormulaScenario {
        case_id: "explorer-sum",
        family: FormulaScenarioFamily::ExplorerSum,
        host_profile: OneCalcHostProfile::OcH0,
        plane_tags: &["explorer", "xray"],
        formula: "=SUM(1,2,3)",
        stable_id: "fixture.oc-h0.explorer.sum",
        scenario_slug: "Explorer SUM",
        expected_value_summary: "Number(6)",
    },
    PromotedFormulaScenario {
        case_id: "explorer-invalid",
        family: FormulaScenarioFamily::ExplorerInvalid,
        host_profile: OneCalcHostProfile::OcH0,
        plane_tags: &["diagnostics", "xray"],
        formula: "=SUM(1,",
        stable_id: "fixture.oc-h0.explorer.invalid",
        scenario_slug: "Explorer Invalid SUM",
        expected_value_summary: "invalid",
    },
    PromotedFormulaScenario {
        case_id: "explorer-completion-stem",
        family: FormulaScenarioFamily::ExplorerCompletionStem,
        host_profile: OneCalcHostProfile::OcH0,
        plane_tags: &["explorer"],
        formula: "=SU",
        stable_id: "fixture.oc-h0.explorer.completion",
        scenario_slug: "Explorer Completion Stem",
        expected_value_summary: "pending",
    },
    PromotedFormulaScenario {
        case_id: "explorer-sequence-2x3",
        family: FormulaScenarioFamily::ExplorerSequence23,
        host_profile: OneCalcHostProfile::OcH0,
        plane_tags: &["explorer", "result_surface"],
        formula: "=SEQUENCE(2,3)",
        stable_id: "fixture.oc-h0.explorer.sequence",
        scenario_slug: "Explorer Sequence",
        expected_value_summary: "Array(2x3)",
    },
    PromotedFormulaScenario {
        case_id: "retained-sum-baseline",
        family: FormulaScenarioFamily::RetainedSumBaseline,
        host_profile: OneCalcHostProfile::OcH1,
        plane_tags: &["replay", "xray", "retained"],
        formula: "=SUM(1,2,3)",
        stable_id: "fixture.oc-h1.retained.sum-family",
        scenario_slug: "SUM baseline",
        expected_value_summary: "Number(6)",
    },
    PromotedFormulaScenario {
        case_id: "retained-sum-shifted",
        family: FormulaScenarioFamily::RetainedSumShifted,
        host_profile: OneCalcHostProfile::OcH1,
        plane_tags: &["replay", "xray", "retained"],
        formula: "=SUM(1,2,4)",
        stable_id: "fixture.oc-h1.retained.sum-family",
        scenario_slug: "SUM baseline",
        expected_value_summary: "Number(7)",
    },
    PromotedFormulaScenario {
        case_id: "observation-compare-sum",
        family: FormulaScenarioFamily::ObservationCompareSum,
        host_profile: OneCalcHostProfile::OcH1,
        plane_tags: &["observation", "compare"],
        formula: "=SUM(10,20,12)",
        stable_id: "fixture.oc-h1.observation.compare",
        scenario_slug: "Twin compare family",
        expected_value_summary: "Number(42)",
    },
];

pub(crate) const fn promoted_formula_scenarios() -> &'static [PromotedFormulaScenario] {
    &PROMOTED_FORMULA_SCENARIOS
}

pub(crate) fn promoted_formula_scenario(
    family: FormulaScenarioFamily,
) -> &'static PromotedFormulaScenario {
    PROMOTED_FORMULA_SCENARIOS
        .iter()
        .find(|scenario| scenario.family == family)
        .expect("promoted formula scenario should exist")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FormattingScenarioFamily {
    CurrencyHintWithHostAccent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PromotedFormattingScenario {
    pub case_id: &'static str,
    pub family: FormattingScenarioFamily,
    pub plane_tags: &'static [&'static str],
    pub worksheet_value_summary: &'static str,
    pub returned_presentation_hint_status: &'static str,
    pub host_style_state_status: &'static str,
    pub effective_display_status: &'static str,
    pub expected_display_text: &'static str,
}

pub(crate) const PROMOTED_FORMATTING_SCENARIOS: [PromotedFormattingScenario; 1] =
    [PromotedFormattingScenario {
        case_id: "formatting-currency-host-accent",
        family: FormattingScenarioFamily::CurrencyHintWithHostAccent,
        plane_tags: &["formatting", "effective_display"],
        worksheet_value_summary: "Number(6)",
        returned_presentation_hint_status: "number_format:none;style:Currency",
        host_style_state_status: "accent",
        effective_display_status:
            "presentation_hint:number_format:none;style:Currency;host_style:accent",
        expected_display_text: "6",
    }];

pub(crate) const fn promoted_formatting_scenarios() -> &'static [PromotedFormattingScenario] {
    &PROMOTED_FORMATTING_SCENARIOS
}

pub(crate) fn promoted_formatting_scenario(
    family: FormattingScenarioFamily,
) -> &'static PromotedFormattingScenario {
    PROMOTED_FORMATTING_SCENARIOS
        .iter()
        .find(|scenario| scenario.family == family)
        .expect("promoted formatting scenario should exist")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ObservationScenarioFamily {
    XlPlayCaptureValuesFormulae001,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PromotedObservationScenario {
    pub case_id: &'static str,
    pub family: ObservationScenarioFamily,
    pub plane_tags: &'static [&'static str],
    pub source_relpath: &'static str,
    pub expected_scenario_id: &'static str,
    pub expected_value_surface_id: &'static str,
}

pub(crate) const PROMOTED_OBSERVATION_SCENARIOS: [PromotedObservationScenario; 1] =
    [PromotedObservationScenario {
        case_id: "xlplay-capture-values-formulae-001",
        family: ObservationScenarioFamily::XlPlayCaptureValuesFormulae001,
        plane_tags: &["observation", "compare"],
        source_relpath: "states/excel/xlplay_capture_values_formulae_001",
        expected_scenario_id: "xlplay_capture_values_formulae_001",
        expected_value_surface_id: "sheet1_a1_value",
    }];

pub(crate) const fn promoted_observation_scenarios() -> &'static [PromotedObservationScenario] {
    &PROMOTED_OBSERVATION_SCENARIOS
}

pub(crate) fn promoted_observation_scenario(
    family: ObservationScenarioFamily,
) -> &'static PromotedObservationScenario {
    PROMOTED_OBSERVATION_SCENARIOS
        .iter()
        .find(|scenario| scenario.family == family)
        .expect("promoted observation scenario should exist")
}

fn observation_source_root(scenario: &PromotedObservationScenario) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
        .join("OxXlPlay")
        .join(scenario.source_relpath)
}

pub(crate) fn adapter_for(profile: OneCalcHostProfile) -> RuntimeAdapter {
    RuntimeAdapter::new(profile)
}

#[derive(Debug)]
pub(crate) struct DrivenHostFixture {
    pub adapter: RuntimeAdapter,
    pub host: DrivenSingleFormulaHost,
}

impl DrivenHostFixture {
    pub(crate) fn new(profile: OneCalcHostProfile, scenario: FormulaScenarioFamily) -> Self {
        let adapter = adapter_for(profile);
        let scenario_case = promoted_formula_scenario(scenario);
        let host = adapter
            .new_driven_single_formula_host(scenario_case.stable_id, scenario_case.formula)
            .expect("fixture host should initialize");

        Self { adapter, host }
    }

    pub(crate) fn edit_accept(
        &mut self,
        scenario: FormulaScenarioFamily,
        now_serial: f64,
        random_value: f64,
    ) -> (RecalcContext, DrivenRecalcSummary) {
        let scenario_case = promoted_formula_scenario(scenario);
        let context = RecalcContext::edit_accept(Some(now_serial), Some(random_value));
        let summary = self
            .adapter
            .edit_accept_recalc(&mut self.host, scenario_case.formula, context)
            .expect("fixture edit-accept recalc should succeed");

        (context, summary)
    }

    pub(crate) fn persist_run(
        &self,
        store: &RetainedScenarioStore,
        context: &RecalcContext,
        summary: &DrivenRecalcSummary,
        scenario: FormulaScenarioFamily,
    ) -> PersistedScenarioRun {
        let scenario_case = promoted_formula_scenario(scenario);
        self.adapter
            .persist_driven_scenario_run(
                store,
                &self.host,
                context,
                summary,
                scenario_case.scenario_slug,
            )
            .expect("fixture retained run should persist")
    }

    pub(crate) fn emit_replay_capture(
        &self,
        store: &RetainedScenarioStore,
        retained_run: &PersistedScenarioRun,
    ) -> PersistedReplayCapture {
        self.adapter
            .emit_replay_capture_for_run(store, &retained_run.run.scenario_run_id)
            .expect("fixture replay capture should emit")
    }
}

pub(crate) fn persist_observation_fixture(
    adapter: &RuntimeAdapter,
    store: &RetainedScenarioStore,
    scenario: ObservationScenarioFamily,
) -> PersistedObservation {
    let source_root = observation_source_root(promoted_observation_scenario(scenario));
    adapter
        .persist_observation_from_existing_source(store, &source_root)
        .expect("fixture observation should persist")
}
