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
}

//-------------------------------------------------------------------------------------------------------------------

/// Component that stores reactor handles that target a specific entity.
#[derive(Component)]
pub(crate) struct EntityReactors
{
    reactors: SmallVec<[(EntityReactionType, AutoDespawnSignal); ENTITY_REACTORS_STATIC_SIZE]>,
}

impl EntityReactors
{
    pub(crate) fn insert(&mut self, rtype: EntityReactionType, signal: AutoDespawnSignal)
    {
        self.reactors.push((rtype, signal));
    }

    pub(crate) fn remove(&mut self, rtype: EntityReactionType, reactor_id : u64)
    {
        self.reactors.drain_filter(
                |(reaction_type, signal)|
                {
                    if *reaction_type != rtype { return false; }
                    if signal.entity().to_bits() != reactor_id { return false; }
                    true
                }
            );
    }

    pub(crate) fn count(&self, rtype: EntityReactionType) -> usize
    {
        self.iter_rtype(rtype).count()
    }

    pub(crate) fn iter_rtype(&self, rtype: EntityReactionType) -> impl Iterator<Item = SystemCommand> + '_
    {
        self.reactors
            .iter()
            .filter_map(
                move |(reaction_type, signal)|
                {
                    if *reaction_type != rtype { return None; }
                    Some(SystemCommand(signal.entity()))
                }
            )
    }
}

impl Default for EntityReactors
{
    fn default() -> Self
    {
        Self{
            reactors : SmallVec::default(),
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
    ComponentInsertion(TypeId),
    ComponentMutation(TypeId),
    ComponentRemoval(TypeId),
    ResourceMutation(TypeId),
    Broadcast(TypeId),
    Despawn(Entity),
}

//-------------------------------------------------------------------------------------------------------------------

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
