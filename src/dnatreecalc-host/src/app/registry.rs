use dnatreecalc_skin_framework::SkinRegistry;
use dnatreecalc_skins::{
    Bench, CaptureLens, ConsoleCompanion, DependencyInspector, FlowLens, FormulaTree, LedgerLens,
    LensCompanion, NotebookLens, OutlineTable, SheetLens, TransportLens, TreeLens, TripleEditor,
    ValueBoard,
};

/// Build the default skin registry.
///
/// Registration order determines both the switcher tab order and the
/// `Ctrl+1..9` lens slots. The first seven are the ATLAS suite in its
/// canonical grammar order (capture → structure → populations → edit →
/// explore → what-if → time); the walking-skeleton skins follow as legacy
/// surfaces reachable from the switcher (and `Ctrl+8`/`Ctrl+9` for the
/// first two) until the suite absorbs them.
#[must_use]
pub fn build_default_registry() -> SkinRegistry {
    let mut registry = SkinRegistry::new();
    // ATLAS lenses — Ctrl+1..7.
    registry.register(CaptureLens::new());
    registry.register(TreeLens::new());
    registry.register(LedgerLens::new());
    registry.register(SheetLens::new());
    registry.register(FlowLens::new());
    registry.register(Bench::new());
    registry.register(TransportLens::new());
    // N1: the read-only notebook skin, reachable from the switcher so the
    // browser-harness rule (§E.0) has a real route to mount and assert
    // against; the notebook's editing loop (N2+) still lands later.
    registry.register(NotebookLens::new());
    // Walking-skeleton legacy skins.
    registry.register(TripleEditor::new());
    registry.register(FormulaTree::new());
    registry.register(OutlineTable::new());
    registry.register(ValueBoard::new());
    registry.register(DependencyInspector::new());
    // Cockpit companions — companion slots only; never primary tabs.
    registry.register(LensCompanion::new());
    registry.register(ConsoleCompanion::new());
    registry
}
