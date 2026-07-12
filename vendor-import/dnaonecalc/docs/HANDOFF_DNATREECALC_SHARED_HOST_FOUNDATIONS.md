*Posted by Codex agent on behalf of @govert*

# DnaTreeCalc Handoff: Shared DNA Calc Host Foundations

Status: Landed and accepted by DnaOneCalc

Target: `DnaTreeCalc`

Ask: land the canonical shared Skin IR, formula UX, formula Leptos, runtime-profile,
extension-host-core, and native-shell foundations described below from a
DnaTreeCalc-scoped run.

## Why This Handoff Exists

DnaOneCalc now consumes `DnaTreeCalc/src/dnacalc-skin-ir` directly, but agents
working in this repository may not write or commit sibling repositories. The
first coordinated implementation pass left uncommitted shared-crate changes in
DnaTreeCalc. They must be reviewed and landed by a TreeCalc-scoped run before
the corresponding OneCalc adoption can be committed safely.

Observed baselines:

1. DnaTreeCalc: `9d7bcb26340a0a92379e542a550d102b7bd9b89d` on `main`.
2. DnaOneCalc: `c8fd8f89fef95bb73e83512bdce9fd86bd417eda` on `main`.

Pending TreeCalc work observed from DnaOneCalc:

1. `src/dnacalc-skin-ir/src/lib.rs` exports the shared formula and protocol
   modules.
2. untracked `src/dnacalc-skin-ir/src/formula.rs` defines the OneFormula editor,
   assist, result, formatting, conditional-formatting, drill, and projected
   value protocol.
3. untracked `src/dnacalc-skin-ir/src/protocol.rs` defines `SkinSnapshot`,
   `SkinDocumentProjection`, `SkinIntent`, shell/persistence projections, and
   host capabilities.
4. `.scratch/` is unrelated and must not be staged.

Pending OneCalc work consumes those files through the direct path dependency
and currently changes five tracked files (`Cargo.lock`, the host manifest,
reducer, home-shell projection, and home-shell component). Do not change those
files from the TreeCalc run.

## Required Shared Crate Shape

### `dnacalc-skin-ir`

Keep this crate serde-only and free of Ox, Leptos, Tauri, filesystem, COM, and
native-loader dependencies.

Complete the current protocol with:

1. real recent-document rows, dirty/current-path state, and `Save`, `SaveAs`,
   `Open`, and `OpenRecent` intents,
2. result-array and drill-node array-window requests,
3. formula editing, completion, signature/function help, value presentation,
   formatting, conditional formatting, diagnostics, comparisons, and drill
   traces,
4. host capability projections that distinguish null-reference OneCalc from
   reference-backed TreeCalc and declare extension placement honestly,
5. schema/version validation, unknown-variant rejection, serde round trips, and
   stable OneFormula and Tree/workspace golden fixtures.

### `dnacalc-formula-ux-core`

Create a host-neutral crate depending only on `dnacalc-skin-ir`, `oxfml_core`,
and `oxfunc_core`. It projects native editor/runtime documents, registry
metadata, `CalcValue`/`CoreValue`, presentation hints, comparison views, and
formula drill traces into Skin IR. It must not introduce copied semantic value
enums, bridge packets, parse/bind/eval summaries, or host-specific state.

Public entry points should cover editor/assist, runtime result, formatting/CF,
drill trace, and bounded array-window projection. The array API must preserve
typed shape and permit a caller to request another window without parsing
display text.

### `dnacalc-formula-skin-leptos`

Create the shared Leptos formula surface over Skin IR only. It owns formula
editing, IntelliSense, signature/function help, value/array rendering,
formatting/CF controls, and drill-tree rendering. It emits `SkinIntent` and
never calls OxFml/OxFunc/OxCalc/OxDoc directly. Headerless array grids must use
stable row sizing, and intermediate array nodes must expose bounded preview and
explicit expansion.

### Runtime Profiles And Session Boundary

Put shared profile and capability protocol in an Ox-independent crate or the
existing pure protocol layer. Required profiles are `BrowserWasm`, `HostedWeb`,
`WindowsDesktop`, `WindowsHeadless`, `NativeUnix`, and `NullTest`.

Both hosts should exchange `SkinSnapshot`/delta, enveloped `SkinIntent`, and
typed receipts across in-process, worker, native, or later remote transports.
No Ox runtime type crosses that transport.

Add a minimal Tauri 2 shell for TreeCalc over the existing Leptos UI and
`dnacalc-host-core`. It proves native process placement only; it must not load
XLLs or connect to COM RTD servers in this tranche.

### `dnacalc-extension-host-core`

Create the shared host-neutral extension substrate under TreeCalc W010. It may
depend on current OxFml/OxFunc provider contracts and native `CalcValue`, but
not on Leptos, OxCalc, OxDoc, Tauri, COM, or a dynamic loader.

Required contracts:

1. runtime profiles and extension capability snapshots,
2. provider/catalog lifecycle and registration transactions,
3. typed diagnostics and host-function/library-context composition,
4. `HostInvalidationEvent::{FunctionCatalogChanged, VolatileTick,
   RtdTopicChanged}` plus a host-owned invalidation sink,
5. deterministic tick scheduling and host-neutral RTD topic subscription/update
   state,
6. explicit teardown and update-coalescing behavior.

OneCalc will map invalidations to formula-space rebind/recalculation. TreeCalc
will map them to OxCalc/OxDoc dependency invalidation and recalculation.

Do not promote the current OneCalc-local `onecalc-native-extension-abi-v0`,
Linux `.so` parity, or minimal COM-like RTD claims as shared architecture
without new evidence and review.

## Required Extension Design Proposal

Land one implementation-ready proposal beside the shared extension core for:

1. `dnacalc-extension-oxvba`, consuming current OxVba APIs under supported
   native and sandbox/WASM profiles,
2. `dnacalc-extension-xll-windows`, owning Windows DLL/XLL loading and the
   Excel C API boundary,
3. `dnacalc-extension-rtd-com-windows`, owning apartment-aware COM activation,
   topic subscription, update delivery, reconnect, and shutdown,
4. hosted-web native-companion placement and transport,
5. registration, diagnostics, security boundaries, invalidation, and teardown.

Any unavoidable FFI/COM `unsafe` must be isolated in dedicated Windows adapter
crates with audited safe interfaces and a narrowly scoped lint exception. The
shared core and both host cores remain safe Rust.

## Verification

1. `cargo test -p dnacalc-skin-ir` and no-Ox/no-Leptos dependency guards.
2. formula-core tests for `=SUM(1,2,3)`, `=SEQUENCE(2,2)`, `=SU`, `=SUM(`,
   placeholder-signature suppression, presentation hints, CF, and expandable
   intermediate arrays.
3. shared formula-skin browser tests for editing, help, headerless arrays,
   drill expansion, formatting, and CF.
4. TreeCalc host/web/worker checks plus the minimal Tauri check.
5. extension-core tests for profile rejection, catalog lifecycle, native
   `CalcValue` invocation, invalidation ordering/coalescing, deterministic
   ticks, topic updates, and teardown. No DLL or COM integration test is part
   of this tranche.

## Coordination Back To DnaOneCalc

Append the landed TreeCalc commit SHA and any changed public API names here.
DnaOneCalc will then commit its Skin IR adoption, remove local semantic
projections, split its host core, adopt the shared extension core, and delete
the deferred pseudo-OxVba structures. Do not close this handoff until the
shared commits and their acceptance commands are recorded.

## Downstream Validation Before Handoff Landing

Using the current uncommitted TreeCalc Skin IR files, the pending OneCalc
consumer changes pass:

1. `cargo check -p dnaonecalc-host`,
2. `cargo test -p dnaonecalc-host --lib` (`417` passed),
3. `cargo test -p dnaonecalc-host --test scenarios` (`19` passed),
4. `cargo check -p dnaonecalc-host --target wasm32-unknown-unknown`,
5. package-name dependency guard: `dnacalc-skin-ir`, `oxfml_core`, and
   `oxfunc_core` present; `oxcalc*`, `oxdoc*`, `oxvba*`, `dnatreecalc-*`, and
   `dnacalc-host-core` absent.

These results prove the current API shape is consumable, but they do not make
the OneCalc change commit-safe: a clean checkout of the recorded TreeCalc
baseline does not contain the untracked shared protocol modules. OneCalc bead
`dno-yjk.14` therefore remains blocked by cross-repo gate `dno-uh9y.5`.

## Landed DnaTreeCalc Foundation

The DnaTreeCalc-scoped implementation landed on `main` as the following focused
commit sequence:

1. `df8c267` — move filesystem skin state into the shared host core,
2. `45fad393` — land the canonical shared Skin IR protocol,
3. `256bdc9` — project native formula UX into Skin IR,
4. `d60b984` — carry drill array-window offsets,
5. `c6ee55a` — ship the shared Leptos formula surface,
6. `ecd460b` — align host and worker session protocol,
7. `1a9e2eb` — prove native host placement with Tauri 2,
8. `c2ad049` — land the shared extension-host core,
9. `9bb4e65` — specify native extension adapter boundaries.

The public consumer surface is now:

1. `dnacalc-skin-ir`: `SkinSnapshot`, `SkinDocumentProjection`, `SkinIntent`,
   `SkinIntentEnvelope`, `SkinIntentReceipt`, `SkinShellProjection`,
   `PersistenceProjection`, `HostCapabilityProjection`, runtime profiles, formula
   editor/assist/result/formatting/drill projections, and bounded array windows.
2. `dnacalc-formula-ux-core`: `project_editor`, `project_completion`,
   `project_signature`, `project_signature_with_help`, `project_function_help`,
   `project_runtime_result`, `project_calc_value`, `project_comparison`,
   `project_formatting`, `project_drill`, and `project_array_window`.
3. `dnacalc-formula-skin-leptos`: `FormulaSurface`, `result_expand_intent`, and
   `drill_expand_intent` over Skin IR only.
4. `dnacalc-extension-host-core`: `RuntimeProfile`, `ExtensionCapabilities`,
   `ExtensionCatalog`, provider registration and teardown, native `CalcValue`
   invocation, `HostInvalidationEvent`, invalidation sinks, deterministic ticking,
   and `RtdTopicState`.

DnaOneCalc acceptance consumes these crates directly from the clean TreeCalc
`9bb4e65` tree. The OneCalc validation record is maintained in the accepting bead
and downstream adoption commits; no DnaTreeCalc working-tree mutation is performed
from this repository.
