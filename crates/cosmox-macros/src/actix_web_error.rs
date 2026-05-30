use proc_macro2::{Delimiter, TokenStream};
use quote::quote;
use syn::{
    Ident, LitInt, Token, braced, parenthesized,
    parse::{Parse, ParseStream},
};

struct ActixWebError {
    name: Ident,
    code: u16,
    delimiter: Delimiter,
}

pub struct ActixWebErrorInput {
    enum_name: Ident,
    variants: Vec<ActixWebError>,
}
impl ActixWebErrorInput {
    pub fn expand(&self) -> syn::Result<TokenStream> {
        let enum_name = &self.enum_name;

        let mut status_code_matchs = Vec::with_capacity(self.variants.len());
        let mut error_response_matchs = Vec::with_capacity(self.variants.len());

        for ActixWebError {
            name,
            code,
            delimiter,
        } in &self.variants
        {
            let parten = match delimiter {
                Delimiter::None => quote! {
                  #enum_name::#name
                },
                Delimiter::Parenthesis => quote! {
                  #enum_name::#name (..)
                },
                Delimiter::Brace => quote! {
                  #enum_name::#name {..}
                },
                _ => unreachable!(),
            };

            status_code_matchs.push(quote! {
              #parten => actix_web::http::StatusCode::from_u16(#code).unwrap()
            });
            error_response_matchs.push(quote! {
              #parten => actix_web::HttpResponse::build(self.status_code()).json(message::Message {
                code: #code.to_string(),
                message: self.to_string(),
                status: status,
                datetime: datetime,
                payload: Option::<message::MessagePayload<u8>>::None,
                pagination: pagination,
              })
            });
        }

        let generated_status_code_fn = quote! {
            fn status_code(&self) -> actix_web::http::StatusCode{
                match self {
                    #(#status_code_matchs, )*
                }
            }
        };
        let generated_error_response_fn = quote! {
            fn error_response(&self, status: String, datetime: chrono::DateTime<chrono::Utc>, pagination: Option<common::message::Pagination>) -> actix_web::HttpResponse {
                match self {
                    #(#error_response_matchs, )*
                }
            }
        };

        let response_error_impl = quote! {
            impl crate::message::InnerResponseError for #enum_name {
                #generated_status_code_fn
                #generated_error_response_fn
            }
        };

        Ok(quote! {
            #response_error_impl
        })
    }
}

impl Parse for ActixWebError {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        let delimiter = if input.peek(syn::token::Paren) {
            let _content;
            parenthesized!(_content in input);
            Delimiter::Parenthesis
        } else if input.peek(syn::token::Brace) {
            let _content;
            braced!(_content in input);
            Delimiter::Brace
        } else {
            Delimiter::None
        };

        input.parse::<Token![=>]>()?;

        let content;
        braced!(content in input);
        let mut actix_err = Self {
            name,
            code: 200,
            delimiter,
        };
        while !content.is_empty() {
            let field: Ident = content.parse()?;
            content.parse::<Token![:]>()?;
            match field.to_string().as_str() {
                "code" => {
                    actix_err.code = content.parse::<LitInt>()?.base10_parse::<u16>()?;
                }
                _ => {
                    return Err(syn::Error::new(
                        field.span(),
                        format!(
                            "Unkown field `{}`, only expected `code`, `msg` field.",
                            field
                        ),
                    ));
                }
            }
            if !content.cursor().eof() {
                content.parse::<Token![,]>()?;
            }
        }

        Ok(actix_err)
    }
}

impl Parse for ActixWebErrorInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let enum_name: Ident = input.parse()?;
        let content;
        braced!(content in input);

        let mut variants = Vec::new();
        while !content.is_empty() {
            variants.push(content.parse()?);
            if !content.cursor().eof() {
                content.parse::<Token![,]>()?;
            }
        }
        Ok(Self {
            enum_name,
            variants,
        })
    }
}
