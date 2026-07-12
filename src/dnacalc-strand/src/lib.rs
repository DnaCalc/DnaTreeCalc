//! Strand — the DNA Calc design-token crate.
//!
//! Framework-free (no `leptos`, no `ox*` crate — SHELL_SPEC.md §2's P-gate
//! applies to this crate too, trivially: it has no dependencies to violate
//! it with). `serde` (de)serialization for the public data types is
//! available behind the `serde` feature and is off by default.
//!
//! Design authority: `docs/ux/redesign/STRAND_DESIGN_LANGUAGE.md`.
//! - §2 Palette / §2.1 Contrast law -> [`palette`], [`contrast`]
//! - §4 Block geometry / §5 Motion & density -> [`geometry`]
//! - §6 Theming -> [`theme`], [`css`]
//! - §2 Identity badges -> [`identity`]
//!
//! Shell placement: `docs/ux/redesign/SHELL_SPEC.md` §2 (crate
//! decomposition) places `dnacalc-strand` as the token-provider (TP) tier
//! crate that `dnacalc-shell` and `dnacalc-bridge` depend on alongside
//! `dnacalc-skin-ir` and Leptos, and §8 (Theming & density) — "Contrast
//! gates per STRAND §2.1 run in `dnacalc-strand` tests" — is what
//! [`contrast::SANCTIONED_PAIRS`] and its tests satisfy.
//!
//! ## Version note
//!
//! Strand versions like the rest of the estate's protocols: nothing here is
//! final (STRAND §7, last line). Treat hex values as the current best
//! answer, gated by the contrast law, not as frozen constants.

pub mod contrast;
pub mod css;
pub mod geometry;
pub mod identity;
pub mod palette;
pub mod theme;

pub use contrast::{SANCTIONED_PAIRS, SanctionedPair, contrast_ratio, relative_luminance};
pub use css::css_custom_properties;
pub use geometry::{
    GAP_SCALE, MOTION_DURATION_MAX_MS, MOTION_DURATION_MIN_MS, RADIUS_CARD, RADIUS_CHIP,
    RADIUS_PANEL, REDUCED_MOTION_FLAG,
};
pub use identity::{
    ALL_IDENTITIES, EXCELSTRICT_IDENTITY, IdentityBadge, IdentityBlock, ONECALC_IDENTITY,
    RICHTREE_IDENTITY,
};
pub use palette::Rgb;
pub use theme::{COCKPIT_DARK, COCKPIT_LIGHT, Density, HIGH_CONTRAST, Theme, ThemeTokens};
