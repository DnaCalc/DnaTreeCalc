//! Leptos-, Tauri-, and CLI-free OneCalc session boundary.

use std::ops::{Deref, DerefMut};

pub use dnacalc_skin_ir::{
    RuntimeProfileProjection, SkinIntent, SkinIntentDiagnostic, SkinIntentEnvelope,
    SkinIntentReceipt, SkinSnapshot,
};

pub trait OneCalcSessionHost {
    fn snapshot(&self) -> SkinSnapshot;
    fn dispatch(&mut self, intent: SkinIntent) -> DispatchOutcome;
    fn set_runtime_profile(&mut self, _profile: RuntimeProfileProjection) {}
}

/// Platform-neutral owner of the live OneCalc state.
///
/// Shells may borrow the state for rendering, but state lifetime and persistence
/// orchestration live here. Platform adapters only implement [`StateStore`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OneCalcCore<S> {
    state: S,
}

impl<S> OneCalcCore<S> {
    pub const fn new(state: S) -> Self {
        Self { state }
    }

    pub fn into_state(self) -> S {
        self.state
    }

    pub fn hydrate<E>(&mut self, store: &mut impl StateStore<S, Error = E>) -> Result<bool, E> {
        let Some(state) = store.load()? else {
            return Ok(false);
        };
        self.state = state;
        Ok(true)
    }

    pub fn persist<E>(&self, store: &mut impl StateStore<S, Error = E>) -> Result<(), E> {
        store.save(&self.state)
    }

    pub fn hydrate_with(&mut self, persistence: &mut impl StatePersistence<S>) {
        persistence.hydrate(&mut self.state);
    }

    pub fn persist_with(&self, persistence: &mut impl StatePersistence<S>) {
        persistence.persist(&self.state);
    }
}

impl<S> Deref for OneCalcCore<S> {
    type Target = S;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl<S> DerefMut for OneCalcCore<S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.state
    }
}

/// Persistence port implemented by browser, desktop, CLI, or test adapters.
pub trait StateStore<S> {
    type Error;

    fn load(&mut self) -> Result<Option<S>, Self::Error>;
    fn save(&mut self, state: &S) -> Result<(), Self::Error>;
}

/// In-place persistence port for established wire formats that apply onto a
/// bootstrapped state (for example versioned workspace envelopes).
pub trait StatePersistence<S> {
    fn hydrate(&mut self, state: &mut S);
    fn persist(&mut self, state: &S);
}

impl<S: OneCalcSessionHost> OneCalcSessionHost for OneCalcCore<S> {
    fn snapshot(&self) -> SkinSnapshot {
        self.state.snapshot()
    }

    fn dispatch(&mut self, intent: SkinIntent) -> DispatchOutcome {
        self.state.dispatch(intent)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchOutcome {
    Applied,
    NoChange,
    Rejected(SkinIntentDiagnostic),
}

pub struct OneCalcSession<H> {
    host: H,
    revision: u64,
}

pub struct ProfiledSession<H> {
    pub target: RuntimeTarget,
    session: OneCalcSession<H>,
}

impl<H: OneCalcSessionHost> ProfiledSession<H> {
    pub fn new(target: RuntimeTarget, mut host: H) -> Self {
        host.set_runtime_profile(target.profile());
        Self {
            target,
            session: OneCalcSession::new(host),
        }
    }

    pub fn handle_json(&mut self, json: &str) -> Result<String, serde_json::Error> {
        self.session.handle_json(json)
    }

    pub fn snapshot(&self) -> SkinSnapshot {
        self.session.snapshot()
    }
}

impl<H: OneCalcSessionHost> OneCalcSession<H> {
    pub fn new(host: H) -> Self {
        Self { host, revision: 0 }
    }

    pub fn snapshot(&self) -> SkinSnapshot {
        self.host.snapshot()
    }

    pub fn host(&self) -> &H {
        &self.host
    }

    pub fn handle(&mut self, envelope: SkinIntentEnvelope) -> SkinIntentReceipt {
        if let Err(error) = envelope.validate() {
            return SkinIntentReceipt::Rejected {
                intent_id: envelope.intent_id,
                diagnostic: SkinIntentDiagnostic {
                    code: "invalid_intent_envelope".into(),
                    message: format!("{error:?}"),
                    recoverable: false,
                },
            };
        }
        let intent_id = envelope.intent_id;
        match self.host.dispatch(envelope.intent) {
            DispatchOutcome::Applied => {
                self.revision = self.revision.saturating_add(1);
                SkinIntentReceipt::Applied {
                    intent_id,
                    snapshot_revision: self.revision,
                }
            }
            DispatchOutcome::NoChange => SkinIntentReceipt::Rejected {
                intent_id,
                diagnostic: SkinIntentDiagnostic {
                    code: "no_change".into(),
                    message: "intent produced no state change".into(),
                    recoverable: true,
                },
            },
            DispatchOutcome::Rejected(diagnostic) => SkinIntentReceipt::Rejected {
                intent_id,
                diagnostic,
            },
        }
    }

    pub fn handle_json(&mut self, json: &str) -> Result<String, serde_json::Error> {
        let envelope = serde_json::from_str(json)?;
        serde_json::to_string(&self.handle(envelope))
    }
}

#[must_use]
pub const fn runtime_profile() -> RuntimeProfileProjection {
    if cfg!(target_arch = "wasm32") {
        RuntimeProfileProjection::BrowserWasm
    } else if cfg!(target_os = "windows") {
        RuntimeProfileProjection::WindowsDesktop
    } else {
        RuntimeProfileProjection::NativeUnix
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeTarget {
    BrowserWasm,
    HostedWeb,
    WindowsDesktop,
    WindowsHeadless,
    NativeUnix,
    NullTest,
}

impl RuntimeTarget {
    #[must_use]
    pub const fn profile(self) -> RuntimeProfileProjection {
        match self {
            Self::BrowserWasm => RuntimeProfileProjection::BrowserWasm,
            Self::HostedWeb => RuntimeProfileProjection::HostedWeb,
            Self::WindowsDesktop => RuntimeProfileProjection::WindowsDesktop,
            Self::WindowsHeadless => RuntimeProfileProjection::WindowsHeadless,
            Self::NativeUnix => RuntimeProfileProjection::NativeUnix,
            Self::NullTest => RuntimeProfileProjection::NullTest,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_shared_runtime_profile_has_a_onecalc_target() {
        assert_eq!(
            RuntimeTarget::BrowserWasm.profile(),
            RuntimeProfileProjection::BrowserWasm
        );
        assert_eq!(
            RuntimeTarget::HostedWeb.profile(),
            RuntimeProfileProjection::HostedWeb
        );
        assert_eq!(
            RuntimeTarget::WindowsDesktop.profile(),
            RuntimeProfileProjection::WindowsDesktop
        );
        assert_eq!(
            RuntimeTarget::WindowsHeadless.profile(),
            RuntimeProfileProjection::WindowsHeadless
        );
        assert_eq!(
            RuntimeTarget::NativeUnix.profile(),
            RuntimeProfileProjection::NativeUnix
        );
        assert_eq!(
            RuntimeTarget::NullTest.profile(),
            RuntimeProfileProjection::NullTest
        );
    }

    #[derive(Default)]
    struct MemoryStore(Option<String>);

    impl StateStore<String> for MemoryStore {
        type Error = core::convert::Infallible;

        fn load(&mut self) -> Result<Option<String>, Self::Error> {
            Ok(self.0.clone())
        }

        fn save(&mut self, state: &String) -> Result<(), Self::Error> {
            self.0 = Some(state.clone());
            Ok(())
        }
    }

    #[test]
    fn core_owns_state_and_persistence_lifecycle() {
        let mut core = OneCalcCore::new("=1+1".to_owned());
        let mut store = MemoryStore::default();
        core.persist(&mut store).unwrap();
        *core = "changed".to_owned();
        assert!(core.hydrate(&mut store).unwrap());
        assert_eq!(&*core, "=1+1");
    }
}
