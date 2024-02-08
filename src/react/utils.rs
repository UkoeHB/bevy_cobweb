//local shortcuts
use crate::*;

//third-party shortcuts
use bevy::ecs::system::Command;
use bevy::prelude::*;

//standard shortcuts
use core::any::TypeId;
use std::collections::HashMap;
use std::sync::Arc;

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
    EntityInsertion(Entity, ComponentId),
    EntityMutation(Entity, ComponentId),
    EntityRemoval(Entity, ComponentId),
    ComponentInsertion(ComponentId),
    ComponentMutation(ComponentId),
    ComponentRemoval(ComponentId),
    ResourceMutation(ComponentId),
    Event(TypeId),
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
