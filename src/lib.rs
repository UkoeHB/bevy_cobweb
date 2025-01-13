//documentation
#![doc = include_str!("../README.md")]
#[allow(unused_imports)]
use crate as bevy_cobweb;

//module tree
pub mod ecs;
pub mod react;
pub mod result;

//API exports
pub use bevy_cobweb_derive::*;

pub mod prelude
{
    pub use crate::*;
    pub use crate::ecs::*;
    pub use crate::react::*;
    pub use crate::result::*;
}
