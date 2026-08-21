# SHELL_SPEC — the DNA Calc cockpit

Status: v0.9 ratification-candidate · 2026-07-13 · gates S0 beads.
Parents: [REDESIGN_PROGRAM.md](REDESIGN_PROGRAM.md) · [STRAND_DESIGN_LANGUAGE.md](STRAND_DESIGN_LANGUAGE.md) · [MECHANISMS.md](MECHANISMS.md).
Scope: everything outside the Stage viewport, for both products (Bench app, Calc app), both
targets (browser WASM, Tauri), both profiles. Stages have their own specs.

## 1. Anatomy contract

Regions, top to bottom. A product composes a subset; omission must be first-class (no dead
space, no orphaned keyboard verbs — verbs of omitted regions are absent from the catalog).

| Region | Contents | Bench | Calc | Sizing / behavior |
|---|---|---|---|---|
| **Mast** | product mark + profile identity badge (STRAND §2 base-pair badge) · document name + dirty dot · stage switcher (segmented) · persona chip | ✓ (no stage switcher) | ✓ | 40px fixed, never scrolls; petrol chrome |
| **Bridge** | address/name box · formula editor (token-lit) · live reference readout · calc chip | ✓ (hero: two-row) | ✓ (one-row, expandable) | 44px (Calc) / 88px (Bench); chrome |
| **Registry** | rail: Names · Sheets/Top-nodes · Tables · Extensions, each entry with liveness dot; filter box | — | ✓ | 232px, collapsible to 0 (Ctrl+B alt Alt+B); paper-2 |
| **Stage host** | the active stage surface | ✓ (Result stage fixed) | ✓ | fills remainder; paper |
| **Inspector** | selection detail: value/shape · format + provenance · dependencies both ways · stage-specific panels | ✓ | ✓ | 268px, collapsible; paper-2 |
| **Strip** | calc state · dirty count · last-run ms · feed health · locale · revision · zoom · agent light | ✓ | ✓ | 26px fixed; petrol-900 |

Overlays (float over everything, one at a time except Peeks):
**Command deck** (Ctrl+K) · **Keyboard atlas** (Ctrl+/) · **Timeline drawer** (Ctrl+H) ·
**Peek cards** (Alt+hover / `P` on selection; max 3 pinned) · **Extensions manager** (from
Registry/Strip). Esc closes the topmost overlay before it does anything else.

**Esc-vs-text-entry precedence (resolves the §5 guard tension):** an overlay that owns a
focused text input — the command deck's query field is the only S0 case — handles Esc
*locally*: the input's own keydown closes the overlay directly (no command runs) and stops
propagation, so the keystroke never falls into the shell's §5 text-entry guard at all. The §5
guard (`event_target_is_text_entry` first, F-key exemptions second, chord lookup third) governs
everything else that lives inside a text-entry element — stage-level edit buffers in
particular, where a bare Esc is `EscapeRevert`: the buffer's own keydown handler consumes it,
reverts exactly, and stops propagation, so it never reaches `OverlayModel::escape()` while that
buffer holds focus. Only once focus has moved off the buffer entirely does a later Esc fall
through to the overlay ladder. One keystroke, one effect, in both cases — the difference is
which handler owns that keystroke, not a violation of "Esc closes the topmost overlay before
anything else" (that rule is about *overlay* Esc precedence over other overlay mechanics, not
about reaching into a focused input that already claimed the keystroke).

### 1.1 Responsive & touch provisions

The anatomy above is the desktop contract. Below the **narrow breakpoint**
(`≤ 900px` CSS px; `dnacalc-shell::viewport::NARROW_MAX_WIDTH_PX`) the shell adapts:

- **Rails become overlay panels.** Registry and Inspector compose as absolutely-positioned
  panels OVER the stage (media query in `STATIC_SHELL_CSS`) instead of squeezing it;
  their widths cap at `min(<contract>px, ~80vw)`.
- **Rails start collapsed on narrow viewports** — a phone lands on the stage, not chrome.
  The Shell tracks viewport width (wasm-only resize watcher feeding a signal); entering
  narrow collapses an open rail once, exactly as Ctrl+B/Ctrl+I would; leaving narrow
  restores the desktop contract without forcing anything open.
- **Mast controls are the pointer/touch path back.** The mast renders ⌘ Commands
  (`Ctrl+K`), ☰ Registry (`Ctrl+B`), ◫ Inspector (`Ctrl+I`) buttons for every composed
  region — same verbs, same state, zero dispatched intents. They exist on desktop too
  (discoverability), and grow to ≥38px targets under `pointer: coarse`.
- **Overlays fit any width:** the panel is `min(720px, calc(100vw − 24px))`; the Strip
  scrolls horizontally instead of clipping; the Mast's document name ellipsizes and its
  stage switcher scrolls.
- **Sheet stage gestures** (`dnacalc-stage-sheet`, canvas owns its viewport via
  `touch-action: none`): tap = select (same hit-test as mouse), double-tap = edit
  (the touch `dblclick`; classified by the pure `gestures` module), one-finger drag =
  pan (content follows the finger), two-finger pinch = zoom around the pinch's starting
  factor. Mouse behavior is unchanged (the pointer path replaces `mousedown`
  one-for-one; `dblclick`/wheel stay).
- App roots size to `100dvh` (with a `100vh` fallback) so mobile URL-bar chrome does not
  clip the Strip.

The breakpoint decision is pinned by native tests (`viewport::is_narrow_width`, the
gesture classifiers); DOM behavior is proven by the browser harness (§9).

## 2. Crate decomposition (tiers per D5)

| Crate | Tier | Contents |
|---|---|---|
| `dnacalc-strand` | TP | `--dna-*` token definitions (3 themes × density), ThemeTokens type, contrast-assertion tests (STRAND §2.1), block-geometry constants |
| `dnacalc-shell` | TP | region components, overlay system, stage-host + `StageSurface` trait, keyboard registry, command deck, shared-state wiring |
| `dnacalc-bridge` | TP | formula workbench: editor view, token runs renderer, completion popup, signature help, diagnostics, reference readout, X-Ray affordance |
| host adapters | TF / TC | feed the shell: Bench host publishes `OneFormulaProjection`; Calc host publishes `WorkspaceState` — shell code never knows which engine exists (P-gate) |

`dnacalc-shell` and `dnacalc-bridge` depend on `dnacalc-skin-ir` + `dnacalc-strand` + Leptos
only. **P-gate applies: no `ox*` crate in their graphs.**

## 3. Stage host contract

```rust
// dnacalc-shell (sketch — final signatures at bead time)
pub trait StageSurface {
    fn id(&self) -> StageId;                       // sheet | model | notebook | atlas | bench-result
    fn title(&self) -> &'static str;
    fn supports(&self, profile: &ProfileTag) -> bool;   // profiles gate capability
    fn mount(&self, ctx: StageContext) -> StageHandle;  // ctx: signals + dispatcher + preview + shared-state + strand tokens
}
```

Rules (carried from the estate's spine, unchanged in force):
- **Switching is re-projection, never re-load.** Stage switch must not dispatch engine intents.
- **Continuity:** `SharedSkinState.selection_set / focus_key / collapsed_keys / pinned_keys /
  cleave / active_lens→active_stage` survive switches; the incoming stage renders the
  continuity halo (mechanism 09; 160 ms, reduced-motion honored).
- Stage switcher shows only stages whose `supports(profile)` holds; keyboard slots follow the
  visible order.
- A stage that cannot mount renders the fail-loud fallback card (never a blank or a wrong render).

## 4. Data wiring

- Read side: `ReadSignal<WorkspaceState>` + `ReadSignal<WorkspaceDelta>` (Calc) or
  `OneFormulaProjection` (Bench) provided via context; shell chrome subscribes to the slices it
  needs (names, sheets, calc, persistence, capabilities). No component reads engine types.
- Write side: exactly one `Dispatcher` (workspace intents) + `SkinShellIntent` channel
  (documents/files/palette) + audited `SharedStateChange` chokepoint for view-state. No other
  mutation paths exist in TP code.
- Foresight: `PreviewService` when present; every consumer must implement the documented
  degrade (post-attempt receipt errors) when absent — previews are an enhancement, never a
  requirement (worker builds may run without).
- Personas: chrome pre-disables what dispatch would refuse (`CommandCatalogProjection
  ::governed_by`); Reviewer/ReadOnly render the same shell with disabled affordances, not a
  different shell.

## 5. Keyboard grammar

One universal verb table in `dnacalc-shell`, collision-tested (estate pattern). Stage-local
verbs register under the focused stage; the guard order in every keydown handler is:
`event_target_is_text_entry` first, F-key exemptions second (F9 must work from inside edit
buffers), chord lookup third.

Consequence for host-owned edit surfaces (e.g. the Bridge formula editor, SHELL_SPEC §6): their
local keydown handler must call `stop_propagation()` only for the keys it actually consumes
(completion navigation, Commit-Enter, Escape-revert) — every other key, F9 and Ctrl+K included,
must bubble undisturbed to the shell's guard above so the F-key exemption and the modified-chord
passthrough it already implements (`route_key`) can do their job. See §1's Esc-vs-text-entry
precedence note for how this interacts with overlay Esc.

**Undo/Redo carve-out while a host edit buffer is dirty (owner-ratified 2026-07-12, bead
dtc-lfz.2 — supersedes bead dtc-dpo item 1):** the rule above has one further, narrowly-scoped
exception for Ctrl+Z / Ctrl+Y / Ctrl+Shift+Z specifically. While a host edit buffer holds
uncommitted keystrokes — its local text differs from the surface's committed `source_text`,
i.e. it is *dirty* — those three chords are consumed locally instead of bubbling: the editor's
keydown handler calls `stop_propagation()` (so the keystroke never reaches the shell's
Undo/Redo verbs for that keydown) but deliberately never `prevent_default()` (the browser's own
textarea undo/redo stack IS the effect the carve-out exists to produce; calling
`prevent_default()` would silence that native undo while still hiding the chord from the shell,
losing the effect entirely). Once the buffer is clean again — its text matches `source_text` —
the carve-out lapses and Ctrl+Z/Y/Shift+Z fall back to the general rule above: they bubble
undisturbed, and the shell's model Undo/Redo verbs fire, exactly as before this carve-out
existed. This is narrower than it looks: Ctrl+K and F9 are unaffected by buffer dirtiness (they
always bubble, per the rule above, editing or not); clipboard chords (Ctrl+X/C/V) are likewise
unaffected — the owner chose the undo-only carve-out this phase, not a broader "capture
everything while dirty" rule. See §1's Esc-vs-text-entry precedence note for the analogous
one-keystroke-one-effect reasoning this carve-out follows.

### 5.1 Universal verbs (defaults; all rebindable except stage switching)

| Verb | Primary | Browser-hard-reserved? | Browser alternate | Notes |
|---|---|---|---|---|
| Commit | Enter | no | — | context-sensitive (commit+move in grids) |
| EditInPlace | F2 | no | — | Excel-exact |
| EscapeRevert | Esc | no | — | exact revert, then closes overlays |
| Recalculate | F9 | no | — | works inside edit buffers |
| Undo / Redo | Ctrl+Z / Ctrl+Y | no | — | Ctrl+Shift+Z also Redo |
| Command deck | Ctrl+K | no (interceptable) | Ctrl+Shift+P mirror | both always active |
| Keyboard atlas | Ctrl+/ | no | ? (when not editing) | overlay, hold-to-peek later |
| Timeline | Ctrl+H | no | — | revision drawer (mech 19) |
| Peek | P (selection) / Alt+hover | no | — | transient card (mech 06) |
| Registry toggle | Ctrl+B | no | Alt+B | Calc only |
| Inspector toggle | Ctrl+I | no | Alt+I | — |
| Stage 1..4 | Ctrl+Alt+1..4 | Ctrl+1..9 IS reserved | (primary is already safe) | Ctrl+1..4 additionally bound on desktop only |
| Save / Open | Ctrl+S / Ctrl+O | no (interceptable) | — | disabled with honest badge until host support |
| Find/goto | Ctrl+F → deck in goto mode | no (interceptable) | — | model find, not page find |
| NameBox | / (non-edit) | no | — | estate verb, kept |
| Trace fwd/back | ] / [ | no | — | Atlas + Bridge X-Ray |
| Explain | E (non-edit) | no | — | Atlas reading head |
| Fold / Unfold | ← / → on node rows | no | — | Model/Notebook outlines |

**Hard-reserved list (never bind, never intercept):** Ctrl+W/T/N, Ctrl+Shift+W/T/N, Ctrl+Tab,
Ctrl+1..9 (browser tab slots), Ctrl+PgUp/PgDn (browser tab cycle), F5/Ctrl+R, Ctrl+L, F11, F6.
Consequences: **sheet-tab cycling cannot be Ctrl+PgUp/PgDn in the browser** — Sheet stage binds
Alt+PgUp/PgDn as primary with Ctrl+PgUp/PgDn added on desktop (SHEET_SPEC owns the details).
Every such divergence must appear in the keyboard atlas with a "browser" tag.

### 5.2 Keyboard atlas (mechanism 10)

Ctrl+/ opens a full-screen overlay listing every active chord grouped by region/stage, with
rebind affordance (persisted `KeybindingOverrideMap`, collision-revalidated on write, audited).
Browser-alternate rows carry the divergence tag. The atlas renders from the same registry the
dispatcher uses — it cannot drift.

## 6. The Bridge (shared verbatim between products)

- **Renders host truth only:** token runs, diagnostics with spans, completions (px-anchored),
  signature help — all from IR surfaces. The Bridge never tokenizes text itself (no `=`
  sniffing; layering law).
- **Bench context:** full `FormulaEditorSurface` fidelity today (tokens, staged diagnostics,
  completion kinds, signature help, drill).
- **Calc contexts pre-G1 (the honest degrade, stated up front):** plain-text editing with
  entry-rejection spans + optional dry-bind preview diagnostics via `PreviewService`; reference
  readout from `ReferenceResolutionProjection.token_span`. No fake token colors. G1 landing
  upgrades every context at once with no Bridge API change.
- Editing grammar: modeless 1-bit (SELECTED vs EDITING); Enter commits through the context's
  single entry verb (`EnterGridCell` / `EditContent` / name form); Esc exact-reverts; point-mode
  reference insertion via `InsertFormulaReference` with explicit spans; F4 reference-form
  cycling is an OxFml ask (file at S0 if absent; degrade = no-op with atlas note).
- X-Ray affordance (mech 07/11): caret-in-token highlights the resolved target (stage
  renders the highlight); subexpression evaluate = drill surfaces where available (Bench now,
  others post-G1).

## 7. Registry, Inspector, Strip

- **Registry** sections: Names (`DefinedNamesProjection` / tree names), Sheets or top-level
  nodes (`SheetProjection` / root keys), Tables, Extensions (G7; placeholder card with honest
  "not yet projected" state until then). Liveness dots: green fresh · signal stale/volatile ·
  red error — computed from projections, never stored. Rename affordances route through
  rename intents and show the refactor preview (mech 15) where the preview seam exists.
- **Inspector** is projection-fed (`ActiveSelectionDetailProjection` where available):
  value/shape block, format block (number format + provenance; grows with G2), dependency block
  (outgoing/incoming), stage-specific extension point (typed slot, no free-form injection).
- **Strip** readouts and their truth sources: calc chip (`WorkbookCalcProjection.mode`, dirty
  summaries, `CalcRunProjection` timings) · feeds (G7; absent until then — the slot renders
  nothing rather than a fake OK) · locale (host capability) · revision (`revision_history.
  current`) · zoom (stage-owned) · agent light (mech 20; lit while an MCP-driven intent stream
  is active, from dispatcher origin attribution) · **parity slot: reserved, renders nothing
  this phase** ([PARITY_TRUST_UX.md](PARITY_TRUST_UX.md) §5).
- Inspector's typed slot enum includes an `Evidence` variant, reserved and never populated
  this phase (same doc).

## 8. Theming & density

Strand tokens only; themes cockpit-light / cockpit-dark / high-contrast; density Working /
Reading (Reading applies inside Notebook and published views, not to chrome). No per-skin CSS
pipelines; component styles resolve through `--dna-*` custom properties. Contrast gates per
STRAND §2.1 run in `dnacalc-strand` tests.

## 9. Parity & testing (D3)

- Every S0 bead lands browser + Tauri: browser via `wasm-bindgen-test --headless` harness
  (estate pattern), desktop via Tauri build + smoke (window opens, shell mounts, keyboard verb
  dispatch works).
- Programmable-skin harness drives shell behavior tests through the real contract (mount,
  dispatch, assert projections/receipts) — no DOM needed for logic tests.
- Geometry: chrome is DOM; assert layout invariants (region sizes, collapse states, overlay
  stacking) in browser tests, not screenshots.
- Gates in CI: F-gate, P-gate, T0-gate (D5) + strand contrast tests.

## 10. S0 exit acceptance

1. Bench and Calc app skeletons mount the shell with their region subsets; stage switcher
   drives two stub stages in Calc; continuity state survives switching.
2. Bridge edits a real formula end-to-end in the Bench host (tokens, diagnostics, completion,
   commit, revert) and a workbook cell in the Calc host via the degrade path.
3. Keyboard: universal table active, collision test green, atlas overlay renders from the
   registry, one rebind persists and survives reload.
4. Command deck executes ≥ 15 commands incl. goto (A1, name); catalog enablement follows persona.
5. All four dependency/contrast gates green in CI on both targets.

## 11. Open questions (do not block S0 start)

- Mast document switcher for multi-document (post-S2; single document per window for now).
- Hold-to-peek keyboard atlas variant (progressive hints) — evaluate after atlas v1 usage.
- Reading-density toggle placement (Strip vs command-deck-only).
