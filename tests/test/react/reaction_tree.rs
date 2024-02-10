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
