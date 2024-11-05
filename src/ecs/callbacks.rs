//local shortcuts

//third-party shortcuts
use bevy::ecs::system::BoxedSystem;
use bevy::ecs::world::Command;
use bevy::prelude::*;

use std::borrow::BorrowMut;
//standard shortcuts
use std::marker::PhantomData;
use std::sync::Arc;

//-------------------------------------------------------------------------------------------------------------------

/// Callback wrapper for FnOnce functions. Implements `Command`.
/// - The type `T` can be used to mark the callback for query filtering.
#[derive(Component)]
pub struct CallOnce<T: Send + Sync + 'static>
{
    callonce : Box<dyn FnOnce(&mut World) -> () + Send + Sync + 'static>,
    _phantom : PhantomData<T>,
}

impl<T: Send + Sync + 'static> CallOnce<T>
{
    pub fn new(callonce: impl FnOnce(&mut World) -> () + Send + Sync + 'static) -> Self
    {
        Self{ callonce: Box::new(callonce), _phantom: PhantomData::default() }
    }
}

impl<T: Send + Sync + 'static> Command for CallOnce<T>
{
    fn apply(self, world: &mut World)
    {
        (self.callonce)(world);
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Callback wrapper that lets you call with a value once. The helper returned by `.call_with()` implements `Command`.
/// - The type `T` can be used to mark the callback for query filtering.
#[derive(Component)]
pub struct CallOnceWith<T: Send + Sync + 'static, V>
{
    callonce : Box<dyn FnOnce(&mut World, V) -> () + Send + Sync + 'static>,
    _phantom : PhantomData<T>,
}

impl<T: Send + Sync + 'static, V: Send + Sync + 'static> CallOnceWith<T, V>
{
    pub fn new(callonce: impl FnOnce(&mut World, V) -> () + Send + Sync + 'static) -> Self
    {
        Self{ callonce: Box::new(callonce), _phantom: PhantomData::default() }
    }

    pub fn call_with(self, call_value: V) -> CallwithOnce<T, V>
    {
        CallwithOnce{ callonce: self.callonce, call_value, _phantom: PhantomData::default() }
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Callback wrapper with a specific call value baked in. Implements `Command`.
/// - The type `T` can be used to mark the callback for query filtering.
pub struct CallwithOnce<T: Send + Sync + 'static, C>
{
    callonce   : Box<dyn FnOnce(&mut World, C) -> () + Send + Sync + 'static>,
    call_value : C,
    _phantom   : PhantomData<T>,
}

impl<T: Send + Sync + 'static, C> CallwithOnce<T, C>
{
    pub fn new(callonce: impl FnOnce(&mut World, C) -> () + Send + Sync + 'static, call_value: C) -> Self
    {
        Self{ callonce: Box::new(callonce), call_value, _phantom: PhantomData::default() }
    }
}

impl<T: Send + Sync + 'static, C: Send + Sync + 'static> Command for CallwithOnce<T, C>
{
    fn apply(self, world: &mut World)
    {
        (self.callonce)(world, self.call_value);
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Callback wrapper. Implements `Command`.
/// - The type `T` can be used to mark the callback for query filtering.
#[derive(Component)]
pub struct Callback<T: Send + Sync + 'static>
{
    callback : Arc<dyn Fn(&mut World) -> () + Send + Sync + 'static>,
    _phantom : PhantomData<T>,
}

impl<T: Send + Sync + 'static> Clone for Callback<T>
{ fn clone(&self) -> Self { Self{ callback: self.callback.clone(), _phantom: PhantomData::default() } } }

impl<T: Send + Sync + 'static> Callback<T>
{
    pub fn new(callback: impl Fn(&mut World) -> () + Send + Sync + 'static) -> Self
    {
        Self{ callback: Arc::new(callback), _phantom: PhantomData::default() }
    }
}

impl<T: Send + Sync + 'static> Command for Callback<T>
{
    fn apply(self, world: &mut World)
    {
        (self.callback)(world);
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Callback wrapper that lets you call with a value. The helper returned by `.call_with()` implements `Command`.
/// - The type `T` can be used to mark the callback for query filtering.
#[derive(Component)]
pub struct CallbackWith<T: Send + Sync + 'static, V>
{
    callback : Arc<dyn Fn(&mut World, V) -> () + Send + Sync + 'static>,
    _phantom : PhantomData<T>,
}

impl<T: Send + Sync + 'static, V> Clone for CallbackWith<T, V>
{ fn clone(&self) -> Self { Self{ callback: self.callback.clone(), _phantom: PhantomData::default() } } }

impl<T: Send + Sync + 'static, V: Send + Sync + 'static> CallbackWith<T, V>
{
    pub fn new(callback: impl Fn(&mut World, V) -> () + Send + Sync + 'static) -> Self
    {
        Self{ callback: Arc::new(callback), _phantom: PhantomData::default() }
    }

    pub fn call_with(&self, call_value: V) -> Callwith<T, V>
    {
        Callwith{ callback: self.callback.clone(), call_value, _phantom: PhantomData::default() }
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Callback wrapper with a specific call value baked in. Implements `Command`.
/// - The type `T` can be used to mark the callback for query filtering.
pub struct Callwith<T: Send + Sync + 'static, C>
{
    callback   : Arc<dyn Fn(&mut World, C) -> () + Send + Sync + 'static>,
    call_value : C,
    _phantom   : PhantomData<T>,
}

impl<T: Send + Sync + 'static, C> Callwith<T, C>
{
    pub fn new(callback: impl Fn(&mut World, C) -> () + Send + Sync + 'static, call_value: C) -> Self
    {
        Self{ callback: Arc::new(callback), call_value, _phantom: PhantomData::default() }
    }
}

impl<T: Send + Sync + 'static, C: Send + Sync + 'static> Command for Callwith<T, C>
{
    fn apply(self, world: &mut World)
    {
        (self.callback)(world, self.call_value);
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Callback wrapper that mimics `syscall` (**without** an implicit call to `apply_deferred`).
/// - The type `T` can be used to mark the callback for query filtering.
#[derive(Component)]
pub struct SysCall<T: Send + Sync + 'static, I, O>
{
    callback : Arc<dyn Fn(&mut World, I) -> O + Send + Sync + 'static>,
    _phantom : PhantomData<T>,
}

impl<T: Send + Sync + 'static, I, O> Clone for SysCall<T, I, O>
{ fn clone(&self) -> Self { Self{ callback: self.callback.clone(), _phantom: PhantomData::default() } } }

impl<T: Send + Sync + 'static, I, O> SysCall<T, I, O>
{
    pub fn new(callback: impl Fn(&mut World, I) -> O + Send + Sync + 'static) -> Self
    {
        Self{ callback: Arc::new(callback), _phantom: PhantomData::default() }
    }

    pub fn call(&self, world: &mut World, in_val: I) -> O
    {
        (self.callback)(world, in_val)
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Runs a system with cleanup that occurs between running the system and applying deferred commands.
///
/// This function assumes `system` has already been initialized in the world.
pub fn run_initialized_system<I, O>(
    world: &mut World,
    system: &mut dyn System<In = I, Out = O>,
    input: <I as SystemInput>::Inner<'_>,
    cleanup: impl FnOnce(&mut World) + Send + Sync + 'static
) -> O
where
    I: Send + Sync + SystemInput + 'static,
    O: Send + Sync + 'static
{
    if system.is_exclusive() {
        // Add the cleanup to run before any commands added by the system.
        world.commands().queue(move |world: &mut World| (cleanup)(world));
        system.run(input, world)
    } else {
        // For non-exclusive systems we need to run them unsafe because the safe version automatically
        // calls `apply_deferred`.
        let world_cell = world.as_unsafe_world_cell();
        system.update_archetype_component_access(world_cell);
        // SAFETY:
        // - We have exclusive access to the entire world.
        // - `update_archetype_component_access` has been called.
        let result = unsafe { system.run_unsafe(input, world_cell) };

        // Run our custom cleanup method.
        (cleanup)(world);

        // apply any pending changes
        system.apply_deferred(world);

        result
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Represents a system callback.
///
/// See [`RawCallbackSystem`] for a wrapper around raw systems.
#[derive(Default, Component)]
pub enum CallbackSystem<I, O>
{
    #[default]
    Empty,
    New(BoxedSystem<I, O>),
    Initialized(BoxedSystem<I, O>),
}

impl<I, O> CallbackSystem<I, O>
where
    I: Send + Sync + SystemInput + 'static,
    O: Send + Sync + 'static,
{
    pub fn new<Marker, S>(system: S) -> Self
    where
        S: IntoSystem<I, O, Marker> + Send + Sync + 'static,
    {
        CallbackSystem::New(Box::new(IntoSystem::into_system(system)))
    }

    pub fn initialize(&mut self, world: &mut World)
    {
        let CallbackSystem::New(system) = self else { return; };
        system.initialize(world);
    }

    pub fn run(&mut self, world: &mut World, input: <I as SystemInput>::Inner<'_>) -> Option<O>
    {
        self.run_with_cleanup(world, input, |_| {})
    }

    pub fn run_with_cleanup(
        &mut self,
        world: &mut World,
        input: <I as SystemInput>::Inner<'_>,
        cleanup: impl FnOnce(&mut World) + Send + Sync + 'static
    ) -> Option<O>
    {
        let mut system = match std::mem::take(self)
        {
            CallbackSystem::Empty =>
            {
                (cleanup)(world);
                return None;
            }
            CallbackSystem::New(mut system) =>
            {
                system.initialize(world);
                system
            }
            CallbackSystem::Initialized(system) => system,
        };

        // run the system
        let result = run_initialized_system(world, system.borrow_mut(), input, cleanup);

        // Save the system for reuse.
        *self = CallbackSystem::Initialized(system);

        Some(result)
    }

    pub fn take_initialized(self, world: &mut World) -> Option<BoxedSystem<I, O>>
    {
        match self
        {
            CallbackSystem::Empty => None,
            CallbackSystem::New(mut system) =>
            {
                system.initialize(world);
                Some(system)
            }
            CallbackSystem::Initialized(system) => Some(system),
        }
    }

    pub fn has_system(&self) -> bool
    {
        !self.is_empty()
    }

    pub fn is_empty(&self) -> bool
    {
        match &self
        {
            CallbackSystem::Empty => true,
            _ => false
        }
    }

    pub fn is_new(&self) -> bool
    {
        match &self
        {
            CallbackSystem::New(_) => true,
            _ => false
        }
    }

    pub fn is_initialized(&self) -> bool
    {
        match &self
        {
            CallbackSystem::Initialized(_) => true,
            _ => false
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Represents a system callback.
///
/// See [`CallbackSystem`] for a wrapper around boxed systems.
#[derive(Default)]
pub enum RawCallbackSystem<I, O, S: System<In = I, Out = O>>
{
    #[default]
    Empty,
    New(S),
    Initialized(S),
}

impl<I, O, S> RawCallbackSystem<I, O, S>
where
    I: Send + Sync + SystemInput + 'static,
    O: Send + Sync + 'static,
    S: System<In = I, Out = O> + Send + Sync + 'static
{
    pub fn new<Marker, IS>(system: IS) -> Self
    where
        IS: IntoSystem<I, O, Marker, System = S> + Send + Sync + 'static,
    {
        RawCallbackSystem::New(IntoSystem::into_system(system))
    }

    pub fn initialize(&mut self, world: &mut World)
    {
        let RawCallbackSystem::New(system) = self else { return; };
        system.initialize(world);
    }

    pub fn run(&mut self, world: &mut World, input: <I as SystemInput>::Inner<'_>) -> O
    {
        self.run_with_cleanup(world, input, |_| {})
    }

    pub fn run_with_cleanup(
        &mut self,
        world: &mut World,
        input: <I as SystemInput>::Inner<'_>,
        cleanup: impl FnOnce(&mut World) + Send + Sync + 'static
    ) -> O
    {
        let mut system = match std::mem::take(self)
        {
            RawCallbackSystem::Empty =>
            {
                panic!("tried running an empty RawCallbackSystem");
            }
            RawCallbackSystem::New(mut system) =>
            {
                system.initialize(world);
                system
            }
            RawCallbackSystem::Initialized(system) => system,
        };

        // run the system
        let result = run_initialized_system(world, &mut system, input, cleanup);

        // Save the system for reuse.
        *self = RawCallbackSystem::Initialized(system);

        result
    }

    pub fn is_new(&self) -> bool
    {
        match &self
        {
            RawCallbackSystem::New(_) => true,
            _ => false
        }
    }

    pub fn is_initialized(&self) -> bool
    {
        match &self
        {
            RawCallbackSystem::Initialized(_) => true,
            _ => false
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Try to invoke the callback `C` on `entity`.
///
/// Returns `false` if the entity doesn't exist or the callback is not present on the entity.
pub fn try_callback<C: Send + Sync + 'static>(world: &mut World, entity: Entity) -> bool
{
    let Ok(entity_mut) = world.get_entity_mut(entity) else { return false; };
    let Some(cb) = entity_mut.get::<Callback<C>>() else { return false; };
    cb.clone().apply(world);
    true
}

//-------------------------------------------------------------------------------------------------------------------

/// Try to invoke the callback `C` on `entity` with `value`.
///
/// Returns `false` if the entity doesn't exist or the callback is not present on the entity.
pub fn try_callback_with<C, V>(world: &mut World, entity: Entity, value: V) -> bool
where
    C: Send + Sync + 'static,
    V: Send + Sync + 'static
{
    let Ok(entity_mut) = world.get_entity_mut(entity) else { return false; };
    let Some(cb) = entity_mut.get::<CallbackWith<C, V>>() else { return false; };
    cb.call_with(value).apply(world);
    true
}

//-------------------------------------------------------------------------------------------------------------------
