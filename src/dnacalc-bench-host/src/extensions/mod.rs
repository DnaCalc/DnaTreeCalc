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
