//local shortcuts
use bevy_cobweb::prelude::*;
use crate::*;

//third-party shortcuts
use bevy::prelude::*;

//standard shortcuts


//-------------------------------------------------------------------------------------------------------------------

// System events correctly target the right system.

// Multiple system events scheduled in a row do not interfere.

// System events telescope properly.
// - If data is not taken, it won't be available to system command recursive invocations of the same system, nor to
//   other systems that can read the same system event data.

// System events can be recursive.

// System event data is despawned after the target system runs.
