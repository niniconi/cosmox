use proc_macro::TokenStream;
use syn::{DeriveInput, ItemFn, parse_macro_input};

use crate::{
  actix_web_error::expand_derive_actix_web_error, api_doc::expand_attr_auto_webapi_doc,
  metadata::expand_derive_metadata,
};

extern crate proc_macro;
mod actix_web_error;
mod api_doc;
mod metadata;
mod utils;

/// For `webapi` documentation generation.
#[proc_macro_attribute]
pub fn auto_webapi_doc(_attr: TokenStream, input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as ItemFn);
  expand_attr_auto_webapi_doc(input)
    .unwrap_or_else(syn::Error::into_compile_error)
    .into()
}

/// Automatically implements `actix-web`'s `ResponseError` trait.
///
/// [`actix_web::error::ResponseError`]
#[proc_macro_derive(ActixWebError, attributes(validator_error_variant, code))]
pub fn actix_web_error_derive(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as DeriveInput);
  expand_derive_actix_web_error(input)
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
