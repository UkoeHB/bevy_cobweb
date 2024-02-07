//local shortcuts

//third-party shortcuts
use bevy::ecs::system::{Command, BoxedSystem};
use bevy::prelude::*;

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

/// Represents a system callback.
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
    I: Send + Sync + 'static,
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

    pub fn run(&mut self, world: &mut World, input: I) -> Option<O>
    {
        self.run_with_cleanup(world, input, |_| {})
    }

    pub fn run_with_cleanup(&mut self, world: &mut World, input: I, cleanup: impl FnOnce(&mut World) + 'static) -> Option<O>
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
        let result = system.run(input, world);
        (cleanup)(world);
        system.apply_deferred(world);
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

/// Try to invoke the callback `C` on `entity`.
///
/// Returns `false` if the entity doesn't exist or the callback is not present on the entity.
pub fn try_callback<C: Send + Sync + 'static>(world: &mut World, entity: Entity) -> bool
{
    let Some(entity_mut) = world.get_entity_mut(entity) else { return false; };
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
    let Some(entity_mut) = world.get_entity_mut(entity) else { return false; };
    let Some(cb) = entity_mut.get::<CallbackWith<C, V>>() else { return false; };
    cb.call_with(value).apply(world);
    true
}

//-------------------------------------------------------------------------------------------------------------------
