//! SELECTION — the Sheet stage's pure select/navigate grammar (S3.8).
//!
//! A DOM-free, Leptos-free, fully unit-testable layer with two halves that the
//! crate-root wiring ([`crate`]) consumes but never re-implements:
//!
//! 1. **The nav grammar** — a declarative, atlas-taggable TABLE
//!    ([`sheet_keymap`]) mapping a normalized [`KeyChord`] to a Sheet-local
//!    [`SheetAction`], plus [`resolve_action`] / [`action_availability`]. This
//!    mirrors the Notebook stage's `keyboard.rs` discipline (the two stages are
//!    independent TP crates, so the Sheet carries its own honest copy rather than
//!    sharing a lower crate — see `edit.rs`'s note): the grammar is DATA a future
//!    Atlas stage can enumerate and rebind, never scattered `match` arms on raw
//!    key names. Rows whose capability needs the **G3** interaction pack (range
//!    extend via Shift+Arrow) stay *in the table* — so the Atlas can show them and
//!    their chords are reserved — but their [`action_availability`] is
//!    [`ActionAvailability::DisabledWithReason`], and the wiring surfaces that
//!    reason and mutates NOTHING (every reason cites the literal `G3`).
//!
//! 2. **The movement function** — [`next_active`]: the pure, edge-clamped map from
//!    an active cell + a [`NavAction`] to the next active cell over a
//!    [`GridExtent`]. Navigation clamps at the four edges (no wrap), so the active
//!    cell can never leave `[1, rows] × [1, cols]`. This is the one place the
//!    select model's arithmetic lives, so it is exhaustively unit-tested here.
//!
//! Selection is a SINGLE cell in this stage (SHEET_SPEC §4): ranges, whole
//! row/column select, fill grips, and the grid clipboard "all arrive with G3 —
//! affordances render disabled-with-reason until then (no fake handles)". The
//! degrade REASON constants live here so the keyboard grammar and the pointer
//! wiring surface identical honest text.

/// The 1-based extent of the grid the active cell moves over: `rows`/`cols` are
/// the last valid row/column (Excel-style inclusive maxima), so a legal cell is
/// any `(r, c)` with `1 <= r <= rows` and `1 <= c <= cols`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridExtent {
    pub rows: u32,
    pub cols: u32,
}

impl GridExtent {
    /// Build an extent, flooring each dimension at 1 so there is always at least
    /// one legal cell (a degenerate `0` extent would leave [`next_active`] with an
    /// empty clamp range). A grid the stage draws always has a real extent
    /// (`max_rows`/`max_cols` from the projection), so this floor is a guard, not
    /// a normal path.
    #[must_use]
    pub fn new(rows: u32, cols: u32) -> Self {
        Self {
            rows: rows.max(1),
            cols: cols.max(1),
        }
    }

    /// Clamp a candidate `(row, col)` into `[1, rows] × [1, cols]`. Both axes are
    /// clamped independently, so an off-edge move parks the active cell on the
    /// nearest edge cell rather than wrapping or leaving the grid.
    #[must_use]
    pub fn clamp(self, row: u32, col: u32) -> (u32, u32) {
        (row.clamp(1, self.rows), col.clamp(1, self.cols))
    }

    /// [`GridExtent::clamp`] on a `(row, col)` pair — the shape the wiring holds an
    /// active cell in.
    #[must_use]
    pub fn clamp_pair(self, cell: (u32, u32)) -> (u32, u32) {
        self.clamp(cell.0, cell.1)
    }
}

/// A single-cell move over the grid. This is the movement half of the model —
/// distinct from [`SheetAction`], which also carries the mode-change and degrade
/// verbs the keyboard grammar resolves. Every variant lands on exactly one cell
/// (single-cell selection), never a range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NavAction {
    /// One row up (Arrow Up).
    Up,
    /// One row down (Arrow Down).
    Down,
    /// One column left (Arrow Left).
    Left,
    /// One column right (Arrow Right).
    Right,
    /// One column right (Tab).
    NextColumn,
    /// One column left (Shift+Tab).
    PrevColumn,
    /// The first column of the current row (Home).
    RowStart,
    /// The top-left cell `(1, 1)` (Ctrl+Home).
    GridStart,
    /// One row down, keeping the column — the move an accepted Enter-commit makes
    /// (Excel advances down after committing).
    CommitDown,
    /// One column right, keeping the row — the move an accepted Tab-commit makes
    /// (Excel advances right after committing with Tab). The horizontal twin of
    /// [`NavAction::CommitDown`].
    CommitRight,
}

/// The next active cell after applying `action` to `current` over `extent`.
///
/// Pure and total: every move is computed with [`u32::saturating_sub`] on the low
/// edge and [`GridExtent::clamp`] on the high edge, so the result is always a
/// legal in-extent cell and the function never underflows, overflows, or panics.
/// Clamping is per-axis with NO wrap — a move off the top/bottom/left/right edge
/// parks on that edge (documented invariant, asserted by the tests).
#[must_use]
pub fn next_active(current: (u32, u32), action: NavAction, extent: GridExtent) -> (u32, u32) {
    let (row, col) = current;
    let (next_row, next_col) = match action {
        NavAction::Up => (row.saturating_sub(1).max(1), col),
        NavAction::Down => (row.saturating_add(1), col),
        NavAction::Left => (row, col.saturating_sub(1).max(1)),
        NavAction::Right => (row, col.saturating_add(1)),
        NavAction::NextColumn => (row, col.saturating_add(1)),
        NavAction::PrevColumn => (row, col.saturating_sub(1).max(1)),
        NavAction::RowStart => (row, 1),
        NavAction::GridStart => (1, 1),
        NavAction::CommitDown => (row.saturating_add(1), col),
        NavAction::CommitRight => (row, col.saturating_add(1)),
    };
    extent.clamp(next_row, next_col)
}

// --- Keyboard grammar (mirrors the Notebook stage's keyboard.rs) -------------

/// Normalize a raw DOM `KeyboardEvent.key` string into the table's vocabulary: a
/// single alphabetic key is lowercased so its `Shift` state is carried ONLY by
/// [`KeyChord::shift`] (never by the key's case), while every multi-character key
/// (`"ArrowUp"`, `"Tab"`, `"Home"`, `"F2"`, `"Enter"`, `"Escape"`) passes through
/// unchanged. Mirrors the Notebook grammar's `normalize_key`.
fn normalize_key(key: &str) -> String {
    let mut chars = key.chars();
    match (chars.next(), chars.next()) {
        (Some(ch), None) if ch.is_alphabetic() => ch.to_ascii_lowercase().to_string(),
        _ => key.to_string(),
    }
}

/// A normalized key chord: a key name plus the three modifier flags the Sheet
/// grammar distinguishes. The platform `meta` key (macOS `Cmd`) is folded into
/// `ctrl` by the wiring before a chord is built (estate convention), so a chord is
/// layout-portable.
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

/// The Sheet stage's closed set of keyboard actions. A binding row targets exactly
/// one of these; [`action_availability`] then decides whether the action is live
/// or an honest disabled-with-reason G3 degrade.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SheetAction {
    /// Move the active cell one step (Arrows / Tab / Shift+Tab / Home /
    /// Ctrl+Home). Carries the [`NavAction`] so the wiring calls [`next_active`]
    /// with it directly — the movement grammar and the key grammar cannot drift.
    Move(NavAction),
    /// Open the one overlay editor at the active cell, seeded with the cell's
    /// EXISTING authored text (F2 edit-in-place, or a double-click). Type-to-replace
    /// is NOT a keymap row — it is any printable key, handled as a wiring fallback
    /// via [`typed_char`] so the seed is the typed character, not the existing text.
    /// (Enter is NOT an edit chord — in select mode it moves down, Excel-classic.)
    BeginEdit,
    /// Escape in select mode: clear the transient degrade note (in edit mode the
    /// overlay bridge owns Escape as exact-revert, so this never fires there).
    Cancel,
    /// Extend the selection into a RANGE (Shift+Arrow). DEGRADE: ranges need the
    /// G3 interaction pack; the chord is reserved and disabled-with-reason.
    ExtendSelection,
}

/// Whether an action performs its effect ([`ActionAvailability::Live`]) or is an
/// honest disabled-with-reason G3 degrade that must surface its reason and mutate
/// NOTHING ([`ActionAvailability::DisabledWithReason`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionAvailability {
    /// The action is wired and performs its effect.
    Live,
    /// The action is deliberately not built until G3; the wiring surfaces this
    /// reason verbatim (never fake success, never mutate). The reason always cites
    /// the literal substring `G3`.
    DisabledWithReason(&'static str),
}

/// The reason a RANGE selection (Shift+Arrow / Shift+click) is unavailable. Cites
/// `G3` (the interaction pack that lands ranges), shared by the keyboard degrade
/// and the pointer degrade so both read identically.
pub const RANGE_SELECT_DEFERRED_REASON: &str =
    "Range select needs the G3 interaction pack — single cell only for now.";

/// The reason WHOLE-ROW / WHOLE-COLUMN (and select-all) selection is unavailable
/// (a header-strip or corner click). Cites `G3`.
pub const ROW_COL_SELECT_DEFERRED_REASON: &str =
    "Whole row / column select arrives with the G3 interaction pack.";

/// The reason the FILL grip is unavailable. Cites `G3`. The stage renders NO fill
/// handle (an honest absence, not a fake grip); this is the note the disabled
/// affordance carries.
pub const FILL_DEFERRED_REASON: &str = "Fill grips arrive with the G3 interaction pack.";

/// The reason the grid CLIPBOARD is unavailable. Cites `G3`.
pub const CLIPBOARD_DEFERRED_REASON: &str =
    "Grid copy / cut / paste arrives with the G3 interaction pack.";

/// One row of the atlas-taggable keymap: a stable id, the chord that triggers it,
/// the action it targets, and a human label. The `id` is the durable handle a
/// future Atlas rebind surface keys on — it must never change once shipped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyBinding {
    pub id: &'static str,
    pub chord: KeyChord,
    pub action: SheetAction,
    pub label: &'static str,
}

/// The Sheet stage's whole select/navigate grammar, as one declarative table.
///
/// Ids are stable wire strings namespaced under `sheet.` so an Atlas that
/// aggregates several stages' keymaps keeps them unambiguous. F2 opens the editor
/// (edit-in-place); Enter moves the active cell DOWN in select mode (Excel-classic
/// — you edit via type-to-replace / F2 / double-click). The four Shift+Arrow
/// chords are reserved G3 degrades.
#[must_use]
pub fn sheet_keymap() -> Vec<KeyBinding> {
    vec![
        // Arrow navigation (Select mode) — bare, no modifiers.
        KeyBinding {
            id: "sheet.move-up",
            chord: KeyChord::new("ArrowUp", false, false, false),
            action: SheetAction::Move(NavAction::Up),
            label: "Move up",
        },
        KeyBinding {
            id: "sheet.move-down",
            chord: KeyChord::new("ArrowDown", false, false, false),
            action: SheetAction::Move(NavAction::Down),
            label: "Move down",
        },
        KeyBinding {
            id: "sheet.move-left",
            chord: KeyChord::new("ArrowLeft", false, false, false),
            action: SheetAction::Move(NavAction::Left),
            label: "Move left",
        },
        KeyBinding {
            id: "sheet.move-right",
            chord: KeyChord::new("ArrowRight", false, false, false),
            action: SheetAction::Move(NavAction::Right),
            label: "Move right",
        },
        // Tab / Shift+Tab move the active cell one column (Excel horizontal walk).
        KeyBinding {
            id: "sheet.move-next-column",
            chord: KeyChord::new("Tab", false, false, false),
            action: SheetAction::Move(NavAction::NextColumn),
            label: "Next column",
        },
        KeyBinding {
            id: "sheet.move-prev-column",
            chord: KeyChord::new("Tab", true, false, false),
            action: SheetAction::Move(NavAction::PrevColumn),
            label: "Previous column",
        },
        // Home → first column of the row; Ctrl+Home → the top-left cell.
        KeyBinding {
            id: "sheet.move-row-start",
            chord: KeyChord::new("Home", false, false, false),
            action: SheetAction::Move(NavAction::RowStart),
            label: "Row start",
        },
        KeyBinding {
            id: "sheet.move-grid-start",
            chord: KeyChord::new("Home", false, true, false),
            action: SheetAction::Move(NavAction::GridStart),
            label: "Grid start",
        },
        // F2 opens the one overlay editor at the active cell (existing text);
        // type-to-replace and double-click are the other edit entry points.
        KeyBinding {
            id: "sheet.edit-in-place",
            chord: KeyChord::new("F2", false, false, false),
            action: SheetAction::BeginEdit,
            label: "Edit in place",
        },
        // Enter in SELECT mode moves DOWN (Excel-classic) — it does not open the
        // editor. (In EDIT mode the overlay bridge owns Enter as commit+advance,
        // so this select-mode binding never fires while editing.)
        KeyBinding {
            id: "sheet.enter-down",
            chord: KeyChord::new("Enter", false, false, false),
            action: SheetAction::Move(NavAction::Down),
            label: "Enter (move down)",
        },
        // Escape clears the transient degrade note in select mode.
        KeyBinding {
            id: "sheet.cancel",
            chord: KeyChord::new("Escape", false, false, false),
            action: SheetAction::Cancel,
            label: "Cancel",
        },
        // Shift+Arrow range extend — reserved chords, degraded until G3.
        KeyBinding {
            id: "sheet.extend-up",
            chord: KeyChord::new("ArrowUp", true, false, false),
            action: SheetAction::ExtendSelection,
            label: "Extend selection up",
        },
        KeyBinding {
            id: "sheet.extend-down",
            chord: KeyChord::new("ArrowDown", true, false, false),
            action: SheetAction::ExtendSelection,
            label: "Extend selection down",
        },
        KeyBinding {
            id: "sheet.extend-left",
            chord: KeyChord::new("ArrowLeft", true, false, false),
            action: SheetAction::ExtendSelection,
            label: "Extend selection left",
        },
        KeyBinding {
            id: "sheet.extend-right",
            chord: KeyChord::new("ArrowRight", true, false, false),
            action: SheetAction::ExtendSelection,
            label: "Extend selection right",
        },
    ]
}

/// Resolve a chord to its action against `keymap` — a pure, first-match lookup.
/// Returns `None` for an unbound chord (the wiring then tries [`typed_char`] for
/// type-to-replace, else lets the keystroke bubble untouched to the shell).
#[must_use]
pub fn resolve_action(keymap: &[KeyBinding], chord: &KeyChord) -> Option<SheetAction> {
    keymap
        .iter()
        .find(|binding| binding.chord == *chord)
        .map(|binding| binding.action)
}

/// Whether an action is live or an honest disabled-with-reason G3 degrade. Every
/// `Move` / `BeginEdit` / `Cancel` verb is live (single-cell select and the
/// overlay editor are built); `ExtendSelection` is disabled-with-reason citing
/// `G3` (ranges need the interaction pack).
#[must_use]
pub const fn action_availability(action: SheetAction) -> ActionAvailability {
    match action {
        SheetAction::Move(_) | SheetAction::BeginEdit | SheetAction::Cancel => {
            ActionAvailability::Live
        }
        SheetAction::ExtendSelection => {
            ActionAvailability::DisabledWithReason(RANGE_SELECT_DEFERRED_REASON)
        }
    }
}

/// The single character a keydown seeds a type-to-replace edit with, or `None`
/// when the key is not a lone printable character.
///
/// A DOM `KeyboardEvent.key` is exactly one character for a printable key (`"a"`,
/// `"="`, `" "`) and a multi-character name for a control/navigation key
/// (`"Enter"`, `"ArrowUp"`, `"F2"`, `"Tab"`). This returns `Some(ch)` only for the
/// former, excluding control characters, so the wiring can seed the editor with a
/// typed character. Pure over the raw key string — the wiring is responsible for
/// having already ruled out Ctrl/Alt/Meta chords (a Ctrl+C is not type-to-replace).
#[must_use]
pub fn typed_char(key: &str) -> Option<char> {
    let mut chars = key.chars();
    match (chars.next(), chars.next()) {
        (Some(ch), None) if !ch.is_control() => Some(ch),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEMO: GridExtent = GridExtent { rows: 5, cols: 2 };

    /// The four arrow moves step the active cell by one in the expected axis,
    /// interior to the extent (no clamping in play here).
    #[test]
    fn arrows_step_one_cell_in_the_interior() {
        let start = (3, 1);
        assert_eq!(next_active(start, NavAction::Up, DEMO), (2, 1));
        assert_eq!(next_active(start, NavAction::Down, DEMO), (4, 1));
        assert_eq!(next_active((3, 2), NavAction::Left, DEMO), (3, 1));
        assert_eq!(next_active(start, NavAction::Right, DEMO), (3, 2));
    }

    /// Tab / Shift+Tab walk columns; Home / Ctrl+Home jump to the row start and
    /// the grid origin.
    #[test]
    fn tab_home_and_grid_start_land_where_expected() {
        assert_eq!(next_active((3, 1), NavAction::NextColumn, DEMO), (3, 2));
        assert_eq!(next_active((3, 2), NavAction::PrevColumn, DEMO), (3, 1));
        assert_eq!(next_active((4, 2), NavAction::RowStart, DEMO), (4, 1));
        assert_eq!(next_active((4, 2), NavAction::GridStart, DEMO), (1, 1));
    }

    /// An accepted Enter-commit advances one row down, clamping at the last row.
    #[test]
    fn commit_down_advances_and_clamps_at_the_last_row() {
        assert_eq!(next_active((2, 2), NavAction::CommitDown, DEMO), (3, 2));
        // Already on the last row: stays put (no wrap to row 1).
        assert_eq!(next_active((5, 2), NavAction::CommitDown, DEMO), (5, 2));
    }

    /// An accepted Tab-commit advances one column right, clamping at the last
    /// column (the horizontal twin of the Enter-commit, dtc-dzky).
    #[test]
    fn commit_right_advances_and_clamps_at_the_last_column() {
        assert_eq!(next_active((2, 1), NavAction::CommitRight, DEMO), (2, 2));
        // Already on the last column: stays put (no wrap to column 1).
        assert_eq!(next_active((2, 2), NavAction::CommitRight, DEMO), (2, 2));
    }

    /// THE clamping invariant: a move off every edge parks on the edge cell — the
    /// active cell can never leave `[1, rows] × [1, cols]`, and never wraps.
    #[test]
    fn moves_clamp_at_every_edge_with_no_wrap() {
        // Top edge: Up from row 1 stays on row 1.
        assert_eq!(next_active((1, 1), NavAction::Up, DEMO), (1, 1));
        // Left edge: Left / Shift+Tab from col 1 stay on col 1.
        assert_eq!(next_active((3, 1), NavAction::Left, DEMO), (3, 1));
        assert_eq!(next_active((3, 1), NavAction::PrevColumn, DEMO), (3, 1));
        // Bottom edge: Down from the last row stays on the last row.
        assert_eq!(next_active((5, 1), NavAction::Down, DEMO), (5, 1));
        // Right edge: Right / Tab from the last column stay on the last column.
        assert_eq!(next_active((3, 2), NavAction::Right, DEMO), (3, 2));
        assert_eq!(next_active((3, 2), NavAction::NextColumn, DEMO), (3, 2));
    }

    /// A degenerate `(0, 0)` extent floors to a single legal cell, so navigation
    /// over an empty grid can never produce an out-of-range address.
    #[test]
    fn degenerate_extent_floors_to_one_cell() {
        let empty = GridExtent::new(0, 0);
        assert_eq!(empty, GridExtent { rows: 1, cols: 1 });
        assert_eq!(next_active((1, 1), NavAction::Down, empty), (1, 1));
        assert_eq!(next_active((1, 1), NavAction::Right, empty), (1, 1));
    }

    /// A starting cell already outside the extent (e.g. a stale active cell after
    /// the grid shrank) is pulled back inside by the clamp on the very next move.
    #[test]
    fn out_of_extent_start_is_pulled_back_inside() {
        assert_eq!(next_active((99, 99), NavAction::Up, DEMO), (5, 2));
        assert_eq!(next_active((99, 99), NavAction::RowStart, DEMO), (5, 1));
    }

    /// Every declared chord resolves to its declared action — the grammar's
    /// chord→action mapping asserted against the live table.
    #[test]
    fn every_declared_chord_resolves_to_its_action() {
        let keymap = sheet_keymap();
        for (chord, expected) in [
            (
                KeyChord::new("ArrowUp", false, false, false),
                SheetAction::Move(NavAction::Up),
            ),
            (
                KeyChord::new("ArrowDown", false, false, false),
                SheetAction::Move(NavAction::Down),
            ),
            (
                KeyChord::new("ArrowLeft", false, false, false),
                SheetAction::Move(NavAction::Left),
            ),
            (
                KeyChord::new("ArrowRight", false, false, false),
                SheetAction::Move(NavAction::Right),
            ),
            (
                KeyChord::new("Tab", false, false, false),
                SheetAction::Move(NavAction::NextColumn),
            ),
            (
                KeyChord::new("Tab", true, false, false),
                SheetAction::Move(NavAction::PrevColumn),
            ),
            (
                KeyChord::new("Home", false, false, false),
                SheetAction::Move(NavAction::RowStart),
            ),
            (
                KeyChord::new("Home", false, true, false),
                SheetAction::Move(NavAction::GridStart),
            ),
            (
                KeyChord::new("F2", false, false, false),
                SheetAction::BeginEdit,
            ),
            (
                // Enter in select mode moves DOWN (Excel-classic), not begin-edit.
                KeyChord::new("Enter", false, false, false),
                SheetAction::Move(NavAction::Down),
            ),
            (
                KeyChord::new("Escape", false, false, false),
                SheetAction::Cancel,
            ),
            (
                KeyChord::new("ArrowUp", true, false, false),
                SheetAction::ExtendSelection,
            ),
        ] {
            assert_eq!(
                resolve_action(&keymap, &chord),
                Some(expected),
                "chord {chord:?} must resolve to {expected:?}"
            );
        }
    }

    /// A chord not in the table resolves to `None` — the wiring must let it fall
    /// through to type-to-replace or bubble, never swallow it. Ctrl+arrow (the
    /// edge-jump chord that needs a host query, SHEET_SPEC §4) is deliberately
    /// unbound here, so it bubbles rather than silently window-jumping.
    #[test]
    fn non_matching_chords_resolve_to_none() {
        let keymap = sheet_keymap();
        assert_eq!(
            resolve_action(&keymap, &KeyChord::new("ArrowUp", false, true, false)),
            None,
            "Ctrl+ArrowUp (edge jump) is not bound — it must bubble"
        );
        // A bare letter is not a nav chord — it falls through to type-to-replace.
        assert_eq!(
            resolve_action(&keymap, &KeyChord::new("a", false, false, false)),
            None
        );
        // Enter WITH shift is unbound (bare Enter moves down; Shift+Enter is free).
        assert_eq!(
            resolve_action(&keymap, &KeyChord::new("Enter", true, false, false)),
            None
        );
    }

    /// Availability: nav / begin-edit / cancel are live; range extend is
    /// disabled-with-reason, and the reason cites `G3` so the honest degrade is
    /// greppable.
    #[test]
    fn availability_is_live_for_built_and_g3_for_ranges() {
        for live in [
            SheetAction::Move(NavAction::Up),
            SheetAction::Move(NavAction::GridStart),
            SheetAction::BeginEdit,
            SheetAction::Cancel,
        ] {
            assert_eq!(action_availability(live), ActionAvailability::Live);
        }
        match action_availability(SheetAction::ExtendSelection) {
            ActionAvailability::DisabledWithReason(reason) => assert!(
                reason.contains("G3"),
                "range degrade reason must cite `G3`, got {reason:?}"
            ),
            ActionAvailability::Live => panic!("range extend must be a G3 degrade, not live"),
        }
    }

    /// Every G3 degrade reason string cites `G3` — the greppable honesty marker
    /// the pointer wiring and the disabled affordances all reuse.
    #[test]
    fn every_g3_reason_cites_g3() {
        for reason in [
            RANGE_SELECT_DEFERRED_REASON,
            ROW_COL_SELECT_DEFERRED_REASON,
            FILL_DEFERRED_REASON,
            CLIPBOARD_DEFERRED_REASON,
        ] {
            assert!(reason.contains("G3"), "{reason:?} must cite `G3`");
        }
    }

    /// The table is well-formed: every binding id is unique and stable, and F2 is
    /// the single chord that opens the editor (Enter moves down in select mode,
    /// Excel-classic; type-to-replace and double-click are the other edit entries).
    #[test]
    fn keymap_ids_are_unique_and_f2_is_the_begin_edit_chord() {
        let keymap = sheet_keymap();
        let mut ids: Vec<&str> = keymap.iter().map(|b| b.id).collect();
        let total = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), total, "every binding id must be unique");

        let begin_edit = keymap
            .iter()
            .filter(|b| b.action == SheetAction::BeginEdit)
            .count();
        assert_eq!(begin_edit, 1, "F2 is the only begin-edit chord");
    }

    /// The uppercase (shifted) form of an alphabetic key normalizes to the same
    /// chord key as its lowercase form, so `Shift` is carried by the flag, never
    /// by the case — the property type-to-replace relies on for letters.
    #[test]
    fn uppercase_alpha_normalizes_to_lowercase_key() {
        assert_eq!(KeyChord::new("A", true, false, false).key, "a");
        assert_eq!(KeyChord::new("ArrowUp", false, false, false).key, "ArrowUp");
    }

    /// `typed_char` recognizes exactly the lone printable keys (letters, digits,
    /// punctuation, space) and rejects every multi-character control/navigation
    /// key name and the empty string.
    #[test]
    fn typed_char_recognizes_only_lone_printables() {
        for (key, expected) in [
            ("a", Some('a')),
            ("A", Some('A')),
            ("=", Some('=')),
            ("7", Some('7')),
            (" ", Some(' ')),
        ] {
            assert_eq!(typed_char(key), expected, "key {key:?}");
        }
        for control in ["Enter", "ArrowUp", "F2", "Tab", "Home", "Escape", "", "\u{1b}"] {
            assert_eq!(typed_char(control), None, "key {control:?} is not printable");
        }
    }
}
