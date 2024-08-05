//documentation
#![allow(rustdoc::redundant_explicit_links)]
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
mod reaction_trigger;
mod reaction_triggers_impl;
mod syscommand_runner;
mod system_command_spawning;
mod system_event_reader;
mod utils;
mod world_reactor;

//API exports
pub(crate) use command_queue::*;
pub use commands::*;
pub use despawn_reader::*;
pub use entity_reaction_readers::*;
pub use entity_world_reactor::*;
pub use event_readers::*;
pub use extensions::*;
pub use plugin::*;
pub(crate) use react_cache::*;
pub use react_commands::*;
pub use react_component::*;
pub use react_resource::*;
pub use reaction_trigger::*;
pub use reaction_triggers_impl::*;
pub(crate) use syscommand_runner::*;
pub use system_command_spawning::*;
pub use system_event_reader::*;
pub use utils::*;
pub use world_reactor::*;
