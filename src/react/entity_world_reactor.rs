//local shortcuts
use crate::prelude::*;

//third-party shortcuts
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

//standard shortcuts
use std::any::type_name;
use std::marker::PhantomData;

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn cleanup_reactor_data<T: EntityWorldReactor>(
    In((id, entity)): In<(SystemCommand, Entity)>,
    mut commands: Commands,
    entities: Query<&EntityReactors>,
){
    let Ok(reactor) = entities.get(entity) else { return };
    if reactor.iter_reactors().find(|reactor_id| *reactor_id == id).is_some() { return }
    commands.entity(entity).remove::<EntityWorldLocal<T>>();
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

#[derive(Resource)]
pub(crate) struct EntityWorldReactorRes<T: EntityWorldReactor>
{
    sys_command: SystemCommand,
    p: PhantomData<T>,
}

impl<T: EntityWorldReactor> EntityWorldReactorRes<T>
{
    pub(crate) fn new(sys_command: SystemCommand) -> Self
    {
        Self{ sys_command, p: PhantomData::default() }
    }
}

//-------------------------------------------------------------------------------------------------------------------

#[derive(Component)]
pub(crate) struct EntityWorldLocal<T: EntityWorldReactor>
{
    data: T::Local,
}

impl<T: EntityWorldReactor> EntityWorldLocal<T>
{
    fn new(data: T::Local) -> Self
    {
        Self{ data }
    }

    pub(crate) fn inner(&self) -> &T::Local
    {
        &self.data
    }

    pub(crate) fn inner_mut(&mut self) -> &mut T::Local
    {
        &mut self.data
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Trait for persistent reactors that are registered in the world.
///
/// These are 'entity' reactors which means trigger bundles are registered for specific entities. Only trigger
/// bundles that implement [`EntityTriggerBundle`] can be used.
///
/// This reactor type includes [`Self::Local`], which allows data to be tied to a specific entity for this reactor.
/// When the reactor runs, the [`EntityLocal`] system param can be used to access data for the trigger entity.
///
/// The reactor can be accessed with the [`EntityReactor`] system param.
///
/// Example:
/**
```no_run
#[derive(ReactComponent, Debug)]
struct A;

struct MyReactor;
impl EntityWorldReactor for MyReactor
{
    type Triggers = EntityMutationTrigger<A>;
    type Data = String;

    fn reactor(self) -> SystemCommandCallback
    {
        SystemCommandCallback::new(
            |data: EntityLocal<Self>, components: Reactive<A>|
            {
                let (entity, data) = data.get();
                let a = components.get(entity).unwrap();
                println!("New value of A on entity {:?}: {:?}", data, a);
            }
        )
    }
}

struct AddReactorPlugin;
impl Plugin for AddReactorPlugin
{
    fn build(&mut self)
    {
        self.add_entity_reactor(MyReactor);
    }
}
```
*/
pub trait EntityWorldReactor: Send + Sync + 'static
{
    /// Triggers that can be added for an entity with [`EntityReactor::add`].
    ///
    /// The trigger bundle must implement [`EntityTriggerBundle`], which must have at least one entry.
    type Triggers: EntityTriggerBundle + ReactionTriggerBundle;
    /// Data that is 'local' to a specific entity that triggers the reactor.
    type Local: Send + Sync + 'static;

    /// Consumes `Self` and returns the reactor system.
    ///
    /// Use [`SystemCommandCallback::new`] to construct the return value from your reactor system.
    fn reactor(self) -> SystemCommandCallback;
}

//-------------------------------------------------------------------------------------------------------------------

/// System parameter for accessing and updating an [`EntityWorldReactor`].
#[derive(SystemParam)]
pub struct EntityReactor<'w, T: EntityWorldReactor>
{
    inner: Option<ResMut<'w, EntityWorldReactorRes<T>>>,
}

impl<'w, T: EntityWorldReactor> EntityReactor<'w, T>
{
    /// Adds a listener to the reactor.
    ///
    /// Returns `false` if:
    /// - The reactor doesn't exist.
    /// - The trigger entity doesn't exist.
    pub fn add(&self, c: &mut Commands, trigger_entity: Entity, data: T::Local) -> bool
    {
        let Some(inner) = &self.inner
        else
        {
            tracing::warn!("failed adding listener, entity world reactor {:?} is missing; add it to your app with \
                ReactAppExt::add_world_reactor", type_name::<T>());
            return false;
        };

        let Ok(mut ec) = c.get_entity(trigger_entity) else { return false };
        ec.try_insert(EntityWorldLocal::<T>::new(data));

        let triggers = <T as EntityWorldReactor>::Triggers::new_bundle(trigger_entity);
        c.react().with(triggers, inner.sys_command, ReactorMode::Persistent);
        true
    }

    /// Removes triggers from the reactor.
    ///
    /// Note that registered data will be removed from an entity when all its entity-specific triggers for this
    /// reactor have been removed.
    /// It is possible for this method to race with parallel systems that re-add entities referenced by the
    /// removal bundle.
    ///
    /// Returns `false` if the reactor doesn't exist.
    pub fn remove(&self, c: &mut Commands, triggers: impl ReactionTriggerBundle) -> bool
    {
        let Some(inner) = &self.inner
        else
        {
            tracing::warn!("failed removing triggers, entity world reactor {:?} is missing; add it to your app with \
                ReactAppExt::add_world_reactor", type_name::<T>());
            return false;
        };

        let token = RevokeToken::new_from(inner.sys_command, triggers);
        c.react().revoke(token.clone());

        // Remove the reactor data from entities that no longer track this reactor.
        for entity in token.iter_unique_entities()
        {
            c.syscall((token.id, entity), cleanup_reactor_data::<T>);
        }

        true
    }

    /// Gets the reactor's system command.
    ///
    /// Returns `None` if the reactor doesn't exist.
    // Note: This is `crate` visibility so the inner system command can't be accessed easily, since doing so is a danger
    // zone for bugs.
    pub(crate) fn system(&self) -> Option<SystemCommand>
    {
        let Some(inner) = &self.inner
        else
        {
            tracing::warn!("failed accessing entity world reactor {:?} because it is missing; add it to your app with \
                ReactAppExt::add_entity_reactor", type_name::<T>());
            return None;
        };

        Some(inner.sys_command)
    }
}

//-------------------------------------------------------------------------------------------------------------------
