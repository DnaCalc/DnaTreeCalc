# DNA OneCalc Scope And Specification

Status: `active_scope_and_spec`
Date: 2026-03-27
Supersedes:
1. the earlier Foundation-side initial scope note for this topic
2. the fragmented planning spread across the initial scope note and the first review passes

## 1. Purpose
This document is the current single engineering specification for `DnaOneCalc`.

It is intended to be complete enough to:
1. define the repo mission,
2. define the host boundary,
3. define the product direction,
4. define the artifact model,
5. define the dependency and gate model,
6. define the derived engineering obligations for the repo and host,
7. drive repo bootstrap and repo-bound agent work without needing to reconstruct the plan from multiple scattered notes.

It is not the authoritative semantic owner for:
1. formula semantics,
2. function semantics,
3. replay semantics,
4. VBA semantics.

Those remain in the `Ox*` repos.

### 1.1 Reading Order
Use this note in the following order:
1. Sections `2` through `5` define mission, boundary, ownership, and product intent.
2. Sections `6` through `12` define the artifact model, host profiles, UI/runtime shape, replay/comparison contract, persistence, and extension model.
3. Section `13` states what the repo must derive from this engineering spec without turning the note into a live execution tracker.
4. Sections `14` through `19` record the current conservative upstream floor, success criteria, residual pressure, interpretation rules, and the authoritative cross-repo reference set that implementation work should actually consume.

## 2. Role In The Program
`DNA OneCalc` is:
1. a downstream proving host,
2. a serious user-facing application,
3. a co-development and scope-discovery program for the `Ox*` repos,
4. the first product-stage single-node proving surface for the formula, function, replay, and Excel-comparison stack.

It is not:
1. a new semantics lane,
2. a replacement for `OxFml`, `OxFunc`, `OxReplay`, or `OxVba`,
3. a general spreadsheet grid host,
4. a workbook dependency engine,
5. an `OxCalc` host,
6. a claim of “Excel except for some missing pieces.”

The right reading is:
1. Excel as a single isolated calculation node,
2. with explicit host context,
3. with first-class replay and comparison,
4. without workbook graph semantics.

## 3. Core Thesis
The core mission of `DNA OneCalc` is:
1. accept a formula string of arbitrary supported complexity,
2. evaluate it through `OxFml` and `OxFunc`,
3. present the result, help, diagnostics, and effective display state in a serious interactive host,
4. expose mechanism-facing `Live Formula Semantic X-Ray` surfaces over the same active scenario,
5. emit replayable evidence through `OxReplay`,
6. compare and replay scenarios against Excel through `OxXlPlay`,
7. grow a durable scenario library that validates the stack against Excel and pressures the upstream repos productively.

The ordered product direction is:
1. `Formula / Function Explorer` first,
2. `Live Formula Semantic X-Ray` second,
3. `Twin Oracle Workbench` third.

That means the central user experience is not merely:
1. type a formula,
2. see a value.

It is:
1. author and edit a formula,
2. inspect help, diagnostics, result, and effective display,
3. open X-Ray views for parse, bind, evaluation, and provenance,
4. retain the run and reopen it reproducibly,
5. compare against Excel on Windows where that lane is available,
6. explain mismatches,
7. distill witnesses where the lane floor supports it,
8. emit upstream-ready handoff packets.

In short:
1. every meaningful session should be capable of becoming retained evidence,
2. every retained evidence item should be capable of becoming an upstream work request.

## 4. Ownership And Dependency Constitution
Primary runtime dependencies:
1. `OxFml`
2. `OxFunc`
3. `OxReplay`

Primary empirical validation dependency:
1. `OxXlPlay`

Staged later dependency:
1. `OxVba`

Explicit non-dependency for the initial repo mission:
1. `OxCalc`

Ownership split:
1. `DnaOneCalc` owns product shell, host policy, UI, persistence, extension hosting, scenario orchestration, and upstream handoff production.
2. `OxFml` owns formula-language semantics, host-policy seams, semantic formatting, formula-semantic conditional-formatting carriers, and the canonical formula-edit language-service substrate used by hosts.
3. `OxFunc` owns value and function semantics, library/runtime context seams, registered-external function machinery, and the authoritative function-help or signature-metadata truth that OxFml should project into host-facing editor packets.
4. `OxReplay` owns replay bundle, replay execution, diff, explain, witness, and adapter/conformance infrastructure.
5. `OxXlPlay` owns live Excel-facing observation and capture.
6. `OxVba` owns VBA semantics and later VBA-backed extension tooling.

Important rule:
1. `DnaOneCalc` consumes lane semantics,
2. it does not locally redefine them,
3. it should produce structured downstream pressure and actionable upstream work rather than pretending the current libraries are frozen.

### 4.0 Upstream Reference Rule
`DNA OneCalc` should design and implement against the authoritative upstream slices recorded in Section `19`.

Working rule:
1. prefer each upstream repo's root `CHARTER.md`, root `OPERATIONS.md`, `docs/spec/README.md`, non-archive spec docs named there, and the current `docs/IN_PROGRESS_FEATURE_WORKLIST.md` plus `CURRENT_BLOCKERS.md`,
2. treat worksets, handoff notes, execution records, and test-run notes as current-status or evidence docs rather than semantic authority unless Section `19` explicitly names them as temporary downstream references,
3. ignore archive paths, mirrors, local snapshots, and historical synthesis material unless doing archaeology or drift resolution,
4. preserve `prelim`, `draft`, `working-draft`, `design-draft`, or similar status markers as real scope constraints rather than hand-waving past them,
5. if a required downstream surface has no good stable upstream doc, treat that as upstream documentation debt rather than local permission to invent a private contract.

### 4.1 OxCalc/OxFml Seam Reference Rule
`OxCalc` is not a runtime library dependency of `DNA OneCalc`.

But the host-facing seam used to drive `OxFml` is materially related to the seam already documented between `OxCalc` and `OxFml`.

Therefore:
1. `OxCalc` seam documentation is also reference material for `DNA OneCalc`,
2. the fact that `DNA OneCalc` does not depend on the `OxCalc` library does not mean the `OxCalc` spec set is irrelevant,
3. if `DNA OneCalc` discovers that the consumed `OxFml` interface needs to change, the corresponding `OxCalc` seam reference material may also need updating to prevent cross-repo drift.

The most relevant current OxCalc references are:
1. `..\\..\\OxCalc\docs\spec\core-engine\CORE_ENGINE_DOWNSTREAM_HOST_SEAM_REFERENCE.md`
2. `..\\..\\OxCalc\docs\spec\core-engine\CORE_ENGINE_OXFML_SEAM.md`
3. `..\\..\\OxCalc\docs\spec\core-engine\CORE_ENGINE_OXFML_MINIMAL_UPSTREAM_HOST_INTERFACES.md`
4. `..\\..\\OxCalc\docs\spec\core-engine\CORE_ENGINE_TREECALC_SEMANTIC_COMPLETION_PLAN.md`
5. `..\\..\\OxCalc\docs\spec\core-engine\CORE_ENGINE_TREECALC_OXFML_SEAM_NEGOTIATION_MATRIX.md`

Interpretation rule:
1. `OxFml` remains authoritative for evaluator-side semantics and canonical shared seam meaning,
2. `OxCalc` remains an important reference owner for coordinator-facing and consumed-host-packet seam shape,
3. `DNA OneCalc` should treat both repos as part of the reference surface for this seam even while only consuming `OxFml`, `OxFunc`, `OxReplay`, and `OxXlPlay` at runtime.

Current concrete read from the refreshed OxCalc seam docs:
1. the first implementation-backed host packet already carries more than a bare formula string, including caller-anchor facts, structure-context versioning, typed host-query facts, table-context carriage, and a `library_context_snapshot`,
2. that packet is still seam-reference material only and must not be mistaken for a frozen production host API,
3. `DNA OneCalc` should use this packet family to understand what the live seam currently needs, then narrow and productize that shape explicitly in its own host profiles rather than pretending the wider packet does not exist.

The authoritative OxCalc seam-reference slice is enumerated in Section `19.6`.

### 4.2 OxFml Formula-Editing Language-Service Reference Rule
`DNA OneCalc` should also treat the OxFml editor and language-service surface as a first-class upstream dependency seam.

That surface includes, or should include:
1. immutable formula-edit request and result packets,
2. unified live diagnostics and squiggle-ready spans,
3. deterministic completion proposals,
4. validated completion application through the ordinary parse/bind path,
5. signature-help context,
6. function-help lookup and payload flow,
7. external intelligent-completion context and validation boundaries.

The most relevant current OxFml references are:
1. `..\\..\\OxFml\docs\spec\OXFML_DNA_ONECALC_DOWNSTREAM_CONSUMER_CONTRACT.md`
2. `..\\..\\OxFml\docs\spec\OXFML_HOST_RUNTIME_AND_EXTERNAL_REQUIREMENTS.md`
3. `..\\..\\OxFml\docs\spec\OXFML_DNA_ONECALC_HOST_POLICY_BASELINE.md`
4. `..\\..\\OxFml\docs\spec\OXFML_PUBLIC_API_AND_RUNTIME_SERVICE_SKETCH.md`
5. `..\\..\\OxFml\docs\spec\formula-language\OXFML_EDITOR_LANGUAGE_SERVICE_AND_HOST_INTEGRATION_PLAN.md`
6. `..\\..\\OxFml\docs\spec\OXFML_FIXTURE_HOST_AND_COORDINATOR_STANDIN_PACKET.md`
7. `..\\..\\OxFml\docs\IN_PROGRESS_FEATURE_WORKLIST.md`

Current read of the OxFml floor:
1. OxFml now has a real OneCalc-facing first-integration contract in `OXFML_DNA_ONECALC_DOWNSTREAM_CONSUMER_CONTRACT.md`,
2. that contract now makes the H0 or H1 mandatory field floor, probe-only field families, and coordinator-only or TreeCalc-only reference material explicit,
3. there is already a real local language-service packet layer in OxFml,
4. the current integration-ready floor includes `FormulaEditRequest` / `FormulaEditResult`, `LiveDiagnosticSnapshot`, deterministic completion, completion validation and application, `SignatureHelpContext`, `FunctionHelpLookupRequest`, `IntelligentCompletionContext`, and `EditorSyntaxSnapshot`,
5. deterministic local test evidence already exists in OxFml for that floor.

Current residuals that matter to `DNA OneCalc`:
1. no OxFunc-backed help or signature payload retrieval is frozen yet,
2. no shared host or OxCalc immutable formula-edit packet is frozen yet,
3. no shared host-facing packet for validated intelligent-completion results is frozen yet,
4. editor packet evidence is still local deterministic evidence rather than replay-appliance projection,
5. the new OxFml downstream-consumer contract is still a first-integration clarification note rather than a bilateral frozen shared seam.

Interpretation rule:
1. `DNA OneCalc` should exercise and integrate this OxFml language-service surface rather than inventing a second parser/binder/editor truth locally,
2. if `DNA OneCalc` pressures changes to these packets, that should be treated as a real cross-repo seam update request,
3. where the same packet family is also reflected from the coordinator side, the OxCalc seam-reference material may need updating as well.

The authoritative OxFml language-service and host/runtime slice is enumerated in Section `19.2`.

### 4.3 Function Surface Truth Rule
`DNA OneCalc` must pin its function surface against the current OxFunc admission overlays rather than against the broadest exported catalog view.

Working rule:
1. treat the OxFunc library-context snapshot export as the current downstream catalog and metadata seed,
2. read that export together with `OXFUNC_DOWNSTREAM_METADATA_AND_HELP_CONTRACT.md`, `OXFUNC_SURFACE_ADMISSION_AND_LABELING_POLICY.md`, `W050`, and `W051`, not by itself,
3. treat snapshot-export fields in three tiers: `stable`, `usable-but-provisional`, and `current-tree-hint-only`,
4. derive the downstream admission label by joining the snapshot export against the current `W050` and `W051` inventories,
5. use the OxFunc downstream labels `supported`, `preview`, `experimental`, `deferred`, and `catalog_only` in product and scenario metadata rather than inventing private categories,
6. prefer scenario families whose rows are already function-phase-complete or at least `doc_modeled` with a clear seam contract,
7. treat `LET`, `LAMBDA`, helper-family functions, `IMAGE`, `GROUPBY`, `PIVOTBY`, `OP_IMPLICIT_INTERSECTION`, the native-extension lane, and the RTD lane as explicit scope markers rather than as silently complete surface.

Current read of the OxFunc floor:
1. OxFunc now has a consolidated downstream metadata/help contract and a separate surface-admission and labeling policy,
2. the current help or signature floor is still metadata-backed rather than prose-rich,
3. structured help prose, argument names or descriptions, and formatted signature strings remain upstream work items,
4. several seam-heavy rows now sit in `W051` for promotion or seam-freeze reasons rather than because they lack a real OxFunc kernel.

The authoritative OxFunc surface and overlay slice is enumerated in Section `19.3`.

## 5. Product Expression
`DNA OneCalc` should be a serious product, not only a harness.

The ordered product perspectives are:
1. `Formula / Function Explorer`,
2. `Live Formula Semantic X-Ray`,
3. `Twin Oracle Workbench`.

The explorer perspective should make the following first-class:
1. formula text,
2. host-driving and recalc context,
3. live diagnostics,
4. deterministic completion and validated completion application,
5. function and argument help during editing,
6. result and effective display.

The X-Ray perspective should make the following first-class:
1. parse tree,
2. evaluation trace,
3. semantic provenance,
4. diffable mechanism state,
5. capability and host-context truth relevant to interpretation.

The workbench perspective should make the following first-class:
1. replay state,
2. semantic diff,
3. observation and comparison state,
4. witness state,
5. upstream handoff readiness.

The named workbench modes are:
1. `DNA-only`
2. `Excel-observed`
3. `Twin compare`
4. `Replay`
5. `Diff`
6. `Explain`
7. `Distill`
8. `Handoff`

These should be treated as named product modes with explicit capability gates, not as vague inspiration words.

### 5.1 Product Promises
`DNA OneCalc` being a serious product should mean the following concrete host promises:
1. keyboard-first formula and function exploration,
2. visible result, effective display, and currently available formatting truth,
3. visible diagnostics, completion, and help integrated into the main editing flow,
4. durable X-Ray surfaces over the same authored scenario,
5. visible host profile, packet kind, and function-surface policy,
6. visible platform limits and provisionality state,
7. retained scenario authoring rather than session-only work,
8. replay, diff, explain, and handoff controls once the retained and replay floors are honestly admitted,
9. a visible capability center and exportable capability snapshot for the exact dependency set in use as a supporting honesty surface,
10. importable and exportable `Scenario Capsule` transport for real evidence sharing,
11. no silent overclaim about parity, platform reach, or upstream seam maturity.

### 5.2 Mode Gate Discipline
Each named workbench mode must declare:
1. required runtime dependencies,
2. required replay or observation capability floor,
3. platform availability,
4. admissible output artifacts,
5. provisional labeling rules.

If a mode cannot satisfy its declared floor, the UI must:
1. hide it,
2. disable it with an explicit reason,
3. or show it as provisional or experimental with the exact capability gap visible.

### 5.3 Capability Observatory And Dependency Ledger
`DNA OneCalc` should include a first-class `Capability Observatory`.

This is not a diagnostics afterthought.
It is a supporting honesty surface that states what the current host build and dependency set can honestly do.
It should support the explorer, X-Ray, replay, and compare surfaces rather than compete with them for primary attention.

The observatory should have two coordinated forms:
1. a machine-readable `CapabilityLedgerSnapshot`,
2. a UI-visible `Capability Center`.

The machine-readable ledger should record, at minimum:
1. OneCalc build identity,
2. exact dependency set in use,
3. effective seam pin set,
4. active host-profile ladder and packet-kind register,
5. admitted function-surface policy and current overlay inputs,
6. replay capability floor and active adapter-capability claims,
7. active observation envelope and live-capture availability,
8. extension ABI availability and platform gates,
9. declared provisional seams and known capability ceilings.

The UI-visible `Capability Center` should make that truth inspectable and diffable by a user without reading raw manifests.

Working rules:
1. every serious OneCalc environment should be able to emit one capability snapshot for the exact dependency set in use,
2. every retained `ScenarioRun` should be able to point back to the capability snapshot that governed it,
3. `HandoffPacket` generation should pull from the same capability snapshot rather than restating seam and floor truth manually,
4. capability snapshots should be comparable across OneCalc versions and dependency pin sets,
5. the observatory must report actual executable floors, not aspirational documentation claims.

#### 5.3.1 Capability Ledger Snapshot Contract
`CapabilityLedgerSnapshot` should be treated as a retained engineering artifact with a stable contract, not as an ad hoc debug dump.

The snapshot should minimally contain:
1. `capability_snapshot_id`,
2. `schema_id` and `schema_version`,
3. emitted-at timestamp and emitter build identity,
4. host kind, platform, and runtime class,
5. exact dependency identities for `OxFml`, `OxFunc`, `OxReplay`, `OxXlPlay`, and extension providers where loaded,
6. seam pin set and dependency pin set,
7. active host-profile ladder and packet-kind register,
8. admitted function-surface labels and the overlay inputs from which they were derived,
9. replay adapter capability claims actually relied upon,
10. observation envelope availability and live-capture availability,
11. extension ABI, RTD, and platform gate facts,
12. mode-availability facts for `DNA-only`, `Excel-observed`, `Twin compare`, `Replay`, `Diff`, `Explain`, `Distill`, and `Handoff`,
13. declared provisional seams, lossy projections, capability ceilings, and blocked modes.

Hard rules:
1. every field must describe executable truth for the current environment,
2. the snapshot must distinguish observed fact from derived interpretation,
3. aspirational support and roadmap language must not appear in the snapshot.

#### 5.3.2 Capability Snapshot Emission And Lifecycle
Capability snapshots should be emitted whenever the executable truth of the host changes materially.

Required emission points:
1. host startup or workspace-open initialization,
2. dependency re-resolution or seam-pin change,
3. extension enable, disable, load, unload, or registration-state change,
4. explicit user refresh,
5. export of a `ScenarioCapsule` or `HandoffPacket`,
6. any retained `ScenarioRun` whose governing capability snapshot does not already exist.

Lifecycle rules:
1. a snapshot is immutable once emitted,
2. later environment changes create a new snapshot rather than mutating the old one,
3. `ScenarioRun`, `Comparison`, `Witness`, `HandoffPacket`, and `ScenarioCapsule` should refer to snapshots by stable ref,
4. capability-snapshot diffs should compare two immutable snapshots and report added, removed, widened, narrowed, or relabeled capability facts.

#### 5.3.3 Capability Center Product Contract
The `Capability Center` is the human-facing projection of the capability ledger.

It should provide:
1. the current active snapshot,
2. a diff against one prior snapshot or one imported snapshot,
3. per-mode explanations for disabled or hidden surfaces,
4. dependency, seam-pin, and capability-floor inspection without opening raw files,
5. export of the active snapshot and any active diff for handoff or review,
6. stable links from current UI state back to the governing capability facts.

UI rule:
1. the context bar shows compact truth,
2. the `Capability Center` shows expanded truth,
3. both must project the same underlying snapshot rather than maintaining parallel status models.

## 6. Canonical Artifact Spine
The repo should revolve around stable artifacts, not around ad hoc UI state.

### 6.0 Artifact Identity, Provenance, And Container Rule
Every retained artifact should distinguish four identity classes:
1. stable logical ids such as `scenario_id`, `scenario_run_id`, `comparison_id`, and `witness_id`,
2. content fingerprints for persisted payloads and replay-relevant snapshots,
3. seam pins and capability pins that identify which upstream contract floor the artifact relied on,
4. run-local operational ids for ephemeral UI sessions or process-local activity.

The mapping between artifact roles should stay explicit:
1. `Scenario` is the authoring unit,
2. `ScenarioRun` is the execution and replay unit,
3. `Observation` is the external truth unit,
4. `Comparison` is the judgment unit,
5. `Witness` is the retained counterexample unit,
6. `HandoffPacket` is the upstream-action unit,
7. `Document` is the persistence container and must not silently become workbook truth.

Supporting observability artifact:
1. `CapabilityLedgerSnapshot` is a supporting artifact, not a replacement for the canonical spine,
2. it records the effective dependency, capability, and platform truth under which authored and retained artifacts were produced,
3. it should be referenceable from `ScenarioRun`, `HandoffPacket`, and the workspace shell.

Supporting transport artifact:
1. `ScenarioCapsule` is a portable folder-based transport unit, not a replacement for `Document`,
2. it packages one authored scenario together with selected retained evidence and transport metadata,
3. it is the preferred sharing unit for replay, comparison, witness, and upstream handoff work across repos and hosts.

### 6.1 Scenario
The canonical authored unit.

A `Scenario` should minimally contain:
1. `scenario_id` and `scenario_slug`,
2. formula text and formula channel kind,
3. host profile id,
4. host-driving packet kind,
5. host-driving block,
6. recalc and volatility context where admitted,
7. display context,
8. library-context snapshot or provider refs,
9. function-surface policy id,
10. extension profile or provider state where relevant,
11. retained notes and intent,
12. stable identity and provenance fields.

### 6.2 ScenarioRun
A concrete execution of a scenario under:
1. a specific build,
2. a specific profile version,
3. a specific dependency seam set,
4. a specific runtime environment.

It should minimally record:
1. `scenario_run_id` and `scenario_id`,
2. build id and runtime platform,
3. seam pin set id and effective capability floor,
4. result surface ref,
5. candidate, commit, reject, and trace refs where produced,
6. replay capture ref,
7. function-surface effective id,
8. projection and provisionality status,
9. execution metadata and timestamp.

### 6.3 Observation
An external truth artifact, most importantly:
1. Windows-only Excel-observed output through `OxXlPlay`.

It should minimally record:
1. `observation_id`,
2. source lane id and source schema,
3. source artifact ref,
4. capture mode,
5. projection status,
6. provenance ref,
7. capture-loss ref,
8. platform scope.

### 6.4 Comparison
A typed comparison between:
1. a `ScenarioRun` and an `Observation`,
2. or two `ScenarioRun` instances.

It should classify:
1. value agreement,
2. formula-text agreement,
3. type agreement,
4. display agreement,
5. formatting agreement,
6. conditional-formatting agreement,
7. projection limitations,
8. reliability badge,
9. trace-level or provenance-level divergence where available.

Rule:
1. the comparison schema may be wider than the active comparison envelope for a given artifact pair,
2. every `Comparison` must therefore declare the active comparison envelope,
3. and it must mark inactive dimensions as intentionally omitted or unavailable rather than leaving them ambiguous.

### 6.5 Witness
A retained unreduced or reduced counterexample artifact.

It should preserve lineage back to:
1. scenario,
2. run,
3. comparison,
4. replay bundle,
5. predicate,
6. reduction state,
7. lifecycle and quarantine state.

### 6.6 HandoffPacket
A repo-addressable upstream pressure artifact that points to:
1. scenario,
2. run,
3. comparison,
4. witness,
5. exact seam versions,
6. exact requested upstream action,
7. expected behavior,
8. observed behavior,
9. supporting artifact refs,
10. status.

The first normalized `requested_action_kind` taxonomy should include:
1. `freeze_contract`,
2. `clarify_contract`,
3. `close_gap`,
4. `promote_surface`,
5. `narrow_scope`,
6. `document_limit`,
7. `accept_provisionality`,
8. `define_registry`,
9. `define_payload`,
10. `define_projection`.

Readiness rule:
1. drafting a handoff should be cheap,
2. exportability should be gated by a readiness checklist.

The first `ready/exportable` checklist should require:
1. target lane selected,
2. requested action selected,
3. expected versus observed behavior stated concretely,
4. at least one retained source artifact attached,
5. exact build, seam, and platform context present,
6. reliability/projection state present,
7. active comparison envelope declared where comparison is involved,
8. witness lineage present where witness-driven,
9. provisional or lossy evidence stated explicitly where it applies.

Product rule:
1. users may save draft handoffs before the readiness checklist passes,
2. the UI must show missing readiness items explicitly,
3. only ready handoffs may be exported as upstream work requests,
4. the workbench should auto-pull scenario, run, comparison, witness, and replay-floor context rather than making the user retype known facts.

### 6.7 Document
A persisted container for one isolated OneCalc instance plus its retained local context.

For the current persistence scope, a `Document` should hold:
1. exactly one isolated scenario or instance,
2. formatting state,
3. conditional-formatting state,
4. retained local metadata,
5. refs to attached retained artifacts where those are persisted externally.

The document container must not silently imply workbook-graph semantics.

Interpretation rule:
1. `Scenario` remains the authored semantic unit,
2. `Document` remains the persisted envelope,
3. `ScenarioRun` remains the replay and comparison unit,
4. no document feature should imply a shared recalc graph,
5. multi-instance management belongs in the host workspace layer rather than in the meaning of one persisted OneCalc file.

### 6.7.1 ScenarioCapsule
A `ScenarioCapsule` is a portable folder-based export and intake format for one isolated OneCalc scenario and its evidence.

It should minimally contain:
1. one `Scenario`,
2. zero or more retained `ScenarioRun` artifacts,
3. zero or more `Observation` artifacts,
4. zero or more `Comparison` artifacts,
5. zero or more `Witness` artifacts,
6. zero or more `HandoffPacket` artifacts,
7. referenced replay bundles and observation bundles,
8. an attachment index,
9. one or more referenced `CapabilityLedgerSnapshot` artifacts,
10. a capsule manifest that records included artifacts, hashes, and lineage roots.

Rules:
1. a `ScenarioCapsule` is for transport and sharing, not for redefining the authored semantic unit,
2. one capsule may include multiple retained runs and comparisons for the same scenario across versions and platforms,
3. capsule contents must preserve original logical ids rather than rewriting artifact identity on export,
4. export and intake must preserve capability-snapshot refs so upstream consumers can see exactly which executable floor produced the evidence.

### 6.7.2 ScenarioCapsule Layout And Manifest Contract
The first engineering `ScenarioCapsule` should be folder-based and inspectable.

Recommended top-level layout:
1. `capsule_manifest.json`,
2. `scenario/`,
3. `runs/`,
4. `observations/`,
5. `comparisons/`,
6. `witnesses/`,
7. `handoffs/`,
8. `capabilities/`,
9. `attachments/`.

The manifest should minimally contain:
1. `capsule_id`,
2. `schema_id` and `schema_version`,
3. export tool and build identity,
4. root `scenario_id`,
5. included artifact inventory by kind and logical id,
6. included attachment inventory by stable ref and content hash,
7. capability snapshot refs included in the capsule,
8. lineage roots and export-time integrity hashes,
9. any declared redaction, omission, or lossy transport note.

Layout rules:
1. included artifacts should be stored in ordinary inspectable files rather than opaque archives,
2. attachments should be addressable by stable ref and content hash,
3. capsule manifests must be sufficient to validate integrity without repo-local implicit state.

### 6.7.3 ScenarioCapsule Export And Intake Rules
Export and intake should behave deterministically and preserve evidence truth.

Export rules:
1. export selection must be explicit about which runs, observations, comparisons, witnesses, handoffs, and capability snapshots are included,
2. export must preserve logical ids, lineage refs, and content hashes,
3. export must record omitted sibling artifacts rather than pretending the capsule is a total workspace dump when it is not,
4. exported capsules must remain inspectable without OneCalc-specific unpack tooling.

Intake rules:
1. intake must validate the manifest and attachment hashes before accepting the capsule,
2. if an imported artifact matches an existing logical id and content hash, it should deduplicate rather than fork silently,
3. if an imported artifact matches an existing logical id but differs in content hash, intake must preserve both versions explicitly rather than overwriting one,
4. imported artifacts remain imported evidence until explicitly promoted into local authored work,
5. intake must preserve capability-snapshot refs and attachment structure exactly as exported.

Current upstream reference slice for the artifact spine:
1. `..\\..\\OxReplay\docs\spec\OXREPLAY_BUNDLE_WITNESS_AND_REGISTRY_MODEL.md`,
2. `..\\..\\OxReplay\docs\spec\OXREPLAY_ADAPTER_AND_CONFORMANCE_MODEL.md`,
3. `..\\..\\OxReplay\docs\spec\DNA_RECALC_HOST.md`,
4. `..\\..\\OxXlPlay\docs\spec\OXXLPLAY_BUNDLE_EMISSION_AND_HANDOFF_MODEL.md`,
5. `..\\..\\OxXlPlay\docs\spec\OXXLPLAY_SCENARIO_REGISTER.md`.

### 6.8 Shared Artifact Envelope
Every retained artifact should carry a common engineering envelope even where the inner payload differs by artifact kind.

The common minimum envelope should include:
1. `schema_id`,
2. `schema_version`,
3. `artifact_kind`,
4. `logical_id`,
5. `content_hash`,
6. `created_at`,
7. `created_by_build`,
8. `host_profile_id`,
9. `packet_kind`,
10. `seam_pin_set_id`,
11. `capability_floor`,
12. `provisionality_state`,
13. `lineage_refs`,
14. `attachment_refs`,
15. `capability_snapshot_ref` where the artifact depends on an emitted observatory state.

Rule:
1. artifact-specific payloads may evolve,
2. the envelope must remain stable enough that indexing, replay linkage, handoff generation, and provenance inspection do not require per-artifact custom logic for basic identity questions.

### 6.9 First Internal Schema Minimums
The first engineering schema minimum for each canonical artifact should be:

`Scenario`
1. identity block,
2. formula block,
3. host-profile block,
4. host-driving and recalc block,
5. library-context block,
6. display-context block,
7. extension-state block,
8. notes or intent block,
9. seam-pin block.

`ScenarioRun`
1. identity block,
2. scenario ref,
3. environment block,
4. execution-context snapshot,
5. result-surface ref,
6. replay refs,
7. comparison-eligibility block,
8. provisionality and capability block,
9. timing and telemetry summary,
10. capability-snapshot ref.

`Observation`
1. identity block,
2. source-system block,
3. provenance block,
4. capture envelope,
5. observed-surface table,
6. capture-loss and uncertainty table,
7. normalized replay-view refs.

`Comparison`
1. identity block,
2. left and right artifact refs,
3. comparison envelope,
4. typed mismatch table,
5. reliability block,
6. explanation refs,
7. witness-candidate refs.

`Witness`
1. identity block,
2. source comparison ref,
3. predicate block,
4. retained mismatch core,
5. reduction state,
6. lifecycle state,
7. quarantine state,
8. pack-eligibility flags.

`HandoffPacket`
1. identity block,
2. target-lane block,
3. requested-action block,
4. supporting-artifact refs,
5. assumption and warning block,
6. readiness checklist,
7. output or export metadata,
8. capability-snapshot ref or embedded capability summary.

`Document`
1. identity block,
2. document metadata,
3. instance manifest,
4. retained-artifact index,
5. attachment index,
6. UI view-state block,
7. persistence-format metadata.

`CapabilityLedgerSnapshot`
1. identity block,
2. emitter and environment block,
3. dependency identity block,
4. seam-pin and capability-floor block,
5. mode-availability block,
6. provisionality, lossiness, and ceiling block,
7. diff-base refs where a comparison baseline is declared.

`ScenarioCapsule`
1. identity block,
2. root-scenario ref,
3. included-artifact inventory,
4. included capability-snapshot inventory,
5. attachment inventory,
6. export integrity block,
7. import-status block.

### 6.10 Lineage And Referential Integrity Rules
The following invariants should hold:
1. every `ScenarioRun` must point to exactly one `Scenario`,
2. every `Comparison` must name both compared artifacts and the comparison envelope used,
3. every `Witness` must point back to exactly one source `Comparison`,
4. every `HandoffPacket` must point to at least one retained source artifact and at least one target lane,
5. a `Document` may embed or reference retained artifacts, but the artifact ids must remain stable when the document is duplicated or forked,
6. no lineage link may rely only on UI-local state,
7. where a `ScenarioRun` or `HandoffPacket` depends on declared seam floors or platform gates, it must preserve a stable ref to the governing capability snapshot.

## 7. Host Profile Ladder
`DnaOneCalc` should use an explicit host-profile ladder over a single-node substrate.

### 7.0 Packet Kind Rule
`DNA OneCalc` should freeze a small host-profile and packet discipline early and avoid ad hoc packet growth.

The governing rules are:
1. `DNA OneCalc` uses the host-profile ladder `OC-H0`, `OC-H1`, and `OC-H2` as product-level classification,
2. actual consumed seam field names and packet names follow the current OxFml downstream-consumer contract,
3. `OxFml` is normative for the consumed seam and `OxCalc` is informative seam-reference material,
4. no OneCalc-local reference-probe packet family is part of the current scope,
5. the first admitted extension-facing packet family is host-managed extension registration rather than worksheet `REGISTER.ID` / `CALL` semantics.

Charter-alignment rule:
1. the Charter's "single-cell or defined-name substrate" wording is permissive,
2. this note intentionally takes the single-cell branch and excludes defined-name binding from the current OneCalc scope.

### 7.1 OC-H0: Literal And Function Core
Purpose:
1. stand up the narrowest honest host,
2. prove formula string to parse/bind/evaluate/result/replay end to end.

In scope:
1. literals,
2. operators,
3. built-in functions that require no external provider or workbook state,
4. `FormulaSourceRecord`, `formula_channel_kind`, and `structure_context_version`,
5. immutable `LibraryContextSnapshot` carriage through `LibraryContextProvider`,
6. locale and date-system context through `LocaleFormatContext`,
7. deterministic volatile inputs such as `now_serial` and `random_provider` when the active formula family requires them,
8. result display with semantic formatting consequences where already available upstream,
9. replay capture for every executed scenario,
10. attractive but honest result presentation.

Coverage rule:
1. `OC-H0` claims must still be filtered through the admitted OxFunc current surface,
2. exported catalog presence alone is not enough,
3. promoted H0 scenario families should start with rows that are already function-phase-complete or explicitly modeled with a stable seam contract,
4. default demos, screenshots, and product-claim scenarios should use only promoted current surface rather than `W051` pressure rows.

Out of scope:
1. references,
2. defined names,
3. host queries,
4. external providers,
5. add-ins,
6. VBA-backed functions,
7. workbook state.

### 7.2 OC-H1: Driven Single-Formula Host
Purpose:
1. make the first widened single-node host truth real without sliding toward worksheet semantics,
2. remain clearly narrower than `OxCalc`.

In scope:
1. one authoritative formula under test,
2. formula edit-and-accept recalc,
3. manual recalc and forced recalc,
4. scriptable or host-driven formula replacement for repeated-run families,
5. `RtdProvider` where `RTD` is admitted,
6. replay-visible host-driving consequences,
7. base formatting state,
8. effective-display projection,
9. isolated-instance conditional-formatting rules,
10. version-to-version retained replay and comparison over the same scenario family.

Important rules:
1. the default product claim must not expose a generic cell environment,
2. it must not expose a workbook-style name manager,
3. it must not exercise the OxCalc/OxFml dereference seam,
4. it must not admit `defined_name_bindings`, `HostInfoProvider`, direct reference binding, or worksheet-style host queries as part of the current host profile,
5. `LET(...)` is the preferred in-formula factoring mechanism for value setup inside one formula.

Out of scope:
1. formula dependency graphs,
2. multi-node recalculation,
3. scheduler policy,
4. workbook structural edits,
5. cross-instance interaction.

### 7.2.1 Current H1 Driving Model
The current `OC-H1` driving model should be interpreted as follows.

1. the public host model is driven single-formula execution rather than explicit-input binding,
2. formula variation across scenario families comes from:
   - editing the formula,
   - scriptable or host-driven formula replacement,
   - deterministic recalc commands,
   - RTD-backed push updates where admitted,
   - later host-loaded extension-backed functions,
3. the runtime recalc behavior should model a single Excel cell:
   - edit-and-accept triggers recalculation even when the accepted text is unchanged,
   - manual recalc should behave like Excel-style volatile recalc when volatile or push-driven inputs are present,
   - forced recalc should be available as a distinct unconditional rerun path,
4. OneCalc remains a proving host over one isolated formula rather than a partial worksheet environment.

### 7.3 OC-H2: Host Extensions And Add-ins
Purpose:
1. widen into real external function surfaces without drifting into a spreadsheet engine.

In scope:
1. desktop add-ins over a declared portable C ABI,
2. Windows `.xll` packaging,
3. Linux `.so` packaging over the same admitted ABI,
4. host-loaded registration of exported functions into the active function catalog,
5. replay-visible extension registration and invocation,
6. RTD under the admitted host/runtime contract.

ABI direction:
1. support a tight subset of the Excel C API as defined by the Excel SDK and corresponding header/code artifacts,
2. support `XLOPER12` only,
3. do not support legacy `XLOPER`,
4. admit `xlfRegister` Form 1, `xlfEvaluate`, `xlUDF`, and `xlfRtd`,
5. support Excel-style lifecycle entry points such as `xlAutoOpen` and `xlAutoClose`,
6. accept Excel-style `Excel12(...)` host calls for the admitted subset,
7. allow registration strings to carry thread-safe indicators while execution remains on the OneCalc single calculation thread.

Out of scope for the current honest claim:
1. worksheet `REGISTER.ID` / `CALL` semantics as a product lane,
2. workbook macro model,
3. scheduler semantics,
4. fake web parity for native extensions,
5. native add-ins in browser/WASM hosts.

### 7.4 Current Function Surface Rule
`DNA OneCalc` should explicitly carry a function-surface admission layer in its own planning and UI.

That layer should distinguish:
1. promoted current surface,
2. in-scope but not-complete surface,
3. deferred current-version surface,
4. scenario families being used primarily to pressure upstream seam closure.

Initial practical rule:
1. the first comparison and replay spines should avoid using `W051` rows as baseline product claims unless the scenario is explicitly marked as provisional or upstream-pressure driven,
2. help and completion should still be able to show admitted metadata for those rows, but the app should not present them as settled parity,
3. `function-phase-complete` rows should be labeled `supported`,
4. `W051` rows should be labeled `preview` or `experimental` according to the OxFunc labeling policy,
5. `W050` rows should be labeled `deferred`,
6. remaining `catalog_only` rows should be labeled `catalog_only`,
7. OneCalc should derive those labels by joining the snapshot export against `W050` and `W051`, not by reading the export alone.

Product-behavior rule:
1. `supported` rows get normal help, completion, evaluation, and promoted-scenario eligibility,
2. `preview` rows remain evaluable on the admitted slice, carry a visible preview label, and may appear in ordinary completion, but they are not part of default promoted/product-claim scenarios unless explicitly marked,
3. `experimental` rows remain visible and evaluable where the runtime path exists, carry a visible experimental label and gap kind, and are not part of default promoted/product-claim scenarios,
4. `deferred` rows remain visible in help/browser surfaces but should produce a clear host-level not-available-in-current-scope outcome rather than a silent failure,
5. `catalog_only` rows remain discovery/catalog surfaces only and are not treated as evaluable,
6. `deferred` and `catalog_only` rows should be lower-priority or hidden in ordinary completion unless the user explicitly asks to see non-current surfaces.

Current upstream reference slice for the host-profile ladder:
1. `..\\..\\OxFml\docs\spec\OXFML_DNA_ONECALC_DOWNSTREAM_CONSUMER_CONTRACT.md`,
2. `..\\..\\OxFml\docs\spec\OXFML_HOST_RUNTIME_AND_EXTERNAL_REQUIREMENTS.md`,
3. `..\\..\\OxFml\docs\spec\OXFML_DNA_ONECALC_HOST_POLICY_BASELINE.md`,
4. `..\\..\\OxFml\docs\spec\OXFML_PUBLIC_API_AND_RUNTIME_SERVICE_SKETCH.md`,
5. `..\\..\\OxFml\docs\spec\formula-language\OXFML_OXFUNC_LIBRARY_CONTEXT_RUNTIME_INTERFACE.md`,
6. `..\\..\\OxFunc\docs\function-lane\OXFUNC_DOWNSTREAM_METADATA_AND_HELP_CONTRACT.md`,
7. `..\\..\\OxFunc\docs\function-lane\OXFUNC_SURFACE_ADMISSION_AND_LABELING_POLICY.md`,
8. `..\\..\\OxFunc\docs\function-lane\FUNCTION_SLICE_TYPED_CONTEXT_AND_QUERY_BUNDLE_CONTRACT_PRELIM.md`,
9. `..\\..\\OxFunc\docs\function-lane\FUNCTION_SLICE_RETURN_SURFACE_AND_PUBLICATION_HINT_CONTRACT_PRELIM.md`,
10. `..\\..\\OxCalc\docs\spec\core-engine\CORE_ENGINE_DOWNSTREAM_HOST_SEAM_REFERENCE.md`,
11. `..\\..\\OxCalc\docs\spec\core-engine\CORE_ENGINE_OXFML_SEAM.md`,
12. `..\\..\\OxCalc\docs\spec\core-engine\CORE_ENGINE_OXFML_MINIMAL_UPSTREAM_HOST_INTERFACES.md`.

## 8. Formatting And Conditional Formatting Plane
Formatting and conditional formatting are a good fit for this project and should be treated as first-class host scope.

But they must be staged honestly.

### 8.1 Why They Belong Here
They fit because `DNA OneCalc` is:
1. a single isolated calculation host,
2. an effective-display proving surface,
3. a replay-visible comparison surface,
4. a good place to validate formatting behavior without inheriting workbook-graph complexity.

### 8.2 Authority Split
1. `OxFml` remains authoritative for semantic formatting, formatting-sensitive evaluator behavior, and conditional-formatting formula carriers where those are formula-significant.
2. `DNA OneCalc` owns persisted style state, carrier records, rendering, effective-format computation in the product host, and honest cross-platform capability declaration.
3. `OxReplay` must see formatting-significant and conditional-formatting-significant consequences.
4. `OxXlPlay` is the empirical comparison source for Excel-facing formatting and conditional-formatting truth on Windows.

Current carrier split:
1. hosts own conditional-formatting and data-validation carrier records, target-range attachment, rule fields, and rendering policy,
2. `OxFml` owns admission, restriction classification, and the formula-semantic meaning of the currently modeled host fields for `CF` and `DV` carriers,
3. the current OxFml floor treats `CF` and `DV` as distinct restricted carrier profiles rather than as ordinary worksheet-cell formulas.

### 8.3 Carriage Vs Promoted Parity Claim
`DNA OneCalc` should distinguish:
1. what it can carry and persist,
2. what it can render,
3. what it can claim as promoted comparison truth.

Working rule:
1. full format-string text should be carried and round-tripped from the first serious persistence slice,
2. host style state should be carried distinctly from evaluator-returned presentation hints,
3. full conditional-formatting rule carriage is a target direction for the product,
4. promoted execution and comparison claims should still widen in subsets justified by upstream semantics and retained evidence.

### 8.4 Returned Presentation Hint Vs Host Style State
The host should treat these as separate planes, but the main display-composition rule must match Excel behavior rather than host preference.

Current working composition model:
1. start from the returned value surface,
2. apply evaluator-returned presentation or publication hints from `OxFml` and `OxFunc`,
3. compose persisted host style state such as format string, font properties, and colors,
4. apply conditional-formatting rule consequences,
5. compute effective display for rendering and comparison.

Verification rule:
1. Excel behavior is normative for the final composition rule,
2. the working model above is the current engineering starting point,
3. promoted display claims must be verified empirically through `OxXlPlay` and retained evidence,
4. if the working model and Excel evidence diverge, the spec must be corrected.

Inspection rule:
1. no host render path should silently collapse these planes into one undifferentiated display blob,
2. the X-Ray and comparison surfaces should keep the planes separately inspectable.

### 8.5 First Promoted Formatting Subsets
The first promoted formatting proving set should include:
1. base number, percent, scientific, text-literal, and date or time format-string interpretation where upstream semantics already admit it,
2. locale-sensitive and date-system-sensitive effective display,
3. an honest cross-platform font subset consisting of face, size, bold, italic, underline, and foreground or background colors where the runtime can render them consistently,
4. presentation-aware value classes such as date/time and hyperlink-adjacent display consequences.

The first promoted conditional-formatting proving set should include:
1. restricted-carrier formula-expression rules admitted by the current OxFml `CF` floor,
2. scalar comparison rule families local to one isolated instance, including `=`, `<>`, `>`, `>=`, `<`, `<=`, and `between`,
3. blank and nonblank rules,
4. error and non-error rules,
5. text predicates such as contains, begins-with, and ends-with,
6. consequence actions including fill color, font color, bold, italic, underline, simple border changes, number-format override, and local icon-set outcomes where thresholds and icon family are explicit.

Broader carriage and rendering still belong in scope, but not as promoted Excel-parity claims:
1. richer icon-set families,
2. data bars,
3. two-color and three-color scales.

Current conservative upstream floor:
1. presentation-aware return hints already matter for a narrow but real slice such as `NOW`, `TODAY`, and `HYPERLINK`,
2. the current OxFml `CF` / `DV` floor rejects union, intersection, spill-reference, and external-reference families,
3. broader structured-reference, table-aware, workbook-global ranking, multi-range priority/stop-if-true graph behavior, and broader `MS-OE376` parity for `CF` / `DV` sit outside the current promoted-parity claim set,
4. `DNA OneCalc` should therefore start with explicit restricted-carrier scenarios and widen only as upstream semantics and evidence justify it.

Important rule:
1. formatting and conditional formatting do not imply workbook dependency graphs,
2. they apply to isolated instances and their result or input display surfaces,
3. broader workbook precedence systems remain outside current scope,
4. all promoted formatting and conditional-formatting parity claims must be backed by retained Excel evidence rather than host preference.

Current upstream reference slice for this plane:
1. `..\\..\\OxFml\docs\spec\formatting\EXCEL_FORMATTING_HIERARCHY_AND_VISIBILITY_MODEL.md`,
2. `..\\..\\OxFml\docs\spec\OXFML_HOST_RUNTIME_AND_EXTERNAL_REQUIREMENTS.md`,
3. `..\\..\\OxFml\docs\spec\OXFML_DNA_ONECALC_HOST_POLICY_BASELINE.md`,
4. `..\\..\\OxFunc\docs\function-lane\FUNCTION_SLICE_RETURN_SURFACE_AND_PUBLICATION_HINT_CONTRACT_PRELIM.md`,
5. `..\\..\\OxXlPlay\docs\spec\OXXLPLAY_BUNDLE_EMISSION_AND_HANDOFF_MODEL.md`,
6. `..\\..\\OxXlPlay\docs\spec\OXXLPLAY_SCENARIO_REGISTER.md`.

## 9. UI, Runtime, And Platform Model
### 9.1 Runtime Shape
The intended runtime split is:
1. shared `Leptos` UI and state model,
2. Tauri desktop shell for Windows and Linux,
3. browser/WASM host over the same shared application core,
4. optional non-UI harness later if justified.

Important rule:
1. Tauri is not the web host,
2. desktop and browser are separate hosts over a shared core.

### 9.1.1 Platform Honesty Matrix
| Host | Shared UI Core | Live Excel Compare | Native Extensions | OxVba-sensitive Host Work |
|---|---|---|---|---|
| Windows desktop | `Leptos` + Tauri | Yes, Windows-only through `OxXlPlay` | Yes | Later, Windows-first |
| Linux desktop | `Leptos` + Tauri | No live Excel | Yes | Only if explicitly supported later |
| Browser/WASM | `Leptos` web host | No | No native add-ins | No |

### 9.1.2 Internal Application Strata
The first engineering realization should keep these internal strata explicit:
1. `Host shell`
   responsibilities:
   windowing, menus, filesystem entry points, platform capability discovery, native-extension loading, and browser or desktop host glue.
2. `Workbench app core`
   responsibilities:
   scenario lifecycle, command routing, mode switching, artifact creation, and orchestration of editor, execution, replay, comparison, and handoff flows.
3. `Formula execution facade`
   responsibilities:
   adapt OneCalc host packets to OxFml and OxFunc runtime surfaces, execute runs, and normalize returned value surfaces.
4. `Replay and comparison facade`
   responsibilities:
   validate replay artifacts, invoke replay, diff, explain, distill where available, and merge OxXlPlay evidence into comparison workflows.
5. `Persistence facade`
   responsibilities:
   map `Document` and related artifacts to the declared persistence format, perform round-trip loading and saving, and preserve artifact identity.
6. `Extension facade`
   responsibilities:
   manage native extension discovery, enablement, ABI validation, and registered-external provider bridging.
7. `Evidence store`
   responsibilities:
   retain local artifacts, indexes, attachments, and derived views that should survive process restarts.

Rule:
1. UI state and components should talk to facades and state stores,
2. only the facades should talk directly to upstream libraries or host-specific adapters.

### 9.1.3 Core Service Boundaries
The first serious implementation should expose these internal service contracts:
1. `ScenarioService`
   inputs:
   scenario edits, document mutations, fork or duplicate requests.
   outputs:
   canonical `Scenario` records and document-index updates.
2. `ExecutionService`
   inputs:
   `Scenario`, host packet, runtime environment block.
   outputs:
   `ScenarioRun`, result-surface block, execution diagnostics, replay-capture refs.
3. `EditorService`
   inputs:
   formula text mutations, cursor context, current snapshot metadata.
   outputs:
   edit result, diagnostics, completion proposals, validated completion results, signature/help requests.
4. `ReplayService`
   inputs:
   retained artifact refs and replay commands.
   outputs:
   replay validation result, replay view, diff, explain, distill result where available.
5. `ComparisonService`
   inputs:
   `ScenarioRun`, `Observation`, comparison envelope selection.
   outputs:
   `Comparison`, reliability block, mismatch table, explanation refs.
6. `HandoffService`
   inputs:
   source artifacts, requested action, target lane.
   outputs:
   `HandoffPacket`, readiness warnings, export payload.
7. `PersistenceService`
   inputs:
   document-save and load requests.
   outputs:
   persisted `Document`, attachment set, load diagnostics.
8. `ExtensionService`
   inputs:
   extension discovery requests, enable or disable commands, registration actions.
   outputs:
   extension-state snapshot, provider registration result, ABI validation result.

### 9.1.4 UI State Model
The first engineering state model should keep at least these domains separate:
1. `WorkbenchState`
   current scenario id, current mode, selected panel, dirty state, active commands.
2. `EditorState`
   formula buffer, cursor or selection, IME state, diagnostics view, completion popup state, signature-help state.
3. `ExecutionState`
   current host profile, current packet kind, latest run status, latest result summary.
4. `ReplayState`
   selected artifact, replay-validation status, diff availability, explain availability, distill availability.
5. `ComparisonState`
   attached observation, comparison envelope, reliability labels, mismatch filters.
6. `LibraryState`
   corpus filters, saved views, selection detail, lineage trail.
7. `PersistenceState`
   current document ref, save status, attachment status, load warnings.
8. `ExtensionState`
   platform support, discovered extensions, enabled set, provider warnings.

Rule:
1. view-local UI state may be ephemeral,
2. artifact-bearing state must always be serializable back into the canonical artifact spine or document envelope where it is meant to persist.

### 9.1.5 Verification And Acceptance Surfaces
The first engineering implementation should maintain explicit verification layers:
1. service-level tests for `ScenarioService`, `ExecutionService`, `ReplayService`, `ComparisonService`, and `PersistenceService`,
2. seam and fixture-host tests that pin the admitted upstream contracts and current snapshot-backed host facts,
3. snapshot or golden tests for canonical artifact JSON, X-Ray projections, capability snapshots, and persisted `SpreadsheetML 2003` mappings,
4. UI interaction and layout tests for editor basics, keyboard flow, resize persistence, scrolling behavior, compare view, and handoff readiness checks,
5. retained scenario and integration tests that exercise at least one H0 path, one H1 path, one explorer path, one X-Ray path, one replay path, and one Windows-only comparison path where available,
6. platform smoke tests for Windows desktop, Linux desktop, and browser or WASM host,
7. development scaffolding that keeps deterministic scenario corpora and promoted regression families runnable as ordinary local checks.

### 9.1.6 Host Acceptance Matrix
Host acceptance should use a shared-core gate plus host-specific mandatory additions.

Shared core, mandatory on all hosts:
1. formula edit, parse, diagnose, and run for the primary promoted scenario spine,
2. deterministic re-run and forced re-run behavior,
3. replay capture and retained `ScenarioRun`,
4. replay open/diff/explain over retained artifacts where the declared lane floor supports it,
5. persistence round-trip for one-instance-per-file `SpreadsheetML 2003`,
6. formatting/effective-display for the promoted subset,
7. conditional-formatting for the promoted subset,
8. clear status/header truth and keyboard-usable main flows.

Windows desktop mandatory additions:
1. live `OxXlPlay` compare workflow for the first comparison envelope,
2. provenance/reliability labeling for live Excel observations,
3. version-to-version scenario replay/compare,
4. native add-in loading for the admitted Excel-C-API subset,
5. `.xll` lifecycle and registration path,
6. RTD lifecycle for the admitted in-process COM server subset.

Linux desktop mandatory additions:
1. no claim of live Excel comparison,
2. retained Windows-captured observation consumption works,
3. `.so` native add-in loading for the admitted ABI subset,
4. version-to-version replay/compare works,
5. the declared Linux RTD path either works for the admitted design or is explicitly outside the current host claim.

Browser/WASM mandatory additions:
1. no claim of native add-ins,
2. no claim of live Excel comparison,
3. formula workbench, persistence, retained replay, retained comparison, and evidence browsing all work,
4. opening retained Windows-captured observations works,
5. all unsupported host capabilities are visibly gated rather than merely absent.

Acceptance rule:
1. a host is accepted only if it passes the shared core plus its host-specific mandatory items,
2. a host may not borrow acceptance from another host's stronger capability set.

### 9.2 Leptos Position
`Leptos` is the chosen UI framework for this app.

It should also be treated as:
1. a deliberate proving lane for the program,
2. something to validate with explicit evidence,
3. not an unquestioned premise.

This project should therefore produce:
1. proof-of-life evidence,
2. keyboard and IME viability evidence,
3. WASM and runtime-size evidence,
4. explicit escalation criteria if the stack does not hold up.

The proving exit criteria for the first `Leptos` wave should include:
1. editor interactions that are stable under real formula-edit traffic,
2. keyboard navigation and command flow that remain first-class rather than mouse-first,
3. acceptable latency for live diagnostics and editor updates,
4. a browser/WASM build that proves the shared-core architecture honestly,
5. an explicit escape-hatch rule if the chosen UI stack cannot meet these requirements.

### 9.3 UX Priorities
1. immediate parse and bind diagnostics,
2. live error highlighting,
3. keyboard-first command flow,
4. completion and suggestion help where deterministic local truth exists,
5. function and argument help during editing,
6. result surfaces that make value shape, type, and effective formatting obvious,
7. visible host profile and extension state,
8. visible replay state,
9. first-class X-Ray views,
10. fast scenario capture and inspection.

### 9.4 Formula Editing Language-Service Integration
`DNA OneCalc` should explicitly exercise and integrate the OxFml language-service surface as part of the product scope.

That means the app should consume, not re-invent:
1. immutable formula-edit request and result flows,
2. live diagnostics and squiggle-ready spans,
3. deterministic completion proposals,
4. validated completion application that re-enters the ordinary parse/bind path,
5. signature-help context,
6. function-help and argument-help surfaces where the upstream payloads exist,
7. intelligent-completion context and validation boundaries for later external completion lanes.

Working rules:
1. `DNA OneCalc` may add presentation, interaction, and command affordances, but it should not invent a second parser/binder/editor truth locally,
2. diagnostics should remain OxFml-derived wherever the canonical meaning lives in OxFml,
3. function-help content should come from OxFunc through OxFml packetization rather than duplicated host prose,
4. intelligent completion remains host-owned and non-canonical until it re-enters OxFml through the normal validation path,
5. integration-ready editor packet surfaces should be consumed from the OxFml floor exactly where OxFml now marks them integration-ready,
6. help and signature payloads should be treated as metadata-backed and nullable until OxFunc publishes structured help fields rather than being filled in with host-invented prose.

Host-local editor state is still allowed, but only for presentation and interaction concerns such as:
1. cursor and selection state,
2. IME and composition state,
3. popup visibility and navigation state,
4. ephemeral cached render fragments,
5. command routing and undo/redo interaction state.

`DNA OneCalc` should never locally own:
1. canonical parse or bind truth,
2. canonical diagnostic meaning,
3. canonical completion validity,
4. canonical function or signature help payload truth.

Current practical read:
1. the first useful help and completion metadata path is the OxFunc library-context snapshot export plus its stable ids, arity, category, determinism or volatility or host-interaction classification, seam-category fields, and metadata-status fields,
2. that export is a stabilization artifact rather than the frozen long-term cross-repo ABI,
3. OxFunc now explicitly classifies export fields as `stable`, `usable-but-provisional`, or `current-tree-hint-only`,
4. the preferred first payload shape for function help, argument help, and signature metadata is now explicit in the OxFunc downstream metadata contract, with prose help and formatted signature fields still nullable,
5. the preferred longer-term direction is the OxFunc runtime provider and immutable snapshot model, not permanent CSV-only ingestion,
6. `DNA OneCalc` should therefore be designed to consume an immutable snapshot-shaped help/catalog source even if the first implementation is export-backed.

Current upstream reference slice for formula editing:
1. `..\\..\\OxFml\docs\spec\OXFML_DNA_ONECALC_DOWNSTREAM_CONSUMER_CONTRACT.md`,
2. `..\\..\\OxFml\docs\spec\formula-language\OXFML_EDITOR_LANGUAGE_SERVICE_AND_HOST_INTEGRATION_PLAN.md`,
3. `..\\..\\OxFml\docs\spec\OXFML_HOST_RUNTIME_AND_EXTERNAL_REQUIREMENTS.md`,
4. `..\\..\\OxFml\docs\spec\OXFML_PUBLIC_API_AND_RUNTIME_SERVICE_SKETCH.md`,
5. `..\\..\\OxFunc\docs\function-lane\OXFUNC_DOWNSTREAM_METADATA_AND_HELP_CONTRACT.md`,
6. `..\\..\\OxFunc\docs\function-lane\OXFUNC_SURFACE_ADMISSION_AND_LABELING_POLICY.md`,
7. `..\\..\\OxFunc\docs\function-lane\OXFUNC_LIBRARY_CONTEXT_SNAPSHOT_EXPORT_V1_README.md`,
8. `..\\..\\OxFunc\docs\function-lane\FUNCTION_SLICE_TYPED_CONTEXT_AND_QUERY_BUNDLE_CONTRACT_PRELIM.md`,
9. `..\\..\\OxFunc\docs\function-lane\FUNCTION_SLICE_RETURN_SURFACE_AND_PUBLICATION_HINT_CONTRACT_PRELIM.md`.

### 9.5 Multi-Instance Rule
The app may eventually show multiple isolated instances at once.

Initial rule:
1. instances remain semantically isolated,
2. there are no inter-instance references,
3. there is no shared recalc graph,
4. copy/paste and file-level management are allowed.

### 9.6 First-Draft Product Design Overview
The first-draft UI design should follow the imported `DNA OneCalc Design Pack` preserved under:
1. `research/runs/20260329-011500-dna-onecalc-design-pack-integration-pass-01/inputs/design_pack/DNA_OneCalc_Design_Pack.md`,
2. `research/runs/20260329-011500-dna-onecalc-design-pack-integration-pass-01/inputs/design_pack/extracted/DNA_OneCalc_Design_Pack/assets/`.

That design pack is now adopted here as the first draft product-design overview, with the terminology reconciled against the current upstream seam and status vocabulary.

#### 9.6.1 Shell And Information Architecture
The top-level product areas should be:
1. `Workbench` for authoring, running, comparing, replaying, explaining, retaining, and drafting handoffs,
2. `Scenario Library` for browsing the retained evidence corpus,
3. `Document / Instance Manager` for isolated persisted instances without implying workbook semantics,
4. `Environment / Capability Center` for host-profile, admitted-surface, replay-floor, platform-gate, and extension-state truth.

The artifact lineage should be made visually explicit in the product shell:
1. `Document`
2. `Scenario`
3. `Run`
4. `Comparison`
5. `Witness`
6. `Handoff`

The main workbench shell should keep these regions visible:
1. a context bar,
2. a library or lineage rail,
3. an authoring pane,
4. a host-driving or host-context pane,
5. a result and status pane,
6. a durable X-Ray or Compare drawer.

The core rule from the design pack is adopted: the semantic surfaces must stay in the main product shell, not hidden in a separate developer-only area.

#### 9.6.2 Screen Inventory And Priority
P0 surfaces:
1. `Workbench`
2. `X-Ray drawer`
3. `Result and host-context surfaces inside the Workbench`

P1 surfaces:
1. `Compare view`
2. `Scenario Library`
3. `Environment / Capability Center`
4. `Handoff Review`
5. `Witness detail`

P2 surface:
1. `Document / Instance Manager`
2. `Observation import / attach`
3. `Extension center`

Working interpretation:
1. the workbench remains explorer-first,
2. X-Ray is the second perspective over the same scenario rather than a detached developer tool,
3. compare and replay are first-class product lanes but follow explorer and X-Ray stabilization,
4. the capability center is a supporting honesty surface rather than the main workbench experience,
5. the library is an evidence corpus view rather than a recent-files browser.

#### 9.6.3 Core UX Flows
The first promoted flows should be:
1. author and explore a formula or function,
2. diagnose an error,
3. inspect help, result, and effective display behavior,
4. open X-Ray and inspect the current mechanisms,
5. retain and reopen prior evidence,
6. compare with Excel,
7. explain and retain witness,
8. emit handoff,
9. export or import a scenario capsule.

The design implication is that `Scenario`, `Comparison`, `Witness`, and `Handoff` are all ordinary product objects, not developer-only artifacts.

#### 9.6.4 Interaction And Command Model
The first serious command surface should include:
1. `Run / Re-run`
2. `Open or close X-Ray`
3. `Switch mode`
4. `Open compare`
5. `Open replay`
6. `Open diff`
7. `Open explain`
8. `Retain witness`
9. `Draft handoff`
10. `Export scenario capsule`
11. `Import scenario capsule`
12. `Fork scenario`
13. `Open capability center`

Interaction rules adopted from the design pack:
1. the formula editor is the default focus target,
2. diagnostics, completion, compare filters, library filters, and handoff review must all be keyboard-usable,
3. the deep inspection area should behave like a durable drawer or tabbed panel rather than a fragile transient popover,
4. compare and handoff should feel like modes of the same workbench rather than unrelated routes.

#### 9.6.5 Always-Visible Truth And Status System
The context bar and comparison headers should keep the following interpretation-critical truth visible:
1. current host profile (`OC-H0`, `OC-H1`, `OC-H2`),
2. function-surface admission label (`supported`, `preview`, `experimental`, `deferred`, `catalog_only`) where function-surface truth matters,
3. scenario-family or product-surface maturity (`promoted`, `provisional`, `deferred`) where higher-level product maturity matters,
4. replay capability floor (`C0` through `C3` for current honest use),
5. source type (`DNA`, `Excel retained`, `Excel live`),
6. projection or reliability state (`direct`, `derived`, `lossy`, `provisional`),
7. platform gating (`Windows-only`, `desktop only`, `browser limited`),
8. extension state (`not in this host`, `declared but unavailable`, `enabled`),
9. observation caveats (`capture-loss`, `uncertainty`, `unavailable`),
10. run state (`dirty`, `ready`, `ran`, `compared`).

Vocabulary reconciliation rule:
1. the OxFunc admission labels govern function-surface truth,
2. `promoted` / `provisional` / `deferred` govern higher-level scenario-family or product-surface maturity,
3. `direct` / `derived` / `lossy` / `unavailable` govern comparison-input and observation reliability,
4. the UI must not collapse those three vocabularies into one generic status badge.

#### 9.6.6 Wireframe Consequences
The imported wireframes establish the following first-draft layout consequences:
1. the workbench should visibly separate formula editing, host-driving context, result status, and X-Ray inspection,
2. the compare view should show DNA and Excel sources side by side with explicit reliability and platform qualifiers in the header,
3. the scenario library should provide persistent filters by host profile, surface status, replay status, witness presence, handoff presence, and platform relevance,
4. the handoff review should pull scenario, run, comparison, witness, replay-floor, and source-projection context through automatically and validate readiness before export.

#### 9.6.7 Capability Center
The `Capability Center` should be a persistent product surface, not a hidden developer panel.

It should provide:
1. the active dependency ledger for the current workspace,
2. the effective seam pin set and dependency identities,
3. the active host profile and packet-kind register,
4. the admitted function-surface policy and overlay inputs,
5. the replay floor, observation envelope, and platform gates,
6. the extension ABI and RTD availability state,
7. the list of provisional seams, capability ceilings, and blocked product modes,
8. a diff view against an earlier capability snapshot where available.

Working rules:
1. it must not obscure the primary editor or result surfaces by default,
2. if it is docked, it should preserve resize state and provide its own scrolling behavior,
3. the context bar should show compact capability truth,
4. the `Capability Center` should be the expanded view for that same truth rather than a separate status model,
5. users should be able to copy or export the current capability snapshot for review, handoff generation, and version-to-version comparison,
6. the Capability Center should explain disabled or hidden modes by pointing back to capability facts rather than generic error messages.

## 10. Replay, Comparison, And Scenario Library
Replay, comparison, and scenario-library work are the third major product perspective.
They are not optional garnish, but they are also the highest current design-risk area because the app-facing operation model across `OxReplay`, `OxFml`, `OxFunc`, and host-owned retained actions is not yet fully designed or proven.

The next delivery order should therefore stabilize the explorer and X-Ray surfaces first, then treat replay and observation as the first serious proving ground for retained operation logging, reproducible runs, explain or witness flows, and Excel comparison.

`DNA OneCalc` should be the first user-facing host that routinely makes this possible:
1. author scenario,
2. run scenario,
3. emit retained replay evidence,
4. compare with Excel-facing evidence,
5. explain mismatch,
6. retain witness,
7. emit handoff.

Required directions:
1. `DNA OneCalc -> OxReplay -> compare to Excel/OxXlPlay evidence`
2. `OxXlPlay/Excel capture -> OxReplay -> replay/explain against DNA OneCalc`

Windows-only rule:
1. live Excel-facing comparison is Windows-only,
2. Linux desktop and browser/WASM must not imply live Excel availability.

### 10.0 Workbench Mode Gate Matrix
The first serious UI wave should expose named workbench modes under the following rules.

| Mode | Minimum floor | Platforms | Output floor | Label rule |
|---|---|---|---|---|
| `DNA-only` | `OxFml` + `OxFunc` execution path | All declared hosts | `ScenarioRun` | Never imply Excel parity |
| `Excel-observed` | live `OxXlPlay` capture path | Windows desktop only | `Observation` | Must show Windows-only status |
| `Twin compare` | `ScenarioRun` plus accepted comparison envelope | Windows for live compare; retained artifacts elsewhere | `Comparison` | Must show reliability badge and projection limits |
| `Replay` | accepted OneCalc replay intake surface through `OxReplay` | All hosts that can read retained artifacts | validated replay view | Must show capability floor relied upon |
| `Diff` | typed diff surface and comparable artifacts | Same as replay-capable hosts | diff artifact | Must show lossy or provisional inputs |
| `Explain` | current honest floor at least through explain-capable lane adapter surface | Same as replay-capable hosts | explain artifact | Must show if explanation is lane-limited |
| `Distill` | explicit distillation-capable floor for the active lane and artifact family | Only where the active lane floor supports it honestly | witness reduction artifact | Hide or mark experimental until the floor is real |
| `Handoff` | lineage-complete scenario, run, and supporting artifacts | All hosts | `HandoffPacket` | Must include exact provisional seam pins and capability floor |

### 10.1 Current Conservative Comparison And Replay Floor
`DNA OneCalc` should assume the following current honest floor unless retained upstream evidence says otherwise:
1. `OxFml` replay support is currently honest through `C3.explain_valid`; do not assume `C4.distill_valid` or `C5.pack_valid`,
2. `OxFunc` has useful local replay artifacts and manifests, but no accepted direct `OxReplay` intake floor that OneCalc should depend on separately from `OxFml`,
3. `OxXlPlay` now has a dedicated OneCalc observation-consumer contract, but the live exercised observation family is still narrow,
4. the first honest comparison-ready observation family is still `xlplay_capture_values_formulae_001`, with direct cell-value and formula-text comparison only,
5. `OxXlPlay` surfaces must be treated as `direct`, `derived`, or `unavailable`, and `unavailable` surfaces are not comparison-eligible,
6. the current `OxXlPlay` replay-facing normalized view is explicitly `lossy`,
7. `OxVba` replay-facing consumption falls outside the current scope.

Operational consequence:
1. OneCalc should surface replay capability floors, observation provenance, and projection or lossiness markers directly in the UI and retained artifacts,
2. it should not present Windows Excel comparison or replay distillation maturity as broader than the current retained evidence justifies,
3. it should treat retained version-to-version comparison over the same scenario family as part of the core proving workflow, not as an afterthought.

### 10.2 Comparison Reliability Badge
Every comparison surface shown in the UI or retained in artifacts should carry a reliability badge derived from its current evidence shape:
1. `direct` where the compared surface is directly observed or directly produced with no declared projection loss,
2. `derived` where the compared surface is a declared downstream derivation over retained direct facts,
3. `lossy` where the current retained view explicitly drops or normalizes facts,
4. `provisional` where the compared surface depends on a still-provisional seam or capability floor,
5. `unavailable` is not a reliability badge but a hard not-comparison-eligible state and must remain visibly unavailable with its capture-loss reason.

### 10.3 Scenario Promotion Rule
The first scenario families promoted into comparison and replay spines should favor:
1. OxFunc rows with stable semantic closure or explicit doc-modeled seam contracts,
2. OxFml lanes whose host and replay artifacts are already deterministic and typed,
3. OxXlPlay scenarios with retained provenance-rich bundle emission and no hidden capture assumptions.

Avoid making these the first product-claim families unless the scenario is explicitly marked provisional:
1. `W051` OxFunc rows whose broader promotion packet is still open,
2. broad conditional-formatting or `DV` lanes beyond the current restricted-carrier floor,
3. Excel-comparison claims that depend on the current lossy replay projection as if it were complete semantic equivalence truth.

Required scenario families:
1. formula-language edge cases,
2. function-semantic cases,
3. formatting-sensitive cases,
4. base-formatting and effective-display cases,
5. conditional-formatting cases,
6. host-profile-sensitive cases,
7. later extension cases.

Promotion order rule:
1. the first promoted proving spine is formula-core semantic and replay behavior,
2. the second promoted proving spine is the Windows-only twin-oracle lane,
3. the third promoted proving spine is language-service behavior once the editor surfaces are retained and comparable enough to stand on their own.

The scenario library must also produce:
1. structured requirement deltas,
2. seam clarification requests,
3. repo-addressable upstream work requests,
4. concrete widening requests for `OxXlPlay` whenever OneCalc comparison design outruns the currently exercised observation envelope.

### 10.4 Scenario Capsule Transport Rule
Every serious retained scenario family should be exportable and importable as a `ScenarioCapsule`.

The capsule transport should be used when:
1. handing evidence to upstream repo teams,
2. moving retained replay and comparison assets between OneCalc workspaces,
3. preserving cross-version comparison sets,
4. retaining a portable evidence bundle for later repro or review.

The first honest capsule scope should include:
1. one authored `Scenario`,
2. selected retained `ScenarioRun` artifacts,
3. selected `Observation`, `Comparison`, `Witness`, and `HandoffPacket` artifacts where present,
4. replay bundles and observation bundles by ref or included payload,
5. the governing capability snapshots,
6. a manifest with stable hashes and lineage roots.

Working rules:
1. capsule export must not rewrite logical ids,
2. capsule intake must preserve existing ids and attachment structure where possible,
3. capsule export is distinct from document save and from SpreadsheetML persistence,
4. capsule import should enrich the local evidence corpus without silently mutating the authored scenario truth.

Current upstream reference slice for replay, comparison, and scenario growth:
1. `..\\..\\OxReplay\docs\spec\OXREPLAY_DNA_ONECALC_CONSUMPTION_MODEL.md`,
2. `..\\..\\OxReplay\docs\spec\OXREPLAY_ADAPTER_AND_CONFORMANCE_MODEL.md`,
3. `..\\..\\OxReplay\docs\spec\OXREPLAY_BUNDLE_WITNESS_AND_REGISTRY_MODEL.md`,
4. `..\\..\\OxReplay\docs\spec\DNA_RECALC_HOST.md`,
5. `..\\..\\OxReplay\docs\spec\OXREPLAY_OXXLPLAY_OBSERVATION_SEAM.md`,
6. `..\\..\\OxXlPlay\docs\spec\OXXLPLAY_ARCHITECTURE_AND_CAPTURE_MODEL.md`,
7. `..\\..\\OxXlPlay\docs\spec\OXXLPLAY_BUNDLE_EMISSION_AND_HANDOFF_MODEL.md`,
8. `..\\..\\OxXlPlay\docs\spec\OXXLPLAY_SCENARIO_REGISTER.md`,
9. `..\\..\\OxFml\docs\spec\OXFML_REPLAY_APPLIANCE_ADAPTER_V1.md`,
10. `..\\..\\OxFml\docs\spec\fec-f3e\FEC_F3E_TESTING_AND_REPLAY.md`,
11. `..\\..\\OxFunc\docs\function-lane\OXFUNC_REPLAY_APPLIANCE_PACKET_ADAPTER_V1.md`.

## 11. Persistence
The initial externally meaningful persistence target is `SpreadsheetML 2003`.

Why:
1. it is simpler than OOXML,
2. it is externally meaningful,
3. it matches the Foundation reference direction.

Working rules:
1. one XML file means one isolated `DNA OneCalc` instance,
2. the workbook envelope is an Excel-readable container rather than workbook semantic authority,
3. it is not permission to introduce workbook graph semantics,
4. formatting state and conditional-formatting state must round-trip,
5. XML extension lanes may be used where they are safe and where Excel will harmlessly ignore them,
6. the top-level host UI may manage multiple isolated instances by opening a directory or workspace of OneCalc files.

Important rule:
1. `SpreadsheetML 2003` is the first persistence target,
2. it is not necessarily the only long-term persisted truth artifact.
3. `ScenarioCapsule` is a separate evidence-transport format and must not be conflated with document persistence.

Initial mapping decision:
1. the first honest persistence mapping should use one worksheet as the isolated scenario container for one OneCalc file,
2. the worksheet is a persistence envelope for that isolated instance, not a claim of workbook semantics,
3. workspace grouping across multiple files belongs in the host UI rather than in the current XML-file meaning,
4. if a stronger alternative is ever adopted, it must be justified explicitly against this default.

Current upstream-reference note:
1. there is currently no stable `Ox*`-owned persistence contract for the isolated-instance `SpreadsheetML 2003` mapping,
2. until one exists, persistence design should treat the `Ox*` repos as semantic-input owners only and treat the actual container mapping as a `DnaOneCalc` responsibility informed by the Foundation reference corpus and the public `SpreadsheetML 2003` sources already curated there,
3. this documentation gap is recorded explicitly again in Section `19.8`.

### 11.1 First Persisted Document Shape
The first serious persisted document should minimally carry:
1. document identity and metadata,
2. instance manifest,
3. per-instance `Scenario` block,
4. optional retained artifact index for `ScenarioRun`, `Comparison`, `Witness`, and `HandoffPacket`,
5. attachment map,
6. view-state block for reopening the workbench,
7. persistence-format metadata.

Rule:
1. persisted UI state must remain clearly separate from semantic truth,
2. retained artifacts may be embedded or externally attached, but the document must always know which is which.

### 11.2 SpreadsheetML Mapping Detail
The first `SpreadsheetML 2003` mapping should treat the worksheet as:
1. one isolated scenario container,
2. one formula authoring surface,
3. one set of host-driving, display, and retained-local-state fields,
4. one retained local metadata block for artifact refs and host-profile truth.

The engineering mapping should keep these planes distinct:
1. worksheet visible content,
2. worksheet-level metadata or custom properties used for OneCalc identity, lineage, and attachment refs,
3. workbook-level metadata used only for one-file container identity and Excel-tolerant extension lanes.

Hard rule:
1. no saved XML file may imply cross-instance recalc or reference-sharing semantics,
2. workbook-level storage is a container convenience only.

### 11.3 Persistence Round-Trip Invariants
The first persistence implementation should preserve:
1. formula text,
2. host profile id,
3. host-driving and recalc metadata,
4. persistence-format metadata and attachment refs,
5. base formatting state,
6. conditional-formatting rule carriage for the admitted first subset,
7. retained artifact refs,
8. document or scenario ids.

If any of those cannot round-trip through the first persistence mapping:
1. the loss must be explicit,
2. the feature must remain provisional,
3. the saved artifact must record the projection or lossiness.

### 11.4 Scenario Capsule Export And Intake
`ScenarioCapsule` is the first portable evidence-sharing format for OneCalc scenarios.

It is intentionally separate from `SpreadsheetML 2003` persistence:
1. `SpreadsheetML 2003` persists one isolated OneCalc instance in an Excel-readable envelope,
2. `ScenarioCapsule` transports one scenario together with selected retained evidence and attachment structure.

The first engineering implementation should support:
1. exporting a capsule from the current scenario and selected retained artifacts,
2. importing a capsule into the local evidence store and workspace,
3. preserving stable ids, hashes, and capability-snapshot refs,
4. preserving lineage and attachment indexes,
5. validating capsule manifests before intake.

Hard rule:
1. capsule export and intake must not silently alter semantic truth,
2. imported capsules must remain visibly imported evidence until explicitly incorporated into local authored work,
3. capsule transport must remain folder-based and inspectable rather than opaque.

## 12. Extension And Add-In Model
The desktop extension path should be defined as a portable C ABI contract.

Delivery-order rule:
1. this lane remains in scope,
2. but it follows explorer stabilization, X-Ray stabilization, and the first replay or observation proving work,
3. because its desired operation is clearer than the replay lane even though its technical implementation complexity is high.

The admitted subset should be defined against the public Excel SDK reference corpus already curated in Foundation `reference/`, so that OneCalc implements a precise public-source subset rather than a loosely Excel-like add-in story.

That means:
1. the portability claim is about the extension ABI,
2. not about literally reusing Windows `.xll` binaries on Linux.

Platform model:
1. Windows desktop uses native `.xll` packaging,
2. Linux desktop uses native `.so` packaging over the same declared extension ABI,
3. hosted web and browser/WASM begin without native add-in support.

This keeps the extension lane honest while preserving the portability goal.

The extension surface should be split explicitly into:
1. `DNA OneCalc` native-extension ABI v0,
2. `DNA OneCalc` RTD host/runtime path,
3. later `OxVba` add-in or toolchain integration as design input and co-development pressure.

`OxVba` role:
1. OxVba is currently best treated as an embedded host runtime and later add-in toolchain, not as an already-shipped add-in producer,
2. `.basproj` already defines `Library` and `Addin` output kinds in a normative-draft project model,
3. current host-export discovery and embedded-host execution are the real current floor,
4. XLL generation is still planned rather than implemented,
5. Linux shared-library support should therefore be pursued first through the OneCalc portable native-extension ABI, not by pretending the OxVba add-in toolchain is already portable and complete.

Platform honesty table:
1. Windows desktop supports the admitted `.xll` packaging over the frozen ABI subset,
2. Linux desktop supports the same admitted ABI through `.so` packaging,
3. browser/WASM should begin with no native extension loading claim at all,
4. `OxVba`-driven add-in generation remains a separate upstream pressure lane until the upstream toolchain is executable and documented as such.

Current upstream reference slice for the extension lane:
1. `..\\..\\OxFml\docs\spec\formula-language\OXFML_REGISTERED_EXTERNAL_PROVIDER_AND_CALL_REGISTER_ID_BOUNDARY.md`,
2. `..\\..\\OxFml\docs\spec\OXFML_HOST_RUNTIME_AND_EXTERNAL_REQUIREMENTS.md`,
3. `..\\..\\OxFunc\docs\function-lane\FUNCTION_SLICE_CALL_REGISTER_ID_UDF_REGISTRATION_SEAM_PRELIM.md`,
4. `..\\..\\OxFunc\docs\function-lane\XLL_ADDIN_BRIDGE_SHIM_CONTRACT_PRELIM.md`,
5. `..\\..\\OxFunc\docs\function-lane\XLL_VERIFICATION_SEAM_LIMITATIONS.md`,
6. `..\\..\\OxVba\docs\spec\BASPROJ_SPEC_V1.md`,
7. `..\\..\\OxVba\docs\spec\HOSTING_PROJECT_TOOLING_PROPOSAL.md`,
8. `..\\..\\OxVba\docs\spec\COM_CLIENT_SERVER_SCOPE_V1.md`,
9. `..\\..\\OxVba\docs\spec\HAL_RUNTIME_PROFILE_MATRIX_V1.md`,
10. `..\\..\\OxVba\docs\IN_PROGRESS_FEATURE_WORKLIST.md`.

Important current limitation:
1. the relevant `OxVba` docs are still mostly draft-grade rather than a frozen downstream contract,
2. `DNA OneCalc` should therefore treat the current `OxVba` surface as design input and co-development pressure rather than as a fully frozen consumer ABI,
3. Windows COM and Office-style root-object hosting remain Windows-only assumptions unless the host supplies explicit cross-platform replacements.

### 12.1 First ABI Contract Shape
The first OneCalc native-extension ABI should minimally define:
1. extension identity and version query,
2. function catalog export,
3. function invocation entry point,
4. registration and unregister lifecycle,
5. capability flags,
6. error or provider-outcome transport,
7. shutdown hook,
8. the admitted Excel-style entry points and host-call subset.

The ABI should be portable at the C boundary and then adapted separately into:
1. Windows `.xll` or related packaging,
2. Linux `.so` packaging,
3. any later OxVba-produced add-in packaging.

The admitted subset is:
1. Excel-SDK-defined lifecycle entry points such as `xlAutoOpen` and `xlAutoClose`,
2. `Excel12(...)` host calls for the admitted subset,
3. `XLOPER12` support only,
4. no legacy `XLOPER`,
5. `xlfRegister` Form 1,
6. `xlfEvaluate`,
7. `xlUDF`,
8. `xlfRtd`,
9. the admitted worksheet-call data types needed for those surfaces,
10. volatility and related registration flags where declared.

### 12.2 Extension Runtime Load Model
The first engineering load model should separate:
1. discovery,
2. signature or ABI validation,
3. enablement,
4. registration into the host-visible provider set,
5. invocation,
6. teardown,
7. platform-specific RTD activation where admitted.

Rule:
1. discovery success does not imply enablement,
2. enablement success does not imply runtime registration success,
3. runtime registration success does not imply semantic admission for every function in product claims.

### 12.3 Extension Surface Safety Rule
The host must preserve three distinct states for any external or add-in-backed function surface:
1. declared by extension or registry,
2. admitted by current seam contract,
3. promoted in product or scenario claims.

That distinction must remain visible in:
1. the extension center,
2. function help and completion,
3. scenario metadata,
4. comparison and handoff artifacts.

Current host rule:
1. host-loaded add-in registration is in scope,
2. worksheet `REGISTER.ID` / `CALL` semantics are not part of the current product scope merely because add-ins are supported,
3. Windows should support the admitted RTD lifecycle for in-process COM servers,
4. Linux should provide a minimal COM-like activation registry and host contract for the admitted RTD server/interface subset,
5. browser/WASM hosts do not support native add-ins.

## 13. Derived Repo Responsibilities
This note is the engineering source from which the `DnaOneCalc` repo should derive its local docs, workset register, and bead graph.

It is not the live execution tracker.

Working rules:
1. keep product, artifact, seam, and platform truth in this note,
2. keep workset truth in the repo-local workset register and keep execution status,
   blockers, and active sequencing in the bead graph,
3. do not multiply narrative planning documents when the main engineering note, one workset register, and `.beads/` already express the needed truth.

### 13.1 Minimal Derived Repo Surfaces
The repo should derive, at minimum:
1. one repo-owned copy or promoted version of this engineering spec,
2. one living `WORKSET_REGISTER.md`,
3. one bead graph under `.beads/`,
4. one concise bead-doctrine note in repo docs or `OPERATIONS.md`,
5. the minimum control artifacts needed to make the first host slice executable.

### 13.2 Minimal Derived Control Artifacts
The following derived control artifacts should exist either as separate files or as stable machine-readable material embedded in repo-owned docs:
1. host-profile matrix,
2. seam manifest and dependency pin set,
3. artifact envelope and identity rules,
4. minimal scenario and handoff field rules,
5. capability-ledger schema once the observatory surface is implemented,
6. capsule-manifest schema once capsule export or intake is implemented.

Rule:
1. do not fan these out into many small policy documents unless implementation pressure makes that necessary,
2. one main spec can carry multiple control surfaces clearly as long as the fields and rules are explicit.

### 13.3 Translation Rule To Repo Planning
When this note is turned into repo-local execution surfaces:
1. derive a coherent ordered workset register from the major capability areas in
   Sections `7` through `12`,
2. treat the register as the owner of workset truth and dependency shape rather than
   as a second execution-status board,
3. execute through `workset -> epic -> bead` in the repo-local bead graph, with some
   epics and beads created directly from the register and some child beads created
   later by rollout epics,
4. keep blockers in the bead graph rather than in a second blockers document,
5. keep this note as design authority rather than as a live execution tracker,
6. avoid reopening scope questions already settled here unless new upstream evidence or explicit spec work forces it.

### 13.4 Major Derived Implementation Areas
The engineering areas that the repo must eventually realize are:
1. host shell, formula/function explorer UX, and formula-language service integration,
2. `OC-H0` execution plus result, effective-display, and promoted formatting surfaces,
3. durable X-Ray, provenance, and mechanism inspection,
4. deterministic test scaffolding, retained scenario corpora, and host acceptance harnesses,
5. driven single-formula host behavior, retained runs, persistence, capability snapshots, and `ScenarioCapsule` transport,
6. replay capture, operation logging, explain/witness/handoff flows, and replay substrate proving work,
7. Windows twin-oracle comparison through `OxXlPlay`,
8. workspace, capability center, scenario library, corpus hardening, and acceptance evidence,
9. desktop extension ABI, add-in loading, RTD, and later `OxVba` pressure.

These areas are design obligations.
Their detailed execution order belongs in the repo-local workset register and bead graph rather than in this engineering specification.

## 14. Current Conservative Upstream Consumption Baseline
This section summarizes the current honest floor that `DNA OneCalc` should design against now.

### 14.1 Host And Evaluator Seam Floor
1. the primary upstream host/runtime contract is now the OxFml downstream-consumer clarification note plus the broader OxFml host/runtime packet and reduced-profile OneCalc supplement,
2. the H0 mandatory field floor now explicitly includes `FormulaSourceRecord`, `formula_channel_kind`, `structure_context_version`, immutable library-context carriage, `LocaleFormatContext`, and deterministic volatile inputs where the active semantic lane requires them,
3. the H1 additions now center on driven single-formula execution, explicit recalc behavior, scriptable formula replacement, `RtdProvider`, display context, and version-to-version retained comparison,
4. the current product scope does not admit `defined_name_bindings`, `HostInfoProvider`, direct reference binding, or worksheet-style host queries,
5. the current OxCalc seam-reference packet is still useful as informative reference material, but `DNA OneCalc` does not exercise the dereference seam in its present scope,
6. `DNA OneCalc` should therefore keep its public model driven, non-grid, and clearly narrower than `OxCalc`.

### 14.2 Function, Catalog, And Help Floor
1. the current downstream catalog and metadata seed is the OxFunc library-context snapshot export,
2. that export is useful and real, but it is a stabilization artifact rather than a final cross-repo ABI,
3. OxFunc now provides an explicit downstream metadata/help contract and a separate surface-admission and labeling policy,
4. the OxFunc current surface must always be read through that contract plus the `W050` deferred overlay and the `W051` in-scope-not-complete overlay,
5. snapshot fields now have declared `stable`, `usable-but-provisional`, and `current-tree-hint-only` tiers,
6. the current first help or signature path is still snapshot-backed and metadata-limited, with prose help, argument descriptions, and formatted signature strings not currently available upstream,
7. OneCalc should label surfaces as `supported`, `preview`, `experimental`, `deferred`, or `catalog_only` rather than inventing a private admission taxonomy,
8. the preferred long-term direction remains a provider-backed immutable snapshot model rather than permanent CSV-shaped integration.

### 14.3 Replay And Excel-Comparison Floor
1. `OxReplay` now has an explicit `DNA OneCalc` consumption model, but `DNA ReCalc` remains the generic replay host,
2. the current honest replay floor for OneCalc is `OxFml` through `C3.explain_valid`, not broad `C4` or `C5`,
3. `OxFunc` does not currently provide a separately accepted direct replay-intake floor that OneCalc should depend on,
4. `OxXlPlay` now has a dedicated OneCalc observation-consumer contract and a real Windows live-driver baseline, but the live exercised surface is still narrow,
5. the first comparison-ready observation envelope is still only direct cell value and direct formula-text comparison for one retained scenario family,
6. the current `OxXlPlay` normalized replay view is explicitly `lossy`, and richer comparison fidelity must consult the source observation bundle and sidecars,
7. live Excel comparison is Windows-only, while retained replay, diff, explain, and observation-artifact consumption may be used on other platforms,
8. the app-facing replay operation model across `OxReplay`, `OxFml`, `OxFunc`, and host-owned retained actions remains a high-risk proving ground rather than a settled infrastructure layer.

### 14.4 Extension And VBA Floor
1. the portable native-extension ABI remains an admitted current scope item for `DNA OneCalc`, but it is not the first stabilization target for the next iteration,
2. this lane is lower risk from a product-definition standpoint because the desired operation is comparatively clear, but it remains high technical complexity and likely upstream seam pressure,
3. Windows `.xll` packaging and Linux `.so` packaging should preserve the same admitted ABI and behavior,
4. the admitted surface freezes `XLOPER12` only and the `Excel12(...)` host-call subset needed for `xlfRegister` Form 1, `xlfEvaluate`, `xlUDF`, and `xlfRtd`,
5. `DNA OneCalc` supports host-loaded function registration and RTD under that admitted subset, without thereby admitting worksheet `REGISTER.ID` / `CALL` semantics,
6. OxVba's real current floor remains embedded host runtime execution with host-provided root objects and partial host-export discovery, so `DNA OneCalc` should treat OxVba as design input and co-development pressure rather than as a shipped add-in toolchain,
7. Windows COM and Office-style root-object hosting remain Windows-only assumptions unless the host explicitly supplies cross-platform replacements,
8. the Linux RTD activation model remains an admitted design task inside the current scope rather than a reason to weaken the frozen ABI direction.

## 15. Start-Now Judgment
`DnaOneCalc` should be started now.

That is honest because:
1. `OxFml` already has a real single-formula host floor,
2. `OxFunc` already has a real library/runtime seam,
3. `OxReplay` is already usable infrastructure,
4. `OxXlPlay` already provides a live Excel evidence lane,
5. `OxCalc` now has an explicit downstream-host seam-reference note,
6. `OxReplay` now has an explicit `DNA OneCalc` consumption model,
7. the missing work is now mostly about host definition, integration, gating, and product shaping rather than waiting for a hypothetical future lane to exist.

What must remain explicit:
1. the next iteration order is explorer first, X-Ray second, replay or observation proving third, and extensions after those floors stabilize,
2. `OxFml` and `OxFunc` seams are usable but not forever-frozen,
3. `DNA OneCalc` must not quietly slide toward `OxCalc`,
4. formatting and conditional formatting belong here, but honest staged delivery still matters,
5. the OxFml editor-language-service floor is already real enough to integrate against, but shared host packet freezing and OxFunc-backed help payload closure are still active seam work,
6. live Excel comparison is Windows-only,
7. hosted web and browser/WASM begin without native add-ins,
8. the current replay and Excel-observation floor is still narrower than a broad parity story and must stay labeled that way,
9. the app-facing replay or operation model is still high risk because it is not yet designed or proven end to end,
10. the extension lane is later work, with lower product-definition risk but higher technical complexity,
11. repo execution should be derived from this spec through a workset register and bead graph rather than by reopening the scope model here.

## 16. Success Criteria
The first serious `DNA OneCalc` scope should be considered real only when:
1. the repo-derived control surfaces required by this spec are published coherently,
2. a formula string can be entered and evaluated through `OxFml` and `OxFunc` against an explicitly pinned admitted function surface,
3. the formula/function explorer is usable, keyboard-first, and stable under ordinary resize and navigation behavior,
4. OxFml-derived diagnostics are visible and trustworthy in the editor,
5. deterministic completion and currently available function or argument help are integrated into the editor flow,
6. base formatting and effective-display state are visible and honest,
7. the X-Ray surfaces expose current parse, bind, evaluation, and provenance truth for the promoted scenario families,
8. deterministic test scaffolding, promoted scenario corpora, and high-coverage integration checks exist for the promoted explorer and X-Ray paths,
9. the host profile is explicit and visible,
10. packet kind, provisionality state, and comparison or replay capability floor are visible in the UI,
11. the capability center can show and export the effective dependency, seam, replay, observation, and extension truth for the current workspace,
12. replay output can be emitted for at least one nontrivial scenario family,
13. replay capture, replay execution, diff, explain, and retained-scenario control are all available through the UI for the current honest lane floor,
14. at least one retained scenario family is validated against Excel-facing evidence through `OxXlPlay` with provenance and lossiness made explicit,
15. at least one retained formatting or conditional-formatting family exists,
16. persisted documents round-trip through the declared initial file format with formatting state intact,
17. one scenario can be exported and re-imported as a `ScenarioCapsule` without losing lineage or capability truth,
18. extension support is either real for the declared desktop host or explicitly out of scope for that host profile.

## 17. Residual Design Pressure
The main scope holes have now been closed in this document.
The remaining pressure points are now concentrated in next-iteration sequencing, replay-risk reduction, and upstream-alignment items.

### 17.1 Current Upstream Pressure Items
1. the final OxFml shared immutable edit-packet freeze,
2. the final OxFunc help and signature payload contract,
3. app-facing `OxReplay` service tightening for non-`DNA ReCalc` hosts,
4. replay artifact-chain closure across `OxFml`, `OxFunc`, and `OxReplay`,
5. widening the OxXlPlay observation/comparison envelope beyond value and formula text,
6. OneCalc-driven seam-sync and naming cleanup where OxFml and OxCalc still describe shared seam intent differently,
7. later extension and RTD seam hardening where OneCalc becomes the first serious downstream proving host.

### 17.2 Current OneCalc-Owned Design Tasks
1. explorer-first workbench UX stabilization, including result visibility, scrolling, and durable resize behavior,
2. the exact durable X-Ray panel and explanation model for current mechanism truth,
3. the replay-aware operation model for undo or redo, semantic logging, macro-recording-style capture, and reproducible runs,
4. deterministic test scaffolding, promoted scenario corpora, and host acceptance automation,
5. the retained-workspace UX for managing multiple isolated OneCalc files together,
6. the exact admitted Excel-SDK subset register and exclusion list for the OneCalc extension ABI,
7. the Linux minimal COM-like activation registry and host contract for the admitted RTD subset,
8. the exact XML extension-lane usage policy for `SpreadsheetML 2003`,
9. the richer internal comparison dimensions once upstream evidence widens.

### 17.3 Explicitly Out Of Current Scope Unless Reopened By New Spec Work
1. broad worksheet-style name or reference binding in OneCalc,
2. the OxCalc/OxFml dereference seam as a OneCalc runtime lane,
3. worksheet `REGISTER.ID` / `CALL` as a OneCalc product lane,
4. broad conditional-formatting parity claims beyond the explicitly admitted subsets,
5. native add-ins in browser/WASM hosts,
6. workbook-graph semantics or cross-instance recalc,
7. full Office-style add-in breadth.

## 18. Immediate Interpretation Rule
If a future repo bootstrap, charter, or work packet conflicts with this document:
1. keep `DNA OneCalc` narrower than `OxCalc`,
2. keep replay and comparison first-class,
3. keep the artifact spine explicit,
4. keep the extension contract honest by platform,
5. keep the product ordered as explorer first, X-Ray second, and replay/compare proving third,
6. prefer one strong engineering spec plus repo-local workset register and bead graph over multiplying narrative planning docs.

## 19. Authoritative Upstream Reference Set
This section records the current upstream document set that `DNA OneCalc` should use for detailed integration design and library usage.

### 19.1 Reference Use Rule
1. Foundation doctrine remains higher-precedence than repo-local restatements where applicable, especially for replay governance and host topology.
2. For each upstream repo, prefer root `CHARTER.md`, root `OPERATIONS.md`, `docs/spec/README.md`, non-archive spec docs named there, and the current `docs/IN_PROGRESS_FEATURE_WORKLIST.md` plus `CURRENT_BLOCKERS.md`.
3. Treat worksets, handoff notes, execution records, test-run notes, and upstream observation ledgers as current-status or evidence artifacts rather than semantic authority unless a repo section below explicitly names them as temporary downstream references.
4. Treat `prelim`, `draft`, `working-draft`, `design-draft`, and similar labels as real constraints on what can honestly be treated as frozen.
5. If a required downstream surface has no good upstream doc, record that as documentation debt rather than inventing a silent local contract.

### 19.2 OxFml
1. `..\\..\\OxFml\CHARTER.md` - lane scope, precedence, and OxFml ownership of evaluator-side seam meaning.
2. `..\\..\\OxFml\OPERATIONS.md` - lane-level closure, pack, and handoff discipline.
3. `..\\..\\OxFml\docs\spec\README.md` - canonical OxFml spec index and archive filter.
4. `..\\..\\OxFml\docs\IN_PROGRESS_FEATURE_WORKLIST.md` - live current-floor and residual-gap register.
5. `..\\..\\OxFml\CURRENT_BLOCKERS.md` - active blocker truth.
6. `..\\..\\OxFml\docs\spec\OXFML_SYSTEM_DESIGN.md` - top-level formula/evaluator architecture and proving-host framing.
7. `..\\..\\OxFml\docs\spec\OXFML_DNA_ONECALC_DOWNSTREAM_CONSUMER_CONTRACT.md` - canonical OneCalc-facing first-integration contract for host subsets, packet taxonomy, language-service readiness, returned-value obligations, and not-authorized surfaces.
8. `..\\..\\OxFml\docs\spec\OXFML_HOST_RUNTIME_AND_EXTERNAL_REQUIREMENTS.md` - primary host/runtime coordination packet.
9. `..\\..\\OxFml\docs\spec\OXFML_DNA_ONECALC_HOST_POLICY_BASELINE.md` - reduced-profile OneCalc companion for the current direct-host floor.
10. `..\\..\\OxFml\docs\spec\OXFML_PUBLIC_API_AND_RUNTIME_SERVICE_SKETCH.md` - supporting implementation-facing host/runtime and returned-value surface sketch.
11. `..\\..\\OxFml\docs\spec\formula-language\OXFML_OXFUNC_LIBRARY_CONTEXT_RUNTIME_INTERFACE.md` - runtime snapshot/provider seam for function-catalog truth.
12. `..\\..\\OxFml\docs\spec\formula-language\OXFML_EDITOR_LANGUAGE_SERVICE_AND_HOST_INTEGRATION_PLAN.md` - formula-edit and language-service plan plus integration-readiness classification.
13. `..\\..\\OxFml\docs\worksets\W048_editor_language_service_and_immutable_formula_host_plan.md` - temporary downstream status companion for the language-service floor and remaining gaps.
14. `..\\..\\OxFml\docs\spec\fec-f3e\FEC_F3E_DESIGN_SPEC.md` - canonical evaluator/coordinator seam contract.
15. `..\\..\\OxFml\docs\spec\OXFML_MINIMUM_SEAM_SCHEMAS.md` - minimum candidate/commit/reject/trace payload fields.
16. `..\\..\\OxFml\docs\spec\OXFML_DELTA_EFFECT_TRACE_AND_REJECT_TAXONOMIES.md` - typed effect, trace, and reject-family meanings.
17. `..\\..\\OxFml\docs\spec\OXFML_CANONICAL_ARTIFACT_SHAPES.md` - canonical artifact ladder for host-facing evaluator artifacts.
18. `..\\..\\OxFml\docs\spec\OXFML_ARTIFACT_IDENTITIES_AND_VERSION_KEYS.md` - identity, fence, and replay-stable version vocabulary.
19. `..\\..\\OxFml\docs\spec\fec-f3e\FEC_F3E_TESTING_AND_REPLAY.md` - replay-facing evaluator evidence and testing contract.
20. `..\\..\\OxFml\docs\spec\fec-f3e\FEC_F3E_FORMAL_AND_ASSURANCE_MAP.md` - assurance and proof-map companion for the seam.
21. `..\\..\\OxFml\docs\spec\OXFML_REPLAY_APPLIANCE_ADAPTER_V1.md` - OxFml replay adapter contract.
22. `..\\..\\OxFml\docs\spec\OXFML_REPLAY_ADAPTER_CAPABILITY_MANIFEST_V1.json` - current machine-readable replay capability claim surface.
23. `..\\..\\OxFml\docs\spec\OXFML_TEST_LADDER_AND_PROVING_HOSTS.md` - proving-host ladder and maturity framing.
24. `..\\..\\OxFml\docs\spec\OXFML_EMPIRICAL_PACK_PLANNING.md` - empirical pack planning and scenario-promotion posture.
25. `..\\..\\OxFml\docs\spec\OXFML_FIXTURE_HOST_AND_COORDINATOR_STANDIN_PACKET.md` - deterministic fixture-host packet for bounded test reuse, not a production host API.
26. `..\\..\\OxFml\docs\spec\formula-language\OXFML_REGISTERED_EXTERNAL_PROVIDER_AND_CALL_REGISTER_ID_BOUNDARY.md` - design-input reference for the broader registered-external worksheet seam that remains outside the current OneCalc product lane.
27. `..\\..\\OxFml\docs\spec\formatting\EXCEL_FORMATTING_HIERARCHY_AND_VISIBILITY_MODEL.md` - formatting and conditional-format visibility model for OneCalc's effective-display plane.
28. `..\\..\\OxFml\docs\spec\formula-language\OXFML_CF_DV_RESTRICTED_SUBLANGUAGES.md` - current restricted-carrier floor for conditional-formatting and data-validation formulas.
29. `..\\..\\OxFml\docs\spec\formula-language\MS_OE376_FORMULA_AND_FORMATTING_REVIEW.md` - current classification and residual map for formula, formatting, `CF`, and `DV` parity.

### 19.3 OxFunc
1. `..\\..\\OxFunc\README.md` - lane role, ownership split, and file map.
2. `..\\..\\OxFunc\CHARTER.md` - lane scope, completeness semantics, and function-phase claim rules.
3. `..\\..\\OxFunc\OPERATIONS.md` - lane execution doctrine and completion reporting.
4. `..\\..\\OxFunc\docs\IN_PROGRESS_FEATURE_WORKLIST.md` - live function-family, seam, replay, and extension status.
5. `..\\..\\OxFunc\CURRENT_BLOCKERS.md` - blocker truth and current unresolved residuals.
6. `..\\..\\OxFunc\docs\function-lane\OXFUNC_DOWNSTREAM_METADATA_AND_HELP_CONTRACT.md` - canonical downstream metadata/help contract for OneCalc and similar hosts.
7. `..\\..\\OxFunc\docs\function-lane\OXFUNC_SURFACE_ADMISSION_AND_LABELING_POLICY.md` - canonical downstream admission categories, labels, and seam-heavy-row honesty policy.
8. `..\\..\\OxFunc\docs\worksets\W050_DEFERRED_CURRENT_VERSION_SURFACE.md` - canonical deferred-current-version exclusion list for the present product claim.
9. `..\\..\\OxFunc\docs\worksets\W051_IN_SCOPE_CURRENT_VERSION_NOT_COMPLETE_SURFACE.md` - canonical in-scope-but-not-complete exclusion list for the present product claim.
10. `..\\..\\OxFunc\docs\function-lane\EXCEL_FUNCTION_DEFINITION_PRELIM_SPEC.md` - current function-profile schema and semantic-definition baseline.
11. `..\\..\\OxFunc\docs\function-lane\EXCEL_FUNCTION_DEFINITION_PRELIM_CONFORMANCE.csv` - machine-readable function-definition and evidence registry.
12. `..\\..\\OxFunc\docs\function-lane\VALUE_UNIVERSE_PRELIM_SPEC.md` - `EvalValue`, `ExtendedValue`, and `CallArgValue` boundary semantics.
13. `..\\..\\OxFunc\docs\function-lane\FUNCTION_ADAPTER_LAYERING_PRELIM_SPEC.md` - preparation/coercion/kernel split for function dispatch.
14. `..\\..\\OxFunc\docs\function-lane\FORMALIZATION_STRATEGY_EXECUTABLE_SEMANTIC_MODEL.md` - formalization and `function-phase-complete` interpretation.
15. `..\\..\\OxFunc\docs\function-lane\FUNCTION_CATALOG_CURRENT_BASELINE_LOCAL.csv` - broad local catalog baseline, to be read only with the deferred/not-complete overlays above.
16. `..\\..\\OxFunc\docs\function-lane\FUNCTION_LANE_EVIDENCE_ID_REGISTRY.md` - stable local evidence id registry.
17. `..\\..\\OxFunc\docs\function-lane\OXFUNC_LIBRARY_CONTEXT_SNAPSHOT_EXPORT_V1_README.md` - downstream reading and stability rules for the snapshot export.
18. `..\\..\\OxFunc\docs\function-lane\OXFUNC_LIBRARY_CONTEXT_SNAPSHOT_EXPORT_V1.csv` - current exported catalog snapshot for pinning and replay correlation.
19. `..\\..\\OxFunc\docs\function-lane\FUNCTION_SLICE_TYPED_CONTEXT_AND_QUERY_BUNDLE_CONTRACT_PRELIM.md` - first-freeze typed host-query and provider bundle.
20. `..\\..\\OxFunc\docs\function-lane\FUNCTION_SLICE_RETURN_SURFACE_AND_PUBLICATION_HINT_CONTRACT_PRELIM.md` - first-freeze return-surface split.
21. `..\\..\\OxFunc\docs\function-lane\FUNCTION_SLICE_RUNTIME_LIBRARY_CONTEXT_PROVIDER_CONSUMER_MODEL_PRELIM.md` - preferred runtime provider/snapshot model beyond CSV pinning.
22. `..\\..\\OxFunc\docs\function-lane\W49_RUNTIME_LIBRARY_CONTEXT_CONSUMER_WALKTHROUGH.md` - current practical downstream companion for consuming the runtime library-context model.
23. `..\\..\\OxFunc\docs\function-lane\FUNCTION_SLICE_CALL_REGISTER_ID_UDF_REGISTRATION_SEAM_PRELIM.md` - design-input reference for the broader worksheet external-registration seam, not a current OneCalc product contract.
24. `..\\..\\OxFunc\docs\function-lane\FUNCTION_SLICE_RTD_CONTRACT_PRELIM.md` - RTD boundary and typed provider outcome contract.
25. `..\\..\\OxFunc\docs\function-lane\RTD_REFERENCE_CAPTURE_AND_SEAM_NOTES.md` - RTD seam clarification companion.
26. `..\\..\\OxFunc\docs\function-lane\FUNCTION_SLICE_HYPERLINK_IMAGE_VALUE_MODEL_PRELIM.md` - current `HYPERLINK` / `IMAGE` rich-value and publication boundary.
27. `..\\..\\OxFunc\docs\function-lane\OXFUNC_REPLAY_APPLIANCE_PACKET_ADAPTER_V1.md` - OxFunc replay packet adapter contract.
28. `..\\..\\OxFunc\docs\function-lane\OXFUNC_REPLAY_ADAPTER_CAPABILITY_MANIFEST_V1.json` - current replay capability manifest.
29. `..\\..\\OxFunc\docs\function-lane\XLL_ADDIN_BRIDGE_SHIM_CONTRACT_PRELIM.md` - current XLL bridge shim contract.
30. `..\\..\\OxFunc\docs\function-lane\XLL_VERIFICATION_SEAM_LIMITATIONS.md` - mandatory qualifier for XLL-based evidence and extension usage.

### 19.4 OxReplay
1. `..\\..\\OxReplay\CHARTER.md` - local mission, ownership boundary, and `DNA ReCalc` split.
2. `..\\..\\OxReplay\OPERATIONS.md` - local replay execution doctrine, promotion, and evidence rules.
3. `..\\..\\OxReplay\docs\spec\README.md` - canonical local spec index.
4. `..\\..\\OxReplay\docs\IN_PROGRESS_FEATURE_WORKLIST.md` - active capability and workset status.
5. `..\\..\\OxReplay\CURRENT_BLOCKERS.md` - current replay blocker truth.
6. `..\\..\\OxReplay\docs\spec\OXREPLAY_DNA_ONECALC_CONSUMPTION_MODEL.md` - explicit OneCalc-facing replay consumption contract.
7. `..\\..\\OxReplay\docs\spec\OXREPLAY_SCOPE_AND_BOUNDARY.md` - shared-mechanics-only boundary statement.
8. `..\\..\\OxReplay\docs\spec\OXREPLAY_BUNDLE_WITNESS_AND_REGISTRY_MODEL.md` - bundle, witness, registry, and lifecycle mechanics.
9. `..\\..\\OxReplay\docs\spec\OXREPLAY_ADAPTER_AND_CONFORMANCE_MODEL.md` - adapter boundaries and capability ladder `C0` through `C5`.
10. `..\\..\\OxReplay\docs\spec\DNA_RECALC_HOST.md` - current replay-host contract and explicit non-OneCalc boundary.
11. `..\\..\\OxReplay\docs\spec\DNA_RECALC_CLI_CONTRACT.md` - current concrete host command surface.
12. `..\\..\\OxReplay\docs\spec\OXREPLAY_WITNESS_LIFECYCLE_TRANSITIONS.md` - local lifecycle transition floor.
13. `..\\..\\OxReplay\docs\spec\OXREPLAY_INITIAL_ADAPTER_INTAKE_PLAN.md` - current lane-intake ordering and scope.
14. `..\\..\\OxReplay\docs\spec\OXREPLAY_CAPABILITY_AND_PACK_TRACEABILITY.md` - capability-to-pack traceability companion.
15. `..\\..\\OxReplay\docs\spec\OXREPLAY_OXXLPLAY_OBSERVATION_SEAM.md` - current observation-to-replay seam with `OxXlPlay`.

### 19.5 OxXlPlay
1. `..\\..\\OxXlPlay\README.md` - repo role, implementation direction, and Windows live-driver posture.
2. `..\\..\\OxXlPlay\CHARTER.md` - observation ownership boundary and replay-ready evidence rule.
3. `..\\..\\OxXlPlay\OPERATIONS.md` - provenance, lossiness, and handoff discipline.
4. `..\\..\\OxXlPlay\docs\spec\README.md` - canonical local spec index.
5. `..\\..\\OxXlPlay\docs\IN_PROGRESS_FEATURE_WORKLIST.md` - current workset and consumer status.
6. `..\\..\\OxXlPlay\CURRENT_BLOCKERS.md` - blocker truth.
7. `..\\..\\OxXlPlay\docs\spec\OXXLPLAY_ONECALC_OBSERVATION_CONSUMER_CONTRACT.md` - canonical OneCalc-facing observation-consumer contract and comparison-envelope rule.
8. `..\\..\\OxXlPlay\docs\spec\OXXLPLAY_SCOPE_AND_BOUNDARY.md` - clean split between observation and replay/semantic ownership.
9. `..\\..\\OxXlPlay\docs\spec\OXXLPLAY_ARCHITECTURE_AND_CAPTURE_MODEL.md` - observation strata and current stable live capture path.
10. `..\\..\\OxXlPlay\docs\spec\OXXLPLAY_ENVIRONMENT_AND_PROVENANCE_MODEL.md` - environment and provenance contract.
11. `..\\..\\OxXlPlay\docs\spec\OXXLPLAY_BUNDLE_EMISSION_AND_HANDOFF_MODEL.md` - observation-bundle and replay-handoff contract.
12. `..\\..\\OxXlPlay\docs\spec\OXXLPLAY_CAPABILITY_AND_PACK_TRACEABILITY.md` - local observation capability ladder.
13. `..\\..\\OxXlPlay\docs\spec\OXXLPLAY_SCENARIO_REGISTER.md` - stable scenario register and retained-root map.
14. `..\\..\\OxXlPlay\docs\spec\OXXLPLAY_CLI_CONTRACT.md` - declared CLI contract, with only `capture-run` currently exercised.
15. `..\\..\\OxXlPlay\docs\spec\OXXLPLAY_IMPLEMENTATION_BASELINE.md` - current implementation truth and Windows bridge shape.
16. `..\\..\\OxXlPlay\docs\test-runs\W006_STABLE_WINDOWS_EXECUTION_DRIVER.md` - best current evidence for what is actually exercised live on Windows.
17. `..\\..\\OxXlPlay\docs\test-runs\W007_FIRST_CROSS_REPO_REPLAY_AND_DIFF_CONSUMPTION.md` - best current evidence for replay-facing comparison readiness and its present limits.

### 19.6 OxCalc Seam-Reference Set
1. `..\\..\\OxCalc\README.md` - lane role and dependency constitution.
2. `..\\..\\OxCalc\CHARTER.md` - lane scope and co-definition rule for shared seam clauses.
3. `..\\..\\OxCalc\OPERATIONS.md` - handoff and completion discipline for coordinator-facing seam changes.
4. `..\\..\\OxCalc\docs\spec\README.md` - canonical OxCalc spec filter and mirror/archive warnings.
5. `..\\..\\OxCalc\docs\IN_PROGRESS_FEATURE_WORKLIST.md` - current seam, TreeCalc, and replay status.
6. `..\\..\\OxCalc\CURRENT_BLOCKERS.md` - current blocker truth.
7. `..\\..\\OxCalc\docs\spec\core-engine\CORE_ENGINE_DOWNSTREAM_HOST_SEAM_REFERENCE.md` - authoritative OxCalc-local guide for downstream hosts such as OneCalc.
8. `..\\..\\OxCalc\docs\spec\core-engine\CORE_ENGINE_ARCHITECTURE.md` - top-level core-engine architecture and evaluator boundary.
9. `..\\..\\OxCalc\docs\spec\core-engine\CORE_ENGINE_COORDINATOR_AND_PUBLICATION.md` - accepted-candidate versus publication semantics and coordinator rules.
10. `..\\..\\OxCalc\docs\spec\core-engine\CORE_ENGINE_OXFML_SEAM.md` - canonical OxCalc-local seam companion.
11. `..\\..\\OxCalc\docs\spec\core-engine\CORE_ENGINE_OXFML_MINIMAL_UPSTREAM_HOST_INTERFACES.md` - first deterministic upstream-host packet used to drive real OxFml paths.
12. `..\\..\\OxCalc\docs\spec\core-engine\CORE_ENGINE_TREECALC_SEMANTIC_COMPLETION_PLAN.md` - TreeCalc-facing consumer model for OxFml-driven execution and host packet use.
13. `..\\..\\OxCalc\docs\spec\core-engine\CORE_ENGINE_TREECALC_OXFML_SEAM_NEGOTIATION_MATRIX.md` - temporary reference for narrower open seam topics and non-assumptions.

### 19.7 OxVba
1. `..\\..\\OxVba\README.md` - repo mission, quick verification, and file map.
2. `..\\..\\OxVba\CHARTER.md` - lane scope and top-level precedence.
3. `..\\..\\OxVba\OPERATIONS.md` - lane-level execution and verification doctrine.
4. `..\\..\\OxVba\MACH1000_PLAN.md` - detailed architecture and phased implementation plan.
5. `..\\..\\OxVba\docs\spec\README.md` - important warning that the relevant local spec docs are mostly `design-draft` or `working-draft`.
6. `..\\..\\OxVba\docs\IN_PROGRESS_FEATURE_WORKLIST.md` - current parity and scope status.
7. `..\\..\\OxVba\CURRENT_BLOCKERS.md` - current blocker truth.
8. `..\\..\\OxVba\docs\spec\BASPROJ_SPEC_V1.md` - canonical current project-file direction with `Library` and `Addin` outputs.
9. `..\\..\\OxVba\docs\spec\HOSTING_PROJECT_TOOLING_PROPOSAL.md` - current design reference for hosting, packaging, top-level tooling, and future add-in-facing productization.
10. `..\\..\\OxVba\docs\spec\HAL_SPEC_WORKING_DRAFT.md` - current host abstraction layer contract draft.
11. `..\\..\\OxVba\docs\spec\HAL_RUNTIME_PROFILE_MATRIX_V1.md` - platform profile matrix for Windows, Linux, WASM, and headless capability gating.
12. `..\\..\\OxVba\docs\spec\COM_CLIENT_SERVER_SCOPE_V1.md` - current Windows-only COM client/server scope boundary.
13. `..\\..\\OxVba\docs\spec\PROJECT_MODULE_REFERENCE_SPEC_V1.md` - project/module/reference semantics for hosted VBA projects.
14. `..\\..\\OxVba\docs\worksets\WORKSET_2026-03-08_EVENTS_RUNTIME_HOST_PROJECT_HAL_SPLIT.md` - current host-versus-HAL split companion.
15. `..\\..\\OxVba\docs\worksets\WORKSET_2026-03-09_HOST_BRIDGE_OBJECT_VALUE_AND_EVENT_INGRESS_CONTRACT.md` - current host-bridge contract companion.
16. `..\\..\\OxVba\docs\worksets\WORKSET_2026-03-23_XLL_ADDIN_SUPPORT_P8.md` - temporary downstream reference for explicit XLL/add-in direction until a stable dedicated spec exists.
17. `..\\..\\OxVba\docs\evidence\language\MS_VBAL_MODULE_PROJECT_REQUIREMENTS.csv` - best current evidence index for host-export and project-model completion status.

### 19.8 Current Upstream Documentation Gaps
1. `OxFml` now has a real OneCalc-facing downstream-consumer contract, but it is still a first-integration clarification note rather than a fully frozen bilateral shared seam.
2. `OxFml` language-service integration is still incomplete upstream: there is no frozen shared immutable edit packet, no frozen validated-completion result packet, and no frozen OxFunc-backed help or signature payload contract.
3. `OxFml` currently documents broader reference-bearing facts for specific semantic lanes, while the intended OneCalc public model is now driven single-formula and excludes the dereference seam; that boundary is explicit but still needs cleaner upstream/downstream synchronization.
4. `OxFunc` now has a materially better downstream integration baseline through its new metadata/help contract and surface-labeling policy, but structured help prose, argument descriptions, formatted signature strings, and the runtime provider materialization remain open.
5. `OxReplay` now has a `DNA OneCalc` consumption model, but OneCalc still consumes replay as infrastructure rather than through a dedicated app-facing host contract, and the current accepted floor remains uneven across lanes.
6. `OxXlPlay` now has a dedicated `DNA OneCalc` observation-consumer contract, but its live exercised surface is still narrow, its comparison envelope is still limited to a first observation family, and its current replay-facing normalized view remains explicitly `lossy`.
7. `OxVba` now has a clearer project-format direction through `.basproj`, but add-in generation and XLL support are still planned rather than implemented; OneCalc cannot currently rely on a shipped OxVba add-in toolchain.
8. No `Ox*` repo currently owns a stable `SpreadsheetML 2003` isolated-instance persistence contract for `DNA OneCalc`; that mapping remains a `DnaOneCalc`-local design lane informed by Foundation reference corpus rather than current upstream product docs.

