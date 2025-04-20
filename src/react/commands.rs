//local shortcuts
use crate::prelude::*;

//third-party shortcuts
use bevy::prelude::*;

//standard shortcuts
use std::any::TypeId;

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn try_cleanup_data_entity(world: &mut World, entity: Entity)
{
    let Some(mut counter) = world.get_mut::<DataEntityCounter>(entity) else { return };
    counter.decrement();
    if counter.is_done() {
        world.despawn(entity);
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn start_system_event(world: &mut World, system: SystemCommand)
{
    world.resource_mut::<SystemEventAccessTracker>().start(system);
}

fn end_system_event(world: &mut World)
{
    let data_entity = world.resource_mut::<SystemEventAccessTracker>().end();
    world.despawn(data_entity);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn start_entity_reaction(world: &mut World, reactor: SystemCommand)
{
    world.resource_mut::<EntityReactionAccessTracker>().start(reactor);
}

fn end_entity_reaction(world: &mut World)
{
    world.resource_mut::<EntityReactionAccessTracker>().end();
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn start_despawn_reaction(world: &mut World, reactor: SystemCommand)
{
    world.resource_mut::<DespawnAccessTracker>().start(reactor);
}

fn end_despawn_reaction(world: &mut World)
{
    world.resource_mut::<DespawnAccessTracker>().end();
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn start_entity_event(world: &mut World, reactor: SystemCommand)
{
    start_entity_reaction(world, reactor);
    world.resource_mut::<EventAccessTracker>().start(reactor);
}

fn end_entity_event(world: &mut World)
{
    end_entity_reaction(world);
    let data_entity = world.resource_mut::<EventAccessTracker>().end();
    try_cleanup_data_entity(world, data_entity);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn start_broadcast_event(world: &mut World, reactor: SystemCommand)
{
    world.resource_mut::<EventAccessTracker>().start(reactor);
}

fn end_broadcast_event(world: &mut World)
{
    let data_entity = world.resource_mut::<EventAccessTracker>().end();
    try_cleanup_data_entity(world, data_entity);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// Helper for cleaning up event data when the last reactor has run.
#[derive(Component)]
pub(crate) struct DataEntityCounter
{
    count: usize
}

impl DataEntityCounter
{
    pub(crate) fn new(count: usize) -> Self
    {
        Self{ count }
    }

    fn decrement(&mut self)
    {
        self.count = self.count.saturating_sub(1);
    }

    fn is_done(&self) -> bool
    {
        self.count == 0
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// A system command.
///
/// System commands are stored on entities and must be manually scheduled with
/// [`command.apply()`](bevy::ecs::world::Command::apply) or
/// [`commands.send_system_event()`](super::ReactCommandsExt::send_system_event).
///
/// You can spawn your own system command with
/// [`commands.spawn_system_command()`](super::ReactCommandsExt::spawn_system_command).
///
/// All reactors are stored as system commands (i.e. systems registered with [`ReactCommands::on`]).
#[derive(Debug, Copy, Clone, Deref, Eq, PartialEq)]
pub struct SystemCommand(pub Entity);

impl Command for SystemCommand
{
    fn apply(self, world: &mut World)
    {
        syscommand_runner(world, self, SystemCommandSetup::default(), SystemCommandCleanup::default());
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

impl Command for EventCommand
{
    fn apply(self, world: &mut World)
    {
        world.resource_mut::<SystemEventAccessTracker>().prepare(self.system, self.data_entity);
        syscommand_runner(
            world,
            self.system,
            SystemCommandSetup::new(self.system, start_system_event),
            SystemCommandCleanup::new(end_system_event)
        );
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// A reaction command.
///
/// Reaction commands are sent by the internals of [`ReactCommands`].
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
    },
    /// A reaction to a broadcast event.
    BroadcastEvent
    {
        /// Entity where the event data is stored.
        data_entity: Entity,
        /// The system command triggered by this event.
        reactor: SystemCommand,
    },
}

impl Command for ReactionCommand
{
    fn apply(self, world: &mut World)
    {
        match self
        {
            Self::Resource{ reactor } =>
            {
                syscommand_runner(world, reactor, SystemCommandSetup::default(), SystemCommandCleanup::default());
            }
            Self::EntityReaction{ reaction_source, reaction_type, reactor } =>
            {
                world.resource_mut::<EntityReactionAccessTracker>().prepare(reactor, reaction_source, reaction_type);
                syscommand_runner(
                    world,
                    reactor,
                    SystemCommandSetup::new(reactor, start_entity_reaction),
                    SystemCommandCleanup::new(end_entity_reaction)
                );
            }
            Self::Despawn{ reaction_source, reactor, handle } =>
            {
                world.resource_mut::<DespawnAccessTracker>().prepare(reactor, reaction_source, handle);
                syscommand_runner(
                    world,
                    reactor,
                    SystemCommandSetup::new(reactor, start_despawn_reaction),
                    SystemCommandCleanup::new(end_despawn_reaction));
            }
            Self::EntityEvent{ target, data_entity, reactor } =>
            {
                // Include entity reaction tracker for EntityWorldReactor.
                world.resource_mut::<EntityReactionAccessTracker>().prepare(
                    reactor,
                    target,
                    EntityReactionType::Event(TypeId::of::<()>()),
                );
                world.resource_mut::<EventAccessTracker>().prepare(reactor, data_entity);
                syscommand_runner(world,
                    reactor,
                    SystemCommandSetup::new(reactor, start_entity_event),
                    SystemCommandCleanup::new(end_entity_event)
                );
            }
            Self::BroadcastEvent{ data_entity, reactor } =>
            {
                world.resource_mut::<EventAccessTracker>().prepare(reactor, data_entity);
                syscommand_runner(world,
                    reactor,
                    SystemCommandSetup::new(reactor, start_broadcast_event),
                    SystemCommandCleanup::new(end_broadcast_event)
                );
            }
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------
