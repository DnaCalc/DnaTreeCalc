//! The one grammar — a single typed keybinding registry shared by the shell
//! and every lens.
//!
//! ATLAS's promise is "one verb, one meaning, everywhere": a key a user thinks
//! is universal always behaves universally. This module is the single source of
//! truth for that universal verb table. It is pure metadata over the closed
//! [`WorkspaceIntent`](crate::WorkspaceIntent) surface — it resolves a key chord
//! to a [`SkinVerb`] but never executes anything. The shell and lenses map the
//! resolved verb to an intent or a lens-local action.
//!
//! [`KeyChord`] is deliberately DOM-free (plain key string + modifier flags) so
//! the registry is unit-testable without a browser. The shell builds a chord
//! from a `web_sys::KeyboardEvent` and calls [`KeybindingRegistry::resolve`].

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
/// state and layered over [`KeybindingRegistry::universal`].
///
/// Keyed by [`SkinVerb::stable_id`] so it survives serialization and tolerates
/// the grammar growing: an entry naming a verb this build does not know is
/// rejected at [`KeybindingRegistry::with_overrides`] time rather than
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

/// The shared keybinding table.
///
/// Built via [`KeybindingRegistry::universal`] (or layered with a user's
/// [`KeybindingOverrideMap`] through [`KeybindingRegistry::with_overrides`]);
/// the shell and lenses resolve chords through it. A chord resolves to at most
/// one verb — the `universal_grammar_is_collision_free` test guarantees the
/// base table is unique, and `with_overrides` re-verifies after remapping.
#[derive(Debug, Clone)]
pub struct KeybindingRegistry {
    bindings: Vec<(KeyChord, SkinVerb)>,
}

impl Default for KeybindingRegistry {
    fn default() -> Self {
        Self::universal()
    }
}

impl KeybindingRegistry {
    /// The one canonical ATLAS grammar.
    #[must_use]
    pub fn universal() -> Self {
        let mut bindings = vec![
            (KeyChord::bare("Enter"), SkinVerb::Commit),
            (KeyChord::bare("F9"), SkinVerb::Recalculate),
            // Ctrl+Enter is kept as a compatibility chord for Recalculate so
            // the existing shell muscle memory still works.
            (KeyChord::ctrl("Enter"), SkinVerb::Recalculate),
            (KeyChord::ctrl("d"), SkinVerb::Fill),
            (KeyChord::bare("h"), SkinVerb::Fold),
            (KeyChord::bare("l"), SkinVerb::Unfold),
            (KeyChord::bare("]"), SkinVerb::TraceForward),
            (KeyChord::bare("["), SkinVerb::TraceBack),
            (KeyChord::bare("/"), SkinVerb::NameBox),
            (KeyChord::bare(" "), SkinVerb::Leader),
            (KeyChord::bare("e"), SkinVerb::Explain),
            (KeyChord::ctrl("z"), SkinVerb::Undo),
            (KeyChord::ctrl("y"), SkinVerb::Redo),
            (KeyChord::bare("ArrowUp"), SkinVerb::NavPrev),
            (KeyChord::bare("ArrowDown"), SkinVerb::NavNext),
            (KeyChord::bare("ArrowLeft"), SkinVerb::ToParent),
            (KeyChord::bare("ArrowRight"), SkinVerb::ToChild),
            (KeyChord::ctrl("n"), SkinVerb::NewWorkspace),
            (KeyChord::bare("Escape"), SkinVerb::Escape),
        ];
        for slot in 1u8..=9 {
            bindings.push((KeyChord::ctrl(slot.to_string()), SkinVerb::SwitchLens(slot)));
        }
        Self { bindings }
    }

    /// The universal grammar with a user's overrides layered on: each
    /// overridden verb's universal chord(s) are collapsed to the single
    /// remapped chord, every other verb keeps its universal binding, and the
    /// result is re-verified collision-free.
    ///
    /// Errors rather than silently producing an ambiguous table: an override
    /// naming an unknown/non-rebindable verb is [`KeybindingError::UnknownVerb`]
    /// and a remap onto a chord another verb still holds is
    /// [`KeybindingError::Collision`]. Because the final set is validated as a
    /// whole, swapping two verbs' chords is accepted; only a genuine clash is
    /// rejected.
    pub fn with_overrides(overrides: &KeybindingOverrideMap) -> Result<Self, KeybindingError> {
        let mut remapped: Vec<(SkinVerb, KeyChord)> = Vec::new();
        for (id, chord) in overrides.iter() {
            let verb =
                SkinVerb::from_stable_id(id).ok_or_else(|| KeybindingError::UnknownVerb(id.clone()))?;
            remapped.push((verb, chord.clone()));
        }

        let mut bindings: Vec<(KeyChord, SkinVerb)> = Vec::new();
        let mut emitted: Vec<SkinVerb> = Vec::new();
        for (chord, verb) in Self::universal().bindings {
            if let Some((_, new_chord)) = remapped.iter().find(|(candidate, _)| *candidate == verb) {
                // Collapse any compatibility chords to the single new chord.
                if !emitted.contains(&verb) {
                    bindings.push((new_chord.clone(), verb));
                    emitted.push(verb);
                }
            } else {
                bindings.push((chord, verb));
            }
        }

        for index in 0..bindings.len() {
            for other in (index + 1)..bindings.len() {
                if bindings[index].0 == bindings[other].0 && bindings[index].1 != bindings[other].1 {
                    return Err(KeybindingError::Collision {
                        chord: bindings[index].0.clone(),
                        occupied_by: bindings[index].1,
                    });
                }
            }
        }

        Ok(Self { bindings })
    }

    /// Resolve a chord to its universal verb, if any.
    #[must_use]
    pub fn resolve(&self, chord: &KeyChord) -> Option<SkinVerb> {
        self.bindings
            .iter()
            .find(|(bound, _)| bound == chord)
            .map(|(_, verb)| *verb)
    }

    /// All (chord, verb) pairs, for hint rendering and tests.
    #[must_use]
    pub fn bindings(&self) -> &[(KeyChord, SkinVerb)] {
        &self.bindings
    }

    /// The first chord bound to a verb, for shortcut hints.
    #[must_use]
    pub fn primary_chord(&self, verb: SkinVerb) -> Option<&KeyChord> {
        self.bindings
            .iter()
            .find(|(_, bound)| *bound == verb)
            .map(|(chord, _)| chord)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const REBINDABLE: [SkinVerb; 18] = [
        SkinVerb::Commit,
        SkinVerb::Recalculate,
        SkinVerb::Fill,
        SkinVerb::Fold,
        SkinVerb::Unfold,
        SkinVerb::TraceForward,
        SkinVerb::TraceBack,
        SkinVerb::NameBox,
        SkinVerb::Leader,
        SkinVerb::Explain,
        SkinVerb::Undo,
        SkinVerb::Redo,
        SkinVerb::NavPrev,
        SkinVerb::NavNext,
        SkinVerb::ToParent,
        SkinVerb::ToChild,
        SkinVerb::NewWorkspace,
        SkinVerb::Escape,
    ];

    #[test]
    fn stable_id_round_trips_for_rebindable_verbs() {
        for verb in REBINDABLE {
            assert_eq!(SkinVerb::from_stable_id(verb.stable_id()), Some(verb));
        }
        // SwitchLens shares one id and is slot-parameterized — not rebindable.
        assert_eq!(SkinVerb::from_stable_id("switch_lens"), None);
        assert_eq!(SkinVerb::from_stable_id("nonsense"), None);
    }

    #[test]
    fn override_map_ignores_switch_lens_and_round_trips_others() {
        let mut overrides = KeybindingOverrideMap::new();
        overrides.set(SkinVerb::SwitchLens(3), KeyChord::ctrl("3"));
        assert!(overrides.is_empty(), "SwitchLens must not be rebindable");
        overrides.set(SkinVerb::Explain, KeyChord::bare("x"));
        assert_eq!(overrides.get(SkinVerb::Explain), Some(&KeyChord::bare("x")));
        overrides.clear(SkinVerb::Explain);
        assert!(overrides.is_empty());
    }

    #[test]
    fn empty_overrides_equal_the_universal_grammar() {
        let registry = KeybindingRegistry::with_overrides(&KeybindingOverrideMap::new()).unwrap();
        assert_eq!(
            registry.bindings().len(),
            KeybindingRegistry::universal().bindings().len()
        );
        assert_eq!(registry.resolve(&KeyChord::bare("e")), Some(SkinVerb::Explain));
    }

    #[test]
    fn with_overrides_remaps_a_verb_and_collapses_compat_chords() {
        let mut overrides = KeybindingOverrideMap::new();
        overrides.set(SkinVerb::Explain, KeyChord::bare("x"));
        let registry = KeybindingRegistry::with_overrides(&overrides).unwrap();
        assert_eq!(registry.resolve(&KeyChord::bare("x")), Some(SkinVerb::Explain));
        assert_eq!(registry.resolve(&KeyChord::bare("e")), None);

        // Recalculate carries F9 + the Ctrl+Enter compat chord; remapping
        // collapses both to the single chosen chord.
        let mut recalc = KeybindingOverrideMap::new();
        recalc.set(SkinVerb::Recalculate, KeyChord::bare("F5"));
        let registry = KeybindingRegistry::with_overrides(&recalc).unwrap();
        assert_eq!(
            registry.resolve(&KeyChord::bare("F5")),
            Some(SkinVerb::Recalculate)
        );
        assert_eq!(registry.resolve(&KeyChord::bare("F9")), None);
        assert_eq!(registry.resolve(&KeyChord::ctrl("Enter")), None);
    }

    #[test]
    fn with_overrides_accepts_a_swap_but_rejects_a_clash() {
        // Swap Fold (h) and Unfold (l): the final set still has both, no clash.
        let mut swap = KeybindingOverrideMap::new();
        swap.set(SkinVerb::Fold, KeyChord::bare("l"));
        swap.set(SkinVerb::Unfold, KeyChord::bare("h"));
        let registry = KeybindingRegistry::with_overrides(&swap).unwrap();
        assert_eq!(registry.resolve(&KeyChord::bare("l")), Some(SkinVerb::Fold));
        assert_eq!(registry.resolve(&KeyChord::bare("h")), Some(SkinVerb::Unfold));

        // Remap Explain onto Fold's chord while Fold still holds it ⇒ clash.
        let mut clash = KeybindingOverrideMap::new();
        clash.set(SkinVerb::Explain, KeyChord::bare("h"));
        assert!(matches!(
            KeybindingRegistry::with_overrides(&clash),
            Err(KeybindingError::Collision {
                occupied_by: SkinVerb::Fold,
                ..
            })
        ));
    }

    #[test]
    fn with_overrides_rejects_a_persisted_unknown_verb() {
        // A persisted map from another build naming a non-rebindable verb is
        // rejected, not silently dropped.
        let overrides: KeybindingOverrideMap = serde_json::from_str(
            r#"{"by_verb":{"switch_lens":{"key":"3","ctrl":true,"alt":false,"shift":false}}}"#,
        )
        .unwrap();
        assert!(matches!(
            KeybindingRegistry::with_overrides(&overrides),
            Err(KeybindingError::UnknownVerb(_))
        ));
    }
}
