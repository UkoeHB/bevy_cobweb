//local shortcuts
use bevy_cobweb::prelude::*;
use crate::*;

//third-party shortcuts
use bevy::prelude::*;

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn command_ordering_impl(mut rcommands: ReactCommands) -> Vec<usize>
{
    let system_command = rcommands.commands().spawn_system_command(
        |mut history: ResMut<TelescopeHistory>|
        {
            history.push(1);
        }
    );
    let event_command = rcommands.commands().spawn_system_command(
        |mut event: SystemEvent<()>, mut history: ResMut<TelescopeHistory>|
        {
            event.take().unwrap();
            history.push(2);
        }
    );
    rcommands.on(broadcast::<()>(),
        |event: BroadcastEvent<()>, mut history: ResMut<TelescopeHistory>|
        {
            event.read().unwrap();
            history.push(3);
        }
    );

    let parent = rcommands.commands().spawn_system_command(
        move |mut rcommands: ReactCommands|
        {
            rcommands.broadcast(());
            rcommands.commands().send_system_event(event_command, ());
            rcommands.commands().add(system_command);
        }
    );
    rcommands.commands().add(parent);

    vec![1, 2, 3]
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn multitest_prep_commands(mut rcommands: ReactCommands)
{
    let sys1 = rcommands.commands().spawn_system_command(
            |event: BroadcastEvent<usize>, mut history: ResMut<TelescopeHistory>|
            {
                history.push(*event.read().unwrap());
            }
        );
    let sys2 = rcommands.commands().spawn_system_command(
            |event: BroadcastEvent<usize>, mut history: ResMut<TelescopeHistory>|
            {
                history.push(*event.read().unwrap());
            }
        );

    //**saved = Some(sys1);
    rcommands.with(broadcast::<usize>(), sys1);
    rcommands.with(broadcast::<usize>(), sys2);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn multitest_system1(mut rcommands: ReactCommands, mut history: ResMut<TelescopeHistory>)
{
    history.push(1);
    rcommands.broadcast(3usize);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn multitest_system2(mut rcommands: ReactCommands, mut history: ResMut<TelescopeHistory>)
{
    history.push(2);
    rcommands.broadcast(4usize);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

// A system command, system event, and reaction are all executed in that order even when scheduled out of order.
#[test]
fn command_ordering()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .init_resource::<TelescopeHistory>()
        .insert_resource(SavedSystemCommand(None));
    let world = &mut app.world;

    let expected = world.syscall((), command_ordering_impl);
    assert_eq!(expected, **world.resource::<TelescopeHistory>());
}

//-------------------------------------------------------------------------------------------------------------------

// If two user-land systems schedule events, they should both see the results when apply_deferred is applied.
// - Older bug: queuing events directly when event data spawns are deferred would cause the event data to be invisible
//   when the queues are drained by a reaction tree scheduled before the data spawn.
#[test]
fn multisystem_scheduling()
{
    // setup
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins(ReactPlugin)
        .init_resource::<TelescopeHistory>()
        .insert_resource(SavedSystemCommand(None))
        .add_systems(Startup, multitest_prep_commands)
        .add_systems(Update, multitest_system1)
        .add_systems(Update, multitest_system2)
        .update();
    let world = &mut app.world;

    assert_eq!(vec![1, 2, 3, 3, 4, 4], **world.resource::<TelescopeHistory>());
}

//-------------------------------------------------------------------------------------------------------------------
