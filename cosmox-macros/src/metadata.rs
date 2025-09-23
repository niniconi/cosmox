use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Data, DataEnum, DataStruct, DataUnion, DeriveInput, Ident};

pub fn expand_derive_metadata(input: DeriveInput) -> syn::Result<TokenStream> {
  #[allow(unused)]
  let mut is_safe = true;

  let feilds = match input.data {
    Data::Struct(DataStruct { fields, .. }) => fields
      .iter()
      .map(|feild| feild.ident.clone().unwrap())
      .collect::<Vec<_>>(),
    Data::Enum(DataEnum { .. }) => {
      todo!()
    }
    Data::Union(DataUnion { .. }) => {
      // is_safe = false;
      todo!()
    }
  };

  let trait_functions = feilds
    .iter()
    .map(|ident| {
      let name = ident;
      let set_name = Ident::new(
        ("set_".to_string() + &name.to_string()).as_str(),
        Span::call_site(),
      );
      quote! {
        pub(crate) fn #name(&self) -> String;
        pub(crate) fn #set_name(&mut self, data: String);
      }
    })
    .collect::<Vec<_>>();

  let impl_functions = feilds
    .iter()
    .map(|ident| {
      let name = ident;
      let name_string = name.to_string();
      let set_name = Ident::new(
        ("set_".to_string() + &name_string).as_str(),
        Span::call_site(),
      );
      quote! {
        #[inline]
        pub(crate) fn #name(&self) -> String{
          self.get(#name_string.to_string()).unwrap_or(&String::default()).clone()
        }
        #[inline]
        pub(crate) fn #set_name(&mut self, data: String){
          self.insert(#name_string.to_string(), data)
        }
      }
    })
    .collect::<Vec<_>>();
  let default_stmts = feilds
    .iter()
    .map(|ident| {
      let name_string = ident.to_string();
      quote! {
          self.insert(#name_string.to_string(),String::default())
      }
    })
    .collect::<Vec<_>>();

  let metadata_trait = quote! {
    pub(crate) trait _MetdataFiledList{
      pub(crate) fn data_default(&mut self);
      #(#trait_functions)*
    }
  };

  let metadata_impl = quote! {
    impl _MetdataFiledList for HashMap<String, String> {
      #[inline]
      pub(crate) fn data_default(&mut self){
        #(#default_stmts;)*
      }
      #(#impl_functions)*
    }
  };

  Ok(quote! {
    #metadata_trait
    #metadata_impl
  })
}

#[cfg(test)]
mod tests {
  use super::*;
  use syn::parse_quote;

  #[test]
  fn test_expand_derive_metadata() {
    let input: DeriveInput = parse_quote! {
        pub struct AnimeMetadata{
            pub epsi:u32,
            pub season:u32,
            pub description: String
        }
    };
    let expect: TokenStream = quote! {
      pub(crate) trait _MetdataFiledList{
        pub(crate) fn data_default(&mut self);

        pub(crate) fn epsi(&self) -> String;
        pub(crate) fn set_epsi(&mut self, data: String);

        pub(crate) fn season(&self) -> String;
        pub(crate) fn set_season(&mut self, data: String);

        pub(crate) fn description(&self) -> String;
        pub(crate) fn set_description(&mut self, data: String);
      }

      impl _MetdataFiledList for HashMap<String, String> {
        #[inline]
        pub(crate) fn data_default(&mut self){
          self.insert("epsi".to_string(),String::default());
          self.insert("season".to_string(),String::default());
          self.insert("description".to_string(),String::default());
        }
        #[inline]
        pub(crate) fn epsi(&self) -> String{
          self.get("epsi".to_string()).unwrap_or(&String::default()).clone()
        }
        #[inline]
        pub(crate) fn set_epsi(&mut self, data: String){
          self.insert("epsi".to_string(), data)
        }
        #[inline]
        pub(crate) fn season(&self) -> String{
          self.get("season".to_string()).unwrap_or(&String::default()).clone()
        }
        #[inline]
        pub(crate) fn set_season(&mut self, data: String){
          self.insert("season".to_string(), data)
        }
        #[inline]
        pub(crate) fn description(&self) -> String{
          self.get("description".to_string()).unwrap_or(&String::default()).clone()
        }
        #[inline]
        pub(crate) fn set_description(&mut self, data: String){
          self.insert("description".to_string(), data)
        }
      }
    };

    let output = expand_derive_metadata(input).unwrap();
    assert_eq!(expect.to_string(), output.to_string())
  }
}
