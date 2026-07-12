# DnaOneCalc

`DnaOneCalc` is the single-formula proving host in the DNA Calc program.

`DNA Calc` is the broader family of repos that together build a serious,
evidence-driven calculation stack: formula semantics in `OxFml`, function
semantics in `OxFunc`, replay and diff infrastructure in `OxReplay`,
Excel-observation capture in `OxXlPlay`, and host products such as
`DnaOneCalc` that exercise those lanes together. `DnaOneCalc` is the
user-facing single-formula host in that larger system, not a standalone or
independent calculator project.

It is a serious user-facing application and a co-development surface for `OxFml`, `OxFunc`, `OxReplay`, `OxXlPlay`, and later `OxVba`. Its central product expression is the `Twin Oracle Workbench` with `Live Formula Semantic X-Ray`: author a scenario, run it, inspect semantic and replay artifacts, compare against Excel where available, retain evidence, and emit upstream-ready handoffs.

This repo is intentionally slim at the top level.

Read in this order:
1. [docs/CHARTER.md](docs/CHARTER.md)
2. [docs/OPERATIONS.md](docs/OPERATIONS.md)
3. [docs/SCOPE_AND_SPEC.md](docs/SCOPE_AND_SPEC.md)
4. [docs/WORKSET_REGISTER.md](docs/WORKSET_REGISTER.md)
5. [docs/BEADS.md](docs/BEADS.md)

Repo layout:
- [AGENTS.md](AGENTS.md) defines agent execution rules.
- [docs/CHARTER.md](docs/CHARTER.md) defines mission, ownership, and repo boundary.
- [docs/OPERATIONS.md](docs/OPERATIONS.md) defines operational doctrine.
- [docs/SCOPE_AND_SPEC.md](docs/SCOPE_AND_SPEC.md) is the main engineering spec.
- [docs/WORKSET_REGISTER.md](docs/WORKSET_REGISTER.md) is the living ordered workset register.
- [docs/BEADS.md](docs/BEADS.md) is the complete local beads working method.
- `.beads/` is the execution truth surface for epics and beads.

Verification entrypoints:
- `powershell -File .\scripts\run-host-acceptance-fast.ps1` runs the dev-velocity smoke family.
- `powershell -File .\scripts\run-host-integration.ps1` runs the deeper retained/document/workspace integration family.
- `powershell -File .\scripts\run-host-acceptance-full.ps1` runs the full `dnaonecalc-host` acceptance suite.

Cross-repo rule:
- This repo may read sibling repos under `C:\Work\DnaCalc` for upstream contracts, evidence, and alignment.
- This repo must not write to sibling repos. Any required change outside `DnaOneCalc` must be routed through a handoff, prompt, or separate repo-scoped run.
