//local shortcuts
use bevy_cobweb::prelude::*;

//third-party shortcuts
use bevy::prelude::*;

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

#[derive(Component)]
struct TestComponent;

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn spawn_test_entity(mut commands: Commands, despawner: Res<AutoDespawner>) -> AutoDespawnSignal
{
    let entity = commands.spawn(TestComponent);
    despawner.prepare(entity.id())
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn count_entities(num: Query<(), With<TestComponent>>) -> usize
{
    num.iter().count()
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

#[test]
fn auto_despawn_single()
{
    let mut app = App::new();
    app.setup_auto_despawn();

    // pre-entity
    assert_eq!(syscall(app.world_mut(), (), count_entities), 0);

    // add entity
    let _handle = syscall(app.world_mut(), (), spawn_test_entity);
    assert_eq!(syscall(app.world_mut(), (), count_entities), 1);

    // update app
    app.update();
    assert_eq!(syscall(app.world_mut(), (), count_entities), 1);  // entity survives because handle isn't dropped

    // drop handle
    std::mem::drop(_handle);
    assert_eq!(syscall(app.world_mut(), (), count_entities), 1);
    app.update();
    assert_eq!(syscall(app.world_mut(), (), count_entities), 0);  // entity dies now that the handle was dropped
}

//-------------------------------------------------------------------------------------------------------------------

#[test]
fn auto_despawn_clone()
{
    let mut app = App::new();
    app.setup_auto_despawn();

    // pre-entity
    assert_eq!(syscall(app.world_mut(), (), count_entities), 0);

    // add entity
    let _handle = syscall(app.world_mut(), (), spawn_test_entity);
    let _handle_clone = _handle.clone();
    assert_eq!(syscall(app.world_mut(), (), count_entities), 1);

    // update app
    app.update();
    assert_eq!(syscall(app.world_mut(), (), count_entities), 1);  // entity survives because handle isn't dropped

    // drop handle
    std::mem::drop(_handle);
    assert_eq!(syscall(app.world_mut(), (), count_entities), 1);
    app.update();
    assert_eq!(syscall(app.world_mut(), (), count_entities), 1);  // entity survives because there is a signal clone

    // drop handle clone
    std::mem::drop(_handle_clone);
    assert_eq!(syscall(app.world_mut(), (), count_entities), 1);
    app.update();
    assert_eq!(syscall(app.world_mut(), (), count_entities), 0);  // entity dies now that all handles were dropped
}

//-------------------------------------------------------------------------------------------------------------------

#[test]
fn auto_despawn_multiple()
{
    let mut app = App::new();
    app.setup_auto_despawn();

    // pre-entities
    assert_eq!(syscall(app.world_mut(), (), count_entities), 0);

    // add entities
    let _handle1 = syscall(app.world_mut(), (), spawn_test_entity);
    let _handle2 = syscall(app.world_mut(), (), spawn_test_entity);
    assert_eq!(syscall(app.world_mut(), (), count_entities), 2);

    // update app
    app.update();
    assert_eq!(syscall(app.world_mut(), (), count_entities), 2);

    // drop one entity
    std::mem::drop(_handle1);
    assert_eq!(syscall(app.world_mut(), (), count_entities), 2);
    app.update();
    assert_eq!(syscall(app.world_mut(), (), count_entities), 1);

    // drop other entity
    std::mem::drop(_handle2);
    assert_eq!(syscall(app.world_mut(), (), count_entities), 1);
    app.update();
    assert_eq!(syscall(app.world_mut(), (), count_entities), 0);
}

//-------------------------------------------------------------------------------------------------------------------
