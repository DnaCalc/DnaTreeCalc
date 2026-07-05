/// Display-time identity for a registered skin.
///
/// All fields are `&'static str` because the walking skeleton ships only
/// built-in skins; user-authored skins (deferred per `docs/ux/SKINS.md` §8)
/// would need a heap-owned variant when that lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkinManifest {
    pub display_name: &'static str,
    pub description: &'static str,
    pub category: SkinCategory,
    pub version: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkinCategory {
    /// Primary editing surface (`TripleEditor`, `CellView`, `NodesAcross`, `Canvas`).
    Editor,
    /// Panoramic view (`OutlineTable`).
    Overview,
    /// Specialty deep-dive (`FormatEditor`, `TemplateEditor`, dependency map).
    Inspector,
    /// Read-only viewer for sharing.
    Presentation,
}

/// What capabilities a skin declares.
///
/// The walking skeleton populates only the few flags it exercises; the
/// fuller capability set (data bars, icon sets, drill panel, canvas
/// positioning, conditional formatting) arrives with W006/W007 as the
/// corresponding renderers land. Skins should default unset capabilities
/// to `false` rather than lying about coverage.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SkinCapabilities {
    pub supports_multi_select: bool,
    pub supports_inline_formula_edit: bool,
    pub supports_meta_node_display: bool,
    pub renders_arrays_inline: bool,
    pub renders_table_values: bool,
    /// Slots this skin may mount in. `None` (the default) means the
    /// **main-class** slots: the primary Main pane and the side-by-side
    /// `SplitLeft`/`SplitRight` panes, which are just additional main-class
    /// surfaces for running two lenses at once. Existing primary lenses need
    /// no change to appear in a split. Companions declare their companion
    /// slots explicitly. Checked by the shell's mount negotiation.
    pub allowed_slots: Option<&'static [crate::identity::SkinMountSlot]>,
}

impl SkinCapabilities {
    /// Whether this skin may mount in `slot` (the negotiation predicate).
    ///
    /// A `None` slot set is main-class: Main plus the split panes. Splitting
    /// the cockpit is "the same lens, a second copy" — so a primary lens is
    /// mountable side-by-side without re-declaring capabilities, while a
    /// companion (which lists `RightInspector`/`BottomConsole` explicitly)
    /// stays out of the main-class panes.
    #[must_use]
    pub fn slot_allowed(&self, slot: crate::identity::SkinMountSlot) -> bool {
        use crate::identity::SkinMountSlot as Slot;
        match self.allowed_slots {
            None => matches!(slot, Slot::Main | Slot::SplitLeft | Slot::SplitRight),
            Some(slots) => slots.contains(&slot),
        }
    }
}

/// Why a mount negotiation refused. Mismatches fail loudly at mount time
/// instead of rendering wrongly.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CapabilityError {
    #[error("skin {skin_id} does not support slot {slot}")]
    SlotNotSupported {
        skin_id: String,
        slot: crate::identity::SkinMountSlot,
    },
}
