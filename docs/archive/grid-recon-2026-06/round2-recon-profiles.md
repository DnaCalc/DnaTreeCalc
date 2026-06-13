# Profile/Capability Substrate — Code Map (OxFml / OxFunc / OxCalc / DnaTreeCalc)

## 1. OxFml: how "profile" exists today — three separate axes, none named "profile" end-to-end

**Axis A — `FormulaChannelKind` (channel):** `C:\Work\DnaCalc\OxFml\crates\oxfml_core\src\source.rs:17` — enum `{WorksheetA1, WorksheetR1C1, ConditionalFormatting, DataValidation}`, carried on `FormulaSourceRecord` (source.rs:28) and hashed into `formula_token()` (source.rs:62). Dispatch points:
- Parse: `worksheet_cell_entry_literal_root` only for `WorksheetA1` (`syntax\parser.rs:170`) — constant-entry classification is channel-gated.
- Bind: R1C1 row/col reference parsing branches (`binding\mod.rs:2920-2970`).
- Language service: R1C1 branch (`language_service\mod.rs:314`).
- Carrier restrictions: `carrier.rs:36-67` — CF/DataValidation carrier contexts pin a channel kind.

**Axis B — `HostReferenceSyntaxProfile` (host syntax surface):** `syntax\parser.rs:14-93` — token-text→family tables (`collection_members`, `structural_selectors`). Threaded: `parse_formula_with_host_reference_syntax` (parser.rs:110), `BindContext.host_reference_syntax` (`binding\mod.rs:258`), `SingleFormulaHost.host_reference_syntax` (`host\mod.rs:88`, used at host\mod.rs:430-447 — note: non-default profile disables incremental green-tree reuse), `RuntimeEnvironment.host_reference_syntax` (`consumer\runtime\mod.rs:180`). Re-exported as `RuntimeHostReferenceSyntaxProfile` (consumer\runtime\mod.rs:65-69).

**Axis C — per-function availability (capability overlay):** `semantics\mod.rs:63-71` — `LibraryAvailabilityState` incl. `HostProfileUnavailable`; carried per function in `FunctionAvailabilitySummary` (semantics\mod.rs:102) inside `SemanticPlan` (semantics\mod.rs:44). Enforcement:
- Snapshot build: `consumer\runtime\mod.rs:776-786` maps OxFunc `CapabilityOverlay` denial → `HostProfileUnavailable`.
- Eval-time denial: `eval\mod.rs:3089-3098` (`runtime_capability_denied_for_function`).
- Denial surface: `runtime_registry_capability_denials` (consumer\runtime\mod.rs:3226-3241) — state stringified to `"HostProfileUnavailable"`.
- **Dry-bind preview seam:** `RuntimeEnvironment::dry_bind_authored_input` → `dry_bind_profile_violations` (consumer\runtime\mod.rs:272-370) produces `RuntimeDryBindProfileViolation{kind: FunctionUnavailable}` — the only place OxFml emits typed "profile violation" objects.

**The profile *id* itself:** `RuntimeHostFormulaContext` (`consumer\runtime\mod.rs:2524-2534`) — `dialect_id: String`, `capability_profile_id: String`, plus namespace/resolution/caller/table identity strings. **OxFml never branches on `capability_profile_id`** — it only feeds `cache_identity_contribution()` (runtime_hash_debug, line 2537). All actual gating is via axes A–C, which the host configures consistently with the id it claims. `host_query_capability_profile: Option<String>` on `OxFuncAdapterRequest` (`oxfunc_adapter\mod.rs:36`) is likewise pass-through metadata into artifacts, never dispatched on.

## 2. Spec profiles vs implementation (CORE_MODEL_SPEC.md §4)

Spec: `C:\Work\DnaCalc\DnaTreeCalc\docs\model\CORE_MODEL_SPEC.md:282-324` — two ids, parse/bind-time rejection of all TreeCalc syntax under strict-excel, profile recorded in persistence, profile-aware INDIRECT (line 306, 383).

Implemented today:
- **Profile id plumbing host→engine: real.** `CapabilityProfileId{TreecalcV1,StrictExcel}` enum (`src\dnatreecalc-host\src\model\workspace.rs:43-56`); mapping to `host-capabilities:*` strings in `context_for_profile_id` (`src\dnatreecalc-host\src\app\session.rs:6994-7015`) → `OxCalcTreeHostCapabilitySnapshot`. Persistence records profile (workspace.rs:13,163; default TreecalcV1 at 254).
- **strict-excel rejection: implemented as a coarse text-scan, not parse/bind gating.** `strict_excel_unsupported_profile_diagnostic` (`OxCalc\src\oxcalc-core\src\consumer.rs:4900-4953`): if profile == `"host-capabilities:strict-excel"`, scans whitespace-stripped uppercase formula text for `INDIRECT(`, `@`, `^.`, `[`, `.*`, `**`, identifier-bearing `{}` → emits stringly diagnostic `typed_exclusion:strict_excel_profile_not_supported:...`. INDIRECT under strict-excel is an explicit "profile-pending" typed exclusion, not an A1 implementation (self-documented in `OxCalc\src\oxcalc-core\src\formula.rs:2348-2360`).
- **treecalc-v1 gating of TreeCalc surfaces: NOT gated — always on.** `treecalc_host_reference_syntax_profile()` (consumer.rs:5658-5679, duplicated at treecalc.rs:7797) registers CHILDREN/*/PRECEDING/FOLLOWING/ANCESTORS/DESCENDANTS/** + PARENT/SELF/PREV/NEXT/NAME/INDEX/FORMULA **unconditionally** (call sites consumer.rs:4687,5493,5599; treecalc.rs:5756,7787) regardless of profile. The strict-excel string-scan is the only thing preventing TreeCalc syntax under strict-excel.
- **Meta nodes:** gated structurally, not by profile — `meta_node_ids: BTreeSet<TreeNodeId>` on `LocalTreeCalcEnvironmentContext` (treecalc.rs:332) and workspace state (consumer.rs:606,1052-1087), threaded via `with_meta_node_ids` (treecalc.rs:3466 etc.).
- **Default-promotion quirk:** `effective_treecalc_capability_profile_id` (treecalc.rs:5540-5548) rewrites `host-capabilities:default` → `host-capabilities:treecalc-v1`; but `context_formula_host_context` (consumer.rs:5681-5700) passes the namespace-snapshot profile verbatim — two paths construct `RuntimeHostFormulaContext` with different normalization.
- **Conformance tests:** `src\dnatreecalc-host\tests\active_profile_gating_corpus.rs` (corpus `docs/test-corpus/profiles/gating.json`) asserts strict-excel never silently accepts TreeCalc syntax and that `strict_excel_profile_not_supported` cases reject explicitly; `active_table_corpus.rs:876-905` covers tables per profile.

Aspirational/unimplemented: true parse/bind-level strict-excel rejection; grid A1/R1C1 references under strict-excel; profile-aware INDIRECT A1 parsing; hard error on unknown persisted profile; profile-conditional syntax-table selection.

## 3. OxCalc: profile-like machinery

- `OxCalcTreeHostCapabilitySnapshot` (`consumer.rs:6296-6315`): `capability_profile_id: String` + 4 effect booleans; default id `host-capabilities:default`. Flows via `OxCalcTreeContextOptions.runtime_context()` (consumer.rs:6426-6451) into `LocalTreeCalcEnvironmentContext` (treecalc.rs:322-368) — the engine-side environment carrying `capability_profile_id`, `runtime_policy_id`, `meta_node_ids`, namespace/resolution versions.
- Profile participates in **prepared identity / compatibility basis**: treecalc.rs:186-195 (compatibility basis string), treecalc.rs:2609-2614 (runtime-effect environment string), and namespace snapshot identity (`workspace_revision.rs:227,300`). W056 test proves differing profile ids split prepared identity (treecalc.rs:9857-9882).
- `OxCalcTreeRuntimePolicy.policy_id` (consumer.rs:6318-6336) is a **stringly dispatch channel**: cycle engine selected by `runtime_policy_id.contains("cycle.excel_match_iterative")` (treecalc.rs:1035-1038 → `publish_excel_match_iterative_cycle` treecalc.rs:1538), and fixture surfaces by `.contains("excel_iter_two_node_order_001")` etc. (treecalc.rs:1656-1690). `iterative_deterministic_v0` exists in spec (CORE_MODEL_SPEC §6 item 12) and tracecalc (`oxcalc-tracecalc\src\machine.rs:637`), not as a typed engine enum.
- Dry-bind seam: `OxCalcTreeDryBindProfileViolation{Kind}` (consumer.rs:137-160) is a 1:1 re-projection of OxFml's runtime verdict (consumer.rs:5532-5561) — this is how profile violations reach DnaTreeCalc previews.
- Tables: `TreeCalcTableFormulaRuntimeContext` (`structured_table.rs:3301-3327`) carries its own `capability_profile_id` (default `treecalc-v1`) **plus `capability_overlay: Option<CapabilityOverlay>`** — the only place a profile id and a function-availability overlay sit in one struct.
- Engine-level threading for a new profile: `DnaTreeCalc session.rs context_for_profile_id` → `OxCalcTreeContextOptions.with_host_capabilities` → `runtime_context()` → `LocalTreeCalcEnvironmentContext` → `treecalc_host_formula_context`/`context_formula_host_context` → `RuntimeHostFormulaContext` (identity only) + the string-scan gate.

## 4. OxFunc: catalog variant mechanisms

- `CapabilityOverlay` (`OxFunc\crates\oxfunc_core\src\registry.rs:450-484`): function_id→`FunctionAvailability::{Available, Unavailable{reason}}`; consumed by OxFml snapshot build and dry-bind, and by OxCalc table runtime. This is the **per-function availability mechanism** — host-composed, not profile-id-driven.
- Static metadata: `FunctionRegistryMetadata.gating_profile_ref: Option<String>` (registry.rs:221) — populated uniformly as `"oxfunc.local.gating.current_baseline.default.v1"` across the whole seed catalog (`registry_context_seed.rs`, ~hundreds of entries); a placeholder pointer, no profile variance yet. `semantic_trait_profile_ref`, `name_resolution_table_ref` similar. `_xlfn` appears only in seam-integration tests.
- INDIRECT (`functions\indirect.rs:14-26`): per spec, OxFunc stays profile-agnostic — string resolution delegates to host `ReferenceSystemProvider` via `ReferenceTextResolveRequest{mode: ReferenceTextResolutionMode::Indirect, a1_style: Option<bool>}` (`resolver.rs:50-52,103-108`). The tree-path-vs-A1 dispatch point therefore lives in whichever host implements the provider — there is no tree/A1 mode enum yet (`Indirect` is the sole variant).

## 5. Channel vs capability profile — orthogonal?

Yes, in code they are independent axes: `FormulaChannelKind` (syntax-entry channel, OxFml-owned, hashed into formula identity) never interacts with `capability_profile_id` (host-claimed id, identity-only) or `HostReferenceSyntaxProfile` (token tables). OxCalc's upstream-host lane only accepts `worksheet_a1` (`upstream_host_fixture.rs:1017-1031`); the TreeCalc lane doesn't use `FormulaChannelKind` at all (it uses host-reference packets). Spec §6 item 11 ("constant entry classification on the TreeCalc channel") implies a future TreeCalc channel kind that doesn't exist yet — a strict-excel-grid profile would keep WorksheetA1/R1C1 channels and vary the other two axes.

## 6. Minimal touch-set for `strict-excel-grid` + refactor targets

**Types/seams a new profile must touch:**
1. `CapabilityProfileId` enum + serde (DnaTreeCalc workspace.rs:43) and `context_for_profile_id` mapping (session.rs:6994).
2. `OxCalcTreeHostCapabilitySnapshot.capability_profile_id` (consumer.rs:6298) — and make `treecalc_host_reference_syntax_profile()` call sites (consumer.rs:4687,5493,5599; treecalc.rs:5756,7787) **profile-conditional** (empty/default tables under strict-excel-grid).
3. `RuntimeHostFormulaContext.dialect_id/capability_profile_id` (OxFml consumer\runtime\mod.rs:2525-2527) — today free strings; candidates for a typed profile descriptor.
4. `HostReferenceSyntaxProfile` (parser.rs:14) — already the right shape for parse-gating; grid profile supplies none/grid-specific tokens.
5. `CapabilityOverlay` (OxFunc registry.rs:450) — per-profile function denial composition; `gating_profile_ref` seed values would finally vary.
6. `ReferenceTextResolutionMode`/`ReferenceSystemProvider` (OxFunc resolver.rs:50) — needs an A1-grid-capable provider implementation for INDIRECT under strict-excel.
7. Identity surfaces: compatibility basis (treecalc.rs:186), `FenceSnapshot.profile_version`/`capability_view_key` (OxFml host\mod.rs:803-814 — currently overloaded with *locale* profile and a binary host-query flag).

**Stringly-typed / scattered hotspots (refactor targets):**
- `strict_excel_unsupported_profile_diagnostic` + `strict_excel_treecalc_only_syntax` (OxCalc consumer.rs:4900-4953): raw-text substring scan standing in for parse/bind gating; diagnostics matched downstream by `.contains("typed_exclusion:strict_excel_profile_not_supported:INDIRECT")` (consumer.rs:4963).
- Profile ids as bare strings compared with `==`/`match` in ≥6 places (consumer.rs:4910; treecalc.rs:5543; session.rs:6995-6997,7017-7022 incl. a `Box::leak` for unknown profiles — silently tolerated, violating the spec's hard-error rule).
- `runtime_policy_id.contains(...)` cycle/fixture dispatch (treecalc.rs:1035,1656-1690).
- Duplicate `treecalc_host_reference_syntax_profile()` definitions (consumer.rs:5658 vs treecalc.rs:7797) and dual `RuntimeHostFormulaContext` constructors with inconsistent default-promotion (treecalc.rs:5540 vs consumer.rs:5689).
- `gating_profile_ref` uniform placeholder strings across the whole OxFunc seed catalog (registry_context_seed.rs).
- `FenceSnapshot.profile_version` conflating locale profile with capability profile (OxFml host\mod.rs:803-806).