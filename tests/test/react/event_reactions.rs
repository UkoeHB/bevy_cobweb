//local shortcuts
use bevy_cobweb::prelude::*;
use crate::*;

//third-party shortcuts
use bevy::prelude::*;

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn on_broadcast(mut c: Commands) -> RevokeToken
{
    c.react().on_revokable(broadcast::<IntEvent>(), update_test_recorder_with_broadcast)
}

fn on_broadcast_recursive(mut c: Commands) -> RevokeToken
{
    c.react().on_revokable(broadcast::<IntEvent>(), update_test_recorder_with_broadcast_and_recurse)
}

fn on_broadcast_unit(mut c: Commands) -> RevokeToken
{
    c.react().on_revokable(broadcast::<()>(), |mut recorder: ResMut<TestReactRecorder>| { recorder.0 += 1; })
}

fn on_broadcast_int(mut c: Commands) -> RevokeToken
{
    c.react().on_revokable(broadcast::<usize>(),
        |event: BroadcastEvent<usize>, mut recorder: ResMut<TestReactRecorder>|
        {
            recorder.0 += event.read();
        }
    )
}

fn on_broadcast_add(mut c: Commands) -> RevokeToken
{
    c.react().on_revokable(broadcast::<IntEvent>(),
        move |event: BroadcastEvent<IntEvent>, mut recorder: ResMut<TestReactRecorder>|
        {
            let Some(event) = event.try_read() else { return; };
            recorder.0 += event.0;
        }
    )
}

fn on_broadcast_proxy(In(proxy): In<Entity>, mut c: Commands) -> RevokeToken
{
    c.react().on_revokable(broadcast::<AutoDespawnSignal>(),
        move |event: BroadcastEvent<AutoDespawnSignal>|
        {
            let proxy_signal = event.read();
            assert_eq!(proxy, proxy_signal.entity());
        }
    )
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn on_entity_event(In(entity): In<Entity>, mut c: Commands) -> RevokeToken
{
    c.react().on_revokable(entity_event::<IntEvent>(entity),
        move |event: EntityEvent<IntEvent>, mut recorder: ResMut<TestReactRecorder>|
        {
            let Some((received_entity, event)) = event.try_read() else { return; };
            assert_eq!(received_entity, entity);
            recorder.0 = event.0;
        }
    )
}

fn on_entity_event_add(In(entity): In<Entity>, mut c: Commands) -> RevokeToken
{
    c.react().on_revokable(entity_event::<IntEvent>(entity),
        move |event: EntityEvent<IntEvent>, mut recorder: ResMut<TestReactRecorder>|
        {
            let Some((received_entity, event)) = event.try_read() else { return; };
            assert_eq!(received_entity, entity);
            recorder.0 += event.0;
        }
    )
}

fn on_entity_event_proxy(In((entity, proxy)): In<(Entity, Entity)>, mut c: Commands) -> RevokeToken
{
    c.react().on_revokable(entity_event::<AutoDespawnSignal>(entity),
        move |event: EntityEvent<AutoDespawnSignal>|
        {
            let (event_entity, proxy_signal) = event.read();
            assert_eq!(entity, event_entity);
            assert_eq!(proxy, proxy_signal.entity());
        }
    )
}

fn on_entity_event_recursive(In(entity): In<Entity>, mut c: Commands) -> RevokeToken
{
    c.react().on_revokable(entity_event::<IntEvent>(entity),
        move
        |
            mut c         : Commands,
            event         : EntityEvent<IntEvent>,
            mut recorder  : ResMut<TestReactRecorder>
        |
        {
            let Some((received_entity, event)) = event.try_read() else { return; };
            assert_eq!(received_entity, entity);
            recorder.0 += 1;

            // recurse until the event is 0
            if event.0 == 0 { return; }
            c.react().entity_event(entity, IntEvent(event.0.saturating_sub(1)));
        }
    )
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn on_any_entity_event(In(target_entity): In<Entity>, mut c: Commands) -> RevokeToken
{
    c.react().on_revokable(any_entity_event::<IntEvent>(),
        move |event: EntityEvent<IntEvent>, mut recorder: ResMut<TestReactRecorder>|
        {
            let Some((received_entity, event)) = event.try_read() else { return; };
            assert_eq!(received_entity, target_entity);
            recorder.0 = event.0;
        }
    )
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn send_broadcast_with<T: Send + Sync + 'static>(In(event): In<T>, mut c: Commands)
{
    c.react().broadcast(event);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// We send all the events within a system command so they are all processed by the same reaction tree.
fn send_multiple_broadcasts(In(data): In<Vec<usize>>, mut commands: Commands)
{
    let events = commands.spawn_system_command(
        move |mut c: Commands|
        {
            for val in data.iter()
            {
                c.react().broadcast(IntEvent(*val));
            }
        }
    );
    commands.add(events);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// We send all the events within a system command so they are all processed by the same reaction tree.
fn send_multiple_entity_events(In((entity, data)): In<(Entity, Vec<usize>)>, mut commands: Commands)
{
    let events = commands.spawn_system_command(
        move |mut c: Commands|
        {
            for val in data.iter()
            {
                c.react().entity_event(entity, IntEvent(*val));
            }
        }
    );
    commands.add(events);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn send_signal_proxy(In((entity, signal)): In<(Entity, AutoDespawnSignal)>, mut c: Commands)
{
    c.react().entity_event(entity, signal);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn broadcast_signal_proxy(In(signal): In<AutoDespawnSignal>, mut c: Commands)
{
    c.react().broadcast(signal);
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
    let world = app.world_mut();

    // add reactor
    world.syscall((), on_broadcast);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // send event (reaction)
    world.syscall(222, send_broadcast);
    assert_eq!(world.resource::<TestReactRecorder>().0, 222);

    // send event (reaction)
    world.syscall(1, send_broadcast);
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
    let world = app.world_mut();

    // send event (no reaction)
    world.syscall(222, send_broadcast);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // add reactor (no reaction to prior event)
    world.syscall((), on_broadcast);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // send event (reaction)
    world.syscall(1, send_broadcast);
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
    let world = app.world_mut();

    // add recursive reactor (no reaction)
    world.syscall((), on_broadcast_recursive);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // send event (only one reaction)
    world.syscall(0, send_broadcast);
    assert_eq!(world.resource::<TestReactRecorder>().0, 1);

    // send event recursively (two reactions)
    world.resource_mut::<TestReactRecorder>().0 = 0;
    world.syscall(1, send_broadcast);
    assert_eq!(world.resource::<TestReactRecorder>().0, 2);

    // send event recursively (three reactions)
    world.resource_mut::<TestReactRecorder>().0 = 0;
    world.syscall(2, send_broadcast);
    assert_eq!(world.resource::<TestReactRecorder>().0, 3);
}

//-------------------------------------------------------------------------------------------------------------------

// Broadcast events are visible to registered systems only.
#[test]
fn broadcast_scoping()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .init_resource::<TestReactRecorder>();
    let world = app.world_mut();

    // add reactors
    world.syscall((), on_broadcast_unit);
    world.syscall((), on_broadcast_int);

    // send int broadcast
    world.syscall((), send_broadcast_with);
    assert_eq!(world.resource::<TestReactRecorder>().0, 1);

    // send event to b
    world.syscall(10usize, send_broadcast_with);
    assert_eq!(world.resource::<TestReactRecorder>().0, 11);
}

//-------------------------------------------------------------------------------------------------------------------

// Multiple broadcast events scheduled in a row do not interfere.
#[test]
fn multiple_broadcast_noninterference()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .init_resource::<TestReactRecorder>();
    let world = app.world_mut();

    // add reactor
    world.syscall((), on_broadcast_add);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // send event (reaction)
    world.syscall(vec![1, 2, 3], send_multiple_broadcasts);
    assert_eq!(world.resource::<TestReactRecorder>().0, 6);
}

//-------------------------------------------------------------------------------------------------------------------

// Reaction data is despawned after the last reader has run.
#[test]
fn broadcast_data_is_dropped()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin);
    let world = app.world_mut();

    let proxy_entity = world.spawn_empty().id();
    let signal = world.resource::<AutoDespawner>().prepare(proxy_entity);

    // add reactors
    world.syscall(proxy_entity, on_broadcast_proxy);
    world.syscall(proxy_entity, on_broadcast_proxy);

    // send event (reaction)
    assert!(world.get_entity(proxy_entity).is_some());
    world.syscall(signal, broadcast_signal_proxy);
    assert!(world.get_entity(proxy_entity).is_none());
}

//-------------------------------------------------------------------------------------------------------------------

// If a broadcast event is sent, it should be cleaned up if no systems/reactors run
// because the target system doesn't exist.
#[test]
fn broadcast_event_cleanup_on_no_run()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin);
    let world = app.world_mut();

    let proxy_entity = world.spawn_empty().id();
    let signal = world.resource::<AutoDespawner>().prepare(proxy_entity);

    // send event without any listeners
    assert!(world.get_entity(proxy_entity).is_some());
    world.syscall(signal, broadcast_signal_proxy);
    reaction_tree(world);  //garbabe collect the entity
    assert!(world.get_entity(proxy_entity).is_none());
}

//-------------------------------------------------------------------------------------------------------------------

// Test entity events.
#[test]
fn test_entity_event()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .init_resource::<TestReactRecorder>();
    let world = app.world_mut();

    let test_entity = world.spawn_empty().id();

    // add reactor
    world.syscall(test_entity, on_entity_event);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // send event (reaction)
    world.syscall((test_entity, 222), send_entity_event);
    assert_eq!(world.resource::<TestReactRecorder>().0, 222);

    // send event (reaction)
    world.syscall((test_entity, 1), send_entity_event);
    assert_eq!(world.resource::<TestReactRecorder>().0, 1);
}

//-------------------------------------------------------------------------------------------------------------------

// Test entity events with 'any entity event' reactor.
#[test]
fn test_any_entity_event()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .init_resource::<TestReactRecorder>();
    let world = app.world_mut();

    let test_entity = world.spawn_empty().id();

    // add reactor
    world.syscall(test_entity, on_any_entity_event);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // send event (reaction)
    world.syscall((test_entity, 222), send_entity_event);
    assert_eq!(world.resource::<TestReactRecorder>().0, 222);

    // send event (reaction)
    world.syscall((test_entity, 1), send_entity_event);
    assert_eq!(world.resource::<TestReactRecorder>().0, 1);
}

//-------------------------------------------------------------------------------------------------------------------

// Recursive entity events.
#[test]
fn recursive_entity_events()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .init_resource::<TestReactRecorder>();
    let world = app.world_mut();

    let test_entity = world.spawn_empty().id();

    // add recursive reactor (no reaction)
    world.syscall(test_entity, on_entity_event_recursive);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // send event (only one reaction)
    world.syscall((test_entity, 0), send_entity_event);
    assert_eq!(world.resource::<TestReactRecorder>().0, 1);

    // send event recursively (two reactions)
    world.resource_mut::<TestReactRecorder>().0 = 0;
    world.syscall((test_entity, 1), send_entity_event);
    assert_eq!(world.resource::<TestReactRecorder>().0, 2);

    // send event recursively (three reactions)
    world.resource_mut::<TestReactRecorder>().0 = 0;
    world.syscall((test_entity, 2), send_entity_event);
    assert_eq!(world.resource::<TestReactRecorder>().0, 3);
}

//-------------------------------------------------------------------------------------------------------------------

// Entity events are visible to registered systems only.
#[test]
fn entity_event_scoping()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .init_resource::<TestReactRecorder>();
    let world = app.world_mut();

    let test_entity_a = world.spawn_empty().id();
    let test_entity_b = world.spawn_empty().id();

    // add reactors
    world.syscall(test_entity_a, on_entity_event_add);
    world.syscall(test_entity_b, on_entity_event_add);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // send event to a
    world.syscall((test_entity_a, 1), send_entity_event);
    assert_eq!(world.resource::<TestReactRecorder>().0, 1);

    // send event to b
    world.syscall((test_entity_b, 10), send_entity_event);
    assert_eq!(world.resource::<TestReactRecorder>().0, 11);
}

//-------------------------------------------------------------------------------------------------------------------

// Multiple entity events scheduled in a row do not interfere.
#[test]
fn multiple_entity_events_noninterference()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .init_resource::<TestReactRecorder>();
    let world = app.world_mut();

    let test_entity = world.spawn_empty().id();

    // add reactor
    world.syscall(test_entity, on_entity_event_add);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // send event (reaction)
    world.syscall((test_entity, vec![1, 2, 3]), send_multiple_entity_events);
    assert_eq!(world.resource::<TestReactRecorder>().0, 6);
}

//-------------------------------------------------------------------------------------------------------------------

// Reaction data is despawned after the last reader has run.
#[test]
fn entity_event_data_is_dropped()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin);
    let world = app.world_mut();

    let test_entity = world.spawn_empty().id();
    let proxy_entity = world.spawn_empty().id();
    let signal = world.resource::<AutoDespawner>().prepare(proxy_entity);

    // add reactors
    world.syscall((test_entity, proxy_entity), on_entity_event_proxy);
    world.syscall((test_entity, proxy_entity), on_entity_event_proxy);

    // send event (reaction)
    assert!(world.get_entity(proxy_entity).is_some());
    world.syscall((test_entity, signal), send_signal_proxy);
    assert!(world.get_entity(proxy_entity).is_none());
}

//-------------------------------------------------------------------------------------------------------------------

// If an entity event is sent, it should be cleaned up if no systems/reactors run
// because the target system doesn't exist.
#[test]
fn entity_event_cleanup_on_no_run()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin);
    let world = app.world_mut();

    let test_entity = world.spawn_empty().id();
    let proxy_entity = world.spawn_empty().id();
    let signal = world.resource::<AutoDespawner>().prepare(proxy_entity);

    // send event
    assert!(world.get_entity(proxy_entity).is_some());
    world.syscall((test_entity, signal), send_signal_proxy);
    reaction_tree(world);  //garbabe collect the entity
    assert!(world.get_entity(proxy_entity).is_none());
}

//-------------------------------------------------------------------------------------------------------------------

#[test]
fn revoke_broadcast_reactor()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .init_resource::<TestReactRecorder>();
    let world = app.world_mut();

    // add reactor
    let revoke_token = world.syscall((), on_broadcast);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // send event (reaction)
    world.syscall(222, send_broadcast);
    assert_eq!(world.resource::<TestReactRecorder>().0, 222);

    // revoke reactor
    world.syscall(revoke_token, revoke_reactor);

    // send event (no reaction)
    world.syscall(1, send_broadcast);
    assert_eq!(world.resource::<TestReactRecorder>().0, 222);
}

//-------------------------------------------------------------------------------------------------------------------

#[test]
fn revoke_entity_event_reactor()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .init_resource::<TestReactRecorder>();
    let world = app.world_mut();

    let test_entity = world.spawn_empty().id();

    // add reactor
    let revoke_token = world.syscall(test_entity, on_entity_event);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // send event (reaction)
    world.syscall((test_entity, 222), send_entity_event);
    assert_eq!(world.resource::<TestReactRecorder>().0, 222);

    // revoke reactor
    world.syscall(revoke_token, revoke_reactor);

    // send event (no reaction)
    world.syscall((test_entity, 1), send_entity_event);
    assert_eq!(world.resource::<TestReactRecorder>().0, 222);
}

//-------------------------------------------------------------------------------------------------------------------

#[test]
fn revoke_any_entity_event_reactor()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .init_resource::<TestReactRecorder>();
    let world = app.world_mut();

    let test_entity = world.spawn_empty().id();

    // add reactor
    let revoke_token = world.syscall(test_entity, on_any_entity_event);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // send event (reaction)
    world.syscall((test_entity, 222), send_entity_event);
    assert_eq!(world.resource::<TestReactRecorder>().0, 222);

    // revoke reactor
    world.syscall(revoke_token, revoke_reactor);

    // send event (no reaction)
    world.syscall((test_entity, 1), send_entity_event);
    assert_eq!(world.resource::<TestReactRecorder>().0, 222);
}

//-------------------------------------------------------------------------------------------------------------------
