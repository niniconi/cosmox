use proc_macro2::TokenStream;
use quote::quote;
use syn::ItemStruct;

pub fn expand_attr_page_helper(input: ItemStruct) -> syn::Result<TokenStream> {
    let attrs = input.attrs;
    let fields = input.fields.iter();
    let ident = input.ident;
    let vis = input.vis;
    let generics = input.generics;

    Ok(quote! {
      #(#attrs)*
      #vis struct #ident #generics{
        #(#fields,)*
        #[serde(rename = "sort_by")]
        pub sort: Option<String>,
        pub page: Option<u64>,
        #[serde(default = "common::default_constants::default_page_size")]
        pub page_size: u64,
      }
    })
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;

    #[test]
    fn test_expand_attr_page_helper() {
        let input: ItemStruct = parse_quote! {
          #[derive(Deserialize, Serialize, Debug)]
          pub struct QueryParams<T,E>{
            pub id: String,
            pub name: String,
            pub generic_t: T,
            pub generic_e: E,
          }
        };
        let expect: TokenStream = quote! {
          #[derive(Deserialize, Serialize, Debug)]
          pub struct QueryParams<T,E>{
            pub id: String,
            pub name: String,
            pub generic_t: T,
            pub generic_e: E,
            #[serde(rename = "sort_by")]
            pub sort: Option<String>,
            pub page: Option<u64>,
            #[serde(default = "common::default_constants::default_page_size")]
            pub page_size: u64,
          }
        };
        let output = expand_attr_page_helper(input).unwrap();
        println!("{}", output);
        assert_eq!(expect.to_string(), output.to_string())
    }

    #[test]
    fn not_deserialize_derive_error() {}
}
