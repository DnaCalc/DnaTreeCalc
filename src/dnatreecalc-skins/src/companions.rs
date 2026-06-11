//! Cockpit companion skins — the Phase-B promotion of the embedded **Lens**
//! and **Console** widgets to real slots.
//!
//! Each is a thin `WorkspaceSkin` over the same shared spine widgets the
//! mono-lenses embed, so promoting a companion changes *where* the widget
//! renders, never what it reads or writes. Their capability manifests declare
//! companion slots only — mount negotiation refuses them in `Main`, and
//! refuses primary lenses in companion slots.
//!
//! When a companion is mounted the shell records its slot in
//! `SharedSkinState::companion_slots_active`; lenses re-project by standing
//! their embedded copy down (see flow.rs) — same truth, one rendering.

use std::sync::Arc;

use dnatreecalc_skin_framework::{
    ATLAS_SPINE_CSS, SharedStateOrigin, SkinCapabilities, SkinCategory, SkinContext, SkinHandle,
    SkinId, SkinManifest, SkinMountSlot, SkinState,
};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::spine_widgets::{
    ConsoleBar, NodeInspector, SPINE_WIDGETS_CSS, commit_content_edit, focus_edit_buffer,
};

pub const LENS_COMPANION_ID: SkinId = SkinId::new("lens");
pub const CONSOLE_COMPANION_ID: SkinId = SkinId::new("console");

const LENS_COMPANION_SLOTS: &[SkinMountSlot] = &[SkinMountSlot::RightInspector];
const CONSOLE_COMPANION_SLOTS: &[SkinMountSlot] = &[SkinMountSlot::BottomConsole];

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LensCompanionState {}

impl SkinState for LensCompanionState {
    fn schema_version() -> u32 {
        1
    }
}

/// The **Lens** companion: the shared selection inspector in its own slot.
#[derive(Default)]
pub struct LensCompanion;

impl LensCompanion {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl dnatreecalc_skin_framework::WorkspaceSkin for LensCompanion {
    type State = LensCompanionState;

    fn id(&self) -> SkinId {
        LENS_COMPANION_ID
    }

    fn manifest(&self) -> SkinManifest {
        SkinManifest {
            display_name: "Lens",
            description: "Selection inspector companion: facts, value, and the modeless edit buffer.",
            category: SkinCategory::Inspector,
            version: "0.1.0",
        }
    }

    fn capabilities(&self) -> SkinCapabilities {
        SkinCapabilities {
            allowed_slots: Some(LENS_COMPANION_SLOTS),
            ..SkinCapabilities::default()
        }
    }

    fn mount(&self, cx: SkinContext<Self::State>) -> SkinHandle {
        SkinHandle::new(view! { <LensCompanionView cx=cx /> }.into_any())
    }
}

#[component]
fn LensCompanionView(cx: SkinContext<LensCompanionState>) -> impl IntoView {
    let workspace = cx.workspace;
    let selection = cx.selection;
    let shared = cx.shared;
    let dispatch = cx.dispatch.clone();
    let shared_origin = SharedStateOrigin::lens(LENS_COMPANION_ID, cx.slot);

    let editing = RwSignal::new(false);
    let editor_text = RwSignal::new(String::new());
    let edit_ref = NodeRef::<leptos::html::Textarea>::new();

    let selected = Memo::new(move |_| {
        let primary = selection.with(|s| s.primary.clone());
        workspace.with(|ws| primary.as_ref().and_then(|id| ws.node(id).cloned()))
    });
    let selected_id = Memo::new(move |_| selection.with(|s| s.primary.clone()));

    // The same two-effect buffer sync contract as the embedded inspector:
    // reset on selection identity change, refresh-from-published while idle.
    Effect::new(move |_| {
        let _identity = selected_id.get();
        if let Some(node) = selected.get_untracked() {
            editor_text.set(node.content_text);
        } else {
            editor_text.set(String::new());
        }
        editing.set(false);
    });
    Effect::new(move |_| {
        if let Some(node) = selected.get()
            && !editing.get_untracked()
        {
            editor_text.set(node.content_text);
        }
    });

    let commit_dispatch = dispatch.clone();
    let commit_origin = shared_origin.clone();
    let commit: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
        let Some(node) = selected.get_untracked() else {
            return;
        };
        let receipt = commit_content_edit(
            &commit_dispatch,
            shared,
            &commit_origin,
            node.id,
            editor_text.get_untracked(),
        );
        if receipt.accepted {
            editing.set(false);
        }
    });

    // Double-clicking anywhere in the companion focuses the edit buffer —
    // the companion's one local affordance beyond the shared widget.
    let css = format!("{ATLAS_SPINE_CSS}\n{SPINE_WIDGETS_CSS}\n{COMPANION_CSS}");

    view! {
        <style>{css}</style>
        <section
            class="dtc-companion-lens"
            on:dblclick=move |_| {
                if selected.get_untracked().is_some() {
                    editing.set(true);
                    focus_edit_buffer(edit_ref);
                }
            }
        >
            <NodeInspector
                workspace=workspace
                selection=selection
                editing=editing
                editor_text=editor_text
                edit_ref=edit_ref
                commit=commit
                shared=shared
                preview=cx.preview.clone()
                standalone=true
            />
        </section>
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsoleCompanionState {}

impl SkinState for ConsoleCompanionState {
    fn schema_version() -> u32 {
        1
    }
}

/// The **Console** companion: the model-health strip in its own slot, plus the
/// active-lens / focused-slot indicators the cockpit chrome wants.
#[derive(Default)]
pub struct ConsoleCompanion;

impl ConsoleCompanion {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl dnatreecalc_skin_framework::WorkspaceSkin for ConsoleCompanion {
    type State = ConsoleCompanionState;

    fn id(&self) -> SkinId {
        CONSOLE_COMPANION_ID
    }

    fn manifest(&self) -> SkinManifest {
        SkinManifest {
            display_name: "Console",
            description: "Model health, capability profile, and cockpit indicators.",
            category: SkinCategory::Inspector,
            version: "0.1.0",
        }
    }

    fn capabilities(&self) -> SkinCapabilities {
        SkinCapabilities {
            allowed_slots: Some(CONSOLE_COMPANION_SLOTS),
            ..SkinCapabilities::default()
        }
    }

    fn mount(&self, cx: SkinContext<Self::State>) -> SkinHandle {
        SkinHandle::new(view! { <ConsoleCompanionView cx=cx /> }.into_any())
    }
}

#[component]
fn ConsoleCompanionView(cx: SkinContext<ConsoleCompanionState>) -> impl IntoView {
    let workspace = cx.workspace;
    let shared_signal = cx.shared.signal();
    let dispatch = cx.dispatch.clone();

    let active_lens = Memo::new(move |_| {
        shared_signal.with(|s| s.active_lens.clone().unwrap_or_else(|| "—".to_string()))
    });
    let focused_slot = Memo::new(move |_| {
        shared_signal.with(|s| {
            s.focused_slot
                .map_or("main", |slot| slot.stable_id())
                .to_string()
        })
    });
    let persona = Memo::new(move |_| shared_signal.with(|s| s.persona.stable_id().to_string()));
    let workspace_label = Memo::new(move |_| workspace.with(|ws| ws.workspace_id.clone()));

    let css = format!("{ATLAS_SPINE_CSS}\n{SPINE_WIDGETS_CSS}\n{COMPANION_CSS}");

    view! {
        <style>{css}</style>
        <section class="dtc-companion-console">
            <div class="dtc-companion-console__indicators">
                <span class="dtc-companion-console__chip" title="Active lens">
                    "lens: " {move || active_lens.get()}
                </span>
                <span class="dtc-companion-console__chip" title="Focused slot">
                    "focus: " {move || focused_slot.get()}
                </span>
                <span class="dtc-companion-console__chip" title="Governing persona">
                    "persona: " {move || persona.get()}
                </span>
                <span class="dtc-companion-console__chip" title="Workspace">
                    {move || workspace_label.get()}
                </span>
            </div>
            <ConsoleBar workspace=workspace dispatch=dispatch.clone() shared=cx.shared standalone=true />
        </section>
    }
}

const COMPANION_CSS: &str = r#"
.dtc-companion-lens { height: 100%; overflow: auto; }
.dtc-companion-lens .dtc-inspector { width: 100%; border-left: none; box-sizing: border-box; }

.dtc-companion-console { display: flex; flex-direction: column; }
.dtc-companion-console__indicators {
  display: flex; align-items: center; gap: 8px; padding: 4px 12px;
  background: var(--dtc-surface-subtle);
}
.dtc-companion-console__chip {
  padding: 1px 8px; border: 1px solid var(--dtc-border-muted); border-radius: 9px;
  color: var(--dtc-text-subtle); font-size: 11px;
}
.dtc-companion-console .dtc-console { border-top: 1px solid var(--dtc-border-muted); }
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use dnatreecalc_skin_framework::WorkspaceSkin;

    #[test]
    fn companions_declare_companion_slots_only() {
        let lens = LensCompanion::new();
        let console = ConsoleCompanion::new();

        assert!(lens.capabilities().slot_allowed(SkinMountSlot::RightInspector));
        assert!(!lens.capabilities().slot_allowed(SkinMountSlot::Main));
        assert!(!lens.capabilities().slot_allowed(SkinMountSlot::BottomConsole));

        assert!(console.capabilities().slot_allowed(SkinMountSlot::BottomConsole));
        assert!(!console.capabilities().slot_allowed(SkinMountSlot::Main));
        assert!(!console.capabilities().slot_allowed(SkinMountSlot::RightInspector));
    }

    #[test]
    fn primary_lens_defaults_to_main_only() {
        let flow = crate::flow::FlowLens::new();
        assert!(flow.capabilities().slot_allowed(SkinMountSlot::Main));
        assert!(!flow.capabilities().slot_allowed(SkinMountSlot::RightInspector));
    }
}
