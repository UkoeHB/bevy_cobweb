//local shortcuts
use crate::prelude::*;

//third-party shortcuts
use bevy::prelude::*;

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn cleanup_on_abort(world: &mut World, setup: SystemCommandSetup, cleanup: SystemCommandCleanup)
{
    // We run setup even on abort in case there was a 'prepare' step that needs to be cleared.
    setup.run(world);
    cleanup.run(world);
    garbage_collect_entities(world);
    schedule_removal_and_despawn_reactors(world);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

#[derive(Copy, Clone, Debug)]
pub(crate) struct SystemCommandSetup
{
    reactor: SystemCommand,
    setup: fn(&mut World, SystemCommand),
}

impl SystemCommandSetup
{
    pub(crate) fn new(reactor: SystemCommand, setup: fn(&mut World, SystemCommand)) -> Self
    {
        Self { reactor, setup }
    }

    fn run(self, world: &mut World)
    {
        (self.setup)(world, self.reactor);
    }
}

impl Default for SystemCommandSetup
{
    fn default() -> Self
    {
        Self{
            reactor: SystemCommand(Entity::PLACEHOLDER),
            setup: |_, _| {}
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------

#[derive(Copy, Clone, Debug)]
pub(crate) struct BufferedSyscommand
{
    command: SystemCommand,
    setup: SystemCommandSetup,
    cleanup: SystemCommandCleanup,
}

//-------------------------------------------------------------------------------------------------------------------

/// Executes a system command on the world.
///
/// System commands scheduled by this system will be run recursively.
///
/// Pre-existing system commands will be temporarily removed then reinserted once the internal recursion is finished.
pub(crate) fn syscommand_runner(
    world: &mut World,
    command: SystemCommand,
    setup: SystemCommandSetup,
    cleanup: SystemCommandCleanup,
)
{
    let idx = **world.resource::<SyscommandCounter>();

    // cleanup
    garbage_collect_entities(world);
    schedule_removal_and_despawn_reactors(world);

    // extract the callback
    // - On abort we perform garbage collection in case the cleanup auto-despawns entities.
    let Ok(mut entity_mut) = world.get_entity_mut(*command)
    else
    {
        cleanup_on_abort(world, setup, cleanup);
        return
    };
    let Some(mut system_command) = entity_mut.get_mut::<SystemCommandStorage>()
    else
    {
        tracing::error!(?command, "system command component is missing on extract");
        cleanup_on_abort(world, setup, cleanup);
        return
    };
    let Some(mut callback) = system_command.take()
    else
    {
        // Cache the callback unless at the bottom of the pile.
        if idx == 0 {
            tracing::warn!(?command, "system command missing");
            cleanup_on_abort(world, setup, cleanup);
        } else {
            tracing::debug!(?command, "deferring suspected recursive system command");
            world.resource_mut::<CobwebCommandQueue<BufferedSyscommand>>().push(
                BufferedSyscommand{ command, setup, cleanup }
            );
        }

        return
    };

    // run the system command
    **world.resource_mut::<SyscommandCounter>() += 1;
    setup.run(world);
    callback.run(world, cleanup);

    // cleanup
    // - We do this before reinserting the callback in case the callback garbage collected itself.
    garbage_collect_entities(world);

    // reinsert the callback if its target hasn't been despawned
    if let Ok(mut entity_mut) = world.get_entity_mut(*command)
    {
        if let Some(mut system_command) = entity_mut.get_mut::<SystemCommandStorage>()
        {
            system_command.insert(callback);
        }
        else
        {
            std::mem::drop(callback);
            entity_mut.despawn_recursive();
            tracing::error!(?command, "system command component is missing on insert");

            // In case dropping the callback caused entities to be garbage collected.
            garbage_collect_entities(world);
        }
    }
    else
    {
        std::mem::drop(callback);

        // In case dropping the callback caused entities to be garbage collected.
        garbage_collect_entities(world);
    }

    // handle the case of garbage collection causing despawns
    schedule_removal_and_despawn_reactors(world);

    // run recursive system commands
    let mut buffered_syscommands = world.resource_mut::<CobwebCommandQueue<BufferedSyscommand>>().remove();
    buffered_syscommands
        .retain(
            |buffered|
            {
                // If the buffered command equals the current command, then the current command must be
                // 'now available'.
                if buffered.command == command
                {
                    tracing::debug!(?command, "running reordered recursive system command");
                    syscommand_runner(world, buffered.command, buffered.setup, buffered.cleanup);
                    return false;
                }

                true
            }
        );
    world.resource_mut::<CobwebCommandQueue<BufferedSyscommand>>().append(buffered_syscommands);

    // final cleanup
    if idx == 0
    {
        while let Some(to_discard) = world.resource_mut::<CobwebCommandQueue<BufferedSyscommand>>().pop_front() {
            tracing::warn!(?to_discard.command, "failed to run missing system command");
            cleanup_on_abort(world, to_discard.setup, to_discard.cleanup);
        }

        // Reset the counter since we are exiting the system command tree.
        **world.resource_mut::<SyscommandCounter>() = 0;
    }
}

//-------------------------------------------------------------------------------------------------------------------
