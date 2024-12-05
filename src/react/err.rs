use bevy::{ecs::{component::RequiredComponentsError, query::{QueryEntityError, QuerySingleError}, world::{error::EntityFetchError, reflect::GetComponentReflectError}}, prelude::*};

//-------------------------------------------------------------------------------------------------------------------

macro_rules! impl_from_for_ignored_error {
    ($target:ty) => {
        impl From<$target> for IgnoredError
        {
            fn from(_: $target) -> Self
            {
                Self
            }
        }
    };
}

//-------------------------------------------------------------------------------------------------------------------

macro_rules! impl_from_for_warn_error {
    ($target:ty) => {
        impl From<$target> for WarnError
        {
            fn from(err: $target) -> Self
            {
                Self::Msg(format!("WarnError=\"{:?}\"", err))
            }
        }
    };
}

//-------------------------------------------------------------------------------------------------------------------

/// Trait for results returned from reactor systems.
/// 
/// Implemented for `()` so plain systems work automatically.
pub trait ReactorResult: Send + Sync + 'static
{
    /// Handles the result.
    fn handle(self, world: &mut World);
}

impl ReactorResult for ()
{
    fn handle(self, _: &mut World) {}
}

//-------------------------------------------------------------------------------------------------------------------

/// Error for [`ReactorResult`] that drops any error passed to it.
#[derive(Debug)]
pub struct IgnoredError;

impl std::error::Error for IgnoredError
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)>
    {
        None
    }
}

impl std::fmt::Display for IgnoredError
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result
    {
        f.write_str("IgnoredError")
    }
}

impl_from_for_ignored_error!(());
impl_from_for_ignored_error!(String);
impl_from_for_ignored_error!(usize);
impl_from_for_ignored_error!(Entity);
impl_from_for_ignored_error!(Vec<Entity>);
impl_from_for_ignored_error!(EntityFetchError);
impl_from_for_ignored_error!(GetComponentReflectError);
impl_from_for_ignored_error!(RequiredComponentsError);
impl_from_for_ignored_error!(QueryEntityError<'_>);
impl_from_for_ignored_error!(QuerySingleError);
impl_from_for_ignored_error!(core::fmt::Error);
impl_from_for_ignored_error!(std::io::Error);
impl_from_for_ignored_error!(Box<dyn std::error::Error>);
impl_from_for_ignored_error!(NoneError);

//-------------------------------------------------------------------------------------------------------------------

/// Implementor of [`ReactorResult`] that drops and ignores all errors reaceived.
/// 
/// Useful for `?` early-out semantics in reactor systems.
/// 
/// Use [`OptionToDropErr::result`] to convert Options into this result type.
pub type DropErr<R = ()> = Result<R, IgnoredError>;

impl ReactorResult for DropErr
{
    fn handle(self, _: &mut World) {}
}

//-------------------------------------------------------------------------------------------------------------------

/// The `Ok` result for [`DropErr<()>`].
/// 
/// Use this at the end of your reactor system that uses `?` early-out semantics.
pub const DONE: DropErr = Ok(());

//-------------------------------------------------------------------------------------------------------------------

/// Error for [`ReactorResult`] that prints a warning with the error passed to it.
#[derive(Debug)]
pub enum WarnError
{
    None,
    Msg(String)
}

impl std::error::Error for WarnError
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)>
    {
        None
    }
}

impl std::fmt::Display for WarnError
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result
    {
        match self {
            Self::None => f.write_str("WarnError::None"),
            Self::Msg(msg) => f.write_str(msg),
        }
    }
}

impl_from_for_warn_error!(());
impl_from_for_warn_error!(String);
impl_from_for_warn_error!(usize);
impl_from_for_warn_error!(Entity);
impl_from_for_warn_error!(Vec<Entity>);
impl_from_for_warn_error!(EntityFetchError);
impl_from_for_warn_error!(GetComponentReflectError);
impl_from_for_warn_error!(RequiredComponentsError);
impl_from_for_warn_error!(QueryEntityError<'_>);
impl_from_for_warn_error!(QuerySingleError);
impl_from_for_warn_error!(core::fmt::Error);
impl_from_for_warn_error!(std::io::Error);
impl_from_for_warn_error!(Box<dyn std::error::Error>);
impl_from_for_warn_error!(NoneError);

//-------------------------------------------------------------------------------------------------------------------

/// Implementor of [`ReactorResult`] that prints a warning when an error is received.
/// 
/// Useful for `?` early-out semantics in reactor systems.
/// 
/// Use [`OptionToWarnErr::result`] to convert Options into this result type.
pub type WarnErr<R = ()> = Result<R, WarnError>;

impl ReactorResult for WarnErr
{
    fn handle(self, _: &mut World)
    {
        if let Err(err) = self {
            tracing::warn!("{err:?}");
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// The `Ok` result for [`WarnErr<()>`].
/// 
/// Use this at the end of your reactor system that uses `?` early-out semantics.
pub const OK: WarnErr = Ok(());

//-------------------------------------------------------------------------------------------------------------------

#[derive(Debug)]
pub struct NoneError;

/// Extension trait for converting `Option<T>` to `Result<T, NoneError>`.
pub trait OptionToNoneErr<T>
{
    fn result(self) -> Result<T, NoneError>;
}

impl<T> OptionToNoneErr<T> for Option<T>
{
    fn result(self) -> Result<T, NoneError>
    {
        self.ok_or(NoneError)
    }
}

//-------------------------------------------------------------------------------------------------------------------
