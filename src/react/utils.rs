//local shortcuts
use crate::prelude::*;

//third-party shortcuts
use bevy::prelude::*;

//standard shortcuts
use core::any::TypeId;
use std::collections::HashMap;
use std::sync::Arc;

//-------------------------------------------------------------------------------------------------------------------

/// Queues removal and despawn reactors.
///
/// This system should be scheduled manually if you want to promptly detect removals or despawns that occur after
/// normal systems that don't trigger other reactions.
pub fn schedule_removal_and_despawn_reactors(world: &mut World)
{
    let mut react_cache = world.remove_resource::<ReactCache>().unwrap();
    react_cache.schedule_removal_reactions(world);
    react_cache.schedule_despawn_reactions(world);
    world.insert_resource(react_cache);
}

//-------------------------------------------------------------------------------------------------------------------

#[derive(Component)]
pub(crate) struct EntityReactors
{
    pub(crate) insertion_callbacks : HashMap<TypeId, Vec<AutoDespawnSignal>>,
    pub(crate) mutation_callbacks  : HashMap<TypeId, Vec<AutoDespawnSignal>>,
    pub(crate) removal_callbacks   : HashMap<TypeId, Vec<AutoDespawnSignal>>,
}

impl EntityReactors
{
    pub(crate) fn is_empty(&self) -> bool
    {
        self.insertion_callbacks.is_empty() &&
        self.mutation_callbacks.is_empty()  &&
        self.removal_callbacks.is_empty()  
    }
}

impl Default for EntityReactors
{
    fn default() -> Self
    {
        Self{
            insertion_callbacks : HashMap::new(),
            mutation_callbacks  : HashMap::new(),
            removal_callbacks   : HashMap::new(),
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------

#[derive(Clone, Eq, PartialEq, Debug)]
pub enum ReactorType
{
    EntityInsertion(Entity, TypeId),
    EntityMutation(Entity, TypeId),
    EntityRemoval(Entity, TypeId),
    ComponentInsertion(TypeId),
    ComponentMutation(TypeId),
    ComponentRemoval(TypeId),
    ResourceMutation(TypeId),
    Broadcast(TypeId),
    EntityEvent(Entity, TypeId),
    Despawn(Entity),
}

/// Token for revoking reactors.
///
/// See [`ReactCommands::revoke()`].
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct RevokeToken
{
    pub(crate) reactors : Arc<[ReactorType]>,
    pub(crate) id       : u64,
}

//-------------------------------------------------------------------------------------------------------------------
