//! Provides an implementation of a `syn`- and `quote`-compatible syntax item describing the
//! list of arguments that can be passed to an `#[enum_dispatch(...)]` attribute.
use syn::parse::{Parse, ParseStream};
use syn::Path;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Plus;

pub type ListItem = syn::punctuated::Punctuated<syn::Path, syn::token::Comma>;

pub struct ForItem {
    traits : syn::punctuated::Punctuated<syn::Path, syn::token::Plus>,
    for_token : syn::token::For,
    item: syn::Path
}

impl Parse for ForItem {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let traits = Punctuated::<Path, Plus>::parse_separated_nonempty(input)?;
        let for_token = input.parse()?;
        let item = input.parse()?;

        return Ok(ForItem {
            traits, for_token, item
        })

    }
}

pub enum EnumDispatchArgList {
    List (ListItem),
    For (ForItem)
}

impl syn::parse::Parse for EnumDispatchArgList {
    fn parse(input: &syn::parse::ParseBuffer) -> Result<Self, syn::Error> {
        return if let Ok(arg_list) = syn::punctuated::Punctuated::parse_terminated(input) {
            Ok(Self::List(arg_list))
        } else {
            Ok(Self::For(input.parse::<ForItem>()?))
        }
    }
}
