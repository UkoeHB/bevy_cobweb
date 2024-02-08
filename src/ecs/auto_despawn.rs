//local shortcuts
use bevy_kot_utils::*;

//third-party shortcuts
use bevy::prelude::*;

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

fn auto_despawn(mut commands: Commands, despawner: Res<AutoDespawner>)
{
    while let Some(entity) = despawner.try_recv()
    {
        let Some(mut entity_commands) = commands.get_entity(entity) else { continue; };
        entity_commands.despawn();
    }
}

//-------------------------------------------------------------------------------------------------------------------
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
        let (sender, receiver) = new_channel();
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
         self.receiver.try_recv()
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// RAII handle to a despawn signal.
///
/// The signal can be cloned. When the last copy is dropped, the entity will be despawned in the `Last` schedule.
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
        if self.world.contains_resource::<AutoDespawner>() { return self; }
        self.insert_resource(AutoDespawner::new())
            .add_systems(Last, auto_despawn.in_set(AutoDespawnSet))
    }
}

//-------------------------------------------------------------------------------------------------------------------
