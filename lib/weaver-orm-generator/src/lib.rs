use std::{fs::File, io::Read, path::Path, process::Command};

use generator::SourceFile;
use sqlparser::{ast::Statement, dialect::MySqlDialect, parser::Parser};

pub mod generator;

pub fn load_sql_from_file(file_path: &str) -> Result<String, std::io::Error> {
  let mut file = File::open(file_path)?;
  let file_metadata = file.metadata()?;

  let mut sql = String::with_capacity(file_metadata.len() as usize);
  file.read_to_string(&mut sql)?;

  Ok(sql)
}

pub fn load_sql_from_remote(
  host: &str,
  port: u16,
  user: &str,
  password: &str,
  database: &str,
) -> Result<String, std::io::Error> {
  let stdout = Command::new("mariadb-dump")
    .arg("-h")
    .arg(host)
    .arg("-P")
    .arg(port.to_string().as_str())
    .arg("-u")
    .arg(user)
    .arg(format!("-p{password}").as_str())
    .arg(database)
    .arg("--no-data")
    .output()?
    .stdout;

  Ok(unsafe { String::from_utf8_unchecked(stdout) })
}

pub fn generate<P: AsRef<Path>>(output: P, sql: String) -> Result<(), std::io::Error> {
  let mysql_dialect = MySqlDialect {};

  // sqlparser can't process a line that only contains `--`
  let sql = sql.replace("--\n", "");

  match Parser::parse_sql(&mysql_dialect, sql.as_str()) {
    Ok(statements) => {
      // generate source code
      let result: Vec<SourceFile> = statements
        .iter()
        .filter_map(|x| match x {
          Statement::CreateTable(statement) => Some(generator::generate_rust_source(statement)),
          _ => None,
        })
        .collect();

      // save source code
      let mut mods = Vec::with_capacity(result.len());
      for source_file in result {
        println!("generated {}", source_file.filename);
        // println!("{}", source_file.source);

        // save single source file
        generator::store_source_file(&output, &source_file)?;

        // format
        let source_file_path = output.as_ref().join(&source_file.filename);
        Command::new("rustfmt").arg(source_file_path).output()?;

        // store mod name
        let source_filename = source_file.filename;
        let mod_name: String = if source_filename.ends_with(".rs") {
          String::from(&source_filename[..source_filename.len() - ".rs".len()])
        } else {
          String::from(&source_filename[..])
        };
        mods.push(mod_name);
      }
      // save module file
      generator::store_mod_source_file(&output, mods)?;
    }
    Err(err) => panic!("Err:{err}"),
  }

  // format source code
  let mod_file_path = output.as_ref().join("mod.rs");
  Command::new("rustfmt").arg(mod_file_path).output()?;
  Ok(())
}
