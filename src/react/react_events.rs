//local shortcuts

//third-party shortcuts
use bevy::ecs::event::Event;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

struct ReactEventReaderync(u64);

impl FromWorld for ReactEventReaderync
{
    fn from_world(world: &mut World) -> Self
    {
        Self(world.resource::<ReactEventCounter>().0)
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

#[derive(Resource, Default)]
pub(crate) struct ReactEventCounter(u64);

impl ReactEventCounter
{
    pub(crate) fn increment(&mut self) -> u64
    {
        self.0 += 1;
        self.0
    }
}

//-------------------------------------------------------------------------------------------------------------------

#[derive(Event)]
pub(crate) struct ReactEventInner<E: Send + Sync + 'static>
{
    /// This event's id.
    pub(crate) event_id: u64,
    /// The event.
    pub(crate) event: E,
}

//-------------------------------------------------------------------------------------------------------------------

/// Provides access to react events of type `E`.
///
/// The first react event returned by `next()` will be the last event sent before the system was initialized in the world.
/// For event reactors, this will be the first event that triggered a reaction. Our implementation assumes:
/// - Reactors are not initialized until they first execute.
/// - Reactors cannot run recursively, so there is no way for this sequence: event type A fires, reaction 1 reacts, event
///   type A fires again, reaction 2 reacts (it was queued to react to the first event of type A). If that sequence is
///   possible then reaction 2 will only see the second event of type A.
///
/// It is only recommended to use this inside systems registered as event reactors with [`ReactCommands`]. The behavior
/// is likely to be unexpected if used anywhere else.
#[derive(SystemParam)]
pub struct ReactEventReader<'w, 's, E: Send + Sync + 'static>
{
    /// Event counter recording the id of the first react event sent after the system with this param was registered.
    sync: Local<'s, ReactEventReaderync>,
    /// Reads events.
    reader: EventReader<'w, 's, ReactEventInner<E>>,
}

impl<'w, 's, E: Send + Sync + 'static> ReactEventReader<'w, 's, E>
{
    /// Get the next available event.
    ///
    /// It is recommended to call this exactly once per event reactor invocation.
    pub fn next(&mut self) -> Option<&E>
    {
        self.read().next()
    }

    /// Reads all currently-pending react events.
    ///
    /// It is recommended to use [`ReactEventReader::next()`] instead. Event reactors are invoked once per react event, so
    /// `.next()` will always give the event that triggered your system (assuming you only call `.next()` once per
    /// invocation).
    pub fn read(&mut self) -> impl Iterator<Item = &E> + '_
    {
        let floor = self.sync.0;
        self.reader
            .read()
            .filter_map(
                move |e|
                {
                    if e.event_id < floor { return None; }
                    Some(&e.event)
                }
            )
    }

    /// Check if the events queue is empty.
    pub fn is_empty(&self) -> bool
    {
        self.reader.is_empty()
    }

    /*
    /// Get number of pending events.
    ///
    //todo: this is not accurate since we may need to ignore some events in the internal reader
    pub fn len(&self) -> usize
    {
        self.reader.len()
    }
    */

    /// Clear all pending events in this reader.
    pub fn clear(&mut self)
    {
        self.reader.clear()
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Extends the `App` API with reactive event methods.
pub trait ReactEventAppExt
{
    fn add_react_event<E: Send + Sync + 'static>(&mut self) -> &mut Self;
}

impl ReactEventAppExt for App
{
    fn add_react_event<E: Send + Sync + 'static>(&mut self) -> &mut Self
    {
        self.add_event::<ReactEventInner<E>>()
    }
}

//-------------------------------------------------------------------------------------------------------------------
