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
        f.write_fmt(format_args!("{:?}", self))
    }
}

//-------------------------------------------------------------------------------------------------------------------
