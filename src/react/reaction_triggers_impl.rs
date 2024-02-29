//local shortcuts
use crate::prelude::*;

//third-party shortcuts
use bevy::prelude::*;
use crossbeam::channel::Sender;

//standard shortcuts
use core::any::TypeId;
use std::marker::PhantomData;

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn track_removals<C: ReactComponent>(mut cache: ResMut<ReactCache>)
{
    cache.track_removals::<C>();
}

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

fn register_insertion_reactor<C: ReactComponent>(In(signal): In<AutoDespawnSignal>, mut cache: ResMut<ReactCache>)
{
    cache.register_insertion_reactor::<C>(signal);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn register_mutation_reactor<C: ReactComponent>(In(signal): In<AutoDespawnSignal>, mut cache: ResMut<ReactCache>)
{
    cache.register_mutation_reactor::<C>(signal);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn register_removal_reactor<C: ReactComponent>(In(signal): In<AutoDespawnSignal>, mut cache: ResMut<ReactCache>)
{
    cache.track_removals::<C>();
    cache.register_removal_reactor::<C>(signal);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn register_resource_mutation_reactor<R: ReactResource>(In(signal): In<AutoDespawnSignal>, mut cache: ResMut<ReactCache>)
{
    cache.register_resource_mutation_reactor::<R>(signal);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn register_broadcast_reactor<E: Send + Sync + 'static>(In(signal): In<AutoDespawnSignal>, mut cache: ResMut<ReactCache>)
{
    cache.register_broadcast_reactor::<E>(signal);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn register_entity_event_reactor<E: Send + Sync + 'static>(
    In((target, signal)) : In<(Entity, AutoDespawnSignal)>,
    mut cache            : ResMut<ReactCache>
){
    cache.register_entity_event_reactor::<E>(target, signal);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn register_despawn_reactor(
    In((entity, signal)) : In<(Entity, AutoDespawnSignal)>,
    world                : &mut World,
){
    world.resource_scope(
        move |world, mut cache: Mut<ReactCache>|
        {
            // Check if the entity is still alive.
            let Some(mut entity_mut) = world.get_entity_mut(entity) else { return; };

            // Register the reactor.
            cache.register_despawn_reactor(entity, signal);

            // Leave if the entity already has a despawn tracker.
            // - We don't want to accidentally trigger `DespawnTracker::drop()` by replacing the existing component.
            if entity_mut.contains::<DespawnTracker>() { return; }

            // Insert a new despawn tracker.
            entity_mut.insert(DespawnTracker{ parent: entity, notifier: cache.despawn_sender() });
        }
    );
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
        signal
    ))                  : In<(EntityReactionType, Entity, AutoDespawnSignal)>,
    mut commands        : Commands,
    mut entity_reactors : Query<&mut EntityReactors>,
){
    // add callback to entity
    match entity_reactors.get_mut(entity)
    {
        Ok(mut entity_reactors) => entity_reactors.insert(rtype, signal),
        _ =>
        {
            let Some(mut entity_commands) = commands.get_entity(entity) else { return; };

            // make new reactor tracker for the entity
            let mut entity_reactors = EntityReactors::default();

            // add callback and insert to entity
            entity_reactors.insert(rtype, signal);
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
    fn register(self, commands: &mut Commands, sys_handle: &AutoDespawnSignal) -> Option<ReactorType>
    {
        commands.syscall(sys_handle.clone(), register_insertion_reactor::<C>);
        Some(ReactorType::ComponentInsertion(TypeId::of::<C>()))
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
    fn register(self, commands: &mut Commands, sys_handle: &AutoDespawnSignal) -> Option<ReactorType>
    {
        commands.syscall(sys_handle.clone(), register_mutation_reactor::<C>);
        Some(ReactorType::ComponentMutation(TypeId::of::<C>()))
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
    fn register(self, commands: &mut Commands, sys_handle: &AutoDespawnSignal) -> Option<ReactorType>
    {
        commands.syscall(sys_handle.clone(), register_removal_reactor::<C>);
        Some(ReactorType::ComponentRemoval(TypeId::of::<C>()))
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
    fn register(self, commands: &mut Commands, sys_handle: &AutoDespawnSignal) -> Option<ReactorType>
    {
        let comp_id = TypeId::of::<C>();
        let entity = self.0;
        let sys_handle = sys_handle.clone();

        commands.syscall((EntityReactionType::Insertion(comp_id), entity, sys_handle), register_entity_reactor);
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
    fn register(self, commands: &mut Commands, sys_handle: &AutoDespawnSignal) -> Option<ReactorType>
    {
        let comp_id = TypeId::of::<C>();
        let entity = self.0;
        let sys_handle = sys_handle.clone();

        commands.syscall((EntityReactionType::Mutation(comp_id), entity, sys_handle), register_entity_reactor);
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
///   [`reaction_tree()`] is executed, then the reactor will not be scheduled.
pub struct EntityRemovalTrigger<C: ReactComponent>(Entity, PhantomData<C>);

impl<C: ReactComponent> ReactionTrigger for EntityRemovalTrigger<C>
{
    fn register(self, commands: &mut Commands, sys_handle: &AutoDespawnSignal) -> Option<ReactorType>
    {
        let comp_id = TypeId::of::<C>();
        let entity = self.0;
        let sys_handle = sys_handle.clone();

        commands.syscall((), track_removals::<C>);
        commands.syscall((EntityReactionType::Removal(comp_id), entity, sys_handle), register_entity_reactor);
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
    fn register(self, commands: &mut Commands, sys_handle: &AutoDespawnSignal) -> Option<ReactorType>
    {
        commands.syscall(sys_handle.clone(), register_resource_mutation_reactor::<R>);
        Some(ReactorType::ResourceMutation(TypeId::of::<R>()))
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
    fn register(self, commands: &mut Commands, sys_handle: &AutoDespawnSignal) -> Option<ReactorType>
    {
        commands.syscall(sys_handle.clone(), register_broadcast_reactor::<E>);
        Some(ReactorType::Broadcast(TypeId::of::<E>()))
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
    fn register(self, commands: &mut Commands, sys_handle: &AutoDespawnSignal) -> Option<ReactorType>
    {
        commands.syscall((self.0, sys_handle.clone()), register_entity_event_reactor::<E>);
        Some(ReactorType::EntityEvent(self.0, TypeId::of::<E>()))
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
    fn register(self, commands: &mut Commands, sys_handle: &AutoDespawnSignal) -> Option<ReactorType>
    {
        // check if the entity exists
        let Some(_) = commands.get_entity(self.0) else { return None; };

        // add despawn tracker
        commands.syscall((self.0, sys_handle.clone()), register_despawn_reactor);
        Some(ReactorType::Despawn(self.0))
    }
}

/// Returns a [`DespawnTrigger`] reaction trigger.
pub fn despawn(entity: Entity) -> DespawnTrigger { DespawnTrigger(entity) }

//-------------------------------------------------------------------------------------------------------------------
