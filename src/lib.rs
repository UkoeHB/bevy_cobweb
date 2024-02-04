//documentation
#![doc = include_str!("../README.md")]
#[allow(unused_imports)]
use crate as bevy_cobweb;

//module tree
mod ecs;
mod react;
//mod temp;

//API exports
pub use crate::ecs::*;
pub use crate::react::*;
//pub use crate::temp::*;

pub use bevy_cobweb_derive::*;

pub mod prelude
{
    pub use crate::*;
}
