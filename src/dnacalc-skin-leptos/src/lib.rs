//! DNA Calc Leptos skin layer.
//!
//! The Leptos-bound half of the skin architecture: the `WorkspaceSkin` trait
//! and `SkinContext`, the `SkinRegistry`, signal-backed state handles, the
//! signal-backed `InMemoryDispatcher`, and the UI-side helper modules
//! (`keybinding`, `style`, `theme`, `accessibility`).
//!
//! The pure UX wire protocol — intents, receipts, deltas, projections,
//! identity, selection, session channel, the `SkinState`/`SharedSkinState`
//! contracts, and the signal-free `RecordingDispatcher` — lives in the
//! `dnacalc-skin-ir` crate, which this crate re-exports (`pub use
//! dnacalc_skin_ir::*`) so downstream imports stay stable.

pub mod accessibility;
pub mod components;
pub mod in_memory_dispatcher;
pub mod keybinding;
pub mod registry;
pub mod skin;
pub mod state_handles;
pub mod style;
pub mod theme;

// Re-export the whole wire protocol so consumers of this crate keep a single
// import surface.
pub use dnacalc_skin_ir::*;

pub use accessibility::{
    AriaAttrs, SelectableItemA11y, SelectableRowA11y, aria_bool, listbox_a11y, roving_tabindex,
    stable_node_dom_id, table_a11y, tree_a11y,
};
pub use components::cell_entry::{
    CELL_ENTRY_CSS, CellEntryEditor, EntryCommitResult, EntryDiagnostics, EntryFeedback,
    classify_entry_receipt, unresolved_name_note,
};
pub use components::name_form::{
    NAME_FORM_CSS, NAMES_BACKING_GRID, NameCommitResult, NameForm, NameFormFeedback,
    RenameNameForm, commit_dynamic_name, commit_rename, commit_static_name, name_already_defined,
    next_names_backing_cell,
};
pub use in_memory_dispatcher::InMemoryDispatcher;
pub use keybinding::{
    KeyChord, KeybindingError, KeybindingOverrideMap, KeybindingRegistry, SkinVerb,
};
pub use registry::SkinRegistry;
pub use skin::{
    ErasedSkinContext, ErasedSkinFactory, RegisteredSkin, SkinContext, SkinHandle, WorkspaceSkin,
};
pub use state_handles::{SharedSkinStateHandle, SkinStateHandle};
pub use style::{
    ATLAS_SPINE_CSS, ProvenanceTint, calc_state_class, provenance_tint, selection_mode_class,
};
pub use theme::{ThemeMode, ThemeTokens};

#[cfg(test)]
mod tests;
