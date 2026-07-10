//! Host-neutral extension catalog and invalidation substrate.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;

use oxfunc_core::registry::{FunctionRegistry, UdfRegistrationRequest, UdfRegistrationResult};
use oxfunc_core::value::CalcValue;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeProfile {
    BrowserWasm,
    HostedWeb,
    WindowsDesktop,
    WindowsHeadless,
    NativeUnix,
    NullTest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionCapabilities {
    pub native_providers: bool,
    pub sandbox_providers: bool,
    pub native_companion: bool,
    pub volatile_ticks: bool,
    pub rtd_topics: bool,
    pub com_rtd: bool,
}

impl RuntimeProfile {
    #[must_use]
    pub fn capabilities(self) -> ExtensionCapabilities {
        match self {
            Self::BrowserWasm | Self::NullTest => ExtensionCapabilities {
                native_providers: false,
                sandbox_providers: matches!(self, Self::BrowserWasm),
                native_companion: false,
                volatile_ticks: false,
                rtd_topics: false,
                com_rtd: false,
            },
            Self::HostedWeb => ExtensionCapabilities {
                native_providers: false,
                sandbox_providers: false,
                native_companion: true,
                volatile_ticks: true,
                rtd_topics: true,
                com_rtd: false,
            },
            Self::WindowsDesktop | Self::WindowsHeadless | Self::NativeUnix => {
                ExtensionCapabilities {
                    native_providers: true,
                    sandbox_providers: false,
                    native_companion: false,
                    volatile_ticks: true,
                    rtd_topics: true,
                    com_rtd: matches!(self, Self::WindowsDesktop | Self::WindowsHeadless),
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionDiagnostic {
    pub stage: ExtensionDiagnosticStage,
    pub code: String,
    pub native_code: Option<i64>,
    pub message: String,
    pub provider_id: Option<String>,
    pub function_id: Option<String>,
    pub cause: Option<String>,
    pub recoverable: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtensionDiagnosticStage {
    Discovery,
    Validation,
    Registration,
    Invocation,
    Update,
    Teardown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LibraryContextComposition {
    Deferred(ExtensionDiagnostic),
}

/// Typed deferral until OxFml exposes a provider-backed library-context
/// transaction that can commit atomically with `FunctionRegistry`.
#[must_use]
pub fn compose_oxfml_library_context() -> LibraryContextComposition {
    LibraryContextComposition::Deferred(ExtensionDiagnostic {
        stage: ExtensionDiagnosticStage::Registration,
        code: "oxfml_library_context_provider_surface_required".into(),
        native_code: None,
        message: "OxFml provider-backed library-context composition is not yet public".into(),
        provider_id: None,
        function_id: None,
        cause: None,
        recoverable: true,
    })
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ExtensionHostError {
    #[error("profile does not permit native providers")]
    ProfileRejected,
    #[error("provider already registered: {0}")]
    DuplicateProvider(String),
    #[error("unknown function: {0}")]
    UnknownFunction(String),
    #[error("provider invocation failed: {0}")]
    Invocation(String),
    #[error("invalid provider contract: {0}")]
    InvalidProvider(String),
    #[error("tick sequence exhausted")]
    TickExhausted,
}

pub trait ExtensionProvider: Send + Sync {
    fn provider_id(&self) -> &str;
    fn functions(&self) -> Vec<String>;
    fn placement(&self) -> ProviderPlacement {
        ProviderPlacement::NativeInProcess
    }
    fn invoke(&self, function: &str, args: &[CalcValue]) -> Result<CalcValue, ExtensionHostError>;
    fn teardown(&self) -> Result<(), ExtensionHostError> {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderPlacement {
    Sandbox,
    NativeInProcess,
    NativeCompanion,
}

pub fn register_native_functions(
    registry: &mut FunctionRegistry,
    requests: Vec<UdfRegistrationRequest>,
) -> Result<(), Vec<ExtensionDiagnostic>> {
    let before = registry.clone();
    let mut diagnostics = Vec::new();
    for request in requests {
        if let UdfRegistrationResult::Rejected(rejection) = registry.register_udf_request(request) {
            diagnostics.push(ExtensionDiagnostic {
                stage: ExtensionDiagnosticStage::Registration,
                code: format!("{:?}", rejection.code),
                native_code: None,
                message: rejection.detail,
                provider_id: rejection.source_registration_id,
                function_id: rejection.function_id,
                cause: None,
                recoverable: true,
            });
        }
    }
    if diagnostics.is_empty() {
        Ok(())
    } else {
        *registry = before;
        Err(diagnostics)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostInvalidationEvent {
    FunctionCatalogChanged { generation: u64 },
    VolatileTick { tick: u64 },
    RtdTopicChanged { key: RtdTopicKey, version: u64 },
}

pub trait HostInvalidationSink {
    fn invalidate(&mut self, event: HostInvalidationEvent);
}

#[derive(Default)]
pub struct RecordingInvalidationSink {
    pub events: Vec<HostInvalidationEvent>,
}
impl HostInvalidationSink for RecordingInvalidationSink {
    fn invalidate(&mut self, event: HostInvalidationEvent) {
        self.events.push(event);
    }
}

pub struct ExtensionCatalog {
    profile: RuntimeProfile,
    generation: u64,
    providers: BTreeMap<String, Arc<dyn ExtensionProvider>>,
    functions: BTreeMap<String, String>,
    pending: VecDeque<HostInvalidationEvent>,
}

impl ExtensionCatalog {
    #[must_use]
    pub fn new(profile: RuntimeProfile) -> Self {
        Self {
            profile,
            generation: 0,
            providers: BTreeMap::new(),
            functions: BTreeMap::new(),
            pending: VecDeque::new(),
        }
    }
    pub fn register(
        &mut self,
        provider: Arc<dyn ExtensionProvider>,
    ) -> Result<(), ExtensionHostError> {
        let capabilities = self.profile.capabilities();
        let allowed = match provider.placement() {
            ProviderPlacement::Sandbox => capabilities.sandbox_providers,
            ProviderPlacement::NativeInProcess => capabilities.native_providers,
            ProviderPlacement::NativeCompanion => capabilities.native_companion,
        };
        if !allowed {
            return Err(ExtensionHostError::ProfileRejected);
        }
        let id = provider.provider_id().to_string();
        if id.trim().is_empty() {
            return Err(ExtensionHostError::InvalidProvider(
                "provider id is empty".into(),
            ));
        }
        if self.providers.contains_key(&id) {
            return Err(ExtensionHostError::DuplicateProvider(id));
        }
        let functions = provider.functions();
        let unique: BTreeSet<_> = functions
            .iter()
            .map(|name| name.to_ascii_uppercase())
            .collect();
        if unique.len() != functions.len() || functions.iter().any(|name| name.trim().is_empty()) {
            return Err(ExtensionHostError::InvalidProvider(
                "function names must be non-empty and unique".into(),
            ));
        }
        for function in &functions {
            if self.functions.contains_key(&function.to_ascii_uppercase()) {
                return Err(ExtensionHostError::DuplicateProvider(function.clone()));
            }
        }
        for function in functions {
            self.functions
                .insert(function.to_ascii_uppercase(), id.clone());
        }
        self.providers.insert(id, provider);
        self.generation += 1;
        self.pending
            .push_back(HostInvalidationEvent::FunctionCatalogChanged {
                generation: self.generation,
            });
        Ok(())
    }
    pub fn invoke(
        &self,
        function: &str,
        args: &[CalcValue],
    ) -> Result<CalcValue, ExtensionHostError> {
        let canonical_function = function.to_ascii_uppercase();
        let provider = self
            .functions
            .get(&canonical_function)
            .and_then(|id| self.providers.get(id))
            .ok_or_else(|| ExtensionHostError::UnknownFunction(function.into()))?;
        provider.invoke(&canonical_function, args)
    }
    pub fn teardown(&mut self) -> Vec<ExtensionDiagnostic> {
        let mut diagnostics = Vec::new();
        for (id, provider) in &self.providers {
            if let Err(error) = provider.teardown() {
                diagnostics.push(ExtensionDiagnostic {
                    stage: ExtensionDiagnosticStage::Teardown,
                    code: "teardown_failed".into(),
                    native_code: None,
                    message: error.to_string(),
                    provider_id: Some(id.clone()),
                    function_id: None,
                    cause: Some(format!("{error:?}")),
                    recoverable: false,
                });
            }
        }
        if !self.providers.is_empty() {
            self.providers.clear();
            self.functions.clear();
            self.generation += 1;
            self.pending
                .push_back(HostInvalidationEvent::FunctionCatalogChanged {
                    generation: self.generation,
                });
        }
        diagnostics
    }
    pub fn unregister(&mut self, provider_id: &str) -> Result<(), ExtensionHostError> {
        let provider = self.providers.get(provider_id).ok_or_else(|| {
            ExtensionHostError::InvalidProvider(format!("unknown provider {provider_id}"))
        })?;
        provider.teardown()?;
        self.providers.remove(provider_id);
        self.functions.retain(|_, owner| owner != provider_id);
        self.generation += 1;
        self.pending
            .push_back(HostInvalidationEvent::FunctionCatalogChanged {
                generation: self.generation,
            });
        Ok(())
    }
    pub fn notify(&mut self, event: HostInvalidationEvent) {
        self.pending.push_back(event);
    }
    pub fn drain_invalidations(&mut self, sink: &mut dyn HostInvalidationSink) {
        let mut latest_catalog = None;
        let mut latest_tick = None;
        let mut topics: BTreeMap<RtdTopicKey, u64> = BTreeMap::new();
        while let Some(event) = self.pending.pop_front() {
            match event {
                HostInvalidationEvent::FunctionCatalogChanged { generation } => {
                    latest_catalog =
                        Some(latest_catalog.map_or(generation, |old: u64| old.max(generation)))
                }
                HostInvalidationEvent::VolatileTick { tick } => {
                    latest_tick = Some(latest_tick.map_or(tick, |old: u64| old.max(tick)))
                }
                HostInvalidationEvent::RtdTopicChanged { key, version } => {
                    topics
                        .entry(key)
                        .and_modify(|old| *old = (*old).max(version))
                        .or_insert(version);
                }
            }
        }
        if let Some(generation) = latest_catalog {
            sink.invalidate(HostInvalidationEvent::FunctionCatalogChanged { generation });
        }
        if let Some(tick) = latest_tick {
            sink.invalidate(HostInvalidationEvent::VolatileTick { tick });
        }
        for (key, version) in topics {
            sink.invalidate(HostInvalidationEvent::RtdTopicChanged { key, version });
        }
    }
}

#[derive(Debug, Default)]
pub struct DeterministicTickScheduler {
    next: u64,
}
impl DeterministicTickScheduler {
    pub fn tick(&mut self) -> Result<HostInvalidationEvent, ExtensionHostError> {
        self.next = self
            .next
            .checked_add(1)
            .ok_or(ExtensionHostError::TickExhausted)?;
        Ok(HostInvalidationEvent::VolatileTick { tick: self.next })
    }
}

#[derive(Debug, Default)]
pub struct RtdTopicState {
    subscriptions: BTreeSet<RtdTopicKey>,
    values: BTreeMap<RtdTopicKey, (u64, CalcValue)>,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RtdTopicKey {
    pub provider_id: String,
    pub session_id: String,
    pub topic: String,
}
impl RtdTopicState {
    pub fn subscribe(&mut self, topic: impl Into<String>) {
        self.subscribe_key(RtdTopicKey {
            provider_id: "default".into(),
            session_id: "default".into(),
            topic: topic.into(),
        });
    }
    pub fn subscribe_key(&mut self, key: RtdTopicKey) {
        self.subscriptions.insert(key);
    }
    pub fn unsubscribe(&mut self, topic: &str) {
        let key = RtdTopicKey {
            provider_id: "default".into(),
            session_id: "default".into(),
            topic: topic.into(),
        };
        self.unsubscribe_key(&key);
    }
    pub fn unsubscribe_key(&mut self, key: &RtdTopicKey) {
        self.subscriptions.remove(key);
    }
    pub fn update(&mut self, topic: &str, value: CalcValue) -> Option<HostInvalidationEvent> {
        let key = RtdTopicKey {
            provider_id: "default".into(),
            session_id: "default".into(),
            topic: topic.into(),
        };
        self.update_key(&key, value)
    }
    pub fn update_key(
        &mut self,
        key: &RtdTopicKey,
        value: CalcValue,
    ) -> Option<HostInvalidationEvent> {
        if !self.subscriptions.contains(key) {
            return None;
        }
        if self.values.get(key).is_some_and(|(_, old)| old == &value) {
            return None;
        }
        let version = self.values.get(key).map_or(1, |(v, _)| v.saturating_add(1));
        self.values.insert(key.clone(), (version, value));
        Some(HostInvalidationEvent::RtdTopicChanged {
            key: key.clone(),
            version,
        })
    }
    pub fn value(&self, topic: &str) -> Option<&CalcValue> {
        let key = RtdTopicKey {
            provider_id: "default".into(),
            session_id: "default".into(),
            topic: topic.into(),
        };
        self.value_key(&key)
    }
    pub fn value_key(&self, key: &RtdTopicKey) -> Option<&CalcValue> {
        self.values.get(key).map(|(_, value)| value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct AddOne;
    impl ExtensionProvider for AddOne {
        fn provider_id(&self) -> &str {
            "test"
        }
        fn functions(&self) -> Vec<String> {
            vec!["ADDONE".into()]
        }
        fn invoke(
            &self,
            function: &str,
            args: &[CalcValue],
        ) -> Result<CalcValue, ExtensionHostError> {
            if function != "ADDONE" {
                return Err(ExtensionHostError::Invocation(format!(
                    "unexpected id {function}"
                )));
            }
            Ok(CalcValue::number(args[0].as_number().unwrap() + 1.0))
        }
    }
    struct FailingTeardown;
    impl ExtensionProvider for FailingTeardown {
        fn provider_id(&self) -> &str {
            "fail"
        }
        fn functions(&self) -> Vec<String> {
            vec!["FAILFUNC".into()]
        }
        fn invoke(&self, _: &str, _: &[CalcValue]) -> Result<CalcValue, ExtensionHostError> {
            Ok(CalcValue::empty())
        }
        fn teardown(&self) -> Result<(), ExtensionHostError> {
            Err(ExtensionHostError::Invocation("teardown".into()))
        }
    }
    fn topic(name: &str) -> RtdTopicKey {
        RtdTopicKey {
            provider_id: "provider".into(),
            session_id: "session".into(),
            topic: name.into(),
        }
    }
    #[test]
    fn lifecycle_invocation_and_profile_gate() {
        assert_eq!(
            ExtensionCatalog::new(RuntimeProfile::BrowserWasm).register(Arc::new(AddOne)),
            Err(ExtensionHostError::ProfileRejected)
        );
        let mut catalog = ExtensionCatalog::new(RuntimeProfile::WindowsDesktop);
        catalog.register(Arc::new(AddOne)).unwrap();
        assert_eq!(
            catalog.invoke("ADDONE", &[CalcValue::number(2.0)]).unwrap(),
            CalcValue::number(3.0)
        );
        assert!(catalog.teardown().is_empty());
    }
    #[test]
    fn canonical_names_and_failed_unregister_preserve_catalog() {
        let mut catalog = ExtensionCatalog::new(RuntimeProfile::WindowsDesktop);
        catalog.register(Arc::new(AddOne)).unwrap();
        assert_eq!(
            catalog.invoke("addone", &[CalcValue::number(1.0)]).unwrap(),
            CalcValue::number(2.0)
        );
        catalog.register(Arc::new(FailingTeardown)).unwrap();
        assert!(catalog.unregister("fail").is_err());
        assert!(catalog.invoke("failfunc", &[]).is_ok());
    }
    #[test]
    fn invalidations_ticks_and_topics_are_deterministic() {
        let mut ticks = DeterministicTickScheduler::default();
        assert_eq!(
            ticks.tick().unwrap(),
            HostInvalidationEvent::VolatileTick { tick: 1 }
        );
        let mut topics = RtdTopicState::default();
        let price = topic("price");
        topics.subscribe_key(price.clone());
        assert_eq!(
            topics.update_key(&price, CalcValue::number(7.0)),
            Some(HostInvalidationEvent::RtdTopicChanged {
                key: price.clone(),
                version: 1
            })
        );
        assert_eq!(topics.value_key(&price), Some(&CalcValue::number(7.0)));
        assert_eq!(topics.update_key(&price, CalcValue::number(7.0)), None);
        let other = RtdTopicKey {
            provider_id: "other".into(),
            ..price.clone()
        };
        topics.subscribe_key(other.clone());
        assert!(
            matches!(topics.update_key(&other, CalcValue::number(8.0)), Some(HostInvalidationEvent::RtdTopicChanged { key, .. }) if key == other)
        );
    }
    #[test]
    fn coalesces_invalidation_families_in_stable_order() {
        let mut catalog = ExtensionCatalog::new(RuntimeProfile::WindowsDesktop);
        catalog.notify(HostInvalidationEvent::VolatileTick { tick: 1 });
        catalog.notify(HostInvalidationEvent::RtdTopicChanged {
            key: topic("b"),
            version: 1,
        });
        catalog.notify(HostInvalidationEvent::RtdTopicChanged {
            key: topic("a"),
            version: 2,
        });
        catalog.notify(HostInvalidationEvent::VolatileTick { tick: 3 });
        let mut sink = RecordingInvalidationSink::default();
        catalog.drain_invalidations(&mut sink);
        assert_eq!(
            sink.events,
            vec![
                HostInvalidationEvent::VolatileTick { tick: 3 },
                HostInvalidationEvent::RtdTopicChanged {
                    key: topic("a"),
                    version: 2
                },
                HostInvalidationEvent::RtdTopicChanged {
                    key: topic("b"),
                    version: 1
                },
            ]
        );
    }
}
