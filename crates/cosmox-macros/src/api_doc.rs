use proc_macro2::TokenStream;
use quote::quote;
use syn::{ItemFn, spanned::Spanned};

pub fn expand_attr_auto_webapi_doc(input: ItemFn) -> syn::Result<TokenStream> {
    let sig = &input.sig;
    let block = &input.block;
    let span = &input.span();
    let attrs = &input.attrs;
    let file = span.unwrap().file();

    let _split_file_path = file.split('/');
    let tag = _split_file_path
        .clone()
        .rev()
        .peekable()
        .peek()
        .unwrap_or(&"Unknown")
        .replace("_controller", "")
        .replace(".rs", "");

    Ok(quote! {
        #[utoipa::path(
            responses(
                (status = 200, description = "Successfully"),
                (status = 400, description = "Bad Request - The syntax is invalid"),
                (status = 401, description = "Unauthorized - Authentication is required"),
                (status = 403, description = "Forbidden - Insufficient permissions"),
                (status = 404, description = "Not Found - Resource does not exist"),
                (status = 409, description = "Conflict - Resource already exists or state conflict"),
                (status = 500, description = "Internal Server Error"),
            ),
            tag = #tag,
        )]
        #(#attrs)*
        #sig {
            #block
        }
    })
}
