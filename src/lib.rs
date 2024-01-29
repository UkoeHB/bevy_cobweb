//documentation
#![doc = include_str!("../README.md")]
#[allow(unused_imports)]
use crate as bevy_cobweb;

//module tree
//mod temp;

//API exports
//pub use crate::temp::*;

pub mod prelude
{
    pub use crate::*;
}
