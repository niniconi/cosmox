use std::cmp::min;

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{Data, DeriveInput, Fields, Ident};

use crate::utils;

/*
TODO: impl this attr
`#[validator_error_variant]`

```rust
#name::#validator_error_variant(err) => {
    let field_errors_map = err.field_errors().iter().map(|(field, errors)| {
        let messages: Vec<String> = errors.iter().map(|e| e.message.as_ref().map_or("Invalid value".to_string(), |m| m.to_string())).collect();
        json!({
            "field": field,
            "messages": messages
        })
    }).collect::<Vec<serde_json::Value>>();

    HttpResponse::build(StatusCode::BAD_REQUEST).json(json!({
        "code": "VALIDATION_ERROR",
        "message": "One or more input fields failed validation.",
        "details": { "fields": field_errors_map }
    }))
},
```
*/

pub fn expand_derive_actix_web_error(input: DeriveInput) -> syn::Result<TokenStream> {
  let name = &input.ident;

  let enum_data = match input.data {
    Data::Enum(data_enum) => data_enum,
    _ => {
      return Err(syn::Error::new_spanned(
        &input,
        "WebValidatorError can only be derived for enums",
      ));
    }
  };

  let mut status_code_matchs = Vec::with_capacity(enum_data.variants.len());
  let mut error_response_matchs = Vec::with_capacity(enum_data.variants.len());

  for variant in enum_data.variants {
    let variant_name = &variant.ident;

    let status_code =
      if let Some(status_code) = &variant.attrs.iter().find(|x| x.path().is_ident("code")) {
        if let Ok(syn::Lit::Int(lit_int)) = &status_code.parse_args::<syn::Lit>()
          && let Ok(num) = lit_int.base10_parse::<u16>()
        {
          quote! {
             actix_web::http::StatusCode::from_u16(#num).unwrap()
          }
        } else {
          return Err(syn::Error::new_spanned(
            variant.to_token_stream(),
            format!("At variant `{variant_name}`, the code var must be `u16` literal"),
          ));
        }
      } else {
        return Err(syn::Error::new_spanned(
          variant.to_token_stream(),
          format!("Variant `{variant_name}` not found attr code"),
        ));
      };

    let mut format_text_cnt = 0;
    let format_text = if let Some(error_attr) =
      &variant.attrs.iter().find(|x| x.path().is_ident("error"))
      && let Ok(syn::Lit::Str(lit_str)) = &error_attr.parse_args::<syn::Lit>()
    {
      if let Ok(cnt) = utils::count_format_args_in_string(&lit_str.value()) {
        format_text_cnt = cnt;
      }
      quote! { #lit_str }
    } else {
      quote! { "Unimpl" }
    };

    let status_code_match_block = match &variant.fields {
      Fields::Unit => quote! { #name::#variant_name => #status_code },
      Fields::Unnamed(_) => quote! { #name::#variant_name(..) => #status_code },
      Fields::Named(_) => quote! { #name::#variant_name {..} => #status_code },
    };

    let error_response_match_block = match &variant.fields {
      Fields::Unit => quote! {

        #name::#variant_name => actix_web::HttpResponse::build(self.status_code()).json(crate::utils::message::Message {
          code : "AAASA".to_string(),
          message : #format_text.to_string(),
          status: status,
          datetime: datetime,
          payload: Option::<crate::utils::message::MessagePayload<u8>>::None,
          pagination: pagination,
        })
      },
      Fields::Unnamed(fields) => {
        let bindings_cnt = min(fields.unnamed.len(), format_text_cnt);
        let bindings: Vec<Ident> = (0..bindings_cnt)
          .map(|i| Ident::new(&format!("field_{i}"), variant_name.span()))
          .collect();
        let dots = if fields.unnamed.len() > format_text_cnt {
          quote! { .. }
        } else {
          quote! {}
        };

        quote! {
          #name::#variant_name(#(#bindings),* #dots) => actix_web::HttpResponse::build(self.status_code()).json(crate::utils::message::Message {
            code : "AAASA".to_string(),
            message : format!(#format_text, #(#bindings),*),
            status: status,
            datetime: datetime,
            payload: Option::<crate::utils::message::MessagePayload<u8>>::None,
            pagination: pagination,
          })
        }
      }
      Fields::Named(fields) => {
        let bindings_cnt = min(fields.named.len(), format_text_cnt);
        let bindings: Vec<&Ident> = fields
          .named
          .iter()
          .take(bindings_cnt)
          .flat_map(|f| &f.ident)
          .collect();
        let dots = if fields.named.len() > format_text_cnt {
          quote! { .. }
        } else {
          quote! {}
        };
        quote! {
          #name::#variant_name(#(#bindings),* #dots) => actix_web::HttpResponse::build(self.status_code()).json(crate::utils::message::Message {
            code : "AAASA".to_string(),
            message : format!(#format_text, #(#bindings),*),
            status: status,
            datetime: datetime,
            payload: Option::<crate::utils::message::MessagePayload<u8>>::None,
            pagination: pagination,
          })
        }
      }
    };
    status_code_matchs.push(status_code_match_block);
    error_response_matchs.push(error_response_match_block);
  }

  let generated_status_code_fn = quote! {
      fn status_code(&self) -> actix_web::http::StatusCode{
          match *self{
              #(#status_code_matchs, )*
          }
      }
  };

  let generated_error_response_fn = quote! {
      fn error_response(&self) -> actix_web::HttpResponse {
          let status = "failed".to_string();
          let datetime = chrono::Utc::now();
          let pagination = None;
          match self {
              #(#error_response_matchs, )*
          }
      }
  };

  let response_error_impl = quote! {
      impl actix_web::error::ResponseError for #name {
          #generated_status_code_fn
          #generated_error_response_fn

      }
  };

  Ok(quote! {
      #response_error_impl
  })
}
