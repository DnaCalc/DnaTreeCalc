Status: Responded
Target: OxCalc
Ask: Verify the stack-requirement engine readiness claims, land the cheap W0 exposure path where current code supports it, and size the gating engine workstreams.
Context: FLOW/ATLAS need richer Skin IR projections without host-owned semantics. The host must consume OxCalc/OxFml truth, not reconstruct it.

## OxCalc response

OxCalc code inspection confirms the stack-requirement docs are directionally right, but several `expose`/`extend` tags need sharper wording before downstream work depends on them.

### Readiness corrections

| Requirement / question | Verified code state | Corrected readiness | First action |
|---|---|---|---|
| `stable-node-identity` | `TreeNodeId` is already the stable engine node identity. DnaTreeCalc was still path-keyed. | `expose` in OxCalc; transition work in host/Skin IR. | Started: Skin IR now carries `NodeKey` alongside path `NodeId`. |
| `typed-dependency-kinds` | `DependencyDescriptorKind` is a closed OxCalc enum. DnaTreeCalc projected it as `String`. | `expose`. | Started: Skin IR dependency projections now carry typed enum variants. |
| `typed-invalidation-reasons` | `InvalidationReasonKind` is a closed OxCalc enum. DnaTreeCalc projected it as `String`. | `expose`. | Started: Skin IR invalidation projections now carry typed enum variants. |
| `typed-run-and-calc-state` | `OxCalcTreeRunState` and `NodeCalcState` are typed, but some control-relevant conditions still leak through diagnostic strings. | `extend`, not pure expose. | Keep run/node states typed; retire diagnostic parsing in later OxCalc cleanup. |
| `richer-typed-value` | OxCalc publishes `CalcValue` per node; DnaTreeCalc already preserves arrays but still collapses most scalar variants into display strings. | OxCalc `expose`; host projection still `extend`. | Next W0/W1 host batch should add typed scalar/error/reference variants without losing display rendering. |
| `phase-timings-typed` | Current public outcome was `BTreeMap<String, u128>`. It now carries OxCalc-owned `LocalTreeCalcPhaseKey` with `Other(String)` and stable string serialization. | `extend`, first slice landed. | Threaded into DnaTreeCalc as `PhaseKeyProjection`. |
| `reference-resolution-map` | Dependency descriptors carry `source_reference_handle`, targets, and collection membership; no single token-to-target map/reverse index projection exists yet. | bounded `extend` over existing facts, not pure expose. | Add public resolution-map struct from dependency graph + OxFml handles. |
| `full-derivation-trace` | `derivation_traces: Vec<DerivationTraceRecord>` exists on calculation outcome. Need verify payload completeness against prepared-call tree/hole-binding requirement. | `expose` for current trace list; `extend` if payload is too shallow. | Spike payload fields before claiming FLOW explain-stack coverage. |
| `per-edge-cache-evidence` | Existing value-cache surfaces are basis/counter oriented; no projected per-edge `Hit/Miss/Bypassed` found. | `extend`. | Design per-edge scheduling/cache report. |
| `typed-cycle-diagnostics` | Cycle groups exist; general iterative convergence trace with `max_change` is not evident. | `extend`, gated by W055 cycle work. | Keep out of W0; spike with W055. |

### Gating workstreams

| Workstream | Go/no-go | Cost | Code evidence | Notes |
|---|---|---|---|---|
| `transaction-scope` | Go after a focused design spike. | L | Current recalc path updates node inputs, schedules, and publishes as one `recalculate()` call, but conceptual multi-edit transaction receipts/rollback are not exposed. Stage tracker still moves nodes through per-node `PublishReady`. | Best first engine substrate. Define an edit batch API and rollback boundary before multi-target verbs. |
| `revision-graph-retention` | Go as new substrate after transaction ids exist. | L/XL | `WorkspaceRevision` is an immutable identity over structure/input/namespace snapshots, but there is no retained parent-linked store or cursor. | Existing snapshot identities are useful nodes for a graph, not the graph itself. Needs memory/GC policy alignment with W054. |
| `candidate-overlay-handle` | Go for implementation as new OxCalc substrate after W4b spike. | XL | `RuntimeOverlaySet` is keyed from `PublicationSnapshotId`; current candidate results are one-run publish/reject packets; no retained `open_candidate`, addressable handles, layering, or non-publishing run context exists. | Largest risk. Do not schedule scenario/what-if/goal-seek features until OxCalc returns real candidate handles and non-published candidate projections. |
| `value-epoch-keying` | Go as bounded bookkeeping after transaction/revision shape is clear. | M/L | Current context has workspace-level `value_epoch`; per-node `input_epoch` exists in `NodeInputRecord`; no per-node published-value epoch found. | Delta channel can proceed without this using invalidated-node sets; shape-diff/memoization should wait. |

### Additional open-question answers

| Question | Answer |
|---|---|
| Passivity / slicing | Engine remains synchronous/passive. No cooperative slice/resume API was found. Host-worker calc can run whole ticks off-main-thread now; bounded frame pumping needs a later engine API. |
| Delta-only mode | Current host projection republishes full `WorkspaceState`; no sanctioned delta-only/gap-resync mode yet. Additive deltas are safe first. |
| Table without grid coords | OxCalc has structured table snapshots and typed dependency facts over rows/columns/regions while TreeCalc remains non-grid. Authoring structural row/column ops are still new work. |
| Durable scenarios | No durable scenario store or named override substrate was found. Treat scenario durability as new capability above candidate overlays. |

### Landed first slice

DnaTreeCalc now carries a W0 transition-window projection:

- `NodeKey` wraps OxCalc `TreeNodeId` as `tree-node:<id>` while existing path `NodeId` remains for current skins.
- `NodeView.key` and `WorkspaceState.key_order` expose stable engine identity.
- Dependency descriptor and edge projections use `DependencyKindProjection` instead of strings.
- Invalidation records use `InvalidationReasonProjection` instead of strings.
- Calc-run phase timings use OxCalc-owned `LocalTreeCalcPhaseKey` and Skin IR `PhaseKeyProjection` instead of raw string keys.
- `TreeWorkspaceSession` maintains a reverse `TreeNodeId -> NodeId` index, removing the repeated reverse scan in reprojection.

Evidence:

- `cargo test -p dnatreecalc-skin-framework -- --nocapture`
- `cargo test -p dnatreecalc-host --lib -- --nocapture`
- `cargo test -p dnatreecalc-host --test walking_skeleton -- --nocapture`
- Focused test: `session_projects_stable_node_keys_and_typed_engine_classifications`

Still open:

- Complete the host cutover from path-keyed maps to `NodeKey`-keyed maps.
- Add typed scalar `CalcValue` variants beyond the current scalar-display fallback.
- Add reference-resolution map, typed binding diagnostics intake, and derivation-trace payload audit.
- Spike `transaction-scope`, `revision-graph-retention`, and `candidate-overlay-handle` as separate engine beads before dependent Skin IR features are scheduled.

## W4b candidate-overlay spike update

Status: Responded
OxCalc artifact: `OxCalc/docs/spec/core-engine/CORE_ENGINE_CANDIDATE_OVERLAY_HANDLE_SPIKE.md`
Execution bead: `calc-etez`

OxCalc has now answered the `candidate-overlay-handle` spike from live code:

| Question | Answer |
|---|---|
| Can W4b expose existing candidate/overlay state? | No. `OxCalcTreeContext::recalculate` creates one run-local candidate id and then publishes or rejects. `TreeCalcCoordinator` has one in-flight/accepted candidate lane. `RuntimeOverlaySet` is keyed to the current `PublicationSnapshotId`. |
| Is W4b schedulable? | Yes, as new OxCalc substrate. |
| First build slice | Opaque candidate handle over a retained revision; apply one private candidate edit; evaluate without advancing published revision, publication snapshot, published values, or published value epochs; discard and prove the handle is gone. |
| Host/Skin IR rule | Do not expose scenario/what-if/provenance features until OxCalc returns real candidate handles and non-published candidate projections. |

The naming risk is explicit: prior W047/W048 docs use "candidate overlay" for one-run CTRO
consequences before a publish/reject decision. W4b should use `CandidateOverlayHandle` or
`candidate context` when discussing retained host-addressable speculation.

## W4b first substrate slice update

Status: Responded
OxCalc evidence: `cargo test -p oxcalc-core`

OxCalc has landed the first non-publishing candidate substrate slice:

| Capability | Current scope |
|---|---|
| Open candidate | `CandidateOverlayHandle` opens on a retained `WorkspaceRevisionId`. |
| Private edit | `apply_candidate_edit_transaction` applies candidate-private node edits through OxCalc edit machinery with `ApplyOnly` policy. |
| Private evaluate | `evaluate_candidate` evaluates the candidate in a private copy-based context and returns candidate calculation values. |
| Discard | `discard_candidate` removes the handle; later reads return typed `UnknownCandidate`. |
| Non-publication proof | Focused OxCalc test asserts live workspace revision, publication snapshot, runtime overlay set, visible value, and published value epoch remain unchanged after candidate evaluation. |

Still open for DnaTreeCalc consumption, tracked in OxCalc bead `calc-4ipg`: parent/layered
candidate semantics, structural candidate edits, scenario/what-if UX, richer candidate transaction
summary projection, and candidate retention/GC policy.

## W4b first commit bridge update

Status: Responded
OxCalc evidence: focused commit tests in `cargo test -p oxcalc-core treecalc_context_candidate_commit -- --nocapture`

OxCalc has landed the first candidate commit bridge:

| Capability | Current scope |
|---|---|
| Commit | `commit_candidate` promotes a candidate's private evaluated workspace state into the live workspace. |
| Basis guard | Commit is accepted only when the live workspace revision still equals the candidate basis revision. |
| Stale basis | Stale commit returns typed `CandidateBasisNotCurrent` and keeps the candidate retained for discard or future rebase semantics. |
| Handle lifecycle | Successful commit removes the candidate handle. |

## W4b first host/Skin IR projection update

DnaTreeCalc now consumes the first candidate substrate through closed Skin IR intents:

| Surface | Current scope |
|---|---|
| `WorkspaceIntent::OpenCandidate` | Opens an OxCalc candidate on the current published workspace revision. |
| `WorkspaceIntent::EditCandidateContent` | Applies a candidate-local content edit over an existing projected node id. |
| `WorkspaceIntent::EvaluateCandidate` | Evaluates private candidate values and projects them under `WorkspaceState.candidates`. |
| `WorkspaceIntent::DiscardCandidate` | Removes the candidate without publishing workspace state. |
| `WorkspaceIntent::CommitCandidate` | Commits only when OxCalc accepts the basis-current guard; stale basis maps to typed `CandidateBasisNotCurrent`. |

Candidate values are projected separately from published `NodeView.computed_value`, so skins can
show what-if results without owning calculation semantics or mutating published state. Commit
produces a new workspace revision and removes the candidate projection. This slice does not
fabricate a transaction id when the promoted candidate revision has no retained transaction summary.

Still open for DnaTreeCalc consumption, tracked in OxCalc bead `calc-4ipg`: parent/layered
candidate semantics, structural candidate edits, scenario/what-if UX, richer candidate transaction
summary projection, and candidate retention/GC policy.
