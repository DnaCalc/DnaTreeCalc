use dnatreecalc_skin_framework::SkinRegistry;
use dnatreecalc_skins::{OutlineTable, TripleEditor};

/// Build the default skin registry shipped by the walking skeleton.
///
/// Registration order determines the order skins appear in the shell's
/// switcher chrome. The `triple-editor` skin is registered first because
/// it is the default editing surface; `outline-table` is the second
/// category (Overview) used to prove runtime switching.
#[must_use]
pub fn build_default_registry() -> SkinRegistry {
    let mut registry = SkinRegistry::new();
    registry.register(TripleEditor::new());
    registry.register(OutlineTable::new());
    registry
}
