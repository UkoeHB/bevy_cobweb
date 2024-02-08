//local shortcuts
use crate::prelude::*;

//third-party shortcuts
use bevy::prelude::*;

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------

/// Extends `Commands` with [`SystemCommand`] helpers.
pub trait ReactCommandsExt
{
    /// Schedules a [`SystemCommand`] to be spawned.
    ///
    /// Systems are not initialized until they are first run.
    ///
    /// Returns the system command id that will eventually reference the spawned system.
    /// To run the system, schedule it with `commands.add(system_command)`.
    fn spawn_system_command<S, Marker>(&mut self, system: S) -> SystemCommand
    where
        S: IntoSystem<(), (), Marker> + Send + Sync + 'static;

    /// Schedules a [`SystemCommand`] to be spawned from a pre-defined callback.
    ///
    /// Returns the system command id that will eventually reference the spawned system.
    /// To run the system, schedule it with `commands.add(system_command)`.
    fn spawn_system_command_from(&mut self, callback: SystemCommandCallback) -> SystemCommand;

    //todo: allow overwriting an existing command's callback

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
    fn spawn_system_command<S, Marker>(&mut self, system: S) -> SystemCommand
    where
        S: IntoSystem<(), (), Marker> + Send + Sync + 'static
    {
        let mut callback = CallbackSystem::new(system);
        let command = move |world: &mut World, cleanup: SystemCommandCleanup|
        {
            callback.run_with_cleanup(world, (), move |world: &mut World| cleanup.run(world));
        };

        self.spawn_system_command_from(SystemCommandCallback::new(command))
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
