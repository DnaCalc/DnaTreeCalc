use crate::NodeKey;

/// ARIA attributes for a container whose active child follows host selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AriaAttrs {
    pub role: &'static str,
    pub aria_label: String,
    pub aria_activedescendant: Option<String>,
}

#[must_use]
pub fn tree_a11y(
    label: impl Into<String>,
    id_prefix: &str,
    active_node: Option<&NodeKey>,
) -> AriaAttrs {
    AriaAttrs {
        role: "tree",
        aria_label: label.into(),
        aria_activedescendant: active_node.map(|key| stable_node_dom_id(id_prefix, key)),
    }
}

#[must_use]
pub fn listbox_a11y(
    label: impl Into<String>,
    id_prefix: &str,
    active_node: Option<&NodeKey>,
) -> AriaAttrs {
    AriaAttrs {
        role: "listbox",
        aria_label: label.into(),
        aria_activedescendant: active_node.map(|key| stable_node_dom_id(id_prefix, key)),
    }
}

#[must_use]
pub fn table_a11y(
    label: impl Into<String>,
    id_prefix: &str,
    active_node: Option<&NodeKey>,
) -> AriaAttrs {
    AriaAttrs {
        role: "table",
        aria_label: label.into(),
        aria_activedescendant: active_node.map(|key| stable_node_dom_id(id_prefix, key)),
    }
}

#[must_use]
pub fn stable_node_dom_id(prefix: &str, node_key: &NodeKey) -> String {
    let mut id = String::with_capacity(prefix.len() + node_key.as_str().len() + 1);
    id.push_str(prefix);
    id.push('-');
    for ch in node_key.as_str().chars() {
        if ch.is_ascii_alphanumeric() {
            id.push(ch.to_ascii_lowercase());
        } else {
            id.push('-');
        }
    }
    id
}

/// ARIA attributes for a selectable tree or list item.
///
/// The helper keeps accessible selection and roving-tabindex policy in the Skin
/// IR layer. Skins still choose their markup, but they do not invent different
/// semantics for the same host selection state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectableItemA11y {
    pub id: String,
    pub role: &'static str,
    pub aria_selected: &'static str,
    pub aria_level: Option<String>,
    pub aria_posinset: Option<String>,
    pub aria_setsize: Option<String>,
    pub tabindex: &'static str,
}

impl SelectableItemA11y {
    #[must_use]
    pub fn for_tree_item(
        id_prefix: &str,
        node_key: &NodeKey,
        is_selected: bool,
        is_focusable: bool,
        level: u32,
        posinset: usize,
        setsize: usize,
    ) -> Self {
        Self {
            id: stable_node_dom_id(id_prefix, node_key),
            role: "treeitem",
            aria_selected: aria_bool(is_selected),
            aria_level: Some(level.max(1).to_string()),
            aria_posinset: Some(posinset.max(1).to_string()),
            aria_setsize: Some(setsize.max(1).to_string()),
            tabindex: roving_tabindex(is_focusable),
        }
    }

    #[must_use]
    pub fn for_option(
        id_prefix: &str,
        node_key: &NodeKey,
        is_selected: bool,
        is_focusable: bool,
    ) -> Self {
        Self {
            id: stable_node_dom_id(id_prefix, node_key),
            role: "option",
            aria_selected: aria_bool(is_selected),
            aria_level: None,
            aria_posinset: None,
            aria_setsize: None,
            tabindex: roving_tabindex(is_focusable),
        }
    }
}

/// ARIA attributes for a selectable table row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectableRowA11y {
    pub id: String,
    pub aria_selected: &'static str,
    pub tabindex: &'static str,
}

impl SelectableRowA11y {
    #[must_use]
    pub fn new(id_prefix: &str, node_key: &NodeKey, is_selected: bool, is_focusable: bool) -> Self {
        Self {
            id: stable_node_dom_id(id_prefix, node_key),
            aria_selected: aria_bool(is_selected),
            tabindex: roving_tabindex(is_focusable),
        }
    }
}

#[must_use]
pub fn roving_tabindex(is_selected: bool) -> &'static str {
    if is_selected { "0" } else { "-1" }
}

#[must_use]
pub fn aria_bool(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}
