//! SEAM-OXFML-RICH-VALUE-PUBLICATION
//!
//! Target: rich value payloads on `EditorDocument` surface as typed
//! entries in the Inspect value drill-down. Today only the flattened
//! `fallback` text is exposed.

use super::common::seam_pending;

#[test]
#[ignore = "pending SEAM-OXFML-RICH-VALUE-PUBLICATION"]
fn rich_value_payload_surfaces_as_typed_drill_down() {
    seam_pending(
        "SEAM-OXFML-RICH-VALUE-PUBLICATION",
        "RichValue (kvps / key_flags / nested arrays) must project to Inspect value drill-down",
    );
}
