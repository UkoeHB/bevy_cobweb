//local shortcuts
use crate::prelude::*;

//third-party shortcuts
use bevy::ecs::system::Commands;
use bevy::utils::all_tuples;

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------

/// Helper trait for registering reactors with [`ReactCommands`].
pub trait ReactionTrigger: Copy + Clone + Send + Sync + 'static
{
    /// Gets the trigger's [`ReactorType`].
    fn reactor_type(&self) -> ReactorType;

    /// Register a trigger with [`ReactCommands`].
    fn register(&self, commands: &mut Commands, handle: &ReactorHandle);
}

impl<R: ReactionTrigger> ReactionTriggerBundle for R
{
    fn len(&self) -> usize { 1 }

    fn collect_reactor_types(self, func: &mut impl FnMut(ReactorType))
    {
        func(self.reactor_type());
    }

    fn register_triggers(self, commands: &mut Commands, handle: &ReactorHandle)
    {
        self.register(commands, handle);
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Helper trait for registering reactors with [`ReactCommands`].
///
/// All members of a trigger bundle must implement [`ReactionTriggerBundle`]. You should implement [`ReactionTrigger`]
/// on the root members of a bundle.
pub trait ReactionTriggerBundle: Copy + Clone + Send + Sync + 'static
{
    /// Gets the number of triggers in the bundle
    fn len(&self) -> usize;

    /// Traverses reactor types in the bundle.
    fn collect_reactor_types(self, func: &mut impl FnMut(ReactorType));

    /// Registers reactors and passes the reactor types to the injected function.
    fn register_triggers(
            self,
            commands : &mut Commands,
            handle   : &ReactorHandle,
        );
}

//-------------------------------------------------------------------------------------------------------------------

/// Extracts reactor types from a [`ReactionTriggerBundle`].
pub fn get_reactor_types(bundle: impl ReactionTriggerBundle) -> Vec<ReactorType>
{
    let mut reactors = Vec::with_capacity(bundle.len());
    let mut func =
        |reactor_type: ReactorType|
        {
            reactors.push(reactor_type);
        };
    bundle.collect_reactor_types(&mut func);
    reactors
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
            fn collect_reactor_types(self, func: &mut impl FnMut(ReactorType))
            {
                #[allow(non_snake_case)]
                let ($(mut $name,)*) = self;
                $(
                    $name.collect_reactor_types(&mut *func);
                )*
            }

            #[allow(unused_variables, unused_mut)]
            #[inline(always)]
            fn register_triggers(
                self,
                commands : &mut Commands,
                handle   : &ReactorHandle,
            ){
                #[allow(non_snake_case)]
                let ($(mut $name,)*) = self;
                $(
                    $name.register_triggers(commands, handle);
                )*
            }
        }
    }
}

all_tuples!(tuple_impl, 0, 15, B);

//-------------------------------------------------------------------------------------------------------------------
