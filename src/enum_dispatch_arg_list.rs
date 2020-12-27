//! Provides an implementation of a `syn`- and `quote`-compatible syntax item describing the
//! list of arguments that can be passed to an `#[enum_dispatch(...)]` attribute.

use std::collections::HashMap;

#[derive(Debug)]
pub enum ConstValue {
    Literal(syn::Lit),
    Ident(syn::Path)
}

impl syn::parse::Parse for ConstValue {
    fn parse(input: &syn::parse::ParseBuffer) -> Result<Self, syn::Error> {
        let value = match input.parse() {
            Ok(v) => Self::Literal(v),
            Err(_) => Self::Ident(input.parse()?)
        };
        Ok(value)
    }
}

#[derive(Debug)]
pub struct EnumDispatchArgList {
    pub trait_name: syn::Path,
    pub associated_consts: HashMap<syn::Ident, ConstValue>,
}

impl syn::parse::Parse for EnumDispatchArgList {
    fn parse(input: &syn::parse::ParseBuffer) -> Result<Self, syn::Error> {
        let mut args = EnumDispatchArgList{
            trait_name: input.parse()?,
            associated_consts: HashMap::new()
        };
        let _: Option<syn::Token![ , ]> = input.parse()?;
        while !input.is_empty() {
            let key: syn::Ident = input.parse()?;
            let _: syn::Token![ = ] = input.parse()?;
            let value: ConstValue = input.parse()?;
            let _: Option<syn::Token![ , ]> = input.parse()?;
            args.associated_consts.insert(key, value);
        }
        Ok(args)
    }
}

