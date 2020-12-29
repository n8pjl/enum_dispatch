//! Provides an implementation of a `syn`- and `quote`-compatible syntax item describing the
//! list of arguments that can be passed to an `#[enum_dispatch(...)]` attribute.

use std::collections::HashMap;

#[derive(Debug)]
pub enum ConstValue {
    Literal(syn::Lit),
    Identifier(syn::Path)
}

impl syn::parse::Parse for ConstValue {
    fn parse(input: &syn::parse::ParseBuffer) -> Result<Self, syn::Error> {
        let value = match input.parse() {
            Ok(v) => Self::Literal(v),
            Err(_) => Self::Identifier(input.parse()?)
        };
        Ok(value)
    }
}

#[derive(Debug)]
pub struct EnumDispatchTraitArgs {
    pub associated_consts: HashMap<syn::Ident, (syn::Type, ConstValue)>,
}

impl EnumDispatchTraitArgs {
    pub fn new() -> Self {
        Self{
            associated_consts: HashMap::new()
        }
    }
}

impl syn::parse::Parse for EnumDispatchTraitArgs {
    fn parse(input: &syn::parse::ParseBuffer) -> Result<Self, syn::Error> {
        let mut args = Self::new();
        let content = match syn::group::parse_parens(input) {
            Ok(v) => v.content,
            Err(_) => return Ok(args)
        };
        while !content.is_empty() {
            let name: syn::Ident = content.parse()?;
            let _: syn::token::Colon = content.parse()?;
            let ty: syn::Type = content.parse()?;
            let _: syn::token::Eq = content.parse()?;
            let value: ConstValue = content.parse()?;
            let _: Option<syn::token::Comma> = content.parse()?;
            args.associated_consts.insert(name, (ty, value));
        }
        Ok(args)
    }
}

#[derive(Debug)]
pub struct EnumDispatchArgList {
    pub traits: HashMap<syn::Path, EnumDispatchTraitArgs>,
}

impl syn::parse::Parse for EnumDispatchArgList {
    fn parse(input: &syn::parse::ParseBuffer) -> Result<Self, syn::Error> {
        let mut args = Self{
            traits: HashMap::new()
        };
        while !input.is_empty() {
            let trait_name: syn::Path = input.parse()?;
            let trait_args: EnumDispatchTraitArgs = input.parse()?;
            let _: Option<syn::Token![ , ]> = input.parse()?;
            args.traits.insert(trait_name, trait_args);
        }
        Ok(args)
    }
}

