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

fn register_insertion_reactor<C: ReactComponent>(In(handle): In<ReactorHandle>, mut cache: ResMut<ReactCache>)
{
    cache.register_insertion_reactor::<C>(handle);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn register_mutation_reactor<C: ReactComponent>(In(handle): In<ReactorHandle>, mut cache: ResMut<ReactCache>)
{
    cache.register_mutation_reactor::<C>(handle);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn register_removal_reactor<C: ReactComponent>(In(handle): In<ReactorHandle>, mut cache: ResMut<ReactCache>)
{
    cache.track_removals::<C>();
    cache.register_removal_reactor::<C>(handle);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn register_any_entity_event_reactor<E: 'static>(In(handle): In<ReactorHandle>, mut cache: ResMut<ReactCache>)
{
    cache.register_any_entity_event_reactor::<E>(handle);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn register_resource_mutation_reactor<R: ReactResource>(In(handle): In<ReactorHandle>, mut cache: ResMut<ReactCache>)
{
    cache.register_resource_mutation_reactor::<R>(handle);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn register_broadcast_reactor<E: Send + Sync + 'static>(In(handle): In<ReactorHandle>, mut cache: ResMut<ReactCache>)
{
    cache.register_broadcast_reactor::<E>(handle);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn register_despawn_reactor(
    In((entity, handle)) : In<(Entity, ReactorHandle)>,
    world                : &mut World,
){
    world.resource_scope(
        move |world, mut cache: Mut<ReactCache>|
        {
            // Check if the entity is still alive.
            let Ok(mut entity_mut) = world.get_entity_mut(entity) else { return; };

            // Register the reactor.
            cache.register_despawn_reactor(entity, handle);

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
        handle
    ))                  : In<(EntityReactionType, Entity, ReactorHandle)>,
    mut commands        : Commands,
    mut entity_reactors : Query<&mut EntityReactors>,
){
    // add callback to entity
    match entity_reactors.get_mut(entity)
    {
        Ok(mut entity_reactors) => entity_reactors.insert(rtype, handle),
        _ =>
        {
            let Ok(mut entity_commands) = commands.get_entity(entity) else { return; };

            // make new reactor tracker for the entity
            let mut entity_reactors = EntityReactors::default();

            // add callback and insert to entity
            entity_reactors.insert(rtype, handle);
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
impl<C: ReactComponent> Clone for InsertionTrigger<C> { fn clone(&self) -> Self { *self } }
impl<C: ReactComponent> Copy for InsertionTrigger<C> {}

impl<C: ReactComponent> ReactionTrigger for InsertionTrigger<C>
{
    fn reactor_type(&self) -> ReactorType
    {
        ReactorType::ComponentInsertion(TypeId::of::<C>())
    }

    fn register(&self, commands: &mut Commands, handle: &ReactorHandle)
    {
        commands.syscall(handle.clone(), register_insertion_reactor::<C>);
    }
}

/// Returns a [`InsertionTrigger`] reaction trigger.
pub fn insertion<C: ReactComponent>() -> InsertionTrigger<C> { InsertionTrigger::default() }

//-------------------------------------------------------------------------------------------------------------------

/// Reaction trigger for [`ReactComponent`] mutations on any entity.
/// - For reactors that take the entity the component was mutated on.
pub struct MutationTrigger<C: ReactComponent>(PhantomData<C>);
impl<C: ReactComponent> Default for MutationTrigger<C> { fn default() -> Self { Self(PhantomData::default()) } }
impl<C: ReactComponent> Clone for MutationTrigger<C> { fn clone(&self) -> Self { *self } }
impl<C: ReactComponent> Copy for MutationTrigger<C> {}

impl<C: ReactComponent> ReactionTrigger for MutationTrigger<C>
{
    fn reactor_type(&self) -> ReactorType
    {
        ReactorType::ComponentMutation(TypeId::of::<C>())
    }

    fn register(&self, commands: &mut Commands, handle: &ReactorHandle)
    {
        commands.syscall(handle.clone(), register_mutation_reactor::<C>);
    }
}

/// Returns a [`MutationTrigger`] reaction trigger.
pub fn mutation<C: ReactComponent>() -> MutationTrigger<C> { MutationTrigger::default() }

//-------------------------------------------------------------------------------------------------------------------

/// Reaction trigger for [`ReactComponent`] removals from any entity.
/// - Reactions are not triggered if the entity was despawned.
pub struct RemovalTrigger<C: ReactComponent>(PhantomData<C>);
impl<C: ReactComponent> Default for RemovalTrigger<C> { fn default() -> Self { Self(PhantomData::default()) } }
impl<C: ReactComponent> Clone for RemovalTrigger<C> { fn clone(&self) -> Self { *self } }
impl<C: ReactComponent> Copy for RemovalTrigger<C> {}

impl<C: ReactComponent> ReactionTrigger for RemovalTrigger<C>
{
    fn reactor_type(&self) -> ReactorType
    {
        ReactorType::ComponentRemoval(TypeId::of::<C>())
    }

    fn register(&self, commands: &mut Commands, handle: &ReactorHandle)
    {
        commands.syscall(handle.clone(), register_removal_reactor::<C>);
    }
}

/// Returns a [`RemovalTrigger`] reaction trigger.
pub fn removal<C: ReactComponent>() -> RemovalTrigger<C> { RemovalTrigger::default() }

//-------------------------------------------------------------------------------------------------------------------

/// Reaction trigger for [`ReactComponent`] insertions on a specific entity.
/// - Registration does nothing if the entity does not exist.
pub struct EntityInsertionTrigger<C: ReactComponent>(Entity, PhantomData<C>);
impl<C: ReactComponent> Clone for EntityInsertionTrigger<C> { fn clone(&self) -> Self { *self } }
impl<C: ReactComponent> Copy for EntityInsertionTrigger<C> {}

impl<C: ReactComponent> ReactionTrigger for EntityInsertionTrigger<C>
{
    fn reactor_type(&self) -> ReactorType
    {
        ReactorType::EntityInsertion(self.0, TypeId::of::<C>())
    }

    fn register(&self, commands: &mut Commands, handle: &ReactorHandle)
    {
        let handle = handle.clone();
        commands.syscall((EntityReactionType::Insertion(TypeId::of::<C>()), self.0, handle), register_entity_reactor);
    }
}

impl<C: ReactComponent> EntityTrigger for EntityInsertionTrigger<C>
{
    fn new_trigger(entity: Entity) -> Self
    {
        entity_insertion(entity)
    }

    fn entity(&self) -> Entity
    {
        self.0
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
impl<C: ReactComponent> Clone for EntityMutationTrigger<C> { fn clone(&self) -> Self { *self } }
impl<C: ReactComponent> Copy for EntityMutationTrigger<C> {}

impl<C: ReactComponent> ReactionTrigger for EntityMutationTrigger<C>
{
    fn reactor_type(&self) -> ReactorType
    {
        ReactorType::EntityMutation(self.0, TypeId::of::<C>())
    }

    fn register(&self, commands: &mut Commands, handle: &ReactorHandle)
    {
        let handle = handle.clone();
        commands.syscall((EntityReactionType::Mutation(TypeId::of::<C>()), self.0, handle), register_entity_reactor);
    }
}

impl<C: ReactComponent> EntityTrigger for EntityMutationTrigger<C>
{
    fn new_trigger(entity: Entity) -> Self
    {
        entity_mutation(entity)
    }

    fn entity(&self) -> Entity
    {
        self.0
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
pub struct EntityRemovalTrigger<C: ReactComponent>(Entity, PhantomData<C>);
impl<C: ReactComponent> Clone for EntityRemovalTrigger<C> { fn clone(&self) -> Self { *self } }
impl<C: ReactComponent> Copy for EntityRemovalTrigger<C> {}

impl<C: ReactComponent> ReactionTrigger for EntityRemovalTrigger<C>
{
    fn reactor_type(&self) -> ReactorType
    {
        ReactorType::EntityRemoval(self.0, TypeId::of::<C>())
    }

    fn register(&self, commands: &mut Commands, handle: &ReactorHandle)
    {
        let handle = handle.clone();
        commands.syscall((), track_removals::<C>);
        commands.syscall((EntityReactionType::Removal(TypeId::of::<C>()), self.0, handle), register_entity_reactor);
    }
}

impl<C: ReactComponent> EntityTrigger for EntityRemovalTrigger<C>
{
    fn new_trigger(entity: Entity) -> Self
    {
        entity_removal(entity)
    }

    fn entity(&self) -> Entity
    {
        self.0
    }
}

/// Returns a [`EntityRemovalTrigger`] reaction trigger.
pub fn entity_removal<C: ReactComponent>(entity: Entity) -> EntityRemovalTrigger<C>
{
    EntityRemovalTrigger(entity, PhantomData::default())
}

//-------------------------------------------------------------------------------------------------------------------

/// Reaction trigger for entity events.
/// - Reactions only occur for events sent via [`ReactCommands::<E>::entity_event()`].
pub struct EntityEventTrigger<E: Send + Sync + 'static>(Entity, PhantomData<E>);
impl<E: Send + Sync + 'static> Clone for EntityEventTrigger<E> { fn clone(&self) -> Self { *self } }
impl<E: Send + Sync + 'static> Copy for EntityEventTrigger<E> {}

impl<E: Send + Sync + 'static> ReactionTrigger for EntityEventTrigger<E>
{
    fn reactor_type(&self) -> ReactorType
    {
        ReactorType::EntityEvent(self.0, TypeId::of::<E>())
    }

    fn register(&self, commands: &mut Commands, handle: &ReactorHandle)
    {
        let handle = handle.clone();
        commands.syscall((EntityReactionType::Event(TypeId::of::<E>()), self.0, handle), register_entity_reactor);
    }
}

impl<E: Send + Sync + 'static> EntityTrigger for EntityEventTrigger<E>
{
    fn new_trigger(entity: Entity) -> Self
    {
        entity_event(entity)
    }

    fn entity(&self) -> Entity
    {
        self.0
    }
}

/// Returns an [`EntityEventTrigger`] reaction trigger.
pub fn entity_event<E: Send + Sync + 'static>(target: Entity) -> EntityEventTrigger<E>
{
    EntityEventTrigger(target, PhantomData::default())
}

//-------------------------------------------------------------------------------------------------------------------

/// Reaction trigger for any entity event of a given type.
/// - Reactions only occur for events sent via [`ReactCommands::<E>::entity_event()`].
pub struct AnyEntityEventTrigger<E: Send + Sync + 'static>(PhantomData<E>);
impl<E: Send + Sync + 'static> Clone for AnyEntityEventTrigger<E> { fn clone(&self) -> Self { *self } }
impl<E: Send + Sync + 'static> Copy for AnyEntityEventTrigger<E> {}

impl<E: Send + Sync + 'static> ReactionTrigger for AnyEntityEventTrigger<E>
{
    fn reactor_type(&self) -> ReactorType
    {
        ReactorType::AnyEntityEvent(TypeId::of::<E>())
    }

    fn register(&self, commands: &mut Commands, handle: &ReactorHandle)
    {
        commands.syscall(handle.clone(), register_any_entity_event_reactor::<E>);
    }
}

/// Returns an [`AnyEntityEventTrigger`] reaction trigger.
pub fn any_entity_event<E: Send + Sync + 'static>() -> AnyEntityEventTrigger<E>
{
    AnyEntityEventTrigger(PhantomData::default())
}

//-------------------------------------------------------------------------------------------------------------------

/// Reaction trigger for [`ReactResource`] mutations.
pub struct ResourceMutationTrigger<R: ReactResource>(PhantomData<R>);
impl<R: ReactResource> Default for ResourceMutationTrigger<R> { fn default() -> Self { Self(PhantomData::default()) } }
impl<R: ReactResource> Clone for ResourceMutationTrigger<R> { fn clone(&self) -> Self { *self } }
impl<R: ReactResource> Copy for ResourceMutationTrigger<R> {}

impl<R: ReactResource> ReactionTrigger for ResourceMutationTrigger<R>
{
    fn reactor_type(&self) -> ReactorType
    {
        ReactorType::ResourceMutation(TypeId::of::<R>())
    }

    fn register(&self, commands: &mut Commands, handle: &ReactorHandle)
    {
        commands.syscall(handle.clone(), register_resource_mutation_reactor::<R>);
    }
}

/// Returns a [`ResourceMutationTrigger`] reaction trigger.
pub fn resource_mutation<R: ReactResource>() -> ResourceMutationTrigger<R> { ResourceMutationTrigger::default() }

//-------------------------------------------------------------------------------------------------------------------

/// Reaction trigger for broadcast events.
/// - Reactions only occur for events sent via [`ReactCommands::<E>::broadcast()`].
pub struct BroadcastTrigger<E: Send + Sync + 'static>(PhantomData<E>);
impl<E: Send + Sync + 'static> Default for BroadcastTrigger<E> { fn default() -> Self { Self(PhantomData::default()) } }
impl<E: Send + Sync + 'static> Clone for BroadcastTrigger<E> { fn clone(&self) -> Self { *self } }
impl<E: Send + Sync + 'static> Copy for BroadcastTrigger<E> {}

impl<E: Send + Sync + 'static> ReactionTrigger for BroadcastTrigger<E>
{
    fn reactor_type(&self) -> ReactorType
    {
        ReactorType::Broadcast(TypeId::of::<E>())
    }

    fn register(&self, commands: &mut Commands, handle: &ReactorHandle)
    {
        commands.syscall(handle.clone(), register_broadcast_reactor::<E>);
    }
}

/// Returns a [`BroadcastTrigger`] reaction trigger.
pub fn broadcast<E: Send + Sync + 'static>() -> BroadcastTrigger<E> { BroadcastTrigger::default() }

//-------------------------------------------------------------------------------------------------------------------

/// Reaction trigger for despawns.
/// - Registration does nothing if the entity does not exist.
///
/// Note that `DespawnTrigger` does not implement [`EntityTrigger`] because [`EntityWorldReactor`] only runs for entities
/// that exist.
#[derive(Copy, Clone)]
pub struct DespawnTrigger(Entity);

impl ReactionTrigger for DespawnTrigger
{
    fn reactor_type(&self) -> ReactorType
    {
        ReactorType::Despawn(self.0)
    }

    fn register(&self, commands: &mut Commands, handle: &ReactorHandle)
    {
        // check if the entity exists
        let Ok(_) = commands.get_entity(self.0) else { return; };

        // add despawn tracker
        commands.syscall((self.0, handle.clone()), register_despawn_reactor);
    }
}

/// Returns a [`DespawnTrigger`] reaction trigger.
pub fn despawn(entity: Entity) -> DespawnTrigger { DespawnTrigger(entity) }

//-------------------------------------------------------------------------------------------------------------------
