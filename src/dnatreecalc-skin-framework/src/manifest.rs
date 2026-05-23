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
}
