//! Platform-derived initial values for [`AmbientAppContext`].
//!
//! Excel sources its date / time defaults from Windows regional
//! settings. DnaOneCalc cannot read those directly from the browser
//! sandbox, so the best-effort substitute is the BCP-47 language tag
//! the host platform reports — `navigator.language` on wasm,
//! deferred to ISO defaults on non-wasm SSR builds.
//!
//! The mapping is deliberately a small hand-curated table rather
//! than a full ICU integration: it covers the locale shapes that
//! materially differ in their canonical short-date / short-datetime
//! orderings, and falls back to ISO for anything not listed. Users
//! whose locale isn't in the table — or whose Windows regional
//! settings differ from their browser language — can override the
//! result through the workspace preferences UI (a SEAM-pending
//! follow-up; the state slot itself is already in place).

use crate::state::AmbientAppContext;

/// Build an `AmbientAppContext` with platform-derived defaults.
/// On wasm, reads `navigator.language` and consults the locale
/// table below. On non-wasm, returns the ISO-default fallback.
pub fn detect_ambient_app_context_for_platform() -> AmbientAppContext {
    let language_tag = platform_language_tag();
    ambient_app_context_for_language_tag(language_tag.as_deref())
}

/// Pure mapping from a BCP-47 language tag to an
/// [`AmbientAppContext`]. Exposed so tests can pin the mapping
/// without wasm-only platform calls.
pub fn ambient_app_context_for_language_tag(tag: Option<&str>) -> AmbientAppContext {
    let Some(tag) = tag else {
        return AmbientAppContext::default();
    };
    let language_tag = canonical_language_tag(tag);
    let normalised = tag.trim().to_ascii_lowercase();
    let language_part = normalised
        .split(['-', '_'])
        .next()
        .unwrap_or("")
        .to_string();
    let region_part = normalised
        .split(['-', '_'])
        .nth(1)
        .map(str::to_owned)
        .unwrap_or_default();

    // Ordering rule with examples for the user-visible region:
    // - dd/mm/yyyy: en-GB, en-IE, en-AU, en-NZ, en-ZA, fr-*, es-*,
    //   pt-PT, it-*, nl-NL, pl-PL, …
    // - yyyy/MM/dd: zh-*, ja-JP, ko-KR, hu-HU
    // - dd.MM.yyyy: de-*, ru-*, fi-FI, et-EE, lv-LV, lt-LT, sk-SK,
    //   cs-CZ, nb-NO, sv-SE (Sweden uses '-' but stays day-first
    //   for the spoken form; we keep ISO `yyyy-mm-dd` for sv to
    //   avoid getting it subtly wrong)
    // - mm/dd/yyyy: en-US, en-CA, en-PH
    let (date_code, datetime_code, time_code) = match (language_part.as_str(), region_part.as_str())
    {
        // East Asian: yyyy/mm/dd, 24-hour. Matches the screenshot
        // the user showed (Excel `=NOW()` rendered as
        // `2026/05/04 13:55`).
        ("zh", _) | ("ja", _) | ("ko", _) | ("hu", _) => {
            ("yyyy/mm/dd", "yyyy/mm/dd HH:mm:ss", "HH:mm:ss")
        }
        // German / Russian / Eastern European: dd.MM.yyyy, 24-hour.
        ("de", _)
        | ("ru", _)
        | ("fi", _)
        | ("et", _)
        | ("lv", _)
        | ("lt", _)
        | ("sk", _)
        | ("cs", _)
        | ("nb", _)
        | ("nn", _) => ("dd.mm.yyyy", "dd.mm.yyyy HH:mm:ss", "HH:mm:ss"),
        // British / Commonwealth English + most romance languages:
        // dd/mm/yyyy, 24-hour.
        ("en", "gb")
        | ("en", "ie")
        | ("en", "au")
        | ("en", "nz")
        | ("en", "za")
        | ("en", "in")
        | ("fr", _)
        | ("es", _)
        | ("pt", "pt")
        | ("it", _)
        | ("nl", _)
        | ("pl", _) => ("dd/mm/yyyy", "dd/mm/yyyy HH:mm:ss", "HH:mm:ss"),
        // US English / Canadian English / Filipino English:
        // m/d/yyyy with 12-hour clock for the time portion.
        ("en", "us") | ("en", "ca") | ("en", "ph") | ("en", "") => {
            ("m/d/yyyy", "m/d/yyyy h:mm:ss AM/PM", "h:mm:ss AM/PM")
        }
        // Brazilian Portuguese: dd/MM/yyyy.
        ("pt", "br") => ("dd/mm/yyyy", "dd/mm/yyyy HH:mm:ss", "HH:mm:ss"),
        // Anything we don't recognise stays on the ISO defaults.
        _ => return AmbientAppContext::default(),
    };
    AmbientAppContext {
        language_tag,
        date_format_code: date_code.to_string(),
        datetime_format_code: datetime_code.to_string(),
        time_format_code: time_code.to_string(),
    }
}

/// Normalise a user-supplied language tag into the canonical
/// `xx-YY` shape. ASCII-only, no validation — the locale-preset
/// dropdown is the curated source for valid tags, this helper
/// just makes a fallthrough preset round-trip cleanly.
fn canonical_language_tag(tag: &str) -> String {
    let trimmed = tag.trim();
    if let Some((language, region)) = trimmed.split_once(['-', '_']) {
        format!(
            "{}-{}",
            language.to_ascii_lowercase(),
            region.to_ascii_uppercase()
        )
    } else {
        trimmed.to_ascii_lowercase()
    }
}

/// Curated list of locale presets surfaced in the formatting-panel
/// dropdown. Each preset's `language_tag` round-trips through
/// `ambient_app_context_for_language_tag` to a known
/// `AmbientAppContext`. Order is roughly Excel-marketshare-by-region.
pub fn supported_locale_presets() -> &'static [(&'static str, &'static str)] {
    &[
        ("en-US", "English (United States)"),
        ("en-GB", "English (United Kingdom)"),
        ("de-DE", "German (Germany)"),
        ("fr-FR", "French (France)"),
        ("es-ES", "Spanish (Spain)"),
        ("it-IT", "Italian (Italy)"),
        ("nl-NL", "Dutch (Netherlands)"),
        ("pt-BR", "Portuguese (Brazil)"),
        ("pt-PT", "Portuguese (Portugal)"),
        ("pl-PL", "Polish (Poland)"),
        ("ru-RU", "Russian (Russia)"),
        ("ja-JP", "Japanese (Japan)"),
        ("ko-KR", "Korean (Korea)"),
        ("zh-CN", "Chinese (Mainland)"),
    ]
}

#[cfg(target_arch = "wasm32")]
fn platform_language_tag() -> Option<String> {
    web_sys::window().and_then(|window| window.navigator().language())
}

#[cfg(not(target_arch = "wasm32"))]
fn platform_language_tag() -> Option<String> {
    // Non-wasm SSR builds don't have a browser navigator; ISO
    // defaults are the safe fallback. (A future enhancement could
    // read `LANG` / `LC_TIME` from std::env, but that brings its
    // own portability complications and is rarely meaningful in
    // the SSR-pre-render context the host crate is built for off-
    // wasm.)
    None
}

/// Pick the nearest `LocaleProfileId`-shaped string for a given
/// BCP-47 language tag, drawn from OxFunc W094's first slice
/// (en-US, en-GB, de-DE, fr-FR, es-ES, it-IT, nl-NL, pt-BR,
/// ja-JP, ko-KR, zh-CN, current-excel-host).
///
/// This is the host-side answer to the open lane in
/// `OxFunc/docs/handoffs/HANDOFF-OXFUNC-006_W070_LOCALE_PROFILE_EXPANSION_REQUEST.md`:
/// for tags outside the first slice, which profile do we fall
/// back to? The mapping is hand-curated, explicit, and Excel-
/// faithful where possible — `en-AU` → `en-GB`, `pt-PT` →
/// `pt-BR` (closest available; not perfect, the tags differ in
/// thousands separator) — falling through to
/// `current-excel-host` when no good match exists.
///
/// Returns the canonical profile-id string. The host does *not*
/// yet construct an actual `LocaleFormatContext` from this —
/// that happens once OxFml lands the locale-keyed tables. The
/// mapping function is in place now so the wire-through is a
/// single slice when the chain unblocks.
pub fn nearest_locale_profile_for_language_tag(tag: Option<&str>) -> &'static str {
    let Some(tag) = tag else {
        return "current-excel-host";
    };
    let normalised = tag.trim().to_ascii_lowercase();
    let language = normalised.split(['-', '_']).next().unwrap_or("");
    let region = normalised.split(['-', '_']).nth(1).unwrap_or("");
    match (language, region) {
        // First-slice exact matches.
        ("en", "us") | ("en", "ca") | ("en", "ph") | ("en", "") => "en-US",
        ("en", "gb") | ("en", "ie") | ("en", "au") | ("en", "nz") | ("en", "za") | ("en", "in") => {
            "en-GB"
        }
        ("de", _) => "de-DE",
        ("fr", _) => "fr-FR",
        ("es", _) => "es-ES",
        ("it", _) => "it-IT",
        ("nl", _) => "nl-NL",
        ("pt", "br") | ("pt", "") => "pt-BR",
        ("pt", _) => "pt-BR", // pt-PT nearest match; separators differ
        ("ja", _) => "ja-JP",
        ("ko", _) => "ko-KR",
        ("zh", _) => "zh-CN",
        // Long-tail locales without a W094 first-slice match.
        // `current-excel-host` defers to the host's regional
        // settings — the right fallback when DnaOneCalc has no
        // closer answer.
        ("ru", _)
        | ("fi", _)
        | ("et", _)
        | ("lv", _)
        | ("lt", _)
        | ("sk", _)
        | ("cs", _)
        | ("nb", _)
        | ("nn", _)
        | ("pl", _)
        | ("hu", _) => "current-excel-host",
        _ => "current-excel-host",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_or_empty_language_tag_falls_back_to_iso() {
        assert_eq!(
            ambient_app_context_for_language_tag(None),
            AmbientAppContext::default()
        );
        assert_eq!(
            ambient_app_context_for_language_tag(Some("xx-YY")),
            AmbientAppContext::default()
        );
    }

    #[test]
    fn east_asian_locales_pick_yyyy_first_format() {
        let ctx = ambient_app_context_for_language_tag(Some("ja-JP"));
        assert_eq!(ctx.date_format_code, "yyyy/mm/dd");
        assert_eq!(ctx.datetime_format_code, "yyyy/mm/dd HH:mm:ss");
    }

    #[test]
    fn german_locale_picks_dot_separated_day_first() {
        let ctx = ambient_app_context_for_language_tag(Some("de-DE"));
        assert_eq!(ctx.date_format_code, "dd.mm.yyyy");
    }

    #[test]
    fn british_english_picks_slash_separated_day_first() {
        let ctx = ambient_app_context_for_language_tag(Some("en-GB"));
        assert_eq!(ctx.date_format_code, "dd/mm/yyyy");
    }

    #[test]
    fn us_english_picks_month_first_with_twelve_hour_clock() {
        let ctx = ambient_app_context_for_language_tag(Some("en-US"));
        assert_eq!(ctx.date_format_code, "m/d/yyyy");
        assert_eq!(ctx.datetime_format_code, "m/d/yyyy h:mm:ss AM/PM");
        assert_eq!(ctx.time_format_code, "h:mm:ss AM/PM");
    }

    #[test]
    fn underscore_separator_in_tag_is_normalised() {
        let dotted = ambient_app_context_for_language_tag(Some("de_DE"));
        let dashed = ambient_app_context_for_language_tag(Some("de-DE"));
        assert_eq!(dotted, dashed);
    }

    #[test]
    fn nearest_profile_for_first_slice_languages_returns_exact_match() {
        assert_eq!(
            nearest_locale_profile_for_language_tag(Some("en-US")),
            "en-US"
        );
        assert_eq!(
            nearest_locale_profile_for_language_tag(Some("de-DE")),
            "de-DE"
        );
        assert_eq!(
            nearest_locale_profile_for_language_tag(Some("ja-JP")),
            "ja-JP"
        );
        assert_eq!(
            nearest_locale_profile_for_language_tag(Some("zh-CN")),
            "zh-CN"
        );
    }

    #[test]
    fn nearest_profile_for_commonwealth_english_maps_to_en_gb() {
        for tag in ["en-GB", "en-IE", "en-AU", "en-NZ", "en-ZA", "en-IN"] {
            assert_eq!(
                nearest_locale_profile_for_language_tag(Some(tag)),
                "en-GB",
                "{tag} should map to en-GB",
            );
        }
    }

    #[test]
    fn nearest_profile_for_canadian_or_filipino_english_maps_to_en_us() {
        for tag in ["en-CA", "en-PH", "en"] {
            assert_eq!(
                nearest_locale_profile_for_language_tag(Some(tag)),
                "en-US",
                "{tag} should map to en-US",
            );
        }
    }

    #[test]
    fn nearest_profile_for_long_tail_falls_back_to_current_excel_host() {
        for tag in ["ru-RU", "fi-FI", "et-EE", "pl-PL", "hu-HU", "xx-YY"] {
            assert_eq!(
                nearest_locale_profile_for_language_tag(Some(tag)),
                "current-excel-host",
                "{tag} should fall back to current-excel-host",
            );
        }
    }

    #[test]
    fn nearest_profile_for_no_tag_falls_back_to_current_excel_host() {
        assert_eq!(
            nearest_locale_profile_for_language_tag(None),
            "current-excel-host"
        );
    }
}
