//! Provides an implementation of a `syn`- and `quote`-compatible syntax item describing a single
//! variant for the shortened enum form used by `enum_dispatch`.
//!
//! Each variant can be either just a type, or a name with a single associated tuple type
//! parameter. In the first form, the name is simply the same as the type. In the second, the name
//! is explicitly specified.

use std::iter::FromIterator;

use quote::{ToTokens, TokenStreamExt};
use syn::spanned::Spanned;

use crate::filter_attrs::FilterAttrs;

pub const ENUM_DISPATCH: &str = "enum_dispatch";
pub const DEREF_ATTRIBUTE: &str = "deref";

/// A structure that can be used to store syntax nformation about an `enum_dispatch` enum variant.
#[derive(Clone)]
pub struct EnumDispatchVariant {
    pub deref: bool,
    pub attrs: Vec<syn::Attribute>,
    pub ident: syn::Ident,
    pub field_attrs: Vec<syn::Attribute>,
    pub ty: syn::Type,
}

/// Allows `EnumDispatchItem`s to be parsed from `String`s or `TokenStream`s.
impl syn::parse::Parse for EnumDispatchVariant {
    fn parse(input: syn::parse::ParseStream) -> syn::parse::Result<Self> {
        let mut deref = false;
        let mut extra_attrs = Vec::new();

        for attr in input.call(syn::Attribute::parse_outer)?.into_iter() {
            match parse_variant_attribute(&attr)? {
                Some(EnumDispatchVariantAttribute::Deref) => {
                    deref = true;
                    // The deref attribute needs to be removed, to avoid it leaking into the
                    // expanded code (where the compiler wouldn't know how to handle it).
                }
                None => {
                    // An attribute handled by some other library
                    extra_attrs.push(attr);
                }
            }
        }

        let ident: syn::Ident = input.parse()?;
        let (field_attrs, ty) = if input.peek(syn::token::Brace) {
            unimplemented!("enum_dispatch variants cannot have braces for arguments");
        } else if input.peek(syn::token::Paren) {
            let input: syn::FieldsUnnamed = input.parse()?;
            let mut fields = input.unnamed.iter();
            let field_1 = fields
                .next()
                .expect("Named enum_dispatch variants must have one unnamed field");
            if fields.next().is_some() {
                panic!("Named enum_dispatch variants can only have one unnamed field");
            }
            (field_1.attrs.clone(), field_1.ty.clone())
        } else {
            (vec![], into_type(ident.clone()))
        };
        Ok(EnumDispatchVariant {
            deref,
            attrs: extra_attrs,
            ident,
            field_attrs,
            ty,
        })
    }
}

/// Allows `EnumDispatchVariant`s to be converted into `TokenStream`s.
impl quote::ToTokens for EnumDispatchVariant {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.append_all(self.attrs.outer());
        // In <EnumDispatchVariant as syn::Parse>::parse, we removed the
        // #[enum_dispatch(deref)] attribute, so that it wouldn't go in the
        // expanded code.
        //
        // However, this function is called by enum_dispatch to cache the enum
        // source code, so that it can be parsed (again) later.  In that case,
        // the second parse should in fact see the attribute, so we put it back
        // just for this purpose.
        if self.deref {
            let deref_attr: syn::Attribute = syn::parse_quote! { #[enum_dispatch(deref)] };
            deref_attr.to_tokens(tokens);
        }
        self.ident.to_tokens(tokens);
        syn::token::Paren::default().surround(tokens, |tokens| {
            tokens.append_all(self.field_attrs.iter());
            self.ty.to_tokens(tokens);
        });
    }
}

/// When expanding shorthand `enum_dispatch` enum syntax, each specified, unnamed type variant must
/// acquire an associated identifier to use for the name of the standard Rust enum variant.
///
/// Note that `proc_macro_attribute`s cannot provide custom syntax parsing. Unless using a
/// function-style procedural macro, each type must already be parseable as a unit enum variant.
/// This rules out, for example, generic types with lifetime or type parameters. For these, an
/// explicitly named variant must be used.
fn into_type(ident: syn::Ident) -> syn::Type {
    syn::Type::Path(syn::TypePath {
        path: syn::Path {
            leading_colon: None,
            segments: syn::punctuated::Punctuated::from_iter(vec![syn::PathSegment {
                arguments: syn::PathArguments::None,
                ident,
            }]),
        },
        qself: None,
    })
}

enum EnumDispatchVariantAttribute {
    Deref,
}

fn parse_variant_attribute(
    attr: &syn::Attribute,
) -> syn::parse::Result<Option<EnumDispatchVariantAttribute>> {
    if attr.meta.path().is_ident(ENUM_DISPATCH) {
        let ident = attr.parse_args::<syn::Ident>()?.to_string();
        match ident.as_str() {
            DEREF_ATTRIBUTE => Ok(Some(EnumDispatchVariantAttribute::Deref)),
            _ => {
                let message = format!(
                    "Expected #[enum_dispatch(...)] but found {}",
                    attr.to_token_stream()
                );
                Err(syn::parse::Error::new(attr.span(), message))
            }
        }
    } else {
        // It's an attribute that we don't handle.
        Ok(None)
    }
}
