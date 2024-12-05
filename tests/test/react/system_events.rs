//local shortcuts
use bevy_cobweb::prelude::*;
use crate::*;

//third-party shortcuts
use bevy::prelude::*;

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn basic_system_events_impl(mut commands: Commands) -> Vec<usize>
{
    let command1 = commands.spawn_system_command(
        |mut event: SystemEvent<usize>, mut history: ResMut<TelescopeHistory>|
        {
            history.push(event.take().unwrap());
        }
    );
    let command2 = commands.spawn_system_command(
        |mut event: SystemEvent<usize>, mut history: ResMut<TelescopeHistory>|
        {
            history.push(event.take().unwrap());
        }
    );

    let parent = commands.spawn_system_command(
        move |mut commands: Commands|
        {
            commands.send_system_event(command1, 1usize);
            commands.send_system_event(command2, 2usize);
        }
    );
    commands.queue(parent);

    vec![1, 2]
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn system_event_noninterference_impl(mut commands: Commands) -> Vec<usize>
{
    let command1 = commands.spawn_system_command(
        |mut event: SystemEvent<usize>, mut history: ResMut<TelescopeHistory>|
        {
            history.push(event.take().unwrap());
        }
    );

    let parent = commands.spawn_system_command(
        move |mut commands: Commands|
        {
            commands.send_system_event(command1, 1usize);
            commands.send_system_event(command1, 2usize);
            commands.send_system_event(command1, 3usize);
        }
    );
    commands.queue(parent);

    vec![1, 2, 3]
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn system_event_telescoping_impl(mut commands: Commands) -> Vec<usize>
{
    let command1 = commands.spawn_system_command(
        |mut event: SystemEvent<()>, mut history: ResMut<TelescopeHistory>|
        {
            assert!(event.take().is_err());
            history.push(1);
        }
    );
    let command2 = commands.spawn_system_command(
        move
        |
            mut commands : Commands,
            mut history  : ResMut<TelescopeHistory>,
            mut event    : SystemEvent<()>,
            mut saved    : ResMut<SavedSystemCommand>
        |
        {
            commands.queue(command1);
            match saved.take()
            {
                Some(inner) =>
                {
                    commands.queue(inner);
                    history.push(2);
                }
                None =>
                {
                    assert!(event.take().is_err());
                    history.push(3);
                }
            }
        }
    );

    let parent = commands.spawn_system_command(
        move |mut commands: Commands, mut saved: ResMut<SavedSystemCommand>, mut history: ResMut<TelescopeHistory>|
        {
            history.push(0);
            saved.0 = Some(command2);
            commands.queue(command1);
            commands.send_system_event(command2, ());
        }
    );
    commands.queue(parent);

    vec![0, 1, 2, 1, 3, 1]
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn system_event_recursion_impl(mut commands: Commands) -> Vec<usize>
{
    let mut inner: Option<SystemCommand> = None;
    let command1 = commands.spawn_system_command(
        move
        |
            mut commands : Commands,
            mut history  : ResMut<TelescopeHistory>,
            mut event    : SystemEvent<usize>,
            mut saved    : ResMut<SavedSystemCommand>
        |
        {
            if let Some(saved) = saved.take() { inner = Some(saved); }
            let event = event.take().unwrap();
            history.push(event);

            if event == 0 { return; }
            commands.send_system_event(inner.unwrap(), event - 1);
        }
    );

    let parent = commands.spawn_system_command(
        move |mut commands: Commands, mut saved: ResMut<SavedSystemCommand>|
        {
            saved.0 = Some(command1);
            commands.send_system_event(command1, 3usize);
        }
    );
    commands.queue(parent);

    vec![3, 2, 1, 0]
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn send_proxy_entity_system_event_and_take(In(signal): In<AutoDespawnSignal>, mut commands: Commands)
{
    let command1 = commands.spawn_system_command(
        |mut event: SystemEvent<AutoDespawnSignal>| { assert!(event.take().is_ok()); }
    );
    commands.send_system_event(command1, signal);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn send_proxy_entity_system_event_and_ignore(In(signal): In<AutoDespawnSignal>, mut commands: Commands)
{
    let command1 = commands.spawn_system_command(
        |_: SystemEvent<AutoDespawnSignal>| { }
    );
    commands.send_system_event(command1, signal);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn send_proxy_entity_system_event_to_nonexistent(In(signal): In<AutoDespawnSignal>, mut commands: Commands)
{
    let command1 = commands.spawn_system_command(|| { });
    commands.queue(move |world: &mut World| { world.despawn(*command1); });
    commands.send_system_event(command1, signal);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

// System events correctly target the right system.
#[test]
fn basic_system_events()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .init_resource::<TelescopeHistory>();
    let world = app.world_mut();

    let expected = world.syscall((), basic_system_events_impl);
    assert_eq!(expected, **world.resource::<TelescopeHistory>());
}

//-------------------------------------------------------------------------------------------------------------------

// Multiple system events scheduled in a row do not interfere.
#[test]
fn system_event_noninterference()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .init_resource::<TelescopeHistory>();
    let world = app.world_mut();

    let expected = world.syscall((), system_event_noninterference_impl);
    assert_eq!(expected, **world.resource::<TelescopeHistory>());
}

//-------------------------------------------------------------------------------------------------------------------

// System events telescope properly.
// - If data is not taken, it won't be available to system command recursive invocations of the same system, nor to
//   other systems that can read the same system event data.
#[test]
fn system_event_telescoping()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .init_resource::<TelescopeHistory>()
        .insert_resource(SavedSystemCommand(None));
    let world = app.world_mut();

    let expected = world.syscall((), system_event_telescoping_impl);
    assert_eq!(expected, **world.resource::<TelescopeHistory>());
}

//-------------------------------------------------------------------------------------------------------------------

// System events can be recursive.
#[test]
fn system_event_recursion()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .init_resource::<TelescopeHistory>()
        .insert_resource(SavedSystemCommand(None));
    let world = app.world_mut();

    let expected = world.syscall((), system_event_recursion_impl);
    assert_eq!(expected, **world.resource::<TelescopeHistory>());
}

//-------------------------------------------------------------------------------------------------------------------

// System event data is despawned after the target system runs when data is taken.
#[test]
fn system_event_data_is_dropped_on_take()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin);
    let world = app.world_mut();

    let proxy_entity = world.spawn_empty().id();
    let signal = world.resource::<AutoDespawner>().prepare(proxy_entity);

    // send signal via system event
    world.syscall(signal, send_proxy_entity_system_event_and_take);
    assert!(world.get_entity(proxy_entity).is_err());
}

//-------------------------------------------------------------------------------------------------------------------

// System event data is despawned after the target system runs when data is not taken.
#[test]
fn system_event_data_is_dropped_on_ignore()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin);
    let world = app.world_mut();

    let proxy_entity = world.spawn_empty().id();
    let signal = world.resource::<AutoDespawner>().prepare(proxy_entity);

    // send signal via system event
    world.syscall(signal, send_proxy_entity_system_event_and_ignore);
    assert!(world.get_entity(proxy_entity).is_err());
}

//-------------------------------------------------------------------------------------------------------------------

// If a system event is sent, it should be cleaned up if no systems/reactors run
// because the target system doesn't exist.
#[test]
fn system_event_cleanup_on_no_run()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin);
    let world = app.world_mut();

    let proxy_entity = world.spawn_empty().id();
    let signal = world.resource::<AutoDespawner>().prepare(proxy_entity);

    // send signal via system event
    world.syscall(signal, send_proxy_entity_system_event_to_nonexistent);
    assert!(world.get_entity(proxy_entity).is_err());
}

//-------------------------------------------------------------------------------------------------------------------
