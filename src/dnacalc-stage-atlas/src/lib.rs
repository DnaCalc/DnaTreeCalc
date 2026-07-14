//! TIER: TP (Presentation) — T0 skin-ir + Leptos + TP crates only; no Ox* ever (P-gate).
//!
//! The Atlas stage (S2.15): a [`dnacalc_shell::StageSurface`] registered under
//! [`dnacalc_shell::StageId::Atlas`]. `mount` renders two reactive panels, both
//! closures over `ctx.workspace` that re-derive on every workspace change:
//!
//!  - a **structure map** ([`render_structure_map`]) — the workbook's sheets
//!    (`WorkspaceState::sheets`) and its defined-name catalog
//!    (`WorkspaceState::defined_names`), reduced to display labels by
//!    [`derive_structure_map`], and
//!  - a **calc HUD** ([`render_calc_hud`]) — `WorkspaceState::workbook_calc`
//!    (mode, last-recalc tick, per-sheet dirty summary) plus, when a run has
//!    ever completed, `WorkspaceState::last_run`'s outcome, reduced by
//!    [`derive_calc_hud`].
//!
//! Both derivations are pure functions of [`WorkspaceState`] — no engine truth
//! is re-derived and nothing here classifies `=`-vs-literal (SHELL_SPEC §6
//! layering law). The Atlas stage is read-only: it never dispatches an
//! intent, so `ctx.dispatch` is not even touched by `mount`.
//!
//! HONEST-EMPTY / no shims: an empty sheet list, an empty defined-name
//! catalog, and a `None` `workbook_calc` each render an explicit, testable
//! note — never a placeholder row and never a fabricated calc number. The
//! stage is never rendered blank: even an all-empty workspace still shows the
//! structure-map frame (with its two honest-empty notes) and the HUD's own
//! honest-empty note.

use leptos::prelude::*;

use dnacalc_shell::{ProfileTag, StageContext, StageHandle, StageId, StageSurface};
use dnacalc_skin_ir::{
    CalcModeProjection, CalcRunProjection, CalcRunStateProjection, DefinedNamesProjection,
    SheetCalcSummaryProjection, SheetProjection, WorkbookCalcProjection, WorkspaceState,
};

/// The Atlas stage surface.
#[derive(Debug, Clone, Copy, Default)]
pub struct AtlasStage;

impl AtlasStage {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl StageSurface for AtlasStage {
    fn id(&self) -> StageId {
        StageId::Atlas
    }

    fn title(&self) -> &'static str {
        "Atlas"
    }

    fn supports(&self, profile: &ProfileTag) -> bool {
        matches!(profile, ProfileTag::ExcelStrict)
    }

    fn mount(&self, ctx: StageContext) -> StageHandle {
        // The one host-truth read this stage makes. `ReadSignal` is `Copy`, so
        // both closures below can re-run their own derivation on every
        // workspace change without holding any owned state — the structure
        // map and the HUD are *views* of the workspace, never a copy that can
        // drift. The Atlas stage never dispatches an intent, so `ctx.dispatch`
        // is unused here (read-only stage).
        let workspace = ctx.workspace;
        let view = view! {
            <style>{ATLAS_CSS}</style>
            <section
                class="dna-atlas"
                data-stage="atlas"
                data-testid="atlas-root"
                data-dna-density="reading"
            >
                {move || workspace.with(render_structure_map)}
                {move || workspace.with(render_calc_hud)}
            </section>
        }
        .into_any();
        StageHandle::new(view)
    }
}

/// Pure derivation of the structure map's content from workspace truth —
/// never a DOM-dependent read, so it is unit-testable without mounting
/// Leptos. Reduces [`WorkspaceState::sheets`] and
/// [`WorkspaceState::defined_names`] to plain display labels, in their
/// existing (sheet order / catalog) order — never fabricated, never
/// reordered.
struct StructureMapDerivation {
    sheet_names: Vec<String>,
    name_labels: Vec<String>,
}

fn derive_structure_map(workspace: &WorkspaceState) -> StructureMapDerivation {
    StructureMapDerivation {
        sheet_names: sheet_display_names(&workspace.sheets),
        name_labels: defined_name_labels(&workspace.defined_names),
    }
}

/// The workbook's sheet display names, in sheet order — a pure projection of
/// [`SheetProjection::display_name`], never fabricated.
fn sheet_display_names(sheets: &[SheetProjection]) -> Vec<String> {
    sheets
        .iter()
        .map(|sheet| sheet.display_name.clone())
        .collect()
}

/// The workbook's defined-name catalog reduced to display labels — a pure
/// projection of [`DefinedNamesProjection::entries`]' `name` field, never
/// fabricated.
fn defined_name_labels(defined_names: &DefinedNamesProjection) -> Vec<String> {
    defined_names
        .entries
        .iter()
        .map(|entry| entry.name.clone())
        .collect()
}

/// Render the structure-map panel: a sheet list and a defined-name list, each
/// honest-empty when the workbook carries none — never a placeholder row.
fn render_structure_map(workspace: &WorkspaceState) -> AnyView {
    let derived = derive_structure_map(workspace);

    let sheets_view = if derived.sheet_names.is_empty() {
        view! {
            <p class="dna-atlas__empty" data-testid="atlas-sheets-empty">
                "no sheets in this workbook"
            </p>
        }
        .into_any()
    } else {
        derived
            .sheet_names
            .into_iter()
            .map(|name| {
                view! {
                    <p class="dna-atlas__sheet" data-testid="atlas-sheet">
                        {name}
                    </p>
                }
            })
            .collect_view()
            .into_any()
    };

    let names_view = if derived.name_labels.is_empty() {
        view! {
            <p class="dna-atlas__empty" data-testid="atlas-names-empty">
                "no defined names in this workbook"
            </p>
        }
        .into_any()
    } else {
        derived
            .name_labels
            .into_iter()
            .map(|name| {
                view! {
                    <p class="dna-atlas__name" data-testid="atlas-name">
                        {name}
                    </p>
                }
            })
            .collect_view()
            .into_any()
    };

    view! {
        <div class="dna-atlas__panel" data-testid="atlas-structure-map">
            <h3 class="dna-atlas__panel-title">"Structure"</h3>
            <div class="dna-atlas__section">
                <h4 class="dna-atlas__section-title">"Sheets"</h4>
                {sheets_view}
            </div>
            <div class="dna-atlas__section">
                <h4 class="dna-atlas__section-title">"Defined names"</h4>
                {names_view}
            </div>
        </div>
    }
    .into_any()
}

/// Pure derivation of the calc HUD's content from workspace truth, gated
/// honestly on [`WorkspaceState::workbook_calc`] — never a DOM-dependent
/// read, so it is unit-testable without mounting Leptos.
enum CalcHudDerivation {
    /// `workbook_calc` is `None`: no calc-mode/recalc state has ever been
    /// published for this session. The honest empty note, never a
    /// zeros-as-if-real HUD.
    Empty,
    /// `workbook_calc` is `Some`: the real field readouts to render.
    Present(CalcHudPresent),
}

/// The real field readouts for a populated calc HUD — every field is a
/// direct projection of [`WorkbookCalcProjection`] (mode, last-recalc tick,
/// per-sheet dirty summary) plus, when [`WorkspaceState::last_run`] carries
/// one, the last run's [`CalcRunStateProjection`]. Never a fabricated number.
struct CalcHudPresent {
    mode_label: &'static str,
    tick_label: String,
    dirty_summary: String,
    run_label: &'static str,
}

fn derive_calc_hud(workspace: &WorkspaceState) -> CalcHudDerivation {
    match workspace.workbook_calc.as_ref() {
        None => CalcHudDerivation::Empty,
        Some(calc) => {
            CalcHudDerivation::Present(calc_hud_present(calc, workspace.last_run.as_ref()))
        }
    }
}

/// Build the populated HUD's field readouts from the real
/// [`WorkbookCalcProjection`] plus the (separately optional)
/// [`CalcRunProjection`] — the two are distinct `WorkspaceState` fields
/// (`workbook_calc` is the calc-mode/recalc catalog, `last_run` is the most
/// recent run's outcome), so a HUD gated open by `workbook_calc` still reports
/// "no run recorded" honestly if no run has ever completed.
fn calc_hud_present(
    calc: &WorkbookCalcProjection,
    last_run: Option<&CalcRunProjection>,
) -> CalcHudPresent {
    CalcHudPresent {
        mode_label: calc_mode_label(calc.mode),
        tick_label: last_recalc_tick_label(calc.last_recalc_tick),
        dirty_summary: sheet_dirty_summary(&calc.sheets),
        run_label: last_run_label(last_run),
    }
}

/// Mirror of [`CalcModeProjection`]'s two variants as display text — a
/// scheduling fact, never a value fact (matches the projection's own doc
/// contract).
fn calc_mode_label(mode: CalcModeProjection) -> &'static str {
    match mode {
        CalcModeProjection::Automatic => "automatic",
        CalcModeProjection::Manual => "manual",
    }
}

/// The last-recalc tick as display text: the real tick id, or an honest
/// "never run" note — never a fabricated `0`.
fn last_recalc_tick_label(tick: Option<u64>) -> String {
    match tick {
        Some(tick) => tick.to_string(),
        None => "never run".to_string(),
    }
}

/// The per-sheet dirty summary as a `{dirty}/{total} sheets dirty` count over
/// the real [`SheetCalcSummaryProjection`] rows — an honest "no sheets
/// tracked" note when the catalog is empty, never a `0/0` fabrication framed
/// as a real count.
fn sheet_dirty_summary(sheets: &[SheetCalcSummaryProjection]) -> String {
    if sheets.is_empty() {
        return "no sheets tracked".to_string();
    }
    let dirty = sheets.iter().filter(|sheet| sheet.dirty).count();
    format!("{dirty}/{} sheets dirty", sheets.len())
}

/// The last completed run's status as display text — the real
/// [`CalcRunStateProjection`] when [`WorkspaceState::last_run`] carries one,
/// else an honest "no run recorded" note (never a fabricated status).
fn last_run_label(run: Option<&CalcRunProjection>) -> &'static str {
    match run {
        None => "no run recorded",
        Some(run) => calc_run_state_label(run.run_state),
    }
}

/// Mirror of [`CalcRunStateProjection`]'s three variants as display text.
fn calc_run_state_label(state: CalcRunStateProjection) -> &'static str {
    match state {
        CalcRunStateProjection::Published => "published",
        CalcRunStateProjection::VerifiedClean => "verified clean",
        CalcRunStateProjection::Rejected => "rejected",
    }
}

/// Render the calc-HUD panel: the honest empty note when `workbook_calc` is
/// `None`, else the real field readouts from [`derive_calc_hud`].
///
/// The empty note describes the absent field precisely — `workbook_calc`
/// (calc-mode/recalc *state*), NOT the run history. Saying "no calc *run*
/// recorded" here would be a falsehood in the representable state
/// `workbook_calc: None, last_run: Some(_)` (a pre-H5 mirror can carry a run
/// outcome with no published calc state); the note talks only about the field
/// it actually gated on, so it can never contradict a present `last_run`.
fn render_calc_hud(workspace: &WorkspaceState) -> AnyView {
    match derive_calc_hud(workspace) {
        CalcHudDerivation::Empty => view! {
            <div class="dna-atlas__panel dna-atlas__hud" data-testid="atlas-hud">
                <h3 class="dna-atlas__panel-title">"Calc"</h3>
                <p class="dna-atlas__empty" data-testid="atlas-hud-empty">
                    "no calc state published"
                </p>
            </div>
        }
        .into_any(),
        CalcHudDerivation::Present(present) => view! {
            <div class="dna-atlas__panel dna-atlas__hud" data-testid="atlas-hud">
                <h3 class="dna-atlas__panel-title">"Calc"</h3>
                <p class="dna-atlas__hud-row" data-testid="atlas-hud-mode">
                    "mode: " {present.mode_label}
                </p>
                <p class="dna-atlas__hud-row" data-testid="atlas-hud-tick">
                    "last recalc: " {present.tick_label}
                </p>
                <p class="dna-atlas__hud-row" data-testid="atlas-hud-sheets">
                    {present.dirty_summary}
                </p>
                <p class="dna-atlas__hud-row" data-testid="atlas-hud-run">
                    "last run: " {present.run_label}
                </p>
            </div>
        }
        .into_any(),
    }
}

/// The Atlas stage's scoped stylesheet — Strand `--dna-*` tokens only, mirrors
/// `dnacalc-stage-notebook`'s `NOTEBOOK_CSS` in structure and density
/// (`68ch` measure, `1.6` reading leading).
pub const ATLAS_CSS: &str = "\
.dna-atlas{display:flex;flex-direction:column;gap:var(--dna-gap-4);max-width:68ch;line-height:1.6;padding:var(--dna-gap-5);color:var(--dna-ink)}
.dna-atlas__panel{display:flex;flex-direction:column;gap:var(--dna-gap-3);border:1px solid var(--dna-line);border-radius:var(--dna-radius-card);background:var(--dna-paper);padding:var(--dna-gap-4)}
.dna-atlas__panel-title{margin:0;font-size:13px;font-weight:600;color:var(--dna-ink);text-transform:uppercase;letter-spacing:0.04em}
.dna-atlas__section{display:flex;flex-direction:column;gap:var(--dna-gap-2)}
.dna-atlas__section-title{margin:0;font-size:11px;font-weight:600;color:var(--dna-ink-2);text-transform:uppercase;letter-spacing:0.04em}
.dna-atlas__sheet,.dna-atlas__name{margin:0;font-family:'Recursive Mono','Cascadia Code',Consolas,ui-monospace,monospace;font-size:12px;color:var(--dna-value-ink)}
.dna-atlas__empty{margin:0;color:var(--dna-ink-3);font-style:italic}
.dna-atlas__hud-row{margin:0;font-family:'Recursive Mono','Cascadia Code',Consolas,ui-monospace,monospace;font-size:12px;color:var(--dna-ink-2)}
";

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use dnacalc_skin_ir::{
        DefinedNameProjection, DefinedNameScopeProjection, DefinedNameTargetProjection, NodeId,
    };

    use super::*;

    #[test]
    fn atlas_stage_identity() {
        let stage = AtlasStage::new();
        assert_eq!(stage.id(), StageId::Atlas);
        assert_eq!(stage.title(), "Atlas");
        assert!(stage.supports(&ProfileTag::ExcelStrict));
    }

    /// A hand-built fixture: 2 sheets + 1 workbook-scoped defined name — no
    /// `dnacalc-host-core` involved, per the P-gate discipline (this crate
    /// omits the host-core dev-dep entirely).
    fn fixture_workspace_with_two_sheets_and_one_name() -> WorkspaceState {
        WorkspaceState {
            sheets: vec![
                SheetProjection {
                    grid_node_id: NodeId::new("sheet:Sheet1"),
                    display_name: "Sheet1".to_string(),
                    position: 0,
                },
                SheetProjection {
                    grid_node_id: NodeId::new("sheet:Sheet2"),
                    display_name: "Sheet2".to_string(),
                    position: 1,
                },
            ],
            defined_names: DefinedNamesProjection {
                entries: vec![DefinedNameProjection {
                    scope: DefinedNameScopeProjection::Workbook,
                    name: "GrowthRate".to_string(),
                    target: DefinedNameTargetProjection::Dynamic {
                        source_text: "0.12".to_string(),
                    },
                    is_dynamic: true,
                }],
            },
            ..Default::default()
        }
    }

    #[test]
    fn structure_map_lists_real_sheets_and_names() {
        let workspace = fixture_workspace_with_two_sheets_and_one_name();
        let derived = derive_structure_map(&workspace);
        assert_eq!(
            derived.sheet_names,
            vec!["Sheet1".to_string(), "Sheet2".to_string()]
        );
        assert_eq!(derived.name_labels, vec!["GrowthRate".to_string()]);
    }

    #[test]
    fn structure_map_is_honest_empty_when_workbook_carries_none() {
        let workspace = WorkspaceState::default();
        let derived = derive_structure_map(&workspace);
        assert!(
            derived.sheet_names.is_empty(),
            "no sheets fabricated for an empty workbook"
        );
        assert!(
            derived.name_labels.is_empty(),
            "no defined names fabricated for an empty workbook"
        );
    }

    #[test]
    fn calc_hud_is_honest_empty_when_workbook_calc_is_none() {
        let workspace = WorkspaceState::default();
        assert!(
            matches!(derive_calc_hud(&workspace), CalcHudDerivation::Empty),
            "None workbook_calc must never render as a zeros-as-if-real HUD"
        );
    }

    #[test]
    fn calc_hud_reflects_real_workbook_calc_fields() {
        let workspace = WorkspaceState {
            workbook_calc: Some(WorkbookCalcProjection {
                mode: CalcModeProjection::Manual,
                last_recalc_tick: Some(42),
                sheets: vec![
                    SheetCalcSummaryProjection {
                        grid_node_id: NodeId::new("sheet:Sheet1"),
                        dirty: true,
                    },
                    SheetCalcSummaryProjection {
                        grid_node_id: NodeId::new("sheet:Sheet2"),
                        dirty: false,
                    },
                ],
            }),
            last_run: Some(CalcRunProjection {
                run_state: CalcRunStateProjection::VerifiedClean,
                evaluation_order: Vec::new(),
                runtime_effect_count: 0,
                runtime_effects: Vec::new(),
                runtime_overlay_count: 0,
                runtime_overlays: Vec::new(),
                derivation_trace_count: 0,
                derivation_traces: Vec::new(),
                invalidated_nodes: Vec::new(),
                binding_diagnostics: Vec::new(),
                phase_timings_micros: BTreeMap::new(),
                diagnostics: Vec::new(),
            }),
            ..Default::default()
        };

        match derive_calc_hud(&workspace) {
            CalcHudDerivation::Present(present) => {
                assert_eq!(present.mode_label, "manual");
                assert_eq!(present.tick_label, "42");
                assert_eq!(present.dirty_summary, "1/2 sheets dirty");
                assert_eq!(present.run_label, "verified clean");
            }
            CalcHudDerivation::Empty => panic!("Some(workbook_calc) must derive Present"),
        }
    }

    #[test]
    fn calc_hud_present_is_honest_about_no_run_recorded_even_with_calc_state() {
        // workbook_calc can be populated (a session that has set a calc mode)
        // while no recalc has ever actually run — the run_label must stay
        // honest ("no run recorded"), never borrow a status from nowhere.
        let workspace = WorkspaceState {
            workbook_calc: Some(WorkbookCalcProjection {
                mode: CalcModeProjection::Automatic,
                last_recalc_tick: None,
                sheets: Vec::new(),
            }),
            ..Default::default()
        };
        match derive_calc_hud(&workspace) {
            CalcHudDerivation::Present(present) => {
                assert_eq!(present.mode_label, "automatic");
                assert_eq!(present.tick_label, "never run");
                assert_eq!(present.dirty_summary, "no sheets tracked");
                assert_eq!(present.run_label, "no run recorded");
            }
            CalcHudDerivation::Empty => panic!("Some(workbook_calc) must derive Present"),
        }
    }

    #[test]
    fn calc_mode_label_maps_each_variant() {
        assert_eq!(calc_mode_label(CalcModeProjection::Automatic), "automatic");
        assert_eq!(calc_mode_label(CalcModeProjection::Manual), "manual");
    }

    #[test]
    fn last_recalc_tick_label_is_honest_about_never_run() {
        assert_eq!(last_recalc_tick_label(None), "never run");
        assert_eq!(last_recalc_tick_label(Some(7)), "7");
    }

    #[test]
    fn sheet_dirty_summary_counts_real_dirty_sheets() {
        assert_eq!(sheet_dirty_summary(&[]), "no sheets tracked");
        let sheets = vec![
            SheetCalcSummaryProjection {
                grid_node_id: NodeId::new("sheet:Sheet1"),
                dirty: true,
            },
            SheetCalcSummaryProjection {
                grid_node_id: NodeId::new("sheet:Sheet2"),
                dirty: false,
            },
            SheetCalcSummaryProjection {
                grid_node_id: NodeId::new("sheet:Sheet3"),
                dirty: true,
            },
        ];
        assert_eq!(sheet_dirty_summary(&sheets), "2/3 sheets dirty");
    }

    #[test]
    fn last_run_label_is_honest_when_no_run_recorded() {
        assert_eq!(last_run_label(None), "no run recorded");
    }

    #[test]
    fn calc_run_state_label_maps_each_variant() {
        assert_eq!(
            calc_run_state_label(CalcRunStateProjection::Published),
            "published"
        );
        assert_eq!(
            calc_run_state_label(CalcRunStateProjection::VerifiedClean),
            "verified clean"
        );
        assert_eq!(
            calc_run_state_label(CalcRunStateProjection::Rejected),
            "rejected"
        );
    }
}
