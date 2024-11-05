//local shortcuts
use bevy_cobweb::prelude::*;
use crate::*;

//third-party shortcuts
use bevy::prelude::*;

//standard shortcuts

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn on_entity_mutation_chain_to_res(In(entity): In<Entity>, mut c: Commands)
{
    c.react().on(entity_mutation::<TestComponent>(entity),
            move
            |
                mut c         : Commands,
                mut react_res : ReactResMut<TestReactRes>,
                test_entities : Query<&React<TestComponent>>
            |
            {
                react_res.get_mut(&mut c).0 = test_entities.get(entity).unwrap().0;
            }
        );
}

fn on_broadcast_or_resource(mut c: Commands) -> RevokeToken
{
    c.react().on_revokable((broadcast::<IntEvent>(), resource_mutation::<TestReactRes>()),
        update_test_recorder_with_broadcast_and_resource)
}

fn on_resource_mutation(mut c: Commands) -> RevokeToken
{
    c.react().on_revokable(resource_mutation::<TestReactRes>(), update_test_recorder_with_resource)
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn register_all_reactors(mut c: Commands)
{
    let entity = c.spawn_empty().id();

    c.react().on(
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

/// Here the reactor receives a broadcast event containing its own id, then schedules itself as a follow-up system
/// command. The follow-up should not read the event data.
fn reaction_telescoping_data_visibility_impl(mut c: Commands)
{
    let null_reader = c.spawn_system_command(
            |event: BroadcastEvent<SystemCommand>|
            {
                assert!(event.is_empty());
            }
        );

    let mut count = 0;
    let broadcast_reader = c.spawn_system_command(
            move |mut commands: Commands, event: BroadcastEvent<SystemCommand>|
            {
                match count
                {
                    0 =>
                    {
                        let command = event.read();
                        commands.queue(*command);
                        commands.queue(null_reader);
                        count += 1;
                    }
                    _ =>
                    {
                        assert!(event.is_empty());
                        commands.queue(null_reader);
                    }
                }
            }
        );

    c.react().with(broadcast::<SystemCommand>(), broadcast_reader, ReactorMode::Persistent);
    c.react().broadcast(broadcast_reader);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// Here there are two reactors to broadcasts of `usize`.
/// 
/// - The initial broadcast will schedule reactor 1 and reactor 2.
/// - Reactor 1 will run once, triggering itself and reactor 2.
/// - Reactor 1's recursive trigger will be displaced *after* reactor 2.
/// - Reactor 2 will run.
/// - Reactor 1 will run again, triggering itself and reactor 2.
/// - Etc.
/// - At the end, reactor 2 will run for the initial broadcast.
///
/// Returns the expected event history after the reaction tree is processed.
fn reaction_telescoping_inner_reactions_impl(mut c: Commands) -> Vec<usize>
{
    c.react().on(broadcast::<usize>(),
            move |mut c: Commands, event: BroadcastEvent<usize>, mut history: ResMut<TelescopeHistory>|
            {
                let data = *event.read();
                history.push(data);

                if data == 0 { return; }
                c.react().broadcast(data - 1);
            }
        );
    c.react().on(broadcast::<usize>(),
            move |event: BroadcastEvent<usize>, mut history: ResMut<TelescopeHistory>|
            {
                let data = *event.read();
                history.push(data);
            }
        );

    c.react().broadcast(3usize);

    vec![3, 2, 2, 1, 1, 0, 0, 3]
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
    let world = app.world_mut();

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
    let world = app.world_mut();

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
    let world = app.world_mut();

    // add reactor
    world.syscall((), register_all_reactors);
}

//-------------------------------------------------------------------------------------------------------------------

// Reactions telescope properly.
// - Reaction reader data won't be available to system command recursive invocations of the same reactor, nor to other
//   reactors that can read the same reaction data.
#[test]
fn reaction_telescoping_data_visibility()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin);
    let world = app.world_mut();

    world.syscall((), reaction_telescoping_data_visibility_impl);
}

//-------------------------------------------------------------------------------------------------------------------

// Reactions telescope properly.
// - If a reaction of the same data type is triggered recursively, the reactors for that 'inner reaction' will read the
//   inner data, and then when the pending output reactions run they will read the original data.
#[test]
fn reaction_telescoping_inner_reactions()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .init_resource::<TelescopeHistory>();
    let world = app.world_mut();

    let expected = world.syscall((), reaction_telescoping_inner_reactions_impl);
    assert_eq!(expected, **world.resource::<TelescopeHistory>());
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
    let world = app.world_mut();

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
