use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Data, DataStruct, DeriveInput, Fields, GenericArgument, LitStr, PathArguments, Type, TypePath,
};

enum FieldKind {
    String,
    OptString,
    Value,
    OptValue,
}

/// Classify a field and return the type used for `parse::<T>()` in the
/// read path (unwrapped for `Option<T>`); `None` for string fields.
fn classify(ty: &Type) -> (FieldKind, Option<&Type>) {
    let Type::Path(TypePath { path, .. }) = ty else {
        return (FieldKind::Value, Some(ty));
    };
    let Some(last) = path.segments.last() else {
        return (FieldKind::Value, Some(ty));
    };
    if last.ident == "String" {
        return (FieldKind::String, None);
    }
    if last.ident == "Option" {
        let PathArguments::AngleBracketed(args) = &last.arguments else {
            return (FieldKind::OptValue, Some(ty));
        };
        let Some(GenericArgument::Type(inner)) = args.args.first() else {
            return (FieldKind::OptValue, Some(ty));
        };
        if let Type::Path(TypePath {
            path: inner_path, ..
        }) = inner
            && inner_path
                .segments
                .last()
                .is_some_and(|seg| seg.ident == "String")
        {
            (FieldKind::OptString, None)
        } else {
            (FieldKind::OptValue, Some(inner))
        }
    } else {
        (FieldKind::Value, Some(ty))
    }
}

pub fn expand(input: DeriveInput) -> syn::Result<TokenStream> {
    let mut extend_key: Option<String> = None;
    for attr in &input.attrs {
        if !attr.path().is_ident("extend") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("key") {
                let value: LitStr = meta.value()?.parse()?;
                extend_key = Some(value.value());
            }
            Ok(())
        })?;
    }
    let extend_key = extend_key.ok_or_else(|| {
        syn::Error::new_spanned(&input, "missing #[extend(key = \"...\")] attribute")
    })?;
    let struct_ident = input.ident;

    let fields = match input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(named),
            ..
        }) => named
            .named
            .iter()
            .map(|f| {
                let (kind, parse_ty) = classify(&f.ty);
                (f.ident.clone().unwrap(), kind, parse_ty.cloned())
            })
            .collect::<Vec<_>>(),
        _ => {
            return Err(syn::Error::new_spanned(
                &struct_ident,
                "MetadataExtend only supports structs with named fields",
            ));
        }
    };

    let destructure = fields.iter().map(|(ident, _, _)| quote!(#ident));
    let field_count = fields.len();

    let write_stmts = fields.iter().map(|(ident, kind, _)| {
        let field_name = ident.to_string();
        match kind {
            FieldKind::String => quote! {
                out.push((format!("{}:{}", Self::EXTEND_KEY, #field_name), #ident));
            },
            FieldKind::OptString => quote! {
                if let Some(v) = #ident {
                    out.push((format!("{}:{}", Self::EXTEND_KEY, #field_name), v));
                }
            },
            FieldKind::Value => quote! {
                out.push((format!("{}:{}", Self::EXTEND_KEY, #field_name), #ident.to_string()));
            },
            FieldKind::OptValue => quote! {
                if let Some(v) = #ident {
                    out.push((format!("{}:{}", Self::EXTEND_KEY, #field_name), v.to_string()));
                }
            },
        }
    });

    let read_stmts = fields.iter().map(|(ident, kind, parse_ty)| {
        let field_name = ident.to_string();
        match kind {
            FieldKind::String => quote! {
                #ident: get(#field_name).cloned().unwrap_or_default(),
            },
            FieldKind::OptString => quote! {
                #ident: get(#field_name).cloned(),
            },
            FieldKind::Value | FieldKind::OptValue => {
                let parse_ty = parse_ty.as_ref().expect("value field has a parse type");
                let default = matches!(kind, FieldKind::Value)
                    .then(|| quote! { .unwrap_or_default() })
                    .unwrap_or_default();
                quote! {
                    #ident: get(#field_name)
                        .map(|v| {
                            v.parse::<#parse_ty>().map_err(|err| {
                                ::cosmox_api::extend::ExtendError::Parse(err.to_string())
                            })
                        })
                        .transpose()? #default,
                }
            }
        }
    });

    Ok(quote! {
        impl ::cosmox_api::extend::MetadataExtend for #struct_ident {
            const EXTEND_KEY: &'static str = #extend_key;

            fn to_extend_pairs(self) -> Vec<(String, String)> {
                let Self { #(#destructure,)* .. } = self;
                let mut out = Vec::with_capacity(#field_count);
                #(#write_stmts)*
                out
            }

            fn from_extend_pairs(
                pairs: &std::collections::HashMap<String, String>,
            ) -> Result<Self, ::cosmox_api::extend::ExtendError> {
                let get = |name: &str| pairs.get(&format!("{}:{name}", Self::EXTEND_KEY));
                Ok(Self {
                    #(#read_stmts)*
                })
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_expand_metadata_extend() {
        let input: DeriveInput = parse_quote! {
            #[derive(Default)]
            #[extend(key = "anime")]
            pub struct AnimeExtend {
                pub season_number: Option<u32>,
                pub episode_number: Option<u32>,
                pub extra_kind: Option<String>,
                pub extra_tag: Option<String>,
                pub alias: String,
            }
        };
        let expect: TokenStream = quote! {
            impl ::cosmox_api::extend::MetadataExtend for AnimeExtend {
                const EXTEND_KEY: &'static str = "anime";

                fn to_extend_pairs(self) -> Vec<(String, String)> {
                    let Self { season_number, episode_number, extra_kind, extra_tag, alias, .. } = self;
                    let mut out = Vec::with_capacity(5usize);
                    if let Some(v) = season_number {
                        out.push((format!("{}:{}", Self::EXTEND_KEY, "season_number"), v.to_string()));
                    }
                    if let Some(v) = episode_number {
                        out.push((format!("{}:{}", Self::EXTEND_KEY, "episode_number"), v.to_string()));
                    }
                    if let Some(v) = extra_kind {
                        out.push((format!("{}:{}", Self::EXTEND_KEY, "extra_kind"), v));
                    }
                    if let Some(v) = extra_tag {
                        out.push((format!("{}:{}", Self::EXTEND_KEY, "extra_tag"), v));
                    }
                    out.push((format!("{}:{}", Self::EXTEND_KEY, "alias"), alias));
                    out
                }

                fn from_extend_pairs(
                    pairs: &std::collections::HashMap<String, String>,
                ) -> Result<Self, ::cosmox_api::extend::ExtendError> {
                    let get = |name: &str| pairs.get(&format!("{}:{name}", Self::EXTEND_KEY));
                    Ok(Self {
                        season_number: get("season_number")
                            .map(|v| {
                                v.parse::<u32>().map_err(|err| {
                                    ::cosmox_api::extend::ExtendError::Parse(err.to_string())
                                })
                            })
                            .transpose()?,
                        episode_number: get("episode_number")
                            .map(|v| {
                                v.parse::<u32>().map_err(|err| {
                                    ::cosmox_api::extend::ExtendError::Parse(err.to_string())
                                })
                            })
                            .transpose()?,
                        extra_kind: get("extra_kind").cloned(),
                        extra_tag: get("extra_tag").cloned(),
                        alias: get("alias").cloned().unwrap_or_default(),
                    })
                }
            }
        };
        let output = expand(input).unwrap();
        println!("{}", output);
        assert_eq!(expect.to_string(), output.to_string())
    }

    #[test]
    fn test_missing_extend_key_errors() {
        let input: DeriveInput = parse_quote! {
            pub struct NoKey {
                pub field: String,
            }
        };
        assert!(expand(input).is_err());
    }
}
