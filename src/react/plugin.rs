//local shortcuts
use crate::prelude::*;

//third-party shortcuts
use bevy::prelude::*;
use bevy_fn_plugin::*;

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------

/// Prepares the react framework so that reactors may be registered with [`ReactCommands`].
/// - Un-handled removals and despawns will be automatically processed in `Last`.
#[bevy_plugin]
pub fn ReactPlugin(app: &mut App)
{
    if !app.world.contains_resource::<ReactCache>()
    {
        app.init_resource::<ReactCache>();
    }
    app.init_resource::<CobwebCommandQueue<SystemCommand>>()
        .init_resource::<CobwebCommandQueue<EventCommand>>()
        .init_resource::<CobwebCommandQueue<ReactionCommand>>()
        .init_resource::<SystemEventAccessTracker>()
        .init_resource::<EntityReactionAccessTracker>()
        .init_resource::<EventAccessTracker>()
        .init_resource::<DespawnAccessTracker>()
        .setup_auto_despawn()
        .add_systems(Last, reaction_tree.after(AutoDespawnSet));
}

//-------------------------------------------------------------------------------------------------------------------
