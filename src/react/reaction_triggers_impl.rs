//local shortcuts
use crate::prelude::*;
use bevy_kot_utils::Sender;

//third-party shortcuts
use bevy::prelude::*;

//standard shortcuts
use core::any::TypeId;
use std::marker::PhantomData;

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// Tag for tracking despawns of entities with despawn reactors.
#[derive(Component)]
struct DespawnTracker
{
    parent   : Entity,
    notifier : Sender<Entity>,
}

impl Drop for DespawnTracker
{
    fn drop(&mut self)
    {
        let _ = self.notifier.send(self.parent);
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn add_despawn_tracker(In((entity, notifier)): In<(Entity, Sender<Entity>)>, world: &mut World)
{
    // try to get the entity
    // - if the entity doesn't exist, then notify the reactor in case we have despawn reactors waiting
    let Some(mut entity_mut) = world.get_entity_mut(entity)
    else
    {
        let _ = notifier.send(entity);
        return;
    };

    // leave if entity already has a despawn tracker
    // - we don't want to accidentally trigger `DespawnTracker::drop()` by replacing the existing component
    if entity_mut.contains::<DespawnTracker>() { return; }

    // insert a new despawn tracker
    entity_mut.insert(DespawnTracker{ parent: entity, notifier });
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// Adds a reactor to an entity.
///
/// The reactor will be invoked when the trigger targets the entity.
fn register_entity_reactor(
    In((
        rtype,
        entity,
        sys_handle
    ))                  : In<(EntityReactionType, Entity, AutoDespawnSignal)>,
    mut commands        : Commands,
    mut entity_reactors : Query<&mut EntityReactors>,
){
dbg!("registering", entity);
    // callback adder
    let add_callback_fn =
        move |entity_reactors: &mut EntityReactors|
        {
            let callbacks = match rtype
            {
                EntityReactionType::Insertion(comp_id) => entity_reactors.insertion_callbacks.entry(comp_id).or_default(),
                EntityReactionType::Mutation(comp_id)  => entity_reactors.mutation_callbacks.entry(comp_id).or_default(),
                EntityReactionType::Removal(comp_id)   => entity_reactors.removal_callbacks.entry(comp_id).or_default(),
            };
            callbacks.push(sys_handle);
        };

    // add callback to entity
    match entity_reactors.get_mut(entity)
    {
        Ok(mut entity_reactors) => add_callback_fn(&mut entity_reactors),
        _ =>
        {
            let Some(mut entity_commands) = commands.get_entity(entity) else { return; };

            // make new reactor tracker for the entity
            let mut entity_reactors = EntityReactors::default();

            // add callback and insert to entity
            add_callback_fn(&mut entity_reactors);
            entity_commands.insert(entity_reactors);
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// Reaction trigger for [`ReactComponent`] insertions on any entity.
/// - For reactors that take the entity the component was inserted to.
pub struct InsertionTrigger<C: ReactComponent>(PhantomData<C>);
impl<C: ReactComponent> Default for InsertionTrigger<C> { fn default() -> Self { Self(PhantomData::default()) } }

impl<C: ReactComponent> ReactionTrigger for InsertionTrigger<C>
{
    fn register(self, rcommands: &mut ReactCommands, sys_handle: &AutoDespawnSignal) -> Option<ReactorType>
    {
        Some(rcommands.cache.register_insertion_reactor::<C>(sys_handle))
    }
}

/// Returns a [`InsertionTrigger`] reaction trigger.
pub fn insertion<C: ReactComponent>() -> InsertionTrigger<C> { InsertionTrigger::default() }

//-------------------------------------------------------------------------------------------------------------------

/// Reaction trigger for [`ReactComponent`] mutations on any entity.
/// - For reactors that take the entity the component was mutated on.
pub struct MutationTrigger<C: ReactComponent>(PhantomData<C>);
impl<C: ReactComponent> Default for MutationTrigger<C> { fn default() -> Self { Self(PhantomData::default()) } }

impl<C: ReactComponent> ReactionTrigger for MutationTrigger<C>
{
    fn register(self, rcommands: &mut ReactCommands, sys_handle: &AutoDespawnSignal) -> Option<ReactorType>
    {
        Some(rcommands.cache.register_mutation_reactor::<C>(sys_handle))
    }
}

/// Returns a [`MutationTrigger`] reaction trigger.
pub fn mutation<C: ReactComponent>() -> MutationTrigger<C> { MutationTrigger::default() }

//-------------------------------------------------------------------------------------------------------------------

/// Reaction trigger for [`ReactComponent`] removals from any entity.
/// - For reactors that take the entity the component was removed from.
/// - Reactions are not triggered if the entity was despawned.
pub struct RemovalTrigger<C: ReactComponent>(PhantomData<C>);
impl<C: ReactComponent> Default for RemovalTrigger<C> { fn default() -> Self { Self(PhantomData::default()) } }

impl<C: ReactComponent> ReactionTrigger for RemovalTrigger<C>
{
    fn register(self, rcommands: &mut ReactCommands, sys_handle: &AutoDespawnSignal) -> Option<ReactorType>
    {
        rcommands.cache.track_removals::<C>();
        Some(rcommands.cache.register_removal_reactor::<C>(sys_handle))
    }
}

/// Returns a [`RemovalTrigger`] reaction trigger.
pub fn removal<C: ReactComponent>() -> RemovalTrigger<C> { RemovalTrigger::default() }

//-------------------------------------------------------------------------------------------------------------------

/// Reaction trigger for [`ReactComponent`] insertions on a specific entity.
/// - Registration does nothing if the entity does not exist.
pub struct EntityInsertionTrigger<C: ReactComponent>(Entity, PhantomData<C>);

impl<C: ReactComponent> ReactionTrigger for EntityInsertionTrigger<C>
{
    fn register(self, rcommands: &mut ReactCommands, sys_handle: &AutoDespawnSignal) -> Option<ReactorType>
    {
        let comp_id = TypeId::of::<C>();
        let entity = self.0;
        let sys_handle = sys_handle.clone();

        rcommands.commands.add(
                move |world: &mut World|
                syscall(world, (EntityReactionType::Insertion(comp_id), entity, sys_handle), register_entity_reactor)
            );

        Some(ReactorType::EntityInsertion(entity, comp_id))
    }
}

/// Returns a [`EntityInsertionTrigger`] reaction trigger.
pub fn entity_insertion<C: ReactComponent>(entity: Entity) -> EntityInsertionTrigger<C>
{
    EntityInsertionTrigger(entity, PhantomData::default())
}

//-------------------------------------------------------------------------------------------------------------------

/// Reaction trigger for [`ReactComponent`] mutations on a specific entity.
/// - Registration does nothing if the entity does not exist.
pub struct EntityMutationTrigger<C: ReactComponent>(Entity, PhantomData<C>);

impl<C: ReactComponent> ReactionTrigger for EntityMutationTrigger<C>
{
    fn register(self, rcommands: &mut ReactCommands, sys_handle: &AutoDespawnSignal) -> Option<ReactorType>
    {
        let comp_id = TypeId::of::<C>();
        let entity = self.0;
        let sys_handle = sys_handle.clone();

        rcommands.commands.add(
                move |world: &mut World|
                syscall(world, (EntityReactionType::Mutation(comp_id), entity, sys_handle), register_entity_reactor)
            );

        Some(ReactorType::EntityMutation(entity, comp_id))
    }
}

/// Returns a [`EntityMutationTrigger`] reaction trigger.
pub fn entity_mutation<C: ReactComponent>(entity: Entity) -> EntityMutationTrigger<C>
{
    EntityMutationTrigger(entity, PhantomData::default())
}

//-------------------------------------------------------------------------------------------------------------------

/// Reaction trigger for [`ReactComponent`] removals from a specific entity.
/// - Registration does nothing if the entity does not exist.
/// - If a component is removed from the entity then despawned (or removed due to a despawn) before
///   [`react_to_removals()`] is executed, then the reactor will not be scheduled.
pub struct EntityRemovalTrigger<C: ReactComponent>(Entity, PhantomData<C>);

impl<C: ReactComponent> ReactionTrigger for EntityRemovalTrigger<C>
{
    fn register(self, rcommands: &mut ReactCommands, sys_handle: &AutoDespawnSignal) -> Option<ReactorType>
    {
        let comp_id = TypeId::of::<C>();
        let entity = self.0;
        let sys_handle = sys_handle.clone();

        rcommands.cache.track_removals::<C>();

        rcommands.commands.add(
                move |world: &mut World|
                syscall(world, (EntityReactionType::Removal(comp_id), entity, sys_handle), register_entity_reactor)
            );

        Some(ReactorType::EntityRemoval(entity, comp_id))
    }
}

/// Returns a [`EntityRemovalTrigger`] reaction trigger.
pub fn entity_removal<C: ReactComponent>(entity: Entity) -> EntityRemovalTrigger<C>
{
    EntityRemovalTrigger(entity, PhantomData::default())
}

//-------------------------------------------------------------------------------------------------------------------

/// Reaction trigger for [`ReactResource`] mutations.
pub struct ResourceMutationTrigger<R: ReactResource>(PhantomData<R>);
impl<R: ReactResource> Default for ResourceMutationTrigger<R> { fn default() -> Self { Self(PhantomData::default()) } }

impl<R: ReactResource> ReactionTrigger for ResourceMutationTrigger<R>
{
    fn register(self, rcommands: &mut ReactCommands, sys_handle: &AutoDespawnSignal) -> Option<ReactorType>
    {
        Some(rcommands.cache.register_resource_mutation_reactor::<R>(sys_handle))
    }
}

/// Returns a [`ResourceMutationTrigger`] reaction trigger.
pub fn resource_mutation<R: ReactResource>() -> ResourceMutationTrigger<R> { ResourceMutationTrigger::default() }

//-------------------------------------------------------------------------------------------------------------------

/// Reaction trigger for broadcast events.
/// - Reactions only occur for events sent via [`ReactCommands::<E>::broadcast()`].
pub struct BroadcastEventTrigger<E: Send + Sync + 'static>(PhantomData<E>);
impl<E: Send + Sync + 'static> Default for BroadcastEventTrigger<E> { fn default() -> Self { Self(PhantomData::default()) } }

impl<E: Send + Sync + 'static> ReactionTrigger for BroadcastEventTrigger<E>
{
    fn register(self, rcommands: &mut ReactCommands, sys_handle: &AutoDespawnSignal) -> Option<ReactorType>
    {
        Some(rcommands.cache.register_event_reactor::<E>(sys_handle))
    }
}

/// Returns a [`BroadcastEventTrigger`] reaction trigger.
pub fn broadcast<E: Send + Sync + 'static>() -> BroadcastEventTrigger<E> { BroadcastEventTrigger::default() }

//-------------------------------------------------------------------------------------------------------------------

/// Reaction trigger for entity events.
/// - Reactions only occur for events sent via [`ReactCommands::<E>::entity_event()`].
pub struct EntityEventTrigger<E: Send + Sync + 'static>(Entity, PhantomData<E>);

impl<E: Send + Sync + 'static> ReactionTrigger for EntityEventTrigger<E>
{
    fn register(self, rcommands: &mut ReactCommands, sys_handle: &AutoDespawnSignal) -> Option<ReactorType>
    {
        Some(rcommands.cache.register_entity_event_reactor::<E>(self.0, sys_handle))
    }
}

/// Returns an [`EntityEventTrigger`] reaction trigger.
pub fn entity_event<E: Send + Sync + 'static>(target: Entity) -> EntityEventTrigger<E>
{
    EntityEventTrigger(target, PhantomData::default())
}

//-------------------------------------------------------------------------------------------------------------------

/// Reaction trigger for despawns.
pub struct DespawnTrigger(Entity);

impl ReactionTrigger for DespawnTrigger
{
    fn register(self, rcommands: &mut ReactCommands, sys_handle: &AutoDespawnSignal) -> Option<ReactorType>
    {
        // check if the entity exists
        let Some(_) = rcommands.commands.get_entity(self.0) else { return None; };

        // add despawn tracker
        let notifier = rcommands.cache.despawn_sender();
        rcommands.commands.add(move |world: &mut World| syscall(world, (self.0, notifier), add_despawn_tracker));

        Some(rcommands.cache.register_despawn_reactor(self.0, sys_handle))
    }
}

/// Returns a [`DespawnTrigger`] reaction trigger.
pub fn despawn(entity: Entity) -> DespawnTrigger { DespawnTrigger(entity) }

//-------------------------------------------------------------------------------------------------------------------
