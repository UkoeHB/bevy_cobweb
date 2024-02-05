//local shortcuts
use crate::*;

//third-party shortcuts
use bevy::prelude::*;

//standard shortcuts
use std::hash::Hash;

//-------------------------------------------------------------------------------------------------------------------

/// Tracks metadata for accessing broadcast events.
#[derive(Resource)]
pub(crate) struct BroadcastEventAccessTracker
{
    /// True when in a system processing a broadcast event.
    currently_reacting: bool,
    /// Entity where the event data is stored.
    data_entity: Entity,
}

impl BroadcastEventAccessTracker
{
    /// Sets the 'is reacting' flag.
    pub(crate) fn start(&mut self, data_id: TypeId, data_entity: Entity)
    {
        self.currently_reacting = true;
        self.data_entity = data_entity;
    }

    /// Unsets the 'is reacting' flag.
    ///
    /// Returns the data entity so it can be despawned. It should only be despawned after the *last* reader is done.
    pub(crate) fn end(&mut self) -> Entity
    {
        self.currently_reacting = false;
        self.data_entity
    }

    /// Returns `true` if a broadcast event is currently being processed.
    fn is_reacting(&self) -> bool
    {
        self.currently_reacting
    }

    /// Returns the data entity of the most recent broadcast event.
    fn data_entity(&self) -> Entity
    {
        self.data_entity
    }
}

impl Default for BroadcastEventAccessTracker
{
    fn default() -> Self
    {
        Self{
            currently_reacting: false,
            data_entity: Entity::from_raw(0u32),
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Stores data for a broadcast event.
#[derive(Component)]
pub(crate) struct BroadcastEventData<T: Send + Sync + 'static>
{
    /// event data
    data: T,
}

impl<T: Send + Sync + 'static> BroadcastEventData<T>
{
    /// Makes a new broadcast event data.
    pub(crate) fn new(data: T) -> Self
    {
        Self{ data }
    }

    /// Reads the event data.
    fn read(&self) -> &T
    {
        &self.data
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// System parameter for reading broadcast event data.
#[derive(SystemParam)]
pub struct BroadcastEvent<'w, 's, T: Send + Sync + 'static>
{
    tracker: Res<'w, BroadcastEventAccessTracker>,
    data: Query<'w, 's, &'static BroadcastEventData<T>>,
}

impl<'w, 's, T: Send + Sync + 'static> BroadcastEvent<'w, 's, T>
{
    /// Reads broadcast event data if it exists.
    ///
    /// This will return at most one unique `T` each time a system runs.
    pub fn read(&mut self) -> Option<&T>
    {
        if !self.tracker.is_reacting() { return None; }
        let Ok(data) = self.data.get_mut(self.tracker.data_entity()) else { return None; };

        Some(data.read())
    }
}

//-------------------------------------------------------------------------------------------------------------------
