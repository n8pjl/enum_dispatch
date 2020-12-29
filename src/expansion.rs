//! Provides a utility for generating `enum_dispatch` impl blocks given `EnumDispatchItem` and
//! `syn::ItemTrait` definitions.
use crate::{cache, proc_macro};
use quote::{quote, ToTokens};
use syn::spanned::Spanned;

use crate::enum_dispatch_item::EnumDispatchItem;
use crate::enum_dispatch_variant::EnumDispatchVariant;
use crate::syn_utils::plain_identifier_expr;

/// Name bound to the single enum field in generated match statements. It doesn't really matter
/// what this is, as long as it's consistent across the left and right sides of generated match
/// arms. For simplicity's sake, the field is bound to this name everywhere it's generated.
const FIELDNAME: &str = "inner";

/// Implements the specified trait for the given enum definition, assuming the trait definition is
/// already present in local storage.
pub fn add_enum_impls(
    enum_def: EnumDispatchItem,
    traitdef: syn::ItemTrait,
) -> proc_macro2::TokenStream {
    let traitname = traitdef.ident;
    let traitfns = traitdef.items;

    let (generic_impl_constraints, enum_type_generics, where_clause) =
        enum_def.generics.split_for_impl();
    let (_, trait_type_generics, _) = traitdef.generics.split_for_impl();

    let enumname = &enum_def.ident.to_owned();
    let trait_impl = quote! {
        impl #generic_impl_constraints #traitname #trait_type_generics for #enumname #enum_type_generics #where_clause {

        }
    };
    let mut trait_impl: syn::ItemImpl = syn::parse(trait_impl.into()).unwrap();

    trait_impl.unsafety = traitdef.unsafety;

    let variants: Vec<&EnumDispatchVariant> = enum_def.variants.iter().collect();

    for impl_const in &enum_def.consts {
        trait_impl.items.push(syn::ImplItem::Const(impl_const.clone()));
    }

    for trait_fn in traitfns {
        match create_trait_match(
            trait_fn,
            &trait_type_generics,
            &traitname,
            &enum_def.ident,
            &variants,
        ) {
            Some(v) => trait_impl.items.push(v),
            None => ()
        };
    }

    let mut impls = proc_macro2::TokenStream::new();

    // Only generate From impls once per enum_def
    if !cache::conversion_impls_def_by_enum(
        &enum_def.ident,
        enum_def.generics.type_params().count(),
    ) {
        let from_impls = generate_from_impls(&enum_def.ident, &variants, &enum_def.generics);
        for from_impl in from_impls.iter() {
            from_impl.to_tokens(&mut impls);
        }

        let try_into_impls =
            generate_try_into_impls(&enum_def.ident, &variants, &trait_impl.generics);
        for try_into_impl in try_into_impls.iter() {
            try_into_impl.to_tokens(&mut impls);
        }
        cache::cache_enum_conversion_impls_defined(
            enum_def.ident.clone(),
            enum_def.generics.type_params().count(),
        );
    }

    trait_impl.to_tokens(&mut impls);
    impls
}

/// Returns whether or not an attribute from an enum variant should be applied to other usages of
/// that variant's identifier.
fn use_attribute(attr: &&syn::Attribute) -> bool {
    attr.path.is_ident("cfg")
}

/// Generates impls of core::convert::From for each enum variant.
fn generate_from_impls(
    enumname: &syn::Ident,
    enumvariants: &[&EnumDispatchVariant],
    generics: &syn::Generics,
) -> Vec<syn::ItemImpl> {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    enumvariants
        .iter()
        .map(|variant| {
            let variant_name = &variant.ident;
            let variant_type = &variant.ty;
            let attributes = &variant.attrs.iter().filter(use_attribute).collect::<Vec<_>>();
            let impl_block = quote! {
                #(#attributes)*
                impl #impl_generics ::core::convert::From<#variant_type> for #enumname #ty_generics #where_clause {
                    fn from(v: #variant_type) -> #enumname #ty_generics {
                        #enumname::#variant_name(v)
                    }
                }
            };
            syn::parse(impl_block.into()).unwrap()
        }).collect()
}

/// Generates impls of core::convert::TryInto for each enum variant.
fn generate_try_into_impls(
    enumname: &syn::Ident,
    enumvariants: &[&EnumDispatchVariant],
    generics: &syn::Generics,
) -> Vec<syn::ItemImpl> {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    enumvariants
        .iter()
        .enumerate()
        .map(|(i, variant)| {
            let variant_name = &variant.ident;
            let variant_type = &variant.ty;
            let attributes = &variant.attrs.iter().filter(use_attribute).collect::<Vec<_>>();

            // Instead of making a specific match arm for each of the other variants we could just
            // use a catch-all wildcard, but doing it this way means we get nicer error messages
            // that say what the wrong variant is. It also degrades nicely in the case of a single
            // variant enum so we don't get an unsightly "unreachable pattern" warning.
            let other = enumvariants
                .iter()
                .enumerate()
                .filter_map(
                    |(j, other)| if i != j { Some(other) } else { None });
            let other_attributes = other
                .clone()
                .map(|other| {
                    let attrs = other.attrs.iter().filter(use_attribute);
                    quote! { #(#attrs)* }
                });
            let other_idents = other
                .map(|other| other.ident.clone());
            let from_str = other_idents.clone().map(|ident| ident.to_string());
            let to_str = core::iter::repeat(variant_name.to_string());
            let repeated = core::iter::repeat(&enumname);

            let impl_block = quote! {
                #(#attributes)*
                impl #impl_generics core::convert::TryInto<#variant_type> for #enumname #ty_generics #where_clause {
                    type Error = &'static str;
                    fn try_into(self) -> ::core::result::Result<#variant_type, <Self as core::convert::TryInto<#variant_type>>::Error> {
                        match self {
                            #enumname::#variant_name(v) => {Ok(v)},
                            #(  #other_attributes
                                #repeated::#other_idents(v) => {
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
            syn::FnArg::Receiver(syn::Receiver {
                reference: Some(_), ..
            }) => {
                method_type = MethodType::ByReference;
                None
            }
            syn::FnArg::Receiver(syn::Receiver {
                reference: None, ..
            }) => {
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
        })
        .collect();
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
fn create_trait_fn_call(
    trait_method: &syn::TraitItemMethod,
    trait_generics: &syn::TypeGenerics,
    trait_name: &syn::Ident,
) -> syn::ExprCall {
    let trait_args = trait_method.to_owned().sig.inputs;
    let (method_type, mut args) = extract_fn_args(trait_args);

    // Insert FIELDNAME at the beginning of the argument list for UCFS-style method calling
    let explicit_self_arg = syn::Ident::new(FIELDNAME, trait_method.span());
    args.insert(0, plain_identifier_expr(explicit_self_arg));

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
                let method_name = &trait_method.sig.ident;
                let trait_turbofish = trait_generics.as_turbofish();

                let method_type_generics = trait_method.sig.generics.split_for_impl().1;
                let method_turbofish = method_type_generics.as_turbofish();

                Box::new(
                    syn::parse_quote! { #trait_name#trait_turbofish::#method_name#method_turbofish },
                )
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
    trait_generics: &syn::TypeGenerics,
    trait_name: &syn::Ident,
    enum_name: &syn::Ident,
    enumvariants: &[&EnumDispatchVariant],
) -> syn::Expr {
    let trait_fn_call = create_trait_fn_call(trait_method, trait_generics, trait_name);

    // Creates a Vec containing a match arm for every enum variant
    let match_arms = enumvariants
        .iter()
        .map(|variant| {
            let variant_name = &variant.ident;
            let attrs = variant
                .attrs
                .iter()
                .filter(use_attribute)
                .cloned()
                .collect::<Vec<_>>();
            syn::Arm {
                attrs,
                pat: {
                    let fieldname = syn::Ident::new(FIELDNAME, variant.span());
                    syn::parse_quote! {#enum_name::#variant_name(#fieldname)}
                },
                guard: None,
                fat_arrow_token: Default::default(),
                body: Box::new(syn::Expr::from(trait_fn_call.to_owned())),
                comma: Some(Default::default()),
            }
        })
        .collect();

    // Creates the match expression
    syn::Expr::from(syn::ExprMatch {
        attrs: vec![],
        match_token: Default::default(),
        expr: Box::new(plain_identifier_expr(syn::Ident::new(
            "self",
            syn::export::Span::call_site(),
        ))),
        brace_token: Default::default(),
        arms: match_arms,
    })
}

/// Builds an implementation of the given trait function for the given enum type.
fn create_trait_match(
    trait_item: syn::TraitItem,
    trait_generics: &syn::TypeGenerics,
    trait_name: &syn::Ident,
    enum_name: &syn::Ident,
    enumvariants: &[&EnumDispatchVariant],
) -> Option<syn::ImplItem> {
    match trait_item {
        syn::TraitItem::Method(trait_method) => {
            let match_expr = create_match_expr(
                &trait_method,
                trait_generics,
                trait_name,
                enum_name,
                enumvariants,
            );

            let mut impl_attrs = trait_method.attrs.clone();
            // Inline impls - #[inline] is never already specified in a trait method signature
            impl_attrs.push(syn::Attribute {
                pound_token: Default::default(),
                style: syn::AttrStyle::Outer,
                bracket_token: Default::default(),
                path: syn::parse_str("inline").unwrap(),
                tokens: proc_macro::TokenStream::new().into(),
            });

            Some(syn::ImplItem::Method(syn::ImplItemMethod {
                attrs: impl_attrs,
                vis: syn::Visibility::Inherited,
                defaultness: None,
                sig: trait_method.sig,
                block: syn::Block {
                    brace_token: Default::default(),
                    stmts: vec![syn::Stmt::Expr(match_expr)],
                },
            }))
        },
        syn::TraitItem::Const(trait_const) => None,
        _ => panic!("Unsupported trait item"),
    }
}
