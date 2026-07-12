//! SEAM-OXFML-FONT-MODEL
//! SEAM-OXFML-FONT-COLOR
//! SEAM-OXFML-BORDER-MODEL
//! SEAM-OXFML-ALIGNMENT-MODEL
//! SEAM-OXFML-PROTECTION-MODEL
//! SEAM-OXFML-FILL-COLOR
//! SEAM-OXFML-FILL-EFFECTS
//! SEAM-OXFML-STYLE-XF
//!
//! Target: cell style payload (font, fill, border, alignment, protection,
//! XF) surfaces as a typed style cluster on the Configure drawer.

use super::common::seam_pending;

#[test]
#[ignore = "pending SEAM-OXFML-FONT-MODEL"]
fn font_model_surfaces_family_size_weight_style_underline() {
    seam_pending(
        "SEAM-OXFML-FONT-MODEL",
        "font payload (family/size/weight/style/underline) must project to Configure drawer Font tab",
    );
}

#[test]
#[ignore = "pending SEAM-OXFML-FONT-COLOR"]
fn font_color_surfaces_as_typed_colour() {
    seam_pending(
        "SEAM-OXFML-FONT-COLOR",
        "font colour must project as a typed colour annotation on the Font tab",
    );
}

#[test]
#[ignore = "pending SEAM-OXFML-BORDER-MODEL"]
fn border_model_surfaces_per_side_line_and_colour() {
    seam_pending(
        "SEAM-OXFML-BORDER-MODEL",
        "border model (per-side line/colour) must project to Configure drawer Border tab",
    );
}

#[test]
#[ignore = "pending SEAM-OXFML-ALIGNMENT-MODEL"]
fn alignment_model_surfaces_horizontal_vertical_wrap_and_indent() {
    seam_pending(
        "SEAM-OXFML-ALIGNMENT-MODEL",
        "alignment model must project horizontal/vertical/wrap/indent to Configure drawer",
    );
}

#[test]
#[ignore = "pending SEAM-OXFML-PROTECTION-MODEL"]
fn protection_model_surfaces_locked_and_hidden_flags() {
    seam_pending(
        "SEAM-OXFML-PROTECTION-MODEL",
        "protection model must project locked/hidden flags to Configure drawer Protection tab",
    );
}

#[test]
#[ignore = "pending SEAM-OXFML-FILL-COLOR"]
fn fill_color_surfaces_foreground_and_background() {
    seam_pending(
        "SEAM-OXFML-FILL-COLOR",
        "fill colour must project foreground/background to Configure drawer Fill tab",
    );
}

#[test]
#[ignore = "pending SEAM-OXFML-FILL-EFFECTS"]
fn fill_effects_surface_gradients_and_patterns() {
    seam_pending(
        "SEAM-OXFML-FILL-EFFECTS",
        "fill effects (gradients/patterns) must project to Configure drawer Fill tab",
    );
}

#[test]
#[ignore = "pending SEAM-OXFML-STYLE-XF"]
fn style_xf_hierarchy_resolves_to_cluster_payload() {
    seam_pending(
        "SEAM-OXFML-STYLE-XF",
        "XF style hierarchy must resolve through the projection to a typed cluster payload",
    );
}
