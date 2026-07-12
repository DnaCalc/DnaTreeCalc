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
use crate::events::{BridgeEvent, BridgeEvents, EditDiscipline};
use crate::vm::{degrade_segments, dry_bind_preview, text_edited_from_dom, utf8_to_utf16};

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

    let on_keydown = move |ev: leptos::ev::KeyboardEvent| {
        ev.stop_propagation();
        match ev.key().as_str() {
            "Enter" if !ev.shift_key() => {
                ev.prevent_default();
                edit_state.set(EditDiscipline::Selected);
                on_event.run(BridgeEvent::CommitRequested);
            }
            "Escape" => {
                ev.prevent_default();
                edit_state.set(EditDiscipline::Selected);
                on_event.run(BridgeEvent::RevertRequested);
            }
            _ => {}
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
