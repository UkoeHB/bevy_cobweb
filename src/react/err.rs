use crate::prelude::*;

use bevy::prelude::*;

//-------------------------------------------------------------------------------------------------------------------

#[derive(Debug)]
pub enum CobwebReactError
{
    DespawnEvent,
    InsertionEvent(&'static str),
    MutationEvent(&'static str),
    RemovalEvent(&'static str),
    BroadcastEvent(&'static str),
    EntityEvent(&'static str),
    Reactive(Entity, &'static str),
    ReactiveMut(Entity, &'static str),
    SystemEvent(&'static str),
}

impl std::error::Error for CobwebReactError
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)>
    {
        None
    }
}

impl std::fmt::Display for CobwebReactError
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result
    {
        match self {
            Self::DespawnEvent => f.write_fmt(format_args!("DespawnEvent")),
            Self::InsertionEvent(t) => f.write_fmt(format_args!("InsertionEvent<{t}>")),
            Self::MutationEvent(t) => f.write_fmt(format_args!("MutationEvent<{t}>")),
            Self::RemovalEvent(t) => f.write_fmt(format_args!("RemovalEvent<{t}>")),
            Self::BroadcastEvent(t) => f.write_fmt(format_args!("BroadcastEvent<{t}>")),
            Self::EntityEvent(t) => f.write_fmt(format_args!("EntityEvent<{t}>")),
            Self::Reactive(entity, t) => f.write_fmt(format_args!("Reactive<{t}>({entity:?})")),
            Self::ReactiveMut(entity, t) => f.write_fmt(format_args!("ReactiveMut<{t}>({entity:?})")),
            Self::SystemEvent(t) => f.write_fmt(format_args!("SystemEvent<{t}>")),
        }
    }
}

impl From<CobwebReactError> for IgnoredError
{
    fn from(_: CobwebReactError) -> Self
    {
        IgnoredError
    }
}

impl From<CobwebReactError> for WarnError
{
    fn from(err: CobwebReactError) -> Self
    {
        WarnError::Msg(format!("CobwebReactError::{}", err))
    }
}

//-------------------------------------------------------------------------------------------------------------------
