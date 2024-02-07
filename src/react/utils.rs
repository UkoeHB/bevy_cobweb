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

/// Queue a command with a call to react to all removals and despawns.
///
/// Note that we assume the specified command internally handles its deferred state. We don't want to call
/// `apply_deferred` here since the global `apply_deferred` is inefficient.
pub(crate) fn enque_command(commands: &mut Commands, cb: impl Command)
{
    commands.add(
            move |world: &mut World|
            {
                cb.apply(world);
                react_to_all_removals_and_despawns(world);
            }
        );
}

//-------------------------------------------------------------------------------------------------------------------

/// Queue a named system then react to all removals and despawns.
/// - Note that all side effects and chained reactions will be applied when the syscall applies its deferred commands.
///   This means this reaction's effects will be propagated before any 'sibling' reactions/commands.
pub(crate) fn enque_reaction<I: Send + Sync + 'static>(commands: &mut Commands, sys_id: SysId, input: I)
{
    commands.add(
            move |world: &mut World|
            {
                let Ok(()) = spawned_syscall::<I, ()>(world, sys_id, input)
                else { tracing::warn!(?sys_id, "reaction system failed"); return; };
                react_to_all_removals_and_despawns(world);
            }
        );
}

//-------------------------------------------------------------------------------------------------------------------

pub(crate) enum EntityReactType
{
    Insertion,
    Mutation,
    Removal,
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
