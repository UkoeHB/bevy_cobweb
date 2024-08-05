//local shortcuts
use crate::prelude::*;

//third-party shortcuts
use bevy::prelude::*;

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------

/// Tracks position within a tree of system commands.
///
/// Used to identify deferred recursive system commands that need to be discarded.
#[derive(Resource, Default, Debug, Deref, DerefMut)]
pub(crate) struct SyscommandCounter(usize);

//-------------------------------------------------------------------------------------------------------------------

/// Prepares the react framework so that reactors may be registered with [`ReactCommands`].
/// - Un-handled removals and despawns will be automatically processed in `Last`.
pub struct ReactPlugin;

impl Plugin for ReactPlugin
{
    fn build(&self, app: &mut App)
    {
        if !app.world().contains_resource::<ReactCache>()
        {
            app.init_resource::<ReactCache>();
        }
        app.init_resource::<CobwebCommandQueue<BufferedSyscommand>>()
            .init_resource::<SyscommandCounter>()
            .init_resource::<SystemEventAccessTracker>()
            .init_resource::<EntityReactionAccessTracker>()
            .init_resource::<EventAccessTracker>()
            .init_resource::<DespawnAccessTracker>()
            .setup_auto_despawn()
            .add_systems(Last, schedule_removal_and_despawn_reactors.after(AutoDespawnSet));
    }
}

//-------------------------------------------------------------------------------------------------------------------
