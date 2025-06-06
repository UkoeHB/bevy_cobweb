//local shortcuts
use crate::prelude::*;

//third-party shortcuts
use bevy::prelude::*;
use bevy::ecs::system::SystemParam;

//standard shortcuts
use core::ops::Deref;
use std::any::type_name;

//-------------------------------------------------------------------------------------------------------------------

/// Tag trait for reactive components.
///
/// It is not recommended to add `ReactComponent` and `Component` to the same struct, as it will likely cause confusion.
pub trait ReactComponent: Send + Sync + 'static {}

//-------------------------------------------------------------------------------------------------------------------

/// Component wrapper that enables reacting to component mutations.
/// - WARNING: It is possible to remove a `React` from one entity and manually insert it to another entity. That WILL
///            break the react framework. Instead use `react_commands.insert(new_entity, react_component.take());`.
#[derive(Component)]
pub struct React<C: ReactComponent>
{
    pub(crate) entity    : Entity,
    pub(crate) component : C,
}

impl<C: ReactComponent> React<C>
{
    /// Constructs the component without setting a valid entity or triggering on-insert reactions.
    pub fn new_unsafe(component: C) -> Self
    {
        Self{ entity: Entity::PLACEHOLDER, component }
    }

    /// Immutably accesses the component.
    pub fn get(&self) -> &C
    {
        &self.component
    }

    /// Mutably accesses the component and triggers reactions.
    pub fn get_mut<'a>(&'a mut self, c: &mut Commands) -> &'a mut C
    {
        c.syscall(self.entity, ReactCache::schedule_mutation_reaction::<C>);
        &mut self.component
    }

    /// Allows manually triggering mutation reactions when in an exclusive context.
    pub fn trigger_mutation(entity: Entity, world: &mut World)
    {
        world.syscall(entity, ReactCache::schedule_mutation_reaction::<C>);
    }

    /// Mutably accesses the component without triggering reactions.
    pub fn get_noreact(&mut self) -> &mut C
    {
        &mut self.component
    }

    /// Sets the component value and triggers mutations only if the value will change.
    ///
    /// Returns the previous value if it changed.
    pub fn set_if_neq(&mut self, c: &mut Commands, new: C) -> Option<C>
    where
        C: PartialEq
    {
        if new == self.component { return None; }

        c.syscall(self.entity, ReactCache::schedule_mutation_reaction::<C>);
        let old = std::mem::replace(&mut self.component, new);
        Some(old)
    }

    /// Unwrap the `React`.
    pub fn take(self) -> C
    {
        self.component
    }
}

impl<C: ReactComponent> Deref for React<C>
{
    type Target = C;

    fn deref(&self) -> &C
    {
        &self.component
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// System parameter for accessing [`React<T>`] components immutably.
///
/// See [`ReactiveMut`] for the mutable version.
#[derive(SystemParam)]
pub struct Reactive<'w, 's, T: ReactComponent>
{
    components: Query<'w, 's, (Entity, &'static React<T>)>,
}

impl<'w, 's, T: ReactComponent> Reactive<'w, 's, T>
{
    /// Reads `T` on `entity`.
    ///
    /// Does not trigger reactions.
    pub fn get(&self, entity: Entity) -> Result<&T, CobwebReactError>
    {
        let t = type_name::<T>();
        self.components.get(entity).map(|(_, c)| c.get()).map_err(|_| CobwebReactError::Reactive(entity, t))
    }

    /// Reads `T` on a single entity.
    ///
    /// Does not trigger reactions.
    ///
    /// Panics if the inner query doesn't have exactly one entity.
    pub fn single(&self) -> (Entity, &T)
    {
        let (e, x) = self.components.single().unwrap();
        (e, x.get())
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// System parameter for accessing [`React<T>`] components mutably.
///
/// See [`Reactive`] for the immutable version.
#[derive(SystemParam)]
pub struct ReactiveMut<'w, 's, T: ReactComponent>
{
    components: Query<'w, 's, (Entity, &'static mut React<T>)>,
}

impl<'w, 's, T: ReactComponent> ReactiveMut<'w, 's, T>
{
    /// Reads `T` on `entity`.
    ///
    /// Does not trigger reactions.
    pub fn get(&self, entity: Entity) -> Result<&T, CobwebReactError>
    {
        let t = type_name::<T>();
        self.components.get(entity).map(|(_, c)| c.get()).map_err(|_| CobwebReactError::ReactiveMut(entity, t))
    }

    /// Reads `T` on a single entity.
    ///
    /// Does not trigger reactions.
    ///
    /// Panics if the inner query doesn't have exactly one entity.
    pub fn single(&self) -> (Entity, &T)
    {
        let (e, x) = self.components.single().unwrap();
        (e, x.get())
    }

    /// Gets a mutable reference to `T` on `entity`.
    ///
    /// Triggers mutation reactions.
    pub fn get_mut(&mut self, c: &mut Commands, entity: Entity) -> Result<&mut T, CobwebReactError>
    {
        let t = type_name::<T>();
        let (_, x) = self.components.get_mut(entity).map_err(|_| CobwebReactError::ReactiveMut(entity, t))?;
        Ok(x.into_inner().get_mut(c))
    }

    /// Gets a mutable reference to `T` on a single entity.
    ///
    /// Triggers mutation reactions.
    ///
    /// Panics if the inner query doesn't have exactly one entity.
    pub fn single_mut(&mut self, c: &mut Commands) -> (Entity, &mut T)
    {
        let (e, x) = self.components.single_mut().unwrap();
        (e, x.into_inner().get_mut(c))
    }

    /// Gets a mutable reference to `T` on `entity`.
    ///
    /// Does not trigger reactions.
    pub fn get_noreact(&mut self, entity: Entity) -> Result<&mut T, CobwebReactError>
    {
        let t = type_name::<T>();
        let (_, x) = self.components.get_mut(entity).map_err(|_| CobwebReactError::ReactiveMut(entity, t))?;
        Ok(x.into_inner().get_noreact())
    }

    /// Gets a mutable reference to `T` on a single entity
    ///
    /// Does not trigger reactions.
    ///
    /// Panics if the inner query doesn't have exactly one entity.
    pub fn single_noreact(&mut self) -> (Entity, &mut T)
    {
        let (e, x) = self.components.single_mut().unwrap();
        (e, x.into_inner().get_noreact())
    }

    /// Sets a new value on the specified entity if it would change.
    ///
    /// Returns the previous value if changed.
    pub fn set_if_neq(&mut self, c: &mut Commands, entity: Entity, new: T) -> Option<T>
    where
        T: PartialEq
    {
        let (_, mut x) = self.components.get_mut(entity).ok()?;
        (*x).set_if_neq(c, new)
    }

    /// Sets a new value on a single entity if it would change.
    ///
    /// Returns the previous value if changed.
    ///
    /// Panics if the inner query doesn't have exactly one entity.
    pub fn set_single_if_not_eq(&mut self, c: &mut Commands, new: T) -> (Entity, Option<T>)
    where
        T: PartialEq
    {
        let (e, mut x) = self.components.single_mut().unwrap();
        (e, (*x).set_if_neq(c, new))
    }
}

//-------------------------------------------------------------------------------------------------------------------
