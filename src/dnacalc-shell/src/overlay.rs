//! The overlay system — SHELL_SPEC.md §1 (overlays float over everything,
//! one at a time except Peeks) and the Esc ladder ("Esc closes the topmost
//! overlay before it does anything else").
//!
//! S0 scope: the Command deck (bead dtc-tsc.8) mounts through the
//! [`OverlaySurface`] / [`ShellOverlaySlots::command_deck`] seam; Timeline is
//! a *placeholder* overlay; the Keyboard atlas is real and renders from the
//! live registry. Peek cards are a data model + rendering stub (max 3 pinned).

use std::sync::Arc;

use leptos::prelude::*;

use dnacalc_skin_ir::identity::NodeKey;
use dnacalc_skin_ir::intent::Dispatcher;
use dnacalc_skin_ir::protocol::SkinShellIntent;
use dnacalc_skin_ir::selection::SelectionState;
use dnacalc_skin_ir::workspace::WorkspaceState;
use dnacalc_skin_leptos::state_handles::SharedSkinStateHandle;
use dnacalc_strand::{Density, Theme};

use crate::keyboard::ShellKeyboardRegistry;

/// The active overlay, at most one at a time (Peeks are exempt and live in
/// [`PeekModel`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveOverlay {
    /// Command deck (Ctrl+K), or goto mode when opened via Ctrl+F.
    CommandDeck { goto_mode: bool },
    /// Keyboard atlas (Ctrl+/).
    KeyboardAtlas,
    /// Timeline / revision drawer (Ctrl+H).
    Timeline,
}

/// A peek card (mechanism 06) — data model only in S0; rendering is a stub.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeekCard {
    /// The node the card peeks at.
    pub key: NodeKey,
}

/// Maximum pinned peek cards (SHELL_SPEC §1: "max 3 pinned").
pub const MAX_PINNED_PEEKS: usize = 3;

/// Why a peek operation was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PeekError {
    #[error("peek pin limit reached ({MAX_PINNED_PEEKS} cards)")]
    PinLimit,
}

/// Peek cards: one transient card plus up to three pinned. Exempt from the
/// one-at-a-time overlay rule — pinned peeks survive overlays opening and
/// closing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PeekModel {
    pub transient: Option<PeekCard>,
    pub pinned: Vec<PeekCard>,
}

impl PeekModel {
    /// Show the transient peek (P on selection / Alt+hover).
    pub fn show_transient(&mut self, card: PeekCard) {
        self.transient = Some(card);
    }

    /// Pin the transient card (or a given card) into the pinned set.
    pub fn pin(&mut self, card: PeekCard) -> Result<(), PeekError> {
        if self.pinned.len() >= MAX_PINNED_PEEKS {
            return Err(PeekError::PinLimit);
        }
        if !self.pinned.contains(&card) {
            self.pinned.push(card);
        }
        Ok(())
    }

    pub fn unpin(&mut self, key: &NodeKey) {
        self.pinned.retain(|card| &card.key != key);
    }
}

/// What one Esc keypress did to the overlay state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscapeOutcome {
    /// The active overlay closed. Esc is consumed.
    ClosedOverlay,
    /// No overlay was open; the transient peek closed. Esc is consumed.
    ClosedTransientPeek,
    /// Nothing overlay-side to close — Esc falls through to the stage /
    /// edit-buffer EscapeRevert grammar.
    Unhandled,
}

/// The overlay state machine: one active overlay + the peek model.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OverlayModel {
    pub active: Option<ActiveOverlay>,
    pub peeks: PeekModel,
}

impl OverlayModel {
    /// Open an overlay — replaces whatever was active (one at a time).
    /// Opening the already-active overlay closes it (toggle), which is what
    /// a chord like Ctrl+/ pressed twice should do.
    pub fn open(&mut self, overlay: ActiveOverlay) {
        if self.active == Some(overlay) {
            self.active = None;
        } else {
            self.active = Some(overlay);
        }
    }

    pub fn close(&mut self) {
        self.active = None;
    }

    /// The Esc ladder: topmost overlay first, then the transient peek, then
    /// fall through. Pinned peeks are never Esc-closed (they have explicit
    /// unpin affordances).
    pub fn escape(&mut self) -> EscapeOutcome {
        if self.active.is_some() {
            self.active = None;
            return EscapeOutcome::ClosedOverlay;
        }
        if self.peeks.transient.is_some() {
            self.peeks.transient = None;
            return EscapeOutcome::ClosedTransientPeek;
        }
        EscapeOutcome::Unhandled
    }
}

/// Shell-local control callbacks an overlay (the command deck) drives to
/// change view-state the Shell component owns — stage projection, region
/// collapse, theme/density, and the sibling overlays. These are *not*
/// dispatched intents (view-state, not model mutation); the Shell populates
/// them with closures over its own reactive signals.
#[derive(Clone, Copy)]
pub struct ShellControls {
    /// Switch to the visible stage in this 1-based slot (re-projection only).
    pub switch_stage: Callback<u8>,
    pub toggle_registry: Callback<()>,
    pub toggle_inspector: Callback<()>,
    pub open_atlas: Callback<()>,
    pub open_timeline: Callback<()>,
    pub set_theme: Callback<Theme>,
    pub set_density: Callback<Density>,
    /// Close the active overlay (a command deck closes itself on execute).
    pub close: Callback<()>,
}

/// Context handed to a mounted overlay surface.
///
/// Extended additively for the command deck (bead dtc-tsc.8): it reads
/// `workspace`/`selection`/`shared` (persona + keybinding overrides), shows
/// effective chords from `keyboard`, adapts the in-process
/// `CommandCatalogProjection`, and drives view-state through `controls`.
#[derive(Clone)]
pub struct OverlayContext {
    pub shared: SharedSkinStateHandle,
    pub dispatch: Arc<dyn Dispatcher>,
    /// For the command deck: opened in goto mode (Ctrl+F)?
    pub goto_mode: bool,
    pub workspace: ReadSignal<WorkspaceState>,
    pub selection: ReadSignal<SelectionState>,
    /// The live keyboard registry, so the deck shows each command's effective
    /// chord from the same source the dispatcher resolves.
    pub keyboard: ShellKeyboardRegistry,
    /// Current chrome theme / density (for the deck's active-marker + toggles).
    pub theme: Theme,
    pub density: Density,
    /// Visible stages in slot order (1-based slot, title) — the deck's stage
    /// commands and their effective chords.
    pub visible_stages: Vec<(u8, &'static str)>,
    /// Host persistence capability (SHELL_SPEC §4), threaded from the
    /// product's own `HostCapabilityProjection`/`PersistenceProjection`
    /// (bead dtc-lfz.3). A product that has not wired a capability seam
    /// leaves the `Shell`'s `host_persistence` prop `None`, which resolves
    /// to `false` here — the deck's save/open render disabled-with-reason
    /// exactly as before this field carried real truth.
    pub host_can_save: bool,
    pub host_can_open: bool,
    /// Where the deck dispatches `SkinShellIntent::{Save, SaveAs, Open,
    /// OpenRecent}` (bead dtc-lfz.3) — the OneFormula/OneCalc host's own
    /// shell-intent channel, distinct from the tree-workspace `dispatch`
    /// above. `None` when the product has not wired the channel; a deck
    /// entry gated on `host_can_save`/`host_can_open` never reaches this in
    /// that case (both are `false`), but `run_action` also no-ops rather
    /// than panicking if it ever were reached while unwired.
    pub shell_intent: Option<Callback<SkinShellIntent>>,
    pub controls: ShellControls,
}

/// The typed slot an overlay implementation mounts into. Bead dtc-tsc.8
/// provides the real command deck through this trait; until then the shell
/// renders the honest placeholder card.
pub trait OverlaySurface: Send + Sync + 'static {
    fn mount(&self, ctx: OverlayContext) -> AnyView;
}

/// The overlay surfaces a product hands the shell. All optional — a `None`
/// slot renders the honest placeholder overlay, never a fake.
#[derive(Clone, Default)]
pub struct ShellOverlaySlots {
    /// Command deck v1 lands in bead dtc-tsc.8.
    pub command_deck: Option<Arc<dyn OverlaySurface>>,
    /// Timeline drawer (mechanism 19) lands with revision UX work.
    pub timeline: Option<Arc<dyn OverlaySurface>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(name: &str) -> NodeKey {
        NodeKey::new(name)
    }

    #[test]
    fn overlays_are_one_at_a_time() {
        let mut model = OverlayModel::default();
        model.open(ActiveOverlay::CommandDeck { goto_mode: false });
        model.open(ActiveOverlay::KeyboardAtlas);
        assert_eq!(model.active, Some(ActiveOverlay::KeyboardAtlas));
        model.open(ActiveOverlay::Timeline);
        assert_eq!(model.active, Some(ActiveOverlay::Timeline));
    }

    #[test]
    fn reopening_the_active_overlay_toggles_it_closed() {
        let mut model = OverlayModel::default();
        model.open(ActiveOverlay::KeyboardAtlas);
        model.open(ActiveOverlay::KeyboardAtlas);
        assert_eq!(model.active, None);
    }

    #[test]
    fn escape_closes_topmost_overlay_first_then_transient_peek_then_falls_through() {
        let mut model = OverlayModel::default();
        model.peeks.show_transient(PeekCard { key: key("a") });
        model.open(ActiveOverlay::Timeline);

        assert_eq!(model.escape(), EscapeOutcome::ClosedOverlay);
        assert_eq!(model.active, None);
        assert!(
            model.peeks.transient.is_some(),
            "one Esc, one closure: the peek survives the overlay's Esc"
        );

        assert_eq!(model.escape(), EscapeOutcome::ClosedTransientPeek);
        assert_eq!(model.peeks.transient, None);

        assert_eq!(model.escape(), EscapeOutcome::Unhandled);
    }

    #[test]
    fn pinned_peeks_cap_at_three_with_a_typed_error() {
        let mut peeks = PeekModel::default();
        peeks.pin(PeekCard { key: key("a") }).unwrap();
        peeks.pin(PeekCard { key: key("b") }).unwrap();
        peeks.pin(PeekCard { key: key("c") }).unwrap();
        assert_eq!(
            peeks.pin(PeekCard { key: key("d") }),
            Err(PeekError::PinLimit)
        );
        assert_eq!(peeks.pinned.len(), MAX_PINNED_PEEKS);
        peeks.unpin(&key("b"));
        assert_eq!(peeks.pinned.len(), 2);
        peeks.pin(PeekCard { key: key("d") }).unwrap();
    }

    #[test]
    fn pinned_peeks_survive_overlays_and_escapes() {
        let mut model = OverlayModel::default();
        model.peeks.pin(PeekCard { key: key("a") }).unwrap();
        model.open(ActiveOverlay::CommandDeck { goto_mode: true });
        model.escape();
        model.escape();
        model.escape();
        assert_eq!(model.peeks.pinned.len(), 1, "pins are never Esc-closed");
    }
}
