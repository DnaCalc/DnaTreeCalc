# DNA TreeCalc

The multi-node, tree-substrate calculation host of the DNA Calc family — a model is a tree of named formulas, Excel-faithful but freed from the grid. Built on the OxCalc engine, the way DNA OneCalc is built on OxFml.

Developed by DNA Kode, as part of the DNA Calc program.

## Orientation

- [`CHARTER.md`](CHARTER.md) — mission, north star, place in the DNA Calc program, position among sibling repos.
- [`OPERATIONS.md`](OPERATIONS.md) — how we work: the Spec, worksets, beads, handovers, issues.
- [`AGENTS.md`](AGENTS.md) — agent operating instructions.
- [`docs/SPEC.md`](docs/SPEC.md) — the spec/design document set (model, interop, ux + prototypes).

## Where things are

| Path | What |
|---|---|
| `docs/SPEC.md` | Spec index — start here for the design. |
| `docs/model/` | Calculation & language model. |
| `docs/interop/` | Excel import/export & replay verification. |
| `docs/ux/` | UX requirements, technical plan, skin architecture, prototypes. |
| `docs/WORKSET_REGISTER.md` | Roadmap of large planned work areas (W###); `br` owns live execution state. |
| `docs/handovers/` | Cross-repo coordination docs. |
| `.beads/` | Execution-truth bead store (managed via `br`). |

## Status

Design/planning stage. The implementation tree (Rust workspace) is not yet scaffolded. Bead store initialization (`br init`) is still pending — see `docs/WORKSET_REGISTER.md` workset `W001`.

## License

MIT.
