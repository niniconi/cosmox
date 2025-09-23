use std::{
  fmt::Display,
  fs::{self, File},
  io::Write,
  path::Path,
};

use sqlparser::ast::{
  ColumnOption, CommentDef, CreateTable, DataType, ObjectName, TableConstraint,
};

pub struct SourceFile {
  pub source: String,
  pub filename: String,
}

// This is for more standardized code generation.
pub struct Field {
  pub ident: String,
  pub is_option: bool,
  pub is_primary_key: bool,
  pub r#type: String,
  pub attr: Option<Vec<String>>,
  pub comment: Option<Vec<String>>,
}

impl Display for Field {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let ident = &self.ident;
    let r#type = &self.r#type;
    let attrs = "";

    if self.is_option {
      write!(
        f,
        r#"
      {attrs}
      pub {ident}: Option<{type}>,
      "#
      )
    } else {
      write!(
        f,
        r#"
      {attrs}
      pub {ident}:<{type}>,
        "#
      )
    }
  }
}

pub struct Model {}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn test_fileld() {
    let field = Field {
      ident: "username".into(),
      is_option: false,
      is_primary_key: true,
      r#type: "Sting".into(),
      attr: Some(vec![
        r#"#[serde(with = "chrono::naive::serde::ts_seconds_option")]"#.into(),
      ]),
      comment: None,
    };

    println!("{field}")
  }
}

macro_rules! field_generate {
  {keywords = $keywords:expr, input = $input:expr, $(($type:pat, $name:expr, $member_type:literal, $is_not_null:expr, $is_primary_key:expr, $attr:expr)),*$(,)?} => {
    match $input{
      $($type => {
        let name = if $keywords.contains(&$name.as_str()) {
            format!("r#{}",$name)
        }else{ $name.clone() };
        format!("{}{}{}",
          if $is_primary_key { "#[sea_orm(primary_key)]\n" } else { "" },
          $attr,
          if $is_not_null{
            format!(concat!("pub {}:",$member_type,", \n"), name)
          } else {
            format!( concat!("pub {}:Option<",$member_type,">, \n"), name)
          }
        )
      })*
      _ => {String::from("// unimpl")}
    }
  };

  {input = $input:expr, $(($type:pat, $name:expr, $member_type:literal, $is_not_null:expr, $is_primary_key:expr, $attr:literal)),*$(,)?} => {
    field_generate!{
      keywords = vec![],
      input = $input,
      $(($type, $name, $member_type, $is_not_null, $is_primary_key, $attr)),*
    }
  };
}

fn generate_rust_member_list(statement: &CreateTable) -> String {
  let mut primary_keys = Vec::with_capacity(2);
  for x in &statement.constraints {
    if let TableConstraint::PrimaryKey { columns, .. } = x {
      for ident in columns {
        primary_keys.push(ident)
      }
    }
  }

  statement
    .columns
    .iter()
    .map(|x| {
      let mut is_not_null = false;
      let is_primary_key = primary_keys.contains(&&x.name);
      for option in &x.options {
        if matches!(option.option, ColumnOption::NotNull) {
          is_not_null = true;
        }
      }
      println!("{:#?}", x.data_type);
      field_generate! {
          keywords = vec!["type", "else", "if", "else", "let","fn", "pub", "match", "struct", "use", "mod", "extern", "crate"],
          input = x.data_type,
          (DataType::BigIntUnsigned(Some(64)), x.name.value, "u64", is_not_null, is_primary_key, ""),
          (DataType::BigInt(Some(64)), x.name.value, "i64", is_not_null, is_primary_key, ""),
          (DataType::Varchar(Some(_)), x.name.value, "String", is_not_null, is_primary_key, ""),
          (DataType::Datetime(_), x.name.value, "NaiveDateTime", is_not_null, is_primary_key, if is_not_null {
            r#"#[serde(with = "chrono::naive::serde::ts_seconds")]"#
          }else {
            r#"#[serde(with = "chrono::naive::serde::ts_seconds_option")]"#
          })
      }
    })
    .collect()
}

#[inline]
fn generate_multi_line_doc_comment(comment: &Option<CommentDef>) -> String {
  match comment {
    Some(comment) => format!("/// {}", comment.to_string().replace("\n", "\n/// ")),
    None => String::from(""),
  }
}

fn generate_using(code: &str) -> String {
  let chrono_using = if code.contains("NaiveDateTime") {
    "use chrono::NaiveDateTime;"
  } else {
    ""
  };
  format!(
    r#"{chrono_using}
use serde::{{Deserialize, Serialize}};
use sqlx::FromRow;
use utoipa::ToSchema;
use sea_orm::entity::prelude::*;
    "#
  )
}

#[inline]
fn unwarp_ident(name: &ObjectName) -> Option<String> {
  if let Some(table_ident) = name.0.last() {
    let ident = table_ident.as_ident().unwrap();
    Some(ident.value.clone())
  } else {
    None
  }
}

// fn fmt_struct_name(name: String) -> String {
//   name
// }

pub fn generate_rust_source(statement: &CreateTable) -> SourceFile {
  println!("{statement}");
  println!("{statement:#?}");
  let table_name = unwarp_ident(&statement.name).unwrap();
  let comment = &statement.comment;

  let rust_member_list = generate_rust_member_list(statement);
  let multi_line_doc_comment = generate_multi_line_doc_comment(comment);
  let using = generate_using(&rust_member_list);

  SourceFile {
    source: format!(
      r#"
{using}
{multi_line_doc_comment}
#[derive(Debug, Clone, PartialEq, Eq, Hash, FromRow, Serialize, Deserialize, ToSchema, DeriveEntityModel)]
#[sea_orm(table_name = "{table_name}")]
pub struct Model {{
{rust_member_list}
}}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {{}}

impl RelationTrait for Relation {{
    fn def(&self) -> RelationDef {{
        panic!("No Relation");
    }}
}}

impl ActiveModelBehavior for ActiveModel {{}}
    "#
    ),
    filename: format!("{table_name}.rs"),
  }
}

pub fn store_source_file<P: AsRef<Path>>(
  output_dir: P,
  source_file: &SourceFile,
) -> Result<(), std::io::Error> {
  fs::create_dir_all(&output_dir)?;

  let source_file_path = output_dir.as_ref().join(&source_file.filename);
  let mut file = File::create(source_file_path)?;
  file.write_all(source_file.source.as_bytes())?;
  Ok(())
}

pub fn store_mod_source_file<P: AsRef<Path>>(
  output_dir: P,
  sub_mods: Vec<String>,
) -> Result<(), std::io::Error> {
  let mod_source: String = sub_mods.iter().map(|x| format!("pub mod {x};\n")).collect();
  let mod_source = format!("//! Generated by {} \n{}", file!(), mod_source);

  fs::create_dir_all(&output_dir)?;

  let mod_file_path = output_dir.as_ref().join("mod.rs");
  let mut file = File::create(mod_file_path)?;
  file.write_all(mod_source.as_bytes())?;
  Ok(())
}
