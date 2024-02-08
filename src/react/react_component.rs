//local shortcuts
use crate::*;

//third-party shortcuts
use bevy::ecs::system::CommandQueue;
use bevy::prelude::*;

//standard shortcuts
use core::ops::Deref;
use std::vec::Vec;

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
    /// Mutably access the component and trigger reactions.
    pub fn get_mut<'a>(&'a mut self, rcommands: &mut ReactCommands) -> &'a mut C
    {
        rcommands.cache.react_to_mutation::<C>(&mut rcommands.commands, &mut rcommands.react_queue, self.entity);
        &mut self.component
    }

    /// Mutably access the component without triggering reactions.
    pub fn get_mut_noreact(&mut self) -> &mut C
    {
        &mut self.component
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
