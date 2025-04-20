//local shortcuts
use crate::prelude::*;

//third-party shortcuts
use ahash::AHasher;
use bevy::ecs::system::BoxedSystem;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

//standard shortcuts
use std::any::TypeId;
use std::hash::{Hash, Hasher};

//-------------------------------------------------------------------------------------------------------------------
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
    input  : <I as SystemInput>::Inner<'_>,
    system : S
) -> O
where
    H: Hash,
    I: Send + Sync + SystemInput + 'static,
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

//-------------------------------------------------------------------------------------------------------------------

/// Directly invoke a named system.
///
/// Returns `Err` if the system cannot be found.
pub fn named_syscall_direct<I, O>(
    world: &mut World,
    sys_name: SysName,
    input: <I as SystemInput>::Inner<'_>
) -> Result<O, CobwebEcsError>
where
    I: Send + Sync + SystemInput + 'static,
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
            None => return Err(CobwebEcsError::NamedSyscall(sys_name)),
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

//-------------------------------------------------------------------------------------------------------------------

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
    I: Send + Sync + SystemInput + 'static,
    O: Send + Sync + 'static,
    S: IntoSystem<I, O, Marker> + Send + Sync + 'static,
{
    register_named_system_from(world, sys_name, CallbackSystem::new(system));
}

//-------------------------------------------------------------------------------------------------------------------

pub fn register_named_system_from<I, O>(world: &mut World, sys_name: SysName, callback: CallbackSystem<I, O>)
where
    I: Send + Sync + SystemInput + 'static,
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

//-------------------------------------------------------------------------------------------------------------------

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

//-------------------------------------------------------------------------------------------------------------------

/// Tracks named systems.
#[derive(Resource)]
pub struct IdMappedSystems<I, O>
where
    I: Send + Sync + SystemInput + 'static,
    O: Send + Sync + 'static,
{
    systems: HashMap<SysName, Option<BoxedSystem<I, O>>>,
}

impl<I, O> IdMappedSystems<I, O>
where
    I: Send + Sync + SystemInput + 'static,
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
    I: Send + Sync + SystemInput + 'static,
    O: Send + Sync + 'static,
{
    fn default() -> Self { Self{ systems: HashMap::default() } }
}

//-------------------------------------------------------------------------------------------------------------------
