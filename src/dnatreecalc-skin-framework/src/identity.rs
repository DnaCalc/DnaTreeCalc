use std::fmt;

use serde::{Deserialize, Serialize};

/// Display-path identifier for a tree node during the NodeKey transition window.
///
/// In the walking skeleton this wraps the workspace path (`Accounts.2005.Q1.Margin`)
/// emitted by [`crate::workspace::WorkspaceState`]. It remains as the cosmetic
/// path key while [`NodeKey`] carries the stable OxCalc node identity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct NodeId(String);

impl NodeId {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for NodeId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for NodeId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

/// Stable engine identity for a tree node.
///
/// This wraps OxCalc's `TreeNodeId` as a host/skin-safe string so skins can
/// preserve selection, expansion, pins, and reference correlations across
/// display-path changes without depending on OxCalc internals.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct NodeKey(String);

impl NodeKey {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    #[must_use]
    pub fn from_engine_id(value: u64) -> Self {
        Self(format!("tree-node:{value}"))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for NodeKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for NodeKey {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for NodeKey {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

/// Stable, host-wide identifier for a registered skin.
///
/// Skins must keep this constant across releases unless they ship a migration
/// plan for their persisted state (`SkinState::migrate`). Used as the lookup
/// key in [`crate::SkinRegistry`] and as the meta-namespace path component
/// (`skins.<skin_id>`) once persistence lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SkinId(&'static str);

impl SkinId {
    #[must_use]
    pub const fn new(value: &'static str) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        self.0
    }
}

impl fmt::Display for SkinId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

/// Where a skin is mounted in the shell's slot layout.
///
/// The walking skeleton uses only [`SkinMountSlot::Main`]. Split-pane and
/// inspector composition (`SKINS.md` §7) layer in later worksets without
/// changing the trait surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SkinMountSlot {
    Main,
    RightInspector,
    SplitLeft,
    SplitRight,
}
