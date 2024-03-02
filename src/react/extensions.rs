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
}

impl ReactAppExt for App
{
    fn add_reactor<R>(&mut self, reactor: R) -> &mut Self
    where
        R: WorldReactor<StartingTriggers = ()>
    {
        let sys_command = self.world.spawn_system_command_from(reactor.reactor());
        self.world.insert_resource(WorldReactorRes::<R>::new(sys_command));
        self
    }

    fn add_reactor_with<R: WorldReactor>(&mut self, reactor: R, triggers: R::StartingTriggers) -> &mut Self
    {
        let sys_command = self.world.spawn_system_command_from(reactor.reactor());
        self.world.insert_resource(WorldReactorRes::<R>::new(sys_command));

        // Make sure app is ready to use ReactCommands.
        if !self.world.contains_resource::<ReactCache>()
        {
            self.init_resource::<ReactCache>();
        }
        self.setup_auto_despawn();

        // Add starting triggers.
        self.world.syscall((),
            move |mut rc: ReactCommands, mut reactor: Reactor<R>|
            {
                reactor.add_starting_triggers(&mut rc, triggers);
            }
        );
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
}

//-------------------------------------------------------------------------------------------------------------------

/// Extends `Commands` with reactivity helpers.
pub trait ReactCommandsExt
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
}

impl<'w, 's> ReactCommandsExt for Commands<'w, 's>
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
        self.add(EventCommand{ system: command, data_entity });
    }
}

//-------------------------------------------------------------------------------------------------------------------
