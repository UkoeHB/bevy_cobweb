//local shortcuts
use crate::prelude::*;

//third-party shortcuts
use bevy::ecs::system::Commands;
use bevy::utils::all_tuples;

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------

/// Helper trait for registering reactors with [`ReactCommands`].
pub trait ReactionTrigger
{
    /// Register a trigger with [`ReactCommands`].
    fn register(self, commands: &mut Commands, handle: &ReactorHandle) -> Option<ReactorType>;
}

impl<R: ReactionTrigger> ReactionTriggerBundle for R
{
    fn len(&self) -> usize { 1 }

    fn get_reactor_types(
            self,
            func     : &mut impl FnMut(Option<ReactorType>),
            commands : &mut Commands,
            handle   : &ReactorHandle,
        )
    {
        func(self.register(commands, handle));
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Helper trait for registering reactors with [`ReactCommands`].
///
/// All members of a trigger bundle must implement [`ReactionTriggerBundle`]. You should implement [`ReactionTrigger`]
/// on the root members of a bundle.
pub trait ReactionTriggerBundle
{
    /// Get the number of triggers in the bundle
    fn len(&self) -> usize;

    /// Register reactors and pass the reactor types to the injected function.
    fn get_reactor_types(
            self,
            func     : &mut impl FnMut(Option<ReactorType>),
            commands : &mut Commands,
            handle   : &ReactorHandle,
        );
}

//-------------------------------------------------------------------------------------------------------------------

pub fn reactor_registration(
    commands : &mut Commands,
    handle   : &ReactorHandle,
    triggers : impl ReactionTriggerBundle,
    mode     : ReactorMode,
) -> Option<RevokeToken>
{
    match mode
    {
        ReactorMode::Persistent |
        // note: cleanup is handled automatically by the ReactorHandle type
        ReactorMode::Cleanup =>
        {
            let mut func = |_| {};
            triggers.get_reactor_types(&mut func, commands, handle);

            None
        }
        ReactorMode::Revokable =>
        {
            let mut reactors = Vec::with_capacity(triggers.len());
            let mut func =
                |reactor_type: Option<ReactorType>|
                {
                    let Some(reactor_type) = reactor_type else { return; };
                    reactors.push(reactor_type);
                };
            triggers.get_reactor_types(&mut func, commands, handle);

            Some(RevokeToken{ reactors: reactors.into(), id: handle.sys_command() })
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------

// Implements [`ReactionTriggerBundle`] for tuples of triggers.
macro_rules! tuple_impl
{
    ($($name: ident),*) =>
    {
        impl<$($name: ReactionTriggerBundle),*> ReactionTriggerBundle for ($($name,)*)
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
                func     : &mut impl FnMut(Option<ReactorType>),
                commands : &mut Commands,
                handle   : &ReactorHandle,
            ){
                #[allow(non_snake_case)]
                let ($(mut $name,)*) = self;
                $(
                    $name.get_reactor_types(&mut *func, commands, handle);
                )*
            }
        }
    }
}

all_tuples!(tuple_impl, 0, 15, B);

//-------------------------------------------------------------------------------------------------------------------
