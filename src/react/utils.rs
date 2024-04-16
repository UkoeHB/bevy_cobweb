//local shortcuts
use crate::prelude::*;

//third-party shortcuts
use bevy::prelude::*;
use smallvec::SmallVec;

//standard shortcuts
use core::any::TypeId;
use std::sync::Arc;

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

const ENTITY_REACTORS_STATIC_SIZE: usize = 4;

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// Queues removal and despawn reactors.
///
/// This system should be scheduled manually if you want to promptly detect removals or despawns that occur after
/// normal systems that don't trigger other reactions.
pub fn schedule_removal_and_despawn_reactors(world: &mut World)
{
    let mut cache = world.remove_resource::<ReactCache>().unwrap();
    cache.schedule_removal_reactions(world);
    cache.schedule_despawn_reactions(world);
    world.insert_resource(cache);
}

//-------------------------------------------------------------------------------------------------------------------

/// The type of an entity reaction.
//todo: switch to ComponentId when observers are integrated
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(crate) enum EntityReactionType
{
    /// A component was inserted.
    Insertion(TypeId),
    /// A component was mutated.
    Mutation(TypeId),
    /// A component was removed.
    Removal(TypeId),
    /// An event was sent to this entity.
    Event(TypeId),
    /// This entity was despawned.
    Despawn,
}

//-------------------------------------------------------------------------------------------------------------------

/// Component that stores reactor handles that target a specific entity.
#[derive(Component)]
pub(crate) struct EntityReactors
{
    reactors: SmallVec<[(EntityReactionType, ReactorHandle); ENTITY_REACTORS_STATIC_SIZE]>,
}

impl EntityReactors
{
    pub(crate) fn insert(&mut self, rtype: EntityReactionType, handle: ReactorHandle)
    {
        self.reactors.push((rtype, handle));
    }

    pub(crate) fn remove(&mut self, rtype: EntityReactionType, reactor_id: SystemCommand)
    {
        self.reactors.drain_filter(
                |(reaction_type, handle)|
                {
                    if *reaction_type != rtype { return false; }
                    if handle.sys_command() != reactor_id { return false; }
                    true
                }
            );
    }

    pub(crate) fn count(&self, rtype: EntityReactionType) -> usize
    {
        self.iter_rtype(rtype).count()
    }

    pub(crate) fn iter_reactors(&self) -> impl Iterator<Item = SystemCommand> + '_
    {
        self.reactors
            .iter()
            .map(|(_, handle)| handle.sys_command())
    }

    pub(crate) fn iter_rtype(&self, rtype: EntityReactionType) -> impl Iterator<Item = SystemCommand> + '_
    {
        self.reactors
            .iter()
            .filter_map(
                move |(reaction_type, handle)|
                {
                    if *reaction_type != rtype { return None; }
                    Some(handle.sys_command())
                }
            )
    }
}

impl Default for EntityReactors
{
    fn default() -> Self
    {
        Self{
            reactors: SmallVec::default(),
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
    EntityEvent(Entity, TypeId),
    AnyEntityEvent(TypeId),
    ComponentInsertion(TypeId),
    ComponentMutation(TypeId),
    ComponentRemoval(TypeId),
    ResourceMutation(TypeId),
    Broadcast(TypeId),
    Despawn(Entity),
}

impl ReactorType
{
    /// Gets the entity targeted by this reactor type if it's an entity-specific reactor.
    pub fn get_entity(&self) -> Option<Entity>
    {
        match *self
        {
            Self::EntityInsertion(entity, _) |
            Self::EntityMutation(entity, _) |
            Self::EntityRemoval(entity, _) |
            Self::EntityEvent(entity, _) |
            Self::Despawn(entity) => Some(entity),
            Self::AnyEntityEvent(_) |
            Self::ComponentInsertion(_) |
            Self::ComponentMutation(_) |
            Self::ComponentRemoval(_) |
            Self::ResourceMutation(_) |
            Self::Broadcast(_) => None,
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Token for revoking reactors.
///
/// See [`ReactCommands::revoke()`].
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct RevokeToken
{
    pub(crate) reactors : Arc<[ReactorType]>,
    pub(crate) id       : SystemCommand,
}

impl RevokeToken
{
    /// Makes a new token from raw parts.
    ///
    /// This is useful for manually removing triggers from persistent reactors. See [`Reactor::remove_triggers`].
    pub(crate) fn new_from(sys_command: SystemCommand, triggers: impl ReactionTriggerBundle) -> Self
    {
        Self{
            reactors : Arc::from(get_reactor_types(triggers).as_slice()),
            id       : sys_command,
        }
    }

    /// Not efficient for very large numbers of reactors, which is unlikely to be an issue in practice.
    pub(crate) fn iter_unique_entities(&self) -> impl Iterator<Item = Entity> + '_
    {
        self.reactors
            .iter()
            .enumerate()
            .filter_map(
                |(idx, reactor)|
                {
                    let Some(entity) = reactor.get_entity() else { return None };
                    for idx in 0..idx
                    {
                        if self.reactors[idx].get_entity() == Some(entity) { return None }
                    }
                    Some(entity)
                }
            )
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Handle for managing a reactor within the react backend.
#[derive(Clone)]
pub enum ReactorHandle
{
    Persistent(SystemCommand),
    AutoDespawn(AutoDespawnSignal)
}

impl ReactorHandle
{
    pub(crate) fn sys_command(&self) -> SystemCommand
    {
        match self
        {
            Self::Persistent(sys_command) => *sys_command,
            Self::AutoDespawn(signal)     => SystemCommand(signal.entity()),
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------
