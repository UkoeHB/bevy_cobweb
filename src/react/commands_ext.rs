//local shortcuts
use crate::*;

//third-party shortcuts
use bevy::prelude::*;

//standard shortcuts
use std::hash::Hash;

//-------------------------------------------------------------------------------------------------------------------

/// Extends `Commands` with [`SystemCommand`] helpers.
pub trait CobwebCommandsExt
{
    /// Schedules a system command to be spawned.
    ///
    /// Returns the system command id that will eventually reference the spawned system.
    /// To run the system, schedule it with `commands.add(system_command)`.
    fn spawn_system_command(&mut self, callback: SystemCommandCallback) -> SystemCommand;

    /// Schedules a system command to be spawned from a given raw system.
    ///
    /// Systems are not initialized until they are first run.
    ///
    /// Returns the system command id that will eventually reference the spawned system.
    /// To run the system, schedule it with `commands.add(system_command)`.
    fn spawn_system_command_from<S, Marker>(&mut self, system: S) -> SystemCommand
    where
        S: IntoSystem<(), (), Marker> + Send + Sync + 'static;

    //todo: allow overwriting an existing command's callback

    /// Schedules a system event.
    ///
    /// If scheduled from user-land, this will cause a [`reaction_tree()`] to execute, otherwise it will be
    /// processed within the already-running reaction tree.
    fn system_event<T: Send + Sync + 'static>(&mut self, command: SystemCommand, event: T);
}

impl<'w, 's> CobwebCommandsExt for Commands<'w, 's>
{
    fn spawn_system_command(&mut self, callback: SystemCommandCallback) -> SystemCommand
    {
        SystemCommand::new(self.spawn(SystemCommandStorage::new(callback)).id())
    }

    fn spawn_system_command_from<S, Marker>(&mut self, system: S) -> SystemCommand
    where
        S: IntoSystem<(), (), Marker> + Send + Sync + 'static
    {
        let mut callback = CallbackSystem::new(system);
        let command = move |world: &mut World, cleanup: SystemCommandCleanup|
        {
            callback.run_with_cleanup(world, (), |world: &mut World| cleanup.run(world));
        };

        self.spawn_system_command(SystemCommandCallback::new(command))
    }

    fn system_event<T: Send + Sync + 'static>(&mut self, command: SystemCommand, event: T)
    {
        let data_entity = self.spawn(SystemEventData::new(event)).id();
        self.add(EventCommand{ system: *command, data_entity });
    }
}

//-------------------------------------------------------------------------------------------------------------------
