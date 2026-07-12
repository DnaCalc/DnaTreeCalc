# DNA OneCalc Programmatic Formula Testing Path

Status: `draft_programmatic_test_path`
Date: 2026-04-05
Scope: headless and scripted batch execution path for formula corpora, retained discrepancy artifacts, and later UX-side browsing and triage

Authority:
1. [SCOPE_AND_SPEC.md](SCOPE_AND_SPEC.md) remains the authority for product boundary, retained evidence, replay, and Excel-comparison obligations.
2. [APP_IMPLEMENTATION_LAYOUT_AND_TDD.md](APP_IMPLEMENTATION_LAYOUT_AND_TDD.md) remains the authority for active code layout and test layering.
3. This note records the intended programmatic execution path that complements the interactive UX without redefining product scope.

## 1. Purpose
This note defines the intended path for running large formula corpora through OneCalc-compatible execution and comparison flows without going through the interactive shell.

The motivating use case is:
1. prepare a corpus of formulas and host-policy profiles,
2. run them headlessly through `OxFml` and `OxXlPlay`,
3. record retained replay and discrepancy artifacts,
4. and then open those artifacts in the ordinary `Inspect` and `Workbench` UX for deeper analysis.

## 2. Scope Reading
This path is:
1. within current product direction,
2. aligned with replay and comparison being first-class surfaces,
3. and a natural extension of the retained-evidence model already defined for the interactive app.

This path is not:
1. a separate product,
2. a second retained-artifact format,
3. or a reason to fork the comparison and replay semantics away from the interactive host.

Interpretation rule:
1. the headless path should produce the same retained artifact families the UX already wants to consume,
2. so that OneCalc remains the browser and triage surface over retained evidence rather than becoming only one of multiple incompatible tools.

## 3. Architectural Fit
The current architecture already points in the right direction.

The headless path should be added as:
1. a service and orchestration layer,
2. plus persistence and indexing support,
3. plus import and open flows in the ordinary app,
4. not as a redesign of the shell or mode model.

The intended fit is:
1. `adapters/oxfml` executes or prepares formula analysis and evaluation,
2. `adapters/oxxlplay` performs Excel observation and empirical comparison where available,
3. `adapters/oxreplay` produces replayable and explainable retained artifacts,
4. `services/` orchestrates batch runs, discrepancy filtering, and retained artifact emission,
5. `persistence/` stores the retained run, comparison, and witness bundles,
6. `Workbench` and `Inspect` reopen the same retained artifacts for human analysis.

## 4. Intended Batch Runner Shape
The batch path should eventually support:
1. corpus input from a file or structured fixture set,
2. host-policy profile selection,
3. capability and platform admission awareness,
4. headless execution against `OxFml`,
5. optional Excel observation through `OxXlPlay` on admitted Windows hosts,
6. discrepancy classification,
7. replay and witness bundle generation,
8. retained artifact indexing for later browsing.

The first concrete product shape should likely be:
1. a repo-local CLI or scripted runner over the shared app and service core,
2. not a second host application,
3. and not shell-only logic embedded in the Tauri entry point.

## 5. Artifact Rule
The headless path should emit the same broad artifact families already implied by the interactive product:
1. scenario input identity,
2. host-policy and capability context,
3. run result and effective display facts,
4. replay bundle,
5. comparison outcome,
6. blocked or lossy explanations,
7. witness or handoff packet where supported.

Rule:
1. if a retained artifact is useful to the headless path, it should usually also be openable from `Workbench`,
2. and if `Workbench` needs a retained artifact family, the headless path should not invent a parallel incompatible representation.

## 6. Host And Capability Constraints
The batch path must preserve platform honesty.

That means:
1. `OxFml`-only analysis and replay preparation should remain available cross-platform,
2. Excel observation and empirical comparison should remain explicitly gated by platform and host admission,
3. discrepancy outputs must distinguish:
   1. compared and mismatched,
   2. compared and matched,
   3. not compared because the empirical lane was unavailable,
   4. not compared because the requested host policy or capability was blocked.

## 7. Persistence And Indexing
The batch path needs one more persistence layer than the current interactive shell.

That layer should support:
1. corpus-run identity,
2. retained run indexing,
3. discrepancy indexing,
4. filterable open flows into `Workbench` and `Inspect`,
5. lightweight metadata for browsing before opening a full retained artifact.

The intended design is:
1. metadata index first,
2. retained payload store second,
3. shell browsing and open-by-id over those persisted outputs.

## 8. Service Families To Add
Likely service additions:
1. `corpus_run_service`
2. `comparison_batch_service`
3. `retained_artifact_index_service`
4. `discrepancy_classification_service`
5. `artifact_open_service`

Likely persistence additions:
1. batch-run manifest
2. retained artifact catalog
3. discrepancy summary store
4. import/open references for Workbench

## 9. UX Relationship
The headless path should not create a second analysis UX.

Instead:
1. `Explore` remains the authoring surface,
2. `Inspect` remains the semantic and mechanism-analysis surface,
3. `Workbench` remains the retained evidence and discrepancy-analysis surface,
4. the batch runner simply feeds those surfaces with a larger retained corpus than an individual interactive session would produce.

## 10. Initial Non-Goals
The first headless path does not need:
1. distributed scheduling,
2. remote fleet execution,
3. workbook-scale orchestration,
4. a separate web dashboard,
5. or a new independent retained artifact ontology.

## 11. Implementation Guidance
The first implementation should aim for:
1. a small batch runner over the shared services,
2. one corpus input format,
3. one retained artifact store,
4. one discrepancy index,
5. one open flow into the app.

That is enough to prove the path without prematurely turning OneCalc into a generalized automation platform.
