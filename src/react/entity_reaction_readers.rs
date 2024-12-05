//local shortcuts
use crate::prelude::*;

//third-party shortcuts
//use bevy::ecs::component::ComponentId;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

//standard shortcuts
use std::any::{type_name, TypeId};
use std::marker::PhantomData;

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

//todo: switch to ComponentId when observers are implemented
//(cannot do so yet because component ids are not available when reactions are triggered, only type ids)
struct ReactComponentId<T: ReactComponent>
{
    //id: ComponentId,
    id: TypeId,
    p: PhantomData<T>,
}

impl<T: ReactComponent> ReactComponentId<T>
{
    fn id(&self) -> TypeId
    {
        self.id
    }
}

impl<T: ReactComponent> FromWorld for ReactComponentId<T>
{
    fn from_world(_world: &mut World) -> Self
    {
        Self{
            //id: world.components().get_id(std::any::TypeId::of::<React<T>>()),
            id: TypeId::of::<T>(),
            p: PhantomData::default(),
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// Tracks metadata for accessing entity reactions (entity events use [`EventAccessTracker`] instead).
#[derive(Resource)]
pub(crate) struct EntityReactionAccessTracker
{
    /// True when in a system reacting to an entity reaction.
    currently_reacting: bool,
    /// The system command that is running the current entity reaction.
    system: SystemCommand,
    /// The source of the most recent entity reaction.
    reaction_source: Entity,
    /// The type of the most recent entity reaction trigger.
    reaction_type: EntityReactionType,

    /// Reaction information cached for when the reaction system actually runs.
    prepared: Vec<(SystemCommand, Entity, EntityReactionType)>,
}

impl EntityReactionAccessTracker
{
    /// Caches metadata for an entity reaction.
    pub(crate) fn prepare(&mut self, system: SystemCommand, source: Entity, reaction: EntityReactionType)
    {
        self.prepared.push((system, source, reaction));
    }

    /// Sets metadata for the current entity reaction.
    pub(crate) fn start(&mut self, reactor: SystemCommand)
    {
        let Some(pos) = self.prepared.iter().position(|(s, _, _)| *s == reactor) else {
            tracing::error!("prepared entity reaction is missing {:?}", reactor);
            debug_assert!(false);
            return;
        };
        let (system, source, reaction) = self.prepared.swap_remove(pos);

        debug_assert!(!self.currently_reacting);
        self.currently_reacting = true;
        self.system = system;
        self.reaction_source = source;
        self.reaction_type = reaction;
    }

    /// Unsets the 'is reacting' flag.
    pub(crate) fn end(&mut self)
    {
        self.currently_reacting = false;
    }

    /// Returns `true` if an entity reaction is currently being processed.
    fn is_reacting(&self) -> bool
    {
        self.currently_reacting
    }

    /// Returns the system running the entity reaction.
    fn system(&self) -> SystemCommand
    {
        self.system
    }

    /// Returns the source of the most recent entity reaction.
    fn source(&self) -> Entity
    {
        self.reaction_source
    }

    /// Returns the [`EntityReactionType`] of the most recent entity reaction.
    fn reaction_type(&self) -> EntityReactionType
    {
        self.reaction_type
    }
}

impl Default for EntityReactionAccessTracker
{
    fn default() -> Self
    {
        Self{
            currently_reacting: false,
            system: SystemCommand(Entity::PLACEHOLDER),
            reaction_source: Entity::PLACEHOLDER,
            reaction_type: EntityReactionType::Insertion(TypeId::of::<()>()),
            prepared: Vec::default(),
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// System parameter for reading entity component insertion events in systems that react to those events.
///
/// Can only be used within [`SystemCommands`](super::SystemCommand).
///
/// Use [`entity_insertion`] or [`insertion`] to make a trigger that will read these events.
///
/*
```rust
fn example(mut c: Commands)
{
    let entity = c.spawn_empty().id();
    c.react().on(
        insertion::<A>(),  // entity-specific: entity_insertion::<A>(target_entity)
        |event: InsertionEvent<A>|
        {
            let entity = event.get()?;
            println!("'A' was inserted to {:?}", entity);
            DONE
        }
    );

    rcommands.insert(entity, A);
}
```
*/
#[derive(SystemParam)]
pub struct InsertionEvent<'w, 's, T: ReactComponent>
{
    component_id: Local<'s, ReactComponentId<T>>,
    tracker: Res<'w, EntityReactionAccessTracker>,
}

impl<'w, 's, T: ReactComponent> InsertionEvent<'w, 's, T>
{
    /// Returns the entity that received a `React<T>` component insertion that the system is reacting to.
    ///
    /// This will return at most one unique entity each time a reactor runs.
    ///
    /// Panics if the system is not reacting to an insertion event for `T`.
    pub fn entity(&self) -> Entity
    {
        self.get()
            .unwrap_or_else(|_| panic!("failed reading insertion event for {}, there is no event", type_name::<T>()))
    }

    /// See [`Self::entity`].
    pub fn get(&self) -> Result<Entity, ()>
    {
        if !self.tracker.is_reacting() { return Err(()); }
        let EntityReactionType::Insertion(component_id) = self.tracker.reaction_type() else { return Err(()); };
        if component_id != self.component_id.id() { return Err(()); }

        Ok(self.tracker.source())
    }

    /// Returns `true` if there is nothing to read.
    ///
    /// Equivalent to `event.get().is_ok()`.
    pub fn is_empty(&self) -> bool
    {
        self.get().is_err()
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// System parameter for reading entity component mutation events in systems that react to those events.
///
/// Can only be used within [`SystemCommands`](super::SystemCommand).
///
/// Use [`entity_mutation`] or [`mutation`] to make a trigger that will read these events.
///
/*
```rust
fn example(mut c: Commands, query: Query<&mut React<A>>)
{
    c.react().on(
        mutation::<A>(),  // entity-specific: entity_mutation::<A>(target_entity)
        |event: MutationEvent<A>|
        {
            let entity = event.get()?;
            println!("'A' was mutated on {:?}", entity);
            DONE
        }
    );

    query.single_mut().get_mut(&mut rcommands);  //triggers mutation reactions
}
```
*/
#[derive(SystemParam)]
pub struct MutationEvent<'w, 's, T: ReactComponent>
{
    component_id: Local<'s, ReactComponentId<T>>,
    tracker: Res<'w, EntityReactionAccessTracker>,
}

impl<'w, 's, T: ReactComponent> MutationEvent<'w, 's, T>
{
    /// Returns the entity on which a `React<T>` component was mutated that the system is reacting to.
    ///
    /// This will return at most one unique entity each time a reactor runs.
    ///
    /// Panics if the system is not reacting to a mutation event for `T`.
    pub fn entity(&self) -> Entity
    {
        self.get()
            .unwrap_or_else(|_| panic!("failed reading mutation event for {}, there is no event", type_name::<T>()))
    }

    /// See [`Self::entity`].
    pub fn get(&self) -> Result<Entity, ()>
    {
        if !self.tracker.is_reacting() { return Err(()); }
        let EntityReactionType::Mutation(component_id) = self.tracker.reaction_type() else { return Err(()); };
        if component_id != self.component_id.id() { return Err(()); }

        Ok(self.tracker.source())
    }

    /// Returns `true` if there is nothing to read.
    ///
    /// Equivalent to `event.get().is_ok()`.
    pub fn is_empty(&self) -> bool
    {
        self.get().is_err()
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// System parameter for reading entity component removal events in systems that react to those events.
///
/// Note that removals are detected for entity despawns, so if the entity returned from `RemovalEvent` does not
/// exist that implies that it was removed due to a despawn (although not a guarantee, since it could have been removed
/// and the entity despawned later).
///
/// Use [`entity_removal`] or [`removal`] to make a trigger that will read these events.
///
/*
```rust
fn example(mut c: Commands, query: Query<Entity, With<React<A>>>)
{
    c.react().on(
        removal::<A>(),  // entity-specific: entity_removal::<A>(target_entity)
        |event: RemovalEvent<A>|
        {
            let entity = event.get()?;
            println!("'A' was removed from {:?}", entity);
            DONE
        }
    );

    c.entity(*query.single()).remove::<A>();
}
```
*/
#[derive(SystemParam)]
pub struct RemovalEvent<'w, 's, T: ReactComponent>
{
    component_id: Local<'s, ReactComponentId<T>>,
    tracker: Res<'w, EntityReactionAccessTracker>,
}

impl<'w, 's, T: ReactComponent> RemovalEvent<'w, 's, T>
{
    /// Returns the entity from which a `React<T>` component was removed that the system is reacting to.
    ///
    /// This will return at most one unique entity each time a reactor runs.
    ///
    /// Panics if the system is not reacting to a removal event for `T`.
    pub fn entity(&self) -> Entity
    {
        self.get()
            .unwrap_or_else(|_| panic!("failed reading removal event for {}, there is no event", type_name::<T>()))
    }

    /// See [`Self::entity`].
    pub fn get(&self) -> Result<Entity, ()>
    {
        if !self.tracker.is_reacting() { return Err(()); }
        let EntityReactionType::Removal(component_id) = self.tracker.reaction_type() else { return Err(()); };
        if component_id != self.component_id.id() { return Err(()); }

        Ok(self.tracker.source())
    }

    /// Returns `true` if there is nothing to read.
    ///
    /// Equivalent to `event.get().is_ok()`.
    pub fn is_empty(&self) -> bool
    {
        self.get().is_err()
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// System parameter for reading entity-specific data for [`EntityWorldReactor`] reactors.
///
/*
```rust
#[derive(ReactComponent)]
struct MyComponent(Duration);

struct MyReactor;

impl EntityWorldReactor for MyReactor
{
    type Triggers = EntityMutationTrigger::<MyComponent>;
    type Local = String;

    fn reactor() -> SystemCommandCallback
    {
        SystemCommandCallback::new(
            |data: EntityLocal<MyReactor>, components: Reactive<MyComponent>|
            {
                let (entity, data) = data.get();
                let Some(component) = components.get(entity) else { return };
                println!("Entity {:?} now has {:?}", data, component);
            }
        )
    }
}

fn prep_entity(mut c: Commands, reactor: EntityReactor<MyReactor>)
{
    let entity = c.spawn(MyComponent(Duration::default()));
    reactor.add(&mut c, entity, "ClockTracker");
}

fn update_entity(mut commands: Commands, time: Res<Time>, mut components: ReactiveMut<MyComponent>)
{
    let elapsed = time.elapsed();
    let component = components.single_mut(&mut c);
    component.0 = elapsed;
}

struct ExamplePlugin;
impl Plugin for ExamplePlugin
{
    fn build(&self, app: &mut App)
    {
        app.add_entity_reactor::<MyReactor>()
            .add_systems(Setup, prep_entity)
            .add_systems(Update, update_entity);
    }
}
```
*/
#[derive(SystemParam)]
pub struct EntityLocal<'w, 's, T: EntityWorldReactor>
{
    reactor: EntityReactor<'w, T>,
    tracker: Res<'w, EntityReactionAccessTracker>,
    data: Query<'w, 's, &'static mut EntityWorldLocal<T>>,
}

impl<'w, 's, T: EntityWorldReactor> EntityLocal<'w, 's, T>
{
    /// Gets the current entity.
    ///
    /// Panics if not called from within an [`EntityWorldReactor`] system.
    pub fn entity(&self) -> Entity
    {
        self.check();
        self.tracker.source()
    }

    /// Gets the current entity's local data.
    ///
    /// Panics if not called from within an [`EntityWorldReactor`] system.
    pub fn get(&self) -> (Entity, &T::Local)
    {
        self.check();
        (
            self.tracker.source(),
            self.data.get(self.tracker.source()).expect("entity missing local data in EntityLocal").inner()
        )
    }

    /// Gets the current entity's local data.
    ///
    /// Panics if not called from within an [`EntityWorldReactor`] system.
    pub fn get_mut(&mut self) -> (Entity, &mut T::Local)
    {
        self.check();
        (
            self.tracker.source(),
            self.data.get_mut(self.tracker.source())
                .expect("entity missing local data in EntityLocal")
                .into_inner()
                .inner_mut()
        )
    }

    fn check(&self)
    {
        if !self.tracker.is_reacting()
        {
            panic!("EntityLocal should only be used in an EntityWorldReactor");
        }
        if self.tracker.system() != self.reactor.system().expect("EntityLocal should only be used in an EntityWorldReactor")
        {
            panic!("EntityLocal should only be used in an EntityWorldReactor");
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------
