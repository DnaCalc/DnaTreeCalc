# BENCH_SPEC — DNA OneCalc, the formula instrument

Status: v0.9 ratification-candidate · 2026-07-13 · gates S1 beads (after the D5 import bead).
Parents: [REDESIGN_PROGRAM.md](REDESIGN_PROGRAM.md) · [SHELL_SPEC.md](SHELL_SPEC.md) ·
[STRAND_DESIGN_LANGUAGE.md](STRAND_DESIGN_LANGUAGE.md).

## 1. Product definition

DNA OneCalc is the single-formula instrument: author one formula, understand it completely,
trust its result. It carries the existing charter mission — Twin Oracle Workbench with Live
Formula Semantic X-Ray — onto the Strand shell. Identity: cyan `dot-long` badge; one window,
one formula space; **F-gate: the app's dependency graph never contains `oxcalc*`** (TF host:
OxFml + OxFunc only). The Twin Oracle *identity* persists, but its parity UI is deferred out
of this phase — seams only, per [PARITY_TRUST_UX.md](PARITY_TRUST_UX.md).

Sentence on the box: *"I want to understand exactly what this formula does."*

## 2. Layout

Shell composition (SHELL_SPEC §1): Mast (no stage switcher) · **Bridge, hero form** (two-row:
entry row + persistent readout row) · **Result stage** · **X-Ray panel** (dockable bottom, F8
toggles) · Inspector · Strip. No Registry; the Extensions manager is an overlay reached from
Strip/Inspector.

```
┌ Mast: [◗▬ OneCalc] scenario-name.dnaone            persona ┐
├ Bridge: [name] [ =XLOOKUP(Code, Rates[ISO], Rates[Fx]) ]   │
│         readout: Rates[ISO] → 21×1 text · caret at arg 2   │
├──────────────────────────────┬─ Inspector ─────────────────┤
│  Result stage                │ format · CF · locale        │
│  18.4302  (number)           │ provenance readout          │
│  [array window when RxC]     │ extension context           │
├─ X-Ray (F8): drill tree ─────┴─────────────────────────────┤
└ Strip: ● Ready · en-ZA · rev 12 · ext 2 ok · [parity slot: reserved, empty] ┘
```

## 3. IR bindings (every affordance names its surface/intent)

| Affordance | Read | Write |
|---|---|---|
| Editor text/caret/selection | `FormulaEditorSurface` (source_text, caret, syntax_runs, diagnostics, metrics) | `OneFormulaIntent::EditText`, `SetSelection` |
| Entry mode chip (formula/value/text/empty) | `OneFormulaProjection.entry_mode` | — (host classifies; never skin-side) |
| Completion popup | `CompletionSurface` (px anchor, items×7 kinds, docs ref) | `ApplyCompletion{proposal_id}` |
| Signature help / function help | `SignatureHelpSurface`, `FunctionHelpSurface` | — |
| Result (scalar/error) | `FormulaResultSurface::Display/Error` + `CalcValueProjection` | `Recalculate` |
| Result (array) | `FormulaResultSurface::Array` + `ArrayWindowProjection` (≤4096 cells) | `RequestResultArrayWindow{offsets, counts}` |
| X-Ray drill | `FormulaDrillSurface` (per-node value_preview, array_preview, state, spans) | `ToggleFormulaDrill`, `RequestDrillArrayWindow` |
| Formatting panel | `FormattingSurface` (number format, font/fill, date1904, locale_language_tag, CF typed rules) | `SetNumberFormat`; further verbs = S1 asks (see §8) |
| Scenario policy | `FormattingSurface.scenario_policy` | `SetScenarioPolicy` |
| Status | `FormulaStatusSurface` (bridge_health, truth_source, scenario_label, load_diagnostics) | — |
| Parity (deferred) | seam only: `HostCapabilityProjection.replay_or_comparison` + reserved Strip slot — see [PARITY_TRUST_UX.md](PARITY_TRUST_UX.md); nothing renders this phase | — |
| Documents | `PersistenceProjection` (recents, dirty, paths) | `SkinShellIntent::{Open, OpenRecent, Save, SaveAs}` |
| Extensions | G7 minimal slice (see §6) | provider lifecycle commands via host (typed, not raw) |

## 4. The X-Ray (mechanisms 07 + 11, at full fidelity here)

- Caret in a token → its span lights in the editor; if the token resolves (name, table
  column), the readout row shows target + shape + a bounded preview.
- **Select any subexpression → partial-evaluation pill** rendered in place above the selection:
  value, type, shape (drill node matched by source span). Non-destructive; Esc dismisses; F9
  with a selection evaluates just the selection (Excel-F9 muscle memory, without destroying
  the formula).
- Step-out rings: repeated `]` widens the evaluated span to the enclosing expression;
  `[` narrows back. The X-Ray panel mirrors the current ring as a highlighted drill row.
- Drill rows: expression text, state chip (Pending/Evaluated/Bound/Skipped/Opaque/Blocked/
  Error), value preview, argument name/role where known; array rows page through
  `RequestDrillArrayWindow` (64×64 window cap honored).
- Every pill and drill row is addressable (mechanism 20): stable node ids from
  `FormulaDrillNodeProjection.node_id`.

## 5. Parity seam (designed-for, deferred — owner direction 2026-07-13)

Bench will eventually be the fullest parity instrument (per-axis diff, evidence provenance,
witness hop). **None of it is built in S1.** This spec reserves exactly two footprints: the
empty Strip parity slot and the Inspector "Evidence" slot variant, both rendering nothing.
State vocabulary, diff anatomy, and the P-register of future IR asks live in
[PARITY_TRUST_UX.md](PARITY_TRUST_UX.md) — beads must not cite them.

## 6. Extensions manager v0 (G7 minimal slice)

Overlay listing providers from the extension catalog: name, kind (VBA/XLL/RTD/native), state
(Available / Loading / Quarantined / Rejected / Unavailable-on-this-runtime), diagnostics, and
per-runtime honesty (BrowserWasm: native providers explicitly "requires desktop or companion" —
`RuntimeProfileProjection` × `ExtensionPlacementProjection` legality matrix already in the IR).
Function-to-provider attribution appears in Inspector when the caret is on a provided function.
The G7 ask defines the projection; until it lands, the manager renders catalog data available
in-process on desktop and the honest placeholder in browser. No lifecycle actions in v0 beyond
enable/disable where the host exposes them typed.

## 7. Formatting · CF · locale panel

- Number format: code entry + preset gallery (General, thousands, currency, date, percent,
  scientific); live preview against the current result via the host's render (never a skin-side
  formatter).
- Conditional formatting: list of typed rules (ColorScale / DataBar / IconSet / Rank / Average)
  with threshold editors matching `CfThreshold` forms; result preview on the array window
  (`ArrayCellFormatProjection` already carries per-cell CF outcomes).
- Locale: `locale_language_tag` selector + date1904 indicator; switching re-renders results
  through host truth. This panel is the proving ground for the Calc-wide G2 family — panel
  components are built TP-pure so they graduate unchanged.

## 8. S1 asks (file at kickoff; degrade documented per SHELL_SPEC rules)

- CF rule authoring verbs and font/fill set verbs on the OneFormula document (read surfaces
  exist; write verbs beyond `SetNumberFormat`/`SetScenarioPolicy` do not).
- F4 reference-form cycling in the editor service (OxFml).
- G7 minimal extension projection (inventory + states + diagnostics).

(The former oracle-projection ask moved to the P-register in PARITY_TRUST_UX.md — not filed
this phase.)

## 9. Keyboard (beyond universal)

F8 X-Ray panel · F9-with-selection partial eval · `]`/`[` ring out/in · Ctrl+M format panel ·
Alt+O oracle detail. All in the atlas; all rebindable.

## 10. Migration & testing

- D5 import precedes S1: crates arrive as `dnacalc-bench-core/-host/-desktop`; existing
  acceptance families (`run-host-acceptance-*`, browser suite, verification CLI) keep running
  unmodified through the rename bead, then fold into workspace CI lanes.
- The existing OneCalc preview UI is reference-only; S1 builds the Strand shell composition
  fresh (D1) while `dnaonecalc` UI code is retired with the rename.
- Parity: every S1 bead lands browser + Tauri; browser tests drive the Bridge/X-Ray through the
  real host; the fixture set reuses the verification corpus (twin-oracle cases double as UI
  fixtures).

## 11. S1 exit acceptance

1. Author `=XLOOKUP(Code, Rates[ISO], Rates[Fx])` from empty: completions, signature help,
   staged diagnostics, commit, result — browser and desktop.
2. X-Ray: select `Rates[ISO]` → pill shows 21×1 text with preview; ring out twice reaches the
   full call; drill panel mirrors; array windows page.
3. Array result (spilling formula) renders windowed with shape readout and truncation honesty.
4. Format panel: number-format change re-renders result via host; a CF color-scale rule shows
   per-cell outcomes in the array window; locale switch to `af-ZA` re-renders dates/decimals.
5. Extensions overlay states are honest per runtime; F-gate + P-gate green in CI.
6. No parity UI anywhere: the reserved slots render nothing (asserted — an accidental
   placeholder is a bug, per PARITY_TRUST_UX §8).
