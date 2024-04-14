//local shortcuts

//third-party shortcuts
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------

/// Tracks metadata for accessing reactive events.
#[derive(Resource)]
pub(crate) struct EventAccessTracker
{
    /// True when in a system processing a reactive event.
    currently_reacting: bool,
    /// Entity where the event data is stored.
    data_entity: Entity,
}

impl EventAccessTracker
{
    /// Sets the 'is reacting' flag.
    pub(crate) fn start(&mut self, data_entity: Entity)
    {
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
        |mut event: BroadcastEvent<()>|
        {
            if let Some(()) = event.take()
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
    /// Reads broadcast event data if it exists.
    ///
    /// This will return at most one unique `T` each time a system runs.
    pub fn read(&self) -> Option<&T>
    {
        if !self.tracker.is_reacting() { return None; }
        let Ok(data) = self.data.get(self.tracker.data_entity()) else { return None; };

        Some(data.read())
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
        |mut event: EntityEvent<()>|
        {
            if let Some(entity, data) = event.read()
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
    /// Reads entity event data if it exists.
    ///
    /// This will return at most one unique `T` each time a system runs.
    pub fn read(&self) -> Option<(Entity, &T)>
    {
        if !self.tracker.is_reacting() { return None; }
        let Ok(data) = self.data.get(self.tracker.data_entity()) else { return None; };

        Some(data.read())
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
