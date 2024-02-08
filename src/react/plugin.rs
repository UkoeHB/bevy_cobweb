//local shortcuts
use crate::*;

//third-party shortcuts
use bevy_fn_plugin::*;

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------

/// Queues removal and despawn reactors.
///
/// This system should be scheduled manually if you want to promptly detect removals or despawns that occur after
/// normal systems that don't trigger other reactions.
pub fn schedule_removal_and_despawn_reactors(world: &mut World)
{
    let mut react_cache = world.remove_resource::<ReactCache>().unwrap();
    react_cache.react_to_removals(world);
    react_cache.react_to_despawns(world);
    world.insert_resource(react_cache);
}

//-------------------------------------------------------------------------------------------------------------------

/// Prepares the react framework so that reactors may be registered with [`ReactCommands`].
/// - Un-handled removals and despawns will be automatically processed in `Last`.
#[bevy_plugin]
pub fn ReactPlugin(app: &mut App)
{
    app.init_resource::<ReactCache>()
        .init_resource::<CobwebCommandQueue<SystemCommand>>()
        .init_resource::<CobwebCommandQueue<EventCommand>>()
        .init_resource::<CobwebCommandQueue<ReactionCommand>>()
        .init_resource::<SystemEventAccessTracker>()
        .init_resource::<EntityReactionAccessTracker>()
        .init_resource::<EventAccessTracker>()
        .setup_auto_despawn()
        .add_systems(Last,
            (
                schedule_removal_and_despawn_reactors,
                reaction_tree,
            )
                .chain()
                .after(AutoDespawnSet)
        );
}

//-------------------------------------------------------------------------------------------------------------------
