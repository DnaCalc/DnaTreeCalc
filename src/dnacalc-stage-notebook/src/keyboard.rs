//! KEYBOARD — the Notebook stage's pure, atlas-taggable keyboard grammar (S2.7).
//!
//! A DOM-free, Leptos-free, fully unit-testable declarative TABLE that maps a
//! normalized [`KeyChord`] to a Notebook-local [`NotebookAction`]. The whole
//! grammar is data: [`notebook_keymap`] returns a `Vec<KeyBinding>`, each row
//! carrying a **stable string id**, its chord, its action, and a human label —
//! exactly the shape a future Atlas stage needs to *enumerate and rebind* the
//! bindings without reaching into `lib.rs`. There are no scattered hardcoded
//! `match` arms on raw key names anywhere in the stage; the DOM wiring in
//! [`crate`] resolves every keystroke through [`resolve_action`] against this
//! one table (mirroring how `dnacalc-shell`'s dispatcher and keyboard atlas both
//! read the single `ShellKeyboardRegistry`, so the two can never drift).
//!
//! WHY A SEPARATE TABLE from the shell's universal `SkinVerb` grammar: these are
//! precisely the *lens-local secondary chords* that
//! [`dnacalc_skin_ir::keychord::SkinVerb`]'s own doc calls "intentionally
//! absent" from the universal vocabulary — "they stay lens-local and badged as
//! such, so a key a user thinks is universal never silently means something
//! different". Enter-to-open, Shift+Enter-to-commit-and-advance, Alt+Arrow
//! reorder, and the two new-block chords are Notebook grammar, not shell
//! grammar, so they live in a Notebook-owned table.
//!
//! HONEST DEGRADE: two grammar rows describe capabilities the Notebook does not
//! yet have — block reordering (no ordering-persistence model) and prose blocks
//! (no prose-block model). They are still *in the table* (so the Atlas can show
//! them and their chords are reserved rather than silently free), but their
//! [`action_availability`] is [`ActionAvailability::DisabledWithReason`] with a
//! reason the wiring surfaces verbatim through an `aria-live` note — never a
//! faked success, never a model mutation (see the crate-root wiring). Every
//! reason carries the literal `ask #1` marker so a reviewer can grep the honest
//! degrades back to their design question.

/// Normalize a raw DOM `KeyboardEvent.key` string into the table's vocabulary:
/// a single alphabetic key is lowercased so its `Shift` state is carried ONLY by
/// [`KeyChord::shift`] (never by the key's case), while every multi-character
/// key (`"Enter"`, `"ArrowUp"`, `"Escape"`, `"F2"`) passes through unchanged.
/// This mirrors `dnacalc_skin_ir::keychord`'s own `normalize_key`, so a chord
/// built here from `"N"`+shift and one built from `"n"`+shift are the same key.
fn normalize_key(key: &str) -> String {
    let mut chars = key.chars();
    match (chars.next(), chars.next()) {
        (Some(ch), None) if ch.is_alphabetic() => ch.to_ascii_lowercase().to_string(),
        _ => key.to_string(),
    }
}

/// A normalized key chord: a key name plus the three modifier flags the Notebook
/// grammar distinguishes. `key` mirrors the DOM `KeyboardEvent.key` value except
/// single alphabetic keys are lowercased (see [`normalize_key`]).
///
/// The platform `meta` key (macOS `Cmd`) is NOT a distinct field — the wiring
/// folds it into `ctrl` before constructing the chord, matching the estate
/// convention (`dnacalc_shell::keyboard::chord_from_key_event`), so a Notebook
/// chord is layout-portable.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyChord {
    pub key: String,
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

impl KeyChord {
    /// Build a chord from raw parts, normalizing the key name. `ctrl` should
    /// already fold the platform `meta` key in (the wiring does this).
    #[must_use]
    pub fn new(key: impl AsRef<str>, shift: bool, ctrl: bool, alt: bool) -> Self {
        Self {
            key: normalize_key(key.as_ref()),
            shift,
            ctrl,
            alt,
        }
    }
}

/// The Notebook stage's closed set of keyboard actions. A binding row targets
/// exactly one of these; [`action_availability`] then decides whether the action
/// is live or an honest disabled-with-reason degrade.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NotebookAction {
    /// Open (focus) the focused block's editor — Jupyter's command→edit step.
    OpenEditor,
    /// Commit the focused block's in-progress edit, then advance focus to the
    /// next block.
    CommitAndAdvance,
    /// Move the focused block one position earlier. DEGRADE (no ordering model).
    ReorderUp,
    /// Move the focused block one position later. DEGRADE (no ordering model).
    ReorderDown,
    /// Insert a new prose block. DEGRADE (no prose-block model).
    NewProse,
    /// Open the name-first authoring form (S2.8's `+ name`). LIVE.
    NewName,
}

impl NotebookAction {
    /// A stable, telemetry-friendly id for the action (distinct from a binding
    /// id: many bindings could target one action, though today it is 1:1).
    #[must_use]
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::OpenEditor => "open-editor",
            Self::CommitAndAdvance => "commit-advance",
            Self::ReorderUp => "reorder-up",
            Self::ReorderDown => "reorder-down",
            Self::NewProse => "new-prose",
            Self::NewName => "new-name",
        }
    }
}

/// The reason string for the reorder degrade. Carries the literal `ask #1`
/// marker (see the module doc). Shared by both reorder directions so the
/// disabled-with-reason note reads identically for Up and Down.
pub const REORDER_DEFERRED_REASON: &str =
    "reorder not available — block ordering persistence deferred (ask #1)";

/// The reason string for the new-prose-block degrade. Carries the literal
/// `ask #1` marker (see the module doc).
pub const NEW_PROSE_DEFERRED_REASON: &str = "new prose block deferred (ask #1)";

/// Whether an action performs its effect ([`ActionAvailability::Live`]) or is an
/// honest disabled-with-reason degrade that must surface its reason and mutate
/// NOTHING ([`ActionAvailability::DisabledWithReason`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionAvailability {
    /// The action is wired and performs its effect.
    Live,
    /// The action is deliberately not built; the wiring must surface this reason
    /// verbatim (never fake success, never mutate the model). The reason always
    /// contains the literal substring `ask #1`.
    DisabledWithReason(&'static str),
}

/// One row of the atlas-taggable keymap: a stable id, the chord that triggers
/// it, the action it targets, and a human label. The `id` is the durable handle
/// a future Atlas stage keys a rebind on — it must never change once shipped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyBinding {
    pub id: &'static str,
    pub chord: KeyChord,
    pub action: NotebookAction,
    pub label: &'static str,
}

/// The Notebook stage's whole keyboard grammar, as one declarative table.
///
/// Exactly one binding per [`NotebookAction`] (asserted by the tests). The ids
/// are stable wire strings (`"notebook.<verb>"`), namespaced under `notebook.`
/// so an Atlas that aggregates several stages' keymaps keeps them unambiguous.
#[must_use]
pub fn notebook_keymap() -> Vec<KeyBinding> {
    vec![
        // Enter opens the focused block's editor (command→edit). Bare — no
        // modifiers — and distinguished from Shift+Enter only by the shift flag.
        KeyBinding {
            id: "notebook.open-editor",
            chord: KeyChord::new("Enter", false, false, false),
            action: NotebookAction::OpenEditor,
            label: "Open editor",
        },
        // Shift+Enter commits the focused block and advances to the next.
        KeyBinding {
            id: "notebook.commit-advance",
            chord: KeyChord::new("Enter", true, false, false),
            action: NotebookAction::CommitAndAdvance,
            label: "Commit and next block",
        },
        // Alt+ArrowUp / Alt+ArrowDown reorder — reserved chords, degraded.
        KeyBinding {
            id: "notebook.reorder-up",
            chord: KeyChord::new("ArrowUp", false, false, true),
            action: NotebookAction::ReorderUp,
            label: "Move block up",
        },
        KeyBinding {
            id: "notebook.reorder-down",
            chord: KeyChord::new("ArrowDown", false, false, true),
            action: NotebookAction::ReorderDown,
            label: "Move block down",
        },
        // Ctrl+Shift+M inserts a prose block — reserved chord, degraded.
        KeyBinding {
            id: "notebook.new-prose",
            chord: KeyChord::new("m", true, true, false),
            action: NotebookAction::NewProse,
            label: "New prose block",
        },
        // Ctrl+Shift+N opens the name-first authoring form (S2.8 landed) — LIVE.
        KeyBinding {
            id: "notebook.new-name",
            chord: KeyChord::new("n", true, true, false),
            action: NotebookAction::NewName,
            label: "New name block",
        },
    ]
}

/// Resolve a chord to its action against `keymap` — a pure, first-match lookup.
/// Returns `None` for an unbound chord (the wiring then lets the keystroke bubble
/// untouched to the shell, so universal verbs like F9/Ctrl+K still work).
#[must_use]
pub fn resolve_action(keymap: &[KeyBinding], chord: &KeyChord) -> Option<NotebookAction> {
    keymap
        .iter()
        .find(|binding| binding.chord == *chord)
        .map(|binding| binding.action)
}

/// Whether an action is live or an honest disabled-with-reason degrade.
///
/// `OpenEditor` / `CommitAndAdvance` / `NewName` are live (the editor grammar and
/// the S2.8 name form are built). `ReorderUp` / `ReorderDown` / `NewProse` are
/// disabled with a reason citing `ask #1`, because neither block-ordering
/// persistence nor a prose-block model exists — the wiring surfaces the reason
/// and mutates nothing.
#[must_use]
pub const fn action_availability(action: NotebookAction) -> ActionAvailability {
    match action {
        NotebookAction::OpenEditor | NotebookAction::CommitAndAdvance | NotebookAction::NewName => {
            ActionAvailability::Live
        }
        NotebookAction::ReorderUp | NotebookAction::ReorderDown => {
            ActionAvailability::DisabledWithReason(REORDER_DEFERRED_REASON)
        }
        NotebookAction::NewProse => {
            ActionAvailability::DisabledWithReason(NEW_PROSE_DEFERRED_REASON)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every declared chord resolves to its declared action — the spec's
    /// chord→action mapping, asserted row by row against the live table.
    #[test]
    fn every_declared_chord_resolves_to_its_action() {
        let keymap = notebook_keymap();
        for (chord, expected) in [
            (
                KeyChord::new("Enter", false, false, false),
                NotebookAction::OpenEditor,
            ),
            (
                KeyChord::new("Enter", true, false, false),
                NotebookAction::CommitAndAdvance,
            ),
            (
                KeyChord::new("ArrowUp", false, false, true),
                NotebookAction::ReorderUp,
            ),
            (
                KeyChord::new("ArrowDown", false, false, true),
                NotebookAction::ReorderDown,
            ),
            (
                KeyChord::new("m", true, true, false),
                NotebookAction::NewProse,
            ),
            (
                KeyChord::new("n", true, true, false),
                NotebookAction::NewName,
            ),
        ] {
            assert_eq!(
                resolve_action(&keymap, &chord),
                Some(expected),
                "chord {chord:?} must resolve to {expected:?}"
            );
        }
    }

    /// The normalizer lets an uppercase (shifted) alphabetic key match the same
    /// binding as its lowercase form: `Ctrl+Shift+N` arrives as `key="N"` on US
    /// layouts, and must still resolve to `NewName`.
    #[test]
    fn uppercase_alpha_key_normalizes_to_the_same_binding() {
        let keymap = notebook_keymap();
        assert_eq!(
            resolve_action(&keymap, &KeyChord::new("N", true, true, false)),
            Some(NotebookAction::NewName),
            "Ctrl+Shift+N (key=\"N\") must normalize to the new-name binding"
        );
        assert_eq!(
            resolve_action(&keymap, &KeyChord::new("M", true, true, false)),
            Some(NotebookAction::NewProse),
        );
        // Multi-character keys pass through unchanged.
        assert_eq!(KeyChord::new("ArrowUp", false, false, true).key, "ArrowUp");
    }

    /// A chord not in the table resolves to `None` — the wiring must let it
    /// bubble, never swallow it.
    #[test]
    fn non_matching_chord_resolves_to_none() {
        let keymap = notebook_keymap();
        // Bare `q` — bound to nothing.
        assert_eq!(
            resolve_action(&keymap, &KeyChord::new("q", false, false, false)),
            None
        );
        // Bare ArrowUp (no Alt) is NOT the reorder chord.
        assert_eq!(
            resolve_action(&keymap, &KeyChord::new("ArrowUp", false, false, false)),
            None
        );
        // Ctrl+N (no Shift) is NOT the new-name chord.
        assert_eq!(
            resolve_action(&keymap, &KeyChord::new("n", false, true, false)),
            None
        );
    }

    /// Availability: the three built actions are live; the three deferred ones
    /// are disabled-with-reason, and every reason carries the `ask #1` marker so
    /// the honest degrade is greppable.
    #[test]
    fn availability_is_live_for_built_and_reasoned_for_deferred() {
        for live in [
            NotebookAction::OpenEditor,
            NotebookAction::CommitAndAdvance,
            NotebookAction::NewName,
        ] {
            assert_eq!(
                action_availability(live),
                ActionAvailability::Live,
                "{live:?} must be live"
            );
        }
        for deferred in [
            NotebookAction::ReorderUp,
            NotebookAction::ReorderDown,
            NotebookAction::NewProse,
        ] {
            match action_availability(deferred) {
                ActionAvailability::DisabledWithReason(reason) => assert!(
                    reason.contains("ask #1"),
                    "{deferred:?} degrade reason must cite `ask #1`, got {reason:?}"
                ),
                ActionAvailability::Live => {
                    panic!("{deferred:?} must be a disabled-with-reason degrade, not live")
                }
            }
        }
    }

    /// The table is well-formed: exactly one binding per action, and every
    /// binding id is unique and stable (`notebook.<verb>`). This is the
    /// invariant a future Atlas rebind surface relies on.
    #[test]
    fn keymap_has_one_stable_unique_binding_per_action() {
        let keymap = notebook_keymap();

        // One binding per action — every action appears exactly once.
        for action in [
            NotebookAction::OpenEditor,
            NotebookAction::CommitAndAdvance,
            NotebookAction::ReorderUp,
            NotebookAction::ReorderDown,
            NotebookAction::NewProse,
            NotebookAction::NewName,
        ] {
            let count = keymap
                .iter()
                .filter(|binding| binding.action == action)
                .count();
            assert_eq!(
                count, 1,
                "exactly one binding for {action:?}, found {count}"
            );
        }
        // No action is bound more than we enumerated: table length equals the
        // number of distinct actions.
        assert_eq!(keymap.len(), 6, "the keymap has exactly six rows");

        // Ids are unique.
        let mut ids: Vec<&str> = keymap.iter().map(|binding| binding.id).collect();
        ids.sort_unstable();
        let unique = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), unique, "every binding id must be unique");

        // Ids are the stable, namespaced strings the Atlas keys rebinds on.
        let expected = [
            "notebook.commit-advance",
            "notebook.new-name",
            "notebook.new-prose",
            "notebook.open-editor",
            "notebook.reorder-down",
            "notebook.reorder-up",
        ];
        assert_eq!(
            ids, expected,
            "binding ids drifted from their stable values"
        );
    }
}
