//! dnacalc-bridge — the formula workbench (SHELL_SPEC.md §2/§6, v0.9
//! owner-ratified; BENCH_SPEC.md §3/§4 for the surfaces it renders).
//!
//! One component family, two modes, both products:
//!
//! - [`FormulaBridge`] — full fidelity (Bench today, every Calc context at
//!   G1): token-lit editor from `FormulaEditorSurface.syntax_runs`, staged
//!   diagnostics, px-anchored completion with all 7 kinds, signature/function
//!   help panes, readout row, X-Ray affordance.
//! - [`FormulaBridgeDegrade`] — the honest pre-G1 Calc degrade: plain-text
//!   editing + rejection spans + optional dry-bind preview. No fake token
//!   colors (tested — zero token-role classes in its DOM).
//!
//! ## Layering laws (tested, not aspirational)
//!
//! - **The bridge never tokenizes or inspects formula text.** No `=`
//!   sniffing anywhere; pasted input reaches [`BridgeEvent::TextEdited`]
//!   byte-for-byte verbatim. All coloring/underlining comes from host spans.
//! - **Emission model:** semantic [`BridgeEvent`]s through a [`BridgeEvents`]
//!   callback only. The bridge never constructs `WorkspaceIntent` /
//!   `OneFormulaIntent` — host adapters do. That seam is what lets one
//!   component serve the Bench host and the Calc degrade path.
//! - **Dependency law (SHELL_SPEC §2):** `dnacalc-skin-ir` +
//!   `dnacalc-strand` + Leptos only. P-gate: no `ox*` crate in this graph.
//! - **Style law:** every color and spacing value resolves through
//!   `--dna-*` strand tokens (see [`bridge_css`]); reduced motion is honored
//!   through strand's [`REDUCED_MOTION_FLAG`] convention; editing is
//!   modeless 1-bit ([`EditDiscipline`]: SELECTED vs EDITING).

pub mod assist;
pub mod degrade;
pub mod diagnostics;
pub mod editor;
pub mod events;
pub mod readout;
pub mod vm;

pub use assist::{CompletionPopup, FunctionHelpPane, SignaturePane};
pub use degrade::{DegradePreviewBinding, DryBindList, FormulaBridgeDegrade, RejectionList};
pub use diagnostics::DiagnosticsList;
pub use editor::FormulaBridge;
pub use events::{BridgeEvent, BridgeEvents, CommitAdvance, EditDiscipline};
pub use vm::{
    ALL_COMPLETION_KINDS, DegradeKeyDisposition, MAX_WINDOW_EDGE, PartialEvalVm, ReadoutVm,
    RenderSegment, buffer_is_dirty, completion_applied, completion_kind_class,
    completion_kind_glyph, completion_kind_id, completion_next, degrade_buffer_is_dirty,
    degrade_key_disposition, degrade_segments, diagnostic_row, drill_node_at_caret,
    drill_node_for_selection, drill_state_class, drill_state_id, drill_state_label,
    dry_bind_diagnostics, dry_bind_preview, editor_segments, is_stale, is_undo_redo_chord,
    next_preview_window, partial_eval, readout, role_class, role_id, segment_lit_by_caret,
    segments_snapshot, segments_text, selection_from_dom, severity_class, severity_id,
    severity_label, shape_label, should_consume_undo_redo_locally, snap_to_char_boundary,
    stage_label, text_edited_from_dom, utf8_to_utf16, utf16_to_utf8,
};

use dnacalc_strand::REDUCED_MOTION_FLAG;

/// Component-scoped styles, colors and spacing strictly through `--dna-*`
/// strand custom properties (emitted by `dnacalc_strand::css_custom_properties`
/// on the shell root — this crate never hardcodes a hex value; tested).
///
/// Token-role ink assignments follow STRAND §2's "every color has a job":
/// identifiers (references/structure) take `accent-ink` (teal's stated job),
/// numbers take `value-ink`, string literals take `prov-const` (typed
/// constants), functions are bold plain ink, operators/delimiters/trivia are
/// the neutral ink ramp. Diagnostic underlines: error = wavy `red-ink`,
/// warning = wavy `prov-ext` (S0 compromise: strand has no dedicated
/// warn-ink yet; prov-ext is the contrast-lawful orange), info = dotted
/// `accent-ink` — wavy/dotted is the non-color cue.
const BRIDGE_CSS_BODY: &str = "\
.dna-bridge{position:relative;display:flex;flex-direction:column;gap:var(--dna-gap-3);font-family:'Recursive','Segoe UI Variable',system-ui,sans-serif;color:var(--dna-ink);background:var(--dna-paper);border-radius:var(--dna-radius-card);padding:var(--dna-gap-4)}
.dna-bridge__editor{position:relative;border:1px solid var(--dna-line);border-radius:var(--dna-radius-chip);background:var(--dna-paper)}
.dna-bridge[data-edit-state=\"editing\"] .dna-bridge__editor{border-color:var(--dna-accent);box-shadow:0 0 0 1px var(--dna-accent)}
.dna-bridge__tokens,.dna-bridge__input{box-sizing:border-box;font-family:'Recursive Mono','Cascadia Code',Consolas,ui-monospace,monospace;font-size:14px;line-height:var(--dna-line-height);padding:var(--dna-gap-3) var(--dna-gap-4);margin:0;border:0;white-space:pre-wrap;overflow-wrap:anywhere;text-align:left}
.dna-bridge__tokens{min-height:1.5em;color:var(--dna-ink)}
.dna-bridge__input{position:absolute;inset:0;width:100%;height:100%;resize:none;background:transparent;color:transparent;caret-color:var(--dna-accent);outline:none}
.dna-bridge__input::selection{background:var(--dna-accent-soft)}
.dna-bridge__editor--pending-echo .dna-bridge__tokens{visibility:hidden}
.dna-bridge__editor--pending-echo .dna-bridge__input{color:var(--dna-ink)}
.dna-bridge__seg--role-operator{color:var(--dna-ink-2)}
.dna-bridge__seg--role-function{color:var(--dna-ink);font-weight:600}
.dna-bridge__seg--role-number{color:var(--dna-value-ink)}
.dna-bridge__seg--role-delimiter{color:var(--dna-ink-3)}
.dna-bridge__seg--role-identifier{color:var(--dna-accent-ink)}
.dna-bridge__seg--role-text{color:var(--dna-prov-const)}
.dna-bridge__seg--role-trivia{color:var(--dna-ink-3)}
.dna-bridge__seg--caret{background:var(--dna-accent-soft);border-radius:2px}
.dna-bridge__seg--diag-error{text-decoration:underline wavy var(--dna-red-ink);text-underline-offset:3px}
.dna-bridge__seg--diag-warning{text-decoration:underline wavy var(--dna-prov-ext);text-underline-offset:3px}
.dna-bridge__seg--diag-info{text-decoration:underline dotted var(--dna-accent-ink);text-underline-offset:3px}
.dna-bridge__stale{align-self:flex-start;background:var(--dna-amber-soft);color:var(--dna-ink);border:1px solid var(--dna-line);border-radius:var(--dna-radius-chip);padding:0 var(--dna-gap-3);font-size:11px}
.dna-bridge__readout{display:flex;flex-wrap:wrap;align-items:baseline;gap:var(--dna-gap-4);color:var(--dna-ink-2);font-size:12px}
.dna-bridge__readout-callee{color:var(--dna-accent-ink);font-weight:600}
.dna-bridge__readout-arg{color:var(--dna-ink-2)}
.dna-bridge__readout-target{color:var(--dna-accent-ink)}
.dna-bridge__readout-shape{color:var(--dna-ink-3)}
.dna-bridge__readout-value{color:var(--dna-value-ink)}
.dna-bridge__readout-pager{font:inherit;color:var(--dna-accent-ink);background:var(--dna-paper-2);border:1px solid var(--dna-line);border-radius:var(--dna-radius-chip);padding:0 var(--dna-gap-3);cursor:pointer}
.dna-bridge__completion{position:absolute;z-index:10;list-style:none;margin:0;padding:var(--dna-gap-1);background:var(--dna-paper);border:1px solid var(--dna-line);border-radius:var(--dna-radius-card);box-shadow:0 4px 14px var(--dna-line-soft);transition:opacity var(--dna-motion-min) ease-out}
.dna-bridge__completion-item{display:flex;gap:var(--dna-gap-3);align-items:baseline;padding:var(--dna-gap-1) var(--dna-gap-3);border-radius:var(--dna-radius-chip);cursor:pointer}
.dna-bridge__completion-item--active{background:var(--dna-accent-soft)}
.dna-bridge__completion-glyph{width:1.4em;text-align:center;font-weight:600}
.dna-bridge__completion-item--kind-function .dna-bridge__completion-glyph{color:var(--dna-accent-ink)}
.dna-bridge__completion-item--kind-defined_name .dna-bridge__completion-glyph{color:var(--dna-prov-const)}
.dna-bridge__completion-item--kind-table_name .dna-bridge__completion-glyph{color:var(--dna-value-ink)}
.dna-bridge__completion-item--kind-table_column .dna-bridge__completion-glyph{color:var(--dna-ink)}
.dna-bridge__completion-item--kind-structured_selector .dna-bridge__completion-glyph{color:var(--dna-ink-2)}
.dna-bridge__completion-item--kind-syntax_assist .dna-bridge__completion-glyph{color:var(--dna-ink-3)}
.dna-bridge__completion-item--kind-profile_reference .dna-bridge__completion-glyph{color:var(--dna-prov-ext)}
.dna-bridge__completion-doc{color:var(--dna-ink-3);font-size:11px}
.dna-bridge__signature{color:var(--dna-ink-2);font-size:12px;font-family:'Recursive Mono','Cascadia Code',Consolas,ui-monospace,monospace}
.dna-bridge__signature-callee{color:var(--dna-accent-ink);font-weight:600}
.dna-bridge__signature-param--active{color:var(--dna-accent-ink);font-weight:600}
.dna-bridge__function-help{display:flex;flex-direction:column;gap:var(--dna-gap-1);padding:var(--dna-gap-3);background:var(--dna-paper-2);border-radius:var(--dna-radius-chip);font-size:12px}
.dna-bridge__function-help-signature{color:var(--dna-ink-2)}
.dna-bridge__function-help-description{margin:0;color:var(--dna-ink-2)}
.dna-bridge__function-help-availability{color:var(--dna-ink-3)}
.dna-bridge__function-help-limited{color:var(--dna-prov-ext)}
.dna-bridge__diagnostics,.dna-bridge__rejections,.dna-bridge__dry-bind{list-style:none;margin:0;padding:0;display:flex;flex-direction:column;gap:var(--dna-gap-1);font-size:12px}
.dna-bridge__diagnostic{display:flex;align-items:baseline;gap:var(--dna-gap-3);padding:var(--dna-gap-1) var(--dna-gap-3);border-radius:var(--dna-radius-chip);background:var(--dna-paper-2)}
.dna-bridge__diagnostic--error .dna-bridge__diagnostic-severity{color:var(--dna-red-ink);font-weight:600}
.dna-bridge__diagnostic--warning .dna-bridge__diagnostic-severity{color:var(--dna-prov-ext);font-weight:600}
.dna-bridge__diagnostic--info .dna-bridge__diagnostic-severity{color:var(--dna-accent-ink);font-weight:600}
.dna-bridge__diagnostic-stage{color:var(--dna-ink-3)}
.dna-bridge__diagnostic-code{color:var(--dna-ink-3);font-family:'Recursive Mono','Cascadia Code',Consolas,ui-monospace,monospace}
.dna-bridge__rejection{display:flex;align-items:baseline;gap:var(--dna-gap-3);padding:var(--dna-gap-1) var(--dna-gap-3);border-radius:var(--dna-radius-chip);background:var(--dna-red-soft);color:var(--dna-red-ink)}
.dna-bridge__rejection-span{color:var(--dna-ink-3);font-family:'Recursive Mono','Cascadia Code',Consolas,ui-monospace,monospace;font-size:11px}
.dna-bridge__dry-bind-row{display:flex;align-items:baseline;gap:var(--dna-gap-3);padding:var(--dna-gap-1) var(--dna-gap-3);border-radius:var(--dna-radius-chip);background:var(--dna-signal-soft)}
.dna-bridge__dry-bind-stage{color:var(--dna-prov-ext);font-weight:600}
.dna-bridge__dry-bind-span{color:var(--dna-ink-3);font-size:11px}
.dna-bridge__drill-toggle{align-self:flex-start;font:inherit;font-size:12px;color:var(--dna-accent-ink);background:var(--dna-paper-2);border:1px solid var(--dna-line);border-radius:var(--dna-radius-chip);padding:0 var(--dna-gap-3);cursor:pointer}
.dna-bridge__pill{position:absolute;z-index:12;top:calc(-1 * var(--dna-gap-5));left:var(--dna-gap-3);display:flex;align-items:baseline;gap:var(--dna-gap-3);background:var(--dna-paper);border:1px solid var(--dna-accent);border-radius:var(--dna-radius-chip);box-shadow:0 4px 14px var(--dna-line-soft);padding:var(--dna-gap-1) var(--dna-gap-3);font-size:12px;max-width:100%;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.dna-bridge__pill-expr{font-family:'Recursive Mono','Cascadia Code',Consolas,ui-monospace,monospace;color:var(--dna-accent-ink);font-weight:600}
.dna-bridge__pill-type{color:var(--dna-ink-3);text-transform:lowercase}
.dna-bridge__pill-shape{color:var(--dna-ink-3)}
.dna-bridge__pill-value{color:var(--dna-value-ink);font-weight:600}
.dna-bridge__pill-chip{font-size:10px;text-transform:uppercase;letter-spacing:0.04em;border-radius:var(--dna-radius-chip);padding:0 var(--dna-gap-2);background:var(--dna-paper-2);color:var(--dna-ink-2)}
.dna-bridge__pill-chip[data-state=\"error\"],.dna-bridge__pill-chip[data-state=\"blocked\"]{background:var(--dna-red-soft);color:var(--dna-red-ink)}
.dna-bridge__pill-chip[data-state=\"evaluated\"]{background:var(--dna-accent-soft);color:var(--dna-accent-ink)}
";

/// The bridge's component-scoped stylesheet. Assembled at call time so the
/// reduced-motion kill switch is built from strand's own
/// [`REDUCED_MOTION_FLAG`] constant (no string drift): shells that detect
/// `prefers-reduced-motion: reduce` set the flag as a `data-*` attribute or
/// class on an ancestor, and every bridge transition/animation dies.
#[must_use]
pub fn bridge_css() -> String {
    format!(
        "{BRIDGE_CSS_BODY}\
[data-{flag}] .dna-bridge *,.{flag} .dna-bridge *{{transition:none !important;animation:none !important}}\n",
        flag = REDUCED_MOTION_FLAG
    )
}
