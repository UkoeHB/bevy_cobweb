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
    rcommands.on_revokable(entity_insertion::<TestComponent>(entity),
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
    let world = &mut app.world;

    // entity
    let test_entity = world.spawn_empty().id();

    // add reactor (should panic)
    world.syscall(test_entity, on_entity_insertion);
}

//-------------------------------------------------------------------------------------------------------------------
