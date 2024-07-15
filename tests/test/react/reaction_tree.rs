//local shortcuts
use bevy_cobweb::prelude::*;
use crate::*;

//third-party shortcuts
use bevy::prelude::*;

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn command_ordering_impl(mut c: Commands) -> Vec<usize>
{
    let system_command = c.spawn_system_command(
        |mut history: ResMut<TelescopeHistory>|
        {
            history.push(1);
        }
    );
    let event_command = c.spawn_system_command(
        |mut event: SystemEvent<()>, mut history: ResMut<TelescopeHistory>|
        {
            event.take().unwrap();
            history.push(2);
        }
    );
    c.react().on(broadcast::<()>(),
        |event: BroadcastEvent<()>, mut history: ResMut<TelescopeHistory>|
        {
            event.read();
            history.push(3);
        }
    );

    let parent = c.spawn_system_command(
        move |mut c: Commands|
        {
            c.react().broadcast(());
            c.react().commands().send_system_event(event_command, ());
            c.react().commands().add(system_command);
        }
    );
    c.add(parent);

    vec![1, 2, 3]
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn multitest_prep_commands(mut c: Commands)
{
    let sys1 = c.spawn_system_command(
            |event: BroadcastEvent<usize>, mut history: ResMut<TelescopeHistory>|
            {
                history.push(*event.read());
            }
        );
    let sys2 = c.spawn_system_command(
            |event: BroadcastEvent<usize>, mut history: ResMut<TelescopeHistory>|
            {
                history.push(*event.read());
            }
        );

    //**saved = Some(sys1);
    c.react().with(broadcast::<usize>(), sys1, ReactorMode::Persistent);
    c.react().with(broadcast::<usize>(), sys2, ReactorMode::Persistent);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn multitest_system1(mut c: Commands, mut history: ResMut<TelescopeHistory>)
{
    history.push(1);
    c.react().broadcast(3usize);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn multitest_system2(mut c: Commands, mut history: ResMut<TelescopeHistory>)
{
    history.push(2);
    c.react().broadcast(4usize);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn invoke_echo_system(event: BroadcastEvent<usize>, mut c: Commands)
{
    assert!(event.try_read().is_some());
    c.syscall((), move |event: BroadcastEvent<usize>| {
        assert!(event.try_read().is_none());
    });
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
    let world = app.world_mut();

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
    let world = app.world_mut();

    //todo: if this fails in bevy v0.15, it's because of system ordering
    assert_eq!(vec![1, 2, 3, 3, 4, 4], **world.resource::<TelescopeHistory>());
}

//-------------------------------------------------------------------------------------------------------------------

// Event cleanup should properly run between a system and when its commands are applied.
#[test]
fn cleanup_ordering()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .react(|rc| rc.on_persistent(broadcast::<usize>(), invoke_echo_system))
        .update();
    app.react(|rc| rc.broadcast(0usize));
}

//-------------------------------------------------------------------------------------------------------------------

// If reactions infinitely recurse then it should panic.
#[test]
#[should_panic]
fn infinite_recursion()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .react(|rc| rc.on_persistent(broadcast::<usize>(), |mut c: Commands| {
            c.react().broadcast(0usize);
        }))
        .update();
    app.react(|rc| rc.broadcast(0usize));
}

//-------------------------------------------------------------------------------------------------------------------
