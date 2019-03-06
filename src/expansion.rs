//! Provides a utility for generating `enum_dispatch` impl blocks given `EnumDispatchItem` and
//! `syn::ItemTrait` definitions.
use crate::proc_macro;
use proc_macro2;
use quote::{
    quote,
    ToTokens
};
use syn;
use syn::spanned::Spanned;

use crate::enum_dispatch_item::EnumDispatchItem;
use crate::enum_dispatch_variant::EnumDispatchVariant;

/// Name bound to the single enum field in generated match statements. It doesn't really matter
/// what this is, as long as it's consistent across the left and right sides of generated match
/// arms. For simplicity's sake, the field is bound to this name everywhere it's generated.
const FIELDNAME: &str = "inner";

/// Implements the specified trait for the given enum definition, assuming the trait definition is
/// already present in local storage.
pub fn add_enum_impls(enum_def: EnumDispatchItem, traitdef: syn::ItemTrait) -> proc_macro2::TokenStream {
    let traitname = traitdef.ident;
    let trait_items = traitdef.items;

    let mut impls = proc_macro2::TokenStream::new();

    let variants: Vec<&EnumDispatchVariant> = enum_def.variants.iter().collect();

    let from_impls = generate_from_impls(&enum_def.ident, &variants);
    for from_impl in from_impls.iter() {
        from_impl.to_tokens(&mut impls);
    }

    let fn_impls: Vec<syn::ImplItem> = trait_items
        .iter()
        .filter_map(|trait_item| create_trait_match(trait_item, &enum_def.ident, &variants))
        .collect();
    // If we are missing some trait items then we can't satisfy the trait, but that doesn't mean we
    // should give up! The non-static methods we can provide are still useful!
    let traits_impl = if fn_impls.len() == trait_items.len() {
        format!("impl {} for {} {{ }}", traitname, enum_def.ident)
    } else {
        format!("impl {} {{ }}", enum_def.ident)
    };
    let mut traits_impl: syn::ItemImpl = syn::parse_str(traits_impl.as_str()).unwrap();
    traits_impl.unsafety = traitdef.unsafety;
    traits_impl.generics = traitdef.generics;
    traits_impl.items.extend(fn_impls.into_iter());
    traits_impl.to_tokens(&mut impls);

    impls
}

/// Generates impls of std::convert::From for each enum variant.
fn generate_from_impls(enumname: &syn::Ident, enumvariants: &[&EnumDispatchVariant]) -> Vec<syn::ItemImpl> {
    enumvariants
        .iter()
        .map(|variant| {
            let variant_name = &variant.ident;
            let variant_type = &variant.ty;
            let impl_block = quote! {
                impl ::std::convert::From<#variant_type> for #enumname {
                    fn from(v: #variant_type) -> #enumname {
                        #enumname::#variant_name(v)
                    }
                }
            };
            syn::parse(impl_block.into()).unwrap()
        }).collect()
}

/// Used to keep track of the 'self' arguments in a trait's function signature.
/// Static -> no 'self' arguments
/// ByReference -> &self, &mut self
/// ByValue -> self, mut self
enum MethodType {
    Static,
    ByReference,
    ByValue,
}

/// Parses the arguments of a trait method's signature, returning all non-self arguments as well as
/// a MethodType enum describing the self argument, if present.
fn extract_fn_args(
    trait_args: syn::punctuated::Punctuated<syn::FnArg, syn::token::Comma>,
) -> (
    MethodType,
    syn::punctuated::Punctuated<syn::Expr, syn::token::Comma>,
) {
    // TODO we need to detect methods with signatures dependent on trait member items and skip
    // them! This will avoid a confusing error on the enum implementation we provide.
    let mut method_type = MethodType::Static;
    let new_args: Vec<syn::Ident> = trait_args
        .iter()
        .filter_map(|arg| match arg {
            syn::FnArg::SelfRef(_) => {
                method_type = MethodType::ByReference;
                None
            }
            syn::FnArg::SelfValue(_) => {
                method_type = MethodType::ByValue;
                None
            }
            syn::FnArg::Captured(syn::ArgCaptured {
                pat: syn::Pat::Ident(syn::PatIdent { ident, .. }),
                ..
            }) => Some(ident.to_owned()),
            _ => panic!("Unsupported argument type"),
        }).collect();
    let args = {
        let mut args = syn::punctuated::Punctuated::new();
        new_args.iter().for_each(|arg| {
            args.push(syn::parse_str(arg.to_string().as_str()).unwrap());
        });
        args
    };
    (method_type, args)
}

/// Creates a method call that can be used in the match arms of all non-static method
/// implementations.
fn create_trait_fn_call(trait_method: &syn::TraitItemMethod) -> Option<syn::ExprCall> {
    let trait_args = trait_method.to_owned().sig.decl.inputs;
    let (method_type, args) = extract_fn_args(trait_args);

    match method_type {
        // Concrete enum to match on, it's impossible to tell
        // which variant to call.
        MethodType::Static => None,
        _ => Some(syn::ExprCall {
            attrs: vec![],
            func: {
                let fieldname = syn::Ident::new(FIELDNAME, trait_method.span());
                let trait_method_name = &trait_method.sig.ident;
                Box::new(syn::parse_quote! { #fieldname.#trait_method_name })
            },
            paren_token: Default::default(),
            args,
        }),
    }
}

/// Constructs a match expression that matches on all variants of the specified enum, creating a
/// binding to their single field and calling the provided trait method on each.
fn create_match_expr(
    trait_method: &syn::TraitItemMethod,
    enum_name: &syn::Ident,
    enumvariants: &[&EnumDispatchVariant],
) -> Option<syn::Expr> {
    let trait_fn_call = create_trait_fn_call(trait_method)?;

    // Creates a Vec containing a match arm for every enum variant
    let match_arms = enumvariants
        .iter()
        .map(|variant| {
            let variant_name = &variant.ident;
            syn::Arm {
            attrs: vec![],
            leading_vert: None,
            pats: {
                let mut segments = syn::punctuated::Punctuated::new();
                let fieldname = syn::Ident::new(FIELDNAME, variant.span());
                segments.push(syn::parse_quote! {#enum_name::#variant_name(#fieldname)});
                segments
            },
            guard: None,
            fat_arrow_token: Default::default(),
            body: Box::new(syn::Expr::from(trait_fn_call.to_owned())),
            comma: Some(Default::default()),
        }}).collect();

    // Creates the match expression
    Some(syn::Expr::from(syn::ExprMatch {
        attrs: vec![],
        match_token: Default::default(),
        expr: Box::new(syn::Expr::from(syn::ExprPath {
            attrs: vec![],
            qself: None,
            path: syn::Path {
                leading_colon: None,
                segments: {
                    let mut segments = syn::punctuated::Punctuated::new();
                    segments.push(syn::PathSegment {
                        ident: syn::Ident::new("self", syn::export::Span::call_site()),
                        arguments: syn::PathArguments::None,
                    });
                    segments
                },
            },
        })),
        brace_token: Default::default(),
        arms: match_arms,
    }))
}

/// Builds an implementation of the given trait function for the given enum type.
fn create_trait_match(
    trait_item: &syn::TraitItem,
    enum_name: &syn::Ident,
    enumvariants: &[&EnumDispatchVariant],
) -> Option<syn::ImplItem> {
    match trait_item {
        syn::TraitItem::Method(trait_method) => {
            let match_expr = create_match_expr(&trait_method, enum_name, enumvariants)?;

            Some(syn::ImplItem::Method(syn::ImplItemMethod {
                attrs: vec![syn::Attribute {
                    pound_token: Default::default(),
                    style: syn::AttrStyle::Outer,
                    bracket_token: Default::default(),
                    path: syn::parse_str("inline").unwrap(),
                    tts: proc_macro::TokenStream::new().into(),
                }],
                vis: syn::Visibility::Inherited,
                defaultness: None,
                sig: trait_method.sig.clone(),
                block: syn::Block {
                    brace_token: Default::default(),
                    stmts: vec![syn::Stmt::Expr(match_expr)],
                },
            }))
        }
        // It's not possible to resolve to these types of things from the enum; they are static
        // properties of the trait implementations which means when we refer to them we don't have
        // a Self type to work with.
        syn::TraitItem::Const(_) => None,
        syn::TraitItem::Type(_) => None,
        syn::TraitItem::Macro(_) => None,
        syn::TraitItem::Verbatim(_) => None,
    }
}
