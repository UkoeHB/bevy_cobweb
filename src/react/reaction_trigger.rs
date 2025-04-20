//local shortcuts
use crate::prelude::*;

//third-party shortcuts
use bevy::ecs::system::Commands;
use bevy::prelude::*;
use smallvec::SmallVec;
use variadics_please::all_tuples;

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

/// Helper trait for [`EntityTriggerBundle`].
pub trait EntityTrigger: Copy + Clone + Send + Sync + 'static
{
    /// Sets the trigger entity.
    fn new_trigger(entity: Entity) -> Self;

    /// Gets the trigger's trigger entity.
    fn entity(&self) -> Entity;
}

impl<E: EntityTrigger> EntityTriggerBundle for E
{
    fn new_bundle(entity: Entity) -> Self
    {
        Self::new_trigger(entity)
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

/// Helper trait for registering reactors with [`EntityWorldReactor`].
///
/// All triggers in a bundle must implement `EntityTrigger`, and they must all reference the same entity.
pub trait EntityTriggerBundle
{
    /// Makes a new bundle from an entity.
    fn new_bundle(entity: Entity) -> Self;
}

//-------------------------------------------------------------------------------------------------------------------

/// Extracts reactor types from a [`ReactionTriggerBundle`].
pub fn get_reactor_types(bundle: impl ReactionTriggerBundle) -> SmallVec<[ReactorType; 10]>
{
    let mut reactors = SmallVec::<[ReactorType; 10]>::with_capacity(bundle.len());
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

// Implements [`EntityTriggerBundle`] for tuples of entity triggers.
macro_rules! tuple_impl
{
    ($($name: ident),*) =>
    {
        impl<$($name: EntityTriggerBundle),*> EntityTriggerBundle for ($($name,)*)
        {
            #[allow(unused_variables, unused_mut)]
            #[inline(always)]
            fn new_bundle(entity: Entity) -> Self
            {
                #[allow(non_snake_case)]
                ($(
                    $name::new_bundle(entity),
                )*)
            }
        }
    }
}

all_tuples!(tuple_impl, 1, 15, B);

//-------------------------------------------------------------------------------------------------------------------
