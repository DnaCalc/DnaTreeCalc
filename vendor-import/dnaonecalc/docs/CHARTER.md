# DnaOneCalc Charter

`DnaOneCalc` is one repo in the larger `DNA Calc` program.
Read [../../Foundation/CHARTER.md](../../Foundation/CHARTER.md) as the
top-level charter for that wider project.
This local charter defines the narrower role, scope, and ownership of
`DnaOneCalc` within that broader system.

## 1. Mission
`DnaOneCalc` is the single-formula proving host for the DNA Calc stack.

Its job is to:
1. make one isolated formula scenario explorable as a serious interactive product,
2. drive `OxFml` and `OxFunc` honestly,
3. expose mechanism-facing `Live Formula Semantic X-Ray` surfaces over that same scenario,
4. prove retained replay, comparison, and Excel-observation flows through `OxReplay` and `OxXlPlay`,
5. pressure upstream repos with structured evidence rather than local reinvention.

## 2. Product Identity
The ordered product expression is:
1. `Formula / Function Explorer` as the first interactive surface,
2. `Live Formula Semantic X-Ray` as the second perspective over the same scenario,
3. `Twin Oracle Workbench` as the third proving surface for retained replay, observation, and compare.

This repo is not:
1. a worksheet engine,
2. a workbook dependency host,
3. a substitute for `OxCalc`,
4. a new semantics lane.

## 3. Dependency Constitution
Primary runtime dependencies:
1. `OxFml`
2. `OxFunc`
3. `OxReplay`

Primary empirical validation dependency:
1. `OxXlPlay`

Staged later dependency:
1. `OxVba`

Runtime non-dependency:
1. `OxCalc`

`OxCalc` remains informative seam-reference material only.

Current interpretation:
1. `OxFml` and `OxFunc` are the first-order explorer dependencies,
2. `OxReplay` is a first-order proving dependency and the highest-risk current design lane,
3. `OxXlPlay` is the Windows-first empirical validation lane,
4. `OxVba` remains later design input for add-ins and host tooling.

## 4. Ownership Boundary
`DnaOneCalc` owns:
1. product shell,
2. host policy,
3. persistence,
4. scenario orchestration,
5. replay and comparison presentation,
6. final host verdict policy over `Matched` / `Mismatched` / `Blocked`,
7. upstream handoff production,
8. host-level test scaffolding, retained scenario corpora, and acceptance harnesses,
9. extension hosting.

`DnaOneCalc` does not own:
1. formula semantics,
2. function semantics,
3. replay comparison or equivalence semantics,
4. VBA semantics,
5. Excel observation semantics.

Those remain in the relevant `Ox*` repos.

## 5. Scope Boundary
The active design takes the single-cell branch of the Charter:
1. `OC-H0` literal-and-function core,
2. `OC-H1` driven single-formula host,
3. `OC-H2` host extensions and add-ins.

It excludes:
1. worksheet-style reference binding,
2. defined-name binding as a public OneCalc model,
3. workbook graph semantics,
4. worksheet `REGISTER.ID` / `CALL` as a product lane.

## 6. Delivery Order
The current delivery order is:
1. stabilize interactive formula/function explorer UX and result surfaces,
2. widen mechanism inspection and X-Ray surfaces,
3. prove retained replay, observation, and compare as honest product lanes,
4. land RTD, XLL, and later OxVba-driven extension work after the above floors are stable.

## 7. Evidence Rule
Every meaningful session should be capable of becoming retained evidence.
Every retained evidence item should be capable of becoming an upstream work request.

## 8. Cross-Repo Boundary Rule
This repo may read sibling repos under the shared `DnaCalc` root for seam consumption, evidence intake, and architectural alignment.

Those sibling repos are read-only from the perspective of this repo.
Required changes outside `DnaOneCalc` must be routed through a handoff, prompt, or separate repo-scoped run.
