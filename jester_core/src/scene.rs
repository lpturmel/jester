use std::{
    any::{Any, TypeId},
    hash::{DefaultHasher, Hash, Hasher},
    ops::Deref,
    path::PathBuf,
    sync::atomic::{AtomicU32, Ordering},
};

use crate::{InputState, Sprite, TextureId};
use hashbrown::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SceneKey(usize);

impl Deref for SceneKey {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl SceneKey {
    pub fn new(value: usize) -> Self {
        Self(value)
    }
    pub fn of<S: Scene + 'static>() -> Self {
        let mut hasher = DefaultHasher::new();
        TypeId::of::<S>().hash(&mut hasher);
        SceneKey(hasher.finish() as usize)
    }
}

pub trait Scene: Send {
    fn start(&mut self, _ctx: &mut Ctx<'_>) {}
    fn update(&mut self, _ctx: &mut Ctx<'_>) {}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EntityId(u32);

pub struct Ctx<'a> {
    pub dt: f32,
    pub resources: &'a mut Resources,
    pub commands: &'a mut Commands,
    pub pool: &'a mut EntityPool,
    pub input: &'a InputState,
}

impl<'a> Ctx<'a> {
    pub fn spawn_sprite(&mut self, s: Sprite) -> EntityId {
        self.pool.spawn(s)
    }
    pub fn load_asset(&mut self, p: impl Into<PathBuf>) -> TextureId {
        let id = TextureId(self.pool.next_id.fetch_add(1, Ordering::Relaxed));
        self.commands.assets_to_load.push((id, p.into()));
        id
    }
    pub fn goto_scene<S>(&mut self)
    where
        S: Scene + 'static,
    {
        self.commands.scene_switch = Some(TypeId::of::<S>());
    }
}

#[derive(Default)]
pub struct EntityPool {
    next_id: AtomicU32,
    pub entities: HashMap<EntityId, Sprite>,
}

impl EntityPool {
    pub fn spawn(&mut self, s: Sprite) -> EntityId {
        let id = EntityId(self.next_id.fetch_add(1, Ordering::Relaxed));
        self.entities.insert(id, s);
        id
    }
    pub fn sprite_mut(&mut self, id: EntityId) -> Option<&mut Sprite> {
        self.entities.get_mut(&id)
    }
}

#[derive(Default)]
pub struct Commands {
    pub sprites_to_spawn: Vec<Sprite>,
    pub assets_to_load: Vec<(TextureId, PathBuf)>,
    pub despawn: Vec<EntityId>,
    pub scene_switch: Option<TypeId>,
}

#[derive(Default)]
pub struct Resources {
    // any Send + Sync object, keyed by its concrete TypeId
    inner: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl Resources {
    /// Insert or replace a resource.
    pub fn insert<R: Any + Send + Sync>(&mut self, res: R) {
        self.inner.insert(TypeId::of::<R>(), Box::new(res));
    }

    /// Immutable access.
    pub fn get<R: Any + Send + Sync>(&self) -> Option<&R> {
        self.inner
            .get(&TypeId::of::<R>())
            .and_then(|b| b.downcast_ref::<R>())
    }

    /// Mutable access – rarely needed in `Ctx` but handy for the **apply** phase.
    pub fn get_mut<R: Any + Send + Sync>(&mut self) -> Option<&mut R> {
        self.inner
            .get_mut(&TypeId::of::<R>())
            .and_then(|b| b.downcast_mut::<R>())
    }

    /// Remove (returns previous value).
    pub fn take<R: Any + Send + Sync>(&mut self) -> Option<R> {
        self.inner
            .remove(&TypeId::of::<R>())
            .and_then(|b| b.downcast::<R>().ok())
            .map(|b| *b)
    }
}
