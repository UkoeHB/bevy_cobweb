//local shortcuts
use bevy_cobweb::prelude::*;
//use crate::*;

//third-party shortcuts
use bevy::prelude::*;

//standard shortcuts

//-------------------------------------------------------------------------------------------------------------------

// persistent: reactor not despawned if there are no triggers
#[test]
fn persistent_reactor_lives_without_triggers()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin);
    let world = &mut app.world;

    // register reactor without any triggers
    let sys_command = world.syscall((),
        |mut rc: ReactCommands|
        {
            rc.on_persistent((), ||{})
        }
    );
    reaction_tree(world);

    // reactor should be alive
    assert!(world.get_entity(*sys_command).is_some());
}

//-------------------------------------------------------------------------------------------------------------------

// persistent: reactor not despawned when all despawn triggers fire

//-------------------------------------------------------------------------------------------------------------------

// persistent: reactor not despawned when entity-specific reactors dropped after entity is despawned

//-------------------------------------------------------------------------------------------------------------------

// persistent: reactor can acquire more triggers using .with()

//-------------------------------------------------------------------------------------------------------------------

// cleanup: reactor despawned if there are no triggers
#[test]
fn cleanup_reactor_dies_without_triggers()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin);
    let world = &mut app.world;

    // register reactor without any triggers
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

//-------------------------------------------------------------------------------------------------------------------

// cleanup: reactor despawned when entity-specific reactors dropped after entity is despawned

//-------------------------------------------------------------------------------------------------------------------

// revokable: reactor despawned if there are no triggers
#[test]
fn revokable_reactor_dies_without_triggers()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin);
    let world = &mut app.world;

    // register reactor without any triggers
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

//-------------------------------------------------------------------------------------------------------------------

// revokable: reactor despawned when entity-specific reactors dropped after entity is despawned

//-------------------------------------------------------------------------------------------------------------------

// revokable: reactor despawned when revoked

//-------------------------------------------------------------------------------------------------------------------
