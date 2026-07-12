*Posted by Codex agent on behalf of @govert*

# OxFml Handoff: Multi-Locale Expansion (Names, Separators, Currency)

Status: **LANDED 2026-05-06** — OxFunc W094 +
  `oxfml_locale_context` ship; host wired through
  `live_bridge::build_runtime_locale_context`. Originally blocked
  on OxFunc (`BLK-FML-005`, `HANDOFF-OXFUNC-006`).
Direction: DnaOneCalc → OxFml (with OxFunc co-implementation)
Source repo / workset: DnaOneCalc / Formatting closeout
Filed date: 2026-05-04
Triage acknowledged: 2026-05-04 in
  `OxFml/docs/handoffs/HANDOFF-DNAONECALC-007_W070_LOCALE_AND_CUSTOM_FORMAT_TRIAGE.md`
Related:
  `OxFml/crates/oxfml_core/src/format/{engine.rs,datetime.rs,locale_tables.rs}`,
  `OxFunc/crates/oxfunc_core/src/locale_format.rs`,
  `docs/HANDOFF_OXFML_FORMAT_ENGINE_TIME_FRACTION_ACCOUNTING.md` (W069 landed)

## Landing summary (2026-05-06)

Upstream:
- OxFunc W094 ships 30 canonical `LocaleProfileId` entries
  (`CANONICAL_LOCALE_PROFILE_IDS`), `format_profile(id)` returning
  per-locale `FormatProfile`s, and
  `LocaleProfileId::from_bcp47_language_tag(tag) -> Option<Self>`
  for resolving workspace tags.
- OxFml exposes `oxfml_core::format::oxfml_locale_context(profile,
  date_system)` to construct a runtime `LocaleFormatContext` for
  any locale, plus the existing `oxfml_en_us_locale_context()`
  static fast-path.

Host wiring:
- `FormulaEditRequest.language_tag` (BCP-47 tag) added to the
  bridge boundary in `adapters/oxfml/bridge.rs`.
- `app::intents::ApplyFormulaEditIntent.language_tag` mirrors the
  bridge field; `services::live_edit::build_live_edit_intent`
  populates it from
  `OneCalcHostState.ambient_app_context.language_tag`.
- `services::editor_session::handle_formula_edit_intent` copies
  the tag through to the bridge request.
- `adapters/oxfml/live_bridge::build_runtime_locale_context`
  resolves the tag (en-US fallback when empty / unrecognised) and
  builds `LocaleFormatContext` per round-trip.
- `services::home_shell_view_model::FormattingControlsView.locale_seam_id`
  defaults to `None`. The workspace-locale dropdown surfaces in
  the formatting panel and is fully functional today.

Test pin:
- `tests/seams/locale_expand.rs` un-ignored, replaced with three
  positive assertions against the upstream API.

## Blocker note (2026-05-04, in progress)

OxFml triaged this handoff in W070's `HANDOFF-DNAONECALC-007`
and confirmed the direction is correct, but the locale-profile
portion is blocked on **OxFunc exposing canonical locale profile
identities and constants**. OxFml filed `HANDOFF-OXFUNC-006`
(`BLK-FML-005`); OxFunc has acknowledged in
`OxFunc/docs/handoffs/HANDOFF-OXFUNC-006_W070_LOCALE_PROFILE_EXPANSION_REQUEST.md`
and is landing W094's first slice with explicit profile ids and
`format_profile(...)` rows for:

```
en-US, en-GB, de-DE, fr-FR, es-ES, it-IT, nl-NL, pt-BR, ja-JP,
ko-KR, zh-CN, current-excel-host
```

OxFml will then consume the new profile ids, grow locale-keyed
month / weekday tables, and switch parser / general-render paths
to consult the profile rather than the `LocaleProfileId::EnUs`
shortcut.

### Long-tail locales not in the W094 first slice

OxFunc's `HANDOFF-OXFUNC-006` flags an open lane: the wider
DnaOneCalc ambient language-tag table covers locales beyond the
first 11 — `en-IE`, `en-AU`, `en-NZ`, `en-ZA`, `en-IN`, `en-CA`,
`en-PH`, `pt-PT`, `ru-RU`, `fi-FI`, `et-EE`, `lv-LV`, `lt-LT`,
`sk-SK`, `cs-CZ`, `nb-NO`, `nn-NO`, `pl-PL`, `hu-HU`. OxFunc
asks for either exact profile-id coverage *or* an explicit
nearest-profile policy from DnaOneCalc.

DnaOneCalc lands a `nearest_locale_profile_for_language_tag`
helper in `services::ambient_app_context` that picks the closest
profile from the W094 first slice for any unsupported tag (e.g.
`en-AU` → `en-GB`, `ru-RU` → closest Cyrillic profile when
available, otherwise `current-excel-host`, etc.). The mapping is
hand-curated and explicit; it goes live the moment OxFml's
profile-aware tables are reachable.

### Host status

- `AmbientAppContext` state slot exists; format codes (the
  *ordering* of date / datetime / time tokens) are already
  derived from `navigator.language`.
- `nearest_locale_profile_for_language_tag` is added in this
  slice (returns `LocaleProfileId`-shaped strings keyed off the
  W094 list) so the host is ready to construct a
  `LocaleFormatContext` once OxFml's tables compile in.
- `SEAM-OXFML-LOCALE-EXPAND` stays on the result-foot locale
  chip until the chain unblocks (OxFunc → OxFml → host wire-
  through).

## Summary

After W069 the OxFml format engine renders user-supplied codes
correctly across the date / time / fraction / accounting families.
What it cannot yet do is render those same codes against any
locale other than `en-US`: month and weekday name tables are
English-only, the only `LocaleProfileId` variants are `EnUs` and
`CurrentExcelHost`, and several locale-sensitive rendering paths
short-circuit through `if profile.id == LocaleProfileId::EnUs`.

DnaOneCalc has just landed a workspace-level `AmbientAppContext`
that derives the user's preferred date / datetime / time format
codes from `navigator.language` (mapping `de-DE` → `dd.mm.yyyy
HH:mm:ss`, `ja-JP` → `yyyy/mm/dd HH:mm:ss`, etc.). That gets the
*ordering* and *separators* right because the format codes
themselves carry that information. What it **cannot** fix from the
host side is anything sourced from the locale's data tables —
month names, weekday names, decimal separator inside numeric
codes, currency symbol resolution, the `General` rendering for
date serials.

Closing the formatting topic between OneCalc and OxFml requires
OxFml to grow proper multi-locale plumbing.

## Symptom from DnaOneCalc

User on a German Windows machine sees Excel render
`=TEXT(NOW(),"dddd, mmmm d, yyyy")` as
`Montag, 4. Mai 2026`. Same formula in DnaOneCalc renders
`Monday, May 4, 2026` regardless of how `navigator.language` is set
or what the user picks in the (forthcoming) workspace-locale UI —
because OxFml's `month_name` / `weekday_name` tables in
`crates/oxfml_core/src/format/locale_tables.rs` are hard-coded
English.

Same observation for the `General` rendering of a Number through
`oxfml_core::format::general` — decimal separator picks `.` for
every locale because the whole code path is keyed off the en-US
profile.

The same hint-default we already wire (DateLike → datetime code)
will continue to look subtly wrong in any non-en-US setting:
`Mai` vs. `May`, `lun.` vs. `Mon`, `Lundi` vs. `Monday`. Currency
formatting via `render_currency` is keyed on `profile.currency_symbol`
but every value of that symbol traces back to the en-US table
today.

## What lives where today

### `OxFunc::oxfunc_core::locale_format`

```rust
pub enum LocaleProfileId {
    EnUs,
    CurrentExcelHost,
}

pub struct FormatProfile {
    pub id: LocaleProfileId,
    pub decimal_separator: &'static str,
    pub thousands_separator: &'static str,
    pub date_separator: &'static str,
    pub time_separator: &'static str,
    pub currency_symbol: &'static str,
    pub currency_decimals: u8,
}
```

Two profile variants. `CurrentExcelHost` is a deliberate shape
that means "the host's regional settings"; today its values are
the same as `EnUs`. Adding more locales lives here as new enum
variants plus the matching profile constants.

### `OxFml::oxfml_core::format`

- `engine.rs::oxfml_en_us_format_profile()` and
  `oxfml_current_excel_host_format_profile()` — the only two
  profiles you can hand in.
- `engine.rs::OxFmlLocaleValueParser::parse_value_text` — every
  parser branch checks `profile.id == LocaleProfileId::EnUs`
  before parsing slash-form dates etc.
- `locale_tables.rs` — English-only `month_name` and
  `weekday_name`.
- `datetime.rs` and `number.rs` — render date / time / numeric
  codes. They already accept an injected `FormatProfile` for
  separators (decimal / thousands / date / time / currency
  symbol), but they call into `locale_tables.rs` for names.

The split is therefore: **OxFunc** owns the `FormatProfile`
*shape* and the *profile constants*; **OxFml** owns the *rendering
tables* and the *parser branching*.

## What needs to change

### 1. OxFunc — additional `LocaleProfileId` variants and profile constants

Grow `LocaleProfileId` to cover at least the locales that
DnaOneCalc's `services::ambient_app_context` already maps from
`navigator.language`:

```rust
pub enum LocaleProfileId {
    EnUs,
    EnGb,    // dd/MM/yyyy, '.' decimal, ',' thousands, '£' currency
    DeDe,    // dd.MM.yyyy, ',' decimal, '.' thousands, '€' currency
    FrFr,    // dd/MM/yyyy, ',' decimal, '\u{00A0}' thousands, '€' currency
    EsEs,    // dd/MM/yyyy, ',' decimal, '.' thousands, '€' currency
    ItIt,    // dd/MM/yyyy, ',' decimal, '.' thousands, '€' currency
    NlNl,    // dd-MM-yyyy, ',' decimal, '.' thousands, '€' currency
    PtBr,    // dd/MM/yyyy, ',' decimal, '.' thousands, 'R$' currency
    JaJp,    // yyyy/MM/dd, '.' decimal, ',' thousands, '¥' currency
    KoKr,    // yyyy/MM/dd, '.' decimal, ',' thousands, '₩' currency
    ZhCn,    // yyyy/MM/dd, '.' decimal, ',' thousands, '¥' currency
    CurrentExcelHost,
}
```

Each new variant gets a profile constant in
`locale_format::default_format_profile_for(LocaleProfileId)` (or
the equivalent existing surface) with the right separators,
currency symbol, and currency decimals.

The exact set above is suggestive; the goal is "the locales the
host already knows about" — see
`DnaOneCalc/src/dnaonecalc-host/src/services/ambient_app_context.rs`
for the full table that's populated from `navigator.language`.
Extending the enum is a one-line-per-locale change; the table
data is the slow part.

### 2. OxFml — locale-keyed name tables

`crates/oxfml_core/src/format/locale_tables.rs` becomes
locale-aware:

```rust
pub fn month_name(profile: &FormatProfile, month: i64, abbreviated: bool) -> &'static str {
    match profile.id {
        LocaleProfileId::EnUs | LocaleProfileId::CurrentExcelHost => en_us_month_name(month, abbreviated),
        LocaleProfileId::DeDe => de_de_month_name(month, abbreviated),
        // ...
    }
}

pub fn weekday_name(profile: &FormatProfile, index: usize, abbreviated: bool) -> &'static str {
    // same shape
}
```

Per-locale month / weekday tables for each new variant. Reference
data is the standard CLDR / ICU short-and-full names; Excel's
own tables are a faithful subset.

### 3. OxFml — locale-aware parsing branches

`engine.rs::OxFmlLocaleValueParser::parse_value_text` currently
short-circuits on `LocaleProfileId::EnUs` for slash-date parsing.
Replace with the locale's `date_separator`-driven parse (the
profile already carries it). Same for the `currency_symbol` strip.

### 4. OxFml — `General` rendering keyed on profile

`format::general::render_visible_number` and similar paths
should respect the profile's decimal separator. Today they're
implicitly en-US.

### 5. OxFml — locale-prefixed format codes (`[$-040C]…`)

Excel allows a locale prefix at the start of a format code:
`[$-040C]dddd, d mmmm yyyy` overrides the active locale for *that
code*. Out of W069's bounded scope; mention here so it's
considered alongside the locale tables. If this is a separate
slice, fine — but the tables it depends on are the same ones
this handoff asks for.

## Test coverage

In `oxfml_core`:

1. `month_name(&de_de_profile, 5, false) == "Mai"` and the
   abbreviated form is `"Mai"` (German short and full are
   identical for May).
2. `month_name(&fr_fr_profile, 1, true) == "janv."`.
3. `weekday_name(&ja_jp_profile, 1, false) == "月曜日"`.
4. `render_with_code(&de_de_profile, ..., 46045.0, "dddd, d. mmmm yyyy") == "Sonntag, 1. Februar 2026"`
   (the date is illustrative — pick a real Sunday from the 1900
   calendar).
5. `render_with_code(&de_de_profile, ..., 1234.5, "#,##0.00") == "1.234,50"`
   — the thousands separator is `.` and the decimal is `,` per
   the German profile.
6. `render_currency(&de_de_profile, 1234.5, 2) == "1.234,50 €"`
   (note the trailing currency symbol in the German convention —
   if the profile encodes "currency before vs. after", surface
   that as a `currency_position` field; otherwise document the
   limitation explicitly).
7. The locale-prefix test `[$-040C]d mmmm yyyy` against
   `LocaleProfileId::EnUs` produces French output (only if
   item 5 above is in scope).

## DnaOneCalc-side state in the meantime

DnaOneCalc's `AmbientAppContext` already gets the date / time
*ordering* right by handing OxFml a code like `dd.mm.yyyy
HH:mm:ss` for `de-DE`. The visible gap is purely in the data
tables — month and weekday names render in English regardless of
how the user's locale is set. Currency rendering shows `$` for
every locale.

Once OxFml lands locale-keyed tables and the host can supply a
non-en-US `LocaleFormatContext`, DnaOneCalc:

- Adds a workspace-locale dropdown UI (locked to `en-US` today
  because that's the only `LocaleFormatContext` we can build).
- Reads the locale id off the workspace and constructs the right
  `LocaleFormatContext` per bridge call.
- Removes the `SEAM-OXFUNC-LOCALE-EXPAND` marker from the
  result-foot context chip.

That cleanup is host-side and proceeds the moment OxFml's tables
are reachable.

## Closure conditions

- `LocaleProfileId` covers the locales DnaOneCalc enumerates in
  `services::ambient_app_context::ambient_app_context_for_language_tag`.
- `FormatProfile` constants exist for each new variant with
  correct separators / currency / decimals.
- `oxfml_core::format::locale_tables` exposes `month_name` and
  `weekday_name` keyed on `FormatProfile`.
- Locale-sensitive parser / general-render paths consult the
  profile rather than `LocaleProfileId::EnUs` checks.
- Host-supplied non-en-US `LocaleFormatContext` round-trips
  through `oxfml_core::publication::render_effective_display_text`
  — pinned by integration test.
- Tests above (or the OxFml-side equivalents) are green.
- Optional: locale-prefix grammar — covered separately if it
  isn't part of this slice.
