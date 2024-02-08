//local shortcuts
use crate::*;

//third-party shortcuts
use bevy::prelude::*;

//standard shortcuts
use std::hash::Hash;

//-------------------------------------------------------------------------------------------------------------------

/// Records a cleanup callback that can be injected into system commands for cleanup after the system command
/// runs but before its `apply_deferred` is called.
///
/// For efficiency, only function pointer callbacks are supported.
#[derive(Debug, Default, Copy, Clone)]
pub(crate) struct SystemCommandCleanup
{
    cleanup: Option<fn(&mut World)>,
}

impl SystemCommandCleanup
{
    /// Makes a new system cleanup.
    pub(crate) fn new(cleanup: fn(&mut World)) -> Self
    {
        Self{ cleanup: Some(cleanup) }
    }

    /// Runs the system cleanup on the world.
    ///
    /// Does nothing if no callback is stored.
    pub(crate) fn run(self, world: &mut World)
    {
        let Some(cleanup) = self.cleanup else { return; };
        (cleanup)(world);
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Owns a system command callback.
///
/// The callback should own the actual system that you want to run. The [`SystemCommandCleanup`] callback must be invoked
/// between running your system and calling `apply_deferred` on that system.
//todo: wrap the callback in a trait that lets you reassign the injected callback if it is the same type
pub(crate) struct SystemCommandCallback
{
    inner: Box<dyn FnMut(&mut World, SystemCommandCleanup) + Send + Sync 'static>,
}

impl SystemCommandCallback
{
    /// Makes a new system command callback.
    pub(crate) fn new(callback: impl FnMut(&mut World, SystemCommandCleanup) + Send + Sync 'static) -> Self
    {
        Self{ inner: Box::new(callback) }
    }

    /// Runs the system command callback.
    ///
    /// The `cleanup` should be invoked between running the callback's inner system and
    /// calling `apply_deferred` on the inner system.
    pub(crate) fn run(&mut self, world: &mut World, cleanup: SystemCommandCleanup)
    {
        (self.inner)(world, cleanup);
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Stores a system command's callback.
///
/// We store the callback in an option in order to avoid archetype moves when taking/reinserting the callback in order to
/// call it.
#[derive(Component)]
pub(crate) struct SystemCommandStorage
{
    callback: Option<SystemCommandCallback>,
}

impl SystemCommandStorage
{
    pub(crate) fn new(callback: SystemCommandCallback) -> Self
    {
        Self{ callback: Some(callback) }
    }

    pub(crate) fn insert(&mut self, callback: SystemCommandCallback)
    {
        self.callback = Some(callback);
    }

    pub(crate) fn take(&mut self) -> Option<SystemCommandCallback>
    {
        self.callback.take()
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Spawns a system command as an entity.
///
/// The system can be invoked by calling [`syscommand_runner()`].
pub fn spawn_system_command(world: &mut World, callback: SystemCommandCallback) -> SystemCommand
{
    SystemCommand::new(SysId::new(world.spawn(SystemCommandStorage::new(callback)).id()))
}

//-------------------------------------------------------------------------------------------------------------------

/// Spawns a system as a system command from a given raw system.
///
/// Systems are not initialized until they are first run.
///
/// The system can be invoked by calling [`syscommand_runner()`].
pub fn spawn_system_command_from<S, Marker>(world: &mut World, system: S) -> SystemCommand
where
    S: IntoSystem<(), (), Marker> + Send + Sync + 'static,
{
    let mut callback = CallbackSystem::new(system);
    let command = move |world: &mut World, cleanup: SystemCommandCleanup|
    {
        callback.run_with_cleanup(world, (), |world: &mut World| cleanup.run(world));
    };

    spawn_system_command(world, SystemCommandCallback::new(command))
}

//todo: allow overwriting an existing command's callback

//-------------------------------------------------------------------------------------------------------------------

/// Spawns a ref-counted system command.
///
/// Returns a cleanup handle. The system will be dropped when the last copy of the handle is dropped.
///
/// Panics if [`setup_auto_despawn()`] was not added to your app.
pub fn spawn_rc_system_command(world: &mut World, callback: SystemCommandCallback) -> AutoDespawnSignal
{
    let system_command = spawn_system_command(world, callback);
    world.resource::<AutoDespawner>().prepare(system_command.entity())
}

//-------------------------------------------------------------------------------------------------------------------

/// Spawns a ref-counted system command from a given raw system.
///
/// Systems are not initialized until they are first run.
///
/// Returns a cleanup handle. The system will be dropped when the last copy of the handle is dropped.
///
/// Panics if [`setup_auto_despawn()`] was not added to your app.
pub fn spawn_rc_system_command_from<S, Marker>(world: &mut World, system: S) -> AutoDespawnSignal
where
    S: IntoSystem<(), (), Marker> + Send + Sync + 'static,
{
    let system_command = spawn_system_command_from(world, system);
    world.resource::<AutoDespawner>().prepare(system_command.entity())
}

//-------------------------------------------------------------------------------------------------------------------
