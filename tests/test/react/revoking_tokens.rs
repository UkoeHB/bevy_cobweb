//local shortcuts
use bevy_cobweb::prelude::*;
use crate::*;

//third-party shortcuts
use bevy::prelude::*;

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------

#[test]
fn revoke_entity_mutation_reactor()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .init_resource::<TestReactRecorder>();
    let mut world = &mut app.world;

    // entities
    let test_entity = world.spawn_empty().id();

    // add reactor
    let token = syscall(&mut world, test_entity, on_entity_mutation);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // insert (no reaction)
    syscall(&mut world, (test_entity, TestComponent(5)), insert_on_test_entity);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // update (reaction)
    syscall(&mut world, (test_entity, TestComponent(10)), update_test_entity);
    assert_eq!(world.resource::<TestReactRecorder>().0, 10);

    // revoke
    syscall(&mut world, token, revoke_reactor);

    // update (no reaction)
    syscall(&mut world, (test_entity, TestComponent(1)), update_test_entity);
    assert_eq!(world.resource::<TestReactRecorder>().0, 10);
}

//-------------------------------------------------------------------------------------------------------------------

#[test]
fn revoke_component_mutation_reactor()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .init_resource::<TestReactRecorder>();
    let mut world = &mut app.world;

    // entities
    let test_entity = world.spawn_empty().id();

    // add reactor
    let token = syscall(&mut world, (), on_mutation);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // insert (no reaction)
    syscall(&mut world, (test_entity, TestComponent(5)), insert_on_test_entity);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // update (reaction)
    syscall(&mut world, (test_entity, TestComponent(10)), update_test_entity);
    assert_eq!(world.resource::<TestReactRecorder>().0, 10);

    // revoke
    syscall(&mut world, token, revoke_reactor);

    // update (no reaction)
    syscall(&mut world, (test_entity, TestComponent(1)), update_test_entity);
    assert_eq!(world.resource::<TestReactRecorder>().0, 10);
}

//-------------------------------------------------------------------------------------------------------------------

#[test]
fn revoke_event_reactor()
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

#[test]
fn revoke_multiple_reactors()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .insert_react_resource(TestReactRes::default())
        .init_resource::<TestReactRecorder>();
    let mut world = &mut app.world;

    // add reactor
    let revoke_token = syscall(&mut world, (), on_broadcast_or_resource);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // send event (reaction)
    syscall(&mut world, 222, send_broadcast);
    assert_eq!(world.resource::<TestReactRecorder>().0, 222);

    // mutate resource (reaction)
    syscall(&mut world, 1, update_react_res);
    assert_eq!(world.resource::<TestReactRecorder>().0, 223);

    // revoke reactor
    syscall(&mut world, revoke_token, revoke_reactor);

    // mutate resource (no reaction)
    syscall(&mut world, 1, update_react_res);
    assert_eq!(world.resource::<TestReactRecorder>().0, 223);
}

//-------------------------------------------------------------------------------------------------------------------

#[test]
fn revoke_once_reactor()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .insert_react_resource(TestReactRes::default())
        .init_resource::<TestReactRecorder>();
    let mut world = &mut app.world;

    // add reactor
    let revoke_token = syscall(&mut world, (), on_resource_mutation_once);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // revoke reactor
    syscall(&mut world, revoke_token, revoke_reactor);

    // mutate resource (no reaction)
    syscall(&mut world, 1, update_react_res);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);
}

//-------------------------------------------------------------------------------------------------------------------
