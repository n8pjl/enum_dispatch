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
    let traitfns = traitdef.items;

    let (impl_generics, ty_generics, where_clause) = traitdef.generics.split_for_impl();
    let enumname = &enum_def.ident.to_owned();
    let trait_impl = quote! {
        impl #impl_generics #traitname #ty_generics for #enumname #ty_generics #where_clause {

        }
    };
    let mut trait_impl: syn::ItemImpl = syn::parse(trait_impl.into()).unwrap();

    trait_impl.unsafety = traitdef.unsafety;

    let variants: Vec<&EnumDispatchVariant> = enum_def.variants.iter().collect();

    for trait_fn in traitfns {
        trait_impl
            .items
            .push(create_trait_match(trait_fn, &enum_def.ident, &variants));
    }

    let from_impls = generate_from_impls(&enum_def.ident, &variants, &trait_impl.generics);

    let mut impls = proc_macro2::TokenStream::new();
    for from_impl in from_impls.iter() {
        from_impl.to_tokens(&mut impls);
    }

    let try_from_impls = generate_try_from_impls(&enum_def.ident, &variants, &trait_impl.generics);
    for try_from_impl in try_from_impls.iter() {
        try_from_impl.to_tokens(&mut impls);
    }

    trait_impl.to_tokens(&mut impls);
    impls
}

/// Generates impls of std::convert::From for each enum variant.
fn generate_from_impls(enumname: &syn::Ident, enumvariants: &[&EnumDispatchVariant], generics: &syn::Generics) -> Vec<syn::ItemImpl> {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    enumvariants
        .iter()
        .map(|variant| {
            let variant_name = &variant.ident;
            let variant_type = &variant.ty;
            let impl_block = quote! {
                impl #impl_generics ::std::convert::From<#variant_type> for #enumname #ty_generics #where_clause {
                    fn from(v: #variant_type) -> #enumname #ty_generics {
                        #enumname::#variant_name(v)
                    }
                }
            };
            syn::parse(impl_block.into()).unwrap()
        }).collect()
}

/// Generates impls of std::convert::TryFrom for each enum variant.
fn generate_try_from_impls(enumname: &syn::Ident, enumvariants: &[&EnumDispatchVariant], generics: &syn::Generics) -> Vec<syn::ItemImpl> {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    enumvariants
        .iter()
        .enumerate()
        .map(|(i, variant)| {
            let variant_name = &variant.ident;
            let variant_type = &variant.ty;

            // Instead of making a specific match arm for each of the other variants we could just
            // use a catch-all wildcard, but doing it this way means we get nicer error messages
            // that say what the wrong variant is. It also degrades nicely in the case of a single
            // variant enum so we don't get an unsightly "unreachable pattern" warning.
            let other = enumvariants
                .iter()
                .enumerate()
                .filter_map(
                    |(j, other)| if i != j { Some(&other.ident) } else { None });
            let from_str = other.clone().map(|ident| ident.to_string());
            let to_str = std::iter::repeat(variant_name.to_string());
            let repeated = std::iter::repeat(&enumname);

            let impl_block = quote! {
                impl #impl_generics std::convert::TryFrom<#enumname #ty_generics > for #variant_type #where_clause {
                    type Error = &'static str;
                    fn try_from(e: #enumname #ty_generics ) -> ::std::result::Result<Self, Self::Error> {
                        match e {
                            #enumname::#variant_name(v) => {Ok(v)},
                            #(  #repeated::#other(v) => {
                                Err(concat!("Tried to convert variant ",
                                            #from_str, " to ", #to_str))}    ),*
                        }
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
    let mut method_type = MethodType::Static;
    let new_args: Vec<syn::Ident> = trait_args
        .iter()
        .filter_map(|arg| match arg {
            syn::FnArg::Receiver(syn::Receiver { reference: Some(_), .. }) => {
                method_type = MethodType::ByReference;
                None
            }
            syn::FnArg::Receiver(syn::Receiver { reference: None, .. }) => {
                method_type = MethodType::ByValue;
                None
            }
            syn::FnArg::Typed(syn::PatType { pat, .. }) => {
                if let syn::Pat::Ident(syn::PatIdent { ident, .. }) = &**pat {
                    Some(ident.to_owned())
                } else {
                    panic!("Unsupported argument type")
                }
            }
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
fn create_trait_fn_call(trait_method: &syn::TraitItemMethod) -> syn::ExprCall {
    let trait_args = trait_method.to_owned().sig.inputs;
    let (method_type, args) = extract_fn_args(trait_args);

    syn::ExprCall {
        attrs: vec![],
        func: {
            if let MethodType::Static = method_type {
                // Trait calls can be created when the inner type is known, like this:
                //
                // syn::parse_quote! { #type::#trait_method_name }
                //
                // However, without a concrete enum to match on, it's impossible to tell
                // which variant to call.
                unimplemented!(
                    "Static methods cannot be enum_dispatched (no self argument to match on)"
                );
            } else {
                let fieldname = syn::Ident::new(FIELDNAME, trait_method.span());
                let trait_method_name = &trait_method.sig.ident;
                Box::new(syn::parse_quote! { #fieldname.#trait_method_name })
            }
        },
        paren_token: Default::default(),
        args,
    }
}

/// Constructs a match expression that matches on all variants of the specified enum, creating a
/// binding to their single field and calling the provided trait method on each.
fn create_match_expr(
    trait_method: &syn::TraitItemMethod,
    enum_name: &syn::Ident,
    enumvariants: &[&EnumDispatchVariant],
) -> syn::Expr {
    let trait_fn_call = create_trait_fn_call(trait_method);

    // Creates a Vec containing a match arm for every enum variant
    let match_arms = enumvariants
        .iter()
        .map(|variant| {
            let variant_name = &variant.ident;
            syn::Arm {
                attrs: vec![],
                pat: {
                    let fieldname = syn::Ident::new(FIELDNAME, variant.span());
                    syn::parse_quote! {#enum_name::#variant_name(#fieldname)}
                },
                guard: None,
                fat_arrow_token: Default::default(),
                body: Box::new(syn::Expr::from(trait_fn_call.to_owned())),
                comma: Some(Default::default()),
            }
        }).collect();

    // Creates the match expression
    syn::Expr::from(syn::ExprMatch {
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
    })
}

/// Builds an implementation of the given trait function for the given enum type.
fn create_trait_match(
    trait_item: syn::TraitItem,
    enum_name: &syn::Ident,
    enumvariants: &[&EnumDispatchVariant],
) -> syn::ImplItem {
    match trait_item {
        syn::TraitItem::Method(trait_method) => {
            let match_expr = create_match_expr(&trait_method, enum_name, enumvariants);

            syn::ImplItem::Method(syn::ImplItemMethod {
                attrs: vec![syn::Attribute {
                    pound_token: Default::default(),
                    style: syn::AttrStyle::Outer,
                    bracket_token: Default::default(),
                    path: syn::parse_str("inline").unwrap(),
                    tokens: proc_macro::TokenStream::new().into(),
                }],
                vis: syn::Visibility::Inherited,
                defaultness: None,
                sig: trait_method.sig,
                block: syn::Block {
                    brace_token: Default::default(),
                    stmts: vec![syn::Stmt::Expr(match_expr)],
                },
            })
        }
        _ => panic!("Unsupported trait item"),
    }
}
