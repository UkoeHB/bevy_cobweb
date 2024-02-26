//local shortcuts
use crate::prelude::*;

//third-party shortcuts
use bevy::prelude::*;

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn garbage_collect_entities(world: &mut World)
{
    while let Some(entity) = world.resource::<AutoDespawner>().try_recv()
    {
        world.despawn(entity);
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// Executes a system command on the world.
///
/// System commands scheduled by this system will be run recursively.
///
/// Pre-existing system commands will be temporarily removed then reinserted once the internal recursion is finished.
pub(crate) fn syscommand_runner(world: &mut World, command: SystemCommand, cleanup: SystemCommandCleanup)
{
    // cleanup
    garbage_collect_entities(world);

    // On abort we perform garbage collection in case the cleanup auto-despawns entities.
    let cleanup_on_abort = |world: &mut World|
    {
        cleanup.run(world);
        garbage_collect_entities(world);
        schedule_removal_and_despawn_reactors(world);
    };

    // extract the callback
    let Some(mut entity_mut) = world.get_entity_mut(*command)
    else
    {
        (cleanup_on_abort)(world);
        return;
    };
    let Some(mut system_command) = entity_mut.get_mut::<SystemCommandStorage>()
    else
    {
        tracing::error!(?command, "system command component is missing on extract");
        (cleanup_on_abort)(world);
        return;
    };
    let Some(mut callback) = system_command.take()
    else
    {
        tracing::warn!(?command, "system command missing");
        (cleanup_on_abort)(world);
        return;
    };

    // remove existing system commands temporarily
    let preexisting_syscommands = world.resource_mut::<CobwebCommandQueue<SystemCommand>>().remove();

    // run the system command
    callback.run(world, cleanup);

    // cleanup
    // - We do this before reinserting the callback in case the callback garbage collected itself.
    garbage_collect_entities(world);

    // reinsert the callback if its target hasn't been despawned
    if let Some(mut entity_mut) = world.get_entity_mut(*command)
    {
        if let Some(mut system_command) = entity_mut.get_mut::<SystemCommandStorage>()
        {
            system_command.insert(callback);
        }
        else
        {
            std::mem::drop(callback);
            tracing::error!(?command, "system command component is missing on insert");
        }
    }
    else
    {
        std::mem::drop(callback);
    }

    // cleanup
    // - We do this again just in case dropping the callback caused entities to be garbage collected.
    garbage_collect_entities(world);

    // schedule component removal and despawn reactors
    schedule_removal_and_despawn_reactors(world);

    // recurse over new system commands
    // - Note that when we recurse, any system commands from this scope will be removed and reinserted, so this
    //   loop will only act on commands added by the system command for this scope.
    while let Some(next_command) = world.resource_mut::<CobwebCommandQueue<SystemCommand>>().pop_front()
    {
        next_command.run(world);
    }

    // reinsert previously-existing system commands
    world.resource_mut::<CobwebCommandQueue<SystemCommand>>().append(preexisting_syscommands);
}

//-------------------------------------------------------------------------------------------------------------------

/// Runs a reaction tree to completion.
///
/// This is used for running system commands, system events, and reactions that are scheduled from outside the
/// reaction tree.
pub fn reaction_tree(world: &mut World)
{
    // Set the reaction tree flag to prevent the reaction tree from being recursively scheduled.
    // - We return if we are already in a reaction tree.
    if !world.resource_mut::<ReactCache>().start_reaction_tree() { return; }

    let mut reaction_queue = world.resource_mut::<CobwebCommandQueue<ReactionCommand>>().remove();
    let mut event_queue = world.resource_mut::<CobwebCommandQueue<EventCommand>>().remove();

    // Schedule component removal and despawn reactors.
    // - We do this once at the beginning of the tree in case the scheduled command that triggered the tree
    //   fails to actually run. Even if it doesn't run, we should still handle removals and despawns.
    garbage_collect_entities(world);
    schedule_removal_and_despawn_reactors(world);

    // Run the tree.
    'r: loop
    {
        'e: loop
        {
            // run all system commands recursively
            while let Some(next_command) = world.resource_mut::<CobwebCommandQueue<SystemCommand>>().pop_front()
            {
                next_command.run(world);
            }

            // new events go to the front
            event_queue = world.resource_mut::<CobwebCommandQueue<EventCommand>>().append_and_remove(event_queue);

            // run one system event
            let Some(next_event) = event_queue.pop_front() else { break 'e; };
            next_event.run(world);
        }

        // new reactions go to the front
        reaction_queue = world.resource_mut::<CobwebCommandQueue<ReactionCommand>>().append_and_remove(reaction_queue);

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
