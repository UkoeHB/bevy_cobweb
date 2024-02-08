//local shortcuts
use crate::*;

//third-party shortcuts
use bevy::prelude::*;

//standard shortcuts
use std::any::TypeId;
use std::hash::Hash;

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

//todo: switch to ComponentId when observers are implemented
//(cannot do so yet because component ids are not available when reactions are triggered, only type ids)
struct ReactComponentId<T: ReactComponent>
{
    //id: ComponentId,
    id: TypeId,
    p: PhantomData<T>,
};

impl<T: ReactComponent> ReactComponentId<T>
{
    fn id(&self) -> ComponentId
    {
        self.id
    }
}

impl<T: ReactComponent> FromWorld for ReactComponentId<T>
{
    fn from_world(world: &mut World) -> Self
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

/// The type of an entity reaction.
//todo: switch to ComponentId when observers are integrated
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
pub(crate) enum EntityReactionType
{
    /// Default type.
    #[default]
    None,
    /// A component was inserted.
    Insertion(TypeId),
    /// A component was removed.
    Removal(TypeId),
    /// A component was mutated.
    Mutation(TypeId),
}

//-------------------------------------------------------------------------------------------------------------------

/// Tracks metadata for accessing entity reactions.
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
            reaction_type: EntityReactionType::default(),
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// System parameter for reading entity component insertion events in systems that react to those events.
///
/*
```rust
fn example(mut rcommands: ReactCommands)
{
    let entity = rcommands.commands().spawn_empty().id();
    rcommands.on(
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

impl<'w, T: ReactComponent> InsertionEvent<'w, T>
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
}

//-------------------------------------------------------------------------------------------------------------------

/// System parameter for reading entity component removal events in systems that react to those events.
///
/*
```rust
fn example(mut rcommands: ReactCommands, query: Query<Entity, With<React<A>>>)
{
    rcommands.on(
        removal::<A>(),  // entity-specific: entity_removal::<A>(target_entity)
        |event: RemovalEvent<A>|
        {
            if let Some(entity) = event.read()
            {
                println!("'A' was removed from {:?}", entity);
            }
        }
    );

    rcommands.commands().entity(*query.single()).remove::<A>();
}
```
*/
#[derive(SystemParam)]
pub struct RemovalEvent<'w, 's, T: ReactComponent>
{
    component_id: Local<'s, ReactComponentId<T>>,
    tracker: Res<'w, EntityReactionAccessTracker>,
}

impl<'w, T: ReactComponent> RemovalEvent<'w, T>
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
}

//-------------------------------------------------------------------------------------------------------------------

/// System parameter for reading entity component mutation events in systems that react to those events.
///
/// Can only be used within [`SystemCommands`](super::SystemCommand).
///
/*
```rust
fn example(mut rcommands: ReactCommands, query: Query<&mut React<A>>)
{
    rcommands.on(
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

impl<'w, T: ReactComponent> MutationEvent<'w, T>
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
}

//-------------------------------------------------------------------------------------------------------------------
