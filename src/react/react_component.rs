//local shortcuts
use crate::prelude::*;

//third-party shortcuts
use bevy::prelude::*;
use bevy::ecs::system::SystemParam;

//standard shortcuts
use core::ops::Deref;

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

    /// Mutably accesses the component without triggering reactions.
    pub fn get_noreact(&mut self) -> &mut C
    {
        &mut self.component
    }

    /// Sets the component value and triggers mutations only if the value will change.
    ///
    /// Returns the previous value if it changed.
    pub fn set_if_not_eq(&mut self, c: &mut Commands, new: C) -> Option<C>
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
    components: Query<'w, 's, &'static React<T>>,
}

impl<'w, 's, T: ReactComponent> Reactive<'w, 's, T>
{
    /// Reads `T` on `entity`.
    ///
    /// Does not trigger reactions.
    pub fn get(&self, entity: Entity) -> Option<&T>
    {
        self.components.get(entity).ok().map(React::get)
    }

    /// Reads `T` on a single entity.
    ///
    /// Does not trigger reactions.
    ///
    /// Panics if the inner query is empty.
    pub fn single(&self) -> &T
    {
        self.components.single().get()
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// System parameter for accessing [`React<T>`] components mutably.
///
/// See [`Reactive`] for the immutable version.
#[derive(SystemParam)]
pub struct ReactiveMut<'w, 's, T: ReactComponent>
{
    components: Query<'w, 's, &'static mut React<T>>,
}

impl<'w, 's, T: ReactComponent> ReactiveMut<'w, 's, T>
{
    /// Reads `T` on `entity`.
    ///
    /// Does not trigger reactions.
    pub fn get(&self, entity: Entity) -> Option<&T>
    {
        self.components.get(entity).ok().map(React::get)
    }

    /// Reads `T` on a single entity.
    ///
    /// Does not trigger reactions.
    ///
    /// Panics if the inner query is empty.
    pub fn single(&self) -> &T
    {
        self.components.single().get()
    }

    /// Gets a mutable reference to `T` on `entity`.
    ///
    /// Triggers mutation reactions.
    pub fn get_mut(&mut self, c: &mut Commands, entity: Entity) -> Option<&mut T>
    {
        let x = self.components.get_mut(entity).ok()?;
        Some(x.into_inner().get_mut(c))
    }

    /// Gets a mutable reference to `T` on a single entity.
    ///
    /// Triggers mutation reactions.
    ///
    /// Panics if the inner query is empty.
    pub fn single_mut(&mut self, c: &mut Commands) -> &mut T
    {
        let x = self.components.single_mut();
        x.into_inner().get_mut(c)
    }

    /// Gets a mutable reference to `T` on `entity`.
    ///
    /// Does not trigger reactions.
    pub fn get_noreact(&mut self, entity: Entity) -> Option<&mut T>
    {
        let x = self.components.get_mut(entity).ok()?;
        Some(x.into_inner().get_noreact())
    }

    /// Gets a mutable reference to `T` on a single entity
    ///
    /// Does not trigger reactions.
    ///
    /// Panics if the inner query is empty.
    pub fn single_noreact(&mut self) -> &mut T
    {
        let x = self.components.single_mut();
        x.into_inner().get_noreact()
    }
}

//-------------------------------------------------------------------------------------------------------------------
