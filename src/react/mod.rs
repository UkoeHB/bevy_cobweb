//documentation
#![doc = include_str!("REACT.md")]
#[allow(unused_imports)]
use crate as bevy_cobweb;

//module tree
mod command_queue;
mod commands;
mod despawn_reader;
mod entity_reaction_readers;
mod entity_world_reactor;
mod event_readers;
mod extensions;
mod plugin;
mod react_cache;
mod react_commands;
mod react_component;
mod react_resource;
mod reaction_tree;
mod reaction_trigger;
mod reaction_triggers_impl;
mod system_command_spawning;
mod system_event_reader;
mod utils;
mod world_reactor;

//API exports
pub(crate) use crate::react::command_queue::*;
pub use crate::react::commands::*;
pub use crate::react::despawn_reader::*;
pub use crate::react::entity_reaction_readers::*;
pub use crate::react::entity_world_reactor::*;
pub use crate::react::event_readers::*;
pub use crate::react::extensions::*;
pub use crate::react::plugin::*;
pub(crate) use crate::react::react_cache::*;
pub use crate::react::react_commands::*;
pub use crate::react::react_component::*;
pub use crate::react::react_resource::*;
pub use crate::react::reaction_tree::*;
pub use crate::react::reaction_trigger::*;
pub use crate::react::reaction_triggers_impl::*;
pub use crate::react::system_command_spawning::*;
pub use crate::react::system_event_reader::*;
pub use crate::react::utils::*;
pub use crate::react::world_reactor::*;
