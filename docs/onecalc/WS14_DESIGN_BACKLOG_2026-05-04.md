*Posted by Codex agent on behalf of @govert*

# WS-14 design backlog — bigger items needing design before code

This note captures the items the user flagged on 2026-05-04 that
need design / requirements work before they're worth building.
Each section is intentionally self-contained: when one is picked
up, it's a single design slice followed by a single
implementation slice, no cross-dependency.

---

## 1. Pin / clone / manage formulas

The user asks: "How to pin formula? How to clone / manage
different ones — needs design and requirements specs."

The shell already has the affordances at the schema level —
`workspace_shell.recent_formula_space_order`,
`workspace_shell.pinned_formula_space_ids` (`BTreeSet`),
`workspace_shell.open_formula_space_order`, and the scenario
breadcrumb dropdown. What's missing is a coherent UX that
covers:

### Required surface

- **Active formula** — the one currently in the home shell.
  Already has a breadcrumb chip (`[unsaved] ▾` or `[name] ▾`).
- **Pinning the active formula** — one-click action that adds
  the formula's id to `pinned_formula_space_ids`. Pinned
  formulas survive scenario-cleanup and surface in the
  breadcrumb dropdown's "Pinned" section.
- **Cloning the active formula** — duplicates the formula's
  state (raw text + formatting + CF rules + scenario policy) to
  a new formula-space id. The new id becomes the active formula.
  Distinct from "save as" which renames in place.
- **Switching between formulas** — open multiple formulas, see
  them as tabs / cards, switch by clicking. The schema supports
  it (`open_formula_space_order`), the UI doesn't surface it.

### Design questions to settle

- **Tabs vs. dropdown vs. side rail?** WS-14's "no persistent
  left rail" thesis is firm — formulas are not peers, the active
  one is privileged. So *not* a side rail. Options:
  - **Tab strip above the editor**: small, dense, one chip per
    open formula, click to switch. Standard browser-app pattern.
  - **Dropdown only**: keep everything in the breadcrumb
    dropdown's "Recent" / "Pinned" lists; no tab strip. Lower
    chrome footprint, slower switching.
  - **Tabs as overflow inside the breadcrumb dropdown**: the
    breadcrumb chip itself never grows; the dropdown is the
    only way to switch. Cleanest but loses muscle-memory click-
    to-switch.
- **How many open formulas at once?** The current schema has
  no cap. Pre-MVP: cap at maybe 8 (to avoid hidden state
  growth) and surface a "close other tabs" action.
- **Pin semantics**: "pinned" means "always visible in the
  breadcrumb dropdown" (workspace-level) vs. "always visible in
  the tab strip" (per-session)? Pre-MVP proposal: pinned is
  workspace-level; the tab strip shows open formulas only,
  pinned ones don't auto-open.
- **Clone vs. duplicate vs. save-as** — terminology matters:
  - `Save as` (existing) — names the active formula, sticks.
  - `Duplicate` (existing) — copies the active formula to a new
    id, both stay open.
  - `Clone` (proposed name) — same as Duplicate; pick one term.
- **Naming UX**: when the user clones, do they get a name
  prompt immediately, or does the clone start as `[unsaved
  copy of <name>]`? The latter is faster; the former is more
  intentional.

### Minimum viable surface

A small slice that delivers the core capabilities:

1. **Pin button** in the breadcrumb dropdown's per-row affordance
   (already drafted in `ScenarioBreadcrumbAction`; today it just
   carries a SEAM marker — wire it to `add_pinned_formula_space`
   reducer).
2. **Tab strip** between titlebar and editor caption, one chip
   per `workspace_shell.open_formula_space_order` entry, with
   the active one styled distinctly. Click switches. Tiny `✕`
   on each chip closes (with dirty-check confirmation).
3. **`+ new formula` button** at the right of the tab strip —
   alias for `Ctrl+N`.
4. **Drop the breadcrumb dropdown's "Open scenario" file path**
   for in-session switching; that action stays for opening
   `.dnafml` files from disk. In-session switching uses the tab
   strip.

State / reducer work:

- `pin_active_formula_space(state)` — adds the active id to
  `pinned_formula_space_ids`.
- `unpin_formula_space(state, id)` — removes.
- `clone_active_formula_space(state)` → returns the new id (or
  `None` when no active space). Reuses
  `case_lifecycle::new_formula_space` for the id-allocation
  half; copies `formatting` + `formula_drill_open` + raw text +
  `committed_cell_text` from the source.

Persistence: `workspace.json` already has the slots. Pin
state survives reload as long as workspace persistence ships.

---

## 2. Typed CF visualization-rule authoring UI — LANDED

User flag #1 / #4 from the earlier turn: the bounded-payload
conventions for `colorScale` / `dataBar` / `iconSet` worked as
seeds, but power-user authoring was awkward (free-text
`"min:#F8696B"` strings, no picker for the icon set, no preview
of the gradient).

OxFml `HANDOFF-DNAONECALC-012` (W073) shipped the typed payload
**and, in the 2026-05-04 update, removed the W072 bounded-string
fallback** for the seven typed families. As of that update,
`VerificationConditionalFormattingRule.typed_rule` is the only
accepted metadata source for `colorScale`, `dataBar`, `iconSet`,
`top`, `bottom`, `aboveAverage`, `belowAverage`. The host has
landed the matching authoring UI and persistence wiring; this
section is kept as the design record.

### Upstream shape (mirror this in the host)

`oxfml_core::publication::ConditionalFormattingTypedRule`:

```
struct ConditionalFormattingTypedRule {
    color_scale: Option<ColorScaleRuleOptions>,
    data_bar:    Option<DataBarRuleOptions>,
    icon_set:    Option<IconSetRuleOptions>,
    rank:        Option<RankRuleOptions>,
    average:     Option<AverageRuleOptions>,
}

enum ConditionalFormattingThreshold {
    Min, Mid, Max,
    Percent(f64),    // 0..100
    Percentile(f64), // 0..100
    Number(f64),
}

struct ColorScaleStop { threshold: …, color: Rgba }
struct ColorScaleRuleOptions { stops: Vec<ColorScaleStop> }

enum DataBarDirection { LeftToRight, RightToLeft }
struct DataBarRuleOptions {
    minimum: Option<…>, maximum: Option<…>,
    bar_color: Rgba,
    direction: DataBarDirection,
    show_bar_only: bool,
}

struct IconSetRuleOptions {
    kind: String,                // e.g. "3Arrows", "5Rating"
    thresholds: Vec<…>,          // n-1 thresholds for n icons
}

enum ConditionalFormattingRank { Count(usize), Percent(f64) }
struct RankRuleOptions { from_top: bool, rank: … }

struct AverageRuleOptions {
    above: bool, equal_average: bool,
    stddev_multiplier: Option<f64>,
}
```

### Host-side migration plan

1. **Mirror the shape** in
   `dnaonecalc-host::persistence::formula_file` as
   `FormulaConditionalFormattingTypedRule` plus the four
   sub-options structs and the two enums. All fields
   `Option<...>` so empty rules round-trip. Keep host names
   aligned with upstream (color_scale / data_bar / icon_set /
   rank / average).
2. **Bridge it** in
   `dnaonecalc-host::adapters::oxfml::bridge` — when the host
   typed rule is set, populate the upstream
   `verification_request_types::ConditionalFormattingTypedRule`
   and attach via `typed_rule: Some(...)`. The W072
   bounded-string `thresholds` continues to ride along as the
   compatibility fallback during this transition.
3. **Round-trip** through `.dnafml` JSON: the typed rule is a
   sibling of the existing `kind` + `thresholds` fields. Keep
   both for now; drop the bounded strings only after a
   deprecation cycle.

### Per-kind UI surfaces

One rule card grows a per-kind sub-form (replaces the free-text
`thresholds` field for the supported families):

- **`colorScale`**: 2-stop / 3-stop toggle. Each stop is a
  `(stop_kind, value?, color)` triple. Stop-kind picker:
  `min` / `max` / `mid` / `percent` / `percentile` / `num`.
  Color picker per stop. Inline 24×8 gradient preview swatch.
- **`dataBar`**: min / max threshold (same stop-kind picker as
  colorScale, both `Option`), bar colour picker, `direction`
  toggle (Left / Right), `showBarOnly` checkbox. Inline bar
  preview.
- **`iconSet`**: kind dropdown showing all 16 published Excel
  sets (`3Arrows`, `3ArrowsGray`, `3Flags`, `3Symbols`,
  `3Symbols2`, `3Stars`, `3Triangles`, `4Arrows`, `4ArrowsGray`,
  `4RedToBlack`, `4Rating`, `4TrafficLights`, `5Arrows`,
  `5ArrowsGray`, `5Rating`, `5Quarters`); per-icon threshold
  values (above which the icon kicks in).
- **`top` / `bottom`**: count / percent toggle and numeric
  input.
- **`aboveAverage` / `belowAverage`**: `equal_average`
  checkbox and optional stddev multiplier input.

### Compatibility / fallback

For the seven W073-typed families, OxFml no longer reads
bounded `thresholds`; the typed sub-form is the rule's only
authoring path. `seed_visualization_rule_defaults` always seeds
a `FormulaConditionalFormattingTypedRule` on kind switch and
clears any stale bounded thresholds. The persistence loader
drops bounded thresholds for these kinds at load time so older
saved files migrate cleanly.

For rule kinds outside the W073 set (`cell_value`, `text`,
`dates`, `expression`, `blanks` / `noBlanks`, `errors` /
`noErrors`, `uniqueValues` / `duplicateValues`) the existing
threshold-text input stays — those kinds still take their
operands through `thresholds` per OxFml W070/W072.

---

## 3. Print collision & command-palette UX

Decided this turn: `Ctrl+K` is the canonical chord (modern app
convention — VS Code, GitHub, Linear); `Ctrl+Shift+P` is the
discoverable secondary chord (also VS Code). `Ctrl+P` collides
with the browser print dialog and we don't override it.

The chord is wired at the shell level (works regardless of
which surface has focus); the actual palette UI is
SEAM-pending under `SEAM-ONECALC-COMMAND-PALETTE`.

### Palette content (when it lands)

- **Scenario actions**: New, Save as, Open, Duplicate, Pin,
  Unpin, Close — same set as the scenario breadcrumb dropdown.
- **Recent / Pinned formulas**: typing the name jumps.
- **Workspace settings**: locale picker, format presets,
  developer view, …
- **Compare**: `Compare with Excel` (titlebar button mirror).
- **Function reference**: typing `=SUM` shows the function-help
  packet inline in the palette (same content as the hover-help
  card).

The palette is its own component; the chord just toggles a
state flag (`global_ui_chrome.command_palette_open`).

---

## 4. Cursor-past-`)` & popup-update behaviour

These were the two editor-side concerns in the user's list.
Both are now wired through the new caret-sync handlers
(`on:keyup` for arrow keys / Home / End / PageUp/Down,
`on:click` for mouse positioning). Each fires a synthetic
`EditorInputEvent` that re-runs the bridge against the new
caret position.

OxFml's `signature_help_context_at_cursor` already returns
`None` when the cursor is past the closing `)` of a closed
call, so the popup disappears the moment the bridge re-runs at
the new position — which now happens on every caret move.

Tab-to-accept-completion: already wired (`on_textarea_keydown`
intercepts `Tab` when popup is `Open`). If the popup isn't
auto-opening for a given prefix the user expects, that's a
proposal-collector behavior question and lives in OxFml's
W068 follow-on lanes.

---

## 5. Result-foot rethink — LANDED (2026-05-05)

The user said: "rethink the design a little bit — I like
bottom area below result, just not sure what goes there —
maybe it is not needed."

**Decision:** progressive — collapse on default, surface on
divergence, *always* surface in `ManualRecalc`.

`project_result_context` returns `Option<ResultContextChip>`:

- `None` (chip + chrome collapse) when:
  - format code empty (General)
  - no CF rules
  - policy is `LiveRecalc` (the default)

  In this state the result-foot disappears entirely and the
  result hero gets its vertical space back. This matches the
  "maybe it is not needed" instinct — when nothing's
  configured, nothing's worth saying.

- `Some(...)` whenever any of the three diverges. The user has
  authored a format / a CF rule / a non-default policy, so the
  chip gives them visible state.

- `Some(...)` always when policy is `ManualRecalc`, regardless
  of format / CF state. ManualRecalc is the user's lever for
  decoupling typing latency from formula complexity (see §1
  for the full perf chain). The `manual-recalc` chip stays on
  screen as a reminder that text edits aren't running the
  runtime — the user reaches for F9 / Calculate when they want
  a fresh value.

Future enhancements (still optional):

- **Add a "last calculated at" timestamp** when the formula's
  cached value is older than N seconds — gentle reminder in
  `ManualRecalc` mode that the visible value is stale. Not
  yet implemented; the chip alone has been enough.
