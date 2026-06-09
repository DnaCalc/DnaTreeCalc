# The ATLAS Spine — one grammar, one continuity, one styling

The spine is what makes the lens suite *one tool, not seven*. It is shared
machinery in the `dnatreecalc-skin-framework` crate that every lens consumes, so
a verb, a piece of state, or a color means the same thing in every lens. This
document is the reference contract; the [Flow lens](lenses/FLOW.md) is its first
realization.

> Status: **built** (ATLAS Phase A, slices 1–2: the spine, then all seven
> mono-lens primaries on it). The cockpit/multi-slot platform (Phase B / stack
> wave W5) is not part of the spine yet — see *Phase-B notes*.

**Registry order (the `Ctrl+1..9` slots):** Capture · Tree · Ledger · Sheet ·
Flow · Bench · Transport (the canonical grammar order: capture → structure →
populations → edit → explore → what-if → time), then the legacy
walking-skeleton skins. The switcher shows shortcut chips only for bound slots.

---

## 1. One grammar — `keybinding.rs`

A single typed verb table (`KeybindingRegistry::universal()`), consulted by the
shell and every lens. It is **pure metadata over the closed `WorkspaceIntent`
surface** — it resolves a `KeyChord` to a `SkinVerb` and never executes. The
shell owns *global* verbs; the focused lens owns *lens-local* verbs (the shell
leaves those untouched so they reach the lens). Bare-key verbs are suppressed
while typing in an edit buffer.

| Chord | `SkinVerb` | Owner | Effect |
|---|---|---|---|
| `Enter` | `Commit` | lens | enter edit on the selection / commit the edit buffer |
| `F9`, `Ctrl+Enter` | `Recalculate` | shell | `WorkspaceIntent::Recalculate` |
| `Ctrl+D` | `Fill` | lens | fill from anchor *(reserved; not yet in Flow)* |
| `h` / `l` | `Fold` / `Unfold` | lens | collapse / expand *(reserved in Flow)* |
| `]` / `[` | `TraceForward` / `TraceBack` | lens | Flow: grow dependent / precedent trace depth |
| `/` | `NameBox` | lens | open the Name-Box quick-jump |
| `Space` | `Leader` | lens | leader / health palette *(reserved)* |
| `e` | `Explain` | lens | Flow: toggle the derivation-trace explain panel |
| `Ctrl+Z` / `Ctrl+Y` | `Undo` / `Redo` | shell | `WorkspaceIntent::Undo` / `Redo` |
| `↑` / `↓` | `NavPrev` / `NavNext` | shell | `SelectNode` previous / next in order |
| `←` / `→` | `ToParent` / `ToChild` | shell | `SelectNode` parent / first child |
| `Ctrl+1..9` | `SwitchLens(n)` | shell | switch the active lens by slot |
| `Ctrl+N` | `NewWorkspace` | shell | `WorkspaceIntent::NewWorkspace` |
| `Escape` | `Escape` | lens | close Name-Box / exit edit (the Esc ladder) |

- `KeyChord::from_parts` lowercases single alphabetic keys, so `Shift` is carried
  only by the flag — `Ctrl+Shift+D` never collides with `Ctrl+D`.
- The table is **collision-free** (one chord → at most one verb; enforced by test).
- `SkinVerb::command_kind()` joins a verb to its `CommandIntentKindProjection`, so a
  lens reads **enablement + disabled-reason** for a verb from the existing
  `WorkspaceState::command_catalog`.
- Lens-local secondary chords (`Tab`, drag) are deliberately **not** in the table;
  they stay lens-local and are badged as such.
- **Typing discipline (load-bearing, learned the hard way):** every lens keydown
  handler's *first* check is `event_target_is_text_entry`; every text input a
  lens owns stops propagation of its keydowns; `Enter` is only the Commit verb
  when the target is not an interactive control (`event_target_is_interactive`);
  and **function keys are exempt** from the typing suppression so `F9`
  recalculates from inside any edit buffer, exactly like Excel. (`Ctrl+Enter`
  remains a compatibility alias for Recalculate — "sole recalc" means one verb,
  not one chord.)

*Today the registry is a constant; per-user remapping would later thread a
customized instance through context instead of reconstructing `universal()`.*

## 2. One continuity — `SharedSkinState`

Host-owned, reactive (`SharedSkinStateHandle`), serialized, and **survives lens
switches** (switching a lens is re-projection, never re-load). ATLAS continuity
fields are **`NodeKey`-keyed** (identity is permanent, path is cosmetic):

| Field | Meaning |
|---|---|
| `selection_set: Vec<NodeKey>` + `selection_anchor` | multi-select continuity |
| `collapsed_keys: HashSet<NodeKey>` | fold set shared across lenses |
| `pinned_keys: Vec<NodeKey>` | pin set shared across lenses |
| `focus_key: Option<NodeKey>` | degree-of-interest center (Flow's trace origin) |
| `cleave: Option<CleavePredicate>` | the shared filter/sort predicate |
| `active_lens: Option<String>` | current `SkinId`, for cross-lens chrome |

`SharedSkinState::gc(live_nodes)` drops continuity references to nodes that no
longer exist (including a `cleave` predicate that targets a dead node).

*Also in `SharedSkinState` (pre-ATLAS, still live):* `recalc_mode` +
`manual_recalc_pending` (the Auto/Manual commit policy every lens honors via
`commit_content_edit`), `workspace_ids` + `active_workspace_id` (shell
workspace switcher), and the legacy NodeId-keyed `tree_collapsed`/`pinned` —
transition leftovers superseded by the NodeKey-keyed fields; their removal is a
tracked cleanup once the legacy skins are absorbed.

**Design note — multi-select lives in shared view-state, not a new intent.** The
host-owned single `SelectionState.primary` (dispatcher-routed `SelectNode`)
remains the auditable anchor and is unchanged. Promoting multi-select to a
dispatcher-routed, fully-auditable intent is a tracked follow-up.

### The cleave predicate — `WorkspaceState::cleave_filtered_keys`

`CleavePredicate { filter: Option<CleaveFilter>, sort: Option<CleaveSort> }` is
the *filter/sort-as-continuity* primitive. It is typed and lens-agnostic:

- `CleaveFilter`: `CalcState(..)`, `HasError`, `TextMatch(String)`,
  `DependsOn(NodeKey)`, `Kind(DependencyKindProjection)`.
- `CleaveSort`: name / value / depth, ascending or descending (numbers sort
  before non-numbers).

The **predicate** is shared continuity; the **result is not** — each lens calls
`cleave_filtered_keys` on its own projection and re-applies it, so the cleave you
set in Ledger carries into Flow without materializing a frozen filtered set.

## 3. One styling — `style.rs`

Three invariants, codified once. Every class resolves through the existing
`var(--dtc-...)` design tokens (`ATLAS_SPINE_CSS`), so it themes automatically
across light / dark / high-contrast.

- **`calc_state` is the only saturated channel.** `calc_state_class(..)` maps the
  8 `NodeCalcStateProjection` variants to one class set
  (`clean`/`stale`/`evaluating`/`ready`/`rejected`/`cycle`/`unknown`). Nothing
  else in a lens is allowed saturated color.
- **Provenance is structural, never ambiguous.** `provenance_tint(node, ws)` →
  `Published | Pending | Speculative | Scenario | External`, rendered as
  *structural* tints (opacity / dashes / underline), not decoration.
  `Speculative` and `External` are **reserved — pending engine-typed facts**
  (no typed candidate/external-source provenance exists on `NodeView` yet); a
  lens must never hand-roll them from heuristics. Bench applies
  `dtc-prov--speculative` only to candidate-*sourced* comparison columns, where
  the source is engine-typed (`ComparativeSourceProjection::Candidate`).
- **Authoring is modeless 1-bit.** `selection_mode_class(is_selected, is_editing)`
  → the single SELECTED-vs-EDITING border cue.

## 4. The shared spine widgets — `dnatreecalc-skins/src/spine_widgets.rs`

The embedded **Lens**, **Console**, and **Name-Box** are one implementation
consumed by every mono-lens (they become real companion slots in Phase B):

- `NodeInspector` — selection facts, value, the modeless edit buffer
  (caller-owned `editing`/`editor_text`/`edit_ref` signals so the lens grammar
  can drive Enter-to-edit with programmatic focus), binding diagnostics;
  lens-specific extras (Flow's explain stack) render as children.
- `ConsoleBar` — calc-state health tallies over the visible (non-effective-meta)
  population, node count, the **capability-profile badge**, and the F9
  recalculate affordance.
- `NameBoxBar` + `first_match` — the canonical `/` model query (path/name
  contains, case-insensitive, effective-meta filtered) so `/` matches
  identically in every lens.
- `commit_content_edit` — the one recalc-mode-aware content-commit path.
- `health_tallies`, `focus_edit_buffer`, and the keyboard-target guards.

**Effective meta is defined once:** `WorkspaceState::is_effective_meta` walks
the ancestor chain per `docs/model/META_NODES.md` (self **or any ancestor**
meta). Lenses use it instead of the raw `is_meta` flag; exposing the contagion
on the engine node view itself is a tracked upstream follow-up.

## Phase-B notes (not yet built)

The spine is built so the cockpit drops in without rework. Phase B (stack wave
W5) adds: multi-slot composition, focus arbitration, **per-slot** keybinding
routing (the registry already exists to route through), capability-manifest
negotiation, persona enforcement, and promotion of the embedded Lens/Console to
real companion slots. None of that changes the contract above.
