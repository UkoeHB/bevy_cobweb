//local shortcuts
use bevy_cobweb::prelude::*;
use crate::*;

//third-party shortcuts
use bevy::prelude::*;

//standard shortcuts

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn on_entity_mutation_chain_to_res(In(entity): In<Entity>, mut rcommands: ReactCommands)
{
    rcommands.on(entity_mutation::<TestComponent>(entity),
            move
            |
                mut rcommands : ReactCommands,
                mut react_res : ReactResMut<TestReactRes>,
                test_entities : Query<&React<TestComponent>>
            |
            {
                react_res.get_mut(&mut rcommands).0 = test_entities.get(entity).unwrap().0;
            }
        );
}

fn on_broadcast_or_resource(mut rcommands: ReactCommands) -> RevokeToken
{
    rcommands.on((broadcast::<IntEvent>(), resource_mutation::<TestReactRes>()),
        update_test_recorder_with_broadcast_and_resource)
}

fn on_resource_mutation(mut rcommands: ReactCommands) -> RevokeToken
{
    rcommands.on(resource_mutation::<TestReactRes>(), update_test_recorder_with_resource)
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn register_all_reactors(mut rcommands: ReactCommands)
{
    let entity = rcommands.commands().spawn_empty().id();

    rcommands.on(
            (
                resource_mutation::<TestReactRes>(),
                insertion::<TestComponent>(),
                mutation::<TestComponent>(),
                removal::<TestComponent>(),
                entity_insertion::<TestComponent>(entity),
                entity_mutation::<TestComponent>(entity),
                entity_removal::<TestComponent>(entity),
                despawn(entity),
                broadcast::<()>(),
                entity_event::<()>(entity),
            ),
            || {}
        );
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

//react chain: component mutation into resource mutation
#[test]
fn mutation_chain()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .insert_react_resource(TestReactRes::default())
        .init_resource::<TestReactRecorder>();
    let mut world = &mut app.world;

    // entities
    let test_entity_a = world.spawn_empty().id();
    let test_entity_b = world.spawn_empty().id();

    // add reactors
    syscall(&mut world, test_entity_a, on_entity_mutation_chain_to_res);
    syscall(&mut world, (), on_resource_mutation);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // insert (no reaction)
    syscall(&mut world, (test_entity_a, TestComponent(1)), insert_on_test_entity);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // insert other entity (no reaction)
    syscall(&mut world, (test_entity_b, TestComponent(2)), insert_on_test_entity);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // update (reaction chain)
    syscall(&mut world, (test_entity_a, TestComponent(3)), update_test_entity);
    assert_eq!(world.resource::<TestReactRecorder>().0, 3);

    // update other entity (no reaction reaction)
    syscall(&mut world, (test_entity_b, TestComponent(4)), update_test_entity);
    assert_eq!(world.resource::<TestReactRecorder>().0, 3);
}

//-------------------------------------------------------------------------------------------------------------------

#[test]
fn multiple_reactors()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .insert_react_resource(TestReactRes::default())
        .init_resource::<TestReactRecorder>();
    let mut world = &mut app.world;

    // add reactor
    syscall(&mut world, (), on_broadcast_or_resource);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // send event (reaction)
    syscall(&mut world, 222, send_broadcast);
    assert_eq!(world.resource::<TestReactRecorder>().0, 222);

    // mutate resource (reaction)
    syscall(&mut world, 1, update_react_res);
    assert_eq!(world.resource::<TestReactRecorder>().0, 223);
}

//-------------------------------------------------------------------------------------------------------------------

// All trigger types can be mixed together in one trigger bundle.
#[test]
fn all_reactors()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin);
    let mut world = &mut app.world;

    // add reactor
    syscall(&mut world, (), register_all_reactors);
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
