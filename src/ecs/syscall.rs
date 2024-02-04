//local shortcuts

//third-party shortcuts
use bevy::ecs::system::BoxedSystem;
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

/// Execute a system on some data then apply the system's deferred commands.
///
/// # WARNING
/// If a system is called recursively, the Local system parameters of all but the outer-most invocation will not
/// persist.
///
/// # Examples
///
/// ```
/// use bevy_kot::ecs::*;
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
/// syscall(&mut world, 0u16, test_system);
/// syscall(&mut world, 1u16, test_system);  //Local is preserved
///
/// // function-like system: takes an input and returns an output
/// fn test_function(In(input): In<u16>) -> u16
/// {
///     input * 2
/// }
/// 
/// let mut world = World::new();
/// 
/// assert_eq!(syscall(&mut world, 1u16, test_function), 2u16);
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
    let result = system.sys.run(input, world);

    // apply any pending changes
    system.sys.apply_deferred(world);

    // put the system back
    world.insert_resource(system);

    return result;
}

//-------------------------------------------------------------------------------------------------------------------

/// Wrap a `Fn` system in a system that consumes the system input.
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
