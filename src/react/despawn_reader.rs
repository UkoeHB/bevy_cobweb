//local shortcuts
use crate::prelude::*;

//third-party shortcuts
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------

/// Tracks metadata for accessing entity reactions.
#[derive(Resource)]
pub(crate) struct DespawnAccessTracker
{
    /// True when in a system reacting to an entity reaction.
    currently_reacting: bool,
    /// The source of the most recent entity reaction.
    reaction_source: Entity,
    /// A handle to the current reactor.
    ///
    /// This will be dropped after the reactor runs, allowing it to be cleaned up automatically.
    reactor_handle: Option<ReactorHandle>,
}

impl DespawnAccessTracker
{
    /// Sets metadata for the current entity reaction.
    pub(crate) fn start(&mut self, source: Entity, handle: ReactorHandle)
    {
        self.currently_reacting = true;
        self.reaction_source = source;
        self.reactor_handle = Some(handle);
    }

    /// Unsets the 'is reacting' flag and drops the auto despawn signal.
    pub(crate) fn end(&mut self)
    {
        self.currently_reacting = false;
        self.reactor_handle = None;
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
}

impl Default for DespawnAccessTracker
{
    fn default() -> Self
    {
        Self{
            currently_reacting: false,
            reaction_source: Entity::from_raw(0u32),
            reactor_handle: None,
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// System parameter for reading entity despawn events in systems that react to those events.
///
/// Can only be used within [`SystemCommands`](super::SystemCommand).
///
/// Use [`despawn`] to make a trigger that will read these events.
///
/*
```rust
fn example(mut c: Commands)
{
    let entity = c.spawn_empty().id();
    c.react().on(
        despawn(entity),
        |event: DespawnEvent|
        {
            if let Some(entity) = event.read()
            {
                println!("{:?} was despawned", entity);
            }
        }
    );

    c.despawn(entity);
}
```
*/
#[derive(SystemParam)]
pub struct DespawnEvent<'w>
{
    tracker: Res<'w, DespawnAccessTracker>,
}

impl<'w> DespawnEvent<'w>
{
    /// Returns the entity that was despawned if the current system is reacting to that despawn.
    ///
    /// This will return at most one unique entity each time a reactor runs.
    pub fn read(&self) -> Option<Entity>
    {
        if !self.tracker.is_reacting() { return None; }
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
