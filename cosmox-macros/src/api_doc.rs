use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{
  GenericArgument, Ident, ItemFn, LitStr, PatType, PathArguments, Type, TypePath, TypeTuple,
  spanned::Spanned,
};

use crate::utils::is_primitive_type;

fn parse_params(text: &str, start_delimiter: &str, end_delimiter: &str) -> Vec<String> {
  let mut index = 0;
  let mut params = Vec::with_capacity(4);
  while index < text.len() {
    if let Some(start_index) = text[index..].find(start_delimiter)
      && let Some(end_index) = text[(index + start_index)..].find(end_delimiter)
    {
      params.push(String::from(
        &text[(index + start_index + 1)..(index + start_index + end_index)],
      ));
      index += start_index + end_index + 1;
    } else {
      break;
    }
  }
  params
}

pub fn expand_attr_auto_webapi_doc(input: ItemFn) -> syn::Result<TokenStream> {
  let sig = &input.sig;
  let block = &input.block;
  let span = &input.span();
  let fn_ident = &input.sig.ident;
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

  let api_start_path: String = _split_file_path
    .filter_map(|x| {
      if x.ends_with("_controller.rs") {
        Some(x.trim_end_matches("_controller.rs"))
      } else if x.ends_with("_controller") {
        Some(x.trim_end_matches("_controller"))
      } else {
        None
      }
    })
    .flat_map(|x| ["/", x])
    .collect();

  let mut api_attr_path: Option<String> = None;
  let mut http_method: Option<&Ident> = None;
  for attr in attrs {
    let path = attr.path();
    if (path.is_ident("get")
      || path.is_ident("post")
      || path.is_ident("put")
      || path.is_ident("delete")
      || path.is_ident("options")
      || path.is_ident("patch")
      || path.is_ident("head")
      || path.is_ident("trace")
      || path.is_ident("connect"))
      && let Ok(path_literal) = attr.parse_args::<LitStr>()
    {
      api_attr_path = Some(path_literal.value());
      http_method = path.get_ident();
      break;
    }
  }
  let api_attr_path = api_attr_path.unwrap_or_else(|| fn_ident.to_string());
  let api_path_param_idents = parse_params(&api_attr_path, "{", "}");
  let default_http_method = Ident::new("get", Span::call_site());
  let http_method = http_method.unwrap_or(&default_http_method);

  let mut request_body_type = None;
  let mut query_params_type = None;
  let mut path_param_types: Vec<Type> = Vec::with_capacity(4);

  for param in &input.sig.inputs {
    if let syn::FnArg::Typed(PatType{ ty, .. }) = param
      && let Type::Path(type_path) = &**ty // Check if the type is a Path (e.g., `web::Json<...>`, `String`, `u32`)
      && let Some(last_segment) = type_path.path.segments.last()
    {
      if (last_segment.ident == "Json" || last_segment.ident == "Form") // Check if the last segment of the path is "Json" or "Form"
      && let PathArguments::AngleBracketed(generics) = &last_segment.arguments // If it's `Json` or `Form`, check its generic arguments (e.g., `<MyStruct>`)
      && let Some(GenericArgument::Type(inner_type)) = generics.args.first()
      // Extract the first generic argument (which should be `MyStruct`)
      {
        // let ident = Ident::new("", Span::call_site().into());
        request_body_type = Some(inner_type.clone());
      } else if last_segment.ident == "Query" // Check if the last segment of the path is "Query"
      && let PathArguments::AngleBracketed(generics) = &last_segment.arguments // If it's `Query`, check its generic arguments (e.g., `<MyStruct>`)
      && let Some(GenericArgument::Type(query_param)) = generics.args.first()
      // Extract the first generic argument (which should be `MyStruct`)
      {
        let ok = match query_param {
          Type::Path(TypePath { path, .. }) => path
            .segments
            .last()
            .is_some_and(|val| !is_primitive_type(val.ident.to_string().as_str())),
          _ => false,
        };
        if ok {
          query_params_type = Some(query_param.clone());
        }
      } else if last_segment.ident == "Path"
      && let PathArguments::AngleBracketed(generics) = &last_segment.arguments // If it's `Path`, check its generic arguments (e.g., `<MyStruct>`)
      && let Some(GenericArgument::Type(path_param)) = generics.args.first()
      // Extract the first generic argument (which should be `MyStruct`)
      {
        let mut error = None;

        // TODO check the Type::Path is primitive type
        match path_param {
          Type::Path(..) => path_param_types.push(path_param.clone()),
          Type::Tuple(TypeTuple { elems, .. }) => {
            for r#type in elems {
              match r#type {
                Type::Path(..) => path_param_types.push(r#type.clone()),
                _ => {
                  error = Some((
                    "Expected `primitive type`, but found an unsupported type".to_string(),
                    r#type.to_token_stream(),
                  ))
                }
              }
            }
          }
          _ => {
            error = Some((
              "Expected `primitive type` or `tuple`, but found an unsupported type".to_string(),
              path_param.to_token_stream(),
            ))
          }
        }
        if let Some((error_message, token_stream)) = error {
          return Err(syn::Error::new_spanned(token_stream, error_message));
        }
      }
    }
  }

  let generated_request_body = match request_body_type {
    Some(inner_type) => quote! { request_body = #inner_type, },
    None => quote! {},
  };

  let generated_query_params_attribute = match query_params_type {
    Some(query_params_struct) => quote! { params( #query_params_struct ), },
    None => quote! {},
  };

  let generated_path_params_attribute =
    if !path_param_types.is_empty() && path_param_types.len() == api_path_param_idents.len() {
      quote! {
        params(
          #((#api_path_param_idents=#path_param_types,Path, description="Auto generated"),)*
        ),
      }
    } else {
      quote! {}
    };
  let generated_api_path = quote! {
      path = concat!("api",#api_start_path,"/", #api_attr_path),
  };

  Ok(quote! {
      #[utoipa::path(
          #http_method,
          #generated_api_path
          #generated_request_body
          #generated_query_params_attribute
          #generated_path_params_attribute
          responses(
              (status = 200, description = "successfully"),
              (status = 400, description = ""),
              (status = 409, description = ""),
              (status = 500, description = "Internal server error"),
          ),
          tag = #tag,
      )]
      #(#attrs)*
      #sig {
          #block
      }
  })
}
