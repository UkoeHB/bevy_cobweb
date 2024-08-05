//local shortcuts
use bevy_cobweb::prelude::*;
use crate::*;

//third-party shortcuts
use bevy::prelude::*;

//standard shortcuts
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

//-------------------------------------------------------------------------------------------------------------------

// persistent: reactor not despawned if there are no triggers
#[test]
fn persistent_reactor_lives_without_triggers()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin);
    let world = app.world_mut();

    // register reactor
    let sys_command = world.syscall((),
        |mut c: Commands|
        {
            c.react().on_persistent((), ||{})
        }
    );

    // reactor should not be garbage collected
    assert!(world.get_entity(*sys_command).is_some());
    garbage_collect_entities(world);
    schedule_removal_and_despawn_reactors(world);
    assert!(world.get_entity(*sys_command).is_some());
    garbage_collect_entities(world);
    schedule_removal_and_despawn_reactors(world);
    assert!(world.get_entity(*sys_command).is_some());
}

//-------------------------------------------------------------------------------------------------------------------

// persistent: reactor not despawned when all despawn triggers fire
#[test]
fn persistent_reactor_lives_with_despawn_triggers_finished()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin);
    let world = app.world_mut();

    // prep target entity
    let target = world.spawn_empty().id();

    // register reactor
    let sys_command = world.syscall((),
        move |mut c: Commands|
        {
            c.react().on_persistent(despawn(target), ||{})
        }
    );

    // reactor should not be garbage collected
    assert!(world.get_entity(*sys_command).is_some());
    garbage_collect_entities(world);
    schedule_removal_and_despawn_reactors(world);
    assert!(world.get_entity(*sys_command).is_some());
    world.despawn(target);
    garbage_collect_entities(world);
    schedule_removal_and_despawn_reactors(world);
    assert!(world.get_entity(*sys_command).is_some());
}

//-------------------------------------------------------------------------------------------------------------------

// persistent: reactor not despawned when entity-specific reactors dropped after entity is despawned
#[test]
fn persistent_reactor_lives_with_entity_triggers_despawned()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin);
    let world = app.world_mut();

    // prep target entity
    let target = world.spawn_empty().id();

    // register reactor
    let sys_command = world.syscall((),
        move |mut c: Commands|
        {
            c.react().on_persistent(entity_mutation::<TestComponent>(target), ||{})
        }
    );

    // reactor should not be garbage collected
    assert!(world.get_entity(*sys_command).is_some());
    garbage_collect_entities(world);
    schedule_removal_and_despawn_reactors(world);
    assert!(world.get_entity(*sys_command).is_some());
    world.despawn(target);
    garbage_collect_entities(world);
    schedule_removal_and_despawn_reactors(world);
    assert!(world.get_entity(*sys_command).is_some());
}

//-------------------------------------------------------------------------------------------------------------------

// persistent: reactor can acquire more triggers using .with() even when old triggers are despawned
#[test]
fn persistent_reactor_acquires_more_triggers()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin);
    let world = app.world_mut();

    // prep target entity
    let target1 = world.spawn_empty().id();

    // register reactor
    let count = Arc::new(AtomicU32::new(0u32));
    let count_inner = count.clone();
    let sys_command = world.syscall((),
        move |mut c: Commands|
        {
            let count_inner = count_inner.clone();
            c.react().on_persistent(entity_mutation::<TestComponent>(target1),
                move |reader: DespawnEvent|
                {
                    assert!(!reader.is_empty());
                    count_inner.fetch_add(1, Ordering::Relaxed);
                }
            )
        }
    );

    // remove target
    world.despawn(target1);
    garbage_collect_entities(world);
    schedule_removal_and_despawn_reactors(world);

    // prep target entity
    let target2 = world.spawn_empty().id();

    // add more triggers
    world.syscall((),
        move |mut c: Commands|
        {
            c.react().with(despawn(target2), sys_command, ReactorMode::Persistent);
        }
    );

    // despawn new target
    world.despawn(target2);
    garbage_collect_entities(world);
    schedule_removal_and_despawn_reactors(world);
    assert!(world.get_entity(*sys_command).is_some());

    // event should be received
    assert_eq!(count.load(Ordering::Relaxed), 1);
}

//-------------------------------------------------------------------------------------------------------------------

// cleanup: reactor despawned if there are no triggers
#[test]
fn cleanup_reactor_dies_without_triggers()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin);
    let world = app.world_mut();

    // register reactor
    let sys_command = world.syscall((),
        |mut c: Commands|
        {
            let sys_command = c.spawn_system_command(||{});
            c.react().with((), sys_command, ReactorMode::Cleanup);
            sys_command
        }
    );

    // reactor should be garbage collected
    assert!(world.get_entity(*sys_command).is_some());
    garbage_collect_entities(world);
    schedule_removal_and_despawn_reactors(world);
    assert!(world.get_entity(*sys_command).is_none());
}

//-------------------------------------------------------------------------------------------------------------------

// cleanup: reactor despawned when all despawn triggers fire
#[test]
fn cleanup_reactor_dies_with_despawn_triggers_finished()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin);
    let world = app.world_mut();

    // prep target entity
    let target = world.spawn_empty().id();

    // register reactor
    let sys_command = world.syscall((),
        move |mut c: Commands|
        {
            let sys_command = c.spawn_system_command(||{});
            c.react().with(despawn(target), sys_command, ReactorMode::Cleanup);
            sys_command
        }
    );

    // reactor should be garbage collected
    assert!(world.get_entity(*sys_command).is_some());
    garbage_collect_entities(world);
    schedule_removal_and_despawn_reactors(world);
    assert!(world.get_entity(*sys_command).is_some());
    world.despawn(target);
    garbage_collect_entities(world);
    schedule_removal_and_despawn_reactors(world);
    assert!(world.get_entity(*sys_command).is_none());
}

//-------------------------------------------------------------------------------------------------------------------

// cleanup: reactor despawned when entity-specific reactors dropped after entity is despawned
#[test]
fn cleanup_reactor_dies_with_entity_triggers_despawned()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin);
    let world = app.world_mut();

    // prep target entity
    let target = world.spawn_empty().id();

    // register reactor
    let sys_command = world.syscall((),
        move |mut c: Commands|
        {
            let sys_command = c.spawn_system_command(||{});
            c.react().with(entity_mutation::<TestComponent>(target), sys_command, ReactorMode::Cleanup);
            sys_command
        }
    );

    // reactor should be garbage collected
    assert!(world.get_entity(*sys_command).is_some());
    garbage_collect_entities(world);
    schedule_removal_and_despawn_reactors(world);
    assert!(world.get_entity(*sys_command).is_some());
    world.despawn(target);
    garbage_collect_entities(world);
    schedule_removal_and_despawn_reactors(world);
    assert!(world.get_entity(*sys_command).is_none());
}

//-------------------------------------------------------------------------------------------------------------------

// revokable: reactor despawned if there are no triggers
#[test]
fn revokable_reactor_dies_without_triggers()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin);
    let world = app.world_mut();

    // register reactor
    let token = world.syscall((),
        |mut c: Commands|
        {
            c.react().on_revokable((), ||{})
        }
    );

    // reactor should be garbage collected
    assert!(world.get_entity(*SystemCommand::from(token.clone())).is_some());
    garbage_collect_entities(world);
    schedule_removal_and_despawn_reactors(world);
    assert!(world.get_entity(*SystemCommand::from(token)).is_none());
}

//-------------------------------------------------------------------------------------------------------------------

// revokable: reactor despawned when all despawn triggers fire
#[test]
fn revokable_reactor_dies_with_despawn_triggers_finished()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin);
    let world = app.world_mut();

    // prep target entity
    let target = world.spawn_empty().id();

    // register reactor
    let token = world.syscall((),
        move |mut c: Commands|
        {
            c.react().on_revokable(despawn(target), ||{})
        }
    );

    // reactor should be garbage collected
    assert!(world.get_entity(*SystemCommand::from(token.clone())).is_some());
    garbage_collect_entities(world);
    schedule_removal_and_despawn_reactors(world);
    assert!(world.get_entity(*SystemCommand::from(token.clone())).is_some());
    world.despawn(target);
    garbage_collect_entities(world);
    schedule_removal_and_despawn_reactors(world);
    assert!(world.get_entity(*SystemCommand::from(token)).is_none());
}

//-------------------------------------------------------------------------------------------------------------------

// revokable: reactor despawned when entity-specific reactors dropped after entity is despawned
#[test]
fn revokable_reactor_dies_with_entity_triggers_despawned()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin);
    let world = app.world_mut();

    // prep target entity
    let target = world.spawn_empty().id();

    // register reactor
    let token = world.syscall((),
        move |mut c: Commands|
        {
            c.react().on_revokable(entity_mutation::<TestComponent>(target), ||{})
        }
    );

    // reactor should be garbage collected
    assert!(world.get_entity(*SystemCommand::from(token.clone())).is_some());
    garbage_collect_entities(world);
    schedule_removal_and_despawn_reactors(world);
    assert!(world.get_entity(*SystemCommand::from(token.clone())).is_some());
    world.despawn(target);
    garbage_collect_entities(world);
    schedule_removal_and_despawn_reactors(world);
    assert!(world.get_entity(*SystemCommand::from(token)).is_none());
}

//-------------------------------------------------------------------------------------------------------------------

// revokable: reactor despawned when revoked
#[test]
fn revokable_reactor_dies_when_revoked()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin);
    let world = app.world_mut();

    // register reactor
    let token = world.syscall((),
        move |mut c: Commands|
        {
            c.react().on_revokable(broadcast::<()>(), ||{})
        }
    );

    // reactor should be alive
    let reactor_entity = *SystemCommand::from(token.clone());
    assert!(world.get_entity(reactor_entity).is_some());
    garbage_collect_entities(world);
    schedule_removal_and_despawn_reactors(world);
    assert!(world.get_entity(reactor_entity).is_some());

    // revoke the reactor
    world.syscall((),
        move |mut c: Commands|
        {
            c.react().revoke(token.clone());
        }
    );

    // reactor should be garbage collected
    assert!(world.get_entity(reactor_entity).is_some());
    garbage_collect_entities(world);
    schedule_removal_and_despawn_reactors(world);
    assert!(world.get_entity(reactor_entity).is_none());
}

//-------------------------------------------------------------------------------------------------------------------

// revokable: reactor with multiple tokens despawned when one token used to revoke it
#[test]
fn revokable_reactor_dies_when_revoked_with_multiple_tokens()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin);
    let world = app.world_mut();

    // register reactor
    let token1 = world.syscall((),
        move |mut c: Commands|
        {
            c.react().on_revokable(broadcast::<()>(), ||{})
        }
    );

    // reactor should be alive
    let sys_command = SystemCommand::from(token1.clone());
    assert!(world.get_entity(*sys_command).is_some());
    garbage_collect_entities(world);
    schedule_removal_and_despawn_reactors(world);
    assert!(world.get_entity(*sys_command).is_some());

    // add another trigger
    let token2 = world.syscall((),
        move |mut c: Commands|
        {
            c.react().with(broadcast::<usize>(), sys_command, ReactorMode::Revokable).unwrap()
        }
    );
    assert_eq!(sys_command, SystemCommand::from(token2.clone()));

    // reactor should be alive
    assert!(world.get_entity(*sys_command).is_some());
    garbage_collect_entities(world);
    schedule_removal_and_despawn_reactors(world);
    assert!(world.get_entity(*sys_command).is_some());

    // revoke the first reactor
    world.syscall((),
        move |mut c: Commands|
        {
            c.react().revoke(token1.clone());
        }
    );

    // reactor should be garbage collected
    assert!(world.get_entity(*sys_command).is_some());
    garbage_collect_entities(world);
    schedule_removal_and_despawn_reactors(world);
    assert!(world.get_entity(*sys_command).is_none());

    // revoke the second reactor
    world.syscall((),
        move |mut c: Commands|
        {
            c.react().revoke(token2.clone());
        }
    );

    // there should be no effect
    assert!(world.get_entity(*sys_command).is_none());
    garbage_collect_entities(world);
    schedule_removal_and_despawn_reactors(world);
    assert!(world.get_entity(*sys_command).is_none());
}

//-------------------------------------------------------------------------------------------------------------------
