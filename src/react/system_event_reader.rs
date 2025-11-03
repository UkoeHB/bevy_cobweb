//local shortcuts
use crate::prelude::*;

//third-party shortcuts
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use std::any::type_name;

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------

/// Tracks metadata for accessing system events.
#[derive(Resource)]
pub(crate) struct SystemEventAccessTracker
{
    /// True when in a system processing a system event.
    currently_reacting: bool,
    /// The entity where system event data is stored.
    data_entity: Entity,

    /// Information cached for when the system actually runs.
    prepared: Vec<(SystemCommand, Entity)>,
}

impl SystemEventAccessTracker
{
    /// Caches metadata for a system event.
    pub(crate) fn prepare(&mut self, system: SystemCommand, data_entity: Entity)
    {
        self.prepared.push((system, data_entity));
    }

    /// Sets metadata for the current entity reaction.
    pub(crate) fn start(&mut self, reactor: SystemCommand)
    {
        let Some(pos) = self.prepared.iter().position(|(s, _)| *s == reactor) else {
            tracing::error!("prepared system event is missing {:?}", reactor);
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
    /// Returns the data entity so it can be despawned.
    pub(crate) fn end(&mut self) -> Entity
    {
        self.currently_reacting = false;
        self.data_entity
    }

    /// Returns `true` if a system event is currently being processed.
    fn is_reacting(&self) -> bool
    {
        self.currently_reacting
    }

    /// Returns the data entity of the most recent system event.
    fn data_entity(&self) -> Entity
    {
        self.data_entity
    }
}

impl Default for SystemEventAccessTracker
{
    fn default() -> Self
    {
        Self{
            currently_reacting: false,
            data_entity: Entity::from_raw_u32(0u32).unwrap(),
            prepared: Vec::default(),
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Stores data for a system event.
#[derive(Component)]
pub(crate) struct SystemEventData<T: Send + Sync + 'static>
{
    data: Option<T>,
}

impl<T: Send + Sync + 'static> SystemEventData<T>
{
    /// Makes a new system event data.
    pub(crate) fn new(data: T) -> Self
    {
        Self{ data: Some(data) }
    }

    /// Takes the system event data.
    fn take(&mut self) -> Option<T>
    {
        self.data.take()
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// System parameter for receiving system event data.
///
/// Can only be used within [`SystemCommands`](super::SystemCommand).
///
/*
```rust
let cmd = commands.spawn_system_command(
    |mut event: SystemEvent<()>|
    {
        event.take()?;
        println!("event received");
        DONE
    }
);

commands.send_system_event(cmd, ());
```
*/
#[derive(SystemParam)]
pub struct SystemEvent<'w, 's, T: Send + Sync + 'static>
{
    tracker: Res<'w, SystemEventAccessTracker>,
    data: Query<'w, 's, &'static mut SystemEventData<T>>,
}

impl<'w, 's, T: Send + Sync + 'static> SystemEvent<'w, 's, T>
{
    /// Takes system event data if it exists.
    ///
    /// This will return at most one unique `T` each time a system runs.
    pub fn take(&mut self) -> Result<T, CobwebReactError>
    {
        let t = type_name::<T>();
        if !self.tracker.is_reacting() { return Err(CobwebReactError::SystemEvent(t)); }
        let Ok(mut data) = self.data.get_mut(self.tracker.data_entity()) else {
            return Err(CobwebReactError::SystemEvent(t));
        };

        data.take().ok_or(CobwebReactError::SystemEvent(t))
    }
}

//-------------------------------------------------------------------------------------------------------------------
