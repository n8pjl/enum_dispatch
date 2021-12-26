//! Provides an implementation of a `syn`- and `quote`-compatible syntax item describing the
//! list of arguments that can be passed to an `#[enum_dispatch(...)]` attribute.

pub struct EnumDispatchArgList {
    pub arg_list: syn::punctuated::Punctuated<syn::Path, syn::token::Comma>,
}

impl syn::parse::Parse for EnumDispatchArgList {
    fn parse(input: &syn::parse::ParseBuffer) -> Result<Self, syn::Error> {
        let arg_list = syn::punctuated::Punctuated::parse_terminated(input)?;
        Ok(Self { arg_list })
    }
}

#[cfg(feature = "extend")]
pub struct EnumDispatchExtendArgList {
    pub ident_trait: syn::Ident,
    pub ident_enum: syn::Ident,
}

#[cfg(feature = "extend")]
impl syn::parse::Parse for EnumDispatchExtendArgList {
    fn parse(input: &syn::parse::ParseBuffer) -> Result<Self, syn::Error> {
        let ident_trait = input.parse()?;
        let _: proc_macro2::Punct = input.parse()?;
        let ident_enum = input.parse()?;
        Ok(Self { ident_trait, ident_enum })
    }
}
