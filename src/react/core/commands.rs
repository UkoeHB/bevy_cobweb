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

fn end_event(world: &mut World)
{
    world.resource_mut::<EventAccessTracker>().end();
    // note: cleanup is end_event_with_cleanup()
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn end_event_with_cleanup(world: &mut World)
{
    let data_entity = world.resource_mut::<EventAccessTracker>().end();
    world.despawn(data_entity);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// A system command.
///
/// If scheduled as a `Command` from user-land, this will cause a [`reaction_tree()`] to execute, otherwise it will be
/// processed within the already-running reaction tree.
#[derive(Debug, Copy, Clone, Deref)]
pub struct SystemCommand(pub(crate) Entity);

impl SystemCommand
{
    pub(crate) fn run(self, &mut World)
    {
        syscommand_runner(world, self, SystemCommandCleanup::default());
    }
}

impl Command for SystemCommand
{
    fn apply(self, world: &mut World)
    {
        move |world: &mut World|
        {
            world.resource_mut::<CobwebCommandQueue<SystemCommand>>().push(self);
            reaction_tree(world);
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// A system event command.
//todo: validate that data entities will always be cleaned up
///
/// If scheduled as a `Command` from user-land, this will cause a [`reaction_tree()`] to execute, otherwise it will be
/// processed within the already-running reaction tree.
#[derive(Debug, Copy, Clone)]
pub struct EventCommand
{
    /// The system command triggered by this event.
    pub(crate) system: SystemCommand,
    /// Entity where the event data is stored.
    ///
    /// This entity will despawned in the system command cleanup callback.
    pub(crate) data_entity: Entity,
}

impl EventCommand
{
    /// Runs this event command on the world.
    pub(crate) fn run(self, &mut World)
    {
        world.resource_mut::<SystemEventAccessTracker>().start(self.data_entity);
        syscommand_runner(world, self.system, SystemCommandCleanup::new(end_system_event));
    }
}

impl Command for EventCommand
{
    fn apply(self, world: &mut World)
    {
        move |world: &mut World|
        {
            world.resource_mut::<CobwebCommandQueue<EventCommand>>().push(self);
            reaction_tree(world);
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// A reaction command.
//todo: validate that data entities will always be cleaned up
///
/// If scheduled as a `Command` from user-land, this will cause a [`reaction_tree()`] to execute, otherwise it will be
/// processed within the already-running reaction tree.
#[derive(Debug, Copy, Clone)]
pub enum ReactionCommand
{
    /// A reaction to a resource mutation.
    ResourceReaction
    {
        /// The system command triggered by this event.
        reactor: SystemCommand,
    },
    /// A reaction to an entity mutation.
    EntityReaction
    {
        /// The entity that triggered this reaction.
        reaction_source: Entity,
        /// The type of the entity reaction trigger.
        reaction_type: EntityReactionType,
        /// The system command triggered by this event.
        reactor: SystemCommand,
    },
    /// A reaction to an event (can be a broadcasted event or an entity event).
    Event
    {
        /// Entity where the event data is stored.
        data_entity: Entity,
        /// The system command triggered by this event.
        reactor: SystemCommand,
        /// True if this is the last reaction that will read this event.
        ///
        /// The `data_entity` will despawned in the system command cleanup callback if this is true.
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
                syscommand_runner(world, reactor, SystemCommandCleanup::default());
            }
            Self::EntityReaction{ reaction_source, reaction_type, reactor } =>
            {
                world.resource_mut::<EntityReactionAccessTracker>().start(reaction_source, reaction_type);
                syscommand_runner(world, reactor, SystemCommandCleanup::new(end_entity_reaction, None));
            }
            Self::Event{ data_entity, reactor, last_reader } =>
            {
                world.resource_mut::<EventAccessTracker>().start(data_entity);
                let cleanup = if last_reader { end_event_with_cleanup } else { end_event };
                syscommand_runner(world, reactor, SystemCommandCleanup::new(cleanup));
            }
        }
    }
}

impl Command for ReactionCommand
{
    fn apply(self, world: &mut World)
    {
        move |world: &mut World|
        {
            world.resource_mut::<CobwebCommandQueue<ReactionCommand>>().push(self);
            reaction_tree(world);
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------
