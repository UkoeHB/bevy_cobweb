//local shortcuts
use crate::*;

//third-party shortcuts
use bevy::ecs::system::{SystemParam, SystemState, BoxedSystem};
use bevy::prelude::*;
use bevy::utils::{AHasher, HashMap};
use fxhash::FxHasher32;

//standard shortcuts
use std::any::TypeId;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;


//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// A system caller may have multiple instances. We need to ensure the local state of these instances is
/// not shared. This hashmap allows us to dynamically store instance states.
#[derive(Default, Resource)]
struct StateInstances<T: SystemParam + 'static>
{
    instances: HashMap<CallId, SystemState<T>>,
}

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

/// Stores a callable system.
///
/// We store the system in an option in order to avoid archetype moves when taking/reinserting the system in order to
/// call it.
#[derive(Component)]
struct SpawnedSystem<I, O>
where
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    system: Option<CallbackSystem<I, O>>,
}

impl<I, O> SpawnedSystem<I, O>
where
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    fn new(system: CallbackSystem<I,O>) -> Self
    {
        Self{ system: Some(system) }
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn init_callable_system<S: SystemParam + 'static>(world: &mut World, id: CallId)
{
    // 1. obtain the callable system's existing state instances or make a new one
    let mut state_instances =
        match world.remove_resource::<StateInstances<S>>()
        {
            Some(s) => s,
            None    =>
            {
                // Note, this message should only appear once! If you see it twice in the logs, the function
                // may have been called recursively, and will panic.
                debug!("Init system state {}", std::any::type_name::<S>());
                StateInstances::<S>{instances: HashMap::new()}
            }
        };

    // 2. make sure our callable system has an instance for this call id
    if !state_instances.instances.contains_key(&id)
    {
        debug!("Registering system state for system caller {id:?} of type {}", std::any::type_name::<S>());
        state_instances.instances.insert(id, SystemState::new(world));
    }

    // 3. add the state instances to the world
    world.insert_resource(state_instances);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// An identifier for [CallableSystem]s. Each identifier represents a unique system context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CallId(pub u64);

impl CallId
{
    /// Makes a new caller id.
    pub fn new(name: &str) -> Self
    {
        let bytes = name.as_bytes();
        let mut hasher = FxHasher32::default();
        hasher.write(bytes);
        CallId(hasher.finish())
    }

    /// Makes a caller id by extending an existing caller id.
    pub fn with(&self, name: &str) -> CallId
    {
        Self::new(&format!("{}{name}", self.0))
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Implemented types can be called like normal functions.
/// - Accepts one function argument.
pub trait CallableSystem: SystemParam
{
    /// Parameter type that allows custom data to be passed from caller to the callable system.
    type ArgT;

    /// Implementation of the callable system.
    fn system(world: &mut World, state: &mut SystemState<Self>, id: CallId, arg: Self::ArgT);
}

/// Implemented types can be called like normal functions.
/// - accepts no function arguments
pub trait BasicCallableSystem: SystemParam
{
    /// Implementation of the callable system for callables with no arguments.
    fn system(world: &mut World, state: &mut SystemState<Self>, id: CallId);
}

//-------------------------------------------------------------------------------------------------------------------

/// Call a callable system (one function argument).
///
/// # Examples
///
/// ```
/// use bevy_kot::ecs::*;
/// use bevy::ecs::system::{SystemParam, SystemState};
/// use bevy::prelude::*;
/// use std::marker::PhantomData;
/// use std::ops::Deref;
/// 
/// #[derive(SystemParam)]
/// pub struct CallableTest<'w, 's>
/// {
///     counter: Local<'s, usize>,
/// 
///     // we must use phantom data to ignore unused lifetime names ('w in this case)
///     #[system_param(ignore)]
///     _phantom: PhantomData<(&'w (), &'s ())>,
/// }
/// 
/// impl<'w, 's> CallableSystem for CallableTest<'w, 's>
/// {
///     type ArgT = usize;
/// 
///     fn system(world: &mut World, state: &mut SystemState<Self>, _id: CallId, test_counter: usize)
///     {
///         // extract the system context
///         let mut context = state.get_mut(world);
/// 
///         // expect counter matches test_counter
///         assert_eq!(*context.counter.deref(), test_counter);
///         *context.counter = *context.counter + 1;
///     }
/// }
/// 
/// let mut world = World::new();
/// 
/// call::<CallableTest>(&mut world, CallId::new("a"), 0);
/// call::<CallableTest>(&mut world, CallId::new("a"), 1);
///
/// call::<CallableTest>(&mut world, CallId::new("b"), 0);
/// call::<CallableTest>(&mut world, CallId::new("b"), 1);
/// ```
///
pub fn call<S: CallableSystem + 'static>(world: &mut World, id: CallId, arg: S::ArgT)
{
    // 1. make sure the callable system has been cached for this call id
    init_callable_system::<S>(world, id);

    // 2. call our cached system
    world.resource_scope(
            | world, mut states: Mut<StateInstances<S>> |
            {
                let cached_state = states.instances.get_mut(&id).unwrap();
                S::system(world, cached_state, id, arg);
                cached_state.apply(world);
            }
        );
}

/// Call a callable system (no function arguments).
///
/// # Examples
///
/// ```
/// use bevy_kot::ecs::*;
/// use bevy::ecs::system::{SystemParam, SystemState};
/// use bevy::prelude::*;
/// use std::marker::PhantomData;
/// use std::ops::Deref;
/// 
/// #[derive(Resource)]
/// struct CallCounter(u16);
/// 
/// #[derive(SystemParam)]
/// pub struct BasicCallableTest<'w, 's>
/// {
///     counter: ResMut<'w, CallCounter>,
/// 
///     // we must use phantom data to ignore unused lifetime names ('w in this case)
///     #[system_param(ignore)]
///     _phantom: PhantomData<(&'w (), &'s ())>,
/// }
/// 
/// impl<'w, 's> BasicCallableSystem for BasicCallableTest<'w, 's>
/// {
///     fn system(world: &mut World, state: &mut SystemState<Self>, _id: CallId)
///     {
///         // extract the system context
///         let mut context = state.get_mut(world);
/// 
///         // increment global counter
///         context.counter.0 = context.counter.0 + 1;
///     }
/// }
///
/// let mut world = World::new();
/// world.insert_resource::<CallCounter>(CallCounter(0));
/// 
/// call_basic::<BasicCallableTest>(&mut world, CallId::new("a"));
/// call_basic::<BasicCallableTest>(&mut world, CallId::new("a"));
///
/// call_basic::<BasicCallableTest>(&mut world, CallId::new("b"));
/// call_basic::<BasicCallableTest>(&mut world, CallId::new("b"));
///
/// let counter = world.remove_resource::<CallCounter>().unwrap();
/// assert_eq!(counter.0, 4);
/// ```
///
pub fn call_basic<S: BasicCallableSystem + 'static>(world: &mut World, id: CallId)
{
    // 1. make sure the callable system has been cached for this call id
    init_callable_system::<S>(world, id);

    // 2. call our cached system
    world.resource_scope(
            | world, mut states: Mut<StateInstances<S>> |
            {
                let cached_state = states.instances.get_mut(&id).unwrap();
                S::system(world, cached_state, id);
                cached_state.apply(world);
            }
        );
}

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
    I: Send + Sync + 'static,
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
    I: Send + Sync + 'static,
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
/// Panics if [`setup_auto_despawn()`] was not added to your app.
pub fn spawn_rc_system<I, O, S, Marker>(world: &mut World, system: S) -> AutoDespawnSignal
where
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
    S: IntoSystem<I, O, Marker> + Send + Sync + 'static,
{
    spawn_rc_system_from(world, CallbackSystem::new(system))
}

/// Spawn a ref-counted system.
///
/// Returns a cleanup handle. The system will be dropped when the last copy of the handle is dropped.
///
/// Panics if [`setup_auto_despawn()`] was not added to your app.
pub fn spawn_rc_system_from<I, O>(world: &mut World, system: CallbackSystem<I, O>) -> AutoDespawnSignal
where
    I: Send + Sync + 'static,
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
/// use bevy_kot_ecs::*;
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
pub fn spawned_syscall<I, O>(world: &mut World, sys_id: SysId, input: I) -> Result<O, ()>
where
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    // extract the callback
    let Some(mut entity_mut) = world.get_entity_mut(sys_id.0) else { return Err(()); };
    let Some(mut spawned_system) = entity_mut.get_mut::<SpawnedSystem<I, O>>()
    else { tracing::error!(?sys_id, "spawned system component is missing"); return Err(()); };
    let Some(mut callback) = spawned_system.system.take()
    else { tracing::warn!(?sys_id, "recursive spawned system call detected"); return Err(()); };

    // invoke the callback
    let result = callback.run(world, input).ok_or(())?;

    // reinsert the callback if its target hasn't been despawned
    let Some(mut entity_mut) = world.get_entity_mut(sys_id.0) else { return Ok(result); };
    let Some(mut spawned_system) = entity_mut.get_mut::<SpawnedSystem<I, O>>()
    else { tracing::error!(?sys_id, "spawned system component is missing"); return Ok(result); };
    spawned_system.system = Some(callback);

    Ok(result)
}

//-------------------------------------------------------------------------------------------------------------------

pub trait SystemCallerCommandsExt
{
    /// Schedule a system to be spawned.
    ///
    /// Systems are not initialized until they are first run.
    ///
    /// Returns the system id that will eventually reference the spawned system. It can be used to invoke the system with
    /// [`spawned_syscall()`] or [`SystemCallerCommandsExt::spawned_syscall()`].
    fn spawn_system<I, O, S, Marker>(&mut self, system: S) -> SysId
    where
        I: Send + Sync + 'static,
        O: Send + Sync + 'static,
        S: IntoSystem<I, O, Marker> + Send + Sync + 'static;

    /// Schedule a system to be spawned.
    ///
    /// Returns the system id that will eventually reference the spawned system. It can be used to invoke the system with
    /// [`spawned_syscall()`] or [`SystemCallerCommandsExt::spawned_syscall()`].
    fn spawn_system_from<I, O>(&mut self, system: CallbackSystem<I, O>) -> SysId
    where
        I: Send + Sync + 'static,
        O: Send + Sync + 'static;

    /// Schedule a system to be inserted into the specified entity.
    ///
    /// This is useful for constructing self-referential systems, e.g. for systems that only run once then clean themselves
    /// up.
    ///
    /// Returns an error if the entity does not exist.
    fn insert_system<I, O, S, Marker>(&mut self, entity: Entity, system: S) -> Result<(), ()>
    where
        I: Send + Sync + 'static,
        O: Send + Sync + 'static,
        S: IntoSystem<I, O, Marker> + Send + Sync + 'static;

    /// Schedule a system call.
    ///
    /// Syntax sugar for [`syscall()`].
    fn syscall<I, S, Marker>(&mut self, input: I, system: S)
    where
        I: Send + Sync + 'static,
        S: IntoSystem<I, (), Marker> + Send + Sync + 'static;

    /// Schedule a spawned system call.
    ///
    /// It is the responsibility of the caller to correctly match the system entity with the target system signature.
    ///
    /// Logs a warning if the system entity doesn't exist.
    ///
    /// Syntax sugar for [`spawned_syscall()`].
    fn spawned_syscall<I>(&mut self, sys_id: SysId, input: I)
    where
        I: Send + Sync + 'static;
}

impl<'w, 's> SystemCallerCommandsExt for Commands<'w, 's>
{
    fn spawn_system<I, O, S, Marker>(&mut self, system: S) -> SysId
    where
        I: Send + Sync + 'static,
        O: Send + Sync + 'static,
        S: IntoSystem<I, O, Marker> + Send + Sync + 'static
    {
        self.spawn_system_from(CallbackSystem::new(system))
    }

    fn spawn_system_from<I, O>(&mut self, system: CallbackSystem<I, O>) -> SysId
    where
        I: Send + Sync + 'static,
        O: Send + Sync + 'static
    {
        SysId::new(self.spawn(SpawnedSystem::new(system)).id())
    }

    fn insert_system<I, O, S, Marker>(&mut self, entity: Entity, system: S) -> Result<(), ()>
    where
        I: Send + Sync + 'static,
        O: Send + Sync + 'static,
        S: IntoSystem<I, O, Marker> + Send + Sync + 'static
    {
        let Some(mut entity) = self.get_entity(entity) else { return Err(()); };
        entity.insert(SpawnedSystem::new(CallbackSystem::new(system)));

        Ok(())
    }

    fn syscall<I, S, Marker>(&mut self, input: I, system: S)
    where
        I: Send + Sync + 'static,
        S: IntoSystem<I, (), Marker> + Send + Sync + 'static,
    {
        self.add(move |world: &mut World| syscall(world, input, system));
    }

    fn spawned_syscall<I>(&mut self, sys_id: SysId, input: I)
    where
        I: Send + Sync + 'static,
    {
        self.add(
                move |world: &mut World|
                {
                    if let Err(_) = spawned_syscall::<I, ()>(world, sys_id, input)
                    {
                        tracing::warn!(?sys_id, "spawned syscall failed");
                    }
                }
            );
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Execute a named system on some data then apply the system's deferred commands.
///
/// Systems with different names will have different Local state.
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
/// fn test_system(In(input): In<u16>, mut local: Local<u16>) -> u16
/// {
///     *local += input;
///     *local
/// }
/// 
/// let mut world = World::new();
/// 
/// assert_eq!(named_syscall(&mut world, "a", 1u16, test_system), 1);
/// assert_eq!(named_syscall(&mut world, "a", 1u16, test_system), 2);    //Local is preserved
/// assert_eq!(named_syscall(&mut world, "b", 10u16, test_system), 10);  //new Local
/// assert_eq!(named_syscall(&mut world, "b", 10u16, test_system), 20);
/// ```
///
pub fn named_syscall<H, I, O, S, Marker>(
    world  : &mut World,
    id     : H,
    input  : I,
    system : S
) -> O
where
    H: Hash,
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
    S: IntoSystem<I, O, Marker> + Send + Sync + 'static,
{
    // the system id
    let sys_name = SysName::new::<S>(id);

    // get resource storing the id-mapped systems
    let mut id_mapped_systems = world.get_resource_or_insert_with::<IdMappedSystems<I, O>>(
            || IdMappedSystems::default()
        );

    // take the initialized system
    let mut system =
        match id_mapped_systems.systems.get_mut(&sys_name).map_or(None, |node| node.take())
        {
            Some(system) => system,
            None =>
            {
                let mut sys = IntoSystem::into_system(system);
                sys.initialize(world);
                Box::new(sys)
            }
        };

    // run the system
    let result = system.run(input, world);

    // apply any pending changes
    system.apply_deferred(world);

    // re-acquire mutable access to id-mapped systems
    let mut id_mapped_systems = world.get_resource_or_insert_with::<IdMappedSystems<I, O>>(
            || IdMappedSystems::default()
        );

    // put the system back
    // - we ignore overwrites
    match id_mapped_systems.systems.get_mut(&sys_name)
    {
        Some(node) => { let _ = node.replace(system); },
        None       => { let _ = id_mapped_systems.systems.insert(sys_name, Some(system)); },
    }

    result
}

/// Directly invoke a named system.
///
/// Returns `Err` if the system cannot be found.
pub fn named_syscall_direct<I, O>(world: &mut World, sys_name: SysName, input: I) -> Result<O, ()>
where
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    // get resource storing the id-mapped systems
    let mut id_mapped_systems = world.get_resource_or_insert_with::<IdMappedSystems<I, O>>(
            || IdMappedSystems::default()
        );

    // take the initialized system
    let mut system =
        match id_mapped_systems.systems.get_mut(&sys_name).map_or(None, |node| node.take())
        {
            Some(system) => system,
            None => return Err(()),
        };

    // run the system
    let result = system.run(input, world);

    // apply any pending changes
    system.apply_deferred(world);

    // re-acquire mutable access to id-mapped systems
    let mut id_mapped_systems = world.get_resource_or_insert_with::<IdMappedSystems<I, O>>(
            || IdMappedSystems::default()
        );

    // put the system back
    // - we ignore overwrites
    match id_mapped_systems.systems.get_mut(&sys_name)
    {
        Some(node) => { let _ = node.replace(system); },
        None       => { let _ = id_mapped_systems.systems.insert(sys_name, Some(system)); },
    }

    Ok(result)
}

/// Register a named system for future use.
///
/// Over-writes the existing system with the same id and type, if one exists.
///
/// Useful for inserting a closure-type system that captures non-Copy data when you need to invoke the system
/// multiple times.
///
/// We pass in `sys_name` directly to enable direct control over defining the id. Manually defining the id may
/// be appropriate if you are potentially generating large numbers of named systems and want to ensure there
/// are no collisions. It may also be appropriate if you have multiple naming regimes and want to domain-separate
/// the system ids (e.g. via type wrappers: `SysName::new_raw::<Wrapper<S>>(counter)`)
pub fn register_named_system<I, O, S, Marker>(world: &mut World, sys_name: SysName, system: S)
where
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
    S: IntoSystem<I, O, Marker> + Send + Sync + 'static,
{
    register_named_system_from(world, sys_name, CallbackSystem::new(system));
}

pub fn register_named_system_from<I, O>(world: &mut World, sys_name: SysName, callback: CallbackSystem<I, O>)
where
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    // initialize the callback
    let Some(boxed_system) = callback.take_initialized(world) else { return; };

    // get resource storing the id-mapped systems
    let mut id_mapped_systems = world.get_resource_or_insert_with::<IdMappedSystems<I, O>>(
        || IdMappedSystems::default()
    );

    // insert the system
    match id_mapped_systems.systems.get_mut(&sys_name)
    {
        Some(node) => { let _ = node.replace(boxed_system); },
        None       => { let _ = id_mapped_systems.systems.insert(sys_name, Some(boxed_system)); },
    }
}

/// System identifier for use in named systems.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SysName(u64, TypeId);

impl SysName
{
    pub fn new<S: 'static>(id: impl Hash) -> Self
    {
        let mut hasher = AHasher::default();
        id.hash(&mut hasher);
        SysName(hasher.finish(), TypeId::of::<S>())
    }

    pub fn new_raw<S: 'static>(id: u64) -> Self
    {
        SysName(id, TypeId::of::<S>())
    }

    pub fn id(&self) -> u64
    {
        self.0
    }

    pub fn type_id(&self) -> TypeId
    {
        self.1
    }
}

/// Tracks named systems.
#[derive(Resource)]
pub struct IdMappedSystems<I, O>
where
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    systems: HashMap<SysName, Option<BoxedSystem<I, O>>>,
}

impl<I, O> IdMappedSystems<I, O>
where
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    pub fn revoke<S: 'static>(&mut self, id: impl Hash)
    {
        let id = SysName::new::<S>(id);
        let _ = self.systems.remove(&id);
    }

    pub fn revoke_sysname(&mut self, id: SysName)
    {
        let _ = self.systems.remove(&id);
    }
}

impl<I, O> Default for IdMappedSystems<I, O>
where
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    fn default() -> Self { Self{ systems: HashMap::default() } }
}

//-------------------------------------------------------------------------------------------------------------------
