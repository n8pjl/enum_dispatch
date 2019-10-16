//! Contains helper utilities for parsing items that have been annotated with the `enum_dispatch`
//! procedural macro attribute.
use crate::proc_macro;
use syn;
use crate::enum_dispatch_item;

/// Enumerates all successful results of parsing an `enum_dispatch` annotated syntax block.
#[derive(Clone)]
pub enum ParsedItem {
    Trait(syn::ItemTrait),
    EnumDispatch(enum_dispatch_item::EnumDispatchItem),
}

/// Parses any syntax item that was annotated with the `enum_dispatch` attribute and returns its
/// itemized results.
pub fn parse_attributed(item: proc_macro::TokenStream) -> Result<ParsedItem, ()> {
    if let Ok(enumdef) = syn::parse(item.clone()) {
        Ok(ParsedItem::EnumDispatch(enumdef))
    } else if let Ok(traitdef) = syn::parse(item) {
        Ok(ParsedItem::Trait(traitdef))
    } else {
        Err(())
    }
}
