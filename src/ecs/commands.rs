//local shortcuts
use crate::*;

//third-party shortcuts
use bevy::prelude::*;

//standard shortcuts
use std::any::TypeId;
use std::hash::Hash;

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn end_system_event(world: &mut World)
{
    let data_entity = world.resource_mut::<SystemEventAccessTracker>().end();
    world.despawn(data_entity);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn end_entity_reaction(world: &mut World)
{
    world.resource_mut::<EntityReactionAccessTracker>().end();
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn end_entity_event(world: &mut World)
{
    world.resource_mut::<EntityEventAccessTracker>().end();
    // note: cleanup is end_entity_event_with_cleanup()
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn end_entity_event_with_cleanup(world: &mut World)
{
    let data_entity = world.resource_mut::<EntityEventAccessTracker>().end();
    world.despawn(data_entity);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn end_broadcast_event(world: &mut World)
{
    world.resource_mut::<BroadcastEventAccessTracker>().end();
    // note: cleanup is end_broadcast_event_with_cleanup()
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn end_broadcast_event_with_cleanup(world: &mut World)
{
    let data_entity = world.resource_mut::<BroadcastEventAccessTracker>().end();
    world.despawn(data_entity);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// A system command.
#[derive(Debug, Copy, Clone, Deref)]
pub(crate) struct SystemCommand(pub(crate) SysId);

impl SystemCommand
{
    pub(crate) fn run(self, &mut World)
    {
        system_runner(world, self.0, SystemCommandCleanup::default());
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// A system event command.
//todo: validate that data entities will always be cleaned up
#[derive(Debug, Copy, Clone)]
pub(crate) struct EventCommand
{
    pub(crate) system: SysId,
    pub(crate) data_entity: Entity,
}

impl EventCommand
{
    /// Runs this event command on the world.
    pub(crate) fn run(self, &mut World)
    {
        world.resource_mut::<SystemEventAccessTracker>().start(self.data_entity);
        system_runner(world, self.system, SystemCommandCleanup::new(end_system_event));
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// A reaction command.
//todo: validate that data entities will always be cleaned up
#[derive(Debug, Copy, Clone)]
pub(crate) enum ReactionCommand
{
    /// A reaction to a resource mutation.
    ResourceReaction
    {
        /// The system command triggered by this event.
        reactor: SysId,
    },
    /// A reaction to an entity mutation.
    EntityReaction
    {
        /// The entity that triggered this reaction.
        reaction_source: Entity,
        /// The type of the entity reaction trigger.
        reaction_type: EntityReactionType,
        /// The system command triggered by this event.
        reactor: SysId,
    },
    /// A reaction to an event targeted at a specific entity.
    EntityEvent
    {
        /// Entity where the event data is stored.
        data_entity: Entity,
        /// The system command triggered by this event.
        reactor: SysId,
        /// True if this is the last reaction that will read this event.
        last_reader: bool,
    },
    /// A reaction to a broadcasted event.
    BroadcastEvent
    {
        /// Entity where the event data is stored.
        data_entity: Entity,
        /// The system command triggered by this event.
        reactor: SysId,
        /// True if this is the last reaction that will read this event.
        last_reader: bool,
    },
}

impl ReactionCommand
{
    pub(crate) fn run(self, &mut World)
    {
        match self
        {
            Self::ResourceReaction{ reactor } =>
            {
                system_runner(world, reactor, SystemCommandCleanup::default());
            }
            Self::EntityReaction{ reaction_source, reaction_type, reactor } =>
            {
                world.resource_mut::<EntityReactionAccessTracker>().start(reaction_source, reaction_type);
                system_runner(world, reactor, SystemCommandCleanup::new(end_entity_reaction, None));
            }
            Self::EntityEvent{ data_entity, reactor, last_reader } =>
            {
                world.resource_mut::<EntityEventAccessTracker>().start(data_entity);
                let cleanup = if last_reader { end_entity_event_with_cleanup } else { end_entity_event };
                system_runner(world, reactor, SystemCommandCleanup::new(cleanup));
            }
            Self::BroadcastEvent{ data_entity, reactor, last_reader } =>
            {
                world.resource_mut::<BroadcastEventAccessTracker>().start(data_entity);
                let cleanup = if last_reader { end_broadcast_event_with_cleanup } else { end_broadcast_event };
                system_runner(world, reactor, SystemCommandCleanup::new(cleanup));
            }
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------
