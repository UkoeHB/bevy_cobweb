use crate::prelude::*;

//-------------------------------------------------------------------------------------------------------------------

#[derive(Debug)]
pub enum CobwebEcsError
{
    NamedSyscall(SysName)
}

impl std::error::Error for CobwebEcsError
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)>
    {
        None
    }
}

impl std::fmt::Display for CobwebEcsError
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result
    {
        match self {
            Self::NamedSyscall(n) => f.write_fmt(format_args!("NamedSyscall({n:?})")),
        }
    }
}

impl From<CobwebEcsError> for IgnoredError
{
    fn from(_: CobwebEcsError) -> Self
    {
        IgnoredError
    }
}

impl From<CobwebEcsError> for WarnError
{
    fn from(err: CobwebEcsError) -> Self
    {
        WarnError::Msg(format!("CobwebEcsError::{}", err))
    }
}

//-------------------------------------------------------------------------------------------------------------------
