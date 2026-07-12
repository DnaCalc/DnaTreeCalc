//! Per-product shell composition — SHELL_SPEC.md §1 (anatomy contract).
//!
//! A product composes a subset of the regions; omission is first-class:
//! an omitted region occupies no space and contributes **zero verbs** to
//! the keyboard catalog (no dead space, no orphaned chords — tested in
//! `crate::keyboard`).

use std::sync::Arc;

use leptos::prelude::AnyView;

use crate::profile::ProfileTag;
use crate::stage::StageContext;

// --- Region sizing (SHELL_SPEC §1, CSS pixels) ---

/// Mast height: fixed, never scrolls.
pub const MAST_HEIGHT_PX: u32 = 40;
/// Bridge height in the Calc composition (one-row, expandable).
pub const BRIDGE_HEIGHT_ONE_ROW_PX: u32 = 44;
/// Bridge height in the Bench composition (hero: two-row).
pub const BRIDGE_HEIGHT_TWO_ROW_PX: u32 = 88;
/// Registry rail width (collapsible to 0).
pub const REGISTRY_WIDTH_PX: u32 = 232;
/// Inspector panel width (collapsible).
pub const INSPECTOR_WIDTH_PX: u32 = 268;
/// Strip height: fixed.
pub const STRIP_HEIGHT_PX: u32 = 26;

/// Mast contents that vary per product.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MastComposition {
    /// Bench composes the mast *without* the stage switcher (Result stage
    /// fixed); Calc with it.
    pub stage_switcher: bool,
}

/// The typed slot the `dnacalc-bridge` workbench (bead dtc-tsc.7) mounts
/// into. Until a surface is provided the shell renders an honest
/// not-mounted note — never a fake editor.
pub trait BridgeSurface: Send + Sync + 'static {
    fn mount(&self, ctx: StageContext) -> AnyView;
}

/// Bridge region composition. The Bridge itself is present in every
/// product (SHELL_SPEC §1); only its height profile and mounted surface
/// vary.
#[derive(Clone)]
pub struct BridgeSlot {
    /// Bench hero form (two-row, 88px) vs Calc one-row (44px).
    pub hero_two_row: bool,
    /// The workbench surface, when one is mounted (dtc-tsc.7).
    pub surface: Option<Arc<dyn BridgeSurface>>,
}

impl BridgeSlot {
    #[must_use]
    pub const fn height_px(&self) -> u32 {
        if self.hero_two_row {
            BRIDGE_HEIGHT_TWO_ROW_PX
        } else {
            BRIDGE_HEIGHT_ONE_ROW_PX
        }
    }
}

/// Registry rail composition (present in Calc only).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RegistryComposition {}

/// Inspector composition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InspectorComposition {}

/// Strip composition (present in every product).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StripComposition {}

/// A product's composed shell (SHELL_SPEC §1). `Option` fields are the
/// first-class omissions.
#[derive(Clone)]
pub struct ShellComposition {
    /// Product mark text shown in the mast (e.g. "DNA Bench", "DNA Calc").
    pub product_mark: &'static str,
    /// The product profile — gates stage visibility and picks the mast
    /// identity badge.
    pub profile: ProfileTag,
    pub mast: MastComposition,
    pub bridge_slot: BridgeSlot,
    pub registry: Option<RegistryComposition>,
    pub inspector: Option<InspectorComposition>,
    pub strip: StripComposition,
}

impl ShellComposition {
    /// The Bench product composition: no stage switcher (Result stage
    /// fixed), hero two-row bridge, no Registry rail.
    #[must_use]
    pub fn bench() -> Self {
        Self {
            product_mark: "DNA Bench",
            profile: ProfileTag::OneCalc,
            mast: MastComposition {
                stage_switcher: false,
            },
            bridge_slot: BridgeSlot {
                hero_two_row: true,
                surface: None,
            },
            registry: None,
            inspector: Some(InspectorComposition {}),
            strip: StripComposition {},
        }
    }

    /// The Calc product composition under `profile` (RichTree or
    /// ExcelStrict): stage switcher, one-row bridge, Registry rail.
    #[must_use]
    pub fn calc(profile: ProfileTag) -> Self {
        Self {
            product_mark: "DNA Calc",
            profile,
            mast: MastComposition {
                stage_switcher: true,
            },
            bridge_slot: BridgeSlot {
                hero_two_row: false,
                surface: None,
            },
            registry: Some(RegistryComposition {}),
            inspector: Some(InspectorComposition {}),
            strip: StripComposition {},
        }
    }

    /// What this composition contributes to the keyboard catalog: the
    /// region-scoped verb families exist only where their region does
    /// (omission is first-class), and stage-switch slots exist only where
    /// the switcher does.
    #[must_use]
    pub fn catalog_composition(&self, visible_stage_count: usize) -> CatalogComposition {
        CatalogComposition {
            registry_region: self.registry.is_some(),
            inspector_region: self.inspector.is_some(),
            stage_slots: if self.mast.stage_switcher {
                visible_stage_count.min(4) as u8
            } else {
                0
            },
        }
    }
}

/// The keyboard-relevant projection of a [`ShellComposition`] — everything
/// `crate::keyboard::ShellKeyboardRegistry` needs to seed the catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CatalogComposition {
    pub registry_region: bool,
    pub inspector_region: bool,
    /// Number of stage-switch keyboard slots (0 when the switcher is
    /// omitted; capped at 4 per SHELL_SPEC §5.1 "Stage 1..4").
    pub stage_slots: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bench_omits_registry_and_stage_switcher() {
        let bench = ShellComposition::bench();
        assert!(bench.registry.is_none());
        assert!(!bench.mast.stage_switcher);
        assert!(bench.bridge_slot.hero_two_row);
        assert_eq!(bench.bridge_slot.height_px(), BRIDGE_HEIGHT_TWO_ROW_PX);
        let catalog = bench.catalog_composition(1);
        assert!(!catalog.registry_region);
        assert_eq!(catalog.stage_slots, 0, "no switcher => zero stage slots");
        assert!(catalog.inspector_region);
    }

    #[test]
    fn calc_composes_registry_switcher_and_one_row_bridge() {
        let calc = ShellComposition::calc(ProfileTag::RichTree);
        assert!(calc.registry.is_some());
        assert!(calc.mast.stage_switcher);
        assert!(!calc.bridge_slot.hero_two_row);
        assert_eq!(calc.bridge_slot.height_px(), BRIDGE_HEIGHT_ONE_ROW_PX);
        let catalog = calc.catalog_composition(3);
        assert!(catalog.registry_region);
        assert_eq!(catalog.stage_slots, 3);
    }

    #[test]
    fn stage_slots_cap_at_four() {
        let calc = ShellComposition::calc(ProfileTag::RichTree);
        assert_eq!(calc.catalog_composition(9).stage_slots, 4);
    }
}
