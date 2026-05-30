use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Attribute, Ident, Token, Type, Visibility, braced,
    parse::{Parse, ParseStream},
};

/// `skip <name>,` — declares a field the entity has but this view intentionally omits.
/// Listed in the destructure pattern but bound to `_` so the compiler doesn't force
/// us to consume it. Without `skip`, every entity field must appear in the view.
struct SkipToken;
impl syn::parse::Parse for SkipToken {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;
        if ident == "skip" {
            Ok(SkipToken)
        } else {
            Err(syn::Error::new(ident.span(), "expected `skip`"))
        }
    }
}

/// Returns true if the next token is the `skip` keyword (via fork lookahead).
fn peek_skip(input: ParseStream<'_>) -> bool {
    input.fork().parse::<SkipToken>().is_ok()
}

/// Input parsed from the `rkyv_ipc_view!` invocation.
pub struct RkyvIpcViewInput {
    attrs: Vec<Attribute>,
    vis: Visibility,
    struct_name: Ident,
    entity_type: Type,
    fields: Vec<FieldDef>,
}

pub struct FieldDef {
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub name: Ident,
    pub ty: Option<Type>,
    pub as_i64: bool,
    pub skip: bool,
}

impl Parse for RkyvIpcViewInput {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let vis: Visibility = input.parse()?;
        input.parse::<Token![struct]>()?;
        let struct_name: Ident = input.parse()?;
        input.parse::<Token![for]>()?;
        let entity_type: Type = input.parse()?;

        let content;
        braced!(content in input);

        let mut fields = Vec::new();

        while !content.is_empty() {
            // `skip <name>,` — explicitly ignored field
            if peek_skip(&content) {
                content.parse::<SkipToken>()?;
                let field_name: Ident = content.parse()?;
                content.parse::<Token![,]>()?;
                fields.push(FieldDef {
                    attrs: vec![],
                    vis: Visibility::Inherited,
                    name: field_name,
                    ty: None,
                    as_i64: false,
                    skip: true,
                });
                continue;
            }

            // Normal field: `#[attrs] <vis> <name>: <type>,`
            let field_attrs = content.call(Attribute::parse_outer)?;
            let field_vis: Visibility = content.parse()?;
            let field_name: Ident = content.parse()?;
            content.parse::<Token![:]>()?;
            let field_ty: Type = content.parse()?;
            content.parse::<Token![,]>()?;

            let is_as_i64 = field_attrs.iter().any(|a| a.path().is_ident("as_i64"));
            let cleaned: Vec<Attribute> = field_attrs
                .into_iter()
                .filter(|a| !a.path().is_ident("as_i64"))
                .collect();

            fields.push(FieldDef {
                attrs: cleaned,
                vis: field_vis,
                name: field_name,
                ty: Some(field_ty),
                as_i64: is_as_i64,
                skip: false,
            });
        }

        Ok(RkyvIpcViewInput {
            attrs,
            vis,
            struct_name,
            entity_type,
            fields,
        })
    }
}

pub(crate) fn expand(input: RkyvIpcViewInput) -> syn::Result<TokenStream> {
    let RkyvIpcViewInput {
        attrs,
        vis,
        struct_name,
        entity_type,
        fields,
    } = input;

    let mut struct_defs = Vec::new();

    // Every listed field (view + skip) appears here. No `..` — every entity field must
    // be listed as either a view field or a `skip` declaration. This catches:
    //   - Field removed from entity → compile error (pattern references it)
    //   - Field renamed → compile error (old name doesn't match)
    //   - New field added → compile error (must be added to view or skipped)
    let mut destructure_entries = Vec::new();

    let mut from_body = Vec::new();

    for f in &fields {
        let name = &f.name;

        if f.skip {
            // `skip name,` → destructure as `name: _` (acknowledge + ignore)
            destructure_entries.push(quote! { #name : _ , });
        } else {
            let attrs = &f.attrs;
            let fvis = &f.vis;
            let fty = &f.ty;

            struct_defs.push(quote! { #(#attrs)* #fvis #name : #fty , });
            destructure_entries.push(quote! { #name , });

            if f.as_i64 {
                from_body.push(quote! { #name : #name .and_utc().timestamp() , });
            } else {
                from_body.push(quote! { #name : #name .into() , });
            }
        }
    }

    Ok(quote! {
      #(#attrs)*
      #[derive(::rkyv::Archive, ::rkyv::Serialize)]
      #[rkyv(bytecheck())]
      #vis struct #struct_name {
        #(#struct_defs)*
      }

      impl From<#entity_type> for #struct_name {
        fn from(v: #entity_type) -> Self {
          // FIELD CHECK: every entity field must be listed as a view field or a `skip`.
          // A compile error here means a field was added, removed, or renamed.
          let #entity_type { #(#destructure_entries)* } = v;
          Self {
            #(#from_body)*
          }
        }
      }
    })
}
