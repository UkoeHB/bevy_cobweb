//local shortcuts
use crate::*;

//third-party shortcuts
use bevy::prelude::*;

//standard shortcuts
use std::hash::Hash;

//-------------------------------------------------------------------------------------------------------------------

/// The type of an entity reaction.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
pub(crate) enum EntityReactionType
{
    /// Default type.
    #[default]
    None,
    /// A component was inserted.
    Insertion(ComponentId),
    /// A component was removed.
    Removal(ComponentId),
    /// A component was mutated.
    Mutation(ComponentId),
    /// An entity was despawned.
    Despawn,
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
#[derive(SystemParam)]
pub struct InsertionEvent<'w, T: ReactComponent>
{
    tracker: Res<'w, EntityReactionAccessTracker>,
    components: Components<'w>,
    p: PhantomData<T>,
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
        //todo: is there some way to cache this and avoid a hashmap lookup?
        let Some(expected_component_id) = self.components.get_id(std::any::TypeId::of::<React<T>>()) else { return None; };
        if component_id != expected_component_id { return None; }

        Some(self.tracker.source())
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// System parameter for reading entity component removal events in systems that react to those events.
#[derive(SystemParam)]
pub struct RemovalEvent<'w, T: ReactComponent>
{
    tracker: Res<'w, EntityReactionAccessTracker>,
    components: Components<'w>,
    p: PhantomData<T>,
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
        //todo: is there some way to cache this and avoid a hashmap lookup?
        let Some(expected_component_id) = self.components.get_id(std::any::TypeId::of::<React<T>>()) else { return None; };
        if component_id != expected_component_id { return None; }

        Some(self.tracker.source())
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// System parameter for reading entity component mutation events in systems that react to those events.
#[derive(SystemParam)]
pub struct MutationEvent<'w, T: ReactComponent>
{
    tracker: Res<'w, EntityReactionAccessTracker>,
    components: Components<'w>,
    p: PhantomData<T>,
}

impl<'w, T: ReactComponent> MutationEvent<'w, T>
{
    /// Returns the entity from which a `React<T>` component was removed if the current system is
    /// reacting to that mutation.
    ///
    /// This will return at most one unique entity each time a reactor runs.
    pub fn read(&self) -> Option<Entity>
    {
        if !self.tracker.is_reacting() { return None; }
        let EntityReactionType::Mutation(component_id) = self.tracker.reaction_type() else { return None; };
        //todo: is there some way to cache this and avoid a hashmap lookup?
        let Some(expected_component_id) = self.components.get_id(std::any::TypeId::of::<React<T>>()) else { return None; };
        if component_id != expected_component_id { return None; }

        Some(self.tracker.source())
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// System parameter for reading entity despawn events in systems that react to those events.
#[derive(SystemParam)]
pub struct DespawnEvent<'w>
{
    tracker: Res<'w, EntityReactionAccessTracker>,
}

impl<'w> DespawnEvent<'w>
{
    /// Returns the entity that was despawned if the current system is reacting to that despawn.
    ///
    /// This will return at most one unique entity each time a reactor runs.
    pub fn read(&self) -> Option<Entity>
    {
        if !self.tracker.is_reacting() { return None; }
        let EntityReactionType::Despawn = self.tracker.reaction_type() else { return None; };

        Some(self.tracker.source())
    }
}

//-------------------------------------------------------------------------------------------------------------------
