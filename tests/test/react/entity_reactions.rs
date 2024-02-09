//local shortcuts
use bevy_cobweb::prelude::*;
use crate::*;

//third-party shortcuts
use bevy::prelude::*;

//standard shortcuts

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn on_entity_insertion(In(entity): In<Entity>, mut rcommands: ReactCommands) -> RevokeToken
{
    rcommands.on(entity_insertion::<TestComponent>(entity),
            move |world: &mut World| syscall(world, entity, update_test_recorder_with_component)
        )
}

fn on_entity_mutation(In(entity): In<Entity>, mut rcommands: ReactCommands) -> RevokeToken
{
    rcommands.on(entity_mutation::<TestComponent>(entity),
            move |world: &mut World| syscall(world, entity, update_test_recorder_with_component)
        )
}

fn on_entity_removal(In(entity): In<Entity>, mut rcommands: ReactCommands) -> RevokeToken
{
    rcommands.on(entity_removal::<TestComponent>(entity), infinitize_test_recorder)
}

fn on_insertion(mut rcommands: ReactCommands) -> RevokeToken
{
    rcommands.on(insertion::<TestComponent>(), update_test_recorder_on_insertion)
}

fn on_mutation(mut rcommands: ReactCommands) -> RevokeToken
{
    rcommands.on(mutation::<TestComponent>(), update_test_recorder_on_mutation)
}

fn on_removal(mut rcommands: ReactCommands) -> RevokeToken
{
    rcommands.on(removal::<TestComponent>(), |_, world: &mut World| syscall(world, (), infinitize_test_recorder))
}

fn on_despawn_div2(In(entity): In<Entity>, mut rcommands: ReactCommands) -> RevokeToken
{
    rcommands.on(despawn(entity), test_recorder_div2)
}

fn on_despawn(In(entity): In<Entity>, mut rcommands: ReactCommands) -> RevokeToken
{
    rcommands.on(despawn(entity), infinitize_test_recorder)
}

fn on_any_entity_mutation(In(entity): In<Entity>, mut rcommands: ReactCommands) -> RevokeToken
{
    rcommands.on(
            (
                entity_insertion::<TestComponent>(entity),
                entity_mutation::<TestComponent>(entity),
                entity_removal::<TestComponent>(entity),
                despawn(entity)
            ),
            move
            |
                insertion: InsertionEvent<TestComponent>,
                mutation: MutationEvent<TestComponent>,
                removal: RemovalEvent<TestComponent>,
                despawn: DespawnEvent,
                mut recorder: ResMut<TestReactRecorder>
            |
            {
                if let Some(_) = insertion.read()
                {
                    recorder.0 += 1;
                    assert!(mutation.is_empty());
                    assert!(removal.is_empty());
                    assert!(despawn.is_empty());
                }
                if let Some(_) = mutation.read()
                {
                    recorder.0 += 10;
                    assert!(insertion.is_empty());
                    assert!(removal.is_empty());
                    assert!(despawn.is_empty());
                }
                if let Some(_) = removal.read()
                {
                    recorder.0 += 100;
                    assert!(insertion.is_empty());
                    assert!(mutation.is_empty());
                    assert!(despawn.is_empty());
                }
                if let Some(_) = despawn.read()
                {
                    recorder.0 += 1000;
                    assert!(insertion.is_empty());
                    assert!(mutation.is_empty());
                    assert!(removal.is_empty());
                }
            }
        )
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn register_reader_for_insertion_event(In(entity): In<Entity>, mut rcommands: ReactCommands) -> RevokeToken
{
    rcommands.on(entity_insertion::<TestComponent>(entity),
            move
            |
                insertion: InsertionEvent<TestComponent>,
                mutation: MutationEvent<TestComponent>,
                removal: RemovalEvent<TestComponent>,
                despawn: DespawnEvent,
                mut recorder: ResMut<TestReactRecorder>
            |
            {
                assert_eq!(insertion.read().unwrap(), entity);
                assert!(mutation.is_empty());
                assert!(removal.is_empty());
                assert!(despawn.is_empty());
                recorder.0 = 1;
            }
        )
}

fn register_reader_for_mutation_event(In(entity): In<Entity>, mut rcommands: ReactCommands) -> RevokeToken
{
    rcommands.on(entity_mutation::<TestComponent>(entity),
            move
            |
                insertion: InsertionEvent<TestComponent>,
                mutation: MutationEvent<TestComponent>,
                removal: RemovalEvent<TestComponent>,
                despawn: DespawnEvent,
                mut recorder: ResMut<TestReactRecorder>
            |
            {
                assert!(insertion.is_empty());
                assert_eq!(mutation.read().unwrap(), entity);
                assert!(removal.is_empty());
                assert!(despawn.is_empty());
                recorder.0 = 10;
            }
        )
}

fn register_reader_for_removal_event(In(entity): In<Entity>, mut rcommands: ReactCommands) -> RevokeToken
{
    rcommands.on(entity_removal::<TestComponent>(entity),
            move
            |
                insertion: InsertionEvent<TestComponent>,
                mutation: MutationEvent<TestComponent>,
                removal: RemovalEvent<TestComponent>,
                despawn: DespawnEvent,
                mut recorder: ResMut<TestReactRecorder>
            |
            {
                assert!(insertion.is_empty());
                assert!(mutation.is_empty());
                assert_eq!(removal.read().unwrap(), entity);
                assert!(despawn.is_empty());
                recorder.0 = 100;
            }
        )
}

fn register_reader_for_despawn_event(In(entity): In<Entity>, mut rcommands: ReactCommands) -> RevokeToken
{
    rcommands.on(despawn(entity),
            move
            |
                insertion: InsertionEvent<TestComponent>,
                mutation: MutationEvent<TestComponent>,
                removal: RemovalEvent<TestComponent>,
                despawn: DespawnEvent,
                mut recorder: ResMut<TestReactRecorder>
            |
            {
                assert!(insertion.is_empty());
                assert!(mutation.is_empty());
                assert!(removal.is_empty());
                assert_eq!(despawn.read().unwrap(), entity);
                recorder.0 = 1000;
            }
        )
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

// Here we use system commands to do entity modifications separately in case of future command batching.
// - The entire system is inside a system command so that the reactions are all scheduled within the same reaction tree.
fn all_test_entity_mutations(
    In(entity)   : In<Entity>,
    mut commands : Commands,
){
    let inner = commands.spawn_system_command(
        move |mut commands: Commands|
        {
            // insertion
            let insert = commands.spawn_system_command(
                    move |mut rcommands: ReactCommands|
                    {
                        rcommands.insert(entity, TestComponent(0));
                    }
                );
            commands.add(insert);

            // mutation
            let mutate = commands.spawn_system_command(
                    move |mut rcommands: ReactCommands, mut test_entities: Query<&mut React<TestComponent>>|
                    {
                        *test_entities
                            .get_mut(entity)
                            .unwrap()
                            .get_mut(&mut rcommands) = TestComponent(1);
                    }
                );
            commands.add(mutate);

            // removal
            let remove = commands.spawn_system_command(
                    move |mut rcommands: ReactCommands|
                    {
                        rcommands.commands().get_entity(entity).unwrap().remove::<React<TestComponent>>();
                    }
                );
            commands.add(remove);

            // despawn
            let despawn = commands.spawn_system_command(
                    move |world: &mut World|
                    {
                        world.despawn(entity);
                    }
                );
            commands.add(despawn);
        }
    );
    commands.add(inner);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn despawn_other_on_drop(
    In((entity, proxy)) : In<(Entity, Entity)>,
    mut rcommands       : ReactCommands,
    despawner           : Res<AutoDespawner>
){
    let signal = despawner.prepare(proxy);

    rcommands.on(despawn(entity), 
            move ||
            {
                let _ = &signal;
            }
        );
}
//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn dont_despawn_other_on_drop(
    In((entity, proxy)) : In<(Entity, Entity)>,
    mut rcommands       : ReactCommands,
    despawner           : Res<AutoDespawner>
){
    let signal = despawner.prepare(proxy);

    rcommands.on((insertion::<TestComponent>(), despawn(entity)), 
            move ||
            {
                let _ = &signal;
            }
        );
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

#[test]
fn test_entity_insertion()
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
fn component_insertion()
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
fn test_entity_muation()
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
fn component_mutation()
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
fn test_entity_removal()
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
fn component_removal()
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
    reaction_tree(world);
    assert_eq!(world.resource::<TestReactRecorder>().0, usize::MAX);
}

//-------------------------------------------------------------------------------------------------------------------

#[test]
fn entity_despawn()
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
    reaction_tree(world);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // despawn (reaction)
    assert!(world.despawn(test_entity_a));
    // no immediate reaction
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);
    // check for despawns (reaction)
    reaction_tree(world);
    assert_eq!(world.resource::<TestReactRecorder>().0, usize::MAX);

    // despawn other entity (no reaction)
    *world.resource_mut::<TestReactRecorder>() = TestReactRecorder::default();
    assert!(world.despawn(test_entity_b));
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);
}

//-------------------------------------------------------------------------------------------------------------------

#[test]
fn entity_despawn_multiple_reactors()
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
    reaction_tree(world);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // despawn (reaction)
    assert!(world.despawn(test_entity_a));
    // no immediate reaction
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);
    // check for despawns (reaction)
    reaction_tree(world);
    assert_eq!(world.resource::<TestReactRecorder>().0, usize::MAX / 2);

    // despawn other entity (no reaction)
    *world.resource_mut::<TestReactRecorder>() = TestReactRecorder::default();
    assert!(world.despawn(test_entity_b));
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);
}

//-------------------------------------------------------------------------------------------------------------------

// Entity reactions are correctly readable by only their reader: InsertionEvent, RemovalEvent, MutationEvent, DespawnEvent.
#[test]
fn entity_reaction_reader_exclusion()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .init_resource::<TestReactRecorder>();
    let mut world = &mut app.world;

    // entities
    let test_entity = world.spawn_empty().id();

    // add reactors
    syscall(&mut world, test_entity, register_reader_for_insertion_event);
    syscall(&mut world, test_entity, register_reader_for_mutation_event);
    syscall(&mut world, test_entity, register_reader_for_removal_event);
    syscall(&mut world, test_entity, register_reader_for_despawn_event);

    // insert should not panic
    syscall(&mut world, (test_entity, TestComponent(0)), insert_on_test_entity);
    assert_eq!(world.resource::<TestReactRecorder>().0, 1);

    // mutation should not panic
    syscall(&mut world, (test_entity, TestComponent(1)), update_test_entity);
    assert_eq!(world.resource::<TestReactRecorder>().0, 10);

    // removal should not panic
    syscall(&mut world, test_entity, remove_from_test_entity);
    reaction_tree(world);
    assert_eq!(world.resource::<TestReactRecorder>().0, 100);

    // despawn should not panic
    world.despawn(test_entity);
    reaction_tree(world);
    assert_eq!(world.resource::<TestReactRecorder>().0, 1000);
}

//-------------------------------------------------------------------------------------------------------------------

// Multiple entity reactions scheduled in a row do not interfere.
#[test]
fn multiple_entity_reactions_noninterference()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .init_resource::<TestReactRecorder>();
    let mut world = &mut app.world;

    // entities
    let test_entity = world.spawn_empty().id();

    // add reactors
    syscall(&mut world, test_entity, on_any_entity_mutation);

    // perform all entity mutations
    syscall(&mut world, test_entity, all_test_entity_mutations);
    assert_eq!(world.resource::<TestReactRecorder>().0, 1111);
}

//-------------------------------------------------------------------------------------------------------------------

// Reactors registered for only despawns should automatically be dropped after the last despawn.
#[test]
fn despawn_reactor_cleanup()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin);
    let mut world = &mut app.world;

    // entities
    let test_entity = world.spawn_empty().id();
    let proxy_entity = world.spawn_empty().id();

    // add reactors
    syscall(&mut world, (test_entity, proxy_entity), despawn_other_on_drop);

    // despawn the test entity, which should cause the reactor to run and then be dropped, which will despawn the proxy
    world.despawn(test_entity);
    assert!(world.get_entity(proxy_entity).is_some());
    reaction_tree(world);
    assert!(world.get_entity(proxy_entity).is_none());
}

//-------------------------------------------------------------------------------------------------------------------

// Reactors should not be cleaned up if registered for one despawn and a non-despawn trigger.
#[test]
fn despawn_reactor_no_cleanup()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin);
    let mut world = &mut app.world;

    // entities
    let test_entity = world.spawn_empty().id();
    let proxy_entity = world.spawn_empty().id();

    // add reactors
    syscall(&mut world, (test_entity, proxy_entity), dont_despawn_other_on_drop);

    // despawn the test entity, which should cause the reactor to run and then be dropped, which will despawn the proxy
    world.despawn(test_entity);
    assert!(world.get_entity(proxy_entity).is_some());
    reaction_tree(world);
    assert!(world.get_entity(proxy_entity).is_some());
}

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
