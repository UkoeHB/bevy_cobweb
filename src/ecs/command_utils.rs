//local shortcuts
use crate::*;

//third-party shortcuts
use bevy::prelude::*;

//standard shortcuts
use std::any::TypeId;
use std::hash::Hash;

//-------------------------------------------------------------------------------------------------------------------

/// Queues a system command to be executed in the world.
pub(crate) fn send_system_command(queue: &mut CobwebCommandQueue<SystemCommand>, system: SysId)
{
    queue.push(SystemCommand(system));
}

//-------------------------------------------------------------------------------------------------------------------

/// Queues a system event to be executed in the world.
pub(crate) fn send_system_event<T: Send + Sync + 'static>(
    commands : &mut Commands,
    queue    : &mut CobwebCommandQueue<EventCommand>,
    system   : SysId,
    data     : T
){
    let data_entity = commands.spawn(SystemEventData::new(data)).id();
    queue.push(EventCommand{ system, data_entity });
}

//-------------------------------------------------------------------------------------------------------------------

/// Queues a reaction to a resource mutation to be executed in the world.
pub(crate) fn send_resource_reaction(queue: &mut CobwebCommandQueue<ReactionCommand>, reactor: SysId)
{
    queue.push(ReactionCommand::ResourceReaction{ reactor });
}

//-------------------------------------------------------------------------------------------------------------------

/// Queues a reaction to an entity mutation to be executed in the world.
pub(crate) fn send_entity_reaction(
    queue           : &mut CobwebCommandQueue<ReactionCommand>,
    reaction_source : Entity,
    reaction_type   : EntityReactionType,
    reactor         : SysId
){
    queue.push(ReactionCommand::EntityReaction{ reaction_source, reaction_type, reactor });
}

//-------------------------------------------------------------------------------------------------------------------

/// Inserts a broadcast event's data in the world to be read by reactors.
pub(crate) fn prepare_broadcast_event<T: Send + Sync + 'static>(commands: &mut Commands, data: T) -> Entity
{
    commands.spawn(BroadcastEventData::new(data)).id()
}

//-------------------------------------------------------------------------------------------------------------------

/// Queues a broadcast event to be executed in the world.
pub(crate) fn send_broadcast_event<T: Send + Sync + 'static>(
    queue       : &mut CobwebCommandQueue<ReactionCommand>,
    data_entity : Entity,
    reactor     : SysId,
    last_reader : bool
){
    queue.push(ReactionCommand::BroadcastEvent{ data_entity, reactor, last_reader });
}

//-------------------------------------------------------------------------------------------------------------------

/// Inserts an entity event's data in the world to be read by reactors.
pub(crate) fn prepare_entity_event<T: Send + Sync + 'static>(commands: &mut Commands, target_entity: Entity, data: T) -> Entity
{
    commands.spawn(EntityEventData::new(target_entity, data)).id()
}

//-------------------------------------------------------------------------------------------------------------------

/// Queues an entity event to be executed in the world.
pub(crate) fn send_entity_event<T: Send + Sync + 'static>(
    queue         : &mut CobwebCommandQueue<ReactionCommand>,
    data_entity   : Entity,
    reactor       : SysId,
    last_reader   : bool
){
    queue.push(ReactionCommand::EntityEvent{ data_entity, reactor, last_reader });
}

//-------------------------------------------------------------------------------------------------------------------
