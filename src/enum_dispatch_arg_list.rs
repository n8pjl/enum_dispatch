//! Provides an implementation of a `syn`- and `quote`-compatible syntax item describing the
//! list of arguments that can be passed to an `#[enum_dispatch(...)]` attribute.
use syn::parse::{Parse, ParseStream};
use syn::{Path, Token};
use syn::parse::discouraged::Speculative;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::{Plus, Token};

pub type ListItem = syn::punctuated::Punctuated<syn::Path, syn::token::Comma>;

pub struct ForItem {
    pub traits: syn::punctuated::Punctuated<syn::Path, syn::token::Plus>,
    pub for_token: Token![for],
    pub item: syn::Path,
}

impl Parse for ForItem {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let traits = Punctuated::<Path, Plus>::parse_separated_nonempty(input)?;
        let for_token = input.parse::<Token![for]>()?;
        let item = input.parse::<Path>()?;

        return Ok(ForItem {
            traits,
            for_token,
            item,
        });
    }
}

pub enum EnumDispatchArgList {
    List(ListItem),
    For(ForItem),
}

impl Parse for EnumDispatchArgList {
    fn parse(input: &syn::parse::ParseBuffer) -> Result<Self, syn::Error> {
        let fork = input.fork();
        if let Ok(arg_list) = fork.parse::<ForItem>() {
            input.advance_to(&fork);
            return Ok(Self::For(arg_list));
        }

        let fork = input.fork();
        if let Ok(arg_list) = Punctuated::parse_terminated(&fork) {
            input.advance_to(&fork);
            return Ok(Self::List(arg_list));
        }

        Err(syn::Error::new(input.span(), "Failed to parse arg list: Unknown format"))
    }
}
