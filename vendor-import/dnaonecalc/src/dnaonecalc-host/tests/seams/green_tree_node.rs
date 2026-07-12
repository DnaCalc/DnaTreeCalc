//! SEAM-OXFML-GREEN-TREE-NODE-ID
//!
//! Target: every formula walk node carries a stable `green_tree_node_id`
//! so the Inspect Value Panel can display node identity and the
//! "send selection to Inspect" bridge can land on the correct node.

use super::common::seam_pending;

#[test]
#[ignore = "pending SEAM-OXFML-GREEN-TREE-NODE-ID"]
fn inspect_walk_nodes_carry_stable_green_tree_node_id() {
    seam_pending(
        "SEAM-OXFML-GREEN-TREE-NODE-ID",
        "InspectFormulaDrillNodeViewModelView must carry a stable green_tree_node_id",
    );
}
