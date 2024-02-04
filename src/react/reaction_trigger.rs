//local shortcuts
use crate::*;

//third-party shortcuts
use bevy::prelude::*;
use bevy::utils::all_tuples;

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------

/// Helper trait for registering reactors with [`ReactCommands`].
pub trait ReactionTrigger<I>
{
    /// Register a trigger with [`ReactCommands`].
    fn register(self,
        rcommands  : &mut ReactCommands,
        sys_handle : &AutoDespawnSignal,
    ) -> ReactorType;
}

impl<I, R: ReactionTrigger<I>> ReactionTriggerBundle<I> for R
{
    fn len(&self) -> usize { 1 }

    fn get_reactor_types(
            self,
            rcommands  : &mut ReactCommands,
            sys_handle : &AutoDespawnSignal,
            func       : &mut impl FnMut(ReactorType)
        )
    {
        func(self.register(rcommands, sys_handle));
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Helper trait for registering reactors with [`ReactCommands`].
///
/// All members of a trigger bundle must implement [`ReactionTriggerBundle`]. You should implement [`ReactionTrigger`]
/// on the root members of a bundle.
pub trait ReactionTriggerBundle<I>
{
    /// Get the number of triggers in the bundle
    fn len(&self) -> usize;

    /// Register reactors and pass the reactor types to the injected function.
    fn get_reactor_types(
            self,
            rcommands  : &mut ReactCommands,
            sys_handle : &AutoDespawnSignal,
            func       : &mut impl FnMut(ReactorType)
        );
}

//-------------------------------------------------------------------------------------------------------------------

pub fn reactor_registration<I>(
    rcommands  : &mut ReactCommands,
    sys_handle : &AutoDespawnSignal,
    triggers   : impl ReactionTriggerBundle<I>,
) -> RevokeToken
{
    let mut reactors = Vec::with_capacity(triggers.len());
    let mut func =
        |reactor_type: ReactorType|
        {
            reactors.push(reactor_type);
        };
    triggers.get_reactor_types(rcommands, sys_handle, &mut func);

    RevokeToken{ reactors: reactors.into(), id: sys_handle.entity().to_bits() }
}

//-------------------------------------------------------------------------------------------------------------------

// Implements [`ReactionTriggerBundle`] for tuples of `()`-input triggers.
macro_rules! tuple_impl
{
    ($($name: ident),*) =>
    {
        impl<$($name: ReactionTriggerBundle<()>),*> ReactionTriggerBundle<()> for ($($name,)*)
        {
            #[allow(unused_variables, unused_mut)]
            #[inline(always)]
            fn len(&self) -> usize
            {
                let mut len = 0;
                #[allow(non_snake_case)]
                let ($($name,)*) = self;
                $(
                    len += $name.len();
                )*

                len
            }

            #[allow(unused_variables, unused_mut)]
            #[inline(always)]
            fn get_reactor_types(
                self,
                rcommands  : &mut ReactCommands,
                sys_handle : &AutoDespawnSignal,
                func       : &mut impl FnMut(ReactorType)
            ){
                #[allow(non_snake_case)]
                let ($(mut $name,)*) = self;
                $(
                    $name.get_reactor_types(rcommands, sys_handle, &mut *func);
                )*
            }
        }
    }
}

all_tuples!(tuple_impl, 0, 15, B);

//-------------------------------------------------------------------------------------------------------------------

// Implements [`ReactionTriggerBundle`] for tuples of `Entity`-input triggers.
macro_rules! tuple_impl
{
    ($($name: ident),*) =>
    {
        impl<$($name: ReactionTriggerBundle<Entity>),*> ReactionTriggerBundle<Entity> for ($($name,)*)
        {
            #[allow(unused_variables, unused_mut)]
            #[inline(always)]
            fn len(&self) -> usize
            {
                let mut len = 0;
                #[allow(non_snake_case)]
                let ($($name,)*) = self;
                $(
                    len += $name.len();
                )*

                len
            }

            #[allow(unused_variables, unused_mut)]
            #[inline(always)]
            fn get_reactor_types(
                self,
                rcommands  : &mut ReactCommands,
                sys_handle : &AutoDespawnSignal,
                func       : &mut impl FnMut(ReactorType)
            ){
                #[allow(non_snake_case)]
                let ($(mut $name,)*) = self;
                $(
                    $name.get_reactor_types(rcommands, sys_handle, &mut *func);
                )*
            }
        }
    }
}

all_tuples!(tuple_impl, 0, 15, B);

//-------------------------------------------------------------------------------------------------------------------
