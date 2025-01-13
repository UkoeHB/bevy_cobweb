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
        f.write_fmt(format_args!("{:?}", self))
    }
}

//-------------------------------------------------------------------------------------------------------------------
