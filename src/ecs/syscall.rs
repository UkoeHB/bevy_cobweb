//local shortcuts

//third-party shortcuts
use bevy::ecs::system::{BoxedSystem, EntityCommands};
use bevy::prelude::*;

//standard shortcuts
use std::marker::PhantomData;

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

#[derive(Resource)]
struct InitializedSystem<I, O, S>
where
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
    S: Send + Sync + 'static
{
    sys      : BoxedSystem<I, O>,
    _phantom : PhantomData<S>
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// Executes a system on some data then applies the system's deferred commands.
///
/// The system will be cached for reuse. Subsequent calls to `syscall` with the same system will reuse the
/// original system's state. Using `syscall` on a closure that captures data is *not* recommended.
///
/// Use [`WorldSyscallExt::syscall_once`] if you only need to call a system once.
///
/// ## WARNING
/// If a system is called recursively, the Local system parameters of all but the outer-most invocation will not
/// persist.
///
/// ## Examples
///
/// ```
/// use bevy_cobweb::prelude::*;
/// use bevy::prelude::*;
/// 
/// // normal system: takes an input and sets a local
/// fn test_system(In(input): In<u16>, mut local: Local<u16>)
/// {
///     assert_eq!(input, *local);
///     *local += 1;
/// }
/// 
/// let mut world = World::new();
/// 
/// world.syscall(0u16, test_system);
/// world.syscall(1u16, test_system);  //Local is preserved
///
/// // function-like system: takes an input and returns an output
/// fn test_function(In(input): In<u16>) -> u16
/// {
///     input * 2
/// }
/// 
/// let mut world = World::new();
/// 
/// assert_eq!(world.syscall(1u16, test_function), 2u16);
/// ```
///
pub fn syscall<I, O, S, Marker>(world: &mut World, input: I, system: S) -> O
where
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
    S: IntoSystem<I, O, Marker> + Send + Sync + 'static,
{
    // get the initialized system
    let mut system =
        match world.remove_resource::<InitializedSystem<I, O, S>>()
        {
            Some(system) => system,
            None =>
            {
                let mut sys = IntoSystem::into_system(system);
                sys.initialize(world);
                InitializedSystem::<I, O, S>{ sys: Box::new(sys), _phantom: PhantomData::<S>{} }
            }
        };

    // run the system
    // - This automatically calls `apply_deferred`.
    let result = system.sys.run(input, world);

    // put the system back
    world.insert_resource(system);

    return result;
}

//-------------------------------------------------------------------------------------------------------------------

/// Wraps a `Fn` system in a system that consumes the system input.
///
/// This is intended to wrap `Fn` systems. Do not use it if you have a `FnOnce` callback, for example when
/// adding a one-off callback via `Command::add()`, because the input value and system will be unnecessarily cloned.
pub fn prep_fncall<I, O, Marker>(
    input  : I,
    system : impl IntoSystem<I, O, Marker> + Send + Sync + 'static + Clone
) -> impl Fn(&mut World) -> O + Send + Sync + 'static
where
    I: Send + Sync + 'static + Clone,
    O: Send + Sync + 'static,
{
    move |world: &mut World| syscall(world, input.clone(), system.clone())
}

//-------------------------------------------------------------------------------------------------------------------

/// Extends `World` with the [`syscall`] method.
pub trait WorldSyscallExt
{
    /// See [`syscall`].
    fn syscall<I, O, S, Marker>(&mut self, input: I, system: S) -> O
    where
        I: Send + Sync + 'static,
        O: Send + Sync + 'static,
        S: IntoSystem<I, O, Marker> + Send + Sync + 'static;

    /// Similar to [`syscall`] except the system is not cached for reuse.
    fn syscall_once<I, O, S, Marker>(&mut self, input: I, system: S) -> O
    where
        I: Send + Sync + 'static,
        O: Send + Sync + 'static,
        S: IntoSystem<I, O, Marker> + Send + Sync + 'static;
}

impl WorldSyscallExt for World
{
    fn syscall<I, O, S, Marker>(&mut self, input: I, system: S) -> O
    where
        I: Send + Sync + 'static,
        O: Send + Sync + 'static,
        S: IntoSystem<I, O, Marker> + Send + Sync + 'static
    {
        syscall(self, input, system)
    }

    fn syscall_once<I, O, S, Marker>(&mut self, input: I, system: S) -> O
    where
        I: Send + Sync + 'static,
        O: Send + Sync + 'static,
        S: IntoSystem<I, O, Marker> + Send + Sync + 'static
    {
        let mut sys = IntoSystem::into_system(system);
        sys.initialize(self);
        sys.run(input, self)
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Extends `Commands` with the [`syscall`] method.
pub trait CommandsSyscallExt
{
    /// See [`syscall`].
    fn syscall<I, S, Marker>(&mut self, input: I, system: S)
    where
        I: Send + Sync + 'static,
        S: IntoSystem<I, (), Marker> + Send + Sync + 'static;

    /// Similar to [`syscall`] except the system is not cached for reuse.
    fn syscall_once<I, S, Marker>(&mut self, input: I, system: S)
    where
        I: Send + Sync + 'static,
        S: IntoSystem<I, (), Marker> + Send + Sync + 'static;
}

impl CommandsSyscallExt for Commands<'_, '_>
{
    fn syscall<I, S, Marker>(&mut self, input: I, system: S)
    where
        I: Send + Sync + 'static,
        S: IntoSystem<I, (), Marker> + Send + Sync + 'static
    {
        self.add(move |world: &mut World| { world.syscall(input, system); });
    }

    fn syscall_once<I, S, Marker>(&mut self, input: I, system: S)
    where
        I: Send + Sync + 'static,
        S: IntoSystem<I, (), Marker> + Send + Sync + 'static
    {
        self.add(move |world: &mut World| { world.syscall_once(input, system); });
    }
}

impl CommandsSyscallExt for EntityCommands<'_>
{
    fn syscall<I, S, Marker>(&mut self, input: I, system: S)
    where
        I: Send + Sync + 'static,
        S: IntoSystem<I, (), Marker> + Send + Sync + 'static
    {
        self.commands().syscall(input, system);
    }

    fn syscall_once<I, S, Marker>(&mut self, input: I, system: S)
    where
        I: Send + Sync + 'static,
        S: IntoSystem<I, (), Marker> + Send + Sync + 'static
    {
        self.commands().syscall_once(input, system);
    }
}

//-------------------------------------------------------------------------------------------------------------------
