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
    let world = &mut app.world;

    // entities
    let test_entity_a = world.spawn_empty().id();
    let test_entity_b = world.spawn_empty().id();

    // add reactors
    world.syscall(test_entity_a, on_entity_mutation_chain_to_res);
    world.syscall((), on_resource_mutation);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // insert (no reaction)
    world.syscall((test_entity_a, TestComponent(1)), insert_on_test_entity);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // insert other entity (no reaction)
    world.syscall((test_entity_b, TestComponent(2)), insert_on_test_entity);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // update (reaction chain)
    world.syscall((test_entity_a, TestComponent(3)), update_test_entity);
    assert_eq!(world.resource::<TestReactRecorder>().0, 3);

    // update other entity (no reaction reaction)
    world.syscall((test_entity_b, TestComponent(4)), update_test_entity);
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
    let world = &mut app.world;

    // add reactor
    world.syscall((), on_broadcast_or_resource);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // send event (reaction)
    world.syscall(222, send_broadcast);
    assert_eq!(world.resource::<TestReactRecorder>().0, 222);

    // mutate resource (reaction)
    world.syscall(1, update_react_res);
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
    let world = &mut app.world;

    // add reactor
    world.syscall((), register_all_reactors);
}

//-------------------------------------------------------------------------------------------------------------------

// Reactions can be recursive.
//TODO

//-------------------------------------------------------------------------------------------------------------------

// Reactions telescope properly.
// - Reaction reader data won't be available to system command recursive invocations of the same reactor, nor to other
//   reactors that can read the same reaction data.
//TODO

//-------------------------------------------------------------------------------------------------------------------

// Reactions telescope properly.
// - If a reaction of the same data type is triggered recursively, the reactors for that 'inner reaction' will read the
//   inner data, and then when the pending output reactions run they will read the original data.
//TODO

//-------------------------------------------------------------------------------------------------------------------

#[test]
fn revoke_multiple_reactors()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .insert_react_resource(TestReactRes::default())
        .init_resource::<TestReactRecorder>();
    let world = &mut app.world;

    // add reactor
    let revoke_token = world.syscall((), on_broadcast_or_resource);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // send event (reaction)
    world.syscall(222, send_broadcast);
    assert_eq!(world.resource::<TestReactRecorder>().0, 222);

    // mutate resource (reaction)
    world.syscall(1, update_react_res);
    assert_eq!(world.resource::<TestReactRecorder>().0, 223);

    // revoke reactor
    world.syscall(revoke_token, revoke_reactor);

    // mutate resource (no reaction)
    world.syscall(1, update_react_res);
    assert_eq!(world.resource::<TestReactRecorder>().0, 223);
}

//-------------------------------------------------------------------------------------------------------------------
