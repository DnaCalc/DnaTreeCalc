//! Profile and runtime tags the shell composes against.
//!
//! [`ProfileTag`] is the product-profile axis (SHELL_SPEC.md §3: "profiles
//! gate capability" — a [`crate::stage::StageSurface`] declares which
//! profiles it supports). It deliberately mirrors the three Strand identity
//! badges (STRAND §2) so the mast badge and the capability gate can never
//! name different profiles.
//!
//! [`RuntimeContext`] is the target axis (browser WASM vs desktop), derived
//! from the IR's [`RuntimeProfileProjection`]. The keyboard registry uses it
//! for the browser hard-reserved law and the desktop-only chord additions
//! (SHELL_SPEC §5.1).

use dnacalc_skin_ir::RuntimeProfileProjection;
use dnacalc_strand::{EXCELSTRICT_IDENTITY, IdentityBadge, ONECALC_IDENTITY, RICHTREE_IDENTITY};

/// The product profile a shell instance runs under.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProfileTag {
    /// The Rich Tree profile (Calc app, tree-native surfaces).
    RichTree,
    /// The strict Excel-grid profile (Calc app, grid surfaces).
    ExcelStrict,
    /// The Bench profile (one-formula instrument; its own window).
    OneCalc,
}

impl ProfileTag {
    pub const ALL: [ProfileTag; 3] = [
        ProfileTag::RichTree,
        ProfileTag::ExcelStrict,
        ProfileTag::OneCalc,
    ];

    /// Stable id — identical to the Strand identity badge `profile_id`, so
    /// the badge and the gate share one vocabulary (asserted in tests).
    #[must_use]
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::RichTree => "richtree",
            Self::ExcelStrict => "excelstrict",
            Self::OneCalc => "onecalc",
        }
    }

    /// This profile's Strand base-pair identity badge (STRAND §2), rendered
    /// in the mast identity spot.
    #[must_use]
    pub const fn identity_badge(self) -> IdentityBadge {
        match self {
            Self::RichTree => RICHTREE_IDENTITY,
            Self::ExcelStrict => EXCELSTRICT_IDENTITY,
            Self::OneCalc => ONECALC_IDENTITY,
        }
    }
}

/// Browser vs desktop runtime, as the keyboard grammar cares about it.
///
/// Browser runtimes are governed by the hard-reserved chord law
/// (SHELL_SPEC §5.1: chords the browser owns are never bound and never
/// intercepted); desktop runtimes may additionally bind the two
/// spec-sanctioned desktop chord families.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuntimeContext {
    Browser,
    Desktop,
}

impl RuntimeContext {
    /// Fold the IR runtime-profile projection onto the two-value axis the
    /// keyboard grammar needs. Anything that runs inside a browser tab
    /// (including hosted web) is `Browser`; every other host — including the
    /// headless/test profiles, which have no browser to reserve chords for —
    /// is `Desktop`.
    #[must_use]
    pub const fn from_runtime_profile(profile: RuntimeProfileProjection) -> Self {
        match profile {
            RuntimeProfileProjection::BrowserWasm | RuntimeProfileProjection::HostedWeb => {
                Self::Browser
            }
            RuntimeProfileProjection::WindowsDesktop
            | RuntimeProfileProjection::WindowsHeadless
            | RuntimeProfileProjection::NativeUnix
            | RuntimeProfileProjection::NullTest => Self::Desktop,
        }
    }

    #[must_use]
    pub const fn is_browser(self) -> bool {
        matches!(self, Self::Browser)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_ids_match_strand_identity_badges() {
        for profile in ProfileTag::ALL {
            assert_eq!(
                profile.stable_id(),
                profile.identity_badge().profile_id,
                "ProfileTag and Strand badge must share one profile vocabulary"
            );
        }
    }

    #[test]
    fn browser_profiles_fold_to_browser_everything_else_to_desktop() {
        use RuntimeProfileProjection as P;
        assert!(RuntimeContext::from_runtime_profile(P::BrowserWasm).is_browser());
        assert!(RuntimeContext::from_runtime_profile(P::HostedWeb).is_browser());
        for desktop in [
            P::WindowsDesktop,
            P::WindowsHeadless,
            P::NativeUnix,
            P::NullTest,
        ] {
            assert!(!RuntimeContext::from_runtime_profile(desktop).is_browser());
        }
    }
}
