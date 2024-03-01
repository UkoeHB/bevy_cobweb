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
    let world = &mut app.world;

    // register reactor
    let sys_command = world.syscall((),
        |mut rc: ReactCommands|
        {
            rc.on_persistent((), ||{})
        }
    );

    // reactor should not be garbage collected
    assert!(world.get_entity(*sys_command).is_some());
    reaction_tree(world);
    assert!(world.get_entity(*sys_command).is_some());
    reaction_tree(world);
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
    let world = &mut app.world;

    // prep target entity
    let target = world.spawn_empty().id();

    // register reactor
    let sys_command = world.syscall((),
        move |mut rc: ReactCommands|
        {
            rc.on_persistent(despawn(target), ||{})
        }
    );

    // reactor should not be garbage collected
    assert!(world.get_entity(*sys_command).is_some());
    reaction_tree(world);
    assert!(world.get_entity(*sys_command).is_some());
    world.despawn(target);
    reaction_tree(world);
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
    let world = &mut app.world;

    // prep target entity
    let target = world.spawn_empty().id();

    // register reactor
    let sys_command = world.syscall((),
        move |mut rc: ReactCommands|
        {
            rc.on_persistent(entity_mutation::<TestComponent>(target), ||{})
        }
    );

    // reactor should not be garbage collected
    assert!(world.get_entity(*sys_command).is_some());
    reaction_tree(world);
    assert!(world.get_entity(*sys_command).is_some());
    world.despawn(target);
    reaction_tree(world);
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
    let world = &mut app.world;

    // prep target entity
    let target1 = world.spawn_empty().id();

    // register reactor
    let count = Arc::new(AtomicU32::new(0u32));
    let count_inner = count.clone();
    let sys_command = world.syscall((),
        move |mut rc: ReactCommands|
        {
            let count_inner = count_inner.clone();
            rc.on_persistent(entity_mutation::<TestComponent>(target1),
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
    reaction_tree(world);

    // prep target entity
    let target2 = world.spawn_empty().id();

    // add more triggers
    world.syscall((),
        move |mut rc: ReactCommands|
        {
            rc.with(despawn(target2), sys_command, ReactorMode::Persistent);
        }
    );

    // despawn new target
    world.despawn(target2);
    reaction_tree(world);
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
    let world = &mut app.world;

    // register reactor
    let sys_command = world.syscall((),
        |mut rc: ReactCommands|
        {
            let sys_command = rc.commands().spawn_system_command(||{});
            rc.with((), sys_command, ReactorMode::Cleanup);
            sys_command
        }
    );

    // reactor should be garbage collected
    assert!(world.get_entity(*sys_command).is_some());
    reaction_tree(world);
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
    let world = &mut app.world;

    // prep target entity
    let target = world.spawn_empty().id();

    // register reactor
    let sys_command = world.syscall((),
        move |mut rc: ReactCommands|
        {
            let sys_command = rc.commands().spawn_system_command(||{});
            rc.with(despawn(target), sys_command, ReactorMode::Cleanup);
            sys_command
        }
    );

    // reactor should be garbage collected
    assert!(world.get_entity(*sys_command).is_some());
    reaction_tree(world);
    assert!(world.get_entity(*sys_command).is_some());
    world.despawn(target);
    reaction_tree(world);
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
    let world = &mut app.world;

    // prep target entity
    let target = world.spawn_empty().id();

    // register reactor
    let sys_command = world.syscall((),
        move |mut rc: ReactCommands|
        {
            let sys_command = rc.commands().spawn_system_command(||{});
            rc.with(entity_mutation::<TestComponent>(target), sys_command, ReactorMode::Cleanup);
            sys_command
        }
    );

    // reactor should be garbage collected
    assert!(world.get_entity(*sys_command).is_some());
    reaction_tree(world);
    assert!(world.get_entity(*sys_command).is_some());
    world.despawn(target);
    reaction_tree(world);
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
    let world = &mut app.world;

    // register reactor
    let token = world.syscall((),
        |mut rc: ReactCommands|
        {
            rc.on_revokable((), ||{})
        }
    );

    // reactor should be garbage collected
    assert!(world.get_entity(*SystemCommand::from(token.clone())).is_some());
    reaction_tree(world);
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
    let world = &mut app.world;

    // prep target entity
    let target = world.spawn_empty().id();

    // register reactor
    let token = world.syscall((),
        move |mut rc: ReactCommands|
        {
            rc.on_revokable(despawn(target), ||{})
        }
    );

    // reactor should be garbage collected
    assert!(world.get_entity(*SystemCommand::from(token.clone())).is_some());
    reaction_tree(world);
    assert!(world.get_entity(*SystemCommand::from(token.clone())).is_some());
    world.despawn(target);
    reaction_tree(world);
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
    let world = &mut app.world;

    // prep target entity
    let target = world.spawn_empty().id();

    // register reactor
    let token = world.syscall((),
        move |mut rc: ReactCommands|
        {
            rc.on_revokable(entity_mutation::<TestComponent>(target), ||{})
        }
    );

    // reactor should be garbage collected
    assert!(world.get_entity(*SystemCommand::from(token.clone())).is_some());
    reaction_tree(world);
    assert!(world.get_entity(*SystemCommand::from(token.clone())).is_some());
    world.despawn(target);
    reaction_tree(world);
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
    let world = &mut app.world;

    // register reactor
    let token = world.syscall((),
        move |mut rc: ReactCommands|
        {
            rc.on_revokable(broadcast::<()>(), ||{})
        }
    );

    // reactor should be alive
    let reactor_entity = *SystemCommand::from(token.clone());
    assert!(world.get_entity(reactor_entity).is_some());
    reaction_tree(world);
    assert!(world.get_entity(reactor_entity).is_some());

    // revoke the reactor
    world.syscall((),
        move |mut rc: ReactCommands|
        {
            rc.revoke(token.clone());
        }
    );

    // reactor should be garbage collected
    assert!(world.get_entity(reactor_entity).is_some());
    reaction_tree(world);
    assert!(world.get_entity(reactor_entity).is_none());
}

//-------------------------------------------------------------------------------------------------------------------

// revokable: reactor with multiple tokens despawned when one token used to revoke it

//-------------------------------------------------------------------------------------------------------------------
