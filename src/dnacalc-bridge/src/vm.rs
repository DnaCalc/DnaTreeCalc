//! Pure view-model layer: everything the components render is computed here,
//! natively testable, with no DOM in sight.
//!
//! Two conventions rule this module:
//!
//! - **Spans are UTF-8 byte offsets** into the source text (the skin-IR
//!   convention). The DOM's selection API speaks UTF-16 code units, so
//!   [`utf8_to_utf16`] / [`utf16_to_utf8`] are the only two functions allowed
//!   to cross that boundary.
//! - **The bridge never tokenizes.** Segmentation below only *slices* the
//!   source text at span boundaries the host supplied (syntax runs,
//!   diagnostics, rejections). No character of the text is ever inspected
//!   for meaning — there is no `=` sniffing anywhere in this crate.

use std::collections::BTreeSet;

use dnacalc_skin_ir::formula::{
    ArrayPreviewProjection, CompletionItemProjection, CompletionKindProjection,
    DiagnosticSeverityProjection, DiagnosticStageProjection, FormulaAssistSurface,
    FormulaDiagnosticProjection, FormulaDrillNodeProjection, FormulaDrillNodeStateProjection,
    FormulaDrillSurface, FormulaEditorSurface, SyntaxTokenRoleProjection,
};
use dnacalc_skin_ir::identity::NodeId;
use dnacalc_skin_ir::preview::PreviewService;
use dnacalc_skin_ir::workspace::{
    FormulaBindPreviewDiagnosticProjection, FormulaBindPreviewProjection,
    GridEntryDiagnosticProjection,
};

use crate::events::BridgeEvent;

/// The skin-IR array-window cap the bridge's paging requests honor
/// (`MAX_ARRAY_WINDOW_CELLS` = 4096 = 64×64).
pub const MAX_WINDOW_EDGE: usize = 64;

// ---------------------------------------------------------------------------
// UTF-8 <-> UTF-16 offset mapping
// ---------------------------------------------------------------------------

/// Snap a byte offset down to the nearest UTF-8 character boundary of `text`
/// (defensive: IR spans should already be boundaries; a hostile or drifted
/// span must degrade to a valid slice, never a panic).
#[must_use]
pub fn snap_to_char_boundary(text: &str, offset: usize) -> usize {
    let mut offset = offset.min(text.len());
    while offset > 0 && !text.is_char_boundary(offset) {
        offset -= 1;
    }
    offset
}

/// Convert a UTF-8 byte offset into `text` to the DOM's UTF-16 code-unit
/// offset. Non-boundary inputs snap down first.
#[must_use]
pub fn utf8_to_utf16(text: &str, byte_offset: usize) -> u32 {
    let boundary = snap_to_char_boundary(text, byte_offset);
    u32::try_from(text[..boundary].encode_utf16().count()).unwrap_or(u32::MAX)
}

/// Convert a DOM UTF-16 code-unit offset to a UTF-8 byte offset into `text`.
/// An offset landing inside a surrogate pair snaps forward to the next
/// character boundary; offsets past the end clamp to `text.len()`.
#[must_use]
pub fn utf16_to_utf8(text: &str, utf16_offset: u32) -> usize {
    let mut units: u32 = 0;
    for (byte_index, ch) in text.char_indices() {
        if units >= utf16_offset {
            return byte_index;
        }
        units += u32::try_from(ch.len_utf16()).unwrap_or(u32::MAX);
    }
    text.len()
}

// ---------------------------------------------------------------------------
// Render segments (token underlay + diagnostic underlines)
// ---------------------------------------------------------------------------

/// One contiguous slice of the source text with at most one token role and
/// at most one (strongest) diagnostic severity. Segments concatenate back to
/// the source text byte-for-byte — see `segments_text`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderSegment {
    pub text: String,
    pub byte_start: usize,
    pub byte_end: usize,
    /// `None` = plain ink. Degrade-mode segments are ALWAYS `None` — no fake
    /// token colors (SHELL_SPEC §6).
    pub role: Option<SyntaxTokenRoleProjection>,
    /// Strongest severity of any diagnostic span covering this segment.
    pub diag: Option<DiagnosticSeverityProjection>,
}

fn severity_rank(severity: DiagnosticSeverityProjection) -> u8 {
    match severity {
        DiagnosticSeverityProjection::Error => 2,
        DiagnosticSeverityProjection::Warning => 1,
        DiagnosticSeverityProjection::Info => 0,
    }
}

/// Normalize a host span to a non-empty, boundary-snapped `[start, end)`
/// range inside `text`, or `None` when it collapses.
fn normalize_span(text: &str, start: usize, end: usize) -> Option<(usize, usize)> {
    let start = snap_to_char_boundary(text, start);
    let end = snap_to_char_boundary(text, end);
    (start < end).then_some((start, end))
}

fn build_segments(
    text: &str,
    roles: &[(usize, usize, SyntaxTokenRoleProjection)],
    marks: &[(usize, usize, DiagnosticSeverityProjection)],
) -> Vec<RenderSegment> {
    let mut cuts: BTreeSet<usize> = BTreeSet::from([0, text.len()]);
    for &(start, end, _) in roles {
        cuts.insert(start);
        cuts.insert(end);
    }
    for &(start, end, _) in marks {
        cuts.insert(start);
        cuts.insert(end);
    }
    let cuts: Vec<usize> = cuts.into_iter().collect();
    let mut segments = Vec::new();
    for window in cuts.windows(2) {
        let (start, end) = (window[0], window[1]);
        if start >= end {
            continue;
        }
        let role = roles
            .iter()
            .find(|&&(s, e, _)| s <= start && start < e)
            .map(|&(_, _, role)| role);
        let diag = marks
            .iter()
            .filter(|&&(s, e, _)| s <= start && start < e)
            .map(|&(_, _, severity)| severity)
            .max_by_key(|severity| severity_rank(*severity));
        segments.push(RenderSegment {
            text: text[start..end].to_string(),
            byte_start: start,
            byte_end: end,
            role,
            diag,
        });
    }
    segments
}

/// Full-mode segmentation: slice the surface's source text at the host's
/// syntax-run and diagnostic span boundaries. Runs are trusted verbatim —
/// the slice offsets are authoritative, the run's own `text` is never used
/// to re-derive anything.
#[must_use]
pub fn editor_segments(surface: &FormulaEditorSurface) -> Vec<RenderSegment> {
    let text = surface.source_text.as_str();
    let roles: Vec<(usize, usize, SyntaxTokenRoleProjection)> = surface
        .syntax_runs
        .iter()
        .filter_map(|run| {
            let end = run.span_start.checked_add(run.span_len)?;
            let (start, end) = normalize_span(text, run.span_start, end)?;
            Some((start, end, run.role))
        })
        .collect();
    let marks: Vec<(usize, usize, DiagnosticSeverityProjection)> = surface
        .diagnostics
        .iter()
        .filter_map(|diag| {
            let end = diag.span_start.checked_add(diag.span_len)?;
            let (start, end) = normalize_span(text, diag.span_start, end)?;
            Some((start, end, diag.severity))
        })
        .collect();
    build_segments(text, &roles, &marks)
}

/// Degrade-mode segmentation (SHELL_SPEC §6, pre-G1 Calc contexts): plain
/// text + rejection spans (`Error` marks) + optional dry-bind prediction
/// spans (`Warning` marks, deliberately weaker than a hard rejection so the
/// two read differently). `role` is `None` on every segment — zero token
/// colors, honestly.
///
/// Rejection spans are `(start, end)` UTF-8 byte tuples; a rejection with
/// `span = None` produces no underline (it renders message-only in the
/// rejection list, per the skin-IR doc on `GridEntryDiagnosticProjection`).
#[must_use]
pub fn degrade_segments(
    text: &str,
    rejections: &[GridEntryDiagnosticProjection],
    dry_bind: &[FormulaBindPreviewDiagnosticProjection],
) -> Vec<RenderSegment> {
    let mut marks: Vec<(usize, usize, DiagnosticSeverityProjection)> = Vec::new();
    for rejection in rejections {
        if let Some((start, end)) = rejection.span
            && let Some(span) = normalize_span(text, start as usize, end as usize)
        {
            marks.push((span.0, span.1, DiagnosticSeverityProjection::Error));
        }
    }
    for diagnostic in dry_bind {
        if let Some(span) =
            normalize_span(text, diagnostic.span.start_utf8, diagnostic.span.end_utf8)
        {
            marks.push((span.0, span.1, DiagnosticSeverityProjection::Warning));
        }
    }
    build_segments(text, &[], &marks)
}

/// Reassemble segments into text. Identity with the source is a tested law:
/// the underlay renders exactly the host's text, never a re-derivation.
#[must_use]
pub fn segments_text(segments: &[RenderSegment]) -> String {
    segments.iter().map(|s| s.text.as_str()).collect()
}

/// Debug/spanshot rendering used by tests: `[text|role]` per segment, with
/// `!severity` appended when a diagnostic covers it.
#[must_use]
pub fn segments_snapshot(segments: &[RenderSegment]) -> String {
    segments
        .iter()
        .map(|s| {
            let role = s.role.map_or("plain", role_id);
            let diag = s
                .diag
                .map(|d| format!("!{}", severity_id(d)))
                .unwrap_or_default();
            format!("[{}|{}{}]", s.text, role, diag)
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Class / label vocabulary (all strand-token-backed in bridge_css)
// ---------------------------------------------------------------------------

/// Stable id for a token role (used in class names and test snapshots).
#[must_use]
pub const fn role_id(role: SyntaxTokenRoleProjection) -> &'static str {
    match role {
        SyntaxTokenRoleProjection::Operator => "operator",
        SyntaxTokenRoleProjection::Function => "function",
        SyntaxTokenRoleProjection::Number => "number",
        SyntaxTokenRoleProjection::Delimiter => "delimiter",
        SyntaxTokenRoleProjection::Identifier => "identifier",
        SyntaxTokenRoleProjection::Text => "text",
        SyntaxTokenRoleProjection::Trivia => "trivia",
    }
}

/// The token-role class for a segment. The `--role-` infix is the marker the
/// degrade-mode tests grep for: degrade DOM contains zero of these.
#[must_use]
pub fn role_class(role: SyntaxTokenRoleProjection) -> String {
    format!("dna-bridge__seg--role-{}", role_id(role))
}

/// Stable id for a severity (class names, snapshots).
#[must_use]
pub const fn severity_id(severity: DiagnosticSeverityProjection) -> &'static str {
    match severity {
        DiagnosticSeverityProjection::Error => "error",
        DiagnosticSeverityProjection::Warning => "warning",
        DiagnosticSeverityProjection::Info => "info",
    }
}

/// Underline class for a segment covered by a diagnostic span.
#[must_use]
pub fn severity_class(severity: DiagnosticSeverityProjection) -> String {
    format!("dna-bridge__seg--diag-{}", severity_id(severity))
}

/// Severity label rendered verbatim from the IR variant (Error/Warning/Info).
#[must_use]
pub const fn severity_label(severity: DiagnosticSeverityProjection) -> &'static str {
    match severity {
        DiagnosticSeverityProjection::Error => "Error",
        DiagnosticSeverityProjection::Warning => "Warning",
        DiagnosticSeverityProjection::Info => "Info",
    }
}

/// Stage label rendered verbatim from the IR variant (Syntax/Bind/
/// SemanticPlan) — the bridge never reclassifies stages.
#[must_use]
pub const fn stage_label(stage: DiagnosticStageProjection) -> &'static str {
    match stage {
        DiagnosticStageProjection::Syntax => "Syntax",
        DiagnosticStageProjection::Bind => "Bind",
        DiagnosticStageProjection::SemanticPlan => "SemanticPlan",
    }
}

// ---------------------------------------------------------------------------
// Completion
// ---------------------------------------------------------------------------

/// Stable id for a completion kind (matches the IR's serde snake_case).
#[must_use]
pub const fn completion_kind_id(kind: CompletionKindProjection) -> &'static str {
    match kind {
        CompletionKindProjection::Function => "function",
        CompletionKindProjection::DefinedName => "defined_name",
        CompletionKindProjection::TableName => "table_name",
        CompletionKindProjection::TableColumn => "table_column",
        CompletionKindProjection::StructuredSelector => "structured_selector",
        CompletionKindProjection::SyntaxAssist => "syntax_assist",
        CompletionKindProjection::ProfileReference => "profile_reference",
    }
}

/// Per-kind item class — all 7 kinds render distinctly (distinct class,
/// distinct glyph, distinct glyph ink in `bridge_css`).
#[must_use]
pub fn completion_kind_class(kind: CompletionKindProjection) -> String {
    format!(
        "dna-bridge__completion-item--kind-{}",
        completion_kind_id(kind)
    )
}

/// Per-kind glyph — the non-color cue (STRAND a11y doctrine: color is never
/// the sole indicator).
#[must_use]
pub const fn completion_kind_glyph(kind: CompletionKindProjection) -> &'static str {
    match kind {
        CompletionKindProjection::Function => "\u{0192}", // ƒ
        CompletionKindProjection::DefinedName => "\u{25C6}", // ◆
        CompletionKindProjection::TableName => "\u{25A6}", // ▦
        CompletionKindProjection::TableColumn => "\u{25A5}", // ▥
        CompletionKindProjection::StructuredSelector => "\u{00A7}", // §
        CompletionKindProjection::SyntaxAssist => "\u{270E}", // ✎
        CompletionKindProjection::ProfileReference => "\u{25C8}", // ◈
    }
}

/// All seven kinds, for exhaustive rendering/uniqueness tests.
pub const ALL_COMPLETION_KINDS: [CompletionKindProjection; 7] = [
    CompletionKindProjection::Function,
    CompletionKindProjection::DefinedName,
    CompletionKindProjection::TableName,
    CompletionKindProjection::TableColumn,
    CompletionKindProjection::StructuredSelector,
    CompletionKindProjection::SyntaxAssist,
    CompletionKindProjection::ProfileReference,
];

/// Keyboard navigation over the popup: wraps at both ends; empty lists pin
/// to 0.
#[must_use]
pub fn completion_next(item_count: usize, current: usize, delta: i32) -> usize {
    if item_count == 0 {
        return 0;
    }
    let len = item_count as i64;
    let current = current.min(item_count - 1) as i64;
    let next = (current + i64::from(delta)).rem_euclid(len);
    usize::try_from(next).unwrap_or(0)
}

/// The apply event for the highlighted item: carries the host's
/// `proposal_id` untouched (never the display text — the host owns the edit).
#[must_use]
pub fn completion_applied(items: &[CompletionItemProjection], index: usize) -> Option<BridgeEvent> {
    items.get(index).map(|item| BridgeEvent::CompletionApplied {
        proposal_id: item.proposal_id.clone(),
    })
}

// ---------------------------------------------------------------------------
// DOM edit/selection -> events (verbatim passthrough)
// ---------------------------------------------------------------------------

/// Build the edit event from the DOM's state: `text` passes through
/// byte-for-byte verbatim (tested law — pastes, `=`-prefixed or not, arrive
/// unmodified); only the caret is converted from UTF-16 units to UTF-8 bytes.
#[must_use]
pub fn text_edited_from_dom(text: String, caret_utf16: u32) -> BridgeEvent {
    let caret = utf16_to_utf8(&text, caret_utf16);
    BridgeEvent::TextEdited { text, caret }
}

/// Build the selection event from DOM selection state. The DOM reports
/// `[start, end]` plus a direction flag; the IR wants `anchor`/`focus`.
#[must_use]
pub fn selection_from_dom(
    text: &str,
    start_utf16: u32,
    end_utf16: u32,
    backward: bool,
) -> BridgeEvent {
    let start = utf16_to_utf8(text, start_utf16);
    let end = utf16_to_utf8(text, end_utf16);
    let (anchor, focus) = if backward { (end, start) } else { (start, end) };
    BridgeEvent::SelectionSet { anchor, focus }
}

// ---------------------------------------------------------------------------
// Readout row
// ---------------------------------------------------------------------------

/// What the readout row shows: resolved-target summary when the surfaces
/// provide it (BENCH_SPEC §2's `Rates[ISO] → 21×1 text · caret at arg 2`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReadoutVm {
    pub callee: Option<String>,
    pub active_parameter: Option<String>,
    pub target_label: Option<String>,
    pub value_preview: Option<String>,
    pub shape: Option<String>,
}

impl ReadoutVm {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.callee.is_none()
            && self.active_parameter.is_none()
            && self.target_label.is_none()
            && self.value_preview.is_none()
            && self.shape.is_none()
    }
}

/// `rows×cols` with a real multiplication sign.
#[must_use]
pub fn shape_label(rows: usize, cols: usize) -> String {
    format!("{rows}\u{00D7}{cols}")
}

/// Deepest drill node whose source span contains the caret (UTF-8 byte
/// offset). Span-less wrapper nodes are traversed; the first containing
/// subtree wins among siblings.
#[must_use]
pub fn drill_node_at_caret(
    nodes: &[FormulaDrillNodeProjection],
    caret: usize,
) -> Option<&FormulaDrillNodeProjection> {
    for node in nodes {
        let contains = matches!(
            (node.source_span_start, node.source_span_len),
            (Some(start), Some(len)) if start <= caret && caret < start.saturating_add(len)
        );
        if let Some(hit) = drill_node_at_caret(&node.children, caret) {
            return Some(hit);
        }
        if contains {
            return Some(node);
        }
    }
    None
}

/// Assemble the readout: callee + active parameter from signature help;
/// target label / value preview / shape from the drill node under the caret,
/// only when the drill is expanded (drill data is host truth, not a guess).
#[must_use]
pub fn readout(
    assist: &FormulaAssistSurface,
    drill: &FormulaDrillSurface,
    caret: usize,
) -> ReadoutVm {
    let signature = assist.signature_help.as_ref();
    let callee = signature.map(|s| s.callee_text.clone());
    let active_parameter = signature.and_then(|s| {
        s.active_parameter
            .and_then(|index| s.parameters.get(index))
            .or_else(|| s.parameters.iter().find(|p| p.is_active))
            .map(|p| p.name.clone())
    });
    let node = if drill.expanded {
        drill_node_at_caret(&drill.tree, caret)
    } else {
        None
    };
    let target_label = node.map(|n| n.expression_text.clone().unwrap_or_else(|| n.label.clone()));
    let value_preview = node.and_then(|n| n.value_preview.clone());
    let shape = node
        .and_then(|n| n.array_preview.as_ref())
        .map(|p| shape_label(p.total_rows, p.total_cols));
    ReadoutVm {
        callee,
        active_parameter,
        target_label,
        value_preview,
        shape,
    }
}

/// The next-window request for a truncated array preview, honoring the
/// 64×64 skin-IR cap: row-major (finish rows, then advance columns), `None`
/// when the preview already covers the array.
#[must_use]
pub fn next_preview_window(preview: &ArrayPreviewProjection) -> Option<BridgeEvent> {
    let visible_rows = preview.rows.len();
    let visible_cols = preview.rows.first().map_or(0, Vec::len);
    let rows_remain = preview.row_offset.saturating_add(visible_rows) < preview.total_rows;
    let cols_remain = preview.col_offset.saturating_add(visible_cols) < preview.total_cols;
    if !rows_remain && !cols_remain {
        return None;
    }
    let (row_offset, col_offset) = if rows_remain {
        (preview.row_offset + visible_rows, preview.col_offset)
    } else {
        (0, preview.col_offset.saturating_add(visible_cols))
    };
    Some(BridgeEvent::ArrayWindowRequested {
        row_offset,
        col_offset,
        row_count: preview
            .total_rows
            .saturating_sub(row_offset)
            .clamp(1, MAX_WINDOW_EDGE),
        col_count: preview
            .total_cols
            .saturating_sub(col_offset)
            .clamp(1, MAX_WINDOW_EDGE),
    })
}

// ---------------------------------------------------------------------------
// Drill-node state vocabulary (X-Ray chips — BENCH_SPEC §4)
// ---------------------------------------------------------------------------

/// Stable id for a drill-node state (class names, test snapshots) — verbatim
/// from the IR variant; the bridge never reclassifies a node's disposition.
#[must_use]
pub const fn drill_state_id(state: FormulaDrillNodeStateProjection) -> &'static str {
    match state {
        FormulaDrillNodeStateProjection::Pending => "pending",
        FormulaDrillNodeStateProjection::Evaluated => "evaluated",
        FormulaDrillNodeStateProjection::Bound => "bound",
        FormulaDrillNodeStateProjection::Skipped => "skipped",
        FormulaDrillNodeStateProjection::Opaque => "opaque",
        FormulaDrillNodeStateProjection::Blocked => "blocked",
        FormulaDrillNodeStateProjection::Error => "error",
    }
}

/// Human label for a drill-node state chip (BENCH_SPEC §4 vocabulary).
#[must_use]
pub const fn drill_state_label(state: FormulaDrillNodeStateProjection) -> &'static str {
    match state {
        FormulaDrillNodeStateProjection::Pending => "Pending",
        FormulaDrillNodeStateProjection::Evaluated => "Evaluated",
        FormulaDrillNodeStateProjection::Bound => "Bound",
        FormulaDrillNodeStateProjection::Skipped => "Skipped",
        FormulaDrillNodeStateProjection::Opaque => "Opaque",
        FormulaDrillNodeStateProjection::Blocked => "Blocked",
        FormulaDrillNodeStateProjection::Error => "Error",
    }
}

/// The chip class for a drill-node state (strand-token-backed in the panel CSS).
#[must_use]
pub fn drill_state_class(state: FormulaDrillNodeStateProjection) -> String {
    format!("dna-xray__chip--{}", drill_state_id(state))
}

// ---------------------------------------------------------------------------
// Partial-evaluation pill (BENCH_SPEC §4 — select subexpression → value/shape)
// ---------------------------------------------------------------------------

/// Deepest drill node whose source span *contains* the selection `[start,
/// end)` (UTF-8 byte offsets). Children are tried before parents so the
/// smallest containing subexpression wins — exactly the subexpression the
/// user highlighted. Span-less nodes never match. Only a non-empty selection
/// resolves (a collapsed caret uses [`drill_node_at_caret`] instead).
#[must_use]
pub fn drill_node_for_selection(
    nodes: &[FormulaDrillNodeProjection],
    start: usize,
    end: usize,
) -> Option<&FormulaDrillNodeProjection> {
    if start >= end {
        return None;
    }
    for node in nodes {
        if let Some(hit) = drill_node_for_selection(&node.children, start, end) {
            return Some(hit);
        }
        let contains = matches!(
            (node.source_span_start, node.source_span_len),
            (Some(s), Some(len)) if s <= start && end <= s.saturating_add(len)
        );
        if contains {
            return Some(node);
        }
    }
    None
}

/// The partial-evaluation pill's view-model (BENCH_SPEC §4): value, type, and
/// shape of the drill node matched to the current selection span. This is a
/// *read* of host truth — the drill tree already carries every subexpression's
/// evaluated value, so the pill is non-destructive by construction (the whole
/// point of the X-Ray: understand a subexpression without an Excel-F9 rewrite
/// that would destroy the formula).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartialEvalVm {
    /// Stable, addressable node id (mechanism 20) — the same id the panel row
    /// carries, so a pill and its drill row point at one node.
    pub node_id: String,
    pub expression: String,
    pub value_preview: Option<String>,
    pub shape: Option<String>,
    /// The node's disposition/kind label used as the pill's "type" chip.
    pub type_label: String,
    pub state: FormulaDrillNodeStateProjection,
}

/// Build the pill for the current selection, or `None` when the selection is
/// collapsed or matches no drill node. `anchor`/`focus` are the editor
/// surface's own UTF-8 byte offsets (order-independent).
#[must_use]
pub fn partial_eval(
    drill: &FormulaDrillSurface,
    anchor: usize,
    focus: usize,
) -> Option<PartialEvalVm> {
    let (start, end) = (anchor.min(focus), anchor.max(focus));
    let node = drill_node_for_selection(&drill.tree, start, end)?;
    Some(PartialEvalVm {
        node_id: node.node_id.clone(),
        expression: node
            .expression_text
            .clone()
            .unwrap_or_else(|| node.label.clone()),
        value_preview: node.value_preview.clone(),
        shape: node
            .array_preview
            .as_ref()
            .map(|p| shape_label(p.total_rows, p.total_cols)),
        type_label: node
            .kind
            .clone()
            .unwrap_or_else(|| drill_state_label(node.state).to_string()),
        state: node.state,
    })
}

/// Reference X-Ray (mechanism 07): true when `caret` (UTF-8 byte offset) sits
/// inside a rendered segment's span, so the editor can light the token the
/// caret rests in. Boundary caret (`caret == byte_end`) belongs to the *next*
/// segment, matching how a text caret reads.
#[must_use]
pub fn segment_lit_by_caret(segment: &RenderSegment, caret: usize) -> bool {
    segment.byte_start <= caret && caret < segment.byte_end
}

// ---------------------------------------------------------------------------
// Degrade-mode dry-bind preview
// ---------------------------------------------------------------------------

/// Ask the optional [`PreviewService`] what this content *would* do.
/// Preview failure is never escalated (the preview seam's own doctrine):
/// `None` means the degrade editor falls back to post-attempt rejections.
#[must_use]
pub fn dry_bind_preview(
    service: &dyn PreviewService,
    node: &NodeId,
    content: &str,
) -> Option<FormulaBindPreviewProjection> {
    service.preview_formula_bind(node, content).ok()
}

/// Convenience for the diagnostics list / underline marks out of a preview.
#[must_use]
pub fn dry_bind_diagnostics(
    preview: Option<&FormulaBindPreviewProjection>,
) -> Vec<FormulaBindPreviewDiagnosticProjection> {
    preview.map(|p| p.diagnostics.clone()).unwrap_or_default()
}

/// True when the editor surface is stale (`document_is_fresh == false`):
/// render it anyway, but flag it (SHELL_SPEC §6 / bead law: stale surface =
/// render but flag).
#[must_use]
pub fn is_stale(surface: &FormulaEditorSurface) -> bool {
    !surface.document_is_fresh
}

// ---------------------------------------------------------------------------
// Undo/redo carve-out while the edit buffer is dirty (bead dtc-lfz.2 / S1.1,
// owner-ratified 2026-07-12: SHELL_SPEC §5's carve-out clause).
// ---------------------------------------------------------------------------

/// "Dirty" exactly as the carve-out defines it: the local edit buffer's text
/// differs from the host's last-committed `source_text`. This is the same
/// comparison `FormulaBridge`'s echo-pending underlay-hiding logic already
/// uses — one predicate for "dirty", not two definitions drifting apart.
#[must_use]
pub fn buffer_is_dirty(buffer: &str, source: &str) -> bool {
    buffer != source
}

/// The DEGRADE editor's "dirty" fact (bead dtc-j7n8.25, regression fix): the
/// buffer is measured against the host's COMMITTED cell text when the host
/// supplies it, and only falls back to the mount seed where the two coincide
/// by construction (the Notebook block and the Bench slot remount from the
/// last committed text, so their seed IS the committed text). The Sheet stage
/// mounts a type-to-replace editor with seed = the typed character, so a
/// seed-vs-buffer comparison classed that buffer CLEAN and Ctrl+Z bubbled to
/// the shell's model Undo under the open editor (a subsequent Enter then
/// committed the typed text on top of the reverted workspace). Excel never
/// performs workbook undo from edit mode: a type-to-replace buffer is dirty
/// from its first character, exactly as [`buffer_is_dirty`] defines dirty
/// (buffer vs committed `source_text`). An untouched F2 buffer (buffer ==
/// committed) stays clean, so the dtc-lfz.2 rule (clean buffer hands Ctrl+Z
/// to the shell) is unchanged.
#[must_use]
pub fn degrade_buffer_is_dirty(buffer: &str, seed: &str, committed: Option<&str>) -> bool {
    buffer_is_dirty(buffer, committed.unwrap_or(seed))
}

/// Is this keydown one of the three chords the undo/redo carve-out
/// recognizes — Ctrl+Z (undo), Ctrl+Y (redo), Ctrl+Shift+Z (redo)? Matches
/// the letter case-insensitively (`KeyboardEvent.key` capitalizes under
/// Shift on common layouts, so Ctrl+Shift+Z arrives as `"Z"`); Alt
/// disqualifies (never part of an undo/redo chord per SHELL_SPEC §5.1).
/// Shift itself is irrelevant here — both Ctrl+Z and Ctrl+Shift+Z must be
/// recognized, and the caller does not need to distinguish undo from redo to
/// decide whether the carve-out applies.
#[must_use]
pub fn is_undo_redo_chord(key: &str, ctrl: bool, alt: bool) -> bool {
    ctrl && !alt && (key.eq_ignore_ascii_case("z") || key.eq_ignore_ascii_case("y"))
}

/// The carve-out decision itself (SHELL_SPEC §5): consume an undo/redo chord
/// locally — native textarea undo/redo, `stop_propagation()` only, NEVER
/// `prevent_default()` (the browser's own undo/redo stack IS the effect) —
/// only while the buffer is dirty. Once clean, or for any chord that isn't
/// Ctrl+Z/Y/Shift+Z, the answer is `false` and the keydown handler must let
/// the key fall through untouched so it bubbles to the shell exactly like
/// every other unconsumed key (the general H1b rule, bead dtc-1tk.1).
#[must_use]
pub fn should_consume_undo_redo_locally(key: &str, ctrl: bool, alt: bool, dirty: bool) -> bool {
    dirty && is_undo_redo_chord(key, ctrl, alt)
}

/// What the DEGRADE editor's `<textarea>` does with one keydown (bead
/// dtc-j7n8.25). The degrade editor used to call `stop_propagation()` on
/// EVERY keydown before matching, so once the Sheet stage handed the overlay
/// editor keyboard focus (dtc-j7n8.26) Ctrl+S / Ctrl+O / Ctrl+K / F9 typed
/// while editing a cell died in the textarea and never reached the shell's
/// `.dna-shell` keydown pipeline. This is the same propagation policy the full
/// [`crate::FormulaBridge`] editor follows (dtc-1tk.1 / dtc-lfz.2): only a key
/// the editor actually consumes is stopped; everything else bubbles so the
/// shell's guard order (SHELL_SPEC §5 — modified chords and function keys work
/// from inside edit buffers) decides.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DegradeKeyDisposition {
    /// Plain Enter: commit, advance Down (`prevent_default` + stop).
    CommitDown,
    /// Tab without Shift where the host opted in (`commit_on_tab`): commit,
    /// advance Right (`prevent_default` + stop).
    CommitRight,
    /// Escape: exact revert (`prevent_default` + stop).
    Revert,
    /// Ctrl+Z / Ctrl+Y / Ctrl+Shift+Z while the buffer is dirty: the
    /// textarea's own undo/redo stack is the effect — stop propagation only,
    /// never `prevent_default` (the dtc-lfz.2 carve-out).
    ConsumeUndoRedoLocally,
    /// Not the editor's key: leave the event untouched so it bubbles to the
    /// shell (Ctrl+S saves, Ctrl+K opens the deck, F9 recalculates; plain
    /// typing is then suppressed by the shell's text-entry guard).
    Bubble,
}

/// The degrade editor's keydown policy as one pure function over the raw
/// event facts (`key` verbatim from `KeyboardEvent.key`), so the wiring cannot
/// drift from the tested table.
#[must_use]
pub fn degrade_key_disposition(
    key: &str,
    shift: bool,
    ctrl: bool,
    alt: bool,
    commit_on_tab: bool,
    dirty: bool,
) -> DegradeKeyDisposition {
    match key {
        "Enter" if !shift => DegradeKeyDisposition::CommitDown,
        // Shift+Tab is deliberately NOT captured (grid Shift+Tab-left is a
        // later concern); off a grid Tab keeps its browser focus-move.
        "Tab" if commit_on_tab && !shift => DegradeKeyDisposition::CommitRight,
        "Escape" => DegradeKeyDisposition::Revert,
        _ if should_consume_undo_redo_locally(key, ctrl, alt, dirty) => {
            DegradeKeyDisposition::ConsumeUndoRedoLocally
        }
        _ => DegradeKeyDisposition::Bubble,
    }
}

/// One diagnostic row's list metadata (shared by full-mode diagnostics and
/// tests): `(severity label, stage label, message)` all verbatim from the IR.
#[must_use]
pub fn diagnostic_row(diag: &FormulaDiagnosticProjection) -> (String, String, String) {
    (
        severity_label(diag.severity).to_string(),
        stage_label(diag.stage).to_string(),
        diag.message.clone(),
    )
}
