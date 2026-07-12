//! Bench extensions manager v0 + feed instruments — BENCH_SPEC §6, mechanism
//! 18 (MECHANISMS.md), bead dtc-lfz.6. G7 MINIMAL slice: read-only provider
//! inventory + states + diagnostics, per-runtime honesty as a LOOKUP (never
//! skin-side logic), no lifecycle actions.
//!
//! LAYERING: this module is display over `HostCapabilityProjection`
//! (`dnacalc-skin-ir`) — the IR capability projection `dnacalc-bench-host`
//! now derives honestly from `dnacalc-extension-host-core`'s real
//! per-runtime capability gate (`RuntimeProfile::capabilities()`) instead of
//! a hardcoded value (see
//! `dnacalc_bench_host::extensions::honest_extension_placement_for`, fixed
//! alongside this bead). No native code loading happens in this skin.
//!
//! HONESTY DISCIPLINE (read before extending): no live provider catalog
//! exists anywhere in this product today. `dnacalc-extension-host-core`'s
//! `ExtensionCatalog` is a host-neutral framework that is never populated by
//! the running Bench host — no VBA/XLL/RTD adapter crate is implemented yet
//! (see `docs/ux/EXTENSION_ADAPTER_ARCHITECTURE.md`: "no production loader,
//! VBA, XLL, or COM code is authorized by this tranche"). So in PRODUCTION
//! [`BenchExtensionsOverlaySurface`] always resolves an EMPTY descriptor
//! list — the overlay's runtime banner (derived purely from the real
//! `HostCapabilityProjection`) is the only live signal, and the provider
//! table honestly renders "No providers are registered" rather than
//! fabricating rows or claiming a fake `Available` state. [`resolve_provider_rows`]
//! is proven with fixture descriptors in this module's tests so the
//! runtime-honesty transform is real and ready the moment a live catalog
//! surfaces (the S1.0 G7 ask).
//!
//! Function-to-provider attribution in the Inspector (BENCH_SPEC §6) is an
//! HONEST OMISSION this phase: no projection anywhere links "caret is on a
//! provided function" to a provider id (`HostCapabilityProjection` carries
//! no such map, and `OneFormulaProjection` carries no extension-context
//! field), so nothing is added to the Inspector — never fabricated.

use leptos::prelude::*;

use dnacalc_shell::{OverlayContext, OverlaySurface};
use dnacalc_skin_ir::protocol::{
    ExtensionPlacementProjection, HostCapabilityProjection, RuntimeProfileProjection,
};

/// A provider's kind (BENCH_SPEC §6). All four route through the same
/// native/companion placement gate today (`docs/ux/EXTENSION_ADAPTER_ARCHITECTURE.md`
/// lists per-kind adapters — `dnacalc-extension-oxvba`, `-xll-windows`,
/// `-rtd-com-windows`, a generic native provider — but none of them narrow
/// the *runtime* legality further than `ExtensionPlacementProjection`
/// already does). Kept distinct so a future finer per-kind ask only needs to
/// change [`runtime_hosts_native_extensions`], never the overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtensionProviderKind {
    Vba,
    Xll,
    Rtd,
    Native,
}

impl ExtensionProviderKind {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Vba => "VBA",
            Self::Xll => "XLL",
            Self::Rtd => "RTD",
            Self::Native => "native",
        }
    }
}

/// A provider's own reported state — what a live catalog (the S1.0 G7 ask)
/// would report for a provider that IS legally hostable on this runtime.
/// Deliberately excludes "unavailable on this runtime": that is always a
/// runtime-legality override, never something a provider reports about
/// itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtensionProviderNominalState {
    Available,
    Loading,
    Quarantined,
    Rejected,
}

/// The rendered state of a provider row — BENCH_SPEC §6's closed
/// vocabulary. A provider's nominal state passes through UNLESS the
/// runtime-legality lookup overrides it to `UnavailableOnRuntime`; the
/// override always wins over whatever the provider itself reports, so a
/// "loading" native provider on a browser runtime still renders honestly
/// unavailable rather than a stale/misleading nominal state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtensionProviderState {
    Available,
    Loading,
    Quarantined,
    Rejected,
    UnavailableOnRuntime { reason: String },
}

impl ExtensionProviderState {
    #[must_use]
    pub fn label(&self) -> String {
        match self {
            Self::Available => "available".to_string(),
            Self::Loading => "loading".to_string(),
            Self::Quarantined => "quarantined".to_string(),
            Self::Rejected => "rejected".to_string(),
            Self::UnavailableOnRuntime { reason } => format!("unavailable-on-runtime: {reason}"),
        }
    }
}

impl From<ExtensionProviderNominalState> for ExtensionProviderState {
    fn from(nominal: ExtensionProviderNominalState) -> Self {
        match nominal {
            ExtensionProviderNominalState::Available => Self::Available,
            ExtensionProviderNominalState::Loading => Self::Loading,
            ExtensionProviderNominalState::Quarantined => Self::Quarantined,
            ExtensionProviderNominalState::Rejected => Self::Rejected,
        }
    }
}

/// A provider as a live catalog (the S1.0 G7 ask) would describe it —
/// the INPUT to [`resolve_provider_rows`]'s runtime-honesty lookup. No
/// production caller populates this with real data yet (see the module
/// doc); it exists so the transform is real and testable with fixtures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionProviderDescriptor {
    pub name: String,
    pub kind: ExtensionProviderKind,
    pub nominal_state: ExtensionProviderNominalState,
    pub diagnostics: Vec<String>,
}

/// A resolved, runtime-honest provider row — the OUTPUT the overlay renders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionProviderRow {
    pub name: String,
    pub kind: ExtensionProviderKind,
    pub state: ExtensionProviderState,
    pub diagnostics: Vec<String>,
}

/// The legality LOOKUP (not skin logic): true when this runtime's
/// `HostCapabilityProjection.extension_placement` permits hosting a native
/// provider at all — `Unavailable` means no, `InProcess`/`NativeCompanion`
/// mean yes. This is the one place the "requires desktop or companion"
/// decision is made; nothing else in this module re-derives it.
#[must_use]
pub fn runtime_hosts_native_extensions(host_capabilities: &HostCapabilityProjection) -> bool {
    host_capabilities.extension_placement != ExtensionPlacementProjection::Unavailable
}

/// The honest reason a provider is unavailable on this runtime — used both
/// in the per-row override and the overlay's runtime banner.
#[must_use]
pub const fn unavailable_reason(profile: RuntimeProfileProjection) -> &'static str {
    match profile {
        RuntimeProfileProjection::BrowserWasm
        | RuntimeProfileProjection::HostedWeb
        | RuntimeProfileProjection::NullTest => "requires desktop or a companion process",
        RuntimeProfileProjection::WindowsDesktop
        | RuntimeProfileProjection::WindowsHeadless
        | RuntimeProfileProjection::NativeUnix => "extensions unavailable on this host",
    }
}

/// The runtime-honest LOOKUP (BENCH_SPEC §6): resolve each descriptor's
/// rendered state from `RuntimeProfileProjection` × `ExtensionPlacementProjection`
/// — the legality matrix `HostCapabilityProjection` already carries. A
/// descriptor's nominal state passes through untouched when this runtime
/// legally hosts native extensions ("desktop shows in-process catalog
/// data"); otherwise every row is honestly overridden to
/// `UnavailableOnRuntime` regardless of what it nominally reported
/// ("BrowserWasm shows native providers explicitly as requires desktop or
/// companion"). An empty input always yields an empty output — no
/// fabrication either way.
#[must_use]
pub fn resolve_provider_rows(
    descriptors: &[ExtensionProviderDescriptor],
    host_capabilities: &HostCapabilityProjection,
) -> Vec<ExtensionProviderRow> {
    let legal = runtime_hosts_native_extensions(host_capabilities);
    descriptors
        .iter()
        .map(|descriptor| {
            let state = if legal {
                ExtensionProviderState::from(descriptor.nominal_state)
            } else {
                ExtensionProviderState::UnavailableOnRuntime {
                    reason: unavailable_reason(host_capabilities.runtime_profile).to_string(),
                }
            };
            ExtensionProviderRow {
                name: descriptor.name.clone(),
                kind: descriptor.kind,
                state,
                diagnostics: descriptor.diagnostics.clone(),
            }
        })
        .collect()
}

/// The overlay's top-of-panel runtime banner — the ONE always-real signal
/// in production (no descriptors exist to list). Pure text derived from
/// `HostCapabilityProjection`, never fabricated per-provider detail.
#[must_use]
pub fn runtime_banner(host_capabilities: &HostCapabilityProjection) -> String {
    match host_capabilities.extension_placement {
        ExtensionPlacementProjection::Unavailable => format!(
            "Native providers are unavailable on this runtime — {}.",
            unavailable_reason(host_capabilities.runtime_profile)
        ),
        ExtensionPlacementProjection::InProcess => {
            "This runtime hosts extensions in-process.".to_string()
        }
        ExtensionPlacementProjection::NativeCompanion => {
            "This runtime reaches extensions through a companion process.".to_string()
        }
    }
}

/// The Bench Extensions overlay surface (BENCH_SPEC §2/§6): mounted through
/// `ShellOverlaySlots::extensions`, reached by clicking the Strip's Feeds
/// instrument (mechanism 18). Captures the host's real
/// `HostCapabilityProjection` signal at construction (`app.rs`'s
/// remount-per-projection pattern) — never re-derives it.
#[derive(Clone)]
pub struct BenchExtensionsOverlaySurface {
    pub host_capabilities: ReadSignal<HostCapabilityProjection>,
}

impl OverlaySurface for BenchExtensionsOverlaySurface {
    fn mount(&self, ctx: OverlayContext) -> AnyView {
        let host_capabilities = self.host_capabilities;
        let close = ctx.controls.close;
        view! {
            <div class="dna-overlay-backdrop">
                <div
                    class="dna-overlay bench-extensions"
                    role="dialog"
                    aria-label="Extensions"
                    data-overlay="extensions"
                >
                    <h2>"Extensions"</h2>
                    <p class="bench-extensions__banner" data-testid="extensions-banner">
                        {move || runtime_banner(&host_capabilities.get())}
                    </p>
                    {move || {
                        let caps = host_capabilities.get();
                        // PRODUCTION: no live catalog exists yet (see module
                        // doc) — always an honest empty input, never a
                        // fabricated row.
                        let rows = resolve_provider_rows(&[], &caps);
                        if rows.is_empty() {
                            view! {
                                <p class="bench-extensions__empty" data-testid="extensions-empty">
                                    "No providers are registered."
                                </p>
                            }
                                .into_any()
                        } else {
                            view! {
                                <ul class="bench-extensions__list" data-testid="extensions-list">
                                    {rows
                                        .into_iter()
                                        .map(|row| {
                                            let state_label = row.state.label();
                                            view! {
                                                <li
                                                    class="bench-extensions__row"
                                                    data-provider-kind=row.kind.label()
                                                    data-provider-state=state_label.clone()
                                                >
                                                    <span class="bench-extensions__name">{row.name}</span>
                                                    <span class="bench-extensions__kind">
                                                        {row.kind.label()}
                                                    </span>
                                                    <span class="bench-extensions__state">
                                                        {state_label.clone()}
                                                    </span>
                                                </li>
                                            }
                                        })
                                        .collect_view()}
                                </ul>
                            }
                                .into_any()
                        }
                    }}
                    {move || {
                        let caps = host_capabilities.get();
                        (!caps.unavailable_families.is_empty())
                            .then(|| {
                                view! {
                                    <p
                                        class="bench-extensions__diagnostics"
                                        data-testid="extensions-diagnostics"
                                    >
                                        {format!(
                                            "unavailable: {}",
                                            caps.unavailable_families.join(", "),
                                        )}
                                    </p>
                                }
                            })
                    }}
                    <button
                        type="button"
                        class="bench-extensions__close"
                        on:click=move |_| close.run(())
                    >
                        "Close"
                    </button>
                </div>
            </div>
        }
        .into_any()
    }
}

/// Component-scoped styles, resolved through the Strand `--dna-*` cascade
/// (same convention as `app.rs::BENCH_APP_CSS` / `xray.rs::XRAY_CSS`).
pub const EXTENSIONS_CSS: &str = "\
.bench-extensions__banner{font-size:13px;color:var(--dna-ink-2);margin:0 0 var(--dna-gap-3)}
.bench-extensions__empty{font-size:12px;color:var(--dna-ink-3);font-style:italic}
.bench-extensions__list{list-style:none;margin:0;padding:0;display:flex;flex-direction:column;gap:var(--dna-gap-2)}
.bench-extensions__row{display:flex;gap:var(--dna-gap-3);align-items:baseline;font-size:12px;padding:2px 0;border-bottom:1px solid var(--dna-line)}
.bench-extensions__kind{color:var(--dna-ink-3);text-transform:uppercase;font-size:10px}
.bench-extensions__state{margin-left:auto;font-size:11px}
.bench-extensions__diagnostics{font-size:11px;color:var(--dna-ink-3);margin-top:var(--dna-gap-3)}
.bench-extensions__close{margin-top:var(--dna-gap-4);border:1px solid var(--dna-line);background:var(--dna-paper-2);color:var(--dna-ink-2);border-radius:var(--dna-radius-chip);padding:2px 10px;cursor:pointer;font:inherit;font-size:12px}
";

#[cfg(test)]
mod tests {
    use super::*;

    fn caps(
        runtime_profile: RuntimeProfileProjection,
        extension_placement: ExtensionPlacementProjection,
    ) -> HostCapabilityProjection {
        HostCapabilityProjection::onecalc_null_references(runtime_profile, extension_placement)
    }

    fn descriptor(kind: ExtensionProviderKind) -> ExtensionProviderDescriptor {
        ExtensionProviderDescriptor {
            name: "Test Provider".to_string(),
            kind,
            nominal_state: ExtensionProviderNominalState::Available,
            diagnostics: Vec::new(),
        }
    }

    /// The browser/desktop split the bead's acceptance calls out by name: a
    /// native provider's nominal `Available` state is overridden to
    /// `UnavailableOnRuntime` under `BrowserWasm`/`Unavailable` — the
    /// runtime-honest lookup, not a fabricated pass-through.
    #[test]
    fn a_native_provider_shows_unavailable_on_browser_runtime() {
        let browser = caps(
            RuntimeProfileProjection::BrowserWasm,
            ExtensionPlacementProjection::Unavailable,
        );
        let rows = resolve_provider_rows(&[descriptor(ExtensionProviderKind::Native)], &browser);
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0].state,
            ExtensionProviderState::UnavailableOnRuntime {
                reason: "requires desktop or a companion process".to_string(),
            }
        );
    }

    /// Under a desktop runtime with `InProcess` placement, the SAME
    /// descriptor's nominal state passes through untouched — real
    /// "in-process catalog data", not overridden or fabricated.
    #[test]
    fn a_native_provider_shows_in_process_catalog_data_on_desktop_runtime() {
        let desktop = caps(
            RuntimeProfileProjection::WindowsDesktop,
            ExtensionPlacementProjection::InProcess,
        );
        let rows = resolve_provider_rows(&[descriptor(ExtensionProviderKind::Native)], &desktop);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].state, ExtensionProviderState::Available);
        assert_eq!(rows[0].kind, ExtensionProviderKind::Native);
        assert_eq!(rows[0].name, "Test Provider");
    }

    /// Every kind (VBA/XLL/RTD/native) is overridden identically on a
    /// runtime that cannot host native extensions — the lookup is a
    /// runtime-level gate, never a per-kind skin decision.
    #[test]
    fn every_kind_is_overridden_unavailable_on_a_native_incapable_runtime() {
        let browser = caps(
            RuntimeProfileProjection::BrowserWasm,
            ExtensionPlacementProjection::Unavailable,
        );
        let descriptors = [
            descriptor(ExtensionProviderKind::Vba),
            descriptor(ExtensionProviderKind::Xll),
            descriptor(ExtensionProviderKind::Rtd),
            descriptor(ExtensionProviderKind::Native),
        ];
        let rows = resolve_provider_rows(&descriptors, &browser);
        assert_eq!(rows.len(), 4);
        assert!(rows.iter().all(|row| matches!(
            row.state,
            ExtensionProviderState::UnavailableOnRuntime { .. }
        )));
    }

    /// A non-`Available` nominal state (e.g. a quarantined provider) still
    /// passes through faithfully when the runtime is legal — the override
    /// only ever fires for runtime illegality, never masks a provider's own
    /// reported trouble.
    #[test]
    fn a_quarantined_provider_passes_through_on_a_legal_runtime() {
        let desktop = caps(
            RuntimeProfileProjection::WindowsDesktop,
            ExtensionPlacementProjection::InProcess,
        );
        let mut quarantined = descriptor(ExtensionProviderKind::Xll);
        quarantined.nominal_state = ExtensionProviderNominalState::Quarantined;
        quarantined.diagnostics = vec!["hash mismatch".to_string()];
        let rows = resolve_provider_rows(&[quarantined], &desktop);
        assert_eq!(rows[0].state, ExtensionProviderState::Quarantined);
        assert_eq!(rows[0].diagnostics, vec!["hash mismatch".to_string()]);
    }

    /// Honest absence: an empty descriptor list (production's real input,
    /// since no live catalog exists yet) always yields an empty row set,
    /// regardless of runtime — no kind/row is ever fabricated to fill the
    /// gap.
    #[test]
    fn empty_descriptors_never_fabricate_rows_on_any_runtime() {
        for capabilities in [
            caps(
                RuntimeProfileProjection::BrowserWasm,
                ExtensionPlacementProjection::Unavailable,
            ),
            caps(
                RuntimeProfileProjection::WindowsDesktop,
                ExtensionPlacementProjection::InProcess,
            ),
            caps(
                RuntimeProfileProjection::HostedWeb,
                ExtensionPlacementProjection::NativeCompanion,
            ),
        ] {
            assert!(resolve_provider_rows(&[], &capabilities).is_empty());
        }
    }

    #[test]
    fn runtime_banner_is_honest_per_placement() {
        let browser = caps(
            RuntimeProfileProjection::BrowserWasm,
            ExtensionPlacementProjection::Unavailable,
        );
        assert_eq!(
            runtime_banner(&browser),
            "Native providers are unavailable on this runtime — requires desktop or a companion process."
        );

        let desktop = caps(
            RuntimeProfileProjection::WindowsDesktop,
            ExtensionPlacementProjection::InProcess,
        );
        assert_eq!(
            runtime_banner(&desktop),
            "This runtime hosts extensions in-process."
        );

        let hosted_web = caps(
            RuntimeProfileProjection::HostedWeb,
            ExtensionPlacementProjection::NativeCompanion,
        );
        assert_eq!(
            runtime_banner(&hosted_web),
            "This runtime reaches extensions through a companion process."
        );
    }

    #[test]
    fn nominal_state_conversion_is_a_faithful_pass_through() {
        assert_eq!(
            ExtensionProviderState::from(ExtensionProviderNominalState::Available),
            ExtensionProviderState::Available
        );
        assert_eq!(
            ExtensionProviderState::from(ExtensionProviderNominalState::Loading),
            ExtensionProviderState::Loading
        );
        assert_eq!(
            ExtensionProviderState::from(ExtensionProviderNominalState::Quarantined),
            ExtensionProviderState::Quarantined
        );
        assert_eq!(
            ExtensionProviderState::from(ExtensionProviderNominalState::Rejected),
            ExtensionProviderState::Rejected
        );
    }
}
