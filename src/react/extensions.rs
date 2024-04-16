//local shortcuts
use crate::prelude::*;

//third-party shortcuts
use bevy::prelude::*;
use bevy::ecs::system::Command;

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------

/// Extends `App` with reactivity helpers.
pub trait ReactAppExt
{
    /// Adds a [`WorldReactor`] to the app.
    ///
    /// The reactor can be accessed with the [`Reactor`] system param.
    fn add_reactor<R>(&mut self, reactor: R) -> &mut Self
    where
        R: WorldReactor<StartingTriggers = ()>;

    /// Adds a [`WorldReactor`] to the app with starting triggers.
    ///
    /// The reactor be accessed with the [`Reactor`] system param.
    fn add_reactor_with<R: WorldReactor>(&mut self, reactor: R, triggers: R::StartingTriggers) -> &mut Self;

    /// Adds an [`EntityWorldReactor`] to the app.
    ///
    /// The reactor can be accessed with the [`EntityReactor`] system param.
    fn add_entity_reactor<R: EntityWorldReactor>(&mut self, reactor: R) -> &mut Self;
}

impl ReactAppExt for App
{
    fn add_reactor<R>(&mut self, reactor: R) -> &mut Self
    where
        R: WorldReactor<StartingTriggers = ()>
    {
        if self.world.contains_resource::<WorldReactorRes<R>>()
        {
            panic!("duplicate world reactors of type {:?} are not allowed", std::any::type_name::<R>());
        }
        let sys_command = self.world.spawn_system_command_from(reactor.reactor());
        self.world.insert_resource(WorldReactorRes::<R>::new(sys_command));
        self
    }

    fn add_reactor_with<R: WorldReactor>(&mut self, reactor: R, triggers: R::StartingTriggers) -> &mut Self
    {
        if self.world.contains_resource::<WorldReactorRes<R>>()
        {
            panic!("duplicate world reactors of type {:?} are not allowed", std::any::type_name::<R>());
        }
        let sys_command = self.world.spawn_system_command_from(reactor.reactor());
        self.world.insert_resource(WorldReactorRes::<R>::new(sys_command));

        // Make sure app is ready to use ReactCommands.
        if !self.world.contains_resource::<ReactCache>()
        {
            self.init_resource::<ReactCache>();
        }
        self.setup_auto_despawn();

        // Add starting triggers.
        CallbackSystem::new(
            move |mut c: Commands, reactor: Reactor<R>|
            {
                reactor.add_starting_triggers(&mut c, triggers);
            }
        ).run(&mut self.world, ());
        self
    }

    fn add_entity_reactor<R: EntityWorldReactor>(&mut self, reactor: R) -> &mut Self
    {
        if self.world.contains_resource::<EntityWorldReactorRes<R>>()
        {
            panic!("duplicate entity world reactors of type {:?} are not allowed", std::any::type_name::<R>());
        }
        let sys_command = self.world.spawn_system_command_from(reactor.reactor());
        self.world.insert_resource(EntityWorldReactorRes::<R>::new(sys_command));
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
    /// To run the system, schedule it with `commands.add(system_command)`.
    fn spawn_system_command<S, M>(&mut self, system: S) -> SystemCommand
    where
        S: IntoSystem<(), (), M> + Send + Sync + 'static;

    /// Schedules a [`SystemCommand`] to be spawned from a pre-defined callback.
    ///
    /// Returns the system command id that will eventually reference the spawned system.
    /// To run the system, schedule it with `commands.add(system_command)`.
    fn spawn_system_command_from(&mut self, callback: SystemCommandCallback) -> SystemCommand;

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
    fn spawn_system_command<S, M>(&mut self, system: S) -> SystemCommand
    where
        S: IntoSystem<(), (), M> + Send + Sync + 'static
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
    /// To run the system, schedule it with `commands.add(system_command)`.
    fn spawn_system_command<S, M>(&mut self, system: S) -> SystemCommand
    where
        S: IntoSystem<(), (), M> + Send + Sync + 'static;

    /// Schedules a [`SystemCommand`] to be spawned from a pre-defined callback.
    ///
    /// Returns the system command id that will eventually reference the spawned system.
    /// To run the system, schedule it with `commands.add(system_command)`.
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

    fn spawn_system_command<S, M>(&mut self, system: S) -> SystemCommand
    where
        S: IntoSystem<(), (), M> + Send + Sync + 'static
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
        self.add(EventCommand{ system: command, data_entity });
    }
}

//-------------------------------------------------------------------------------------------------------------------
