//! The shell keyboard grammar — SHELL_SPEC.md §5, seeded EXACTLY with the
//! §5.1 universal verb table (primaries + browser alternates), plus the
//! hard-reserved chord law and the §5 guard order.
//!
//! One registry instance is the single source of truth: the keydown
//! dispatcher resolves through it and the keyboard atlas renders from it
//! ([`ShellKeyboardRegistry::atlas_groups`]) — the two cannot drift.
//!
//! Region-scoped verb families exist only where the composed shell has the
//! region ([`CatalogComposition`]): an omitted region contributes zero
//! verbs to the catalog.

use dnacalc_skin_ir::keychord::{KeyChord, KeybindingOverrideMap, SkinVerb};

use crate::composition::CatalogComposition;
use crate::profile::RuntimeContext;
use crate::stage::StageId;

/// Where a binding lives in the atlas grouping (region/stage grouping per
/// SHELL_SPEC §5.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChordScope {
    /// The universal table — active in every composition.
    Universal,
    /// Registry-rail verbs (present only when the Registry region is).
    Registry,
    /// Inspector verbs (present only when the Inspector region is).
    Inspector,
    /// Stage-switcher slots (present only when the mast composes the
    /// switcher).
    Stages,
    /// Stage-local verbs registered under the focused stage.
    Stage(StageId),
}

impl ChordScope {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Universal => "Universal",
            Self::Registry => "Registry",
            Self::Inspector => "Inspector",
            Self::Stages => "Stages",
            Self::Stage(id) => match id {
                StageId::Sheet => "Sheet stage",
                StageId::Model => "Model stage",
                StageId::Notebook => "Notebook stage",
                StageId::Atlas => "Atlas stage",
                StageId::BenchResult => "Result stage",
            },
        }
    }
}

/// Which column of the SHELL_SPEC §5.1 table a chord came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BindingTier {
    /// The primary chord (or a notes-sanctioned extra primary such as
    /// Ctrl+Shift+Z for Redo).
    Primary,
    /// A browser alternate — always active, tagged "browser" in the atlas
    /// (every hard-reserved divergence must appear there with that tag).
    BrowserAlternate,
    /// A desktop-only addition (bound only when the runtime is not a
    /// browser; e.g. Ctrl+1..4 stage slots).
    DesktopOnly,
}

impl BindingTier {
    /// The divergence tag the atlas renders, if any.
    #[must_use]
    pub const fn atlas_tag(self) -> Option<&'static str> {
        match self {
            Self::Primary => None,
            Self::BrowserAlternate => Some("browser"),
            Self::DesktopOnly => Some("desktop"),
        }
    }
}

/// One chord→verb binding with its atlas metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellBinding {
    pub chord: KeyChord,
    pub verb: SkinVerb,
    pub tier: BindingTier,
    pub scope: ChordScope,
}

/// Why a chord could not be bound or a rebind rejected. Typed — a caller
/// can render each case honestly.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ShellKeybindingError {
    #[error(
        "chord {} is hard-reserved for the browser and can never be bound",
        chord_display(chord)
    )]
    HardReserved { chord: KeyChord },
    #[error("chord {} is already bound to {}", chord_display(chord), occupied_by.stable_id())]
    Collision {
        chord: KeyChord,
        occupied_by: SkinVerb,
    },
    #[error("override names a verb this build cannot rebind: {0}")]
    UnknownVerb(String),
    #[error("stage switching is not rebindable")]
    NotRebindable,
}

/// The hard-reserved list (SHELL_SPEC §5.1): chords the browser owns.
/// Never bound, never intercepted in a browser runtime. Exact chords —
/// Ctrl+Alt+1..4 is *not* in this list (the spec calls the Ctrl+Alt
/// primaries "already safe").
#[must_use]
pub fn hard_reserved_chords() -> Vec<KeyChord> {
    let mut chords = Vec::new();
    for key in ["w", "t", "n"] {
        chords.push(KeyChord::ctrl(key));
        chords.push(KeyChord::from_parts(key, true, false, true));
    }
    chords.push(KeyChord::ctrl("Tab"));
    for digit in 1..=9u8 {
        chords.push(KeyChord::ctrl(digit.to_string()));
    }
    chords.push(KeyChord::ctrl("PageUp"));
    chords.push(KeyChord::ctrl("PageDown"));
    chords.push(KeyChord::bare("F5"));
    chords.push(KeyChord::ctrl("r"));
    chords.push(KeyChord::ctrl("l"));
    chords.push(KeyChord::bare("F11"));
    chords.push(KeyChord::bare("F6"));
    chords
}

/// Is this chord on the hard-reserved list?
#[must_use]
pub fn is_hard_reserved(chord: &KeyChord) -> bool {
    hard_reserved_chords().contains(chord)
}

/// The hard-reserved chords the spec explicitly re-sanctions on desktop:
/// Ctrl+1..4 (stage slots, §5.1) and Ctrl+PgUp/PgDn (sheet-tab cycling,
/// SHEET_SPEC's desktop addition). Everything else on the hard-reserved
/// list stays rejected even on desktop — muscle memory must not fork
/// further than the spec says.
#[must_use]
pub fn is_desktop_sanctioned(chord: &KeyChord) -> bool {
    if *chord == KeyChord::ctrl("PageUp") || *chord == KeyChord::ctrl("PageDown") {
        return true;
    }
    (1..=4u8).any(|digit| *chord == KeyChord::ctrl(digit.to_string()))
}

/// The one shell keybinding table: SHELL_SPEC §5.1, seeded per composition
/// and runtime.
#[derive(Debug, Clone)]
pub struct ShellKeyboardRegistry {
    runtime: RuntimeContext,
    bindings: Vec<ShellBinding>,
}

impl ShellKeyboardRegistry {
    /// Seed the §5.1 universal table for `composition` under `runtime`.
    #[must_use]
    pub fn universal(composition: CatalogComposition, runtime: RuntimeContext) -> Self {
        let mut registry = Self {
            runtime,
            bindings: Vec::new(),
        };
        registry.seed(composition);
        registry
    }

    fn seed_binding(
        &mut self,
        chord: KeyChord,
        verb: SkinVerb,
        tier: BindingTier,
        scope: ChordScope,
    ) {
        debug_assert!(
            !(self.runtime.is_browser() && is_hard_reserved(&chord)),
            "seed table must never place a hard-reserved chord in a browser registry"
        );
        self.bindings.push(ShellBinding {
            chord,
            verb,
            tier,
            scope,
        });
    }

    /// The §5.1 table, verbatim. Primaries and browser alternates are both
    /// always active; desktop-only additions only exist off-browser.
    fn seed(&mut self, composition: CatalogComposition) {
        use BindingTier::{BrowserAlternate, DesktopOnly, Primary};
        use ChordScope::{Inspector, Registry, Stages, Universal};

        // --- Universal rows (§5.1, top to bottom) ---
        self.seed_binding(
            KeyChord::bare("Enter"),
            SkinVerb::Commit,
            Primary,
            Universal,
        );
        self.seed_binding(
            KeyChord::bare("F2"),
            SkinVerb::EditInPlace,
            Primary,
            Universal,
        );
        self.seed_binding(
            KeyChord::bare("Escape"),
            SkinVerb::Escape,
            Primary,
            Universal,
        );
        self.seed_binding(
            KeyChord::bare("F9"),
            SkinVerb::Recalculate,
            Primary,
            Universal,
        );
        self.seed_binding(KeyChord::ctrl("z"), SkinVerb::Undo, Primary, Universal);
        self.seed_binding(KeyChord::ctrl("y"), SkinVerb::Redo, Primary, Universal);
        // Notes column: "Ctrl+Shift+Z also Redo".
        self.seed_binding(
            KeyChord::from_parts("z", true, false, true),
            SkinVerb::Redo,
            Primary,
            Universal,
        );
        self.seed_binding(
            KeyChord::ctrl("k"),
            SkinVerb::CommandDeck,
            Primary,
            Universal,
        );
        // "Ctrl+Shift+P mirror — both always active".
        self.seed_binding(
            KeyChord::from_parts("p", true, false, true),
            SkinVerb::CommandDeck,
            BrowserAlternate,
            Universal,
        );
        self.seed_binding(
            KeyChord::ctrl("/"),
            SkinVerb::KeyboardAtlas,
            Primary,
            Universal,
        );
        // "? (when not editing)" — the text-entry guard supplies the
        // "when not editing" clause; the chord itself is the bare `?` key.
        self.seed_binding(
            KeyChord::bare("?"),
            SkinVerb::KeyboardAtlas,
            BrowserAlternate,
            Universal,
        );
        self.seed_binding(KeyChord::ctrl("h"), SkinVerb::Timeline, Primary, Universal);
        self.seed_binding(KeyChord::bare("p"), SkinVerb::Peek, Primary, Universal);
        self.seed_binding(KeyChord::ctrl("s"), SkinVerb::Save, Primary, Universal);
        self.seed_binding(KeyChord::ctrl("o"), SkinVerb::Open, Primary, Universal);
        self.seed_binding(KeyChord::ctrl("f"), SkinVerb::FindGoto, Primary, Universal);
        self.seed_binding(KeyChord::bare("/"), SkinVerb::NameBox, Primary, Universal);
        self.seed_binding(
            KeyChord::bare("]"),
            SkinVerb::TraceForward,
            Primary,
            Universal,
        );
        self.seed_binding(KeyChord::bare("["), SkinVerb::TraceBack, Primary, Universal);
        // F8 toggles the Bench X-Ray drill panel (BENCH_SPEC §9). It is a
        // function key, so the §5 guard-order exemption lets it fire from
        // inside the Bridge edit buffer (like F9); products without an X-Ray
        // surface receive the forwarded verb and ignore it.
        self.seed_binding(
            KeyChord::bare("F8"),
            SkinVerb::FormulaXRay,
            Primary,
            Universal,
        );
        self.seed_binding(KeyChord::bare("e"), SkinVerb::Explain, Primary, Universal);
        self.seed_binding(
            KeyChord::bare("ArrowLeft"),
            SkinVerb::Fold,
            Primary,
            Universal,
        );
        self.seed_binding(
            KeyChord::bare("ArrowRight"),
            SkinVerb::Unfold,
            Primary,
            Universal,
        );

        // --- Region rows: omitted regions contribute zero verbs ---
        if composition.registry_region {
            self.seed_binding(
                KeyChord::ctrl("b"),
                SkinVerb::RegistryToggle,
                Primary,
                Registry,
            );
            self.seed_binding(
                KeyChord::from_parts("b", false, true, false),
                SkinVerb::RegistryToggle,
                BrowserAlternate,
                Registry,
            );
        }
        if composition.inspector_region {
            self.seed_binding(
                KeyChord::ctrl("i"),
                SkinVerb::InspectorToggle,
                Primary,
                Inspector,
            );
            self.seed_binding(
                KeyChord::from_parts("i", false, true, false),
                SkinVerb::InspectorToggle,
                BrowserAlternate,
                Inspector,
            );
        }

        // --- Stage slots: Ctrl+Alt+1..4 primary (already browser-safe);
        // Ctrl+1..4 additionally bound on desktop only ---
        for slot in 1..=composition.stage_slots.min(4) {
            self.seed_binding(
                KeyChord::from_parts(slot.to_string(), true, true, false),
                SkinVerb::SwitchLens(slot),
                Primary,
                Stages,
            );
            if !self.runtime.is_browser() {
                self.seed_binding(
                    KeyChord::ctrl(slot.to_string()),
                    SkinVerb::SwitchLens(slot),
                    DesktopOnly,
                    Stages,
                );
            }
        }
    }

    /// The §5.1 table with a user's persisted overrides layered on
    /// (SHELL_SPEC §5.2: collision-revalidated on write; §5.1: everything
    /// rebindable except stage switching).
    ///
    /// Each overridden verb's chords (primary + alternates) collapse to the
    /// single remapped chord; the result is re-verified collision-free and
    /// hard-reserved-free. An override naming a verb this composition does
    /// not bind is *kept inert* (it applies in compositions that do bind the
    /// verb) rather than erroring — the persisted map is product-portable.
    pub fn with_overrides(
        composition: CatalogComposition,
        runtime: RuntimeContext,
        overrides: &KeybindingOverrideMap,
    ) -> Result<Self, ShellKeybindingError> {
        let mut remapped: Vec<(SkinVerb, KeyChord)> = Vec::new();
        for (id, chord) in overrides.iter() {
            let verb = SkinVerb::from_stable_id(id)
                .ok_or_else(|| ShellKeybindingError::UnknownVerb(id.clone()))?;
            // Same predicate as `register_stage_verb`: hard-reserved chords
            // are rejected unconditionally in a browser runtime, and on
            // desktop everything hard-reserved stays rejected EXCEPT the two
            // spec-sanctioned desktop families (`is_desktop_sanctioned`).
            // Gating this behind `runtime.is_browser()` alone (the prior
            // bug, dtc-1tk.2) rejected nothing at all on desktop, so a
            // persisted override could remap a shell verb onto F5, Ctrl+R,
            // F11, Ctrl+L, Ctrl+W/T/N, or Ctrl+Tab.
            if is_hard_reserved(chord) && (runtime.is_browser() || !is_desktop_sanctioned(chord)) {
                return Err(ShellKeybindingError::HardReserved {
                    chord: chord.clone(),
                });
            }
            remapped.push((verb, chord.clone()));
        }

        let seeded = Self::universal(composition, runtime);
        let mut bindings: Vec<ShellBinding> = Vec::new();
        let mut emitted: Vec<SkinVerb> = Vec::new();
        for binding in seeded.bindings {
            if let Some((_, new_chord)) = remapped
                .iter()
                .find(|(candidate, _)| *candidate == binding.verb)
            {
                if !emitted.contains(&binding.verb) {
                    emitted.push(binding.verb);
                    bindings.push(ShellBinding {
                        chord: new_chord.clone(),
                        verb: binding.verb,
                        tier: BindingTier::Primary,
                        scope: binding.scope,
                    });
                }
            } else {
                bindings.push(binding);
            }
        }

        let registry = Self { runtime, bindings };
        registry.verify_collision_free()?;
        Ok(registry)
    }

    /// Register a stage-local verb under `stage` (SHELL_SPEC §5: "stage-local
    /// verbs register under the focused stage"). Rejects hard-reserved
    /// chords with a typed error — in a browser runtime unconditionally; on
    /// desktop everything hard-reserved stays rejected except the two
    /// spec-sanctioned desktop families ([`is_desktop_sanctioned`]).
    pub fn register_stage_verb(
        &mut self,
        stage: StageId,
        verb: SkinVerb,
        chord: KeyChord,
        tier: BindingTier,
    ) -> Result<(), ShellKeybindingError> {
        if is_hard_reserved(&chord) && (self.runtime.is_browser() || !is_desktop_sanctioned(&chord))
        {
            return Err(ShellKeybindingError::HardReserved { chord });
        }
        if let Some(existing) = self.resolve(&chord)
            && existing != verb
        {
            return Err(ShellKeybindingError::Collision {
                chord,
                occupied_by: existing,
            });
        }
        self.bindings.push(ShellBinding {
            chord,
            verb,
            tier,
            scope: ChordScope::Stage(stage),
        });
        Ok(())
    }

    /// Validate a proposed rebind against this live table and produce the
    /// new override map for the audited `SetKeybindingOverrides` write.
    /// Never mutates `self` — the caller applies the returned map through
    /// the shared-state chokepoint and rebuilds the registry from it.
    pub fn rebind_validated(
        &self,
        current: &KeybindingOverrideMap,
        verb: SkinVerb,
        chord: KeyChord,
    ) -> Result<KeybindingOverrideMap, ShellKeybindingError> {
        if SkinVerb::from_stable_id(verb.stable_id()) != Some(verb) {
            // SwitchLens — §5.1: "all rebindable except stage switching".
            return Err(ShellKeybindingError::NotRebindable);
        }
        // Same predicate as `register_stage_verb` (and `with_overrides`
        // above): browser rejects every hard-reserved chord; desktop
        // rejects every hard-reserved chord except the two spec-sanctioned
        // desktop families. Previously gated behind `self.runtime.is_browser()`
        // alone, which rejected nothing on desktop (dtc-1tk.2).
        if is_hard_reserved(&chord) && (self.runtime.is_browser() || !is_desktop_sanctioned(&chord))
        {
            return Err(ShellKeybindingError::HardReserved { chord });
        }
        if let Some(occupied_by) = self.resolve(&chord)
            && occupied_by != verb
        {
            return Err(ShellKeybindingError::Collision { chord, occupied_by });
        }
        let mut next = current.clone();
        next.set(verb, chord);
        Ok(next)
    }

    fn verify_collision_free(&self) -> Result<(), ShellKeybindingError> {
        for (index, binding) in self.bindings.iter().enumerate() {
            for other in &self.bindings[index + 1..] {
                if binding.chord == other.chord && binding.verb != other.verb {
                    return Err(ShellKeybindingError::Collision {
                        chord: binding.chord.clone(),
                        occupied_by: binding.verb,
                    });
                }
            }
        }
        Ok(())
    }

    /// Resolve a chord to its verb, if bound.
    #[must_use]
    pub fn resolve(&self, chord: &KeyChord) -> Option<SkinVerb> {
        self.bindings
            .iter()
            .find(|binding| binding.chord == *chord)
            .map(|binding| binding.verb)
    }

    /// Every binding, for the atlas and for tests.
    #[must_use]
    pub fn bindings(&self) -> &[ShellBinding] {
        &self.bindings
    }

    /// The first chord bound to `verb`, for hint rendering.
    #[must_use]
    pub fn primary_chord(&self, verb: SkinVerb) -> Option<&KeyChord> {
        self.bindings
            .iter()
            .find(|binding| binding.verb == verb)
            .map(|binding| &binding.chord)
    }

    #[must_use]
    pub const fn runtime(&self) -> RuntimeContext {
        self.runtime
    }

    /// The atlas view of this registry: every binding, grouped by scope, in
    /// seed order. Rendered *from the live instance* (SHELL_SPEC §5.2:
    /// "the atlas renders from the same registry the dispatcher uses — it
    /// cannot drift").
    #[must_use]
    pub fn atlas_groups(&self) -> Vec<AtlasGroup> {
        let mut groups: Vec<AtlasGroup> = Vec::new();
        for binding in &self.bindings {
            let row = AtlasRow {
                verb: binding.verb,
                verb_label: verb_label(binding.verb),
                chord_display: chord_display(&binding.chord),
                tag: binding.tier.atlas_tag(),
                rebindable: SkinVerb::from_stable_id(binding.verb.stable_id())
                    == Some(binding.verb),
            };
            match groups.iter_mut().find(|group| group.scope == binding.scope) {
                Some(group) => group.rows.push(row),
                None => groups.push(AtlasGroup {
                    scope: binding.scope,
                    label: binding.scope.label(),
                    rows: vec![row],
                }),
            }
        }
        groups
    }
}

/// One atlas row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtlasRow {
    pub verb: SkinVerb,
    pub verb_label: &'static str,
    pub chord_display: String,
    /// "browser" / "desktop" divergence tag, if any.
    pub tag: Option<&'static str>,
    pub rebindable: bool,
}

/// One atlas group (region/stage grouping).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtlasGroup {
    pub scope: ChordScope,
    pub label: &'static str,
    pub rows: Vec<AtlasRow>,
}

/// Human label for a verb, as the atlas and command surfaces render it.
#[must_use]
pub fn verb_label(verb: SkinVerb) -> &'static str {
    match verb {
        SkinVerb::Commit => "Commit",
        SkinVerb::EditInPlace => "Edit in place",
        SkinVerb::Escape => "Escape / revert",
        SkinVerb::Recalculate => "Recalculate",
        SkinVerb::Undo => "Undo",
        SkinVerb::Redo => "Redo",
        SkinVerb::CommandDeck => "Command deck",
        SkinVerb::KeyboardAtlas => "Keyboard atlas",
        SkinVerb::Timeline => "Timeline",
        SkinVerb::Peek => "Peek",
        SkinVerb::RegistryToggle => "Registry toggle",
        SkinVerb::InspectorToggle => "Inspector toggle",
        SkinVerb::SwitchLens(_) => "Stage switch",
        SkinVerb::Save => "Save",
        SkinVerb::Open => "Open",
        SkinVerb::FindGoto => "Find / goto",
        SkinVerb::FormulaXRay => "Formula X-Ray",
        SkinVerb::NameBox => "Name box",
        SkinVerb::TraceForward => "Trace forward",
        SkinVerb::TraceBack => "Trace back",
        SkinVerb::Explain => "Explain",
        SkinVerb::Fold => "Fold",
        SkinVerb::Unfold => "Unfold",
        SkinVerb::Fill => "Fill",
        SkinVerb::Leader => "Leader",
        SkinVerb::NavPrev => "Previous",
        SkinVerb::NavNext => "Next",
        SkinVerb::ToParent => "To parent",
        SkinVerb::ToChild => "To child",
        SkinVerb::NewWorkspace => "New workspace",
        SkinVerb::ClearContents => "Clear contents",
    }
}

/// Display form of a chord ("Ctrl+K", "Ctrl+Alt+1", "?", "F9", "Esc", "←").
#[must_use]
pub fn chord_display(chord: &KeyChord) -> String {
    let mut parts: Vec<String> = Vec::new();
    if chord.ctrl {
        parts.push("Ctrl".to_string());
    }
    if chord.alt {
        parts.push("Alt".to_string());
    }
    if chord.shift {
        parts.push("Shift".to_string());
    }
    let key = match chord.key.as_str() {
        "Escape" => "Esc".to_string(),
        "ArrowLeft" => "\u{2190}".to_string(),
        "ArrowRight" => "\u{2192}".to_string(),
        "ArrowUp" => "\u{2191}".to_string(),
        "ArrowDown" => "\u{2193}".to_string(),
        " " => "Space".to_string(),
        single if single.chars().count() == 1 => single.to_uppercase(),
        other => other.to_string(),
    };
    parts.push(key);
    parts.join("+")
}

/// Normalize a raw DOM keydown into the registry's chord vocabulary:
/// `meta` folds into `ctrl` (estate convention), and for printable
/// single-character non-alphabetic keys the character itself already
/// carries the shift state (`?`, `{`, …), so the shift flag is cleared —
/// the §5.1 `?` alternate matches whatever layout produced it.
#[must_use]
pub fn chord_from_key_event(key: &str, ctrl: bool, meta: bool, alt: bool, shift: bool) -> KeyChord {
    let mut chars = key.chars();
    let single_non_alphabetic = matches!(
        (chars.next(), chars.next()),
        (Some(ch), None) if !ch.is_alphabetic()
    );
    let shift = if single_non_alphabetic { false } else { shift };
    KeyChord::from_parts(key, ctrl || meta, alt, shift)
}

/// `F1`..`F24` — the guard-order exemption class (F9 must work from inside
/// edit buffers).
#[must_use]
pub fn is_function_key(key: &str) -> bool {
    key.len() >= 2 && key.starts_with('F') && key[1..].chars().all(|ch| ch.is_ascii_digit())
}

/// Where a keydown ended up after the §5 guard order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyRouting {
    /// The user is typing and the chord belongs to the edit buffer
    /// (bare/shift-only, non-function-key). The shell must do nothing.
    SuppressedByTextEntry,
    /// Not bound in this registry.
    Unbound,
    /// Resolved to a verb.
    Verb(SkinVerb),
}

/// The §5 guard order as one pure function: `event_target_is_text_entry`
/// first, F-key exemptions second (F9 works inside edit buffers; modified
/// chords also pass — Ctrl+K must open the deck from inside the Bridge
/// editor), chord lookup third.
#[must_use]
pub fn route_key(
    registry: &ShellKeyboardRegistry,
    is_text_entry: bool,
    chord: &KeyChord,
) -> KeyRouting {
    if is_text_entry && !chord.ctrl && !chord.alt && !is_function_key(&chord.key) {
        return KeyRouting::SuppressedByTextEntry;
    }
    match registry.resolve(chord) {
        Some(verb) => KeyRouting::Verb(verb),
        None => KeyRouting::Unbound,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn calc_composition() -> CatalogComposition {
        CatalogComposition {
            registry_region: true,
            inspector_region: true,
            stage_slots: 4,
        }
    }

    fn bench_composition() -> CatalogComposition {
        CatalogComposition {
            registry_region: false,
            inspector_region: true,
            stage_slots: 0,
        }
    }

    fn browser_calc() -> ShellKeyboardRegistry {
        ShellKeyboardRegistry::universal(calc_composition(), RuntimeContext::Browser)
    }

    #[test]
    fn full_table_is_collision_free_in_every_composition_and_runtime() {
        for composition in [calc_composition(), bench_composition()] {
            for runtime in [RuntimeContext::Browser, RuntimeContext::Desktop] {
                let registry = ShellKeyboardRegistry::universal(composition, runtime);
                registry
                    .verify_collision_free()
                    .expect("seeded §5.1 table must be collision-free");
            }
        }
    }

    #[test]
    fn browser_table_never_contains_a_hard_reserved_chord() {
        for binding in browser_calc().bindings() {
            assert!(
                !is_hard_reserved(&binding.chord),
                "browser registry contains hard-reserved chord {:?}",
                binding.chord
            );
        }
    }

    #[test]
    fn registering_every_hard_reserved_chord_is_a_typed_error_in_browser() {
        let mut registry = browser_calc();
        for chord in hard_reserved_chords() {
            let result = registry.register_stage_verb(
                StageId::Sheet,
                SkinVerb::Explain,
                chord.clone(),
                BindingTier::Primary,
            );
            assert_eq!(
                result,
                Err(ShellKeybindingError::HardReserved { chord }),
                "hard-reserved chord must be rejected with the typed error"
            );
        }
    }

    #[test]
    fn desktop_allows_only_the_sanctioned_hard_reserved_chords() {
        let mut registry =
            ShellKeyboardRegistry::universal(bench_composition(), RuntimeContext::Desktop);
        // Sanctioned: Ctrl+PgUp/PgDn (SHEET_SPEC's desktop sheet-tab cycle).
        registry
            .register_stage_verb(
                StageId::Sheet,
                SkinVerb::NavPrev,
                KeyChord::ctrl("PageUp"),
                BindingTier::DesktopOnly,
            )
            .expect("Ctrl+PageUp is desktop-sanctioned");
        // Not sanctioned even on desktop: Ctrl+W, F5, Ctrl+L, ...
        for chord in [
            KeyChord::ctrl("w"),
            KeyChord::bare("F5"),
            KeyChord::ctrl("l"),
            KeyChord::ctrl("Tab"),
            KeyChord::bare("F6"),
        ] {
            assert!(matches!(
                registry.register_stage_verb(
                    StageId::Sheet,
                    SkinVerb::Explain,
                    chord,
                    BindingTier::DesktopOnly,
                ),
                Err(ShellKeybindingError::HardReserved { .. })
            ));
        }
    }

    #[test]
    fn desktop_rebind_and_overrides_use_the_same_sanctioned_hard_reserved_predicate() {
        // Bead dtc-1tk.2: `rebind_validated` and `with_overrides` gated
        // hard-reserved rejection behind `runtime.is_browser()` alone, so on
        // `RuntimeContext::Desktop` they rejected NOTHING at all — a user
        // could rebind a shell verb onto F5, Ctrl+R, F11, Ctrl+L,
        // Ctrl+W/T/N, or Ctrl+Tab. This mirrors
        // `desktop_allows_only_the_sanctioned_hard_reserved_chords` (which
        // only exercised `register_stage_verb`) for the two rebind entry
        // points, and follows each acceptance all the way through
        // `with_overrides` to a resolvable chord (not a vacuous `Ok(..)`).
        let registry =
            ShellKeyboardRegistry::universal(bench_composition(), RuntimeContext::Desktop);
        let current = KeybindingOverrideMap::new();

        // Not sanctioned even on desktop: rebind_validated must still reject.
        assert_eq!(
            registry.rebind_validated(&current, SkinVerb::Explain, KeyChord::bare("F5")),
            Err(ShellKeybindingError::HardReserved {
                chord: KeyChord::bare("F5"),
            }),
            "F5 must stay rejected by rebind_validated even on desktop"
        );
        assert_eq!(
            registry.rebind_validated(&current, SkinVerb::Explain, KeyChord::ctrl("r")),
            Err(ShellKeybindingError::HardReserved {
                chord: KeyChord::ctrl("r"),
            }),
            "Ctrl+R must stay rejected by rebind_validated even on desktop"
        );

        // Sanctioned on desktop: rebind_validated accepts, and the override
        // it produces round-trips through with_overrides to a registry that
        // actually resolves the new chord.
        let with_ctrl_1 = registry
            .rebind_validated(&current, SkinVerb::Explain, KeyChord::ctrl("1"))
            .expect("Ctrl+1 is desktop-sanctioned and must be accepted");
        let rebuilt = ShellKeyboardRegistry::with_overrides(
            bench_composition(),
            RuntimeContext::Desktop,
            &with_ctrl_1,
        )
        .expect("with_overrides must accept the desktop-sanctioned override it just produced");
        assert_eq!(
            rebuilt.resolve(&KeyChord::ctrl("1")),
            Some(SkinVerb::Explain)
        );

        let with_ctrl_pageup = registry
            .rebind_validated(&current, SkinVerb::Explain, KeyChord::ctrl("PageUp"))
            .expect("Ctrl+PageUp is desktop-sanctioned and must be accepted");
        let rebuilt = ShellKeyboardRegistry::with_overrides(
            bench_composition(),
            RuntimeContext::Desktop,
            &with_ctrl_pageup,
        )
        .expect("with_overrides must accept Ctrl+PageUp on desktop");
        assert_eq!(
            rebuilt.resolve(&KeyChord::ctrl("PageUp")),
            Some(SkinVerb::Explain)
        );

        // The same two not-sanctioned chords, fed directly into
        // with_overrides (the persisted-map path, rather than the live
        // rebind_validated path), must also be rejected on desktop — this is
        // the second call site the bug lived in.
        let mut f5_override = KeybindingOverrideMap::new();
        f5_override.set(SkinVerb::Recalculate, KeyChord::bare("F5"));
        assert!(
            matches!(
                ShellKeyboardRegistry::with_overrides(
                    bench_composition(),
                    RuntimeContext::Desktop,
                    &f5_override,
                ),
                Err(ShellKeybindingError::HardReserved { .. })
            ),
            "with_overrides must reject a persisted F5 override on desktop, not just in the browser"
        );

        let mut ctrl_r_override = KeybindingOverrideMap::new();
        ctrl_r_override.set(SkinVerb::Recalculate, KeyChord::ctrl("r"));
        assert!(
            matches!(
                ShellKeyboardRegistry::with_overrides(
                    bench_composition(),
                    RuntimeContext::Desktop,
                    &ctrl_r_override,
                ),
                Err(ShellKeybindingError::HardReserved { .. })
            ),
            "with_overrides must reject a persisted Ctrl+R override on desktop"
        );
    }

    #[test]
    fn omitted_regions_contribute_zero_verbs_to_the_catalog() {
        let bench = ShellKeyboardRegistry::universal(bench_composition(), RuntimeContext::Browser);
        assert!(
            bench
                .bindings()
                .iter()
                .all(|binding| binding.verb != SkinVerb::RegistryToggle),
            "no Registry region => no RegistryToggle verb"
        );
        assert!(
            bench
                .bindings()
                .iter()
                .all(|binding| !matches!(binding.verb, SkinVerb::SwitchLens(_))),
            "no stage switcher => no stage-switch verbs"
        );
        assert_eq!(bench.resolve(&KeyChord::ctrl("b")), None);
        // Inspector is composed in Bench, so its verbs are present.
        assert_eq!(
            bench.resolve(&KeyChord::ctrl("i")),
            Some(SkinVerb::InspectorToggle)
        );

        let calc = browser_calc();
        assert_eq!(
            calc.resolve(&KeyChord::ctrl("b")),
            Some(SkinVerb::RegistryToggle)
        );
        assert_eq!(
            calc.resolve(&KeyChord::from_parts("1", true, true, false)),
            Some(SkinVerb::SwitchLens(1))
        );
    }

    /// The §5.1 catalog snapshot for the Calc composition in a browser:
    /// scope / verb / chord / tag, in seed order. A diff here means the
    /// universal table moved — that must be a deliberate spec change.
    #[test]
    fn catalog_snapshot_matches_shell_spec_5_1() {
        let registry = browser_calc();
        let flat: Vec<String> = registry
            .bindings()
            .iter()
            .map(|binding| {
                format!(
                    "{}|{}|{}|{}",
                    binding.scope.label(),
                    binding.verb.stable_id(),
                    chord_display(&binding.chord),
                    binding.tier.atlas_tag().unwrap_or("-")
                )
            })
            .collect();
        let expected = [
            "Universal|commit|Enter|-",
            "Universal|edit_in_place|F2|-",
            "Universal|escape|Esc|-",
            "Universal|recalculate|F9|-",
            "Universal|undo|Ctrl+Z|-",
            "Universal|redo|Ctrl+Y|-",
            "Universal|redo|Ctrl+Shift+Z|-",
            "Universal|command_deck|Ctrl+K|-",
            "Universal|command_deck|Ctrl+Shift+P|browser",
            "Universal|keyboard_atlas|Ctrl+/|-",
            "Universal|keyboard_atlas|?|browser",
            "Universal|timeline|Ctrl+H|-",
            "Universal|peek|P|-",
            "Universal|save|Ctrl+S|-",
            "Universal|open|Ctrl+O|-",
            "Universal|find_goto|Ctrl+F|-",
            "Universal|name_box|/|-",
            "Universal|trace_forward|]|-",
            "Universal|trace_back|[|-",
            "Universal|formula_xray|F8|-",
            "Universal|explain|E|-",
            "Universal|fold|\u{2190}|-",
            "Universal|unfold|\u{2192}|-",
            "Registry|registry_toggle|Ctrl+B|-",
            "Registry|registry_toggle|Alt+B|browser",
            "Inspector|inspector_toggle|Ctrl+I|-",
            "Inspector|inspector_toggle|Alt+I|browser",
            "Stages|switch_lens|Ctrl+Alt+1|-",
            "Stages|switch_lens|Ctrl+Alt+2|-",
            "Stages|switch_lens|Ctrl+Alt+3|-",
            "Stages|switch_lens|Ctrl+Alt+4|-",
        ];
        assert_eq!(flat, expected, "SHELL_SPEC §5.1 catalog drifted");
    }

    #[test]
    fn desktop_adds_ctrl_digit_stage_slots_browser_does_not() {
        let browser = browser_calc();
        assert_eq!(browser.resolve(&KeyChord::ctrl("1")), None);

        let desktop = ShellKeyboardRegistry::universal(calc_composition(), RuntimeContext::Desktop);
        assert_eq!(
            desktop.resolve(&KeyChord::ctrl("1")),
            Some(SkinVerb::SwitchLens(1))
        );
        let ctrl_one = desktop
            .bindings()
            .iter()
            .find(|binding| binding.chord == KeyChord::ctrl("1"))
            .unwrap();
        assert_eq!(ctrl_one.tier, BindingTier::DesktopOnly);
        assert_eq!(ctrl_one.tier.atlas_tag(), Some("desktop"));
        // The Ctrl+Alt primaries stay present on desktop too.
        assert_eq!(
            desktop.resolve(&KeyChord::from_parts("4", true, true, false)),
            Some(SkinVerb::SwitchLens(4))
        );
    }

    #[test]
    fn browser_alternates_resolve_alongside_their_primaries() {
        let registry = browser_calc();
        assert_eq!(
            registry.resolve(&KeyChord::from_parts("p", true, false, true)),
            Some(SkinVerb::CommandDeck)
        );
        assert_eq!(
            registry.resolve(&KeyChord::bare("?")),
            Some(SkinVerb::KeyboardAtlas)
        );
        assert_eq!(
            registry.resolve(&KeyChord::from_parts("b", false, true, false)),
            Some(SkinVerb::RegistryToggle)
        );
        assert_eq!(
            registry.resolve(&KeyChord::from_parts("z", true, false, true)),
            Some(SkinVerb::Redo)
        );
    }

    #[test]
    fn guard_order_text_entry_first_fkey_exemption_second_lookup_third() {
        let registry = browser_calc();
        // Bare grammar suppressed while typing — even though bound.
        assert_eq!(
            route_key(&registry, true, &KeyChord::bare("p")),
            KeyRouting::SuppressedByTextEntry
        );
        assert_eq!(
            route_key(&registry, true, &KeyChord::bare("?")),
            KeyRouting::SuppressedByTextEntry,
            "the ? alternate is 'when not editing' by guard order"
        );
        assert_eq!(
            route_key(&registry, true, &KeyChord::bare("Enter")),
            KeyRouting::SuppressedByTextEntry,
            "Enter belongs to the edit buffer while typing"
        );
        // F9 exempt: recalculate works from inside edit buffers.
        assert_eq!(
            route_key(&registry, true, &KeyChord::bare("F9")),
            KeyRouting::Verb(SkinVerb::Recalculate)
        );
        // Modified chords pass the guard (Ctrl+K from inside the editor).
        assert_eq!(
            route_key(&registry, true, &KeyChord::ctrl("k")),
            KeyRouting::Verb(SkinVerb::CommandDeck)
        );
        // Not typing: bare grammar resolves.
        assert_eq!(
            route_key(&registry, false, &KeyChord::bare("p")),
            KeyRouting::Verb(SkinVerb::Peek)
        );
        // Unbound chords are Unbound, not suppressed.
        assert_eq!(
            route_key(&registry, false, &KeyChord::bare("q")),
            KeyRouting::Unbound
        );
    }

    #[test]
    fn rebind_validated_accepts_a_free_chord_and_rejects_collision_and_reserved() {
        let registry = browser_calc();
        let current = KeybindingOverrideMap::new();

        // Free chord: accepted, and the rebuilt registry resolves it.
        let next = registry
            .rebind_validated(&current, SkinVerb::Explain, KeyChord::bare("x"))
            .expect("free chord rebind must validate");
        let rebuilt = ShellKeyboardRegistry::with_overrides(
            calc_composition(),
            RuntimeContext::Browser,
            &next,
        )
        .unwrap();
        assert_eq!(
            rebuilt.resolve(&KeyChord::bare("x")),
            Some(SkinVerb::Explain)
        );
        assert_eq!(rebuilt.resolve(&KeyChord::bare("e")), None);

        // Collision with a live binding: typed error naming the occupant.
        assert_eq!(
            registry.rebind_validated(&current, SkinVerb::Explain, KeyChord::ctrl("k")),
            Err(ShellKeybindingError::Collision {
                chord: KeyChord::ctrl("k"),
                occupied_by: SkinVerb::CommandDeck,
            })
        );

        // Hard-reserved chord: typed error.
        assert_eq!(
            registry.rebind_validated(&current, SkinVerb::Recalculate, KeyChord::bare("F5")),
            Err(ShellKeybindingError::HardReserved {
                chord: KeyChord::bare("F5"),
            })
        );

        // Stage switching is not rebindable.
        assert_eq!(
            registry.rebind_validated(&current, SkinVerb::SwitchLens(1), KeyChord::bare("x")),
            Err(ShellKeybindingError::NotRebindable)
        );
    }

    #[test]
    fn with_overrides_collapses_alternates_and_rejects_bad_maps() {
        // Remapping CommandDeck collapses Ctrl+K + Ctrl+Shift+P to one chord.
        let mut overrides = KeybindingOverrideMap::new();
        overrides.set(SkinVerb::CommandDeck, KeyChord::ctrl("m"));
        let registry = ShellKeyboardRegistry::with_overrides(
            calc_composition(),
            RuntimeContext::Browser,
            &overrides,
        )
        .unwrap();
        assert_eq!(
            registry.resolve(&KeyChord::ctrl("m")),
            Some(SkinVerb::CommandDeck)
        );
        assert_eq!(registry.resolve(&KeyChord::ctrl("k")), None);
        assert_eq!(
            registry.resolve(&KeyChord::from_parts("p", true, false, true)),
            None
        );

        // A collision inside the persisted map is rejected on rebuild.
        let mut clash = KeybindingOverrideMap::new();
        clash.set(SkinVerb::Explain, KeyChord::ctrl("h"));
        assert!(matches!(
            ShellKeyboardRegistry::with_overrides(
                calc_composition(),
                RuntimeContext::Browser,
                &clash
            ),
            Err(ShellKeybindingError::Collision {
                occupied_by: SkinVerb::Timeline,
                ..
            })
        ));

        // A persisted hard-reserved chord is rejected in a browser runtime.
        let mut reserved = KeybindingOverrideMap::new();
        reserved.set(SkinVerb::Recalculate, KeyChord::bare("F5"));
        assert!(matches!(
            ShellKeyboardRegistry::with_overrides(
                calc_composition(),
                RuntimeContext::Browser,
                &reserved
            ),
            Err(ShellKeybindingError::HardReserved { .. })
        ));

        // A persisted verb id from another build is rejected, not dropped.
        let unknown: KeybindingOverrideMap = serde_json::from_str(
            r#"{"by_verb":{"warp_drive":{"key":"x","ctrl":false,"alt":false,"shift":false}}}"#,
        )
        .unwrap();
        assert!(matches!(
            ShellKeyboardRegistry::with_overrides(
                calc_composition(),
                RuntimeContext::Browser,
                &unknown
            ),
            Err(ShellKeybindingError::UnknownVerb(id)) if id == "warp_drive"
        ));

        // An override for a verb this composition does not bind stays inert.
        let mut inert = KeybindingOverrideMap::new();
        inert.set(SkinVerb::RegistryToggle, KeyChord::ctrl("g"));
        let bench = ShellKeyboardRegistry::with_overrides(
            bench_composition(),
            RuntimeContext::Browser,
            &inert,
        )
        .unwrap();
        assert_eq!(bench.resolve(&KeyChord::ctrl("g")), None);
    }

    #[test]
    fn atlas_renders_from_the_live_registry_and_cannot_drift() {
        let registry = browser_calc();
        let groups = registry.atlas_groups();
        let row_count: usize = groups.iter().map(|group| group.rows.len()).sum();
        assert_eq!(
            row_count,
            registry.bindings().len(),
            "every binding appears in the atlas exactly once"
        );
        // Browser-alternate rows carry the divergence tag.
        let tagged: Vec<_> = groups
            .iter()
            .flat_map(|group| &group.rows)
            .filter(|row| row.tag == Some("browser"))
            .collect();
        assert_eq!(tagged.len(), 4, "Ctrl+Shift+P, ?, Alt+B, Alt+I");

        // A rebound registry's atlas shows the new chord — same instance,
        // no second table anywhere.
        let mut overrides = KeybindingOverrideMap::new();
        overrides.set(SkinVerb::Timeline, KeyChord::ctrl("j"));
        let rebound = ShellKeyboardRegistry::with_overrides(
            calc_composition(),
            RuntimeContext::Browser,
            &overrides,
        )
        .unwrap();
        let timeline_row = rebound
            .atlas_groups()
            .into_iter()
            .flat_map(|group| group.rows)
            .find(|row| row.verb == SkinVerb::Timeline)
            .unwrap();
        assert_eq!(timeline_row.chord_display, "Ctrl+J");
        // Stage switching renders as non-rebindable.
        let switch_row = registry
            .atlas_groups()
            .into_iter()
            .flat_map(|group| group.rows)
            .find(|row| matches!(row.verb, SkinVerb::SwitchLens(_)))
            .unwrap();
        assert!(!switch_row.rebindable);
    }

    #[test]
    fn chord_from_key_event_folds_meta_and_normalizes_printable_shift() {
        // Meta folds into ctrl.
        assert_eq!(
            chord_from_key_event("k", false, true, false, false),
            KeyChord::ctrl("k")
        );
        // `?` arrives with shiftKey=true on US layouts; the character
        // carries the shift, so the flag clears and the §5.1 binding matches.
        assert_eq!(
            chord_from_key_event("?", false, false, false, true),
            KeyChord::bare("?")
        );
        // Alphabetic keys keep their shift flag (Ctrl+Shift+Z).
        assert_eq!(
            chord_from_key_event("Z", true, false, false, true),
            KeyChord::from_parts("z", true, false, true)
        );
        // Multi-character keys pass through.
        assert_eq!(
            chord_from_key_event("ArrowLeft", false, false, false, false),
            KeyChord::bare("ArrowLeft")
        );
    }
}
