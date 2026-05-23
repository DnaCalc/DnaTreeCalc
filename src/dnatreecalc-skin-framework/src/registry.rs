use std::collections::BTreeMap;

use crate::identity::SkinId;
use crate::skin::{RegisteredSkin, WorkspaceSkin};

/// The host-wide registry of installed skins.
///
/// Preserves insertion order for stable enumeration in the shell's
/// skin-switcher chrome. Lookups by id are constant-time over the
/// underlying map. The registry is held by the host; the shell reads
/// it through a shared reference.
#[derive(Default)]
pub struct SkinRegistry {
    skins: BTreeMap<SkinId, RegisteredSkin>,
    order: Vec<SkinId>,
}

impl SkinRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<K: WorkspaceSkin>(&mut self, skin: K) -> SkinId {
        let registered = RegisteredSkin::from_skin(skin);
        self.register_registered(registered)
    }

    pub fn register_registered(&mut self, skin: RegisteredSkin) -> SkinId {
        let id = skin.id();
        if self.skins.insert(id, skin).is_none() {
            self.order.push(id);
        }
        id
    }

    #[must_use]
    pub fn get(&self, id: SkinId) -> Option<&RegisteredSkin> {
        self.skins.get(&id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &RegisteredSkin> {
        self.order.iter().filter_map(|id| self.skins.get(id))
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.skins.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.skins.is_empty()
    }

    #[must_use]
    pub fn ids(&self) -> Vec<SkinId> {
        self.order.clone()
    }
}
