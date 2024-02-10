//local shortcuts
use bevy_cobweb::prelude::*;
use crate::*;

//third-party shortcuts
use bevy::prelude::*;

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn basic_system_command_impl(In(val): In<usize>, mut commands: Commands)
{
    let command = commands.spawn_system_command(
        move |mut recorder: ResMut<TestReactRecorder>|
        {
            recorder.0 = val;
        }
    );
    commands.add(command);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn system_command_telescoping_impl(mut commands: Commands) -> Vec<usize>
{
    let command1 = commands.spawn_system_command(
        |mut history: ResMut<TelescopeHistory>|
        {
            history.push(1);
        }
    );
    let command2 = commands.spawn_system_command(
        move |mut commands: Commands, mut history: ResMut<TelescopeHistory>|
        {
            history.push(2);
            commands.add(command1);
            commands.add(command1);
        }
    );
    let command3 = commands.spawn_system_command(
        move |mut commands: Commands, mut history: ResMut<TelescopeHistory>|
        {
            history.push(3);
            commands.add(command2);
            commands.add(command2);
        }
    );

    let parent = commands.spawn_system_command(
        move |mut commands: Commands, mut history: ResMut<TelescopeHistory>|
        {
            history.push(0);
            commands.add(command1);
            commands.add(command2);
            commands.add(command3);
        }
    );
    commands.add(parent);

    vec![0, 1, 2, 1, 1, 3, 2, 1, 1, 2, 1, 1]
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

#[derive(Resource, Deref, DerefMut)]
struct SavedSystemCommand(Option<SystemCommand>);

fn system_command_recursion_impl(mut commands: Commands) -> Vec<usize>
{
    let command1 = commands.spawn_system_command(
        |mut commands: Commands, mut history: ResMut<TelescopeHistory>, mut saved: ResMut<SavedSystemCommand>|
        {
            match saved.take()
            {
                Some(inner) =>
                {
                    history.push(1);
                    commands.add(inner);
                }
                None =>
                {
                    history.push(2);
                }
            }
        }
    );

    let parent = commands.spawn_system_command(
        move |mut commands: Commands, mut history: ResMut<TelescopeHistory>, mut saved: ResMut<SavedSystemCommand>|
        {
            history.push(0);
            saved.0 = Some(command1);
            commands.add(command1);
        }
    );
    commands.add(parent);

    vec![0, 1, 2]
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

// A system command correctly executes the target system.
#[test]
fn basic_system_command()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .init_resource::<TestReactRecorder>();
    let world = &mut app.world;

    world.syscall(1, basic_system_command_impl);
    assert_eq!(1, world.resource::<TestReactRecorder>().0);
}

//-------------------------------------------------------------------------------------------------------------------

// System commands telescope properly.
#[test]
fn system_command_telescoping()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .init_resource::<TelescopeHistory>();
    let world = &mut app.world;

    let expected = world.syscall((), system_command_telescoping_impl);
    assert_eq!(expected, **world.resource::<TelescopeHistory>());
}

//-------------------------------------------------------------------------------------------------------------------

// System commands can be recursive.
#[test]
fn system_command_recursion()
{
    // setup
    let mut app = App::new();
    app.add_plugins(ReactPlugin)
        .init_resource::<TelescopeHistory>()
        .insert_resource(SavedSystemCommand(None));
    let world = &mut app.world;

    let expected = world.syscall((), system_command_recursion_impl);
    assert_eq!(expected, **world.resource::<TelescopeHistory>());
}

//-------------------------------------------------------------------------------------------------------------------
