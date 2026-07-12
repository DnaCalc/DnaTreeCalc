//! dnacalc-shell — the DNA Calc cockpit shell (SHELL_SPEC.md, S0 slice).
//!
//! TP-tier presentation crate (SHELL_SPEC §2): regions with per-product
//! composition, the `StageSurface` host contract, the overlay system, the
//! §5.1 keyboard grammar with its hard-reserved law and the keyboard atlas,
//! and the Strip/Inspector slot models with their reserved
//! renders-nothing parity seams (PARITY_TRUST_UX §5/§8).
//!
//! Dependency law (P-gate): `dnacalc-skin-ir` + `dnacalc-strand` +
//! `dnacalc-skin-leptos` (ox*-free, verified) + Leptos. No engine crate can
//! ever appear here — the shell renders projections and dispatches intents,
//! nothing else.
//!
//! Out of S0.6 scope, with typed seams left for their beads:
//! - Command deck v1 → bead dtc-tsc.8 mounts through
//!   [`overlay::OverlaySurface`] / [`overlay::ShellOverlaySlots`].
//! - Bridge formula workbench → bead dtc-tsc.7 mounts through
//!   [`composition::BridgeSurface`].

pub mod command_deck;
pub mod composition;
pub mod inspector;
pub mod keyboard;
pub mod overlay;
pub mod profile;
pub mod shell;
pub mod stage;
pub mod strip;

pub use command_deck::{
    A1Address, COMMAND_DECK_CSS, CommandDeck, DeckAction, DeckCommand, DeckGroup, DeckInputs,
    deck_listing, filter_commands, fuzzy_score, goto_commands, parse_a1, static_command_ids,
    static_commands,
};
pub use composition::{
    BRIDGE_HEIGHT_ONE_ROW_PX, BRIDGE_HEIGHT_TWO_ROW_PX, BridgeSlot, BridgeSurface,
    CatalogComposition, INSPECTOR_WIDTH_PX, InspectorComposition, InspectorSurface, MAST_HEIGHT_PX,
    MastComposition, REGISTRY_WIDTH_PX, RegistryComposition, STRIP_HEIGHT_PX, ShellComposition,
    StripComposition,
};
pub use inspector::{InspectorSlotContent, InspectorSlotKind, inspector_slot_content};
pub use keyboard::{
    AtlasGroup, AtlasRow, BindingTier, ChordScope, KeyRouting, ShellBinding, ShellKeybindingError,
    ShellKeyboardRegistry, chord_display, chord_from_key_event, hard_reserved_chords,
    is_desktop_sanctioned, is_function_key, is_hard_reserved, route_key, verb_label,
};
pub use overlay::{
    ActiveOverlay, EscapeOutcome, MAX_PINNED_PEEKS, OverlayContext, OverlayModel, OverlaySurface,
    PeekCard, PeekError, PeekModel, ShellControls, ShellOverlaySlots,
};
pub use profile::{ProfileTag, RuntimeContext};
pub use shell::Shell;
pub use stage::{
    HaloSpec, StageContext, StageHandle, StageId, StageRegistry, StageResolution, StageSurface,
    continuity_halo, resolve_active_stage, switch_stage,
};
pub use strip::{StripSlot, StripSlotContent, strip_slot_content};
