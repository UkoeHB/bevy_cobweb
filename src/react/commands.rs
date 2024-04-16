//local shortcuts
use crate::prelude::*;

//third-party shortcuts
use bevy::ecs::system::Command;
use bevy::prelude::*;

//standard shortcuts
use std::any::TypeId;

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

fn end_despawn_reaction(world: &mut World)
{
    world.resource_mut::<DespawnAccessTracker>().end();
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn end_entity_event(world: &mut World)
{
    end_entity_reaction(world);
    world.resource_mut::<EventAccessTracker>().end();
    // note: data cleanup is end_event_with_cleanup()
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn end_entity_event_with_cleanup(world: &mut World)
{
    end_entity_reaction(world);
    let data_entity = world.resource_mut::<EventAccessTracker>().end();
    world.despawn(data_entity);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn end_broadcast_event(world: &mut World)
{
    world.resource_mut::<EventAccessTracker>().end();
    // note: data cleanup is end_event_with_cleanup()
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn end_broadcast_event_with_cleanup(world: &mut World)
{
    let data_entity = world.resource_mut::<EventAccessTracker>().end();
    world.despawn(data_entity);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// A system command.
///
/// System commands are stored on entities and must be manually scheduled with
/// [`command.apply()`](bevy::ecs::system::Command::apply) or
/// [`commands.send_system_event()`](super::ReactCommandsExt::send_system_event).
///
/// You can spawn your own system command with
/// [`commands.spawn_system_command()`](super::ReactCommandsExt::spawn_system_command).
///
/// All reactors are stored as system commands (i.e. systems registered with [`ReactCommands::on`]).
///
/// If scheduled as a [`Command`](bevy::ecs::system::Command) from user-land, this will cause a [`reaction_tree()`] to
/// execute, otherwise it will be processed within the already-running reaction tree.
#[derive(Debug, Copy, Clone, Deref, Eq, PartialEq)]
pub struct SystemCommand(pub Entity);

impl SystemCommand
{
    pub(crate) fn run(self, world: &mut World)
    {
        syscommand_runner(world, self, SystemCommandCleanup::default());
    }
}

impl Command for SystemCommand
{
    fn apply(self, world: &mut World)
    {
        world.resource_mut::<CobwebCommandQueue<SystemCommand>>().push(self);
        reaction_tree(world);
    }
}

impl From<RevokeToken> for SystemCommand
{
    fn from(token: RevokeToken) -> Self
    {
        token.id
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// A system event command.
///
/// System events are sent with  [`commands.send_system_event()`](super::ReactCommandsExt::send_system_event).
///
/// If scheduled as a `Command` from user-land, this will cause a [`reaction_tree()`] to execute, otherwise it will be
/// processed within the already-running reaction tree.
#[derive(Debug, Copy, Clone)]
pub(crate) struct EventCommand
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
    pub(crate) fn run(self, world: &mut World)
    {
        world.resource_mut::<SystemEventAccessTracker>().start(self.data_entity);
        syscommand_runner(world, self.system, SystemCommandCleanup::new(end_system_event));
    }
}

impl Command for EventCommand
{
    fn apply(self, world: &mut World)
    {
        world.resource_mut::<CobwebCommandQueue<EventCommand>>().push(self);
        reaction_tree(world);
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// A reaction command.
///
/// Reaction commands are sent by the internals of [`ReactCommands`].
///
/// If scheduled as a `Command` from user-land, this will cause a [`reaction_tree()`] to execute, otherwise it will be
/// processed within the already-running reaction tree.
#[derive(Clone)]
pub(crate) enum ReactionCommand
{
    /// A reaction to a resource mutation.
    Resource
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
    /// A reaction to an entity despawn.
    Despawn
    {
        /// The entity that triggered this reaction.
        reaction_source: Entity,
        /// The system command triggered by this event.
        reactor: SystemCommand,
        /// A possible despawn handle for the reactor.
        ///
        /// This will be dropped after the reactor runs, ensuring the reactor will be cleaned up if there are
        /// no other owners of the handle.
        handle: ReactorHandle,
    },
    /// A reaction to an entity event.
    EntityEvent
    {
        /// Target entity for the event.
        target: Entity,
        /// Entity where the event data is stored.
        data_entity: Entity,
        /// The system command triggered by this event.
        reactor: SystemCommand,
        /// True if this is the last reaction that will read this event.
        ///
        /// The `data_entity` will despawned in the system command cleanup callback if this is true.
        last_reader: bool,
    },
    /// A reaction to a broadcast event.
    BroadcastEvent
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
    /// Runs the reaction on the world.
    pub(crate) fn run(self, world: &mut World)
    {
        match self
        {
            Self::Resource{ reactor } =>
            {
                syscommand_runner(world, reactor, SystemCommandCleanup::default());
            }
            Self::EntityReaction{ reaction_source, reaction_type, reactor } =>
            {
                world.resource_mut::<EntityReactionAccessTracker>().start(reactor, reaction_source, reaction_type);
                syscommand_runner(world, reactor, SystemCommandCleanup::new(end_entity_reaction));
            }
            Self::Despawn{ reaction_source, reactor, handle } =>
            {
                world.resource_mut::<DespawnAccessTracker>().start(reaction_source, handle);
                syscommand_runner(world, reactor, SystemCommandCleanup::new(end_despawn_reaction));
            }
            Self::EntityEvent{ target, data_entity, reactor, last_reader } =>
            {
                // Include entity reaction tracker for EntityWorldReactor.
                world.resource_mut::<EntityReactionAccessTracker>().start(
                    reactor,
                    target,
                    EntityReactionType::Event(TypeId::of::<()>()),
                );
                world.resource_mut::<EventAccessTracker>().start(data_entity);
                let cleanup = if last_reader { end_entity_event_with_cleanup } else { end_entity_event };
                syscommand_runner(world, reactor, SystemCommandCleanup::new(cleanup));
            }
            Self::BroadcastEvent{ data_entity, reactor, last_reader } =>
            {
                world.resource_mut::<EventAccessTracker>().start(data_entity);
                let cleanup = if last_reader { end_broadcast_event_with_cleanup } else { end_broadcast_event };
                syscommand_runner(world, reactor, SystemCommandCleanup::new(cleanup));
            }
        }
    }
}

impl Command for ReactionCommand
{
    fn apply(self, world: &mut World)
    {
        world.resource_mut::<CobwebCommandQueue<ReactionCommand>>().push(self);
        reaction_tree(world);
    }
}

//-------------------------------------------------------------------------------------------------------------------
