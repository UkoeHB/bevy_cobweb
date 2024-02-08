//test modules
mod entity_reactions;
mod event_reactions;
mod plugin;
mod reactor_combination;
mod resource_reactions;
mod revoking_tokens;

// TODO

// A system command correctly executes the target system.

// System events correctly target the right system.

// Entity reactions are correctly readable by only their reader: InsertionEvent, RemovalEvent, MutationEvent, DespawnEvent.

// Entity events are visible to registered systems only. The EntityEvent reader correctly returns the right entity and data.

// Broadcast events are visible to registered systems only. The BroadcastEvent reader correctly returns the right data.

// Multiple system events scheduled in a row do not interfere.

// Multiple entity reactions scheduled in a row do not interfere.

// Multiple entity events scheduled in a row do not interfere.

// Multiple broadcast events scheduled in a row do not interfere.

// All trigger types can be mixed together in one trigger bundle.

// A system command, system event, and reaction are all executed in that order even when scheduled out of order.

// System commands telescope properly.

// System commands telescope properly taking into account pre-existing commands.

// System events telescope properly.
// - If data is not taken, it won't be available to system command recursive invocations of the same system, nor to
//   other systems that can read the same system event data.

// Reactions telescope properly.
// - Reaction reader data won't be available to system command recursive invocations of the same reactor, nor to other
//   reactors that can read the same reaction data.
// - If a reaction of the same data type is triggered recursively, the reactors for that 'inner reaction' will read the
//   inner data, and then when the pending output reactions run they will read the original data.

// System commands + system events telescope properly.

// System commands + system events + reactions telescope properly.

// Entity reactions, reactive events, and system events should only be visible to the target systems even with telescoping.

// Entity reactions, reactive events, and system events should only be visible to the target systems even with
// potential readers scheduled in commands (cleanup/apply_deferred ordering).

// System commands can be recursive.

// System events can be recursive.

// Reactions can be recursive.

// Reaction data is only despawned after the last reader has run.

// If a system event, entity event, or broadcast event is sent, it should be cleaned up if no systems/reactors run.
