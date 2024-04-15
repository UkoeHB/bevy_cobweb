//local shortcuts
use crate::prelude::*;

//third-party shortcuts
//use bevy::ecs::component::ComponentId;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use itertools::Either;

//standard shortcuts
use std::any::TypeId;
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
}

impl EntityReactionAccessTracker
{
    /// Sets metadata for the current entity reaction.
    pub(crate) fn start(&mut self, system: SystemCommand, source: Entity, reaction: EntityReactionType)
    {
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
            if let Some(entity) = event.read()
            {
                println!("'A' was inserted to {:?}", entity);
            }
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
    /// Returns the entity that received a `React<T>` component insertion if the current system is
    /// reacting to that insertion.
    ///
    /// This will return at most one unique entity each time a reactor runs.
    pub fn read(&self) -> Option<Entity>
    {
        if !self.tracker.is_reacting() { return None; }
        let EntityReactionType::Insertion(component_id) = self.tracker.reaction_type() else { return None; };
        if component_id != self.component_id.id() { return None; }

        Some(self.tracker.source())
    }

    /// Returns `true` if there is nothing to read.
    ///
    /// Equivalent to `event.read().is_none()`.
    pub fn is_empty(&self) -> bool
    {
        self.read().is_none()
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
            if let Some(entity) = event.read()
            {
                println!("'A' was mutated on {:?}", entity);
            }
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
    /// Returns the entity on which a `React<T>` component was mutated if the current system is
    /// reacting to that mutation.
    ///
    /// This will return at most one unique entity each time a reactor runs.
    pub fn read(&self) -> Option<Entity>
    {
        if !self.tracker.is_reacting() { return None; }
        let EntityReactionType::Mutation(component_id) = self.tracker.reaction_type() else { return None; };
        if component_id != self.component_id.id() { return None; }

        Some(self.tracker.source())
    }

    /// Returns `true` if there is nothing to read.
    ///
    /// Equivalent to `event.read().is_none()`.
    pub fn is_empty(&self) -> bool
    {
        self.read().is_none()
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
            if let Some(entity) = event.read()
            {
                println!("'A' was removed from {:?}", entity);
            }
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
    /// Returns the entity from which a `React<T>` component was removed if the current system is
    /// reacting to that removal.
    ///
    /// This will return at most one unique entity each time a reactor runs.
    pub fn read(&self) -> Option<Entity>
    {
        if !self.tracker.is_reacting() { return None; }
        let EntityReactionType::Removal(component_id) = self.tracker.reaction_type() else { return None; };
        if component_id != self.component_id.id() { return None; }

        Some(self.tracker.source())
    }

    /// Returns `true` if there is nothing to read.
    ///
    /// Equivalent to `event.read().is_none()`.
    pub fn is_empty(&self) -> bool
    {
        self.read().is_none()
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// System parameter for reading entity-specific data for [`EntityWorlReactor`] reactors.
///
/*
```rust
#[derive(ReactComponent)]
struct MyComponent(Duration);

struct MyReactor;

impl EntityWorldReactor for MyReactor
{
    type StartingTriggers = ();
    type Triggers = EntityMutation::<MyComponent>;
    type Data = String;

    fn reactor() -> SystemCommandCallback
    {
        SystemCommandCallback::new(
            |data: ReactorData<MyReactor>, components: Reactive<MyComponent>|
            {
                for (entity, data) in data.iter()
                {
                    let Some(component) = components.get(entity) else { continue };
                    println!("Entity {:?} now has {:?}", data, component);
                }
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
pub struct ReactorData<'w, 's, T: EntityWorldReactor>
{
    reactor: EntityReactor<'w, T>,
    tracker: Res<'w, EntityReactionAccessTracker>,
    data: Query<'w, 's, (Entity, &'static mut EntityWorldReactorData<T>)>,
}

impl<'w, 's: 'w, T: EntityWorldReactor> ReactorData<'w, 's, T>
{
    /// Returns an iterator over reactor entities and their data available to the current reaction.
    ///
    /// If the current reaction is an *entity reaction*, then one entity will be returned. Otherwise all registered
    /// entities will be returned.
    ///
    /// Returns nothing if used in any system other than the [`EntityWorldReactor`] that is `T`.
    pub fn iter(&self) -> impl Iterator<Item = (Entity, &T::Data)> + '_
    {
        self.reactor
            .system()
            .into_iter()
            .filter_map(|system|
            {
                if self.tracker.system() != system { return None }
                Some(system)
            })
            .flat_map(|_|
            {
                if !self.tracker.is_reacting()
                {
                    Either::Left(self.data.iter())
                }
                else
                {
                    Either::Right(self.data.get(self.tracker.source()).ok().into_iter())
                }.into_iter().map(|(e, data)| (e, data.inner()))
            })
    }

    /// Returns a mutable iterator over reactor entities and their data available to the current reaction.
    ///
    /// If the current reaction is an *entity reaction*, then one entity will be returned. Otherwise all registered
    /// entities will be returned.
    ///
    /// Returns nothing if used in any system other than the [`EntityWorldReactor`] that is `T`.
    pub fn iter_mut(&'s mut self) -> impl Iterator<Item = (Entity, Mut<'w, T::Data>)> + '_
    {
        let Some(system) = self.reactor.system() else
        {
            return Either::Left(None.into_iter());
        };

        if self.tracker.system() != system
        {
            return Either::Left(None.into_iter());
        }

        let right = if !self.tracker.is_reacting()
        {
            Either::Left(self.data.iter_mut())
        }
        else
        {
            Either::Right(self.data.get_mut(self.tracker.source()).ok().into_iter())
        }.into_iter().map(|(e, data)| (e, data.map_unchanged(EntityWorldReactorData::inner_mut)));

        Either::Right(right)
    }
}

//-------------------------------------------------------------------------------------------------------------------
