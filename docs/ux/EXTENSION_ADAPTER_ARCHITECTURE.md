*Posted by Codex agent on behalf of @govert*

# DNA Calc Native Extension Adapter Architecture

Status: implementation-ready proposal; no production loader, VBA, XLL, or COM code is authorized by this tranche.

## Placement and ownership

`dnacalc-extension-host-core` owns the host-neutral provider catalog, native `CalcValue` invocation, diagnostics, deterministic ticks, RTD topic state, invalidation coalescing, and teardown. It remains safe Rust and has no UI, OxCalc, OxDoc, Tauri, COM, or loader dependency.

Adapters are separate trust boundaries:

| Adapter | Profiles | Owned boundary |
|---|---|---|
| `dnacalc-extension-oxvba` | WindowsDesktop and WindowsHeadless initially | Load OxVba projects, derive an explicitly reviewed worksheet-function surface, own VM sessions, and translate calls into `ExtensionProvider`. OxVba models WASM runtime classes, but its full host/session graph and JS bridge are not currently a proved browser integration surface. |
| `dnacalc-extension-xll-windows` | WindowsDesktop/Headless only | DLL lifetime, Excel C API ABI, `xlAutoOpen`/registration, argument/result marshaling, crash containment, unload. |
| `dnacalc-extension-rtd-com-windows` | WindowsDesktop/Headless only | COM activation, apartment ownership, topic connect/disconnect, update delivery, reconnect, shutdown. |
| native companion | HostedWeb only | Authenticated local transport to the same provider/catalog commands; browser remains `native_providers=false`. |

## Registration transaction

1. Adapter opens and validates the artifact without publishing functions.
2. It produces a provider id, immutable function metadata, required permissions, and diagnostics.
3. Core preflights duplicate provider/function ids and profile capabilities.
4. Core commits the provider and complete function set atomically, increments catalog generation, and emits one `FunctionCatalogChanged` invalidation.
5. On any failure the adapter closes the provisional resource; no partial catalog entries remain.

OxFml/OxFunc integration consumes the committed metadata through their native registry/provider contracts. Hosts map catalog invalidation to rebind/recalculation; semantic function/value enums are never mirrored here.

## Current OxVba seam (observed 2026-07-10)

The adapter targets current public crates, not a hypothetical umbrella facade:

- `oxvba-project` provides `load_basproj`, `load_vbp`, and `load_project_closure`. The closure is a leaf-first `Vec<oxvba_symbol::manifest::SymbolProjectManifest>` with the entry project last. `LoadedProject`, `BasProjError`, `OutputType`, `BuildTarget`, and module/reference manifest types are public.
- `oxvba-host` provides `Engine`, `HostConfig`, `RuntimeProfileId`, `HostProfileProvider`, `PhaseDiagnostic`, and `ProjectRuntimeSession`. Closure execution is `Engine::execute_project_closure_with_variant_snapshot`; retained object hosting is `prepare_image_session`/`prepare_image_session_bytes`, then `create_class_instance` and `invoke_member_values`.
- Retained-session calls accept and return `oxvba_runtime::Variant`; the optional member hint is `oxvba_bundle::ProjectMemberKind`. Object values are ref-counted `ObjectRef`s.
- `oxvba-diagnostics` exposes `Diagnostic`, code, severity, phase, source, labels, causes, VBA error number, and metadata. `PhaseDiagnostic::diagnostic()` exposes the structured record.

There is no current public API that directly enumerates worksheet UDFs and registers them with OxFunc. `execute_project_closure_with_variant_snapshot` runs the entry point; it is not per-function invocation. `ProjectRuntimeSession::invoke_member_values` invokes a member on a created project-class object; it does not publish standard-module functions. The first implementation bead must therefore obtain an upstream OxVba export/invocation facade or define a narrowly reviewed path from `oxvba-symbol` export surfaces plus a retained VM image. DnaTreeCalc must not scrape source or expose every public VBA member accidentally.

The proposed dependency boundary is `oxvba-project`, `oxvba-symbol` only for the approved export surface, `oxvba-host`, `oxvba-runtime`, `oxvba-bundle`, and `oxvba-diagnostics`. `oxvba-build` is not the normal UDF path: its current public build entry point targets wrapped COM servers and emits `.oxi`, COM descriptors, IDL, typelib, and DLL paths.

### Marshaling, lifetime, and diagnostics

The adapter owns one `Engine` and retained runtime session per provider generation. It converts `CalcValue` to `Variant` immediately before a call and converts the returned `Variant` before releasing the invocation guard. The v1 mapping is closed: blank and missing remain distinct; Boolean, bounded integer/double, string, Excel error, and bounded rectangular arrays are supported. Currency, Date, Decimal, by-reference writeback, UDTs, objects, COM records, jagged arrays, non-zero lower bounds, and callbacks are rejected until a later bead defines a lossless mapping.

Array conversion validates rank, each bound, element count, and allocation budget first. VBA error number and Excel calculation error are not interchangeable; a VBA error remains a diagnostic unless the approved contract maps that exact failure. No `Variant`, `ObjectRef`, borrowed buffer, SAFEARRAY pointer, or VM-owned handle crosses into host core or the companion protocol.

`ProjectRuntimeSession` is not treated as freely cloneable, movable, or concurrent. Calls serialize unless OxVba documents a stronger contract. Unregister enters `Draining`, rejects new calls, waits for invocation guards and callbacks, clears event sinks, drops adapter-held `ObjectRef`s, then drops session and engine. The current `prepare_image_session` promotes image and host storage to process lifetime; record this as a lifetime cost, not as unloadable memory. Repeated registration needs a bounded policy or an upstream disposable-session seam before production authorization.

OxVba `Diagnostic` records are preserved structurally, including code, severity, phase, source, labels, notes/help, causes, VBA error number, and metadata. Provider/function/call ids live in the enclosing adapter diagnostic; OxVba codes and messages are not rewritten. Panics become adapter-fault diagnostics and never silently become spreadsheet values.

## Security and diagnostics

Artifacts are denied by default outside an explicit allowlist/root. Record canonical path, hash, signer evidence where available, adapter/profile, requested capabilities, and load time. Hosted-web companion transport uses a per-session secret, loopback-only binding, origin check, size limits, and protocol version negotiation. Diagnostics are typed at discovery, validation, registration, invocation, update, and teardown stages; native error codes and causal provider ids are retained.

## XLL threading and unsafe boundary

`dnacalc-extension-xll-windows` is the only crate permitted to contain Excel C API FFI. `unsafe` is confined to a small `ffi` module with audited pointer/length conversions and a safe provider facade. Loading and registration occur on one owning native thread. Thread-safe declarations are honored only after metadata validation; otherwise calls serialize on the owner. Panics never cross FFI. Shutdown blocks new calls, drains in-flight calls, unregisters functions, calls the supported close hook, then frees the library. A failed unload quarantines the module until process exit.

An ordinary XLL expects Excel's callback/export surface (`Excel12`/`Excel12v`, commonly imported through `xlcall32`). DnaCalc therefore does not call `xlAutoOpen` directly in the product process. A sacrificial Windows XLL companion process owns the DLL and exposes an `xlcall32` compatibility shim before loading it. The shim implements only the versioned subset approved by fixtures: `xlGetName`, `xlGetHwnd`, `xlFree`, `xlCoerce`, `xlfRegister`, `xlfUnregister`, and the explicitly admitted async/thread registration calls. Every other callback returns the pinned Excel unsupported/not-applicable error and a typed diagnostic; it never fabricates Excel state.

During `xlAutoOpen`, intercepted `xlfRegister` calls become staged records containing module/procedure text, type text, worksheet surface name, argument names, macro/function flags, category/help text, thread/cluster/volatile/asynchronous flags, and the returned registration id. No record is visible to the shared catalog until `xlAutoOpen` returns successfully and every record validates. The adapter maps the staged set to native OxFunc `UdfRegistrationRequest`s and commits it with the provider transaction; failure returns registration ids through the shim's rollback path, calls the admitted close hook when safe, and terminates the companion. Calls from the XLL back through the shim execute on the owning thread; re-entry is bounded by a per-call stack/depth guard and unsupported re-entrant host mutations fail typed.

Invocation messages carry the captured registration generation, function id, bounded `XLOPER12` arguments, deadline, and request id. The companion owns all XLL-allocated values until the admitted `xlFree`/free-result contract completes; only validated owned DTOs cross back. A native fault, access violation, stack corruption, or timeout kills the sacrificial process. The host reports provider failure, invalidates that generation, quarantines the artifact hash, and may restart only into a new session/generation. This is the crash-containment claim. An optional future in-process mode may exist only for explicitly trusted/signed XLLs and must state that arbitrary native faults are **not** contained.

Lifecycle is `Absent -> CompanionStarting -> ShimReady -> LibraryLoaded -> AutoOpening -> RegistrationsStaged -> Active -> Draining -> AutoClosing -> CompanionStopped`. Failure before `Active` rolls staged registrations back in reverse order. A detected contract violation moves to `Quarantined`; a native fault terminates the companion and then quarantines the artifact generation in the host. Re-entry during opening or shutdown is rejected. Compatibility is not claimed until the Excel C API version, supported callback table, `XLOPER12` ownership/freeing rules, calling convention, async handles, and thread-safe/cluster-safe flags are fixture-pinned.

## COM RTD apartment model

`dnacalc-extension-rtd-com-windows` owns a dedicated STA thread: `CoInitializeEx(COINIT_APARTMENTTHREADED)`, activation, message pumping, server calls, and `CoUninitialize` all occur there. Host commands cross a bounded channel. Topic updates enter `RtdTopicState`; multiple updates coalesce by topic/version before `RtdTopicChanged`. Disconnect is idempotent. Shutdown stops subscriptions, calls server termination on the STA, releases interfaces there, drains the pump, then joins the thread. Reconnect uses bounded exponential backoff and never silently reuses stale topic values.

Server lifecycle is `Dormant -> StaStarting -> Activating -> Started -> Running -> Stopping -> Stopped`; activation, server-call, or pump failure enters `Faulted(backoff_attempt)`. Topic lifecycle is `Absent -> Connecting -> Connected(epoch, version) -> Disconnecting -> Absent`. Reconnect creates new epochs so old queued updates cannot win. `UpdateNotify` schedules refresh on the STA and never calls `RefreshData` re-entrantly. Refresh output must be a bounded two-row topic/value matrix before publication.

## Deterministic invalidation

The host owns the invalidation sink. Catalog generations precede volatile ticks, which precede RTD topics sorted by topic id. Tick ids are monotonic and injected by the host scheduler, never wall-clock-derived in tests. OneCalc maps events to formula-space rebind/recalculation. TreeCalc maps them to OxCalc dependency invalidation/recalculation. BrowserWasm rejects native providers explicitly; HostedWeb advertises only native-companion placement.

## Hosted-web companion transport

Commands are versioned equivalents of discover/register/invoke/subscribe/unsubscribe/teardown. No Ox runtime type crosses the transport: requests and responses use stable extension protocol DTOs; the companion converts to native `CalcValue` internally and the host projects results at its boundary. Disconnect tears down session-owned providers and topics. Reconnect creates a new session and requires fresh registration.

State is `Disconnected -> Connecting -> Authenticating -> Negotiating -> Ready -> Closing -> Disconnected`. No discovery or artifact command is accepted before authentication and negotiation. Requests carry session id, monotonic request id, protocol version, deadline, and bounded size; late responses from prior sessions are discarded. Transport loss revokes the secret, cancels calls, and drains providers/topics. Browser capability remains `native_providers=false`; `native_companion=true` is advertised only in `Ready`.

DTOs use only the closed scalar/error/rectangular-array subset. Artifact selection uses a companion-issued opaque handle after local allowlist validation, never an arbitrary browser path.

## OxVba provider state machine and WASM boundary

`Unregistered -> LoadingProject -> ResolvingExports -> CompilingImage -> SessionStarting -> Staged -> Active -> Draining -> Stopped` is the success path. Load/bind/elaboration failure rolls back without catalog publication. The atomic catalog commit is `Staged -> Active`. Calls accepted in `Active` hold a generation guard, so old-generation results cannot publish after replacement. Source changes create and atomically swap a new generation; they do not mutate a live VM session.

Profiles are selected explicitly with OxVba `RuntimeProfileId` and checked against `Engine::hal_descriptor()`. Windows desktop/headless are initial candidates. `WasmWasiLocal` and `WasmBrowserSandbox` are policy/runtime classes, not proof that this adapter or the full `oxvba-host` graph builds and runs in browser WASM. Browser use remains rejected until a dedicated target check, package/bridge implementation, and sandbox test authorize it.

## Delivery gates

Each adapter requires profile-rejection, rollback, structured-diagnostic preservation, invalidation ordering, generation isolation, bounded marshaling, and idempotent teardown tests. XLL adds malformed `XLOPER12`, re-entry, crash/quarantine, and unload fixtures. RTD adds STA, callback non-reentrancy, malformed refresh matrices, reconnect epochs, topic coalescing, and shutdown tests. OxVba adds project-closure fixtures, approved-export filtering, `CalcValue`/`Variant` scalar and rectangular-array round trips, diagnostic equality, repeated-generation lifetime accounting, and teardown-under-call tests. Hosted-web adds authentication-before-command, origin, replay, disconnect cancellation, size limits, and version skew. A WASM gate may claim only the crates and target actually checked; OxVba sandbox parity is not currently a delivery claim. Production work starts only after separate beads authorize the platform/unsafe surface and the missing standard-module invocation seam is resolved.
