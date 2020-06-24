//! Procedural macros don't offer a good way to store information between macro invocations.  In
//! addition, all syntax-related structures implement `!Send` and `!Sync`, making it impossible to
//! keep them in any sort of static storage. This module uses some workarounds to add that
//! functionality.
//!
//! Fortunately, `TokenStream`s can be converted to and from `String`s, which can be stored
//! statically. Unfortunately, doing so strips any related `Span` information, preventing error
//! messages from being as informative as they could be. For now, it seems this is the best option
//! available.
use quote::ToTokens;

use once_cell::sync::Lazy;

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use crate::enum_dispatch_item;

// Magical storage for trait definitions so that they can be used when parsing other syntax
// structures.
static TRAIT_DEFS: Lazy<Mutex<HashMap<String, String>>> = Lazy::new(|| Mutex::new(HashMap::new()));
static ENUM_DEFS: Lazy<Mutex<HashMap<String, String>>> = Lazy::new(|| Mutex::new(HashMap::new()));
static DEFERRED_LINKS: Lazy<Mutex<HashMap<String, Vec<String>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static ENUM_CONVERSION_IMPLS_DEFS: Lazy<Mutex<HashSet<String>>> =
    Lazy::new(|| Mutex::new(HashSet::new()));

/// Store a trait definition for future reference.
pub fn cache_trait(item: syn::ItemTrait) {
    let identname = item.ident.to_string();
    TRAIT_DEFS
        .lock()
        .unwrap()
        .insert(identname, item.into_token_stream().to_string());
}

/// Store an enum definition for future reference.
pub fn cache_enum_dispatch(item: enum_dispatch_item::EnumDispatchItem) {
    let identname = item.ident.to_string();
    ENUM_DEFS
        .lock()
        .unwrap()
        .insert(identname, item.into_token_stream().to_string());
}

/// Store whether a From/TryInto definition has been defined once for an enum.
pub fn cache_enum_conversion_impls_defined(item: syn::Ident) {
    let identname = item.to_string();
    ENUM_CONVERSION_IMPLS_DEFS.lock().unwrap().insert(identname);
}

/// Cache a "link" to be fulfilled once the needed definition is also cached.
pub fn defer_link(needed: &::proc_macro2::Ident, cached: &::proc_macro2::Ident) {
    let (needed, cached) = (needed.to_string(), cached.to_string());
    let mut deferred_links = DEFERRED_LINKS.lock().unwrap();
    if deferred_links.contains_key(&needed) {
        deferred_links
            .get_mut(&needed)
            .unwrap()
            .push(cached.to_owned());
    } else {
        deferred_links.insert(needed.to_owned(), vec![cached.to_owned()]);
    }
    if deferred_links.contains_key(&cached) {
        deferred_links.get_mut(&cached).unwrap().push(needed);
    } else {
        deferred_links.insert(cached, vec![needed]);
    }
}

/// Returns a list of all of the trait definitions that were previously linked to the supplied enum
/// name.
pub fn fulfilled_by_enum(defname: &::proc_macro2::Ident) -> Vec<syn::ItemTrait> {
    let idents = match DEFERRED_LINKS
        .lock()
        .unwrap()
        .remove_entry(&defname.to_string())
    {
        Some((_, links)) => links,
        None => vec![],
    };
    idents
        .iter()
        .filter_map(
            |ident_string| match TRAIT_DEFS.lock().unwrap().get(ident_string) {
                Some(entry) => Some(syn::parse(entry.parse().unwrap()).unwrap()),
                None => None,
            },
        )
        .collect()
}

/// Returns a list of all of the enum definitions that were previously linked to the supplied trait
/// name.
pub fn fulfilled_by_trait(
    defname: &::proc_macro2::Ident,
) -> Vec<enum_dispatch_item::EnumDispatchItem> {
    let idents = match DEFERRED_LINKS
        .lock()
        .unwrap()
        .remove_entry(&defname.to_string())
    {
        Some((_, links)) => links,
        None => vec![],
    };
    idents
        .iter()
        .filter_map(
            |ident_string| match ENUM_DEFS.lock().unwrap().get(ident_string) {
                Some(entry) => Some(syn::parse(entry.parse().unwrap()).unwrap()),
                None => None,
            },
        )
        .collect()
}

/// Returns true if From/TryInto was already defined for this enum
pub fn conversion_impls_def_by_enum(item: &syn::Ident) -> bool {
    ENUM_CONVERSION_IMPLS_DEFS
        .lock()
        .unwrap()
        .contains(&item.to_string())
}

pub fn remove_entry(defname: &::proc_macro2::Ident) {
    DEFERRED_LINKS
        .lock()
        .unwrap()
        .remove_entry(&defname.to_string());
}
