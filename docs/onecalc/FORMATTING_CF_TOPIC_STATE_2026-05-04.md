*Posted by Codex agent on behalf of @govert*

# Formatting / Conditional-Formatting Topic — State of Play (2026-05-04)

This note inventories where the formatting and conditional-
formatting capability stands across the OneCalc / OxFml stack on
the date the topic was paused for OxFml work to land.

## Architecture in one paragraph

Format codes and CF rules live as host-editable state on the
active formula's `FormulaFormattingState`. The host translates
that state into a `VerificationPublicationContext` and a
locale-aware `TypedContextQueryBundle` per bridge call; OxFml's
publication surface evaluates CF rules and renders the
`effective_display_text`. Defaults for hint-driven auto-format
come from a workspace-level `AmbientAppContext` derived from the
browser's `navigator.language` at startup.

## Mental model for array results

A DnaOneCalc result hero showing an array result behaves *as if*
the user had selected the whole spilled result area in Excel and
applied the formatting / CF rules to that selection. Per-cell
formatting falls out of evaluating each cell against the rules
with the array as the implicit "selected range" — colour scales
gradient between the array's min and max, data bars proportional
to each cell's position in the array's range, icon sets bin by
quantile, top-N highlights the top-N cells of the array,
unique/duplicate compares within the array, etc. This is the
shared framing across the per-cell + aggregate-CF handoffs.

## What works today (post-W072, full per-cell + visualization CF wired)

- **Format-code rendering** for every preset chip exposed in the
  formatting panel: General, Number (with separators), Currency,
  Accounting, Percent, Fraction, Scientific, Date, Date (long),
  Time, Time (12h), Datetime, Text. All flow through OxFml's
  format engine and render correctly under en-US.
- **Custom format codes** the user types into the panel —
  anything OxFml's `render_with_code` accepts.
- **Auto-apply on General default** for the `DateLike` /
  `Currency` / `Percentage` / `Scientific` / `Fraction` hints
  emitted by OxFunc functions (`=NOW()`, `=TODAY()`, et al.).
- **Date vs. datetime hint disambiguation** by the value's
  fractional part — `=TODAY()` (integer serial) lands on
  `date_format_code`, `=NOW()` (fractional) lands on
  `datetime_format_code`.
- **AmbientAppContext** with platform-derived defaults from
  `navigator.language`; user can override the format codes
  through the (forthcoming) workspace preferences.
- **CF rules evaluated by OxFml** (W070 / W071 / W072):
  - **Per-cell (operator / predicate) rules:**
    - `cell_value` with operators `greaterThan` /
      `greaterThanOrEqual` / `lessThan` / `lessThanOrEqual` /
      `equal` / `notEqual` / `between` / `notBetween`.
    - `text` with operators `containsText` /
      `notContainsText` / `beginsWith` / `endsWith`.
    - `expression` with AND / OR-combined formulas.
    - `dates` with relative-date thresholds (`today`,
      `yesterday`, `tomorrow`, `last7Days`, `thisWeek`,
      `lastWeek`, `nextWeek`, `thisMonth`, `lastMonth`,
      `nextMonth`) — evaluated against the bridge's
      `now_serial`.
    - `blanks` / `noBlanks` — blank-value predicates.
    - `errors` / `noErrors` — error-value predicates.
  - **Aggregate (array-as-range) predicates (W072):**
    - `aboveAverage` / `belowAverage` — array mean.
    - `top` / `bottom` — top/bottom-N or top/bottom-N% of the
      array.
    - `uniqueValues` / `duplicateValues` — count occurrences
      within the array.
  - **Aggregate (array-as-range) visualizations (W072):**
    - `colorScale` — gradient fill from min/mid/max stops.
    - `dataBar` — proportional bar fill from `fill_color`.
    - `iconSet` — equal-width min/max numeric bins.
- **Per-cell CF on Array values** (W071): the bridge surfaces
  `VerificationPublicationSurface.array_cell_format` and the
  host renders per-cell font/fill, data bars, and icons in the
  array browser. Each cell of a `=SEQUENCE(2,3)` result with a
  `cell_value > 3` rule highlights independently.
- **Visualization-rule authoring defaults**: picking a
  visualization kind in the rule-kind dropdown auto-seeds sane
  thresholds / colour values — `colorScale` gets the standard
  3-stop red-yellow-green, `dataBar` gets a default blue,
  `iconSet` defaults to `3Arrows`, etc. The user can override
  the seeded values; defaults make the rule immediately
  functional without a config dialog.
- **Icon-set rendering**: Unicode glyphs map the 3 / 4 / 5-icon
  sets (`3Arrows`, `3TrafficLights1`, `3Signs`, `3Symbols`,
  `3Flags`, `4Rating`, `4Arrows`, `4TrafficLights`, `4RedToBlack`,
  `5Arrows`, `5Rating`, `5Quarters`).
- **CF rule persistence** — host CF rules round-trip through
  `<dna:CfRules>` in the saved scenario; ScenarioPolicy
  (Deterministic / LiveRecalc) round-trips too.
- **Result-foot context chip** reads live state: `format` reflects
  the active formula's number-format-family (no longer hardcoded
  `"GENERAL"`); `policy` reflects the formula's scenario policy
  (Deterministic vs. LiveRecalc).
- **F9 + Calculate button** force a fresh bridge round-trip.
- **CF-applied colours render on the result hero** for scalar
  values: the host now reads
  `verification_publication_surface.effective_font_color` /
  `effective_fill_color` and applies them via inline `style` on
  the `.value` element. `data-cf-applied="true"` flag is
  surfaced for tests / visual debugging. The same fields will
  carry custom-format colour tokens once the upstream colour-
  token publication handoff lands (transparent upgrade — no
  host change needed).
- **Nearest-profile mapping** for `navigator.language` →
  `LocaleProfileId` is in place
  (`services::ambient_app_context::nearest_locale_profile_for_language_tag`),
  ready to wire through the moment OxFml's locale tables land.
- **Deterministic vs. LiveRecalc** seeding: Deterministic pins
  `now_serial = 46000.0` and `random_provider = 0.5`; LiveRecalc
  derives both from the platform clock / RNG.

## Outstanding upstream work (filed as handoffs)

| Topic | Handoff | Status | Visible host gap until landing |
|---|---|---|---|
| Custom-format colour-token surfacing | `docs/HANDOFF_OXFML_FORMAT_COLOUR_TOKEN_PUBLICATION.md` (focused, broken out from custom-format-grammar lane 2a) | OxFml-local follow-up | Power-user codes like `[Red]#,##0;[Blue](#,##0)` render numerically correctly but the colour token has no effect on the result hero |
| Custom-format text 4th section | `docs/HANDOFF_OXFML_CUSTOM_FORMAT_GRAMMAR.md` (lane 2b) | OxFml-local follow-up | Codes with a text section like `0.00;-0.00;"-";@` render text values empty rather than running the `@` placeholder |
| Multi-locale tables (month / weekday names, separators, currency) | `docs/HANDOFF_OXFML_LOCALE_EXPANSION.md` | OxFunc W094 first slice in progress; OxFml will consume after | Result-foot locale chip stays SEAM-pending; non-en-US users see English month/weekday names regardless of `navigator.language` |
| Custom-format locale-prefix grammar (`[$-040C]…`) | `docs/HANDOFF_OXFML_CUSTOM_FORMAT_GRAMMAR.md` (lane 3) | Locale-blocked (`BLK-FML-005`) | Locale-prefixed codes ignore the prefix and render under the active workspace locale |
| Typed visualization-rule payload | OxFml W073 (`HANDOFF-DNAONECALC-012_W073_TYPED_CF_PAYLOAD_FIRST_SLICE.md`, 2026-05-04 update) | **Landed** — `VerificationConditionalFormattingRule.typed_rule` is now the **only** accepted metadata source for `colorScale`, `dataBar`, `iconSet`, `top`, `bottom`, `aboveAverage`, `belowAverage`. The W072 bounded-string `thresholds` convention is intentionally ignored upstream for those families. Host bridge populates `typed_rule`; persistence loader drops stale `thresholds` for W073 kinds | None — rules without `typed_rule` for these kinds simply do nothing, which the host now prevents by always seeding a typed rule on kind switch |
| Virtualised array browser | (host-side, not upstream) `SEAM-ONECALC-ARRAY-BROWSER-VIRTUALIZATION` | Pending host follow-on slice | Array results above 1000 rows × 100 cols (post-bump cap) still show a `+N rows / +M cols hidden` chip; transmitting all formatted strings through the bridge would dominate a keystroke at higher caps without virtualization |

## Out-of-scope deliberately

- **Worksheet-range CF semantics** (rules attached to a real
  worksheet range as a separate carrier from the formula). The
  result hero treats the formula's spilled array as the implicit
  range; standalone worksheet-range CF needs its own surface and
  its own handoff if and when DnaOneCalc grows that capability.
- **Custom palette resolution for `[ColorN]` indexed tokens**
  beyond the published Excel default palette. Workbook-defined
  palettes don't apply at the formula-only level and don't have
  a host-side knob.

## Closed handoffs

- `docs/HANDOFF_OXFML_FORMAT_ENGINE_TIME_FRACTION_ACCOUNTING.md`
  (W069, time / datetime / fraction / accounting tokens).
- `docs/HANDOFF_OXFML_CF_PREDICATE_AND_RELATIVE_DATE_RULES.md`
  (W070, blanks / noBlanks / errors / noErrors / dates).
- `docs/HANDOFF_OXFML_CF_ARRAY_PER_CELL.md`
  (W071, per-cell carrier on `array_cell_format`).
- `docs/HANDOFF_OXFML_CF_AGGREGATE_VISUALIZATION_RULES.md`
  (W072, aggregate predicates + colorScale / dataBar / iconSet
  visualizations).
- `docs/HANDOFF_OXFML_FUNCTION_HELP_FROM_OXFUNC_REGISTRY.md`
  (W068 function-help path).
- `docs/HANDOFF_OXFML_COMPLETION_PROPOSALS_FROM_REGISTRY.md`
  (W068 completion-proposal path).
- `docs/HANDOFF_OXFML_LET_LAMBDA_AND_SIGNATURE_HELP.md`
  (LET/LAMBDA spurious-diagnostic + signature-help past close
  paren).

## Next host-side step (when remaining upstream lands)

The two big CF chains (per-cell + aggregate visualization) are
done. The remaining lanes are smaller:

1. **OxFml lands custom-format colour-token publication**:
   - No host-side change required; the host already reads
     `effective_font_color` from the publication surface.
2. **OxFml lands the text 4th section**:
   - No host-side change required; user-typed codes like
     `0.00;-0.00;"-";@` start working transparently.
3. **Locale tables land in OxFml** — **DONE 2026-05-06**
   (OxFunc W094 + OxFml `oxfml_locale_context` shipped):
   - Bridge `FormulaEditRequest` now carries a `language_tag`,
     plumbed through `app::intents::ApplyFormulaEditIntent` and
     `services::live_edit::build_live_edit_intent` from
     `OneCalcHostState.ambient_app_context.language_tag`.
   - `live_bridge::build_runtime_locale_context` resolves the tag
     through `LocaleProfileId::from_bcp47_language_tag` and builds
     a runtime `LocaleFormatContext` per call (en-US falls back to
     the static `oxfml_en_us_locale_context()` for an
     allocation-free path).
   - `formatting_controls.locale_seam_id` defaults to `None`. The
     workspace-locale dropdown UI is live; switching presets
     flips the date-format triple, runtime month / weekday tables,
     decimal / thousands separators, currency symbol, and
     `General` rendering on the next bridge round-trip.
   - Coverage pinned in `tests/seams/locale_expand.rs` with three
     fast assertions against `CANONICAL_LOCALE_PROFILE_IDS`,
     `from_bcp47_language_tag`, and per-locale separator
     differences.
4. **OxFml lands locale-prefix grammar** (depends on item 3, now
   independently unblockable):
   - No host-side change required; user-typed codes like
     `[$-040C]dddd, d mmmm yyyy` start working transparently.
5. **Typed CF visualization-rule authoring UI** — landed
   2026-05-04 against OxFml W073, then tightened later the same
   day when OxFml removed the W072 bounded-string fallback:
   - `state::FormulaConditionalFormattingTypedRule` mirrors
     `oxfml_core::publication::ConditionalFormattingTypedRule`
     (color_scale / data_bar / icon_set / rank / average sub-
     options, threshold + rank + direction enums).
   - Persistence: `CfRule` carries `typed_rule: Option<CfTypedRule>`
     round-tripping through `<dna:TypedRule>` JSON-encoded child
     elements; covered by five round-trip tests. The persistence
     loader drops bounded `thresholds` for the seven W073-typed
     kinds at load time — those would just sit on the rule unread
     after OxFml's compatibility removal.
   - Bridge: `FormulaFormattingCfRule.typed_rule` plumbs the host
     shape into `VerificationConditionalFormattingRule.typed_rule`.
     The W073 typed kinds emit `typed_rule` only; bounded
     `thresholds` are no longer authored for them and the
     threshold-control UI is hidden for those kinds.
   - Authoring UI: each visualization rule card grows a per-kind
     sub-form (color-scale stop list with stop-kind pickers,
     data-bar bar-colour + direction + show-bar-only, icon-set
     16-kind dropdown, rank count/percent toggle, average
     include-equal + stddev offset).
   - `seed_visualization_rule_defaults` always seeds a typed
     payload on kind switch and clears any stale bounded
     `thresholds` so the rule is immediately functional under
     the post-fallback upstream.
   - Open follow-ups (post-MVP): inline gradient / data-bar
     preview swatches; explicit `formula` colour-scale stop
     once OxFml grows it; per-icon kind threshold pickers for
     mixed icon sets.
6. **Virtualised array browser** (host-side):
   - Track scroll position; render only the visible row range
     plus a buffer.
   - Bridge surface optionally chunks per-cell carriers so
     gigantic arrays don't dominate a keystroke.

After those four, the formatting and CF capability is closed at
the current scope. Worksheet-range CF, rich-value formatting
(rich-value display inheritance), and any OxFunc-level locale
work beyond the current `LocaleProfileId` set are explicitly
out-of-scope and tracked separately.
