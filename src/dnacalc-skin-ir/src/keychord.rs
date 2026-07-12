//! Keybinding *protocol data* — the DOM-free, Leptos-free value types.
//!
//! `KeyChord`, `SkinVerb`, `KeybindingOverrideMap`, and `KeybindingError` are
//! plain serializable data: they ride in `SharedSkinState`
//! (`keybinding_overrides`) and in persisted/audited shared-state changes, so
//! they belong in the wire-protocol crate. The *grammar* that resolves a
//! chord to a verb — `KeybindingRegistry`, with its universal table — is a UI
//! concern and lives in `dnacalc_skin_leptos::keybinding`, which re-exports
//! these types.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::command::CommandIntentKindProjection;

/// Lowercase a single alphabetic key so `Shift` is carried only by the flag.
/// Multi-character keys (`"Enter"`, `"ArrowUp"`, `"F9"`) pass through unchanged.
fn normalize_key(key: String) -> String {
    let mut chars = key.chars();
    match (chars.next(), chars.next()) {
        (Some(ch), None) if ch.is_alphabetic() => ch.to_ascii_lowercase().to_string(),
        _ => key,
    }
}

/// A normalized key chord: a key name plus modifier flags.
///
/// `key` mirrors the DOM `KeyboardEvent.key` value (`"Enter"`, `"F9"`,
/// `"ArrowUp"`, `"Escape"`, `"1"`, `" "`, `"/"`, `"]"`), except single
/// alphabetic keys are lowercased so `Shift` state is carried only by the
/// `shift` flag rather than by the key's case.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyChord {
    pub key: String,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

impl KeyChord {
    /// Build a chord from raw parts, normalizing single alphabetic keys to
    /// lowercase. `ctrl` should already fold the platform `meta` key in.
    #[must_use]
    pub fn from_parts(key: impl Into<String>, ctrl: bool, alt: bool, shift: bool) -> Self {
        Self {
            key: normalize_key(key.into()),
            ctrl,
            alt,
            shift,
        }
    }

    /// A bare (unmodified) key chord.
    #[must_use]
    pub fn bare(key: impl Into<String>) -> Self {
        Self::from_parts(key, false, false, false)
    }

    /// A `Ctrl`-modified key chord.
    #[must_use]
    pub fn ctrl(key: impl Into<String>) -> Self {
        Self::from_parts(key, true, false, false)
    }
}

/// The universal verb vocabulary every lens shares.
///
/// Only verbs in this enum are part of the universal grammar. Lens-local
/// secondary chords (`Tab`, `Ctrl+Enter`-as-something-else, drag) are
/// intentionally absent — they stay lens-local and badged as such, so a key a
/// user thinks is universal never silently means something different.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SkinVerb {
    /// Commit the active edit / drop to the next sibling (Enter).
    Commit,
    /// Recalculate the workspace (F9; Ctrl+Enter compat).
    Recalculate,
    /// Fill the selection from the anchor (Ctrl+D).
    Fill,
    /// Collapse toward parent (h).
    Fold,
    /// Expand toward child (l).
    Unfold,
    /// Extend the trace one step toward dependents (]).
    TraceForward,
    /// Extend the trace one step toward precedents ([).
    TraceBack,
    /// Open the Name-Box quick jump (/).
    NameBox,
    /// Open the leader / health palette (Space).
    Leader,
    /// Explain the selection's derivation (E).
    Explain,
    /// Navigate the retained revision graph backward (Ctrl+Z).
    Undo,
    /// Navigate the retained revision graph forward (Ctrl+Y).
    Redo,
    /// Move selection to the previous sibling (ArrowUp).
    NavPrev,
    /// Move selection to the next sibling (ArrowDown).
    NavNext,
    /// Move selection toward the parent (ArrowLeft).
    ToParent,
    /// Move selection toward the first child (ArrowRight).
    ToChild,
    /// Switch to the lens at the given 1-based slot (Ctrl+1..9).
    SwitchLens(u8),
    /// Create a new workspace (Ctrl+N).
    NewWorkspace,
    /// The canonical escape ladder step (Escape).
    Escape,
    /// Clear the selected node/cell contents — Excel's `Delete`/`Backspace`.
    /// Safe and non-structural: maps to an empty content edit, never a node
    /// removal.
    ClearContents,
    /// Begin editing the selection in place — Excel's `F2`.
    EditInPlace,

    // --- Shell verbs (SHELL_SPEC.md §5.1, added additively for the redesign
    // shell's registry; the estate grammar above is untouched). They live in
    // this enum so a user rebind of a shell verb rides the same persisted,
    // audited `KeybindingOverrideMap` as every other verb. ---
    /// Open the command deck overlay (Ctrl+K; Ctrl+Shift+P mirror).
    CommandDeck,
    /// Open the keyboard atlas overlay (Ctrl+/; `?` when not editing).
    KeyboardAtlas,
    /// Open the timeline / revision drawer overlay (Ctrl+H).
    Timeline,
    /// Peek card for the selection (P; Alt+hover is pointer-side).
    Peek,
    /// Toggle the Registry rail (Ctrl+B; Alt+B alternate; Calc only).
    RegistryToggle,
    /// Toggle the Inspector panel (Ctrl+I; Alt+I alternate).
    InspectorToggle,
    /// Save the document (Ctrl+S; disabled with honest badge until host support).
    Save,
    /// Open a document (Ctrl+O; disabled with honest badge until host support).
    Open,
    /// Find / goto — opens the command deck in goto mode (Ctrl+F).
    FindGoto,
}

impl SkinVerb {
    /// Stable id for telemetry, tests, and hint rendering.
    #[must_use]
    pub fn stable_id(self) -> &'static str {
        match self {
            Self::Commit => "commit",
            Self::Recalculate => "recalculate",
            Self::Fill => "fill",
            Self::Fold => "fold",
            Self::Unfold => "unfold",
            Self::TraceForward => "trace_forward",
            Self::TraceBack => "trace_back",
            Self::NameBox => "name_box",
            Self::Leader => "leader",
            Self::Explain => "explain",
            Self::Undo => "undo",
            Self::Redo => "redo",
            Self::NavPrev => "nav_prev",
            Self::NavNext => "nav_next",
            Self::ToParent => "to_parent",
            Self::ToChild => "to_child",
            Self::SwitchLens(_) => "switch_lens",
            Self::NewWorkspace => "new_workspace",
            Self::Escape => "escape",
            Self::ClearContents => "clear_contents",
            Self::EditInPlace => "edit_in_place",
            Self::CommandDeck => "command_deck",
            Self::KeyboardAtlas => "keyboard_atlas",
            Self::Timeline => "timeline",
            Self::Peek => "peek",
            Self::RegistryToggle => "registry_toggle",
            Self::InspectorToggle => "inspector_toggle",
            Self::Save => "save",
            Self::Open => "open",
            Self::FindGoto => "find_goto",
        }
    }

    /// The inverse of [`Self::stable_id`] for the *rebindable* verbs. Returns
    /// `None` for an unknown id and for `switch_lens` — `SwitchLens(slot)` is
    /// slot-parameterized and shares one id, so it cannot be remapped as a
    /// single verb (Ctrl+1..9 stays fixed). This is the gate that keeps a
    /// persisted [`KeybindingOverrideMap`] from naming a verb the registry
    /// cannot rebind.
    #[must_use]
    pub fn from_stable_id(id: &str) -> Option<Self> {
        Some(match id {
            "commit" => Self::Commit,
            "recalculate" => Self::Recalculate,
            "fill" => Self::Fill,
            "fold" => Self::Fold,
            "unfold" => Self::Unfold,
            "trace_forward" => Self::TraceForward,
            "trace_back" => Self::TraceBack,
            "name_box" => Self::NameBox,
            "leader" => Self::Leader,
            "explain" => Self::Explain,
            "undo" => Self::Undo,
            "redo" => Self::Redo,
            "nav_prev" => Self::NavPrev,
            "nav_next" => Self::NavNext,
            "to_parent" => Self::ToParent,
            "to_child" => Self::ToChild,
            "new_workspace" => Self::NewWorkspace,
            "escape" => Self::Escape,
            "clear_contents" => Self::ClearContents,
            "edit_in_place" => Self::EditInPlace,
            "command_deck" => Self::CommandDeck,
            "keyboard_atlas" => Self::KeyboardAtlas,
            "timeline" => Self::Timeline,
            "peek" => Self::Peek,
            "registry_toggle" => Self::RegistryToggle,
            "inspector_toggle" => Self::InspectorToggle,
            "save" => Self::Save,
            "open" => Self::Open,
            "find_goto" => Self::FindGoto,
            _ => return None,
        })
    }

    /// The command-catalog kind this verb maps to, when one exists, so a lens
    /// can read enablement + disabled-reason from
    /// [`CommandCatalogProjection`](crate::CommandCatalogProjection) for the
    /// same verb the registry resolves.
    #[must_use]
    pub fn command_kind(self) -> Option<CommandIntentKindProjection> {
        match self {
            Self::Commit => Some(CommandIntentKindProjection::EditContent),
            Self::Recalculate => Some(CommandIntentKindProjection::Recalculate),
            Self::Undo => Some(CommandIntentKindProjection::Undo),
            Self::Redo => Some(CommandIntentKindProjection::Redo),
            Self::NewWorkspace => Some(CommandIntentKindProjection::NewWorkspace),
            _ => None,
        }
    }
}

/// A per-user remapping of rebindable verbs, persisted in audited shared
/// state and layered over `KeybindingRegistry::universal`.
///
/// Keyed by [`SkinVerb::stable_id`] so it survives serialization and tolerates
/// the grammar growing: an entry naming a verb this build does not know is
/// rejected at `KeybindingRegistry::with_overrides` time rather than
/// silently dropped. `SwitchLens` is never representable here (it has no
/// single rebindable id), so Ctrl+1..9 is always fixed.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeybindingOverrideMap {
    by_verb: BTreeMap<String, KeyChord>,
}

impl KeybindingOverrideMap {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Rebind `verb` to `chord`. A no-op for `SwitchLens` (not rebindable),
    /// so the map can only ever hold verbs the registry can re-key.
    pub fn set(&mut self, verb: SkinVerb, chord: KeyChord) {
        if SkinVerb::from_stable_id(verb.stable_id()) == Some(verb) {
            self.by_verb.insert(verb.stable_id().to_string(), chord);
        }
    }

    /// Drop a rebinding, restoring the verb to its universal chord.
    pub fn clear(&mut self, verb: SkinVerb) {
        self.by_verb.remove(verb.stable_id());
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_verb.is_empty()
    }

    /// The override chord for a verb, if one is set.
    #[must_use]
    pub fn get(&self, verb: SkinVerb) -> Option<&KeyChord> {
        self.by_verb.get(verb.stable_id())
    }

    /// The (verb-id, chord) entries, ordered by verb id.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &KeyChord)> {
        self.by_verb.iter()
    }
}

/// Why a set of overrides could not produce a usable registry.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum KeybindingError {
    #[error("override names a verb this build cannot rebind: {0}")]
    UnknownVerb(String),
    #[error("chord is already bound to {}", occupied_by.stable_id())]
    Collision {
        chord: KeyChord,
        occupied_by: SkinVerb,
    },
}
