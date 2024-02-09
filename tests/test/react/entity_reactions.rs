//local shortcuts
use bevy_cobweb::prelude::*;
use crate::*;

//third-party shortcuts
use bevy::prelude::*;

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------

#[test]
fn react_entity_insertion()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .init_resource::<TestReactRecorder>();
    let mut world = &mut app.world;

    // entities
    let test_entity_a = world.spawn_empty().id();
    let test_entity_b = world.spawn_empty().id();

    // add reactor
    syscall(&mut world, test_entity_a, on_entity_insertion);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // insert (reaction)
    syscall(&mut world, (test_entity_a, TestComponent(1)), insert_on_test_entity);
    assert_eq!(world.resource::<TestReactRecorder>().0, 1);

    // insert (reaction)
    syscall(&mut world, (test_entity_a, TestComponent(2)), insert_on_test_entity);
    assert_eq!(world.resource::<TestReactRecorder>().0, 2);

    // insert other entity (no reaction)
    syscall(&mut world, (test_entity_b, TestComponent(3)), insert_on_test_entity);
    assert_eq!(world.resource::<TestReactRecorder>().0, 2);
}

//-------------------------------------------------------------------------------------------------------------------

#[test]
fn react_component_insertion()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .init_resource::<TestReactRecorder>();
    let mut world = &mut app.world;

    // entities
    let test_entity_a = world.spawn_empty().id();
    let test_entity_b = world.spawn_empty().id();

    // add reactor
    syscall(&mut world, (), on_insertion);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // insert (reaction)
    syscall(&mut world, (test_entity_a, TestComponent(1)), insert_on_test_entity);
    assert_eq!(world.resource::<TestReactRecorder>().0, 1);

    // insert (reaction)
    syscall(&mut world, (test_entity_b, TestComponent(2)), insert_on_test_entity);
    assert_eq!(world.resource::<TestReactRecorder>().0, 2);

    // insert (reaction)
    syscall(&mut world, (test_entity_a, TestComponent(3)), insert_on_test_entity);
    assert_eq!(world.resource::<TestReactRecorder>().0, 3);

    // insert (reaction)
    syscall(&mut world, (test_entity_a, TestComponent(4)), insert_on_test_entity);
    assert_eq!(world.resource::<TestReactRecorder>().0, 4);
}

//-------------------------------------------------------------------------------------------------------------------

#[test]
fn react_entity_muation()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .init_resource::<TestReactRecorder>();
    let mut world = &mut app.world;

    // entities
    let test_entity_a = world.spawn_empty().id();
    let test_entity_b = world.spawn_empty().id();

    // add reactor
    syscall(&mut world, test_entity_a, on_entity_mutation);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // insert (no reaction)
    syscall(&mut world, (test_entity_a, TestComponent(5)), insert_on_test_entity);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // update (reaction)
    syscall(&mut world, (test_entity_a, TestComponent(10)), update_test_entity);
    assert_eq!(world.resource::<TestReactRecorder>().0, 10);

    // update (reaction)
    syscall(&mut world, (test_entity_a, TestComponent(1)), update_test_entity);
    assert_eq!(world.resource::<TestReactRecorder>().0, 1);

    // insert other entity (no reaction)
    syscall(&mut world, (test_entity_b, TestComponent(100)), insert_on_test_entity);
    assert_eq!(world.resource::<TestReactRecorder>().0, 1);

    // update other entity (no reaction)
    syscall(&mut world, (test_entity_b, TestComponent(200)), update_test_entity);
    assert_eq!(world.resource::<TestReactRecorder>().0, 1);
}

//-------------------------------------------------------------------------------------------------------------------

#[test]
fn react_component_mutation()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .init_resource::<TestReactRecorder>();
    let mut world = &mut app.world;

    // entities
    let test_entity_a = world.spawn_empty().id();
    let test_entity_b = world.spawn_empty().id();

    // add reactor
    syscall(&mut world, (), on_mutation);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // insert (no reaction)
    syscall(&mut world, (test_entity_a, TestComponent(1)), insert_on_test_entity);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // insert (no reaction)
    syscall(&mut world, (test_entity_b, TestComponent(2)), insert_on_test_entity);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // update (reaction)
    syscall(&mut world, (test_entity_a, TestComponent(3)), update_test_entity);
    assert_eq!(world.resource::<TestReactRecorder>().0, 3);

    // update (reaction)
    syscall(&mut world, (test_entity_b, TestComponent(4)), update_test_entity);
    assert_eq!(world.resource::<TestReactRecorder>().0, 4);
}

//-------------------------------------------------------------------------------------------------------------------

#[test]
fn react_entity_removal()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .init_resource::<TestReactRecorder>();
    let mut world = &mut app.world;

    // entities
    let test_entity_a = world.spawn_empty().id();
    let test_entity_b = world.spawn_empty().id();

    // add reactor
    syscall(&mut world, test_entity_a, on_entity_removal);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // insert (no reaction)
    syscall(&mut world, (test_entity_a, TestComponent(1)), insert_on_test_entity);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // insert (no reaction)
    syscall(&mut world, (test_entity_b, TestComponent(2)), insert_on_test_entity);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // removal
    syscall(&mut world, test_entity_a, remove_from_test_entity);
    // no immediate reaction
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);
    // check for removals (reaction)
    schedule_removal_and_despawn_reactors(world);
    reaction_tree(world);
    assert_eq!(world.resource::<TestReactRecorder>().0, usize::MAX);

    // removal of already removed (no reaction)
    *world.resource_mut::<TestReactRecorder>() = TestReactRecorder::default();
    syscall(&mut world, test_entity_a, remove_from_test_entity);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // removal of other entity (no reaction)
    syscall(&mut world, test_entity_b, remove_from_test_entity);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);
}

//-------------------------------------------------------------------------------------------------------------------

#[test]
fn react_component_removal()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .init_resource::<TestReactRecorder>();
    let mut world = &mut app.world;

    // entities
    let test_entity_a = world.spawn_empty().id();
    let test_entity_b = world.spawn_empty().id();

    // add reactor
    syscall(&mut world, (), on_removal);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // insert (no reaction)
    syscall(&mut world, (test_entity_a, TestComponent(1)), insert_on_test_entity);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // insert (no reaction)
    syscall(&mut world, (test_entity_b, TestComponent(2)), insert_on_test_entity);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // removal
    syscall(&mut world, test_entity_a, remove_from_test_entity);
    // no immediate reaction
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);
    // check for removals (reaction)
    schedule_removal_and_despawn_reactors(world);
    reaction_tree(world);
    assert_eq!(world.resource::<TestReactRecorder>().0, usize::MAX);
 
    // removal of already removed (no reaction)
    *world.resource_mut::<TestReactRecorder>() = TestReactRecorder::default();
    syscall(&mut world, test_entity_a, remove_from_test_entity);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // removal of other entity
    syscall(&mut world, test_entity_b, remove_from_test_entity);
    // no immediate reaction
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);
    // check for removals (reaction)
    schedule_removal_and_despawn_reactors(world);
    reaction_tree(world);
    assert_eq!(world.resource::<TestReactRecorder>().0, usize::MAX);
}

//-------------------------------------------------------------------------------------------------------------------

#[test]
fn react_entity_despawn()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .init_resource::<TestReactRecorder>();
    let mut world = &mut app.world;

    // entities
    let test_entity_a = world.spawn_empty().id();
    let test_entity_b = world.spawn_empty().id();

    // add reactor
    syscall(&mut world, test_entity_a, on_despawn);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // insert (no reaction)
    syscall(&mut world, (test_entity_a, TestComponent(1)), insert_on_test_entity);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // insert (no reaction)
    syscall(&mut world, (test_entity_b, TestComponent(2)), insert_on_test_entity);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // check for despawns (no reaction before despawn)
    schedule_removal_and_despawn_reactors(world);
    reaction_tree(world);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // despawn (reaction)
    assert!(world.despawn(test_entity_a));
    // no immediate reaction
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);
    // check for despawns (reaction)
    schedule_removal_and_despawn_reactors(world);
    reaction_tree(world);
    assert_eq!(world.resource::<TestReactRecorder>().0, usize::MAX);

    // despawn other entity (no reaction)
    *world.resource_mut::<TestReactRecorder>() = TestReactRecorder::default();
    assert!(world.despawn(test_entity_b));
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);
}

//-------------------------------------------------------------------------------------------------------------------

#[test]
fn react_entity_despawn_multiple_reactors()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .init_resource::<TestReactRecorder>();
    let mut world = &mut app.world;

    // entities
    let test_entity_a = world.spawn_empty().id();
    let test_entity_b = world.spawn_empty().id();

    // add reactor
    syscall(&mut world, test_entity_a, on_despawn);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // add second reactor
    syscall(&mut world, test_entity_a, on_despawn_div2);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // insert (no reaction)
    syscall(&mut world, (test_entity_a, TestComponent(1)), insert_on_test_entity);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // insert (no reaction)
    syscall(&mut world, (test_entity_b, TestComponent(2)), insert_on_test_entity);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // check for despawns (no reaction before despawn)
    schedule_removal_and_despawn_reactors(world);
    reaction_tree(world);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // despawn (reaction)
    assert!(world.despawn(test_entity_a));
    // no immediate reaction
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);
    // check for despawns (reaction)
    schedule_removal_and_despawn_reactors(world);
    reaction_tree(world);
    assert_eq!(world.resource::<TestReactRecorder>().0, usize::MAX / 2);

    // despawn other entity (no reaction)
    *world.resource_mut::<TestReactRecorder>() = TestReactRecorder::default();
    assert!(world.despawn(test_entity_b));
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);
}

//-------------------------------------------------------------------------------------------------------------------
