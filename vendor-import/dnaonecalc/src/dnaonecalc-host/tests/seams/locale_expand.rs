//! SEAM-OXFML-LOCALE-EXPAND — LANDED
//!
//! OxFunc W094 ships `LocaleProfileId` with 30 canonical entries
//! (`CANONICAL_LOCALE_PROFILE_IDS`) and `format_profile()` returns a
//! complete `FormatProfile` for each. OxFml's
//! `oxfml_locale_context(profile, date_system)` exposes a runtime
//! `LocaleFormatContext`, and the host's
//! `live_bridge::build_runtime_locale_context` resolves the
//! workspace's BCP-47 tag through `from_bcp47_language_tag` and
//! plumbs the matching context into every bridge round-trip.
//!
//! The pin below is now an actual assertion against the upstream
//! count rather than a `seam_pending` marker.
//!
//! See `docs/HANDOFF_OXFML_LOCALE_EXPANSION.md` for the original
//! handoff and the topic-state doc for the current shape.

use oxfunc_core::locale_format::{format_profile, LocaleProfileId, CANONICAL_LOCALE_PROFILE_IDS};

#[test]
fn capability_snapshot_enumerates_at_least_three_locales() {
    // Today: 30 canonical profiles, well past the original "≥3 beyond
    // EnUs" floor. Pin the floor so a regression that drops profiles
    // is caught upstream-side too.
    assert!(
        CANONICAL_LOCALE_PROFILE_IDS.len() >= 4,
        "expected at least EnUs + 3 additional locales; got {}",
        CANONICAL_LOCALE_PROFILE_IDS.len()
    );
    assert!(CANONICAL_LOCALE_PROFILE_IDS.contains(&LocaleProfileId::EnUs));
    // Spot-check a representative sample of the European + Asian
    // locales the original handoff called for.
    for required in [
        LocaleProfileId::DeDe,
        LocaleProfileId::FrFr,
        LocaleProfileId::EsEs,
        LocaleProfileId::ItIt,
        LocaleProfileId::NlNl,
        LocaleProfileId::PtBr,
        LocaleProfileId::JaJp,
        LocaleProfileId::ZhCn,
        LocaleProfileId::KoKr,
        LocaleProfileId::RuRu,
    ] {
        assert!(
            CANONICAL_LOCALE_PROFILE_IDS.contains(&required),
            "canonical locale profile list missing {:?}",
            required
        );
    }
}

#[test]
fn from_bcp47_language_tag_resolves_canonical_locales() {
    // Smoke-test the BCP-47 resolver path the host depends on. An
    // unmapped tag falls back to `None` (caller decides) and a known
    // tag resolves to the matching profile id.
    assert_eq!(
        LocaleProfileId::from_bcp47_language_tag("en-US"),
        Some(LocaleProfileId::EnUs)
    );
    assert_eq!(
        LocaleProfileId::from_bcp47_language_tag("de-DE"),
        Some(LocaleProfileId::DeDe)
    );
    assert!(LocaleProfileId::from_bcp47_language_tag("xx-YY").is_none());
}

#[test]
fn format_profile_returns_distinct_separators_per_locale() {
    // The whole point of unblocking the locale chain: separators
    // really do differ between en-US and de-DE.
    let en = format_profile(LocaleProfileId::EnUs);
    let de = format_profile(LocaleProfileId::DeDe);
    assert_ne!(
        en.decimal_separator, de.decimal_separator,
        "en-US and de-DE must report different decimal separators"
    );
}
