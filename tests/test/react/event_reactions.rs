//local shortcuts
use bevy_cobweb::prelude::*;
use crate::*;

//third-party shortcuts
use bevy::prelude::*;

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn on_broadcast(mut rcommands: ReactCommands) -> RevokeToken
{
    rcommands.on(broadcast::<IntEvent>(), update_test_recorder_with_broadcast)
}

fn on_broadcast_recursive(mut rcommands: ReactCommands) -> RevokeToken
{
    rcommands.on(broadcast::<IntEvent>(), update_test_recorder_with_broadcast_and_recurse)
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

#[test]
fn test_broadcast()
{
    // prepare tracing
    /*
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
    */

    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .init_resource::<TestReactRecorder>();
    let mut world = &mut app.world;

    // add reactor
    syscall(&mut world, (), on_broadcast);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // send event (reaction)
    syscall(&mut world, 222, send_broadcast);
    assert_eq!(world.resource::<TestReactRecorder>().0, 222);

    // send event (reaction)
    syscall(&mut world, 1, send_broadcast);
    assert_eq!(world.resource::<TestReactRecorder>().0, 1);
}

//-------------------------------------------------------------------------------------------------------------------

#[test]
fn broadcast_out_of_order()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .init_resource::<TestReactRecorder>();
    let mut world = &mut app.world;

    // send event (no reaction)
    syscall(&mut world, 222, send_broadcast);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // add reactor (no reaction to prior event)
    syscall(&mut world, (), on_broadcast);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // send event (reaction)
    syscall(&mut world, 1, send_broadcast);
    assert_eq!(world.resource::<TestReactRecorder>().0, 1);
}

//-------------------------------------------------------------------------------------------------------------------

#[test]
fn recursive_broadcasts()
{
    // prepare tracing
    /*
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
    */

    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .init_resource::<TestReactRecorder>();
    let mut world = &mut app.world;

    // add recursive reactor (no reaction)
    syscall(&mut world, (), on_broadcast_recursive);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // send event (only one reaction)
    syscall(&mut world, 0, send_broadcast);
    assert_eq!(world.resource::<TestReactRecorder>().0, 1);

    // send event recursively (two reactions)
    world.resource_mut::<TestReactRecorder>().0 = 0;
    syscall(&mut world, 1, send_broadcast);
    assert_eq!(world.resource::<TestReactRecorder>().0, 2);

    // send event recursively (three reactions)
    world.resource_mut::<TestReactRecorder>().0 = 0;
    syscall(&mut world, 2, send_broadcast);
    assert_eq!(world.resource::<TestReactRecorder>().0, 3);
}

//-------------------------------------------------------------------------------------------------------------------

#[test]
fn revoke_broadcast_reactor()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .init_resource::<TestReactRecorder>();
    let mut world = &mut app.world;

    // add reactor
    let revoke_token = syscall(&mut world, (), on_broadcast);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // send event (reaction)
    syscall(&mut world, 222, send_broadcast);
    assert_eq!(world.resource::<TestReactRecorder>().0, 222);

    // revoke reactor
    syscall(&mut world, revoke_token, revoke_reactor);

    // send event (no reaction)
    syscall(&mut world, 1, send_broadcast);
    assert_eq!(world.resource::<TestReactRecorder>().0, 222);
}

//-------------------------------------------------------------------------------------------------------------------
