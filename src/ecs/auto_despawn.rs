//local shortcuts

//third-party shortcuts
use bevy::prelude::*;
use crossbeam::channel::{Receiver, Sender};

//standard shortcuts
use std::sync::Arc;

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

struct AutoDespawnSignalInner
{
    entity: Entity,
    sender: Sender<Entity>,
}

impl Drop for AutoDespawnSignalInner
{
    fn drop(&mut self)
    {
        let _ = self.sender.send(self.entity);
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// Drains [`AutoDespawner`] and recursively despawns entities that were auto-despawned.
pub fn garbage_collect_entities(world: &mut World)
{
    while let Some(entity) = world.resource::<AutoDespawner>().try_recv()
    {
        world.get_entity_mut(entity).ok().map(|e| e.despawn());
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Creates [`AutoDespawnSignal`]s.
#[derive(Resource, Clone)]
pub struct AutoDespawner
{
    sender: Sender<Entity>,
    receiver: Receiver<Entity>,
}

impl AutoDespawner
{
    fn new() -> Self
    {
        let (sender, receiver) = crossbeam::channel::unbounded();
        Self{ sender, receiver }
    }

    /// Prepare an entity to be automatically despawned.
    ///
    /// When the last copy of the returned signal is dropped, the entity will be despawned in the `Last` schedule.
    pub fn prepare(&self, entity: Entity) -> AutoDespawnSignal
    {
        AutoDespawnSignal::new(entity, self.sender.clone())
    }

    /// Removes one pending despawned entity.
    pub(crate) fn try_recv(&self) -> Option<Entity>
    {
         self.receiver.try_recv().ok()
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// RAII handle to a despawn signal.
///
/// The signal can be cloned. When the last copy is dropped, the entity will be despawned in the `Last` schedule or the
/// next time a reaction tree runs.
pub struct AutoDespawnSignal(Arc<AutoDespawnSignalInner>);

impl AutoDespawnSignal
{
    fn new(entity: Entity, sender: Sender<Entity>) -> Self
    {
        Self(Arc::new(AutoDespawnSignalInner{ entity, sender }))
    }

    pub fn entity(&self) -> Entity
    {
        self.0.entity
    }
}

impl Clone for AutoDespawnSignal
{
    fn clone(&self) -> Self { Self(self.0.clone()) }
}

//-------------------------------------------------------------------------------------------------------------------

#[derive(SystemSet, Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct AutoDespawnSet;

//-------------------------------------------------------------------------------------------------------------------

/// Extends the `App` API with a method to set up auto despawning.
pub trait AutoDespawnAppExt
{
    /// Set up auto despawning. Can be added to multiple plugins without conflict.
    fn setup_auto_despawn(&mut self) -> &mut Self;
}

impl AutoDespawnAppExt for App
{
    fn setup_auto_despawn(&mut self) -> &mut Self
    {
        if self.world().contains_resource::<AutoDespawner>() { return self; }
        self.insert_resource(AutoDespawner::new())
            .add_systems(Last, garbage_collect_entities.in_set(AutoDespawnSet))
    }
}

//-------------------------------------------------------------------------------------------------------------------
