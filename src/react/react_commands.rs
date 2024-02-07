//local shortcuts
use crate::*;

//third-party shortcuts
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

//standard shortcuts
use core::any::TypeId;

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn revoke_reactor_triggers(In(revoke_token): In<RevokeToken>, mut rcommands: ReactCommands)
{
    rcommands.revoke(revoke_token);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// Revoke an entity reactor.
fn revoke_entity_reactor(
    In((
        entity,
        rtype,
        comp_id,
        reactor_id
    ))                  : In<(Entity, EntityReactType, TypeId, u64)>,
    mut commands        : Commands,
    mut entity_reactors : Query<&mut EntityReactors>,
){
    // get this entity's entity reactors
    let Ok(mut entity_reactors) = entity_reactors.get_mut(entity) else { return; };

    // get cached callbacks
    let callbacks_map = match rtype
    {
        EntityReactType::Insertion => &mut entity_reactors.insertion_callbacks,
        EntityReactType::Mutation  => &mut entity_reactors.mutation_callbacks,
        EntityReactType::Removal   => &mut entity_reactors.removal_callbacks,
    };
    let Some(callbacks) = callbacks_map.get_mut(&comp_id) else { return; };

    // revoke reactor
    for (idx, signal) in callbacks.iter().enumerate()
    {
        if signal.entity().to_bits() != reactor_id { continue; }
        let _ = callbacks.remove(idx);
        break;
    }

    // clean up if entity has no reactors
    if !(callbacks.len() == 0) { return; }
    let _ = callbacks_map.remove(&comp_id);

    if !entity_reactors.is_empty() { return; }
    commands.get_entity(entity).unwrap().remove::<EntityReactors>();
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// Drives reactivity.
///
/// Requires [`ReactPlugin`].
///
/// Note that each time you register a reactor, it is assigned a unique system state (including unique `Local`s). To avoid
/// leaking memory, be sure to revoke reactors when you are done with them.
#[derive(SystemParam)]
pub struct ReactCommands<'w, 's>
{
    pub(crate) commands  : Commands<'w, 's>,
    pub(crate) cache     : ResMut<'w, ReactCache>,
    pub(crate) despawner : Res<'w, AutoDespawner>,
}

impl<'w, 's> ReactCommands<'w, 's>
{
    /// Access [`Commands`].
    pub fn commands<'a>(&'a mut self) -> &'a mut Commands<'w, 's>
    {
        &mut self.commands
    }

    /// Insert a [`ReactComponent`] to the specified entity. It can be queried with [`React<C>`].
    /// - Reactions are enacted after `apply_deferred` is invoked.
    /// - Does nothing if the entity does not exist.
    //todo: consider more ergonomic entity access, e.g. ReactEntityCommands
    pub fn insert<C: ReactComponent>(&mut self, entity: Entity, component: C)
    {
        let Some(mut entity_commands) = self.commands.get_entity(entity) else { return; };
        entity_commands.insert( React{ entity, component } );
        self.cache.react_to_insertion::<C>(&mut self.commands, entity);
    }

    /// Send an event.
    /// - The event is sent and reactions are enacted after `apply_deferred` is invoked.
    /// - Reactors can access the event with the bevy [`ReactEvent<E>`] system parameter.
    pub fn send<E: Send + Sync + 'static>(&mut self, event: E)
    {
        self.commands().add(
                move |world: &mut World|
                {
                    let mut counter = world.resource_mut::<ReactEventCounter>();
                    let event_id = counter.increment();
                    world.send_event(ReactEventInner{ event_id, event });
                }
            );
        self.cache.react_to_event::<E>(&mut self.commands);
    }

    /// Trigger resource mutation reactions.
    ///
    /// Useful for initializing state after a reactor is registered.
    pub fn trigger_resource_mutation<R: ReactResource + Send + Sync + 'static>(&mut self)
    {
        self.cache.react_to_resource_mutation::<R>(&mut self.commands);
    }

    /// Revoke a reactor.
    /// - Entity reactors: revoked after `apply_deferred` is invoked.
    /// - Component, despawn, resource, event reactors: revoked immediately.
    pub fn revoke(&mut self, token: RevokeToken)
    {
        let id = token.id;

        for reactor_type in token.reactors.iter()
        {
            match *reactor_type
            {
                ReactorType::EntityInsertion(entity, comp_id) =>
                {
                    self.commands.add(
                            move |world: &mut World|
                            syscall(world, (entity, EntityReactType::Insertion, comp_id, id), revoke_entity_reactor)
                        );
                }
                ReactorType::EntityMutation(entity, comp_id) =>
                {
                    self.commands.add(
                            move |world: &mut World|
                            syscall(world, (entity, EntityReactType::Mutation, comp_id, id), revoke_entity_reactor)
                        );
                }
                ReactorType::EntityRemoval(entity, comp_id) =>
                {
                    self.commands.add(
                            move |world: &mut World|
                            syscall(world, (entity, EntityReactType::Removal, comp_id, id), revoke_entity_reactor)
                        );
                }
                ReactorType::ComponentInsertion(comp_id) =>
                {
                    self.cache.revoke_component_reactor(EntityReactType::Insertion, comp_id, id);
                }
                ReactorType::ComponentMutation(comp_id) =>
                {
                    self.cache.revoke_component_reactor(EntityReactType::Mutation, comp_id, id);
                }
                ReactorType::ComponentRemoval(comp_id) =>
                {
                    self.cache.revoke_component_reactor(EntityReactType::Removal, comp_id, id);
                }
                ReactorType::ResourceMutation(res_id) =>
                {
                    self.cache.revoke_resource_mutation_reactor(res_id, id);
                }
                ReactorType::Event(event_id) =>
                {
                    self.cache.revoke_event_reactor(event_id, id);
                }
                ReactorType::Despawn(entity) =>
                {
                    self.cache.revoke_despawn_reactor(entity, id);
                }
            }
        }
    }

    /// Register a reactor triggered by ECS changes.
    ///
    /// You can tie a reactor to multiple reaction triggers. Note that the
    /// entity-agnostic component triggers can only be bundled with each other: `insertion()`, `mutation()`,
    /// `removal()`.
    ///
    /// Duplicate triggers will be ignored.
    ///
    /// Reactions are not merged together. If you register a reactor for triggers
    /// `(resource_mutation::<A>(), resource_mutation::<B>())`, then mutate `A` and `B` in succession, the reactor will
    /// execute twice.
    ///
    /// Example:
    /// ```no_run
    /// rcommands.on((resource_mutation::<MyRes>(), component_mutation::<MyComponent>()), my_reactor_system);
    /// ```
    pub fn on<Marker>(
        &mut self,
        triggers : impl ReactionTriggerBundle<I>,
        reactor  : impl IntoSystem<(), (), Marker> + Send + Sync + 'static
    ) -> RevokeToken
    where
        I: Send + Sync + 'static
    {
        let sys_id = self.commands.spawn_system(reactor);
        let sys_handle = self.despawner.prepare(sys_id.entity());

        self.with_syscommand()
    }

    pub fn with_syscommand<Marker>(
        &mut self,
        triggers   : impl ReactionTriggerBundle<I>,
        reactor    : SystemCommand,
        sys_handle : &AutoDespawnSignal,
    ) -> RevokeToken
    where
        I: Send + Sync + 'static
    {
        let sys_id = self.commands.spawn_system(reactor);

        reactor_registration(self, &sys_handle, triggers)
    }

    /// Register a one-off reactor triggered by ECS changes.
    ///
    /// Similar to [`Self::on`] except the reaction will run exactly once then get cleaned up.
    ///
    /// Example:
    /// ```no_run
    /// // The reactor will run on the first mutation of either MyRes or MyComponent.
    /// rcommands.once((resource_mutation::<MyRes>(), component_mutation::<MyComponent>()), my_reactor_system);
    /// ```
    pub fn once<I, Marker>(
        &mut self,
        triggers : impl ReactionTriggerBundle<I>,
        reactor  : impl IntoSystem<I, (), Marker> + Send + Sync + 'static
    ) -> RevokeToken
    where
        I: Send + Sync + 'static
    {
        // register reactors
        let entity = self.commands.spawn_empty().id();
        let sys_handle = self.despawner.prepare(entity);
        let revoke_token = reactor_registration(self, &sys_handle, triggers);

        // wrap reactor in a system that will be called once, then clean itself up
        let revoke_token_clone = revoke_token.clone();
        let mut once_reactor = Some(move |world: &mut World, input: I|
        {
            let mut system = IntoSystem::into_system(reactor);
            system.initialize(world);
            system.run(input, world);
            system.apply_deferred(world);
            world.despawn(entity);
            syscall(world, revoke_token_clone, revoke_reactor_triggers);
        });
        let once_system = move |In(input): In<I>, world: &mut World|
        {
            if let Some(reactor) = once_reactor.take() { (reactor)(world, input); };
        };
        self.commands.insert_system(entity, once_system).unwrap();

        revoke_token
    }
}

//-------------------------------------------------------------------------------------------------------------------
