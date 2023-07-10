//! Contains helper utilities for parsing items that have been annotated with the `enum_dispatch`
//! procedural macro attribute.
use syn::{File, ItemMod};
use syn::spanned::Spanned;
use crate::enum_dispatch_item;

/// Enumerates all successful results of parsing an `enum_dispatch` annotated syntax block.
#[derive(Clone)]
pub enum ParsedItem {
    Trait(syn::ItemTrait),
    EnumDispatch(enum_dispatch_item::EnumDispatchItem),
    Module(syn::ItemMod),
}

/// Parses any syntax item that was annotated with the `enum_dispatch` attribute and returns its
/// itemized results.
pub fn parse_attributed(item: proc_macro2::TokenStream) -> syn::Result<ParsedItem> {
    if let Ok(enumdef) = syn::parse2(item.clone()) {
        Ok(ParsedItem::EnumDispatch(enumdef))
    } else if let Ok(traitdef) = syn::parse2(item.clone()) {
        Ok(ParsedItem::Trait(traitdef))
    } else if let Ok(moddef) = syn::parse2::<ItemMod>(item.clone()) {
        Ok(ParsedItem::Module(moddef))
    } else {
        Err(syn::Error::new(item.clone().span(), format!("Unknown item {}", item.clone().to_string())))
    }
}
