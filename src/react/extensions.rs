//local shortcuts
use crate::prelude::*;

//third-party shortcuts
use bevy::prelude::*;
use bevy::ecs::system::EntityCommands;
use bevy::ecs::world::Command;

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------

/// Extends `App` with reactivity helpers.
pub trait ReactAppExt
{
    /// Adds a [`WorldReactor`] to the app with *only* starting triggers.
    ///
    /// Equivalent to:
    /*
    ```rust
    app.react(|rc| rc.on_persistent(triggers, reactor));
    ```
    */
    fn add_reactor<M, R: ReactorResult>(
        &mut self,
        triggers: impl ReactionTriggerBundle,
        reactor: impl IntoSystem<(), R, M> + Send + Sync + 'static
    ) -> &mut Self;
    /// Adds a [`WorldReactor`] to the app.
    ///
    /// The reactor can be accessed with the [`Reactor`] system param.
    fn add_world_reactor<R>(&mut self, reactor: R) -> &mut Self
    where
        R: WorldReactor<StartingTriggers = ()>;
    /// Adds a [`WorldReactor`] to the app with starting triggers.
    ///
    /// The reactor be accessed with the [`Reactor`] system param.
    fn add_world_reactor_with<R: WorldReactor>(&mut self, reactor: R, triggers: R::StartingTriggers) -> &mut Self;
    /// Adds an [`EntityWorldReactor`] to the app.
    ///
    /// The reactor can be accessed with the [`EntityReactor`] system param.
    fn add_entity_reactor<R: EntityWorldReactor>(&mut self, reactor: R) -> &mut Self;
    /// Provides access to [`ReactCommands`].
    fn react<T>(&mut self, callback: impl FnOnce(&mut ReactCommands) -> T) -> &mut Self;
}

impl ReactAppExt for App
{
    fn add_reactor<M, R: ReactorResult>(
        &mut self,
        triggers: impl ReactionTriggerBundle,
        reactor: impl IntoSystem<(), R, M> + Send + Sync + 'static
    ) -> &mut Self
    {
        // Make sure app is ready to use ReactCommands.
        if !self.world().contains_resource::<ReactCache>()
        {
            self.init_resource::<ReactCache>();
        }
        self.setup_auto_despawn();

        // Add reactor.
        self.react(|rc| rc.on_persistent(triggers, reactor))
    }

    fn add_world_reactor<R>(&mut self, reactor: R) -> &mut Self
    where
        R: WorldReactor<StartingTriggers = ()>
    {
        if self.world().contains_resource::<WorldReactorRes<R>>()
        {
            panic!("duplicate world reactors of type {:?} are not allowed", std::any::type_name::<R>());
        }
        let sys_command = self.world_mut().spawn_system_command_from(reactor.reactor());
        self.world_mut().insert_resource(WorldReactorRes::<R>::new(sys_command));
        self
    }

    fn add_world_reactor_with<R: WorldReactor>(&mut self, reactor: R, triggers: R::StartingTriggers) -> &mut Self
    {
        if self.world().contains_resource::<WorldReactorRes<R>>()
        {
            panic!("duplicate world reactors of type {:?} are not allowed", std::any::type_name::<R>());
        }
        let sys_command = self.world_mut().spawn_system_command_from(reactor.reactor());
        self.world_mut().insert_resource(WorldReactorRes::<R>::new(sys_command));

        // Make sure app is ready to use ReactCommands.
        if !self.world().contains_resource::<ReactCache>()
        {
            self.init_resource::<ReactCache>();
        }
        self.setup_auto_despawn();

        // Add starting triggers.
        self.world_mut().syscall_once((),
            move |mut c: Commands, reactor: Reactor<R>|
            {
                reactor.add_starting_triggers(&mut c, triggers);
            }
        );
        self
    }

    fn add_entity_reactor<R: EntityWorldReactor>(&mut self, reactor: R) -> &mut Self
    {
        if self.world().contains_resource::<EntityWorldReactorRes<R>>()
        {
            panic!("duplicate entity world reactors of type {:?} are not allowed", std::any::type_name::<R>());
        }
        let sys_command = self.world_mut().spawn_system_command_from(reactor.reactor());
        self.world_mut().insert_resource(EntityWorldReactorRes::<R>::new(sys_command));
        self
    }

    fn react<T>(&mut self, callback: impl FnOnce(&mut ReactCommands) -> T) -> &mut Self
    {
        // Ignore returned value.
        let _ = self.world_mut().react(callback);
        self
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Extends `World` with reactivity helpers.
pub trait ReactWorldExt
{
    /// Schedules a [`SystemCommand`] to be spawned.
    ///
    /// Systems are not initialized until they are first run.
    ///
    /// Returns the system command id that will eventually reference the spawned system.
    /// To run the system, schedule it with `commands.queue(system_command)`.
    fn spawn_system_command<S, R: ReactorResult, M>(&mut self, system: S) -> SystemCommand
    where
        S: IntoSystem<(), R, M> + Send + Sync + 'static;

    /// Schedules a [`SystemCommand`] to be spawned from a pre-defined callback.
    ///
    /// Returns the system command id that will eventually reference the spawned system.
    /// To run the system, schedule it with `commands.queue(system_command)`.
    fn spawn_system_command_from(&mut self, callback: SystemCommandCallback) -> SystemCommand;

    /// Provides access to [`ReactCommands`].
    fn react<T>(&mut self, callback: impl FnOnce(&mut ReactCommands) -> T) -> T;

    /// Schedules a system event targeting a given [`SystemCommand`].
    ///
    /// The target system can consume the event with the [`SystemEvent`] system parameter.
    ///
    /// If scheduled from user-land, this will cause a [`reaction_tree()`] to execute, otherwise it will be
    /// processed within the already-running reaction tree.
    fn send_system_event<T: Send + Sync + 'static>(&mut self, command: SystemCommand, event: T);

    /// Sends a broadcasted event.
    /// - Reactors can listen for the event with the [`broadcast()`] trigger.
    /// - Reactors can read the event with the [`BroadcastEvent`] system parameter.
    fn broadcast<E: Send + Sync + 'static>(&mut self, event: E);

    /// Sends an entity-targeted event.
    /// - Reactors can listen for the event with the [`entity_event()`] trigger.
    /// - Reactors can read the event with the [`EntityEvent`] system parameter.
    fn entity_event<E: Send + Sync + 'static>(&mut self, entity: Entity, event: E);
}

impl ReactWorldExt for World
{
    fn spawn_system_command<S, R: ReactorResult, M>(&mut self, system: S) -> SystemCommand
    where
        S: IntoSystem<(), R, M> + Send + Sync + 'static
    {
        self.spawn_system_command_from(SystemCommandCallback::new(system))
    }

    fn spawn_system_command_from(&mut self, callback: SystemCommandCallback) -> SystemCommand
    {
        SystemCommand(self.spawn(SystemCommandStorage::new(callback)).id())
    }

    fn react<T>(&mut self, callback: impl FnOnce(&mut ReactCommands) -> T) -> T
    {
        let mut c = self.commands();
        let mut rc = c.react();
        let result = (callback)(&mut rc);
        self.flush();
        result
    }

    fn send_system_event<T: Send + Sync + 'static>(&mut self, command: SystemCommand, event: T)
    {
        let data_entity = self.spawn(SystemEventData::new(event)).id();
        EventCommand{ system: command, data_entity }.apply(self);
    }

    fn broadcast<E: Send + Sync + 'static>(&mut self, event: E)
    {
        self.syscall(event, ReactCache::schedule_broadcast_reaction::<E>);
    }

    fn entity_event<E: Send + Sync + 'static>(&mut self, entity: Entity, event: E)
    {
        self.syscall((entity, event), ReactCache::schedule_entity_event_reaction::<E>);
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Extends `Commands` with reactivity helpers.
pub trait ReactCommandsExt
{
    /// Obtains a [`ReactCommands`] instance.
    fn react(&mut self) -> ReactCommands<'_, '_>;

    /// Schedules a [`SystemCommand`] to be spawned.
    ///
    /// Systems are not initialized until they are first run.
    ///
    /// Returns the system command id that will eventually reference the spawned system.
    /// To run the system, schedule it with `commands.queue(system_command)`.
    fn spawn_system_command<S, R: ReactorResult, M>(&mut self, system: S) -> SystemCommand
    where
        S: IntoSystem<(), R, M> + Send + Sync + 'static;

    /// Schedules a [`SystemCommand`] to be spawned from a pre-defined callback.
    ///
    /// Returns the system command id that will eventually reference the spawned system.
    /// To run the system, schedule it with `commands.queue(system_command)`.
    fn spawn_system_command_from(&mut self, callback: SystemCommandCallback) -> SystemCommand;

    /// Schedules a system event targeting a given [`SystemCommand`].
    ///
    /// The target system can consume the event with the [`SystemEvent`] system parameter.
    ///
    /// If scheduled from user-land, this will cause a [`reaction_tree()`] to execute, otherwise it will be
    /// processed within the already-running reaction tree.
    fn send_system_event<T: Send + Sync + 'static>(&mut self, command: SystemCommand, event: T);
}

impl<'w, 's> ReactCommandsExt for Commands<'w, 's>
{
    fn react(&mut self) -> ReactCommands<'_, '_>
    {
        ReactCommands{ commands: self.reborrow() }
    }

    fn spawn_system_command<S, R: ReactorResult, M>(&mut self, system: S) -> SystemCommand
    where
        S: IntoSystem<(), R, M> + Send + Sync + 'static
    {
        self.spawn_system_command_from(SystemCommandCallback::new(system))
    }

    fn spawn_system_command_from(&mut self, callback: SystemCommandCallback) -> SystemCommand
    {
        SystemCommand(self.spawn(SystemCommandStorage::new(callback)).id())
    }

    fn send_system_event<T: Send + Sync + 'static>(&mut self, command: SystemCommand, event: T)
    {
        let data_entity = self.spawn(SystemEventData::new(event)).id();
        self.queue(EventCommand{ system: command, data_entity });
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Extends `EntityCommands` with reactivity helpers.
pub trait ReactEntityCommandsExt
{
    /// Obtains a [`ReactCommands`] instance.
    fn react(&mut self) -> ReactCommands<'_, '_>;

    /// Registers the current entity with an [`EntityWorldReactor`].
    fn add_world_reactor<T: EntityWorldReactor>(&mut self, data: T::Local);
}

impl<'a> ReactEntityCommandsExt for EntityCommands<'a>
{
    fn react(&mut self) -> ReactCommands<'_, '_>
    {
        ReactCommands{ commands: self.commands() }
    }

    fn add_world_reactor<T: EntityWorldReactor>(&mut self, data: T::Local)
    {
        let id = self.id();
        self.commands().syscall((id, data),
            |In((id, data)): In<(Entity, T::Local)>, mut c: Commands, reactor: EntityReactor<T>|
            {
                reactor.add(&mut c, id, data);
            }
        );
    }
}

//-------------------------------------------------------------------------------------------------------------------
