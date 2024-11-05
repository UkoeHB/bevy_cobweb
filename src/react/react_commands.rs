//local shortcuts
use crate::prelude::*;

//third-party shortcuts
use bevy::prelude::*;

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn validate_rc(world: &mut World)
{
    if !(
        world.contains_resource::<ReactCache>() &&
        world.contains_resource::<AutoDespawner>()
    )
    {
        panic!("failed accessing ReactCommands, ReactPlugin is missing; you may need to reorder your \
            plugins so ReactPlugin is added sooner");
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn register_reactors<T: ReactionTriggerBundle>(
    In((triggers, syscommand, mode)): In<(T, SystemCommand, ReactorMode)>,
    mut commands: Commands,
    despawner: Res<AutoDespawner>,
){
    let handle = mode.prepare(&despawner, syscommand);
    triggers.register_triggers(&mut commands, &handle);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn revoke_entity_reactor(
    entity     : Entity,
    rtype      : EntityReactionType,
    reactor_id : SystemCommand,
    reactors   : &mut Query<&mut EntityReactors>,
){
    let Ok(mut entity_reactors) = reactors.get_mut(entity) else { return; };
    entity_reactors.remove(rtype, reactor_id);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn revoke_reactor(
    In(token)    : In<RevokeToken>,
    mut cache    : ResMut<ReactCache>,
    mut reactors : Query<&mut EntityReactors>,
){
    let id = token.id;

    for reactor_type in token.reactors.iter()
    {
        match *reactor_type
        {
            ReactorType::EntityInsertion(entity, comp_id) =>
            {
                revoke_entity_reactor(entity, EntityReactionType::Insertion(comp_id), id, &mut reactors);
            }
            ReactorType::EntityMutation(entity, comp_id) =>
            {
                revoke_entity_reactor(entity, EntityReactionType::Mutation(comp_id), id, &mut reactors);
            }
            ReactorType::EntityRemoval(entity, comp_id) =>
            {
                revoke_entity_reactor(entity, EntityReactionType::Removal(comp_id), id, &mut reactors);
            }
            ReactorType::EntityEvent(entity, event_id) =>
            {
                revoke_entity_reactor(entity, EntityReactionType::Event(event_id), id, &mut reactors);
            }
            ReactorType::AnyEntityEvent(event_id) =>
            {
                cache.revoke_any_entity_event_reactor(event_id, id);
            }
            ReactorType::ComponentInsertion(comp_id) =>
            {
                cache.revoke_component_reactor(EntityReactionType::Insertion(comp_id), id);
            }
            ReactorType::ComponentMutation(comp_id) =>
            {
                cache.revoke_component_reactor(EntityReactionType::Mutation(comp_id), id);
            }
            ReactorType::ComponentRemoval(comp_id) =>
            {
                cache.revoke_component_reactor(EntityReactionType::Removal(comp_id), id);
            }
            ReactorType::ResourceMutation(res_id) =>
            {
                cache.revoke_resource_mutation_reactor(res_id, id);
            }
            ReactorType::Broadcast(event_id) =>
            {
                cache.revoke_broadcast_reactor(event_id, id);
            }
            ReactorType::Despawn(entity) =>
            {
                cache.revoke_despawn_reactor(entity, id);
            }
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// Setting for controlling how reactors are cleaned up.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ReactorMode
{
    /// The reactor will live forever.
    ///
    /// Immortal reactors can be freely updated with
    /// new reaction triggers. Note that entity-specific triggers will automatically be removed when the referent entity is
    /// despawned, but all other triggers cannot be removed.
    ///
    /// This is the most efficient mode as it requires the minimum amount of allocations to register a reactor.
    Persistent,
    /// The reactor will be despawned when it has no triggers, including if all [`despawn()`] triggers have fired.
    Cleanup,
    /// The reactor will receive a [`RevokeToken`] that can be used to revoke it.
    ///
    /// The reactor will be automatically dropped when all [`despawn()`] triggers have fired if there are no other triggers.
    Revokable,
}

impl ReactorMode
{
    fn prepare(&self, despawner: &AutoDespawner, sys_command: SystemCommand) -> ReactorHandle
    {
        match self
        {
            Self::Persistent => ReactorHandle::Persistent(sys_command),
            Self::Cleanup    |
            Self::Revokable  => ReactorHandle::AutoDespawn(despawner.prepare(*sys_command)),
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Struct that drives reactivity.
///
/// Obtained via [`Commands::react`](ReactCommandsExt::react).
///
/// Note that each time you register a reactor, it is assigned a unique system state (including unique `Local`s). To avoid
/// leaking memory, be sure to revoke reactors when you are done with them.
///
/// All react command operations are deferred.
pub struct ReactCommands<'w, 's>
{
    pub(crate) commands: Commands<'w, 's>,
}

impl<'w, 's> ReactCommands<'w, 's>
{
    /// Returns a reborrow of the internal `Commands` with a shorter lifetime.
    pub fn commands(&mut self) -> Commands<'_, '_>
    {
        self.commands.reborrow()
    }

    /// Returns a reborrow of `self` with a shorter lifetime.
    pub fn reborrow(&mut self) -> ReactCommands<'_, '_>
    {
        ReactCommands{ commands: self.commands() }
    }

    /// Inserts a [`ReactComponent`] to the specified entity. It can be queried with [`React<C>`].
    /// - Does nothing if the entity does not exist.
    pub fn insert<C: ReactComponent>(&mut self, entity: Entity, component: C)
    {
        let Some(mut entity_commands) = self.commands.get_entity(entity) else { return; };
        entity_commands.try_insert( React{ entity, component } );
        self.commands.syscall_with_validation(entity, ReactCache::schedule_insertion_reaction::<C>, validate_rc);
    }

    /// Sends a broadcasted event.
    /// - Reactors can listen for the event with the [`broadcast()`] trigger.
    /// - Reactors can read the event with the [`BroadcastEvent`] system parameter.
    pub fn broadcast<E: Send + Sync + 'static>(&mut self, event: E)
    {
        self.commands.syscall_with_validation(event, ReactCache::schedule_broadcast_reaction::<E>, validate_rc);
    }

    /// Sends an entity-targeted event.
    /// - Reactors can listen for the event with the [`entity_event()`] trigger.
    /// - Reactors can read the event with the [`EntityEvent`] system parameter.
    pub fn entity_event<E: Send + Sync + 'static>(&mut self, entity: Entity, event: E)
    {
        self.commands.syscall_with_validation(
            (entity, event),
            ReactCache::schedule_entity_event_reaction::<E>,
            validate_rc
        );
    }

    /// Triggers resource mutation reactions.
    ///
    /// Useful for initializing state after a reactor is registered.
    pub fn trigger_resource_mutation<R: ReactResource + Send + Sync + 'static>(&mut self)
    {
        self.commands.syscall_with_validation((), ReactCache::schedule_resource_mutation_reaction::<R>, validate_rc);
    }

    /// Revokes a reactor.
    pub fn revoke(&mut self, token: RevokeToken)
    {
        self.commands.syscall_with_validation(token, revoke_reactor, validate_rc);
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
    /// Uses [`ReactorMode::Cleanup`].
    ///
    /// Example:
    /// ```no_run
    /// rcommands.on((resource_mutation::<MyRes>(), mutation::<MyComponent>()), my_reactor_system);
    /// ```
    pub fn on<M>(
        &mut self,
        triggers : impl ReactionTriggerBundle,
        reactor  : impl IntoSystem<(), (), M> + Send + Sync + 'static
    ){
        let sys_command = self.commands.spawn_system_command(reactor);
        let _ = self.with(triggers, sys_command, ReactorMode::Cleanup);
    }

    /// Registers a reactor triggered by ECS changes using [`ReactorMode::Persistent`].
    ///
    /// See [`Self::on`].
    pub fn on_persistent<M>(
        &mut self,
        triggers : impl ReactionTriggerBundle,
        reactor  : impl IntoSystem<(), (), M> + Send + Sync + 'static
    ) -> SystemCommand
    {
        let sys_command = self.commands.spawn_system_command(reactor);
        self.with(triggers, sys_command, ReactorMode::Persistent);
        sys_command
    }

    /// Registers a reactor triggered by ECS changes using [`ReactorMode::Revokable`].
    ///
    /// See [`Self::on`].
    pub fn on_revokable<M>(
        &mut self,
        triggers : impl ReactionTriggerBundle,
        reactor  : impl IntoSystem<(), (), M> + Send + Sync + 'static
    ) -> RevokeToken
    {
        let sys_command = self.commands.spawn_system_command(reactor);
        self.with(triggers, sys_command, ReactorMode::Revokable).unwrap()
    }

    /// Registers a reactor triggered by ECS changes with a [`SystemCommand`] and [`ReactorMode`].
    ///
    /// You can tie a reactor to multiple reaction triggers.
    /// Duplicate triggers will be ignored.
    ///
    /// Reactions are not merged together. If you register a reactor for triggers
    /// `(resource_mutation::<A>(), resource_mutation::<B>())`, then mutate `A` and `B` in succession, the reactor will
    /// execute twice.
    ///
    /// Note that you can call this method multiple times for the same [`SystemCommand`] to add triggers.
    /// It is highly recommended to use [`ReactorMode::Persistent`] in that case, otherwise your
    /// reactor may be despawned unexpectedly if a [`despawn()`] trigger is used or if you try to revoke any
    /// [`RevokeTokens`](RevokeToken) associated with the reactor.
    ///
    /// Returns `None` unless [`ReactorMode::Revokable`] is used.
    ///
    /// Example:
    /// ```no_run
    /// let command = commands.spawn_system_command(my_reactor_system);
    /// let mode = ReactorMode::Persistent;
    /// commands.react().with((resource_mutation::<MyRes>(), mutation::<MyComponent>()), command, mode);
    /// ```
    pub fn with(
        &mut self,
        triggers    : impl ReactionTriggerBundle,
        sys_command : SystemCommand,
        mode        : ReactorMode,
    ) -> Option<RevokeToken>
    {
        self.commands.syscall_with_validation((triggers, sys_command, mode), register_reactors, validate_rc);
        match mode
        {
            ReactorMode::Revokable => Some(RevokeToken::new_from(sys_command, triggers)),
            _ => None,
        }
    }

    /// Registers a one-off reactor triggered by ECS changes.
    ///
    /// Similar to [`Self::on_revokable`] except the reaction will run exactly once then get cleaned up.
    ///
    /// If an empty trigger bundle is used then the system will be dropped without running.
    ///
    /// Example:
    /// ```no_run
    /// // The reactor will run on the first mutation of either MyRes or MyComponent.
    /// rcommands.once((resource_mutation::<MyRes>(), mutation::<MyComponent>()), my_reactor_system);
    /// ```
    pub fn once<M, S: IntoSystem<(), (), M> + Send + Sync + 'static>(
        &mut self,
        triggers : impl ReactionTriggerBundle,
        reactor  : S
    ) -> RevokeToken
    {
        // register reactors
        let entity = self.commands.spawn_empty().id();
        let syscommand = SystemCommand(entity);
        let mode = ReactorMode::Revokable;
        let revoke_token = RevokeToken::new_from(syscommand, triggers);
        self.commands.syscall_with_validation((triggers, syscommand, mode), register_reactors, validate_rc);

        // wrap reactor in a system that will be called once, then clean itself up
        let revoke_token_clone = revoke_token.clone();
        let mut once_reactor = Some(move |world: &mut World, cleanup: SystemCommandCleanup|
        {
            let mut callback = RawCallbackSystem::new(reactor);
            callback.run_with_cleanup(world, (), move |w| cleanup.run(w));
            world.get_entity_mut(entity).ok().map(|e| e.despawn());
            world.react(|rc| rc.revoke(revoke_token_clone));
        });
        let once_system = move |world: &mut World, cleanup: SystemCommandCleanup|
        {
            if let Some(reactor) = once_reactor.take() { (reactor)(world, cleanup); };
        };
        self.commands.entity(entity).try_insert(SystemCommandStorage::new(SystemCommandCallback::with(once_system)));

        revoke_token
    }
}

//-------------------------------------------------------------------------------------------------------------------
