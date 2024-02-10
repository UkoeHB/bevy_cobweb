//local shortcuts
use crate::prelude::*;

//third-party shortcuts
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn revoke_reactor_triggers(In(revoke_token): In<RevokeToken>, mut rcommands: ReactCommands)
{
    rcommands.revoke(revoke_token);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn revoke_entity_reactor(
    In((
        entity,
        rtype,
        reactor_id
    ))                  : In<(Entity, EntityReactionType, u64)>,
    mut commands        : Commands,
    mut entity_reactors : Query<&mut EntityReactors>,
){
    // get this entity's entity reactors
    let Ok(mut entity_reactors) = entity_reactors.get_mut(entity) else { return; };

    // get cached callbacks
    let (comp_id, callbacks_map) = match rtype
    {
        EntityReactionType::Insertion(comp_id) => (comp_id, &mut entity_reactors.insertion_callbacks),
        EntityReactionType::Mutation(comp_id)  => (comp_id, &mut entity_reactors.mutation_callbacks),
        EntityReactionType::Removal(comp_id)   => (comp_id, &mut entity_reactors.removal_callbacks),
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

/// System paramter that drives reactivity.
///
/// Requires [`ReactPlugin`].
///
/// Note that each time you register a reactor, it is assigned a unique system state (including unique `Local`s). To avoid
/// leaking memory, be sure to revoke reactors when you are done with them.
#[derive(SystemParam)]
pub struct ReactCommands<'w, 's>
{
    pub(crate) commands    : Commands<'w, 's>,
    pub(crate) react_queue : ResMut<'w, CobwebCommandQueue<ReactionCommand>>,
    pub(crate) cache       : ResMut<'w, ReactCache>,
    pub(crate) despawner   : Res<'w, AutoDespawner>,
}

impl<'w, 's> ReactCommands<'w, 's>
{
    /// Returns a mutable reference to `Commands`.
    pub fn commands<'a>(&'a mut self) -> &'a mut Commands<'w, 's>
    {
        &mut self.commands
    }

    /// Inserts a [`ReactComponent`] to the specified entity. It can be queried with [`React<C>`].
    /// - Insertion reactions are enacted after `apply_deferred` is invoked.
    /// - Does nothing if the entity does not exist.
    pub fn insert<C: ReactComponent>(&mut self, entity: Entity, component: C)
    {
        let Some(mut entity_commands) = self.commands.get_entity(entity) else { return; };
        entity_commands.try_insert( React{ entity, component } );
        self.cache.schedule_insertion_reaction::<C>(&mut self.commands, &mut self.react_queue, entity);
    }

    /// Sends a broadcasted event.
    /// - Reactions are enacted after `apply_deferred` is invoked.
    /// - Reactors can listen for the event with the [`event()`] trigger.
    /// - Reactors can read the event with the [`BroadcastEvent`] system parameter.
    pub fn broadcast<E: Send + Sync + 'static>(&mut self, event: E)
    {
        self.cache.schedule_event_reaction::<E>(&mut self.commands, &mut self.react_queue, event);
    }

    /// Sends an entity-targeted event.
    /// - Reactions are enacted after `apply_deferred` is invoked.
    /// - Reactors can listen for the event with the [`entity_event()`] trigger.
    /// - Reactors can read the event with the [`EntityEvent`] system parameter.
    pub fn entity_event<E: Send + Sync + 'static>(&mut self, entity: Entity, event: E)
    {
        self.cache.schedule_entity_event_reaction::<E>(&mut self.commands, &mut self.react_queue, entity, event);
    }

    /// Triggers resource mutation reactions.
    ///
    /// Useful for initializing state after a reactor is registered.
    pub fn trigger_resource_mutation<R: ReactResource + Send + Sync + 'static>(&mut self)
    {
        self.cache.schedule_resource_mutation_reaction::<R>(&mut self.commands, &mut self.react_queue);
    }

    /// Revokes a reactor.
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
                            syscall(world, (entity, EntityReactionType::Insertion(comp_id), id), revoke_entity_reactor)
                        );
                }
                ReactorType::EntityMutation(entity, comp_id) =>
                {
                    self.commands.add(
                            move |world: &mut World|
                            syscall(world, (entity, EntityReactionType::Mutation(comp_id), id), revoke_entity_reactor)
                        );
                }
                ReactorType::EntityRemoval(entity, comp_id) =>
                {
                    self.commands.add(
                            move |world: &mut World|
                            syscall(world, (entity, EntityReactionType::Removal(comp_id), id), revoke_entity_reactor)
                        );
                }
                ReactorType::ComponentInsertion(comp_id) =>
                {
                    self.cache.revoke_component_reactor(EntityReactionType::Insertion(comp_id), id);
                }
                ReactorType::ComponentMutation(comp_id) =>
                {
                    self.cache.revoke_component_reactor(EntityReactionType::Mutation(comp_id), id);
                }
                ReactorType::ComponentRemoval(comp_id) =>
                {
                    self.cache.revoke_component_reactor(EntityReactionType::Removal(comp_id), id);
                }
                ReactorType::ResourceMutation(res_id) =>
                {
                    self.cache.revoke_resource_mutation_reactor(res_id, id);
                }
                ReactorType::Broadcast(event_id) =>
                {
                    self.cache.revoke_broadcast_reactor(event_id, id);
                }
                ReactorType::EntityEvent(entity, event_id) =>
                {
                    self.cache.revoke_entity_event_reactor(entity, event_id, id);
                }
                ReactorType::Despawn(entity) =>
                {
                    self.cache.revoke_despawn_reactor(entity, id);
                }
            }
        }
    }

    /// Registers a reactor triggered by ECS changes.
    ///
    /// You can tie a reactor to multiple reaction triggers.
    /// Duplicate triggers will be ignored.
    ///
    /// Reactions are not merged together. If you register a reactor for triggers
    /// `(resource_mutation::<A>(), resource_mutation::<B>())`, then mutate `A` and `B` in succession, the reactor will
    /// execute twice.
    ///
    /// Example:
    /// ```no_run
    /// rcommands.on((resource_mutation::<MyRes>(), mutation::<MyComponent>()), my_reactor_system);
    /// ```
    pub fn on<M>(
        &mut self,
        triggers : impl ReactionTriggerBundle,
        reactor  : impl IntoSystem<(), (), M> + Send + Sync + 'static
    ) -> RevokeToken
    {
        let sys_id = self.commands.spawn_system_command(reactor);
        self.with(triggers, sys_id)
    }

    /// Registers a reactor triggered by ECS changes with a pre-defined [`SystemCommand`].
    ///
    /// You can tie a reactor to multiple reaction triggers.
    /// Duplicate triggers will be ignored.
    ///
    /// Reactions are not merged together. If you register a reactor for triggers
    /// `(resource_mutation::<A>(), resource_mutation::<B>())`, then mutate `A` and `B` in succession, the reactor will
    /// execute twice.
    ///
    /// Note that you can call this method multiple times for the same [`SystemCommand`] to increase the number
    /// of triggers.
    /// Revoking any of the associated revoke tokens will *despawn* the system command, and cause a *memory leak*
    /// because the trigger entries for the other tokens won't be cleaned up (unless you also revoke those tokens).
    ///
    /// Example:
    /// ```no_run
    /// let command = rcommands.commands().spawn_system_command(my_reactor_system);
    /// rcommands.with((resource_mutation::<MyRes>(), mutation::<MyComponent>()), command);
    /// ```
    pub fn with(
        &mut self,
        triggers : impl ReactionTriggerBundle,
        sys_id   : SystemCommand,
    ) -> RevokeToken
    {
        let sys_handle = self.despawner.prepare(*sys_id);
        reactor_registration(self, &sys_handle, triggers)
    }

    /// Registers a one-off reactor triggered by ECS changes.
    ///
    /// Similar to [`Self::on`] except the reaction will run exactly once then get cleaned up.
    ///
    /// If an empty trigger bundle is used then the system will be dropped without running.
    ///
    /// Example:
    /// ```no_run
    /// // The reactor will run on the first mutation of either MyRes or MyComponent.
    /// rcommands.once((resource_mutation::<MyRes>(), mutation::<MyComponent>()), my_reactor_system);
    /// ```
    pub fn once<M>(
        &mut self,
        triggers : impl ReactionTriggerBundle,
        reactor  : impl IntoSystem<(), (), M> + Send + Sync + 'static
    ) -> RevokeToken
    {
        // register reactors
        let entity = self.commands.spawn_empty().id();
        let sys_handle = self.despawner.prepare(entity);
        let revoke_token = reactor_registration(self, &sys_handle, triggers);

        // wrap reactor in a system that will be called once, then clean itself up
        let revoke_token_clone = revoke_token.clone();
        let mut once_reactor = Some(move |world: &mut World, cleanup: SystemCommandCleanup|
        {
            let mut system = IntoSystem::into_system(reactor);
            system.initialize(world);
            system.run((), world);
            cleanup.run(world);
            system.apply_deferred(world);
            world.despawn(entity);
            syscall(world, revoke_token_clone, revoke_reactor_triggers);
        });
        let once_system = move |world: &mut World, cleanup: SystemCommandCleanup|
        {
            if let Some(reactor) = once_reactor.take() { (reactor)(world, cleanup); };
        };
        self.commands.entity(entity).try_insert(SystemCommandStorage::new(SystemCommandCallback::new(once_system)));

        revoke_token
    }
}

//-------------------------------------------------------------------------------------------------------------------
