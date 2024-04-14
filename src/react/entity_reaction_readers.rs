//local shortcuts
use crate::prelude::*;

//third-party shortcuts
//use bevy::ecs::component::ComponentId;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

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
    /// The source of the most recent entity reaction.
    reaction_source: Entity,
    /// The type of the most recent entity reaction trigger.
    reaction_type: EntityReactionType,
}

impl EntityReactionAccessTracker
{
    /// Sets metadata for the current entity reaction.
    pub(crate) fn start(&mut self, source: Entity, reaction: EntityReactionType)
    {
        debug_assert!(!self.currently_reacting);
        self.currently_reacting = true;
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
            reaction_source: Entity::from_raw(0u32),
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
