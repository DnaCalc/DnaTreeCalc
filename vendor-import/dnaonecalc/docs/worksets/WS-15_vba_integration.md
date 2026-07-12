# WS-15 VBA Integration

Status: `requirements-and-spec`
Date: 2026-05-16
Scope owner: DnaOneCalc host/runtime

## 1. Purpose

Define the DnaOneCalc host module that loads OxVba projects, injects a
DnaOneCalc-specific host project surface, discovers public module functions as
candidate UDFs, publishes admitted functions into OxFml, and invokes them
through a typed boundary that is validated against Excel-observed VBA UDF
behavior.

This workset is the runtime-hosting lane for OxVba integration. OxIde-facing
authoring, project editing, and richer IDE services are adjacent later work.

## 2. Authority And Upstream Inputs

Local authority:
1. `docs/SCOPE_AND_SPEC.md` remains the product and boundary authority.
2. `docs/WORKSET_REGISTER.md` owns workset placement and rollout shape.
3. `.beads/` owns execution state.

Upstream inputs read for this spec:
1. `..\OxVba\README.md`
2. `..\OxVba\docs\spec\BASPROJ_SPEC_V1.md`
3. `..\OxVba\docs\spec\HOSTING_PROJECT_TOOLING_PROPOSAL.md`
4. `..\OxVba\docs\spec\PROJECT_MODULE_REFERENCE_SPEC_V1.md`
5. `..\OxVba\docs\spec\HAL_SPEC_WORKING_DRAFT.md`
6. `..\OxVba\docs\worksets\WORKSET_2026-05-09_WRAPPED_COM_SERVER_INTERFACE_EVENT_UDF_EXECUTION.md`
7. `..\OxVba\docs\worksets\WORKSET_2026-05-10_HOST_PROGRAM_DESIGN_AND_UDF_REWORK.md`
8. `..\OxFml\docs\spec\formula-language\OXFML_REGISTERED_EXTERNAL_PROVIDER_AND_CALL_REGISTER_ID_BOUNDARY.md`
9. `..\OxFunc\docs\function-lane\FUNCTION_SLICE_CALL_REGISTER_ID_UDF_REGISTRATION_SEAM_PRELIM.md`

Current upstream reading:
1. OxVba has a real direct-host substrate: `.basproj`, `OutputType=HostModule`,
   host root object naming through `DefaultRootObject`, project manifests,
   procedural-module host exports, `HostUdfCatalog`, `HostUdfCallContext`, and
   `invoke_host_udf_with_variants`.
2. OxVba's UDF-like host path is still under rework. In the current
   implementation, the runtime call frame is built but the context is not yet
   delivered into execution.
3. OxFml has a current host-function callback seam through
   `TypedContextQueryBundle::with_host_function_provider` and runtime
   library-context entries with `runtime_boundary_kind = vba_host_callback`.
4. OxFml also has a registered-external mutation lane through
   `RegisteredExternalCatalogMutationRequest`, including
   `RegisteredExternalRegistrationChannel::VbaProjectShimRegistration`; that is
   the broader descriptor lane, not the first executable DnaOneCalc slice.
5. OxFunc owns registered-external catalog identity and descriptor-driven
   argument/result policy; OxFml funnels host registration intent and preserves
   channel/provenance when that lane is used.

## 3. Non-Goals

This workset does not implement:
1. OxIde project authoring or editing UI.
2. Excel/VBIDE project storage import/export.
3. COM Automation Add-In behavior.
4. XLL packaging.
5. worksheet `REGISTER.ID` / `CALL` as a public OneCalc product lane.
6. broad worksheet graph semantics.
7. broad Excel `Application` object parity.
8. thread-safe VBA UDF claims.

## 4. Host Module Boundary

The DnaOneCalc runtime host should add a module tentatively named
`vba_host`. The module owns DnaOneCalc policy and orchestration only.

It must not own:
1. VBA parsing, binding, execution, or object semantics.
2. formula-language semantics.
3. function catalog mutation semantics.
4. formula argument coercion semantics.

The first host module responsibilities are:
1. associate one or more OxVba projects with a DnaOneCalc workspace or formula
   space,
2. load each associated project through OxVba direct-host APIs,
3. configure a DnaOneCalc-specific host root object named `Application`,
4. discover public procedural module functions from each loaded project,
5. classify each discovered function as admitted, rejected, or deferred as a
   DnaOneCalc UDF candidate,
6. publish admitted functions through an OxFml library-context snapshot with a
   VBA host callback runtime boundary,
7. define and enforce the typed VBA-UDF invocation boundary between
   OxFml/OxFunc values and OxVba values,
8. provide an invocation adapter that OxFml can call through the host-function
   provider path,
9. record capability, diagnostics, registration, invocation, and Excel-oracle
   comparison evidence in the
   workspace/run artifacts.

Expected code placement in the current host crate:
1. `src/dnaonecalc-host/src/services/vba_host.rs` for association state,
   catalog admission, typed mapping, and invocation orchestration.
2. `src/dnaonecalc-host/src/adapters/oxvba.rs` for the narrow OxVba API
   wrapper, so the rest of the host does not couple to OxVba internals.
3. `src/dnaonecalc-host/src/services/verification_bundle.rs` extension points
   for retained Excel-oracle comparison cases.
4. `src/dnaonecalc-host/src/services/programmatic_testing.rs` schema additions
   for VBA UDF case metadata.
5. UI wiring is out of the first implementation slice except for capability and
   retained-artifact visibility.

## 5. Project Association Model

Each workspace or formula space may associate zero or more OxVba projects.

First persistent shape:
1. `association_id`: stable DnaOneCalc id.
2. `scope`: `workspace` or `formula_space`.
3. `project_ref`: path to `.basproj`, project directory, or future embedded
   project record.
4. `project_identity`: OxVba project name after load.
5. `runtime_profile`: DnaOneCalc-selected OxVba runtime profile.
6. `policy_preset`: DnaOneCalc-selected OxVba policy preset.
7. `root_object_name`: initially always `Application`.
8. `enabled`: whether the association participates in UDF registration.
9. `registration_namespace_policy`: initially `fail_on_collision`.
10. `last_load_status`: unloaded, loaded, failed, disabled, or stale.
11. `source_fingerprint`: hash or version key for the project source or bundle.
12. `last_catalog_generation`: generation key for the admitted/rejected UDF
   catalog derived from the project.
13. `oracle_family_ref`: optional reference to retained Excel oracle evidence
   used to admit the current typed matrix rows.

The first implementation should prefer file/path-backed `.basproj` or project
directory references. Embedded project storage can follow once workspace
persistence has a stable container decision for project source.

Association lifecycle:
1. `discover`: resolve the project reference and identify its project kind.
2. `load`: create or refresh the OxVba host/session state.
3. `catalog`: read the OxVba host UDF catalog and normalize candidate identities.
4. `admit`: apply DnaOneCalc typed admission policy and collision checks.
5. `publish`: expose admitted rows through the current OxFml runtime
   library-context snapshot/provider seam.
6. `invoke`: route formula calls to the associated project session.
7. `disable`: remove admitted descriptors from the published function surface
   and invalidate affected formulas.
8. `refresh`: repeat load/catalog/admit/publish when the source fingerprint
   changes.

## 6. Host Project And Root Object

When a project is associated, DnaOneCalc loads it with a host-provided
DnaOneCalc project surface rather than relying on ambient Office state.

First host root:
1. name: `Application`
2. kind: host-injected root object
3. first property: `Version`
4. `Version` value: the DnaOneCalc host version string for the running build
5. mutability: read-only
6. allowed call contexts: host formula evaluator and future host command
   context

Required behavior:
1. A VBA expression that reads `Application.Version` from an associated project
   receives the DnaOneCalc version string.
2. No other Excel-style `Application` members are implied.
3. Missing members fail through OxVba host-object diagnostics, not through
   DnaOneCalc local fabrication.
4. The capability ledger must distinguish `Application.Version` support from
   broader Excel `Application` support.

Initial example:

```vb
Public Function HostVersion() As String
    HostVersion = Application.Version
End Function
```

## 7. UDF Discovery

For every enabled associated OxVba project, DnaOneCalc asks OxVba for its
host-callable function catalog.

Candidate scan floor:
1. procedural modules only,
2. public functions only,
3. no public subs in the UDF catalog,
4. no class methods in the first tier,
5. no document-module procedures in the first tier,
6. deterministic ordering by project, module, and procedure identity.

The current OxVba implementation exposes public procedural functions through
`HostUdfCatalog` and stable host-call ids. DnaOneCalc should consume that
surface instead of parsing `.bas` files directly.

Example first-tier candidate:

```vb
Public Function AddThem(val1 As Double, val2 As Double) As Double
    AddThem = val1 + val2
End Function
```

The example in the request used `v1 + v2`; that should produce an OxVba
compile/runtime diagnostic unless those names are actually in scope.

## 8. UDF Admission Policy

Discovery and admission are separate.

First admission floor:
1. function is public and procedural,
2. function name is a legal formula-visible function name after DnaOneCalc
   normalization,
3. stable registration id can be generated without collision,
4. parameter and return types have an explicit typed mapping across the
   OxFml/OxFunc boundary and the OxVba boundary,
5. the function has no required unsupported host capability under the selected
   OxVba policy/profile,
6. side-effect policy is `no-host-side-effects`,
7. thread-safety policy is `single-threaded-vba-compatible`,
8. allowed contexts include DnaOneCalc's host formula evaluator context.

Typed admission rule:
1. DnaOneCalc must not admit a VBA UDF only because OxVba can invoke it.
2. Admission requires a declared conversion rule for every argument and the
   return value.
3. Each conversion rule must state whether it is:
   - Excel-observed,
   - Excel-documented but not yet observed,
   - DnaOneCalc provisional,
   - or rejected.
4. Excel-observed behavior is the target. DnaOneCalc should prefer a narrow
   admitted matrix backed by Excel evidence over a broad guessed conversion
   layer.

First scalar candidate matrix:
1. Excel numeric input to typed VBA numeric parameters, including the exact
   Excel/VBA coercion behavior for `Double`, `Single`, `Currency`, `Long`,
   `Integer`, `Byte`, and `Boolean`.
2. Excel text input to VBA `String`, and text-to-number coercions only when
   Excel evidence pins the behavior for the same UDF signature.
3. Excel logical input to VBA `Boolean` and numeric parameters only when the
   observed Excel behavior is captured.
4. Excel errors, blank cells, empty strings, `Empty`, `Null`, `Variant`,
   arrays/ranges, object parameters, optional parameters, `ParamArray`, and
   ByRef writeback are not admitted until the comparison harness captures the
   Excel behavior and the OxVba boundary can represent it.

### 8.1 First Pinned Type Matrix

The first implementation must pin these rows before writing broader conversion
code:

| Row id | VBA signature | Formula call | Expected status | Evidence gate |
|---|---|---|---|---|
| `VBA-UDF-T001` | `AddThem(Double, Double) As Double` | `=AddThem(2,3)` | admitted | Excel-observed retained bundle plus DnaOneCalc/OxVba match |
| `VBA-UDF-T002` | `AddThem(Double, Double) As Double` | `=AddThem(TRUE,3)` | blocked until observed | Excel oracle row required before admission |
| `VBA-UDF-T003` | `AddThem(Double, Double) As Double` | `=AddThem("2",3)` | blocked until observed | Excel oracle row required before admission |
| `VBA-UDF-T004` | `AddThem(Double, Double) As Double` | `=AddThem("",3)` | blocked until observed | Excel oracle row required before admission |
| `VBA-UDF-T005` | `AddThem(Double, Double) As Double` | `=AddThem(A1,3)` where `A1` is blank | blocked until observed | Excel oracle row required before admission |
| `VBA-UDF-T006` | `AddThem(Double, Double) As Double` | `=AddThem(A1,3)` where `A1` is `#DIV/0!` | blocked until observed | Excel oracle row required before admission |
| `VBA-UDF-T007` | `EchoText(String) As String` | `=EchoText("abc")` | blocked until observed | second scalar family |
| `VBA-UDF-T008` | `NotIt(Boolean) As Boolean` | `=NotIt(TRUE)` | blocked until observed | second scalar family |

Only `VBA-UDF-T001` is allowed to become the first green implementation path.
Rows `T002` through `T008` are planning fixtures and must remain blocked until
the Excel oracle harness produces retained observations.

Rejected candidates must keep a visible rejection reason in the host catalog
state. Deferred candidates must keep a `SEAM-OXVBA-UDF-*` id or equivalent
pending-seam record when the blocker is an upstream or bridge gap.

## 9. Publication With OxFml

Admitted functions are published to OxFml through the runtime library-context
snapshot/provider seam in the first executable slice. Each admitted row is a
catalog-known function with `runtime_boundary_kind = vba_host_callback`, so
OxFml can bind the name and route execution to the host-function provider.

For each admitted function, DnaOneCalc builds a `LibraryContextSnapshotEntry`
with:
1. `surface_name = <formula-visible function name>`,
2. `surface_stable_id = <OxVba stable host-call id>`,
3. `name_resolution_table_ref = vba:<association_id>`,
4. `registration_source_kind = Vba`,
5. `parse_bind_state`, `semantic_plan_state`, and `runtime_capability_state`
   set to catalog-known,
6. `admission_interface_kind` naming the admitted typed row,
7. `interface_contract_ref` naming the DnaOneCalc VBA first-slice contract,
8. `runtime_boundary_kind = vba_host_callback`.

Publication that changes bind-visible function names must publish a new
function-surface/library-context generation and invalidate affected formula
binding state.

Broader descriptor lane:
1. OxFml's `RegisteredExternalCatalogMutationRequest::Register` and
   `VbaProjectShimRegistration` channel remain the expected route once the
   descriptor-rich external registration lane is selected for VBA UDFs.
2. The first executable DnaOneCalc slice uses the host-function provider seam
   because that is the smallest current path that binds catalog-known names and
   executes callbacks without inventing extra OxFunc descriptor semantics.
3. The host must still retain the OxVba host-call id and local typed signature
   sidecar; neither the OxFml snapshot entry nor the OxVba id is enough by
   itself.

### 9.1 No-Host-Reference Guardrail

Ordinary single-formula execution must remain the default path:
1. built-in functions, operators, literals, and `LET` / `LAMBDA` formulas must
   evaluate without a DnaOneCalc host namespace, host-reference resolver,
   `HostFunctionProvider`, `RegisteredExternalProvider`, RTD provider, or
   host-query provider;
2. `LET` locals, callable locals, lambda captures, and returned lambda values
   must continue through OxFml/OxFunc lexical and callable-value machinery, not
   through DnaOneCalc-local function mirrors;
3. completion, signature help, and function help must stay registry-backed by
   OxFml/OxFunc rather than by a host-authored catalog copy.

UDF support enters only through an explicit registry-backed extension lane:
1. admitted VBA/XLL functions publish a new function-surface or
   `LibraryContextSnapshot` generation through the OxFml/OxFunc registration
   surfaces;
2. bind-visible UDF registration, unregister, disable, or source-change events
   must invalidate affected formula binding/runtime caches before the next
   ordinary edit or recalc result is trusted;
3. future `VbaProjectShimRegistration` work may replace the current
   host-function-provider first slice, but it must not make the host namespace
   resolver part of ordinary formulas.

## 10. Invocation Path

When a DnaOneCalc formula calls a published VBA UDF:
1. OxFml parses and binds the formula against the current function surface.
2. OxFml routes a catalog-known `vba_host_callback` call through
   `HostFunctionProvider::invoke_host_function`.
3. DnaOneCalc maps the host-function invocation name back to the admitted OxVba
   stable host-call id.
4. DnaOneCalc builds an OxVba `HostUdfCallContext` with:
   - caller identity for the current formula space,
   - locale id where available,
   - calculation pass id where available,
   - dependency tokens for explicit arguments where available,
   - volatile-request sink when supported.
5. DnaOneCalc converts OxFml/OxFunc invocation arguments into OxVba values using
   the admitted Excel-observed conversion rule for the target function
   signature.
6. DnaOneCalc calls OxVba's typed host UDF invocation surface.
7. DnaOneCalc maps the returned OxVba value into an OxFml/OxFunc value or
   formula error using the matching typed return rule.
8. DnaOneCalc records volatile/dependency side effects returned by the invocation
   result where supported.

Current upstream caveat:
1. OxVba currently builds but does not yet execute through the host UDF call
   frame in the way its May 2026 host-program rework intends.
2. The current `Variant`-only invocation surface may not be enough for exact
   Excel-compatible UDF behavior once coercion, error, array, object, optional,
   `Variant`, or ByRef cases enter scope.
3. DnaOneCalc may implement a narrow adapter against the current surface, but must
   record any FEC/provider-delivery gap as upstream pressure if a required
   DnaOneCalc behavior depends on it.
4. If exact Excel behavior requires OxVba to expose a richer typed UDF call
   contract, DnaOneCalc must capture that as an OxVba handoff rather than
   compensating with a private local coercion layer.

## 10.1 Typed Boundary Contract

The typed boundary is a first-class contract, not an implementation afterthought.

Required DnaOneCalc-side records:
1. `VbaUdfSignature`: project, module, function, public name, parameter list,
   return type, optional/default/ByRef/ParamArray flags where available.
2. `VbaUdfTypeMap`: per-parameter and return mapping from OxFml/OxFunc values to
   OxVba values and back.
3. `VbaUdfExcelOracleExpectation`: observed Excel result, observed error, and
   capture provenance for the same UDF signature and formula call.
4. `VbaUdfInvocationRecord`: actual DnaOneCalc/OxVba call inputs, converted
   values, raw OxVba result, converted formula value, diagnostics, and
   comparison verdict.

Minimal JSON shape for retained sidecars:

```json
{
  "schema_id": "dnaonecalc.vba_udf_invocation.v1",
  "case_id": "VBA-UDF-T001",
  "association_id": "vba-assoc-1",
  "signature": {
    "project": "VBAProject",
    "module": "Module1",
    "function": "AddThem",
    "formula_name": "AddThem",
    "parameters": [
      { "name": "val1", "vba_type": "Double", "mode": "ByRefOrDefault" },
      { "name": "val2", "vba_type": "Double", "mode": "ByRefOrDefault" }
    ],
    "return_type": "Double"
  },
  "type_map": {
    "status": "excel-observed",
    "argument_rules": ["number-to-double", "number-to-double"],
    "return_rule": "double-to-number"
  },
  "formula_call": "=AddThem(2,3)",
  "excel_oracle_ref": "retained://excel/vba-udf/VBA-UDF-T001",
  "dna_result": { "comparison_value": { "kind": "number", "value": 5.0 } },
  "verdict": "matched"
}
```

Boundary rule:
1. OxFml/OxFunc remain authoritative for formula argument values and formula
   result values.
2. OxVba remains authoritative for VBA type semantics and execution.
3. DnaOneCalc owns the host bridge policy: which cross-boundary conversions are
   admitted, rejected, or provisional for the current workspace.
4. Excel-observed behavior decides the target when OxFml/OxFunc and OxVba expose
   multiple plausible representations.

## 10.2 Excel Oracle Harness Requirement

The typed UDF lane must include a Windows Excel comparison harness before the
type matrix widens beyond the smallest scalar slice.

Harness goal:
1. create or open a workbook containing a VBA module with one or more UDFs,
2. write worksheet formulas that call those UDFs with controlled argument
   values,
3. calculate in Excel,
4. capture formula text, raw cell value, displayed text, error state, and enough
   VBA/UDF provenance to identify the tested signature,
5. emit retained OxXlPlay observation bundles and OxReplay comparison views,
6. run the same case through DnaOneCalc + OxVba,
7. compare the typed value/result/error surfaces through OxReplay.

The first comparison family should include:
1. `Double -> Double` happy path, for example `AddThem(2,3)`.
2. numeric coercion probes across `Integer`, `Long`, `Double`, `Currency`, and
   `Boolean`.
3. text-to-number and number-to-text probes.
4. blank/empty/error argument probes, initially expected to classify as blocked
   or not admitted until the exact behavior is retained.

This is an upstream harness expansion across OxXlPlay and OxReplay. DnaOneCalc
must track it through handoff docs and beads, not by modifying sibling repos from
this working tree.

### 10.3 Verification Entry Points

The first DnaOneCalc implementation should add verification through existing
host paths instead of creating a separate runner family:
1. extend `VerificationBatchRequest` / `ProgrammaticFormulaCase` with optional
   VBA UDF project/case metadata,
2. add a CLI command only if the existing `verify-formula` and
   `verify-xml-cell` shapes cannot carry the required project association,
3. prefer a new command shape:

```powershell
cargo run -p dnaonecalc-host -- verify-vba-udf `
  --case-id VBA-UDF-T001 `
  --formula "=AddThem(2,3)" `
  --vba-project .\fixtures\vba_udf\AddThem.basproj `
  --excel-oracle-root .\target\onecalc-verification\vba-udf\excel `
  --output-root .\target\onecalc-verification\vba-udf\dna
```

4. keep `scripts/run-vba-udf-oracle.ps1` as the first retained-oracle command
   wrapper,
5. add the new script to the integration or compare-regression family only after
   it can run in a gated way on non-Windows hosts.

Windows gating:
1. live Excel capture is Windows-only and must be skipped or marked blocked when
   Excel COM automation is unavailable,
2. retained Excel oracle bundles may be consumed cross-platform,
3. CI should be able to run the DnaOneCalc/OxVba side against checked-in or
   retained oracle fixtures without live Excel.

## 11. Naming And Collision Policy

First policy:
1. formula-visible names are case-insensitive.
2. unqualified public function names are admitted only when globally unique
   across all enabled VBA associations and built-in function names.
3. on collision, default behavior is reject both conflicting UDF registrations
   with visible diagnostics.
4. optional future policy may admit qualified names such as
   `<Project>.<Module>.<Function>`, but that must not introduce worksheet graph
   semantics.
5. built-in OxFunc functions take precedence over VBA UDFs unless a later
   explicit override policy is designed and accepted.

## 12. Capability And Persistence Requirements

The capability ledger must report:
1. OxVba dependency identity and version/source pin,
2. associated project ids and load state,
3. enabled runtime profiles and policy presets,
4. admitted/rejected/deferred VBA UDF counts,
5. root object surface: `Application.Version` only in the first tier,
6. unsupported host object members,
7. invocation support level by typed matrix row, not only `scalar-only`,
8. Excel-oracle evidence availability for each admitted mapping,
9. array/error/object/Variant/ByRef/optional/ParamArray states as blocked or
   deferred until implemented and observed,
10. upstream gaps affecting the current host.

Workspace persistence must round-trip project associations and last-known
registration state. Retained runs must record the exact association set,
descriptor set, and capability snapshot used for evaluation.

## 13. Diagnostics And Evidence

Every load and registration pass must emit structured diagnostics for:
1. project discovery/load failure,
2. OxVba compile/bind/runtime diagnostics,
3. host root injection failure,
4. rejected UDF candidates,
5. OxFml publication failure,
6. invocation conversion failure,
7. OxVba invocation failure,
8. result conversion failure.
9. missing Excel-oracle evidence for a requested type-map row.
10. live Excel capture unavailable on the current platform.
11. OxReplay comparison-view coverage gap.

First acceptance evidence:
1. workspace with one associated `.basproj` containing `AddThem`.
2. `Application.Version` VBA function returns the DnaOneCalc version string.
3. `=AddThem(2,3)` evaluates through OxFml to `5`.
4. the same `AddThem` case has an Excel-observed retained bundle, an OxReplay
   comparison view, and a DnaOneCalc/OxVba retained run tied to the same typed
   UDF signature.
5. a non-admitted function remains visible in the VBA catalog with a rejection
   reason.
6. collision between two projects is rejected deterministically.
7. disabling an association removes its functions and invalidates affected
   formula binding.

### 13.1 Completion Gates

WS-15 should execute in these gates:

| Gate | Scope | Completion evidence |
|---|---|---|
| `G0 Spec floor` | Current planning/spec/handoff state | this document, workset register entry, handoff docs, and child beads exist; `check-worksets` passes |
| `G1 Local association model` | project association state and persistence | unit tests round-trip association state and source fingerprint/catalog generation |
| `G2 OxVba load + Application.Version` | direct host session and first root object | fixture project returns DnaOneCalc version; missing members produce structured diagnostics |
| `G3 Catalog + publication` | `HostUdfCatalog` to OxFml runtime library context | `AddThem` is published as a catalog-known `vba_host_callback`; rejected candidates stay visible |
| `G4 Typed local invocation` | `VBA-UDF-T001` DnaOneCalc/OxVba execution | `=AddThem(2,3)` evaluates to numeric `5` through the OxFml host-function provider |
| `G5 Excel oracle harness` | retained Excel observation and replay comparison | retained Excel bundle plus DnaOneCalc run compare through OxReplay as matched for `T001` |
| `G6 Blocked matrix discipline` | non-evidenced rows stay blocked | `T002` through `T008` appear as blocked/deferred with reasons and no silent coercion |
| `G7 Capability and retained evidence` | product evidence and capability snapshot | retained run records association, signature, type map, oracle ref, descriptors, and verdict |

No gate may close on documentation alone except `G0`.

## 14. Implementation Lanes

Initial epics:
1. runtime host module and association persistence,
2. DnaOneCalc `Application` root object bridge,
3. OxVba project load/session lifecycle,
4. UDF catalog discovery and admission policy,
5. OxFml runtime library-context publication bridge,
6. typed invocation adapter and Excel-observed conversion matrix,
7. OxXlPlay/OxReplay Excel-oracle comparison harness,
8. capability/evidence/diagnostics integration,
9. upstream pressure and handoff capture for OxVba, OxXlPlay, OxReplay, OxFml,
   or OxFunc gaps.

First implementation should stay narrow: one project, one Excel-observed typed
scalar UDF, and `Application.Version`. Multi-project behavior should then be
added with collision and disable/removal tests before any richer host object or array
support.

### 14.1 Concrete First Slice

The first implementation slice is:
1. one file/path-backed `.basproj`,
2. one standard module,
3. one function: `AddThem(Double, Double) As Double`,
4. one formula: `=AddThem(2,3)`,
5. one DnaOneCalc host root property: `Application.Version`,
6. one typed conversion row: number to VBA `Double`, VBA `Double` to formula
   number,
7. one retained Excel oracle case for the same formula,
8. one OxReplay comparison-view family set:
   - `vba_udf_signature`,
   - `vba_udf_argument_values`,
   - `vba_udf_result_value`,
   - `vba_udf_error_state`,
   - `vba_udf_display_text`,
   - `vba_udf_coercion_observation`.

Everything else is blocked, deferred, or future work until this slice is green.

### 14.2 Required Test Families

Required DnaOneCalc tests:
1. association state serializes and refreshes by source fingerprint,
2. `Application.Version` fixture returns the host version,
3. public procedural function catalog admits `AddThem`,
4. public subs, class methods, document-module procedures, unsupported types, and
   collisions are rejected with reasons,
5. OxFml publication uses a catalog-known `vba_host_callback` library-context
   entry,
6. host-function provider invokes `AddThem` and returns `5`,
7. blocked matrix rows do not invoke,
8. retained invocation sidecar contains signature, type map, oracle ref, and
   verdict,
9. compare regression can consume retained Excel oracle fixture cross-platform,
10. live Excel capture path is visibly gated on non-Windows or missing Excel.

Required upstream-dependent tests once handoffs land:
1. OxVba typed UDF contract carries exact signature and typed results,
2. OxXlPlay captures VBA UDF workbook/module/formula observations,
3. OxReplay compares the six VBA UDF comparison-view families.

### 14.3 Work Not Yet Pinned

The following are intentionally not pinned for implementation until after the
first slice:
1. qualified formula names for duplicate VBA UDFs,
2. arrays/ranges,
3. `Variant`,
4. `Empty` / `Null`,
5. Excel errors as arguments or returns,
6. object parameters or object returns,
7. optional parameters and default values,
8. `ParamArray`,
9. ByRef writeback,
10. `Application.Caller`, `Application.Volatile`, and dependency registration.

## 15. Open Questions

1. Whether DnaOneCalc should use OxVba source sessions, compiled `.oxb` bundles,
   or both as first-class association targets.
2. The exact stable formula-visible naming model for qualified VBA functions.
3. Whether OxVba should replace or complement `invoke_host_udf_with_variants`
   with an explicit typed UDF invocation contract for Excel-compatible
   worksheet calls.
4. The minimum type-text generation needed for OxFml/OxFunc registration
   descriptors.
5. Which OxReplay comparison-view families should carry VBA UDF typed argument,
   return, error, and coercion observations.
6. Whether array/error result support should wait for OxVba host-program rework
   closure.
7. Whether `Application.Version` should be exposed through an OxVba host bridge
   object descriptor, a host-injected project module, or the narrowest available
   current direct-host API.
