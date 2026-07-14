//! TIER: TP (Presentation) — T0 skin-ir + Leptos + TP crates only; no Ox* ever (P-gate).
//!
//! The Notebook stage (S2.1): a [`dnacalc_shell::StageSurface`] registered
//! under [`dnacalc_shell::StageId::Notebook`]. This bead scaffolds the stage
//! with a small, honest-empty static view — a later bead makes it reactive
//! over the workbook's notebook content. Never rendered blank: the empty
//! state is an explicit, testable card.

use leptos::prelude::*;

use dnacalc_shell::{ProfileTag, StageContext, StageHandle, StageId, StageSurface};

pub mod model;

pub use model::{NotebookEntry, NotebookEntryKind, derive_entries};

/// The Notebook stage surface.
#[derive(Debug, Clone, Copy, Default)]
pub struct NotebookStage;

impl NotebookStage {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl StageSurface for NotebookStage {
    fn id(&self) -> StageId {
        StageId::Notebook
    }

    fn title(&self) -> &'static str {
        "Notebook"
    }

    fn supports(&self, profile: &ProfileTag) -> bool {
        matches!(profile, ProfileTag::ExcelStrict)
    }

    fn mount(&self, _ctx: StageContext) -> StageHandle {
        let view = view! {
            <section data-stage="notebook" data-testid="notebook-root">
                <p data-testid="notebook-empty">"No notebook content yet."</p>
            </section>
        }
        .into_any();
        StageHandle::new(view)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notebook_stage_identity() {
        let stage = NotebookStage::new();
        assert_eq!(stage.id(), StageId::Notebook);
        assert_eq!(stage.title(), "Notebook");
        assert!(stage.supports(&ProfileTag::ExcelStrict));
    }
}
