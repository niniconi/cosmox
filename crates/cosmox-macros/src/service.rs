#![allow(unused)]
use proc_macro2::TokenStream;
use quote::quote;
use syn::ItemFn;

pub fn expand(input: ItemFn) -> syn::Result<TokenStream> {
    Ok(quote! {})
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::*;
    #[test]
    fn full_output() {
        let input: ItemFn = parse_quote! {
            pub async service_a(arg1: String, arg2: u64) -> Result<String, Box<dyn Error>> {
                Ok("Service A".to_string())
            }
        };

        let expected = quote! {
            struct ServiceA;
            impl ::Service for ServiceA {
                type Output = Result<String, Box<dyn Error>>;
                async fn call() -> Self::Output {
                    let db = ::get_db_connection().await;
                    Self::call_with_db(db).await
                }
                async fn call_with_db<C: ConnectionTrait>(db: C, f: Fn) -> Self::Output {
                    Ok("Service A".to_string())
                };
            }

            pub async service_a(arg1: String, arg2: u64) -> Result<String, Box<dyn Error>> {
            }
        };
        let output = expand(input).unwrap();
        println!("{}", output);
        assert_eq!(expected.to_string(), output.to_string());
    }
}
