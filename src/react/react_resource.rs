//local shortcuts
use crate::prelude::*;

//third-party shortcuts
use bevy::prelude::*;
use bevy::ecs::component::Tick;
use bevy::ecs::system::SystemParam;

//standard shortcuts
use core::ops::Deref;

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn trigger_resource_mutation<R: ReactResource>(mut c: Commands)
{
    c.react().trigger_resource_mutation::<R>();
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// Resource wrapper that enables reacting to resource mutations.
#[derive(Resource)]
struct ReactResInner<R: ReactResource>
{
    resource: R,
}

impl<R: ReactResource> ReactResInner<R>
{
    /// New react resource.
    fn new(resource: R) -> Self
    {
        Self{ resource }
    }

    /// Mutably access the resource and trigger reactions.
    fn get_mut<'a>(&'a mut self, c: &mut Commands) -> &'a mut R
    {
        c.react().trigger_resource_mutation::<R>();
        &mut self.resource
    }

    /// Mutably access the resource without triggering reactions.
    fn get_noreact(&mut self) -> &mut R
    {
        &mut self.resource
    }

    /// Sets the resource value and triggers mutations only if the value will change.
    ///
    /// Returns the previous value if it changed.
    fn set_if_neq(&mut self, c: &mut Commands, new: R) -> Option<R>
    where
        R: PartialEq
    {
        if new == self.resource { return None; }

        c.react().trigger_resource_mutation::<R>();
        let old = std::mem::replace(&mut self.resource, new);
        Some(old)
    }

    /// Unwrap the resource.
    fn take(self) -> R
    {
        self.resource
    }
}

impl<R: ReactResource> Deref for ReactResInner<R>
{
    type Target = R;

    fn deref(&self) -> &R
    {
        &self.resource
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// Tag trait for reactive resources.
///
/// It is not recommended to add `ReactResource` and `Resource` to the same struct, as it will likely cause confusion.
pub trait ReactResource: Send + Sync + 'static {}

//-------------------------------------------------------------------------------------------------------------------

/// Immutable reader for reactive resources.
#[derive(SystemParam)]
pub struct ReactRes<'w, R: ReactResource>
{
    inner: Res<'w, ReactResInner<R>>,
}

impl<'w, R: ReactResource> DetectChanges for ReactRes<'w, R>
{
    #[inline] fn is_added(&self) -> bool { self.inner.is_added() }
    #[inline] fn is_changed(&self) -> bool { self.inner.is_changed() }
    #[inline] fn last_changed(&self) -> Tick { self.inner.last_changed() }
}

impl<'w, R: ReactResource> Deref for ReactRes<'w, R>
{
    type Target = R;

    fn deref(&self) -> &R
    {
        &self.inner
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Mutable wrapper for reactive resources.
#[derive(SystemParam)]
pub struct ReactResMut<'w, R: ReactResource>
{
    inner: ResMut<'w, ReactResInner<R>>,
}

impl<'w, R: ReactResource> ReactResMut<'w, R>
{
    /// Mutably access the resource and trigger reactions.
    pub fn get_mut<'a>(&'a mut self, c: &mut Commands) -> &'a mut R
    {
        self.inner.get_mut(c)
    }

    /// Mutably access the resource without triggering reactions.
    pub fn get_noreact(&mut self) -> &mut R
    {
        self.inner.get_noreact()
    }

    /// Sets the resource value and triggers mutations only if the value will change.
    ///
    /// Returns the previous value if it changed.
    pub fn set_if_neq(&mut self, c: &mut Commands, new: R) -> Option<R>
    where
        R: PartialEq
    {
        (*self.inner).set_if_neq(c, new)
    }
}

impl<'w, R: ReactResource> DetectChanges for ReactResMut<'w, R>
{
    #[inline] fn is_added(&self) -> bool { self.inner.is_added() }
    #[inline] fn is_changed(&self) -> bool { self.inner.is_changed() }
    #[inline] fn last_changed(&self) -> Tick { self.inner.last_changed() }
}

impl<'w, R: ReactResource> Deref for ReactResMut<'w, R>
{
    type Target = R;

    fn deref(&self) -> &R
    {
        &self.inner
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Extends the `World` API with reactive resource methods.
///
/// Does NOT include `react_resource_mut()` because reactions need to be queued to run *after* a resource is mutated,
/// but world access doesn't make it easy to defer commands. Instead you can use `trigger_resource_mutation()` in
/// combination with `react_resource_mut_noreact()` to manually orchestrate mutation reactions.
pub trait ReactResWorldExt
{
    /// Does nothing if the resource already exists.
    fn init_react_resource<R: ReactResource + FromWorld>(&mut self);
    fn insert_react_resource<R: ReactResource>(&mut self, value: R);
    fn remove_react_resource<R: ReactResource>(&mut self) -> Option<R>;
    fn contains_react_resource<R: ReactResource>(&self) -> bool;
    fn is_react_resource_added<R: ReactResource>(&self) -> bool;
    fn is_react_resource_changed<R: ReactResource>(&self) -> bool;
    /// Panics if the resource doesn't exist.
    fn react_resource<R: ReactResource>(&self) -> &R;
    /// Panics if the resource doesn't exist.
    fn react_resource_mut_noreact<R: ReactResource>(&mut self) -> &mut R;
    fn get_react_resource<R: ReactResource>(&self) -> Option<&R>;
    fn get_react_resource_noreact<R: ReactResource>(&mut self) -> Option<&mut R>;
    fn get_react_resource_or_insert_with<R: ReactResource>(
        &mut self,
        func: impl FnOnce() -> R,
    ) -> &R;
    /// Panics if the resource doesn't exist.
    fn trigger_resource_mutation<R: ReactResource>(&mut self);
}

impl ReactResWorldExt for World
{
    fn init_react_resource<R: ReactResource + FromWorld>(&mut self)
    {
        if self.contains_react_resource::<R>() { return; }
        let value = R::from_world(self);
        self.insert_react_resource(value);
    }

    fn insert_react_resource<R: ReactResource>(&mut self, value: R)
    {
        self.insert_resource(ReactResInner::new(value));
    }

    fn remove_react_resource<R: ReactResource>(&mut self) -> Option<R>
    {
        self.remove_resource::<ReactResInner<R>>().map_or(None, |r| Some(r.take()))
    }

    fn contains_react_resource<R: ReactResource>(&self) -> bool
    {
        self.contains_resource::<ReactResInner<R>>()
    }

    fn is_react_resource_added<R: ReactResource>(&self) -> bool
    {
        self.is_resource_added::<ReactResInner<R>>()
    }

    fn is_react_resource_changed<R: ReactResource>(&self) -> bool
    {
        self.is_resource_changed::<ReactResInner<R>>()
    }

    fn react_resource<R: ReactResource>(&self) -> &R
    {
        &self.resource::<ReactResInner<R>>()
    }

    fn react_resource_mut_noreact<R: ReactResource>(&mut self) -> &mut R
    {
        self.get_react_resource_noreact().expect("react resource missing!")
    }

    fn get_react_resource<R: ReactResource>(&self) -> Option<&R>
    {
        self.get_resource::<ReactResInner<R>>().map_or(None, |r| Some(&r))
    }

    fn get_react_resource_noreact<R: ReactResource>(&mut self) -> Option<&mut R>
    {
        self.get_resource_mut::<ReactResInner<R>>().map_or(None, |r| Some(r.into_inner().get_noreact()))
    }

    fn get_react_resource_or_insert_with<R: ReactResource>(
        &mut self,
        func: impl FnOnce() -> R,
    ) -> &R
    {
        self.get_resource_or_insert_with(move || ReactResInner::new((func)())).into_inner()
    }

    fn trigger_resource_mutation<R: ReactResource>(&mut self)
    {
        self.syscall((), trigger_resource_mutation::<R>);
    }
}

//-------------------------------------------------------------------------------------------------------------------

pub trait ReactResAppExt
{
    /// Does nothing if the resource already exists.
    fn init_react_resource<R: ReactResource + FromWorld>(&mut self) -> &mut Self;
    fn insert_react_resource<R: ReactResource>(&mut self, value: R) -> &mut Self;
}

impl ReactResAppExt for App
{
    fn init_react_resource<R: ReactResource + FromWorld>(&mut self) -> &mut Self
    {
        self.world.init_react_resource::<R>();
        self
    }

    fn insert_react_resource<R: ReactResource>(&mut self, value: R) -> &mut Self
    {
        self.world.insert_react_resource(value);
        self
    }
}

//-------------------------------------------------------------------------------------------------------------------

pub trait ReactResCommandsExt
{
    /// Does nothing if the resource already exists.
    fn init_react_resource<R: ReactResource + FromWorld>(&mut self);
    fn insert_react_resource<R: ReactResource>(&mut self, value: R);
    fn remove_react_resource<R: ReactResource>(&mut self);
}

impl<'w, 's> ReactResCommandsExt for Commands<'w, 's>
{
    fn init_react_resource<R: ReactResource + FromWorld>(&mut self)
    {
        self.add(|world: &mut World| world.init_react_resource::<R>());
    }

    fn insert_react_resource<R: ReactResource>(&mut self, value: R)
    {
        self.insert_resource(ReactResInner::new(value));
    }

    fn remove_react_resource<R: ReactResource>(&mut self)
    {
        self.remove_resource::<ReactResInner<R>>();
    }
}

//-------------------------------------------------------------------------------------------------------------------

