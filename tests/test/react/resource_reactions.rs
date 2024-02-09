//local shortcuts
use bevy_cobweb::prelude::*;
use crate::*;

//third-party shortcuts
use bevy::prelude::*;

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------

#[test]
fn resource_mutation()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .insert_react_resource(TestReactRes::default())
        .init_resource::<TestReactRecorder>();
    let mut world = &mut app.world;

    // add reactor
    syscall(&mut world, (), on_resource_mutation);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // update resource (reaction)
    syscall(&mut world, 100, update_react_res);
    assert_eq!(world.resource::<TestReactRecorder>().0, 100);

    // update resource (reaction)
    syscall(&mut world, 1, update_react_res);
    assert_eq!(world.resource::<TestReactRecorder>().0, 1);
}

//-------------------------------------------------------------------------------------------------------------------

#[test]
fn resource_mutation_once()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .insert_react_resource(TestReactRes::default())
        .init_resource::<TestReactRecorder>();
    let mut world = &mut app.world;

    // add reactor
    syscall(&mut world, (), on_resource_mutation_once);
    assert_eq!(world.resource::<TestReactRecorder>().0, 0);

    // update resource (reaction)
    syscall(&mut world, 100, update_react_res);
    assert_eq!(world.resource::<TestReactRecorder>().0, 100);

    // update resource (no reaction)
    syscall(&mut world, 1, update_react_res);
    assert_eq!(world.resource::<TestReactRecorder>().0, 100);
}

//-------------------------------------------------------------------------------------------------------------------
