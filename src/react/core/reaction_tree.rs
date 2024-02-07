//local shortcuts
use crate::*;

//third-party shortcuts
use bevy::prelude::*;

//standard shortcuts
use std::hash::Hash;

//-------------------------------------------------------------------------------------------------------------------

/// Executes a system on the world.
///
/// System commands scheduled by this system will be run recursively.
///
/// Pre-existing system commands will be temporarily removed then reinserted once the internal recursion is finished.
pub fn syscommand_runner(world: &mut World, command: SystemCommand, cleanup: SystemCommandCleanup)
{
    // extract the callback
    let Some(mut entity_mut) = world.get_entity_mut(*command)
    else
    {
        cleanup.run(world);
        return;
    };
    let Some(mut system_command) = entity_mut.get_mut::<SystemCommandStorage>()
    else
    {
        tracing::error!(?command, "system command component is missing");
        cleanup.run(world);
        return;
    };
    let Some(mut callback) = system_command.take()
    else
    {
        tracing::warn!(?command, "system command missing");
        cleanup.run(world);
        return;
    };

    // remove existing system commands temporarily
    let preexisting_syscommands = world.resource_mut::<CobwebCommandQueue<SystemCommand>>().remove();

    // run the system command
    callback.run(world, cleanup);

    // reinsert the callback if its target hasn't been despawned
    // - We don't log an error if the entity is missing in case the callback despawned itself (e.g. one-off commands).
    if let Some(mut entity_mut) = world.get_entity_mut(*command)
    {
        if let Some(mut system_command) = entity_mut.get_mut::<SystemCommandStorage>()
        {
            system_command.insert(callback);
        }
        else
        {
            tracing::error!(?command, "system command component is missing");
        }
    }

    // recurse over new system commands
    // - Note that when we recurse, any system commands from this scope will be removed and reinserted, so this
    //   loop will only act on commands added by the system command for this scope.
    while let Some(next_command) = world.resource_mut::<CobwebCommandQueue<SystemCommand>>().pop_front();
    {
        next_command.run(world);
    }

    // reinsert previously-existing system commands
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
