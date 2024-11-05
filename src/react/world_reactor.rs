//local shortcuts
use crate::prelude::*;

//third-party shortcuts
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

//standard shortcuts
use std::any::type_name;
use std::marker::PhantomData;

//-------------------------------------------------------------------------------------------------------------------

#[derive(Resource)]
pub(crate) struct WorldReactorRes<T: WorldReactor>
{
    sys_command: SystemCommand,
    p: PhantomData<T>,
}

impl<T: WorldReactor> WorldReactorRes<T>
{
    pub(crate) fn new(sys_command: SystemCommand) -> Self
    {
        Self{ sys_command, p: PhantomData::default() }
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Trait for persistent reactors that are registered in the world.
///
/// Reactors with no starting triggers are registered with [`ReactAppExt::add_world_reactor`].
/// Reactors with starting triggers are registered with [`ReactAppExt::add_world_reactor_with`].
/// Reactors with *only* starting triggers can be registered with [`ReactAppExt::add_reactor`].
///
/// The reactor can be accessed with the [`Reactor`] system param.
///
/// Example:
/**
```no_run
#[derive(ReactComponent, Debug)]
struct A;

struct MyReactor;
impl WorldReactor for MyReactor
{
    type StartingTriggers = ();
    type Triggers = EntityMutationTrigger<A>;
    fn reactor(self) -> SystemCommandCallback
    {
        SystemCommandCallback::new(
            |event: MutationEvent<A>, query: Query<&React<A>>|
            {
                let entity = event.unwrap();
                let a = query.get(entity).unwrap();
                println!("New value of A on entity {:?}: {:?}", entity, a);
            }
        )
    }
}

struct AddReactorPlugin;
impl Plugin for AddReactorPlugin
{
    fn build(&mut self)
    {
        self.add_world_reactor(MyReactor);
    }
}
```
*/
pub trait WorldReactor: Send + Sync + 'static
{
    /// Triggers that must be added when adding the reactor to your app with [`ReactAppExt::add_world_reactor_with].
    type StartingTriggers: ReactionTriggerBundle;
    /// Triggers that can be added to the reactor with [`Reactor::add`].
    type Triggers: ReactionTriggerBundle;

    /// Consumes `Self` and returns the reactor system.
    ///
    /// Use [`SystemCommandCallback::new`] to construct the return value from your reactor system.
    fn reactor(self) -> SystemCommandCallback;
}

//-------------------------------------------------------------------------------------------------------------------

/// System parameter for accessing and updating a [`WorldReactor`].
#[derive(SystemParam)]
pub struct Reactor<'w, T: WorldReactor>
{
    inner: Option<Res<'w, WorldReactorRes<T>>>,
}

impl<'w, T: WorldReactor> Reactor<'w, T>
{
    /// Adds starting triggers to the reactor.
    ///
    /// Returns `false` if the reactor doesn't exist.
    pub(crate) fn add_starting_triggers(&self, c: &mut Commands, triggers: T::StartingTriggers) -> bool
    {
        let Some(inner) = &self.inner
        else
        {
            tracing::warn!("failed adding starting triggers, world reactor {:?} is missing; add it to your app with \
                ReactAppExt::add_world_reactor", type_name::<T>());
            return false;
        };

        c.react().with(triggers, inner.sys_command, ReactorMode::Persistent);
        true
    }

    /// Adds triggers to the reactor.
    ///
    /// Returns `false` if the reactor doesn't exist.
    pub fn add(&self, c: &mut Commands, triggers: T::Triggers) -> bool
    {
        let Some(inner) = &self.inner
        else
        {
            tracing::warn!("failed adding triggers, world reactor {:?} is missing; add it to your app with \
                ReactAppExt::add_world_reactor", type_name::<T>());
            return false;
        };

        c.react().with(triggers, inner.sys_command, ReactorMode::Persistent);
        true
    }

    /// Removes triggers from the reactor.
    ///
    /// Returns `false` if the reactor doesn't exist.
    pub fn remove(&self, c: &mut Commands, triggers: impl ReactionTriggerBundle) -> bool
    {
        let Some(inner) = &self.inner
        else
        {
            tracing::warn!("failed removing triggers, world reactor {:?} is missing; add it to your app with \
                ReactAppExt::add_world_reactor", type_name::<T>());
            return false;
        };

        let token = RevokeToken::new_from(inner.sys_command, triggers);
        c.react().revoke(token);
        true
    }

    /// Manually runs the reactor as a system command.
    ///
    /// Returns `false` if the reactor doesn't exist.
    pub fn run(&self, commands: &mut Commands) -> bool
    {
        let Some(inner) = &self.inner
        else
        {
            tracing::warn!("failed running world reactor {:?} because it is missing; add it to your app with \
                ReactAppExt::add_world_reactor", type_name::<T>());
            return false;
        };

        commands.queue(inner.sys_command);
        true
    }
}

//-------------------------------------------------------------------------------------------------------------------
