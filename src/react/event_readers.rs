//local shortcuts
use crate::prelude::SystemCommand;

//third-party shortcuts
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

//standard shortcuts
use std::any::type_name;

//-------------------------------------------------------------------------------------------------------------------

/// Tracks metadata for accessing reactive events.
#[derive(Resource)]
pub(crate) struct EventAccessTracker
{
    /// True when in a system processing a reactive event.
    currently_reacting: bool,
    /// Entity where the event data is stored.
    data_entity: Entity,

    /// Reaction information cached for when the reaction system actually runs.
    prepared: Vec<(SystemCommand, Entity)>,
}

impl EventAccessTracker
{
    /// Caches metadata for an entity reaction.
    pub(crate) fn prepare(&mut self, system: SystemCommand, data_entity: Entity)
    {
        self.prepared.push((system, data_entity));
    }

    /// Sets metadata for the current entity reaction.
    pub(crate) fn start(&mut self, reactor: SystemCommand)
    {
        let Some(pos) = self.prepared.iter().position(|(s, _)| *s == reactor) else {
            tracing::error!("prepared event reaction is missing {:?}", reactor);
            debug_assert!(false);
            return;
        };
        let (_, data_entity) = self.prepared.swap_remove(pos);

        debug_assert!(!self.currently_reacting);
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

    /// Returns `true` if an reactive event is currently being processed.
    fn is_reacting(&self) -> bool
    {
        self.currently_reacting
    }

    /// Returns the data entity of the most recent reactive event.
    fn data_entity(&self) -> Entity
    {
        self.data_entity
    }
}

impl Default for EventAccessTracker
{
    fn default() -> Self
    {
        Self{
            currently_reacting: false,
            data_entity: Entity::from_raw(0u32),
            prepared: Vec::default(),
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Stores data for a reactive event.
#[derive(Component)]
pub(crate) struct BroadcastEventData<T: Send + Sync + 'static>
{
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

/// Stores data for a reactive event.
#[derive(Component)]
pub(crate) struct EntityEventData<T: Send + Sync + 'static>
{
    entity: Entity,
    data: T,
}

impl<T: Send + Sync + 'static> EntityEventData<T>
{
    /// Makes a new entity event data.
    pub(crate) fn new(target_entity: Entity, data: T) -> Self
    {
        Self{ entity: target_entity, data }
    }

    /// Reads the event data.
    fn read(&self) -> (Entity, &T)
    {
        (self.entity, &self.data)
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// System parameter for reading broadcast event data.
///
/// Can only be used within [`SystemCommands`](super::SystemCommand).
///
/// Use [`broadcast`](crate::prelude::broadcast) to make a trigger that will read these events.
///
/*
```rust
fn example(mut c: Commands)
{
    c.react().on(
        broadcast::<()>(),
        |event: BroadcastEvent<()>|
        {
            if let Some(()) = event.try_read()
            {
                println!("event received");
            }
        }
    );

    c.react().broadcast(());
}
```
*/
#[derive(SystemParam)]
pub struct BroadcastEvent<'w, 's, T: Send + Sync + 'static>
{
    tracker: Res<'w, EventAccessTracker>,
    data: Query<'w, 's, &'static BroadcastEventData<T>>,
}

impl<'w, 's, T: Send + Sync + 'static> BroadcastEvent<'w, 's, T>
{
    /// Reads broadcast event data.
    ///
    /// This will return at most one unique `T` each time a system runs.
    ///
    /// Panics if there is no data to read.
    pub fn read(&self) -> &T
    {
        self.try_read()
            .unwrap_or_else(|| panic!("failed reading broadcast event for {}, there is no event", type_name::<T>()))
    }

    /// See [`Self::read`].
    pub fn try_read(&self) -> Option<&T>
    {
        if !self.tracker.is_reacting() { return None; }
        let Ok(data) = self.data.get(self.tracker.data_entity()) else { return None; };

        Some(data.read())
    }

    /// Returns `true` if there is nothing to read.
    ///
    /// Equivalent to `event.try_read().is_none()`.
    pub fn is_empty(&self) -> bool
    {
        self.try_read().is_none()
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// System parameter for reading entity event data.
///
/// Can only be used within [`SystemCommands`](super::SystemCommand).
///
/// Use [`entity_event`](crate::prelude::entity_event) or [`any_entity_event`](crate::prelude::any_entity_event) to make
/// a trigger that will read these events.
///
/*
```rust
fn example(mut c: Commands)
{
    let entity = c.spawn_empty();
    c.react().on(
        entity_event::<()>(entity),
        |event: EntityEvent<()>|
        {
            if let Some(entity, data) = event.try_read()
            {
                println!("event {:?} received for {:?}", data, entity);
            }
        }
    );

    c.react().entity_event(entity, ());
}
```
*/
#[derive(SystemParam)]
pub struct EntityEvent<'w, 's, T: Send + Sync + 'static>
{
    tracker: Res<'w, EventAccessTracker>,
    data: Query<'w, 's, &'static EntityEventData<T>>,
}

impl<'w, 's, T: Send + Sync + 'static> EntityEvent<'w, 's, T>
{
    /// Reads entity event data.
    ///
    /// This will return at most one unique `T` each time a system runs.
    ///
    /// Panics if there is no data to read.
    pub fn read(&self) -> (Entity, &T)
    {
        self.try_read()
            .unwrap_or_else(|| panic!("failed reading entity event for {}, there is no event", type_name::<T>()))
    }

    /// See [`Self::read`].
    pub fn try_read(&self) -> Option<(Entity, &T)>
    {
        if !self.tracker.is_reacting() { return None; }
        let Ok(data) = self.data.get(self.tracker.data_entity()) else { return None; };

        Some(data.read())
    }

    /// Gets the target entity of the event.
    ///
    /// Panics if there is no event.
    pub fn entity(&self) -> Entity
    {
        self.read().0
    }

    /// See [`Self::entity`].
    pub fn get_entity(&self) -> Option<Entity>
    {
        self.try_read().map(|(e, _)| e)
    }

    /// Returns `true` if there is nothing to read.
    ///
    /// Equivalent to `event.try_read().is_none()`.
    pub fn is_empty(&self) -> bool
    {
        self.try_read().is_none()
    }
}

//-------------------------------------------------------------------------------------------------------------------
