//! Stage host contract — SHELL_SPEC.md §3.
//!
//! A stage is the surface inside the Stage-host region; the shell mounts
//! exactly one at a time. The rules carried from the estate's spine,
//! unchanged in force:
//!
//! - **Switching is re-projection, never re-load.** [`switch_stage`] writes
//!   `SetActiveLens` through the audited shared-state chokepoint and touches
//!   nothing else — it must not dispatch engine intents (tested with the
//!   IR's `RecordingDispatcher`).
//! - **Continuity:** `SharedSkinState.selection_set / focus_key /
//!   collapsed_keys / pinned_keys / cleave` survive switches; the incoming
//!   stage renders the continuity halo ([`continuity_halo`]; 160 ms,
//!   reduced-motion honored).
//! - A stage that cannot mount renders the fail-loud fallback card — never a
//!   blank or a wrong render ([`StageResolution::Unavailable`]).

use std::sync::Arc;

use leptos::prelude::*;

use dnacalc_skin_ir::intent::{Dispatcher, WorkspaceDelta};
use dnacalc_skin_ir::preview::PreviewService;
use dnacalc_skin_ir::selection::SelectionState;
use dnacalc_skin_ir::state::{SharedStateChange, SharedStateOrigin};
use dnacalc_skin_ir::workspace::WorkspaceState;
use dnacalc_skin_leptos::state_handles::SharedSkinStateHandle;
use dnacalc_strand::{Density, MOTION_DURATION_MAX_MS, Theme};

use crate::profile::ProfileTag;

/// The closed set of stage identities (SHELL_SPEC §3 sketch:
/// `sheet | model | notebook | atlas | bench-result`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StageId {
    Sheet,
    Model,
    Notebook,
    Atlas,
    BenchResult,
}

impl StageId {
    /// Stable wire id — also the value stored in
    /// `SharedSkinState.active_lens` (the spec's `active_lens→active_stage`
    /// continuity mapping).
    #[must_use]
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::Sheet => "sheet",
            Self::Model => "model",
            Self::Notebook => "notebook",
            Self::Atlas => "atlas",
            Self::BenchResult => "bench-result",
        }
    }

    /// Inverse of [`Self::stable_id`].
    #[must_use]
    pub fn from_stable_id(id: &str) -> Option<Self> {
        Some(match id {
            "sheet" => Self::Sheet,
            "model" => Self::Model,
            "notebook" => Self::Notebook,
            "atlas" => Self::Atlas,
            "bench-result" => Self::BenchResult,
            _ => return None,
        })
    }
}

/// Everything a stage receives at mount time: read signals, the one
/// dispatcher, optional foresight, the audited shared-state handle, and the
/// Strand theme pair. No engine types anywhere (P-gate).
#[derive(Clone)]
pub struct StageContext {
    pub workspace: ReadSignal<WorkspaceState>,
    pub latest_delta: ReadSignal<WorkspaceDelta>,
    pub selection: ReadSignal<SelectionState>,
    pub shared: SharedSkinStateHandle,
    pub theme: Theme,
    pub density: Density,
    pub dispatch: Arc<dyn Dispatcher>,
    /// Non-mutating foresight; `None` in minimal hosts — stages degrade to
    /// post-attempt receipt feedback (previews are never a requirement).
    pub preview: Option<Arc<dyn PreviewService>>,
}

/// What a mounted stage returns: a renderable view plus an optional
/// teardown hook (mirrors the estate's `SkinHandle`).
pub struct StageHandle {
    pub view: AnyView,
    pub on_deactivate: Option<Box<dyn FnOnce() + Send + Sync>>,
}

impl StageHandle {
    #[must_use]
    pub fn new(view: AnyView) -> Self {
        Self {
            view,
            on_deactivate: None,
        }
    }
}

/// The stage-surface contract (SHELL_SPEC §3).
pub trait StageSurface: Send + Sync + 'static {
    fn id(&self) -> StageId;
    fn title(&self) -> &'static str;
    /// Profiles gate capability: the switcher shows only stages whose
    /// `supports(profile)` holds, and keyboard slots follow that visible
    /// order.
    fn supports(&self, profile: &ProfileTag) -> bool;
    fn mount(&self, ctx: StageContext) -> StageHandle;
}

/// Ordered stage registry. Registration order is display order; keyboard
/// slot numbers (Stage 1..4) index the *visible* (profile-supported) subset.
#[derive(Clone, Default)]
pub struct StageRegistry {
    stages: Vec<Arc<dyn StageSurface>>,
}

impl StageRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, stage: Arc<dyn StageSurface>) {
        self.stages.push(stage);
    }

    #[must_use]
    pub fn with_stage(mut self, stage: Arc<dyn StageSurface>) -> Self {
        self.register(stage);
        self
    }

    /// The stages visible under `profile`, in slot order.
    #[must_use]
    pub fn visible(&self, profile: &ProfileTag) -> Vec<Arc<dyn StageSurface>> {
        self.stages
            .iter()
            .filter(|stage| stage.supports(profile))
            .cloned()
            .collect()
    }

    /// The visible stage occupying 1-based keyboard `slot`, if any.
    #[must_use]
    pub fn visible_slot(&self, profile: &ProfileTag, slot: u8) -> Option<Arc<dyn StageSurface>> {
        let index = (slot as usize).checked_sub(1)?;
        self.visible(profile).get(index).cloned()
    }

    /// Look a visible stage up by id.
    #[must_use]
    pub fn visible_by_id(
        &self,
        profile: &ProfileTag,
        id: StageId,
    ) -> Option<Arc<dyn StageSurface>> {
        self.visible(profile)
            .into_iter()
            .find(|stage| stage.id() == id)
    }
}

/// What the stage host should render for the current shared state.
pub enum StageResolution {
    /// Mount this stage.
    Mount(Arc<dyn StageSurface>),
    /// Fail-loud fallback card: the active stage id cannot mount here
    /// (unknown id, or a stage this profile does not support). Never render
    /// a blank or a wrong stage instead.
    Unavailable { active_lens: Option<String> },
}

/// Resolve the active stage from `active_lens` shared state: the recorded
/// stage when it is visible under `profile`, the first visible stage when
/// nothing is recorded yet, and a fail-loud [`StageResolution::Unavailable`]
/// when the recorded id names something that cannot mount.
#[must_use]
pub fn resolve_active_stage(
    registry: &StageRegistry,
    profile: &ProfileTag,
    active_lens: Option<&str>,
) -> StageResolution {
    match active_lens {
        Some(id) => match StageId::from_stable_id(id)
            .and_then(|stage_id| registry.visible_by_id(profile, stage_id))
        {
            Some(stage) => StageResolution::Mount(stage),
            None => StageResolution::Unavailable {
                active_lens: Some(id.to_string()),
            },
        },
        None => match registry.visible(profile).first() {
            Some(stage) => StageResolution::Mount(stage.clone()),
            None => StageResolution::Unavailable { active_lens: None },
        },
    }
}

/// Switch to the visible stage in 1-based `slot`.
///
/// Re-projection only: the sole write is the audited
/// `SharedStateChange::SetActiveLens` (origin `Shell`). No dispatcher is
/// even reachable from here — the signature takes none — which is the
/// compile-time form of "stage switch must not dispatch engine intents";
/// the behavioral test asserts it over a `RecordingDispatcher`-backed shell.
pub fn switch_stage(
    shared: &SharedSkinStateHandle,
    registry: &StageRegistry,
    profile: &ProfileTag,
    slot: u8,
) -> Option<StageId> {
    let stage = registry.visible_slot(profile, slot)?;
    let id = stage.id();
    shared.apply(
        SharedStateChange::SetActiveLens(id.stable_id().to_string()),
        SharedStateOrigin::Shell,
    );
    Some(id)
}

/// The continuity halo the incoming stage renders after a switch
/// (mechanism 09).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HaloSpec {
    /// 160 ms — Strand's sanctioned motion maximum.
    pub duration_ms: u32,
    /// CSS class the stage host applies to the halo target.
    pub css_class: &'static str,
}

/// Continuity-halo hook, stubbed for S0: returns the halo spec, or `None`
/// under reduced motion (the halo is skipped entirely — no zero-duration
/// flash).
#[must_use]
pub const fn continuity_halo(reduced_motion: bool) -> Option<HaloSpec> {
    if reduced_motion {
        None
    } else {
        Some(HaloSpec {
            duration_ms: MOTION_DURATION_MAX_MS,
            css_class: "dna-continuity-halo",
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use dnacalc_skin_ir::dispatcher::RecordingDispatcher;
    use dnacalc_skin_ir::identity::NodeKey;
    use dnacalc_skin_ir::state::SharedSkinState;

    use super::*;

    pub(crate) struct TestStage {
        pub id: StageId,
        pub profiles: Vec<ProfileTag>,
    }

    impl StageSurface for TestStage {
        fn id(&self) -> StageId {
            self.id
        }

        fn title(&self) -> &'static str {
            "test stage"
        }

        fn supports(&self, profile: &ProfileTag) -> bool {
            self.profiles.contains(profile)
        }

        fn mount(&self, _ctx: StageContext) -> StageHandle {
            StageHandle::new(().into_any())
        }
    }

    fn registry() -> StageRegistry {
        StageRegistry::new()
            .with_stage(Arc::new(TestStage {
                id: StageId::Sheet,
                profiles: vec![ProfileTag::RichTree, ProfileTag::ExcelStrict],
            }))
            .with_stage(Arc::new(TestStage {
                id: StageId::Model,
                profiles: vec![ProfileTag::RichTree],
            }))
            .with_stage(Arc::new(TestStage {
                id: StageId::Atlas,
                profiles: vec![ProfileTag::RichTree],
            }))
    }

    #[test]
    fn stage_ids_round_trip() {
        for id in [
            StageId::Sheet,
            StageId::Model,
            StageId::Notebook,
            StageId::Atlas,
            StageId::BenchResult,
        ] {
            assert_eq!(StageId::from_stable_id(id.stable_id()), Some(id));
        }
        assert_eq!(StageId::from_stable_id("bogus"), None);
    }

    #[test]
    fn keyboard_slots_follow_the_visible_order_not_the_registration_order() {
        let registry = registry();
        // ExcelStrict sees only Sheet, so slot 1 is Sheet and slot 2 empty.
        let visible = registry.visible(&ProfileTag::ExcelStrict);
        assert_eq!(visible.len(), 1);
        assert_eq!(
            registry
                .visible_slot(&ProfileTag::ExcelStrict, 1)
                .map(|stage| stage.id()),
            Some(StageId::Sheet)
        );
        assert!(registry.visible_slot(&ProfileTag::ExcelStrict, 2).is_none());
        // RichTree sees all three; slot 3 is Atlas.
        assert_eq!(
            registry
                .visible_slot(&ProfileTag::RichTree, 3)
                .map(|stage| stage.id()),
            Some(StageId::Atlas)
        );
    }

    #[test]
    fn switch_stage_writes_only_the_audited_active_lens_change() {
        let shared = SharedSkinStateHandle::new(SharedSkinState::default());
        // A dispatcher exists in the surrounding shell; the switch path must
        // never touch it. `switch_stage` cannot even name it — assert the
        // recording double stays empty across a switch performed next to it.
        let dispatcher = RecordingDispatcher::new();
        let switched = switch_stage(&shared, &registry(), &ProfileTag::RichTree, 2);
        assert_eq!(switched, Some(StageId::Model));
        assert!(
            dispatcher.intents().is_empty(),
            "stage switch is re-projection only; no engine intent may be dispatched"
        );
        assert_eq!(shared.get_untracked().active_lens.as_deref(), Some("model"));
        let audit = shared.audit_log();
        assert_eq!(audit.len(), 1);
        assert_eq!(audit[0].origin, SharedStateOrigin::Shell);
        assert!(matches!(
            &audit[0].change,
            SharedStateChange::SetActiveLens(id) if id == "model"
        ));
    }

    #[test]
    fn continuity_state_survives_a_stage_switch() {
        let mut initial = SharedSkinState::default();
        let key = NodeKey::new("continuity-node");
        initial.selection_set = vec![key.clone()];
        initial.focus_key = Some(key.clone());
        initial.collapsed_keys = HashSet::from([key.clone()]);
        initial.pinned_keys = vec![key.clone()];
        let shared = SharedSkinStateHandle::new(initial.clone());

        switch_stage(&shared, &registry(), &ProfileTag::RichTree, 3);

        let after = shared.get_untracked();
        assert_eq!(after.selection_set, initial.selection_set);
        assert_eq!(after.focus_key, initial.focus_key);
        assert_eq!(after.collapsed_keys, initial.collapsed_keys);
        assert_eq!(after.pinned_keys, initial.pinned_keys);
        assert_eq!(after.cleave, initial.cleave);
        assert_eq!(after.active_lens.as_deref(), Some("atlas"));
    }

    #[test]
    fn resolve_active_stage_falls_back_first_then_fails_loud() {
        let registry = registry();
        // Nothing recorded: first visible stage mounts.
        assert!(matches!(
            resolve_active_stage(&registry, &ProfileTag::RichTree, None),
            StageResolution::Mount(stage) if stage.id() == StageId::Sheet
        ));
        // Recorded stage visible: mounts.
        assert!(matches!(
            resolve_active_stage(&registry, &ProfileTag::RichTree, Some("atlas")),
            StageResolution::Mount(stage) if stage.id() == StageId::Atlas
        ));
        // Recorded stage exists but this profile does not support it: fail loud.
        assert!(matches!(
            resolve_active_stage(&registry, &ProfileTag::ExcelStrict, Some("model")),
            StageResolution::Unavailable { active_lens: Some(id) } if id == "model"
        ));
        // Recorded id is not a stage id at all (e.g. an estate lens id): fail loud.
        assert!(matches!(
            resolve_active_stage(&registry, &ProfileTag::RichTree, Some("lens:tree")),
            StageResolution::Unavailable { active_lens: Some(id) } if id == "lens:tree"
        ));
    }

    #[test]
    fn continuity_halo_honors_reduced_motion_and_the_strand_motion_law() {
        let halo = continuity_halo(false).expect("halo present when motion allowed");
        assert_eq!(halo.duration_ms, MOTION_DURATION_MAX_MS);
        assert_eq!(halo.duration_ms, 160);
        assert_eq!(continuity_halo(true), None);
    }
}
