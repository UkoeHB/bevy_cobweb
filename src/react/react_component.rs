//local shortcuts
use crate::prelude::*;

//third-party shortcuts
use bevy::prelude::*;

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
    /// Mutably accesses the component and triggers reactions.
    pub fn get_mut<'a>(&'a mut self, rcommands: &mut ReactCommands) -> &'a mut C
    {
        rcommands.commands.syscall(self.entity, ReactCache::schedule_mutation_reaction::<C>);
        &mut self.component
    }

    /// Mutably accesses the component without triggering reactions.
    pub fn get_mut_noreact(&mut self) -> &mut C
    {
        &mut self.component
    }

    /// Sets the component value and triggers mutations only if the value will change.
    ///
    /// Returns the previous value if it changed.
    pub fn set_if_not_eq(&mut self, rcommands: &mut ReactCommands, new: C) -> Option<C>
    where
        C: PartialEq
    {
        if new == self.component { return None; }

        rcommands.commands.syscall(self.entity, ReactCache::schedule_mutation_reaction::<C>);
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
