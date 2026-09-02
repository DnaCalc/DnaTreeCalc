//! DEGRADE MODE (SHELL_SPEC §6 — "the honest degrade, stated up front"):
//! pre-G1 Calc contexts get plain-text editing + entry-rejection spans
//! (`GridEntryDiagnosticProjection`) + optional dry-bind preview diagnostics
//! via the [`PreviewService`] seam. **No fake token colors**: this
//! constructor takes NO editor surface, and its underlay never carries a
//! token-role class (tested).
//!
//! G1 landing upgrades every context to [`crate::FormulaBridge`] at once
//! with no bridge API change — both modes speak the same [`BridgeEvent`]s.

use std::sync::Arc;

use leptos::prelude::*;
use leptos::web_sys::HtmlTextAreaElement;

use dnacalc_skin_ir::identity::NodeId;
use dnacalc_skin_ir::preview::PreviewService;
use dnacalc_skin_ir::workspace::{FormulaBindPreviewProjection, GridEntryDiagnosticProjection};

use crate::editor::segment_view;
use crate::events::{BridgeEvent, BridgeEvents, CommitAdvance, EditDiscipline};
use crate::vm::{
    DegradeKeyDisposition, buffer_is_dirty, degrade_key_disposition, degrade_segments,
    dry_bind_preview, text_edited_from_dom, utf8_to_utf16,
};

/// The optional dry-bind seam a Calc host can hand the degrade editor: the
/// preview service plus the node whose context the content would bind in.
#[derive(Clone)]
pub struct DegradePreviewBinding {
    pub service: Arc<dyn PreviewService>,
    pub node: NodeId,
}

/// The degrade-mode formula editor: plain text, rejection underlines,
/// optional dry-bind predictions. Takes NO editor surface by construction —
/// there is nothing here a host could feed fake tokens through.
#[component]
pub fn FormulaBridgeDegrade(
    /// The committed text the buffer seeds from.
    #[prop(optional)]
    text: String,
    /// Typed entry rejections from the last commit attempt (post-attempt
    /// channel). Spans are `(start, end)` UTF-8 byte offsets; `None`-span
    /// rows render message-only.
    #[prop(optional)]
    rejections: Vec<GridEntryDiagnosticProjection>,
    /// Optional dry-bind preview seam. Preview failure silently degrades to
    /// the post-attempt rejection channel (the preview doctrine).
    #[prop(optional)]
    preview: Option<DegradePreviewBinding>,
    /// Whether Tab (no Shift) COMMITS and advances RIGHT (Excel grid entry),
    /// emitting [`BridgeEvent::CommitRequested`] with [`CommitAdvance::Right`].
    /// A SPATIAL host (the Sheet grid) opts in; everywhere else (default `false`)
    /// Tab keeps its browser behavior, so a Notebook block or the single-formula
    /// Bench slot is unchanged. Grid horizontal walk is meaningless off a grid, so
    /// this is a per-host capability, not a universal editor behavior.
    #[prop(optional)]
    commit_on_tab: bool,
    /// The bridge's semantic-event sink.
    on_event: BridgeEvents,
) -> impl IntoView {
    let buffer = RwSignal::new(text.clone());
    let edit_state = RwSignal::new(EditDiscipline::Selected);
    let dry_bind: RwSignal<Option<FormulaBindPreviewProjection>> = RwSignal::new(
        preview
            .as_ref()
            .and_then(|binding| dry_bind_preview(&*binding.service, &binding.node, &text)),
    );
    let rejections_stored = StoredValue::new(rejections.clone());

    let refresh_preview = {
        let preview = preview.clone();
        move |content: &str| {
            if let Some(binding) = &preview {
                dry_bind.set(dry_bind_preview(&*binding.service, &binding.node, content));
            }
        }
    };

    let on_input = move |ev| {
        let target: HtmlTextAreaElement = event_target(&ev);
        let content = target.value();
        let caret16 = target
            .selection_start()
            .ok()
            .flatten()
            .unwrap_or_else(|| utf8_to_utf16(&content, content.len()));
        buffer.set(content.clone());
        edit_state.set(EditDiscipline::Editing);
        refresh_preview(&content);
        // Verbatim passthrough — identical law to full mode: the entered
        // text is never inspected (no `=` sniffing) and never altered.
        on_event.run(text_edited_from_dom(content, caret16));
    };

    // Propagation policy (bead dtc-j7n8.25): `stop_propagation()` ONLY on the
    // keys this editor consumes — the pure [`degrade_key_disposition`] table
    // decides. This used to stop EVERY keydown first, so once the Sheet stage
    // gave the overlay editor keyboard focus (dtc-j7n8.26) Ctrl+S typed while
    // editing a cell died here and never reached the shell's `.dna-shell`
    // keydown pipeline (the Save verb); Ctrl+O / Ctrl+K / F9 likewise. Now an
    // unconsumed key bubbles untouched and the shell's guard order decides
    // (SHELL_SPEC §5: modified chords and F-keys work from inside edit
    // buffers; plain typing is suppressed by the text-entry guard). Tab commits
    // and advances RIGHT only where the host opted in (`commit_on_tab`, a
    // grid); elsewhere Tab keeps its browser focus-move, so Notebook/Bench are
    // unchanged. Ctrl+Z/Y/Shift+Z stay TEXT-LOCAL while the buffer is dirty
    // (the dtc-lfz.2 carve-out the full editor also honors): stop only, no
    // `prevent_default`, so the textarea's own undo/redo is the effect.
    let seed_text = text.clone();
    let on_keydown = move |ev: leptos::ev::KeyboardEvent| {
        let dirty = buffer.with_untracked(|content| buffer_is_dirty(content, &seed_text));
        match degrade_key_disposition(
            &ev.key(),
            ev.shift_key(),
            ev.ctrl_key() || ev.meta_key(),
            ev.alt_key(),
            commit_on_tab,
            dirty,
        ) {
            DegradeKeyDisposition::CommitDown => {
                ev.prevent_default();
                ev.stop_propagation();
                edit_state.set(EditDiscipline::Selected);
                on_event.run(BridgeEvent::CommitRequested {
                    advance: CommitAdvance::Down,
                });
            }
            DegradeKeyDisposition::CommitRight => {
                ev.prevent_default();
                ev.stop_propagation();
                edit_state.set(EditDiscipline::Selected);
                on_event.run(BridgeEvent::CommitRequested {
                    advance: CommitAdvance::Right,
                });
            }
            DegradeKeyDisposition::Revert => {
                ev.prevent_default();
                ev.stop_propagation();
                edit_state.set(EditDiscipline::Selected);
                on_event.run(BridgeEvent::RevertRequested);
            }
            DegradeKeyDisposition::ConsumeUndoRedoLocally => ev.stop_propagation(),
            DegradeKeyDisposition::Bubble => {}
        }
    };

    // Underlay: plain-ink slices of the live buffer with rejection/dry-bind
    // underlines. `degrade_segments` guarantees `role = None` on every
    // segment, so `segment_view` can never emit a token-role class here.
    let underlay = move || {
        let content = buffer.get();
        let predictions = dry_bind
            .get()
            .map(|preview| preview.diagnostics)
            .unwrap_or_default();
        rejections_stored
            .with_value(|rejections| degrade_segments(&content, rejections, &predictions))
            .into_iter()
            .map(segment_view)
            .collect_view()
    };

    view! {
        <div
            class="dna-bridge dna-bridge--degrade"
            data-mode="degrade"
            data-edit-state=move || edit_state.get().as_str()
        >
            <style>{crate::bridge_css()}</style>
            <div class="dna-bridge__editor dna-bridge__editor--degrade">
                <div class="dna-bridge__tokens" aria-hidden="true">{underlay}</div>
                <textarea
                    class="dna-bridge__input"
                    prop:value=move || buffer.get()
                    aria-label="Formula entry"
                    spellcheck="false"
                    autocomplete="off"
                    wrap="soft"
                    on:input=on_input
                    on:keydown=on_keydown
                >
                </textarea>
            </div>
            <RejectionList rejections=rejections />
            {move || dry_bind.get().map(|preview| view! { <DryBindList preview=preview /> })}
        </div>
    }
}

/// The post-attempt rejection list: one row per rejection; rows with a span
/// carry a `bytes a–b` badge, `None`-span rows are message-only (both are
/// first-class, per the skin-IR doc).
#[component]
pub fn RejectionList(rejections: Vec<GridEntryDiagnosticProjection>) -> impl IntoView {
    if rejections.is_empty() {
        return ().into_any();
    }
    let rows = rejections
        .into_iter()
        .enumerate()
        .map(|(index, rejection)| {
            let badge = rejection.span.map(|(start, end)| {
                view! {
                    <span class="dna-bridge__rejection-span">
                        {format!("bytes {start}\u{2013}{end}")}
                    </span>
                }
            });
            view! {
                <li
                    class="dna-bridge__rejection"
                    role="alert"
                    tabindex=if index == 0 { "-1" } else { "" }
                >
                    <span class="dna-bridge__rejection-message">{rejection.message}</span>
                    {badge}
                </li>
            }
        })
        .collect::<Vec<_>>();
    view! {
        <ul class="dna-bridge__rejections" aria-label="Entry rejections">
            {rows}
        </ul>
    }
    .into_any()
}

/// The dry-bind prediction list: stage-chipped diagnostics plus profile
/// violations, all verbatim from the preview projection.
#[component]
pub fn DryBindList(preview: FormulaBindPreviewProjection) -> impl IntoView {
    if preview.diagnostics.is_empty() && preview.profile_violations.is_empty() {
        return ().into_any();
    }
    let diagnostics = preview
        .diagnostics
        .into_iter()
        .map(|diagnostic| view! {
            <li class="dna-bridge__dry-bind-row" data-stage=diagnostic.stage.stable_id()>
                <span class="dna-bridge__dry-bind-stage">{diagnostic.stage.stable_id()}</span>
                <span class="dna-bridge__dry-bind-message">{diagnostic.message}</span>
                <span class="dna-bridge__dry-bind-span">
                    {format!("bytes {}\u{2013}{}", diagnostic.span.start_utf8, diagnostic.span.end_utf8)}
                </span>
            </li>
        })
        .collect::<Vec<_>>();
    let violations = preview
        .profile_violations
        .into_iter()
        .map(|violation| {
            view! {
                <li class="dna-bridge__dry-bind-row dna-bridge__dry-bind-row--violation">
                    <span class="dna-bridge__dry-bind-stage">{violation.feature}</span>
                    <span class="dna-bridge__dry-bind-message">{violation.message}</span>
                </li>
            }
        })
        .collect::<Vec<_>>();
    view! {
        <ul class="dna-bridge__dry-bind" aria-label="Dry-bind preview" data-legal=preview.legal.to_string()>
            {diagnostics}
            {violations}
        </ul>
    }
    .into_any()
}
