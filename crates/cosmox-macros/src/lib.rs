use proc_macro::TokenStream;
use syn::{DeriveInput, ItemStruct, parse_macro_input};

use crate::{
    actix_web_error::ActixWebErrorInput, metadata::expand_derive_metadata,
    page::expand_attr_page_helper,
};

extern crate proc_macro;
mod actix_web_error;
mod metadata;
mod page;
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

/// For automatically implementing the extension fields in `Metadata`.
///
/// [`cosmox_api::metadata::Metadata`]
#[proc_macro_derive(Metdata)]
pub fn metdata_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_derive_metadata(input)
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
