//local shortcuts
use crate::*;
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

fn add_despawn_tracker(
    In((entity, notifier)) : In<(Entity, Sender<Entity>)>,
    world                  : &mut World
){
    // try to get the entity
    // - if entity doesn't exist, then notify the reactor in case we have despawn reactors waiting
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

/// Add a reactor to an entity.
///
/// The reactor will be invoked when the trigger targets the entity.
fn register_entity_reactor<C: ReactComponent>(
    In((
        rtype,
        entity,
        sys_handle
    ))                  : In<(EntityReactType, Entity, AutoDespawnSignal)>,
    mut commands        : Commands,
    mut entity_reactors : Query<&mut EntityReactors>,
){
    // callback adder
    let add_callback_fn =
        move |entity_reactors: &mut EntityReactors|
        {
            let callbacks = match rtype
            {
                EntityReactType::Insertion => entity_reactors.insertion_callbacks.entry(TypeId::of::<C>()).or_default(),
                EntityReactType::Mutation  => entity_reactors.mutation_callbacks.entry(TypeId::of::<C>()).or_default(),
                EntityReactType::Removal   => entity_reactors.removal_callbacks.entry(TypeId::of::<C>()).or_default(),
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
pub struct Insertion<C: ReactComponent>(PhantomData<C>);
impl<C: ReactComponent> Default for Insertion<C> { fn default() -> Self { Self(PhantomData::default()) } }

impl<C: ReactComponent> ReactionTrigger<Entity> for Insertion<C>
{
    fn register(self, rcommands: &mut ReactCommands, sys_handle: &AutoDespawnSignal) -> ReactorType
    {
        rcommands.cache.register_insertion_reactor::<C>(sys_handle)
    }
}

/// Obtain a [`Insertion`] reaction trigger.
pub fn insertion<C: ReactComponent>() -> Insertion<C> { Insertion::default() }

//-------------------------------------------------------------------------------------------------------------------

/// Reaction trigger for [`ReactComponent`] mutations on any entity.
/// - For reactors that take the entity the component was mutated on.
pub struct Mutation<C: ReactComponent>(PhantomData<C>);
impl<C: ReactComponent> Default for Mutation<C> { fn default() -> Self { Self(PhantomData::default()) } }

impl<C: ReactComponent> ReactionTrigger<Entity> for Mutation<C>
{
    fn register(self, rcommands: &mut ReactCommands, sys_handle: &AutoDespawnSignal) -> ReactorType
    {
        rcommands.cache.register_mutation_reactor::<C>(sys_handle)
    }
}

/// Obtain a [`Mutation`] reaction trigger.
pub fn mutation<C: ReactComponent>() -> Mutation<C> { Mutation::default() }

//-------------------------------------------------------------------------------------------------------------------

/// Reaction trigger for [`ReactComponent`] removals from any entity.
/// - For reactors that take the entity the component was removed from.
/// - Reactions are not triggered if the entity was despawned.
pub struct Removal<C: ReactComponent>(PhantomData<C>);
impl<C: ReactComponent> Default for Removal<C> { fn default() -> Self { Self(PhantomData::default()) } }

impl<C: ReactComponent> ReactionTrigger<Entity> for Removal<C>
{
    fn register(self, rcommands: &mut ReactCommands, sys_handle: &AutoDespawnSignal) -> ReactorType
    {
        rcommands.cache.track_removals::<C>();
        rcommands.cache.register_removal_reactor::<C>(sys_handle)
    }
}

/// Obtain a [`Removal`] reaction trigger.
pub fn removal<C: ReactComponent>() -> Removal<C> { Removal::default() }

//-------------------------------------------------------------------------------------------------------------------

/// Reaction trigger for [`ReactComponent`] insertions on a specific entity.
/// - Registration does nothing if the entity does not exist.
pub struct EntityInsertion<C: ReactComponent>(Entity, PhantomData<C>);

impl<C: ReactComponent> ReactionTrigger<()> for EntityInsertion<C>
{
    fn register(self, rcommands: &mut ReactCommands, sys_handle: &AutoDespawnSignal) -> ReactorType
    {
        let entity = self.0;
        let sys_handle = sys_handle.clone();

        rcommands.commands.add(
                move |world: &mut World|
                syscall(world, (EntityReactType::Insertion, entity, sys_handle), register_entity_reactor::<C>)
            );

        ReactorType::EntityInsertion(entity, TypeId::of::<C>())
    }
}

/// Obtain a [`EntityInsertion`] reaction trigger.
pub fn entity_insertion<C: ReactComponent>(entity: Entity) -> EntityInsertion<C>
{
    EntityInsertion(entity, PhantomData::default())
}

//-------------------------------------------------------------------------------------------------------------------

/// Reaction trigger for [`ReactComponent`] mutations on a specific entity.
/// - Registration does nothing if the entity does not exist.
pub struct EntityMutation<C: ReactComponent>(Entity, PhantomData<C>);

impl<C: ReactComponent> ReactionTrigger<()> for EntityMutation<C>
{
    fn register(self, rcommands: &mut ReactCommands, sys_handle: &AutoDespawnSignal) -> ReactorType
    {
        let entity = self.0;
        let sys_handle = sys_handle.clone();

        rcommands.commands.add(
                move |world: &mut World|
                syscall(world, (EntityReactType::Mutation, entity, sys_handle), register_entity_reactor::<C>)
            );

        ReactorType::EntityMutation(entity, TypeId::of::<C>())
    }
}

/// Obtain a [`EntityMutation`] reaction trigger.
pub fn entity_mutation<C: ReactComponent>(entity: Entity) -> EntityMutation<C>
{
    EntityMutation(entity, PhantomData::default())
}

//-------------------------------------------------------------------------------------------------------------------

/// Reaction trigger for [`ReactComponent`] removals from a specific entity.
/// - Registration does nothing if the entity does not exist.
/// - If a component is removed from the entity then despawned (or removed due to a despawn) before
///   [`react_to_removals()`] is executed, then the reactor will not be scheduled.
pub struct EntityRemoval<C: ReactComponent>(Entity, PhantomData<C>);

impl<C: ReactComponent> ReactionTrigger<()> for EntityRemoval<C>
{
    fn register(self, rcommands: &mut ReactCommands, sys_handle: &AutoDespawnSignal) -> ReactorType
    {
        let entity = self.0;
        let sys_handle = sys_handle.clone();

        rcommands.cache.track_removals::<C>();

        rcommands.commands.add(
                move |world: &mut World|
                syscall(world, (EntityReactType::Removal, entity, sys_handle), register_entity_reactor::<C>)
            );

        ReactorType::EntityRemoval(entity, TypeId::of::<C>())
    }
}

/// Obtain a [`EntityRemoval`] reaction trigger.
pub fn entity_removal<C: ReactComponent>(entity: Entity) -> EntityRemoval<C>
{
    EntityRemoval(entity, PhantomData::default())
}

//-------------------------------------------------------------------------------------------------------------------

/// Reaction trigger for [`ReactResource`] mutations.
pub struct ResourceMutation<R: ReactResource>(PhantomData<R>);
impl<R: ReactResource> Default for ResourceMutation<R> { fn default() -> Self { Self(PhantomData::default()) } }

impl<R: ReactResource> ReactionTrigger<()> for ResourceMutation<R>
{
    fn register(self, rcommands: &mut ReactCommands, sys_handle: &AutoDespawnSignal) -> ReactorType
    {
        rcommands.cache.register_resource_mutation_reactor::<R>(sys_handle)
    }
}

/// Obtain a [`ResourceMutation`] reaction trigger.
pub fn resource_mutation<R: ReactResource>() -> ResourceMutation<R> { ResourceMutation::default() }

//-------------------------------------------------------------------------------------------------------------------

/// Reaction trigger for events.
/// - Reactions only occur for events sent via [`ReactCommands::<E>::send()`].
pub struct Event<E: Send + Sync + 'static>(PhantomData<E>);
impl<E: Send + Sync + 'static> Default for Event<E> { fn default() -> Self { Self(PhantomData::default()) } }

impl<E: Send + Sync + 'static> ReactionTrigger<()> for Event<E>
{
    fn register(self, rcommands: &mut ReactCommands, sys_handle: &AutoDespawnSignal) -> ReactorType
    {
        rcommands.cache.register_event_reactor::<E>(sys_handle)
    }
}

/// Obtain an [`Event`] reaction trigger.
pub fn event<E: Send + Sync + 'static>() -> Event<E> { Event::default() }

//-------------------------------------------------------------------------------------------------------------------

/// Reactor registration for entity despawns.
/// - Returns `Err` if the entity does not exist.
pub(crate) fn register_despawn_reactor<Marker>(
    rcommands : &mut ReactCommands,
    entity    : Entity,
    reactor   : impl IntoSystem<(), (), Marker> + Send + Sync + 'static
) -> Result<RevokeToken, ()>
{
    // if the entity doesn't exist, return a dummy revoke token
    let Some(_) = rcommands.commands.get_entity(entity) else { return Err(()); };

    // add despawn tracker
    let notifier = rcommands.cache.despawn_sender();
    rcommands.commands.add(move |world: &mut World| syscall(world, (entity, notifier), add_despawn_tracker));

    // register despawn reactor
    let token = rcommands.cache.register_despawn_reactor(
            entity,
            CallOnce::new(
                move |world|
                {
                    let mut system = IntoSystem::into_system(reactor);
                    system.initialize(world);
                    system.run((), world);
                    system.apply_deferred(world);
                }
            ),
        );
    
    Ok(token)
}

//-------------------------------------------------------------------------------------------------------------------
