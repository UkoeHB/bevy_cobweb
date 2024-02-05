//local shortcuts
use crate::*;

//third-party shortcuts
use bevy::prelude::*;

//standard shortcuts
use std::hash::Hash;

//-------------------------------------------------------------------------------------------------------------------

/// Owns a system callback.
///
/// The callback should own the actual system that you want to run. The [`SystemCleanup`] callback must be invoked
/// between running your system and calling `apply_deferred` on that system.
//todo: wrap the callback in a trait that lets you reassign the injected callback if it is the same type
#[derive(Default, Component)]
pub struct SystemCallback
{
    inner: Box<dyn FnMut(&mut World, SystemCleanup) + Send + Sync 'static>,
}

impl SystemCallback
{
    /// Makes a new system callback.
    pub fn new(callback: impl FnMut(&mut World, SystemCleanup) + Send + Sync 'static) -> Self
    {
        Self{ inner: Box::new(callback) }
    }

    /// Runs the system callback.
    ///
    /// The `cleanup` should be invoked between running the callback's inner system and
    /// calling `apply_deferred` on the inner system.
    pub fn run(&mut self, world: &mut World, cleanup: SystemCleanup)
    {
        (self.inner)(world, cleanup);
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Stores a callable system.
///
/// We store the callback in an option in order to avoid archetype moves when taking/reinserting the callback in order to
/// call it.
#[derive(Component)]
pub(crate) struct SystemStorage
{
    callback: Option<SystemCallback>,
}

impl SystemStorage
{
    pub(crate) fn new(callback: SystemCallback) -> Self
    {
        Self{ callback: Some(callback) }
    }

    pub(crate) fn insert(&mut self, callback: SystemCallback)
    {
        self.callback = Some(callback);
    }

    pub(crate) fn take(&mut self) -> Option<SystemCallback>
    {
        self.callback.take()
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Records a cleanup callback that can be injected into system commands for cleanup after the system command
/// runs but before its `apply_deferred` is called.
///
/// For efficiency, only function pointer callbacks are supported.
#[derive(Debug, Default, Copy, Clone)]
pub(crate) struct SystemCleanup
{
    cleanup: Option<fn(&mut World)>,
}

impl SystemCleanup
{
    /// Makes a new system cleanup.
    pub(crate) fn new(cleanup: fn(&mut World)) -> Self
    {
        Self{ cleanup: Some(cleanup) }
    }

    /// Runs the system cleanup on the world.
    ///
    /// Does nothing if no callback is stored.
    pub(crate) fn run(self, world: &mut World)
    {
        let Some(cleanup) = self.cleanup else { return; };
        (cleanup)(world);
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Executes a system on the world.
///
/// System commands scheduled by this system will be run recursively.
///
/// Pre-existing system commands will be temporarily removed then reinserted once the internal recursion is finished.
pub fn system_runner(world: &mut World, id: SysId, cleanup: SystemCleanup)
{
    // extract the callback
    let Some(mut entity_mut) = world.get_entity_mut(id.entity())
    else
    {
        cleanup.run(world);
        return;
    };
    let Some(mut system_command) = entity_mut.get_mut::<SystemStorage>()
    else
    {
        tracing::error!(?id, "system command component is missing");
        cleanup.run(world);
        return;
    };
    let Some(mut callback) = system_command.take()
    else
    {
        tracing::warn!(?id, "system command missing");
        cleanup.run(world);
        return;
    };

    // remove existing system commands temporarily
    let preexisting_syscommands = world.resource_mut::<CobwebCommandQueue<SystemCommand>>().remove();

    // run the system command
    callback.run(world, cleanup);

    // reinsert the callback if its target hasn't been despawned
    // - We don't log an error if the entity is missing in case the callback despawned itself (e.g. one-off commands).
    if let Some(mut entity_mut) = world.get_entity_mut(id.entity())
    {
        if let Some(mut system_command) = entity_mut.get_mut::<SystemStorage>()
        {
            system_command.insert(callback);
        }
        else
        {
            tracing::error!(?id, "system command component is missing");
        }
    }

    // recurse over new system commands
    // - Note that when we recurse, any system commands from this scope will be removed and reinserted, so this
    //   loop will only act on commands added by the system command for this scope.
    while let Some(next_command) = world.resource_mut::<CobwebCommandQueue<SystemCommand>>().pop_front();
    {
        next_command.run(world);
    }

    // replace previously-existing system commands
    world.resource_mut::<CobwebCommandQueue<SystemCommand>>().append(preexisting_syscommands);

    Ok(())
}

//-------------------------------------------------------------------------------------------------------------------

/// Runs a reaction tree to completion.
///
/// This is used for running system commands, system events, and reactions that are scheduled from outside the
/// reaction tree.
pub(crate) fn reaction_tree(world: &mut World)
{
    // Set the reaction tree flag to prevent the reaction tree from being recursively scheduled.
    // - We return if a reaction tree was already started.
    if !world.resource_mut::<ReactCache>().start_reaction_tree() { return; }

    let mut reaction_queue = world.resource_mut::<CobwebCommandQueue<ReactionCommand>>().remove();
    let mut event_queue = world.resource_mut::<CobwebCommandQueue<EventCommand>>().remove();

    'r: loop
    {
        'e: loop
        {
            // run all system commands recursively
            while let Some(next_command) = world.resource_mut::<CobwebCommandQueue<SystemCommand>>().pop_front();
            {
                next_command.run(world);
            }

            // new events go to the front
            world.resource_mut::<CobwebCommandQueue<EventCommand>>().append(std::mem::take(event_queue));
            event_queue = world.resource_mut::<CobwebCommandQueue<EventCommand>>().remove();

            // run one system event
            let Some(next_event) = event_queue.pop_front() else { break 'e; };
            next_event.run(world);
        }

        // new reactions go to the front
        world.resource_mut::<CobwebCommandQueue<ReactionCommand>>().append(std::mem::take(reaction_queue));
        reaction_queue = world.resource_mut::<CobwebCommandQueue<ReactionCommand>>().remove();

        // run one reaction
        let Some(next_reaction) = reaction_queue.pop_front() else { break 'r; };
        next_reaction.run(world);
    }

    world.resource_mut::<CobwebCommandQueue<EventCommand>>().append(event_queue);
    world.resource_mut::<CobwebCommandQueue<ReactionCommand>>().append(reaction_queue);

    // Unset the reaction tree flag now that we are returning to user-land.
    world.resource_mut::<ReactCache>().end_reaction_tree();
}

//-------------------------------------------------------------------------------------------------------------------
