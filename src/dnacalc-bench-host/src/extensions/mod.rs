//! OneCalc attachment policy for the shared DNA Calc extension host core.
//!
//! Native XLL, COM RTD, and OxVba adapters are deliberately not implemented
//! here. Their platform boundaries live in the TreeCalc-owned adapter design.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedExtensionAttachment {
    pub profile: dnacalc_extension_host_core::RuntimeProfile,
    pub capabilities: dnacalc_extension_host_core::ExtensionCapabilities,
}

impl SharedExtensionAttachment {
    #[must_use]
    pub fn current() -> Self {
        let profile = current_shared_runtime_profile();
        Self {
            capabilities: profile.capabilities(),
            profile,
        }
    }
}

#[must_use]
pub const fn current_shared_runtime_profile() -> dnacalc_extension_host_core::RuntimeProfile {
    if cfg!(target_arch = "wasm32") {
        dnacalc_extension_host_core::RuntimeProfile::BrowserWasm
    } else if cfg!(target_os = "windows") {
        dnacalc_extension_host_core::RuntimeProfile::WindowsDesktop
    } else {
        dnacalc_extension_host_core::RuntimeProfile::NativeUnix
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OneCalcInvalidationAction {
    RebindAndRecalculate,
    Recalculate,
}

/// Map the IR's `RuntimeProfileProjection` onto `extension-host-core`'s own
/// `RuntimeProfile` — the two enums name the same runtime classes (this
/// module's `current_shared_runtime_profile` and the IR's own inline cfg!
/// derivation in `home_shell_view_model` agree variant-for-variant), so this
/// is a plain 1:1 relabeling, never a semantic decision of its own.
#[must_use]
pub const fn to_extension_host_runtime_profile(
    profile: dnacalc_skin_ir::RuntimeProfileProjection,
) -> dnacalc_extension_host_core::RuntimeProfile {
    use dnacalc_extension_host_core::RuntimeProfile as Ext;
    use dnacalc_skin_ir::RuntimeProfileProjection as Ir;
    match profile {
        Ir::BrowserWasm => Ext::BrowserWasm,
        Ir::HostedWeb => Ext::HostedWeb,
        Ir::WindowsDesktop => Ext::WindowsDesktop,
        Ir::WindowsHeadless => Ext::WindowsHeadless,
        Ir::NativeUnix => Ext::NativeUnix,
        Ir::NullTest => Ext::NullTest,
    }
}

/// The G7-minimal-slice honest `ExtensionPlacementProjection` for a given IR
/// runtime profile (bead dtc-lfz.6): derived from `extension-host-core`'s
/// real per-runtime capability gate (the same gate `ExtensionCatalog::register`
/// enforces), never a hardcoded value. `native_providers` wins (in-process,
/// matching `HostCapabilityProjection::validate()`'s legal WindowsDesktop/
/// Headless/NativeUnix pairing); otherwise `native_companion` (HostedWeb);
/// otherwise honestly `Unavailable` (BrowserWasm/NullTest, and any profile
/// with neither capability). Every branch here maps onto an
/// `(runtime_profile, extension_placement)` pair `validate()` accepts —
/// proven by this module's tests, so the projection this feeds can never
/// fail `HostCapabilityProjection::validate()`.
#[must_use]
pub fn honest_extension_placement_for(
    profile: dnacalc_skin_ir::RuntimeProfileProjection,
) -> dnacalc_skin_ir::ExtensionPlacementProjection {
    use dnacalc_skin_ir::ExtensionPlacementProjection as Placement;
    let capabilities = to_extension_host_runtime_profile(profile).capabilities();
    if capabilities.native_providers {
        Placement::InProcess
    } else if capabilities.native_companion {
        Placement::NativeCompanion
    } else {
        Placement::Unavailable
    }
}

#[must_use]
pub const fn onecalc_invalidation_action(
    event: &dnacalc_extension_host_core::HostInvalidationEvent,
) -> OneCalcInvalidationAction {
    match event {
        dnacalc_extension_host_core::HostInvalidationEvent::FunctionCatalogChanged { .. } => {
            OneCalcInvalidationAction::RebindAndRecalculate
        }
        dnacalc_extension_host_core::HostInvalidationEvent::VolatileTick { .. }
        | dnacalc_extension_host_core::HostInvalidationEvent::RtdTopicChanged { .. } => {
            OneCalcInvalidationAction::Recalculate
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_extension_attachment_uses_current_profile() {
        let attachment = SharedExtensionAttachment::current();
        assert_eq!(attachment.profile, current_shared_runtime_profile());
        assert_eq!(attachment.capabilities, attachment.profile.capabilities());
    }

    /// bead dtc-lfz.6: every IR runtime profile maps onto a real
    /// extension-host-core profile with matching capabilities, and the
    /// resulting `(runtime_profile, extension_placement)` pair is one
    /// `HostCapabilityProjection::validate()` actually accepts — proof this
    /// projection can never be constructed in an illegal combination.
    #[test]
    fn honest_extension_placement_is_always_a_legal_pair() {
        use dnacalc_skin_ir::RuntimeProfileProjection as Ir;
        for profile in [
            Ir::BrowserWasm,
            Ir::HostedWeb,
            Ir::WindowsDesktop,
            Ir::WindowsHeadless,
            Ir::NativeUnix,
            Ir::NullTest,
        ] {
            let placement = honest_extension_placement_for(profile);
            let capabilities = dnacalc_skin_ir::HostCapabilityProjection::onecalc_null_references(
                profile, placement,
            );
            assert!(
                capabilities.validate().is_ok(),
                "{profile:?} -> {placement:?} must validate"
            );
        }
    }

    /// Browser/null-test runtimes have neither native nor companion
    /// capability (per `RuntimeProfile::capabilities()`) — the honest
    /// placement is `Unavailable`, never a fabricated `InProcess`.
    #[test]
    fn browser_and_null_test_are_honestly_unavailable() {
        use dnacalc_skin_ir::ExtensionPlacementProjection as Placement;
        use dnacalc_skin_ir::RuntimeProfileProjection as Ir;
        assert_eq!(
            honest_extension_placement_for(Ir::BrowserWasm),
            Placement::Unavailable
        );
        assert_eq!(
            honest_extension_placement_for(Ir::NullTest),
            Placement::Unavailable
        );
    }

    /// Windows desktop/headless and native Unix host native providers
    /// in-process (real gate: `ExtensionCapabilities::native_providers`).
    #[test]
    fn desktop_and_native_unix_are_honestly_in_process() {
        use dnacalc_skin_ir::ExtensionPlacementProjection as Placement;
        use dnacalc_skin_ir::RuntimeProfileProjection as Ir;
        for profile in [Ir::WindowsDesktop, Ir::WindowsHeadless, Ir::NativeUnix] {
            assert_eq!(
                honest_extension_placement_for(profile),
                Placement::InProcess
            );
        }
    }

    /// Hosted-web reaches native extensions only through a companion
    /// process (real gate: `ExtensionCapabilities::native_companion`).
    #[test]
    fn hosted_web_is_honestly_native_companion() {
        use dnacalc_skin_ir::ExtensionPlacementProjection as Placement;
        use dnacalc_skin_ir::RuntimeProfileProjection as Ir;
        assert_eq!(
            honest_extension_placement_for(Ir::HostedWeb),
            Placement::NativeCompanion
        );
    }

    #[test]
    fn catalog_changes_rebind_while_ticks_only_recalculate() {
        assert_eq!(
            onecalc_invalidation_action(
                &dnacalc_extension_host_core::HostInvalidationEvent::FunctionCatalogChanged {
                    generation: 2
                }
            ),
            OneCalcInvalidationAction::RebindAndRecalculate
        );
        assert_eq!(
            onecalc_invalidation_action(
                &dnacalc_extension_host_core::HostInvalidationEvent::VolatileTick { tick: 3 }
            ),
            OneCalcInvalidationAction::Recalculate
        );
    }
}
