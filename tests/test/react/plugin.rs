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

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

#[test]
#[should_panic]
fn reactor_panic_without_plugin()
{
    // setup
    let mut app = App::new();
    let mut world = &mut app.world;

    // entity
    let test_entity = world.spawn_empty().id();

    // add reactor (should panic)
    syscall(&mut world, test_entity, on_entity_insertion);
}

//-------------------------------------------------------------------------------------------------------------------
