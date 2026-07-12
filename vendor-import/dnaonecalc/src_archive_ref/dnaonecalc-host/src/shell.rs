use std::env;

use eframe::egui;
use egui::TextBuffer as _;

use crate::{
    ArrayPreviewSummary, CapabilitySnapshotDiffSummary, CompletionProposalSummary,
    FormulaEditPacketSummary, FormulaEditorSession, FormulaEvaluationSummary, FunctionHelpSummary,
    IsolatedConditionalFormattingCarrier, OneCalcHostProfile, OpenedCapabilitySnapshotSummary,
    OpenedXRaySummary, RetainedScenarioStore, RuntimeAdapter, XRayBindSummary, XRayEvalSummary,
    XRayParseSummary, XRayProvenanceSummary, XRayTraceSummary,
};

pub const FORMULA_REGION_ID: &str = "formula";
pub const RESULT_REGION_ID: &str = "result";
pub const DIAGNOSTICS_REGION_ID: &str = "diagnostics";
pub const CAPABILITY_REGION_ID: &str = "capability_center";
pub const XRAY_REGION_ID: &str = "xray";
const INSPECTOR_DEFAULT_WIDTH: f32 = 340.0;
const INSPECTOR_MIN_WIDTH: f32 = 300.0;
const XRAY_DEFAULT_WIDTH: f32 = 300.0;
const XRAY_MIN_WIDTH: f32 = 260.0;
const INSPECTOR_DIAGNOSTICS_SPLIT: f32 = 0.42;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaEditorState {
    pub buffer: String,
    pub cursor_index: usize,
    pub selection_start: usize,
    pub selection_end: usize,
    pub selected_text: String,
}

impl FormulaEditorState {
    fn new(buffer: impl Into<String>) -> Self {
        let buffer = buffer.into();
        let cursor_index = buffer.chars().count();

        Self {
            buffer,
            cursor_index,
            selection_start: cursor_index,
            selection_end: cursor_index,
            selected_text: String::new(),
        }
    }

    fn sync_from_output(&mut self, output: &egui::text_edit::TextEditOutput) {
        if let Some(cursor_range) = output.cursor_range {
            let sorted_chars = cursor_range.as_sorted_char_range();
            self.cursor_index = cursor_range.primary.ccursor.index;
            self.selection_start = sorted_chars.start;
            self.selection_end = sorted_chars.end;
            self.selected_text = self.buffer.char_range(sorted_chars).to_string();
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CapabilityCenterState {
    active_snapshot: Option<OpenedCapabilitySnapshotSummary>,
    snapshot_diff: Option<CapabilitySnapshotDiffSummary>,
    last_error: Option<String>,
}

impl CapabilityCenterState {
    fn empty() -> Self {
        Self {
            active_snapshot: None,
            snapshot_diff: None,
            last_error: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EffectiveDisplayRenderState {
    display_text: String,
    formatting_plane_source: String,
    emphasis: String,
    number_format: String,
}

impl EffectiveDisplayRenderState {
    fn none() -> Self {
        Self {
            display_text: "not evaluated yet".to_string(),
            formatting_plane_source: "none".to_string(),
            emphasis: "plain".to_string(),
            number_format: "none".to_string(),
        }
    }

    fn from_summary(summary: &FormulaEvaluationSummary) -> Self {
        let display_text = decode_display_text(&summary.worksheet_value_summary);
        let number_format = extract_presentation_hint_field(
            &summary.returned_presentation_hint_status,
            "number_format",
        )
        .unwrap_or_else(|| "none".to_string());
        let presentation_style =
            extract_presentation_hint_field(&summary.returned_presentation_hint_status, "style")
                .unwrap_or_else(|| "none".to_string());
        let host_style = if summary.host_style_state_status == "none" {
            None
        } else {
            Some(summary.host_style_state_status.as_str())
        };

        let formatting_plane_source = match (
            summary.returned_presentation_hint_status == "none",
            summary.host_style_state_status == "none",
        ) {
            (true, true) => "none".to_string(),
            (false, true) => "presentation_hint".to_string(),
            (true, false) => "host_style".to_string(),
            (false, false) => "presentation_hint+host_style".to_string(),
        };

        let emphasis = if let Some(host_style) = host_style {
            format!("host:{host_style}")
        } else if presentation_style != "none" {
            format!("hint:{presentation_style}")
        } else {
            "plain".to_string()
        };

        Self {
            display_text,
            formatting_plane_source,
            emphasis,
            number_format,
        }
    }

    fn rich_text(&self) -> egui::RichText {
        let mut text = egui::RichText::new(self.display_text.clone());
        if self.emphasis != "plain" {
            text = text.italics();
        }
        if self.number_format != "none" {
            text = text.monospace();
        }
        text
    }
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq)]
struct ExplorerRegressionState {
    result_visible: bool,
    inspector_visible: bool,
    capability_center_visible: bool,
    xray_visible: bool,
    result_scroll_enabled: bool,
    diagnostics_scroll_enabled: bool,
    capability_scroll_enabled: bool,
    xray_scroll_enabled: bool,
    diagnostics_height_fraction: f32,
}

type ShellXRayModel = OpenedXRaySummary;

pub struct OneCalcShellApp {
    runtime_adapter: RuntimeAdapter,
    capability_store: RetainedScenarioStore,
    capability_center: CapabilityCenterState,
    latest_capability_snapshot_id: Option<String>,
    edit_session: FormulaEditorSession,
    latest_edit_packet: FormulaEditPacketSummary,
    latest_evaluation: Option<FormulaEvaluationSummary>,
    completion_items: Vec<CompletionProposalSummary>,
    function_help: Option<FunctionHelpSummary>,
    rendered_diagnostics: Vec<String>,
    host_profile_id: String,
    packet_register_text: String,
    platform_gate_text: String,
    function_policy_text: String,
    conditional_formatting_policy_text: String,
    editor_state: FormulaEditorState,
    returned_presentation_hint_text: String,
    host_style_state_text: String,
    effective_display_render: EffectiveDisplayRenderState,
    result_text: String,
    diagnostics_text: String,
    latest_host_driving_packet_kind: String,
    support_sidebar_visible: bool,
    capability_center_visible: bool,
    xray_visible: bool,
    editor_focus_requested: bool,
    smoke_mode: bool,
    smoke_reported: bool,
}

impl OneCalcShellApp {
    pub fn new(adapter: RuntimeAdapter, smoke_mode: bool) -> Self {
        Self::with_formula(adapter, "=SUM(1,2,3)".to_string(), smoke_mode)
    }

    fn with_formula(adapter: RuntimeAdapter, formula_text: String, smoke_mode: bool) -> Self {
        let host_profile_id = adapter.host_profile().id().to_string();
        let capability_store = RetainedScenarioStore::new(env::temp_dir().join(format!(
            "dnaonecalc-shell-capability-{}",
            std::process::id()
        )));
        let packet_register_text = adapter
            .packet_kinds()
            .iter()
            .map(|packet| packet.id())
            .collect::<Vec<_>>()
            .join(", ");
        let platform_gate_text = adapter.platform_gate().message().to_string();
        let function_summary = adapter.function_surface_summary();
        let function_policy_text = format!(
            "Function Policy: supported={} preview={} experimental={} deferred={} catalog_only={} executable=supported+preview only",
            function_summary.supported,
            function_summary.preview,
            function_summary.experimental,
            function_summary.deferred,
            function_summary.catalog_only
        );
        let conditional_formatting_policy_text =
            IsolatedConditionalFormattingCarrier::policy_text();
        let probe = adapter.dependency_probe().ok();
        let mut edit_session = FormulaEditorSession::new("onecalc.editor");
        let latest_edit_packet =
            adapter.apply_formula_edit_packet(&mut edit_session, &formula_text);
        let completion_items =
            adapter.collect_completion_proposals(&edit_session, formula_text.chars().count());
        let rendered_diagnostics = Self::render_live_diagnostics(&edit_session);
        let result_text = "result: not evaluated yet".to_string();
        let diagnostics_text = match &probe {
            Some(report) => format!(
                "probe_parse_diagnostic_count: {}\nedit_packet_diagnostic_count: {}\nreplay_ready: {}\npacket_kinds: {}",
                report.parse_diagnostic_count,
                latest_edit_packet.diagnostic_count,
                report.replay_ready,
                packet_register_text
            ),
            None => "dependency probe failed before shell render".to_string(),
        };

        let mut app = Self {
            runtime_adapter: adapter,
            capability_store,
            capability_center: CapabilityCenterState::empty(),
            latest_capability_snapshot_id: None,
            edit_session,
            latest_edit_packet,
            latest_evaluation: None,
            completion_items,
            function_help: None,
            rendered_diagnostics,
            host_profile_id,
            packet_register_text,
            platform_gate_text,
            function_policy_text,
            conditional_formatting_policy_text,
            editor_state: FormulaEditorState::new(formula_text),
            returned_presentation_hint_text: "none".to_string(),
            host_style_state_text: "none".to_string(),
            effective_display_render: EffectiveDisplayRenderState::none(),
            result_text,
            diagnostics_text,
            latest_host_driving_packet_kind: "formula_edit".to_string(),
            support_sidebar_visible: true,
            capability_center_visible: false,
            xray_visible: false,
            editor_focus_requested: false,
            smoke_mode,
            smoke_reported: false,
        };

        app.refresh_capability_center("formula_edit");

        if smoke_mode {
            app.evaluate_current_formula();
        }
        app.refresh_function_help();

        app
    }

    pub const fn region_ids() -> &'static [&'static str] {
        &[
            FORMULA_REGION_ID,
            RESULT_REGION_ID,
            DIAGNOSTICS_REGION_ID,
            CAPABILITY_REGION_ID,
            XRAY_REGION_ID,
        ]
    }

    #[cfg(test)]
    fn diagnostics_height_fraction(&self) -> f32 {
        if self.capability_center_visible {
            INSPECTOR_DIAGNOSTICS_SPLIT
        } else {
            1.0
        }
    }

    #[cfg(test)]
    fn explorer_regression_state(&self) -> ExplorerRegressionState {
        ExplorerRegressionState {
            result_visible: true,
            inspector_visible: self.support_sidebar_visible,
            capability_center_visible: self.capability_center_visible,
            xray_visible: self.xray_visible,
            result_scroll_enabled: true,
            diagnostics_scroll_enabled: self.support_sidebar_visible,
            capability_scroll_enabled: self.capability_center_visible,
            xray_scroll_enabled: self.xray_visible,
            diagnostics_height_fraction: self.diagnostics_height_fraction(),
        }
    }

    fn xray_model(&self) -> ShellXRayModel {
        let parse = XRayParseSummary {
            formula_token: self.latest_edit_packet.formula_token.clone(),
            diagnostic_count: self.latest_edit_packet.diagnostic_count,
            text_change_range: format!("{:?}", self.latest_edit_packet.text_change_range),
        };
        let bind = XRayBindSummary {
            reused_green_tree: Some(self.latest_edit_packet.reused_green_tree),
            reused_red_projection: Some(self.latest_edit_packet.reused_red_projection),
            reused_bound_formula: Some(self.latest_edit_packet.reused_bound_formula),
            current_help_name: self
                .function_help
                .as_ref()
                .map(|help| help.display_name.clone())
                .unwrap_or_else(|| "unavailable".to_string()),
            availability_summary: self
                .function_help
                .as_ref()
                .map(|help| help.availability_summary.clone())
                .unwrap_or_else(|| "unavailable".to_string()),
        };
        let evaluation = self
            .latest_evaluation
            .as_ref()
            .map(|summary| XRayEvalSummary {
                worksheet_value_summary: summary.worksheet_value_summary.clone(),
                payload_summary: summary.payload_summary.clone(),
                effective_display_status: summary.effective_display_status.clone(),
                returned_surface_kind: summary.returned_value_surface_kind.clone(),
                returned_presentation_hint_status: summary
                    .returned_presentation_hint_status
                    .clone(),
                host_style_state_status: summary.host_style_state_status.clone(),
            });
        let trace = self
            .latest_evaluation
            .as_ref()
            .map(|summary| XRayTraceSummary {
                trace_event_count: summary.trace_event_count,
                commit_decision_kind: summary.commit_decision_kind.clone(),
                replay_capture_id: None,
                replay_floor: None,
                replay_projection_source_artifact_family: None,
                replay_projection_phase: None,
                replay_projection_alias: None,
            });
        let provenance = XRayProvenanceSummary {
            host_profile_id: self.host_profile_id.clone(),
            platform_gate_text: self.platform_gate_text.clone(),
            latest_host_driving_packet_kind: self.latest_host_driving_packet_kind.clone(),
            packet_register_text: self.packet_register_text.clone(),
            latest_capability_snapshot_id: self
                .latest_capability_snapshot_id
                .clone()
                .unwrap_or_else(|| "unavailable".to_string()),
            capability_floor: self
                .capability_center
                .active_snapshot
                .as_ref()
                .map(|snapshot| snapshot.capability_floor.clone())
                .unwrap_or_else(|| "unavailable".to_string()),
            runtime_class: self
                .capability_center
                .active_snapshot
                .as_ref()
                .map(|snapshot| snapshot.runtime_class.clone())
                .unwrap_or_else(|| "unavailable".to_string()),
            function_surface_policy_id: self
                .capability_center
                .active_snapshot
                .as_ref()
                .map(|snapshot| snapshot.function_surface_policy_id.clone())
                .unwrap_or_else(|| "unavailable".to_string()),
            mode_availability_summary: self
                .capability_center
                .active_snapshot
                .as_ref()
                .map(|snapshot| {
                    snapshot
                        .mode_availability
                        .iter()
                        .map(|mode| format!("{}:{}", mode.mode_id, mode.state))
                        .collect::<Vec<_>>()
                        .join("|")
                })
                .unwrap_or_else(|| "unavailable".to_string()),
            scenario_id: None,
            scenario_run_id: None,
            formula_text: None,
            formula_text_version: None,
            formatting_truth_plane: "formatting_truth_not_opened".to_string(),
            conditional_formatting_scope: "conditional_formatting_truth_not_opened".to_string(),
            blocked_dimensions: Vec::new(),
        };

        OpenedXRaySummary {
            parse,
            bind,
            evaluation,
            trace,
            provenance,
        }
    }

    fn request_editor_focus(&mut self) {
        self.editor_focus_requested = false;
    }

    fn toggle_support_sidebar(&mut self) {
        self.support_sidebar_visible = !self.support_sidebar_visible;
    }

    fn toggle_capability_center(&mut self) {
        self.capability_center_visible = !self.capability_center_visible;
        if self.capability_center_visible {
            self.support_sidebar_visible = true;
        }
    }

    fn toggle_xray(&mut self) {
        self.xray_visible = !self.xray_visible;
    }

    fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        let evaluate_pressed = ctx.input(|input| {
            input.key_pressed(egui::Key::Enter)
                && input.modifiers.ctrl
                && !input.modifiers.alt
                && !input.modifiers.shift
        });
        if evaluate_pressed {
            self.evaluate_current_formula();
            self.request_editor_focus();
        }

        if ctx.input(|input| input.key_pressed(egui::Key::F6)) {
            self.toggle_support_sidebar();
            self.request_editor_focus();
        }

        if ctx.input(|input| input.key_pressed(egui::Key::F7)) {
            self.toggle_capability_center();
            self.request_editor_focus();
        }

        if ctx.input(|input| input.key_pressed(egui::Key::F8)) {
            self.toggle_xray();
            self.request_editor_focus();
        }
    }

    fn render_context_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("context_bar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.strong(format!("Host Profile: {}", self.host_profile_id));
                ui.separator();
                ui.label(format!("Packet Kinds: {}", self.packet_register_text));
                ui.separator();
                ui.colored_label(
                    egui::Color32::from_rgb(140, 88, 0),
                    &self.platform_gate_text,
                );
                ui.separator();
                ui.label(&self.function_policy_text);
                ui.separator();
                ui.label(&self.conditional_formatting_policy_text);
                ui.separator();
                if ui
                    .button(if self.support_sidebar_visible {
                        "Hide Inspector"
                    } else {
                        "Show Inspector"
                    })
                    .clicked()
                {
                    self.toggle_support_sidebar();
                }
                if ui
                    .button(if self.capability_center_visible {
                        "Hide Capability Center"
                    } else {
                        "Show Capability Center"
                    })
                    .clicked()
                {
                    self.toggle_capability_center();
                }
                if ui
                    .button(if self.xray_visible {
                        "Hide X-Ray"
                    } else {
                        "Show X-Ray"
                    })
                    .clicked()
                {
                    self.toggle_xray();
                }
                ui.separator();
                ui.small("Ctrl+Enter evaluate");
                ui.small("F6 inspector");
                ui.small("F7 capability");
                ui.small("F8 X-Ray");
            });
        });
    }

    fn render_formula_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top(FORMULA_REGION_ID)
            .resizable(false)
            .show(ctx, |ui| {
                ui.heading("Formula Explorer");
                ui.columns(2, |columns| {
                    columns[0].label("Formula");
                    let output = egui::TextEdit::multiline(&mut self.editor_state.buffer)
                        .desired_rows(3)
                        .desired_width(f32::INFINITY)
                        .hint_text("Enter a formula")
                        .show(&mut columns[0]);
                    if !self.editor_focus_requested {
                        output.response.request_focus();
                        self.editor_focus_requested = true;
                    }
                    self.editor_state.sync_from_output(&output);
                    if output.response.changed() {
                        self.sync_edit_packet();
                    }
                    if columns[0].button("Evaluate").clicked() {
                        self.evaluate_current_formula();
                    }
                    columns[0].small(format!(
                        "cursor={} selection={}..{} selected_text=\"{}\"",
                        self.editor_state.cursor_index,
                        self.editor_state.selection_start,
                        self.editor_state.selection_end,
                        self.editor_state.selected_text
                    ));

                    columns[1].group(|ui| {
                        ui.label(format!("Completions ({})", self.completion_items.len()));
                        ui.separator();
                        if self.completion_items.is_empty() {
                            ui.small("No deterministic proposals at the current cursor.");
                        } else {
                            for proposal in self.completion_items.iter().take(6) {
                                ui.horizontal_wrapped(|ui| {
                                    ui.strong(&proposal.proposal_kind);
                                    ui.monospace(&proposal.display_text);
                                });
                            }
                        }
                    });
                    columns[1].add_space(8.0);
                    columns[1].group(|ui| {
                        ui.label("Current Help");
                        ui.separator();
                        if let Some(help) = &self.function_help {
                            ui.strong(&help.display_name);
                            ui.monospace(&help.display_signature);
                            ui.small(format!("Active argument: {}", help.active_argument_index));
                            ui.small(format!("Availability: {}", help.availability_summary));
                            ui.small(if help.provisional {
                                "Status: provisional"
                            } else {
                                "Status: admitted"
                            });
                        } else {
                            ui.small("Current Help: unavailable at cursor");
                        }
                    });
                });
            });
    }

    fn render_diagnostics_panel(&self, ui: &mut egui::Ui, panel_height: f32) {
        ui.heading("Diagnostics");
        ui.separator();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .max_height((panel_height - 40.0).max(0.0))
            .show(ui, |ui| {
                ui.label(&self.diagnostics_text);
                ui.separator();
                ui.monospace(format!(
                    "buffer_len={}\ncursor_index={}\nselection={}..{}\nselected_text={}\nedit_formula_token={}\nedit_diagnostic_count={}\ntext_change_range={:?}\nreused_green_tree={}\nreused_red_projection={}\nreused_bound_formula={}",
                    self.editor_state.buffer.chars().count(),
                    self.editor_state.cursor_index,
                    self.editor_state.selection_start,
                    self.editor_state.selection_end,
                    self.editor_state.selected_text,
                    self.latest_edit_packet.formula_token,
                    self.latest_edit_packet.diagnostic_count,
                    self.latest_edit_packet.text_change_range,
                    self.latest_edit_packet.reused_green_tree,
                    self.latest_edit_packet.reused_red_projection,
                    self.latest_edit_packet.reused_bound_formula
                ));
                ui.separator();
                ui.heading("Live Diagnostics");
                if self.rendered_diagnostics.is_empty() {
                    ui.label("No live diagnostics.");
                } else {
                    for diagnostic in &self.rendered_diagnostics {
                        ui.label(diagnostic);
                    }
                }
            });
    }

    fn render_capability_center(&self, ui: &mut egui::Ui, panel_height: f32) {
        ui.push_id(CAPABILITY_REGION_ID, |ui| {
            ui.heading("Capability Center");
            ui.separator();
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .max_height((panel_height - 40.0).max(0.0))
                .show(ui, |ui| {
                    if let Some(error) = &self.capability_center.last_error {
                        ui.colored_label(egui::Color32::from_rgb(160, 32, 32), error);
                        return;
                    }

                    if let Some(snapshot) = &self.capability_center.active_snapshot {
                        ui.monospace(format!(
                            "snapshot_id={}\nhost_kind={}; runtime_platform={}; runtime_class={}\ncapability_floor={}; seam_pin_set_id={}\nfunction_surface_policy={}",
                            snapshot.capability_snapshot_id,
                            snapshot.host_kind,
                            snapshot.runtime_platform,
                            snapshot.runtime_class,
                            snapshot.capability_floor,
                            snapshot.seam_pin_set_id,
                            snapshot.function_surface_policy_id
                        ));
                        ui.separator();
                        ui.label("Dependency Ledger");
                        ui.monospace(snapshot.dependency_set.join("\n"));
                        ui.separator();
                        ui.label("Packet Kinds");
                        ui.monospace(snapshot.packet_kind_register.join(", "));
                        ui.separator();
                        ui.label("Mode Availability");
                        for mode in &snapshot.mode_availability {
                            let reason = mode.reason.as_deref().unwrap_or("none");
                            ui.monospace(format!("{} = {} ({})", mode.mode_id, mode.state, reason));
                        }
                        ui.separator();
                        ui.label("Provisional Seams");
                        if snapshot.provisional_seams.is_empty() {
                            ui.small("none");
                        } else {
                            ui.monospace(snapshot.provisional_seams.join("\n"));
                        }
                        ui.separator();
                        ui.label("Capability Ceilings");
                        if snapshot.capability_ceilings.is_empty() {
                            ui.small("none");
                        } else {
                            ui.monospace(snapshot.capability_ceilings.join("\n"));
                        }
                        if !snapshot.lossiness.is_empty() {
                            ui.separator();
                            ui.label("Lossiness");
                            ui.monospace(snapshot.lossiness.join("\n"));
                        }
                    } else {
                        ui.small("No immutable capability snapshot available yet.");
                    }

                    ui.separator();
                    ui.label("Snapshot Diff");
                    if let Some(diff) = &self.capability_center.snapshot_diff {
                        ui.monospace(format!(
                            "left={}\nright={}\ndiff_floor={}\nfunction_surface_policy_changed={}\nruntime_class_changed={}",
                            diff.left_snapshot_id,
                            diff.right_snapshot_id,
                            diff.diff_floor,
                            diff.function_surface_policy_changed,
                            diff.runtime_class_changed
                        ));
                        for line in &diff.mode_changes {
                            ui.monospace(format!("mode_change: {line}"));
                        }
                        if !diff.dependencies_added.is_empty()
                            || !diff.dependencies_removed.is_empty()
                        {
                            ui.monospace(format!(
                                "dependencies_added={}\ndependencies_removed={}",
                                diff.dependencies_added.join(","),
                                diff.dependencies_removed.join(",")
                            ));
                        }
                        if !diff.packet_kinds_added.is_empty()
                            || !diff.packet_kinds_removed.is_empty()
                        {
                            ui.monospace(format!(
                                "packet_kinds_added={}\npacket_kinds_removed={}",
                                diff.packet_kinds_added.join(","),
                                diff.packet_kinds_removed.join(",")
                            ));
                        }
                        if !diff.provisional_seams_added.is_empty()
                            || !diff.provisional_seams_removed.is_empty()
                        {
                            ui.monospace(format!(
                                "provisional_seams_added={}\nprovisional_seams_removed={}",
                                diff.provisional_seams_added.join(","),
                                diff.provisional_seams_removed.join(",")
                            ));
                        }
                        if !diff.capability_ceilings_added.is_empty()
                            || !diff.capability_ceilings_removed.is_empty()
                        {
                            ui.monospace(format!(
                                "capability_ceilings_added={}\ncapability_ceilings_removed={}",
                                diff.capability_ceilings_added.join(","),
                                diff.capability_ceilings_removed.join(",")
                            ));
                        }
                    } else {
                        ui.small("No earlier immutable capability snapshot is available for diff.");
                    }
                });
        });
    }

    fn render_support_sidebar(&mut self, ctx: &egui::Context) {
        if !self.support_sidebar_visible {
            return;
        }

        egui::SidePanel::right(DIAGNOSTICS_REGION_ID)
            .resizable(true)
            .default_width(INSPECTOR_DEFAULT_WIDTH)
            .min_width(INSPECTOR_MIN_WIDTH)
            .show(ctx, |ui| {
                ui.heading("Inspector");
                ui.separator();
                ui.horizontal(|ui| {
                    ui.small("Supporting dock");
                    if ui
                        .button(if self.capability_center_visible {
                            "Collapse Capability Center"
                        } else {
                            "Open Capability Center"
                        })
                        .clicked()
                    {
                        self.toggle_capability_center();
                    }
                });
                ui.separator();
                let inspector_height = ui.available_height().max(0.0);
                let diagnostics_height = if self.capability_center_visible {
                    ((inspector_height - 8.0).max(0.0)) * INSPECTOR_DIAGNOSTICS_SPLIT
                } else {
                    inspector_height
                };

                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), diagnostics_height),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.group(|ui| self.render_diagnostics_panel(ui, diagnostics_height));
                    },
                );

                if self.capability_center_visible {
                    ui.add_space(8.0);
                    let capability_height = ui.available_height().max(0.0);
                    ui.allocate_ui_with_layout(
                        egui::vec2(ui.available_width(), capability_height),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            ui.group(|ui| self.render_capability_center(ui, capability_height));
                        },
                    );
                } else {
                    ui.add_space(8.0);
                    ui.group(|ui| {
                        ui.heading("Capability Center");
                        ui.separator();
                        ui.small("Collapsed to keep the explorer and result surfaces in view.");
                    });
                }
            });
    }

    fn render_xray_panel(&self, ctx: &egui::Context) {
        if !self.xray_visible {
            return;
        }

        let xray = self.xray_model();
        egui::SidePanel::left(XRAY_REGION_ID)
            .resizable(true)
            .default_width(XRAY_DEFAULT_WIDTH)
            .min_width(XRAY_MIN_WIDTH)
            .show(ctx, |ui| {
                ui.heading("X-Ray");
                ui.separator();
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.group(|ui| {
                            ui.label("Parse");
                            ui.separator();
                            ui.monospace(format!(
                                "formula_token={}\nparse_diagnostic_count={}\ntext_change_range={:?}",
                                xray.parse.formula_token,
                                xray.parse.diagnostic_count,
                                xray.parse.text_change_range
                            ));
                        });
                        ui.add_space(8.0);
                        ui.group(|ui| {
                            ui.label("Bind");
                            ui.separator();
                            ui.monospace(format!(
                                "reused_green_tree={}\nreused_red_projection={}\nreused_bound_formula={}\ncurrent_help={}\navailability={}",
                                option_bool_text(xray.bind.reused_green_tree),
                                option_bool_text(xray.bind.reused_red_projection),
                                option_bool_text(xray.bind.reused_bound_formula),
                                xray.bind.current_help_name,
                                xray.bind.availability_summary
                            ));
                        });
                        ui.add_space(8.0);
                        ui.group(|ui| {
                            ui.label("Eval");
                            ui.separator();
                            if let Some(summary) = &xray.evaluation {
                                ui.monospace(format!(
                                    "worksheet_value={}\npayload_summary={}\neffective_display={}\nreturned_surface={}\nreturned_presentation_hint={}\nhost_style_state={}",
                                    summary.worksheet_value_summary,
                                    summary.payload_summary,
                                    summary.effective_display_status,
                                    summary.returned_surface_kind,
                                    summary.returned_presentation_hint_status,
                                    summary.host_style_state_status
                                ));
                            } else {
                                ui.small("No evaluated runtime result is available yet.");
                            }
                        });
                        ui.add_space(8.0);
                        ui.group(|ui| {
                            ui.label("Trace");
                            ui.separator();
                            if let Some(summary) = &xray.trace {
                                ui.monospace(format!(
                                    "trace_event_count={}\ncommit_decision={}\nreplay_capture={}\nreplay_floor={}\nreplay_projection_family={}\nreplay_projection_phase={}\nreplay_projection_alias={}",
                                    summary.trace_event_count,
                                    summary.commit_decision_kind,
                                    summary.replay_capture_id.as_deref().unwrap_or("none"),
                                    summary.replay_floor.as_deref().unwrap_or("none"),
                                    summary.replay_projection_source_artifact_family.as_deref().unwrap_or("none"),
                                    summary.replay_projection_phase.as_deref().unwrap_or("none"),
                                    summary.replay_projection_alias.as_deref().unwrap_or("none")
                                ));
                            } else {
                                ui.small("Trace truth appears after evaluation.");
                            }
                        });
                        ui.add_space(8.0);
                        ui.group(|ui| {
                            ui.label("Provenance");
                            ui.separator();
                            ui.monospace(format!(
                                "host_profile={}\nplatform_gate={}\nhost_driving_packet_kind={}\npacket_register={}\nlatest_capability_snapshot={}\ncapability_floor={}\nruntime_class={}\nfunction_surface_policy={}\nmode_availability={}\nscenario_id={}\nscenario_run_id={}\nformula_text_version={}\nformatting_truth_plane={}\nconditional_formatting_scope={}\nblocked_dimensions={}",
                                xray.provenance.host_profile_id,
                                xray.provenance.platform_gate_text,
                                xray.provenance.latest_host_driving_packet_kind,
                                xray.provenance.packet_register_text,
                                xray.provenance.latest_capability_snapshot_id,
                                xray.provenance.capability_floor,
                                xray.provenance.runtime_class,
                                xray.provenance.function_surface_policy_id,
                                xray.provenance.mode_availability_summary,
                                xray.provenance.scenario_id.as_deref().unwrap_or("none"),
                                xray.provenance.scenario_run_id.as_deref().unwrap_or("none"),
                                xray.provenance
                                    .formula_text_version
                                    .map(|value| value.to_string())
                                    .unwrap_or_else(|| "none".to_string()),
                                xray.provenance.formatting_truth_plane,
                                xray.provenance.conditional_formatting_scope,
                                if xray.provenance.blocked_dimensions.is_empty() {
                                    "none".to_string()
                                } else {
                                    xray.provenance.blocked_dimensions.join("|")
                                }
                            ));
                        });
                    });
            });
    }

    fn render_result_panel(&self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.push_id(RESULT_REGION_ID, |ui| {
                ui.heading("Result");
                ui.separator();
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.small(format!(
                            "returned_presentation_hint={} | host_style_state={}",
                            self.returned_presentation_hint_text, self.host_style_state_text
                        ));
                        ui.separator();
                        ui.group(|ui| {
                            ui.label("Effective Display Preview");
                            ui.label(self.effective_display_render.rich_text().size(20.0));
                            ui.small(format!(
                                "formatting_plane_source={} | emphasis={} | number_format={}",
                                self.effective_display_render.formatting_plane_source,
                                self.effective_display_render.emphasis,
                                self.effective_display_render.number_format
                            ));
                        });
                        if let Some(preview) = self
                            .latest_evaluation
                            .as_ref()
                            .and_then(|summary| summary.array_preview.as_ref())
                        {
                            ui.separator();
                            render_array_preview(ui, preview);
                        }
                        ui.separator();
                        ui.code(&self.result_text);
                    });
            });
        });
    }

    fn sync_edit_packet(&mut self) {
        self.latest_edit_packet = self
            .runtime_adapter
            .apply_formula_edit_packet(&mut self.edit_session, self.editor_state.buffer.clone());
        self.refresh_completion_proposals();
        self.rendered_diagnostics = Self::render_live_diagnostics(&self.edit_session);
    }

    fn refresh_completion_proposals(&mut self) {
        self.completion_items = self
            .runtime_adapter
            .collect_completion_proposals(&self.edit_session, self.editor_state.cursor_index);
        self.refresh_function_help();
    }

    fn refresh_function_help(&mut self) {
        self.function_help = self
            .runtime_adapter
            .current_function_help(&self.edit_session, self.editor_state.cursor_index);
    }

    fn evaluate_current_formula(&mut self) {
        match self
            .runtime_adapter
            .evaluate_formula(self.editor_state.buffer.clone())
        {
            Ok(summary) => {
                self.returned_presentation_hint_text =
                    summary.returned_presentation_hint_status.clone();
                self.host_style_state_text = summary.host_style_state_status.clone();
                self.effective_display_render = EffectiveDisplayRenderState::from_summary(&summary);
                self.result_text = format!(
                    "worksheet_value: {}\npayload_summary: {}\nreturned_surface: {}\nreturned_presentation_hint: {}\nhost_style_state: {}\neffective_display: {}\ncommit_decision: {}",
                    summary.worksheet_value_summary,
                    summary.payload_summary,
                    summary.returned_value_surface_kind,
                    summary.returned_presentation_hint_status,
                    summary.host_style_state_status,
                    summary.effective_display_status,
                    summary.commit_decision_kind
                );
                self.latest_evaluation = Some(summary);
            }
            Err(error) => {
                self.returned_presentation_hint_text = "none".to_string();
                self.host_style_state_text = "none".to_string();
                self.effective_display_render = EffectiveDisplayRenderState::none();
                self.result_text = format!("evaluation failed: {error}");
                self.latest_evaluation = None;
            }
        }
        self.refresh_capability_center("edit_accept_recalc");
    }

    fn render_live_diagnostics(edit_session: &FormulaEditorSession) -> Vec<String> {
        edit_session
            .latest_result()
            .map(|result| {
                result
                    .live_diagnostics
                    .diagnostics
                    .iter()
                    .map(|diagnostic| {
                        format!(
                            "{:?} {:?} {}..{} {}",
                            diagnostic.severity,
                            diagnostic.stage,
                            diagnostic.primary_span.start,
                            diagnostic.primary_span.end(),
                            diagnostic.message
                        )
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn refresh_capability_center(&mut self, packet_kind: &str) {
        self.latest_host_driving_packet_kind = packet_kind.to_string();
        let previous_snapshot_id = self.latest_capability_snapshot_id.clone();
        match self.runtime_adapter.persist_capability_snapshot(
            &self.capability_store,
            packet_kind,
            previous_snapshot_id.as_deref(),
        ) {
            Ok(persisted) => {
                let opened = self.runtime_adapter.open_capability_snapshot(
                    &self.capability_store,
                    &persisted.snapshot.capability_snapshot_id,
                );
                let diff = previous_snapshot_id
                    .as_deref()
                    .map(|left_id| {
                        self.runtime_adapter.diff_capability_snapshots(
                            &self.capability_store,
                            left_id,
                            &persisted.snapshot.capability_snapshot_id,
                        )
                    })
                    .transpose();

                match (opened, diff) {
                    (Ok(active_snapshot), Ok(snapshot_diff)) => {
                        self.capability_center = CapabilityCenterState {
                            active_snapshot: Some(active_snapshot),
                            snapshot_diff,
                            last_error: None,
                        };
                        self.latest_capability_snapshot_id =
                            Some(persisted.snapshot.capability_snapshot_id);
                    }
                    (Err(error), _) | (_, Err(error)) => {
                        self.capability_center = CapabilityCenterState {
                            active_snapshot: None,
                            snapshot_diff: None,
                            last_error: Some(error),
                        };
                    }
                }
            }
            Err(error) => {
                self.capability_center = CapabilityCenterState {
                    active_snapshot: None,
                    snapshot_diff: None,
                    last_error: Some(error),
                };
            }
        }
    }
}

impl eframe::App for OneCalcShellApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_keyboard_shortcuts(ctx);
        self.render_context_bar(ctx);
        self.render_formula_panel(ctx);
        self.render_xray_panel(ctx);
        self.render_support_sidebar(ctx);
        self.render_result_panel(ctx);

        if self.smoke_mode && !self.smoke_reported {
            println!("shell_regions={}", Self::region_ids().join(","));
            println!(
                "shell_truth=host_profile:{};packet_kinds:{};platform_gate:{};function_policy:{}",
                self.host_profile_id,
                self.packet_register_text.replace(", ", "|"),
                self.platform_gate_text,
                self.function_policy_text
                    .replace(": ", "=")
                    .replace(" ", "_")
            );
            println!(
                "conditional_formatting_truth={}",
                self.conditional_formatting_policy_text
                    .replace(": ", "=")
                    .replace(" ", "_")
            );
            println!(
                "editor_truth=buffer_len:{};cursor_index:{};selection:{}..{}",
                self.editor_state.buffer.chars().count(),
                self.editor_state.cursor_index,
                self.editor_state.selection_start,
                self.editor_state.selection_end
            );
            println!(
                "edit_packet=formula_token:{};diagnostic_count:{};text_change_range:{:?};reused_green_tree:{};reused_red_projection:{};reused_bound_formula:{}",
                self.latest_edit_packet.formula_token,
                self.latest_edit_packet.diagnostic_count,
                self.latest_edit_packet.text_change_range,
                self.latest_edit_packet.reused_green_tree,
                self.latest_edit_packet.reused_red_projection,
                self.latest_edit_packet.reused_bound_formula
            );
            println!("live_diagnostic_lines={}", self.rendered_diagnostics.len());
            if let Some(summary) = &self.latest_evaluation {
                println!(
                "evaluation_truth=formula_token:{};worksheet_value:{};payload_summary:{};returned_surface:{};returned_presentation_hint:{};host_style_state:{};effective_display:{};commit_decision:{};trace_event_count:{}",
                    summary.formula_token,
                    summary.worksheet_value_summary,
                    summary.payload_summary,
                    summary.returned_value_surface_kind,
                    summary.returned_presentation_hint_status,
                    summary.host_style_state_status,
                    summary.effective_display_status,
                    summary.commit_decision_kind,
                    summary.trace_event_count
                );
            }
            if let Some(snapshot) = &self.capability_center.active_snapshot {
                println!(
                    "capability_center=snapshot:{};runtime_class:{};dependencies:{};modes:{};seams:{};ceilings:{}",
                    snapshot.capability_snapshot_id,
                    snapshot.runtime_class,
                    snapshot.dependency_set.join("|"),
                    snapshot
                        .mode_availability
                        .iter()
                        .map(|mode| format!("{}:{}", mode.mode_id, mode.state))
                        .collect::<Vec<_>>()
                        .join("|"),
                    snapshot.provisional_seams.join("|"),
                    snapshot.capability_ceilings.join("|")
                );
            }
            if let Some(diff) = &self.capability_center.snapshot_diff {
                println!(
                    "capability_diff=left:{};right:{};mode_changes:{};policy_changed:{};runtime_class_changed:{}",
                    diff.left_snapshot_id,
                    diff.right_snapshot_id,
                    diff.mode_changes.join("|"),
                    diff.function_surface_policy_changed,
                    diff.runtime_class_changed
                );
            }
            self.smoke_reported = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

fn extract_presentation_hint_field(summary: &str, field: &str) -> Option<String> {
    if summary == "none" {
        return None;
    }

    let prefix = format!("{field}:");
    summary
        .split(';')
        .find_map(|segment| segment.strip_prefix(&prefix))
        .map(|value| value.to_string())
}

fn option_bool_text(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "true",
        Some(false) => "false",
        None => "unavailable",
    }
}

fn decode_display_text(summary: &str) -> String {
    if let Some(value) = summary
        .strip_prefix("Number(")
        .and_then(|value| value.strip_suffix(')'))
    {
        return value.to_string();
    }
    if let Some(value) = summary
        .strip_prefix("Text(")
        .and_then(|value| value.strip_suffix(')'))
    {
        return value.to_string();
    }

    summary.to_string()
}

fn array_preview_overflow_note(preview: &ArrayPreviewSummary) -> Option<String> {
    if !preview.is_truncated() {
        return None;
    }

    let preview_row_count = preview.rows.len();
    let preview_column_count = preview.rows.first().map_or(0, Vec::len);
    let mut hidden_parts = Vec::new();

    if preview.hidden_row_count > 0 {
        hidden_parts.push(format!("{} more row(s)", preview.hidden_row_count));
    }
    if preview.hidden_column_count > 0 {
        hidden_parts.push(format!("{} more column(s)", preview.hidden_column_count));
    }

    Some(format!(
        "showing {preview_row_count} x {preview_column_count}; {} not shown",
        hidden_parts.join(", ")
    ))
}

fn render_array_preview(ui: &mut egui::Ui, preview: &ArrayPreviewSummary) {
    let preview_column_count = preview.rows.first().map_or(0, Vec::len);

    ui.group(|ui| {
        ui.label(format!(
            "Array Preview ({} x {})",
            preview.row_count, preview.column_count
        ));
        if let Some(note) = array_preview_overflow_note(preview) {
            ui.small(note);
        }
        ui.add_space(4.0);
        egui::Grid::new("result_array_preview_grid")
            .striped(true)
            .min_col_width(56.0)
            .show(ui, |ui| {
                ui.small("");
                for column_index in 0..preview_column_count {
                    ui.small(format!("C{}", column_index + 1));
                }
                ui.end_row();

                for (row_index, row) in preview.rows.iter().enumerate() {
                    ui.small(format!("R{}", row_index + 1));
                    for cell in row {
                        ui.monospace(cell);
                    }
                    ui.end_row();
                }
            });
    });
}

pub fn launch_shell(smoke_mode: bool) -> Result<(), eframe::Error> {
    launch_shell_with_formula("=SUM(1,2,3)", smoke_mode)
}

pub fn launch_shell_with_formula(
    formula_text: impl Into<String>,
    smoke_mode: bool,
) -> Result<(), eframe::Error> {
    let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH0);
    let formula_text = formula_text.into();
    let title = if smoke_mode {
        "DNA OneCalc Shell Smoke"
    } else {
        "DNA OneCalc"
    };

    eframe::run_native(
        title,
        eframe::NativeOptions::default(),
        Box::new(move |_cc| {
            Ok(Box::new(OneCalcShellApp::with_formula(
                adapter,
                formula_text,
                smoke_mode,
            )))
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{
        adapter_for, promoted_formatting_scenario, promoted_formula_scenario,
        FormattingScenarioFamily, FormulaScenarioFamily,
    };

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum ShellTestShortcut {
        Evaluate,
        ToggleInspector,
        ToggleCapabilityCenter,
        ToggleXRay,
    }

    struct ShellInteractionHarness {
        app: OneCalcShellApp,
    }

    impl ShellInteractionHarness {
        fn new(scenario_family: FormulaScenarioFamily, smoke_mode: bool) -> Self {
            let scenario = promoted_formula_scenario(scenario_family);
            Self {
                app: OneCalcShellApp::with_formula(
                    adapter_for(scenario.host_profile),
                    scenario.formula.to_string(),
                    smoke_mode,
                ),
            }
        }

        fn apply_shortcut(&mut self, shortcut: ShellTestShortcut) {
            match shortcut {
                ShellTestShortcut::Evaluate => {
                    self.app.evaluate_current_formula();
                    self.app.request_editor_focus();
                }
                ShellTestShortcut::ToggleInspector => {
                    self.app.toggle_support_sidebar();
                    self.app.request_editor_focus();
                }
                ShellTestShortcut::ToggleCapabilityCenter => {
                    self.app.toggle_capability_center();
                    self.app.request_editor_focus();
                }
                ShellTestShortcut::ToggleXRay => {
                    self.app.toggle_xray();
                    self.app.request_editor_focus();
                }
            }
        }

        fn regression_state(&self) -> ExplorerRegressionState {
            self.app.explorer_regression_state()
        }
    }

    fn shell_xray_golden_lines(xray: &ShellXRayModel) -> Vec<String> {
        let mut lines = vec![
            format!("parse.diagnostic_count={}", xray.parse.diagnostic_count),
            format!("bind.current_help={}", xray.bind.current_help_name),
            format!("bind.availability={}", xray.bind.availability_summary),
            format!(
                "provenance.host_profile={}",
                xray.provenance.host_profile_id
            ),
            format!(
                "provenance.packet_register={}",
                xray.provenance.packet_register_text
            ),
            format!(
                "provenance.host_driving_packet_kind={}",
                xray.provenance.latest_host_driving_packet_kind
            ),
            format!(
                "provenance.capability_floor={}",
                xray.provenance.capability_floor
            ),
        ];

        if let Some(evaluation) = &xray.evaluation {
            lines.push(format!(
                "eval.worksheet_value={}",
                evaluation.worksheet_value_summary
            ));
            lines.push(format!("eval.payload={}", evaluation.payload_summary));
            lines.push(format!(
                "eval.effective_display={}",
                evaluation.effective_display_status
            ));
        }

        if let Some(trace) = &xray.trace {
            lines.push(format!("trace.event_count={}", trace.trace_event_count));
            lines.push(format!(
                "trace.commit_decision={}",
                trace.commit_decision_kind
            ));
        }

        lines
    }

    #[test]
    fn shell_app_exposes_the_promoted_shell_regions() {
        assert_eq!(
            OneCalcShellApp::region_ids(),
            &[
                FORMULA_REGION_ID,
                RESULT_REGION_ID,
                DIAGNOSTICS_REGION_ID,
                CAPABILITY_REGION_ID,
                XRAY_REGION_ID,
            ]
        );
    }

    #[test]
    fn shell_app_seeds_result_and_diagnostics_from_runtime_truth() {
        let app = OneCalcShellApp::new(RuntimeAdapter::new(OneCalcHostProfile::OcH0), true);

        assert!(app.editor_state.buffer.contains("SUM"));
        assert!(app.result_text.contains("worksheet_value: Number(6"));
        assert!(app.result_text.contains("payload_summary: Number"));
        assert!(app.result_text.contains("returned_presentation_hint: none"));
        assert!(app.result_text.contains("host_style_state: none"));
        assert!(app.result_text.contains("effective_display: none"));
        assert_eq!(app.effective_display_render.display_text, "6");
        assert_eq!(app.effective_display_render.formatting_plane_source, "none");
        assert!(app
            .diagnostics_text
            .contains("edit_packet_diagnostic_count"));
        assert_eq!(app.host_profile_id, "OC-H0");
        assert!(app.packet_register_text.contains("formula_edit"));
        assert!(app.platform_gate_text.contains("Desktop native host only"));
        assert!(app.function_policy_text.contains("supported="));
        assert!(app
            .function_policy_text
            .contains("executable=supported+preview only"));
        assert!(app
            .conditional_formatting_policy_text
            .contains("Conditional Formatting: admitted="));
        assert!(app
            .conditional_formatting_policy_text
            .contains("blocked=data_bars"));
        assert!(app.function_help.is_some());
        assert_eq!(
            app.editor_state.cursor_index,
            app.editor_state.buffer.chars().count()
        );
        assert!(!app.latest_edit_packet.formula_token.is_empty());
        assert!(app.latest_evaluation.is_some());
        assert_eq!(app.returned_presentation_hint_text, "none");
        assert_eq!(app.host_style_state_text, "none");
        assert!(app.rendered_diagnostics.is_empty());
        assert!(app.capability_center.active_snapshot.is_some());
        assert!(app.capability_center.snapshot_diff.is_some());
    }

    #[test]
    fn shell_app_projects_live_diagnostics_and_spans_for_invalid_formula() {
        let app = OneCalcShellApp::with_formula(
            adapter_for(OneCalcHostProfile::OcH0),
            promoted_formula_scenario(FormulaScenarioFamily::ExplorerInvalid)
                .formula
                .to_string(),
            true,
        );

        assert!(!app.rendered_diagnostics.is_empty());
        assert!(app.rendered_diagnostics[0].contains("Syntax"));
        assert!(app.rendered_diagnostics[0].contains(".."));
    }

    #[test]
    fn shell_app_defaults_capability_center_to_a_supporting_collapsed_surface() {
        let app = OneCalcShellApp::new(RuntimeAdapter::new(OneCalcHostProfile::OcH0), false);

        assert!(app.support_sidebar_visible);
        assert!(!app.capability_center_visible);
        let regression = app.explorer_regression_state();
        assert!(regression.result_visible);
        assert!(regression.inspector_visible);
        assert!(regression.result_scroll_enabled);
        assert!(regression.diagnostics_scroll_enabled);
        assert!(!regression.capability_scroll_enabled);
        assert_eq!(regression.diagnostics_height_fraction, 1.0);
    }

    #[test]
    fn shell_app_keyboard_toggle_helpers_keep_focus_and_visibility_state_coherent() {
        let mut app = OneCalcShellApp::new(RuntimeAdapter::new(OneCalcHostProfile::OcH0), false);

        app.toggle_support_sidebar();
        assert!(!app.support_sidebar_visible);

        app.toggle_capability_center();
        assert!(app.capability_center_visible);
        assert!(app.support_sidebar_visible);

        app.editor_focus_requested = true;
        app.request_editor_focus();
        assert!(!app.editor_focus_requested);

        app.toggle_xray();
        assert!(app.xray_visible);
    }

    #[test]
    fn shell_app_regression_state_keeps_result_visible_when_support_surfaces_open() {
        let mut app = OneCalcShellApp::new(RuntimeAdapter::new(OneCalcHostProfile::OcH0), false);

        app.toggle_capability_center();
        app.toggle_xray();

        let regression = app.explorer_regression_state();
        assert!(regression.result_visible);
        assert!(regression.inspector_visible);
        assert!(regression.capability_center_visible);
        assert!(regression.xray_visible);
        assert!(regression.result_scroll_enabled);
        assert!(regression.diagnostics_scroll_enabled);
        assert!(regression.capability_scroll_enabled);
        assert!(regression.xray_scroll_enabled);
        assert_eq!(
            regression.diagnostics_height_fraction,
            INSPECTOR_DIAGNOSTICS_SPLIT
        );
    }

    #[test]
    fn shell_app_regression_layout_constants_keep_support_surfaces_out_of_the_result_center() {
        assert!(INSPECTOR_DEFAULT_WIDTH >= INSPECTOR_MIN_WIDTH);
        assert!(XRAY_DEFAULT_WIDTH >= XRAY_MIN_WIDTH);
        assert!(INSPECTOR_DIAGNOSTICS_SPLIT > 0.0);
        assert!(INSPECTOR_DIAGNOSTICS_SPLIT < 1.0);
    }

    #[test]
    fn shell_app_projects_structured_xray_model_from_runtime_truth() {
        let app = OneCalcShellApp::new(RuntimeAdapter::new(OneCalcHostProfile::OcH0), true);
        let xray = app.xray_model();

        assert!(!xray.parse.formula_token.is_empty());
        assert_eq!(xray.parse.diagnostic_count, 0);
        assert_eq!(xray.bind.current_help_name, "SUM");
        assert!(xray.bind.availability_summary.contains("CatalogKnown"));

        let evaluation = xray
            .evaluation
            .expect("evaluated formula should project an eval xray section");
        assert!(evaluation.worksheet_value_summary.contains("Number(6"));
        assert_eq!(evaluation.returned_presentation_hint_status, "none");

        let trace = xray
            .trace
            .expect("evaluated formula should project a trace xray section");
        assert!(trace.trace_event_count > 0);
        assert_eq!(xray.provenance.host_profile_id, "OC-H0");
        assert!(xray
            .provenance
            .latest_capability_snapshot_id
            .contains("capability"));
        assert_eq!(
            xray.provenance.latest_host_driving_packet_kind,
            "edit_accept_recalc"
        );
        assert_eq!(xray.provenance.capability_floor, "OC-H0");
        assert!(xray
            .provenance
            .platform_gate_text
            .contains("Desktop native host only"));
    }

    #[test]
    fn shell_app_xray_golden_lines_stay_stable_for_the_promoted_sum_family() {
        let app = OneCalcShellApp::new(RuntimeAdapter::new(OneCalcHostProfile::OcH0), true);
        let xray = app.xray_model();

        assert_eq!(
            shell_xray_golden_lines(&xray),
            vec![
                "parse.diagnostic_count=0".to_string(),
                "bind.current_help=SUM".to_string(),
                "bind.availability=parse_bind=CatalogKnown; semantic_plan=CatalogKnown; runtime=CatalogKnown; post_dispatch=CatalogKnown".to_string(),
                "provenance.host_profile=OC-H0".to_string(),
                "provenance.packet_register=formula_edit, edit_accept_recalc, replay_capture"
                    .to_string(),
                "provenance.host_driving_packet_kind=edit_accept_recalc".to_string(),
                "provenance.capability_floor=OC-H0".to_string(),
                "eval.worksheet_value=Number(6)".to_string(),
                "eval.payload=Number".to_string(),
                "eval.effective_display=none".to_string(),
                "trace.event_count=2".to_string(),
                "trace.commit_decision=accepted".to_string(),
            ]
        );
    }

    #[test]
    fn shell_app_projects_structured_xray_model_for_invalid_formula_without_eval() {
        let app = OneCalcShellApp::with_formula(
            adapter_for(OneCalcHostProfile::OcH0),
            promoted_formula_scenario(FormulaScenarioFamily::ExplorerInvalid)
                .formula
                .to_string(),
            false,
        );
        let xray = app.xray_model();

        assert!(xray.parse.diagnostic_count > 0);
        assert!(xray.evaluation.is_none());
        assert!(xray.trace.is_none());
    }

    #[test]
    fn shell_app_projects_function_completion_into_editor_flow() {
        let app = OneCalcShellApp::with_formula(
            adapter_for(OneCalcHostProfile::OcH0),
            promoted_formula_scenario(FormulaScenarioFamily::ExplorerCompletionStem)
                .formula
                .to_string(),
            false,
        );

        assert!(
            app.completion_items
                .iter()
                .any(|proposal| proposal.proposal_kind == "Function"
                    && proposal.display_text == "SUM")
        );
    }

    #[test]
    fn shell_app_projects_current_function_help_into_editor_flow() {
        let app = OneCalcShellApp::with_formula(
            RuntimeAdapter::new(OneCalcHostProfile::OcH0),
            "=SUM(1,2,3".to_string(),
            false,
        );

        let help = app
            .function_help
            .expect("function help should be available");
        assert_eq!(help.display_name, "SUM");
        assert!(help.display_signature.contains("SUM("));
        assert!(help.availability_summary.contains("CatalogKnown"));
    }

    #[test]
    fn effective_display_render_state_derives_from_the_two_formatting_planes() {
        let scenario =
            promoted_formatting_scenario(FormattingScenarioFamily::CurrencyHintWithHostAccent);
        let summary = FormulaEvaluationSummary {
            formula_token: "token".to_string(),
            worksheet_value_summary: scenario.worksheet_value_summary.to_string(),
            array_preview: None,
            payload_summary: "Number".to_string(),
            returned_value_surface_kind: "OrdinaryValue".to_string(),
            returned_presentation_hint_status: scenario
                .returned_presentation_hint_status
                .to_string(),
            host_style_state_status: scenario.host_style_state_status.to_string(),
            effective_display_status: scenario.effective_display_status.to_string(),
            commit_decision_kind: "accepted".to_string(),
            trace_event_count: 2,
        };

        let render = EffectiveDisplayRenderState::from_summary(&summary);

        assert_eq!(render.display_text, scenario.expected_display_text);
        assert_eq!(
            render.formatting_plane_source,
            "presentation_hint+host_style"
        );
        assert_eq!(render.emphasis, "host:accent");
        assert_eq!(render.number_format, "none");
    }

    #[test]
    fn shell_interaction_harness_covers_keyboard_shortcuts_and_focus_routing() {
        let mut harness =
            ShellInteractionHarness::new(FormulaScenarioFamily::ExplorerSequence23, false);

        assert!(harness.app.result_text.contains("not evaluated yet"));

        harness.apply_shortcut(ShellTestShortcut::Evaluate);
        assert!(harness
            .app
            .result_text
            .contains("worksheet_value: Array(2x3)"));
        assert!(!harness.app.editor_focus_requested);

        harness.apply_shortcut(ShellTestShortcut::ToggleCapabilityCenter);
        assert!(harness.regression_state().capability_center_visible);
        assert!(harness.regression_state().inspector_visible);

        harness.apply_shortcut(ShellTestShortcut::ToggleXRay);
        assert!(harness.regression_state().xray_visible);

        harness.apply_shortcut(ShellTestShortcut::ToggleInspector);
        assert!(!harness.regression_state().inspector_visible);
        assert!(harness.regression_state().result_visible);
    }

    #[test]
    fn shell_interaction_harness_covers_resize_and_scroll_invariants_for_promoted_scenarios() {
        let mut harness =
            ShellInteractionHarness::new(FormulaScenarioFamily::ExplorerInvalid, false);

        harness.apply_shortcut(ShellTestShortcut::ToggleCapabilityCenter);
        harness.apply_shortcut(ShellTestShortcut::ToggleXRay);

        let regression = harness.regression_state();
        assert!(regression.result_visible);
        assert!(regression.inspector_visible);
        assert!(regression.capability_center_visible);
        assert!(regression.xray_visible);
        assert!(regression.result_scroll_enabled);
        assert!(regression.diagnostics_scroll_enabled);
        assert!(regression.capability_scroll_enabled);
        assert!(regression.xray_scroll_enabled);
        assert_eq!(
            regression.diagnostics_height_fraction,
            INSPECTOR_DIAGNOSTICS_SPLIT
        );
        assert!(!harness.app.rendered_diagnostics.is_empty());
    }

    #[test]
    fn shell_app_projects_array_results_into_the_main_result_surface_state() {
        let app = OneCalcShellApp::with_formula(
            adapter_for(OneCalcHostProfile::OcH0),
            promoted_formula_scenario(FormulaScenarioFamily::ExplorerSequence23)
                .formula
                .to_string(),
            true,
        );

        let preview = app
            .latest_evaluation
            .as_ref()
            .and_then(|summary| summary.array_preview.as_ref())
            .expect("array formulas should produce a preview");
        assert_eq!(preview.row_count, 2);
        assert_eq!(preview.column_count, 3);
        assert_eq!(
            preview.rows,
            vec![
                vec!["1".to_string(), "2".to_string(), "3".to_string()],
                vec!["4".to_string(), "5".to_string(), "6".to_string()],
            ]
        );
        assert!(app.result_text.contains("worksheet_value: Array(2x3)"));
    }

    #[test]
    fn array_preview_overflow_note_makes_bounded_rendering_explicit() {
        let preview = ArrayPreviewSummary {
            row_count: 8,
            column_count: 7,
            rows: vec![vec!["1".to_string(); 6]; 6],
            hidden_row_count: 2,
            hidden_column_count: 1,
        };

        assert_eq!(
            array_preview_overflow_note(&preview),
            Some("showing 6 x 6; 2 more row(s), 1 more column(s) not shown".to_string())
        );
    }
}
