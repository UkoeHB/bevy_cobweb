//module tree
mod react;

//proc shortcuts
use proc_macro::TokenStream;

//-------------------------------------------------------------------------------------------------------------------

#[proc_macro_derive(ReactComponent)]
pub fn derive_react_component(input: TokenStream) -> TokenStream
{
    react::derive_react_component_impl(input)
}

//-------------------------------------------------------------------------------------------------------------------

#[proc_macro_derive(ReactResource)]
pub fn derive_react_resource(input: TokenStream) -> TokenStream
{
    react::derive_react_resource_impl(input)
}

//-------------------------------------------------------------------------------------------------------------------
