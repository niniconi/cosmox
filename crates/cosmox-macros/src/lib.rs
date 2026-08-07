use proc_macro::TokenStream;
use syn::{DeriveInput, ItemMod, ItemStruct, parse_macro_input};

use crate::{
    actix_web_error::ActixWebErrorInput, metadata_extend::expand_metadata_extend,
    page::expand_attr_page_helper, plugin::PluginAttr,
};

extern crate proc_macro;
mod actix_web_error;
mod metadata_extend;
mod page;
mod plugin;
mod rkyv_ipc_view;
mod utils;

/// Automatically implements `actix-web`'s `ResponseError` trait.
///
/// [`actix_web::error::ResponseError`]
// #[proc_macro_derive(ActixWebError, attributes(validator_error_variant, code))]
// pub fn actix_web_error_derive(input: TokenStream) -> TokenStream {
//   let input = parse_macro_input!(input as DeriveInput);
//   expand_derive_actix_web_error(input)
//     .unwrap_or_else(syn::Error::into_compile_error)
//     .into()
// }
#[proc_macro]
pub fn actix_web_error(input: TokenStream) -> TokenStream {
    parse_macro_input!(input as ActixWebErrorInput)
        .expand()
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Derive [`cosmox_api::extend::MetadataExtend`] for a struct of
/// extension fields: each field expands to the flat `extend` key
/// `#[extend(key = "...")]:field`. Supports `String`, `Option<String>`
/// and any `Display` + `FromStr` (or `Option<...>` thereof) field types.
#[proc_macro_derive(MetadataExtend, attributes(extend))]
pub fn metadata_extend_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_metadata_extend(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Auto-generate Wax plugin boilerplate from annotated module.
/// Use on a `mod` block. Supports `#[on_load]`, `#[on_event]`, etc.
#[proc_macro_attribute]
pub fn plugin(attr: TokenStream, input: TokenStream) -> TokenStream {
    let attr = parse_macro_input!(attr as PluginAttr);
    let input = parse_macro_input!(input as ItemMod);
    plugin::expand(attr, input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// generate page_helper fields in struct.
#[proc_macro_attribute]
pub fn page_helper(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);
    expand_attr_page_helper(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Generates an rkyv-serializable view struct with destructure-based `From` impl.
///
/// The generated `From` destructures every listed field (view + `skip`). If a listed
/// field is removed or renamed from the source type, compilation fails. `..` is
/// added to the destructure to accommodate SeaORM relation fields that may be
/// inaccessible from other crates — newly added source fields are NOT caught.
///
/// # Syntax
///
/// ```ignore
/// rkyv_ipc_view! {
///   pub struct LibraryView for librarys::Model {
///     pub lid: u64,
///     pub name: Option<String>,
///     #[as_i64]
///     pub create_datetime: i64,
///     skip library_paths,
///     skip users,
///   }
/// }
/// ```
#[proc_macro]
pub fn rkyv_ipc_view(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as rkyv_ipc_view::RkyvIpcViewInput);
    rkyv_ipc_view::expand(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
