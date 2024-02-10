//local shortcuts
use bevy_cobweb::prelude::*;
use crate::*;

//third-party shortcuts
use bevy::prelude::*;

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------

// A system command, system event, and reaction are all executed in that order even when scheduled out of order.

// System commands + system events telescope properly.

// System commands + system events + reactions telescope properly.

// Entity reactions, reactive events, and system events should only be visible to the target systems even with telescoping.

// Entity reactions, reactive events, and system events should only be visible to the target systems even with
// potential readers scheduled in commands (cleanup/apply_deferred ordering).

// If a system event, entity event, or broadcast event is sent, it should be cleaned up if no systems/reactors run
// because the target system doesn't exist.
