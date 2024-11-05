//local shortcuts
use crate::prelude::*;

//third-party shortcuts
use bevy::prelude::*;

//standard shortcuts
use std::hash::Hash;

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// Stores a callable system.
///
/// We store the system in an option in order to avoid archetype moves when taking/reinserting the system in order to
/// call it.
#[derive(Component)]
struct SpawnedSystem<I, O>
where
    I: Send + Sync + SystemInput + 'static,
    O: Send + Sync + 'static,
{
    system: Option<CallbackSystem<I, O>>,
}

impl<I, O> SpawnedSystem<I, O>
where
    I: Send + Sync + SystemInput + 'static,
    O: Send + Sync + 'static,
{
    fn new(system: CallbackSystem<I,O>) -> Self
    {
        Self{ system: Some(system) }
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// System identifier for referencing spawned systems.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SysId(Entity);

impl SysId
{
    pub fn new(entity: Entity) -> Self { Self(entity) }

    pub fn entity(&self) -> Entity
    {
        self.0
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Spawn a system as an entity.
///
/// Systems are not initialized until they are first run.
///
/// The system can be invoked by calling [`spawned_syscall()`].
pub fn spawn_system<I, O, S, Marker>(world: &mut World, system: S) -> SysId
where
    I: Send + Sync + SystemInput + 'static,
    O: Send + Sync + 'static,
    S: IntoSystem<I, O, Marker> + Send + Sync + 'static,
{
    spawn_system_from(world, CallbackSystem::new(system))
}

/// Spawn a system as an entity.
///
/// The system can be invoked by calling [`spawned_syscall()`].
pub fn spawn_system_from<I, O>(world: &mut World, system: CallbackSystem<I, O>) -> SysId
where
    I: Send + Sync + SystemInput + 'static,
    O: Send + Sync + 'static,
{
    SysId::new(world.spawn(SpawnedSystem::new(system)).id())
}

//-------------------------------------------------------------------------------------------------------------------

/// Spawn a ref-counted system.
///
/// Systems are not initialized until they are first run.
///
/// Returns a cleanup handle. The system will be dropped when the last copy of the handle is dropped.
///
/// Panics if [`setup_auto_despawn()`](AutoDespawnAppExt::setup_auto_despawn) was not added to your app.
pub fn spawn_rc_system<I, O, S, Marker>(world: &mut World, system: S) -> AutoDespawnSignal
where
    I: Send + Sync + SystemInput + 'static,
    O: Send + Sync + 'static,
    S: IntoSystem<I, O, Marker> + Send + Sync + 'static,
{
    spawn_rc_system_from(world, CallbackSystem::new(system))
}

/// Spawn a ref-counted system.
///
/// Returns a cleanup handle. The system will be dropped when the last copy of the handle is dropped.
///
/// Panics if [`setup_auto_despawn()`](AutoDespawnAppExt::setup_auto_despawn) was not added to your app.
pub fn spawn_rc_system_from<I, O>(world: &mut World, system: CallbackSystem<I, O>) -> AutoDespawnSignal
where
    I: Send + Sync + SystemInput + 'static,
    O: Send + Sync + 'static,
{
    let sys_id = spawn_system_from(world, system);
    world.resource::<AutoDespawner>().prepare(sys_id.0)
}

//-------------------------------------------------------------------------------------------------------------------

/// Execute a pre-spawned system on some data then apply the system's deferred commands.
///
/// Returns `Err` if the system does not exist or if the system was called recursively.
///
/// # Example
///
/// ```
/// use bevy_cobweb::prelude::*;
/// use bevy::prelude::*;
///
/// fn test_system(In(input): In<u16>, mut local: Local<u16>) -> u16
/// {
///     *local += input;
///     *local
/// }
///
/// let mut world = World::new();
/// let sys_id1 = spawn_system(test_system);
/// let sys_id2 = spawn_system(test_system);
///
/// assert_eq!(spawned_syscall(&mut world, sys_id1, 1u16), 1);
/// assert_eq!(spawned_syscall(&mut world, sys_id1, 1u16), 2);    //Local is preserved
/// assert_eq!(spawned_syscall(&mut world, sys_id2, 10u16), 10);  //new Local
/// assert_eq!(spawned_syscall(&mut world, sys_id2, 10u16), 20);
/// ```
///
pub fn spawned_syscall<I, O>(world: &mut World, sys_id: SysId, input: <I as SystemInput>::Inner<'_>) -> Result<O, ()>
where
    I: Send + Sync + SystemInput + 'static, <I as SystemInput>::Inner<'static>: Send,
    O: Send + Sync + 'static,
{
    // extract the callback
    let Ok(mut entity_mut) = world.get_entity_mut(sys_id.0) else { return Err(()); };
    let Some(mut spawned_system) = entity_mut.get_mut::<SpawnedSystem<I, O>>()
    else { tracing::error!(?sys_id, "spawned system component is missing"); return Err(()); };
    let Some(mut callback) = spawned_system.system.take()
    else { tracing::warn!(?sys_id, "recursive spawned system call detected"); return Err(()); };

    // invoke the callback
    let result = callback.run(world, input).ok_or(())?;

    // reinsert the callback if its target hasn't been despawned
    let Ok(mut entity_mut) = world.get_entity_mut(sys_id.0) else { return Ok(result); };
    let Some(mut spawned_system) = entity_mut.get_mut::<SpawnedSystem<I, O>>()
    else { tracing::error!(?sys_id, "spawned system component is missing"); return Ok(result); };
    spawned_system.system = Some(callback);

    Ok(result)
}

//-------------------------------------------------------------------------------------------------------------------

pub trait SpawnedSyscallCommandsExt
{
    /// Schedule a system to be spawned.
    ///
    /// Systems are not initialized until they are first run.
    ///
    /// Returns the system id that will eventually reference the spawned system. It can be used to invoke the system with
    /// [`spawned_syscall()`] or [`SpawnedSyscallCommandsExt::spawned_syscall()`].
    fn spawn_system<I, O, S, Marker>(&mut self, system: S) -> SysId
    where
        I: Send + Sync + SystemInput + 'static,
        O: Send + Sync + 'static,
        S: IntoSystem<I, O, Marker> + Send + Sync + 'static;

    /// Schedule a system to be spawned.
    ///
    /// Returns the system id that will eventually reference the spawned system. It can be used to invoke the system with
    /// [`spawned_syscall()`] or [`SpawnedSyscallCommandsExt::spawned_syscall()`].
    fn spawn_system_from<I, O>(&mut self, system: CallbackSystem<I, O>) -> SysId
    where
        I: Send + Sync + SystemInput + 'static,
        O: Send + Sync + 'static;

    /// Schedule a system to be inserted into the specified entity.
    ///
    /// This is useful for constructing self-referential systems, e.g. for systems that only run once then clean themselves
    /// up.
    ///
    /// Returns an error if the entity does not exist.
    fn insert_system<I, O, S, Marker>(&mut self, entity: Entity, system: S) -> Result<(), ()>
    where
        I: Send + Sync + SystemInput + 'static,
        O: Send + Sync + 'static,
        S: IntoSystem<I, O, Marker> + Send + Sync + 'static;

    /// Schedule a spawned system call.
    ///
    /// It is the responsibility of the caller to correctly match the system entity with the target system signature.
    ///
    /// Logs a warning if the system entity doesn't exist.
    ///
    /// Syntax sugar for [`spawned_syscall()`].
    fn spawned_syscall<I>(&mut self, sys_id: SysId, input: <I as bevy::prelude::SystemInput>::Inner<'_>)
    where
        I: Send + Sync + SystemInput + 'static, <I as SystemInput>::Inner<'static>: Send;
}

impl<'w, 's> SpawnedSyscallCommandsExt for Commands<'w, 's>
{
    fn spawn_system<I, O, S, Marker>(&mut self, system: S) -> SysId
    where
        I: Send + Sync + SystemInput + 'static,
        O: Send + Sync + 'static,
        S: IntoSystem<I, O, Marker> + Send + Sync + 'static
    {
        self.spawn_system_from(CallbackSystem::new(system))
    }

    fn spawn_system_from<I, O>(&mut self, system: CallbackSystem<I, O>) -> SysId
    where
        I: Send + Sync + SystemInput + 'static,
        O: Send + Sync + 'static
    {
        SysId::new(self.spawn(SpawnedSystem::new(system)).id())
    }

    fn insert_system<I, O, S, Marker>(&mut self, entity: Entity, system: S) -> Result<(), ()>
    where
        I: Send + Sync + SystemInput + 'static,
        O: Send + Sync + 'static,
        S: IntoSystem<I, O, Marker> + Send + Sync + 'static
    {
        let Some(mut entity) = self.get_entity(entity) else { return Err(()); };
        entity.insert(SpawnedSystem::new(CallbackSystem::new(system)));

        Ok(())
    }

    fn spawned_syscall<I>(&mut self, sys_id: SysId, input: <I as SystemInput>::Inner<'_>)
    where
        I: Send + Sync + SystemInput + 'static, <I as SystemInput>::Inner<'static>: Send
    {
        self.queue(
                move |world: &mut World|
                {
                    if let Err(_) = spawned_syscall::<I, ()>(world, sys_id, input.into())
                    {
                        tracing::warn!(?sys_id, "spawned syscall failed");
                    }
                }

            );
    }
}

//-------------------------------------------------------------------------------------------------------------------
